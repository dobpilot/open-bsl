use open_bsl::{Engine, Value};

#[test]
fn awaited_dynamic_child_errors_preserve_caller_locals() {
    for delay in ["", "Ждать 0;"] {
        let source = format!(
            r#"
Асинх Функция ОшибкаДочерней()
    {delay}
    ВызватьИсключение "ошибка дочерней";
КонецФункции
Асинх Процедура Проверить()
    ЛокальнаяПробы = 1;
    ОбещаниеПробы = Неопределено;
    ОшибкаБыла = Ложь;
    Попытка
        Выполнить("ЛокальнаяПробы = 7; ОбещаниеПробы = ОшибкаДочерней();");
        Ждать ОбещаниеПробы;
    Исключение
        Если СтрНайти(ИнформацияОбОшибке().Описание, "ошибка дочерней") = 0 Тогда
            ВызватьИсключение "получена не исходная ошибка задачи";
        КонецЕсли;
        ОшибкаБыла = Истина;
    КонецПопытки;
    Если Не ОшибкаБыла Тогда ВызватьИсключение "потеря ошибки"; КонецЕсли;
    Если ЛокальнаяПробы <> 7 Тогда ВызватьИсключение "потеря локали"; КонецЕсли;
КонецПроцедуры
Проверить();
Возврат 7;
"#
        );
        check_modes(&source, 7);
    }
}

#[test]
fn a_dynamic_child_writes_the_same_module_after_the_fragment_returns() {
    let source = r#"
Перем ОбщаяПробы, ОбещаниеПробы;
Асинх Функция ЗаписатьПозже()
    Ждать 0;
    ОбщаяПробы = 41;
    ВызватьИсключение "ошибка дочерней";
КонецФункции
Функция Мост(АргументПробы)
    АргументПробы = 7;
    ЛокальнаяПробы = 1;
    Выполнить("ЛокальнаяПробы = 11; ОбещаниеПробы = ЗаписатьПозже();");
    Если АргументПробы <> 7 Тогда ВызватьИсключение "фрагмент ожидал задачу"; КонецЕсли;
    Возврат ЛокальнаяПробы;
КонецФункции
Асинх Процедура Проверить()
    Если Мост(ОбщаяПробы) <> 11 Тогда ВызватьИсключение "потеря локали"; КонецЕсли;
    Попытка Ждать ОбещаниеПробы; Исключение КонецПопытки;
    Если ОбщаяПробы <> 41 Тогда ВызватьИсключение "потеря записи модуля"; КонецЕсли;
КонецПроцедуры
ОбщаяПробы = 1;
Проверить();
Возврат 11;
"#;
    check_modes(source, 11);
    check_modes(
        &source.replace("ВызватьИсключение \"ошибка дочерней\";", ""),
        11,
    );
}

fn check_modes(source: &str, expected: i64) {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let compiled = engine.compile(source).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
            } else {
                engine.compile(source).unwrap()
            };
            assert_eq!(
                engine.new_state().run(&module).unwrap(),
                Value::number_from_i64(expected)
            );
        }
    }
}

#[test]
fn foreign_module_arguments_keep_dynamic_writes_when_the_fragment_fails() {
    let worker = r#"
Функция Ссылка() Экспорт Возврат ЭтотОбъект; КонецФункции
Процедура Изменить(Первый, Второй) Экспорт
    Попытка
        Выполнить("Первый = 31; ВызватьИсключение ""ошибка"";");
    Исключение
        Если Первый <> 31 Или Второй <> 31 Тогда ВызватьИсключение "потеря алиаса"; КонецЕсли;
    КонецПопытки;
    Выполнить("Второй = 41; ВызватьИсключение ""внешняя ошибка"";");
КонецПроцедуры
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("Хранилище", "Перем Ячейка Экспорт; Ячейка = 1;")
            .common_module("Работа", worker)
            .build()
            .unwrap();
        for receiver in ["Работа", "СсылкаРаботы"] {
            let source = format!(
                "СсылкаРаботы = Работа.Ссылка(); ОшибкаБыла = Ложь;
                 Попытка {receiver}.Изменить(Хранилище.Ячейка, Хранилище.Ячейка);
                 Исключение ОшибкаБыла = Истина; КонецПопытки;
                 Если Не ОшибкаБыла Тогда ВызватьИсключение \"потеря ошибки\"; КонецЕсли;
                 Возврат Хранилище.Ячейка;"
            );
            let compiled = engine.compile_entry(&source).unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                } else {
                    engine.compile_entry(&source).unwrap()
                };
                assert_eq!(
                    engine.new_state().run(&module).unwrap(),
                    Value::number_from_i64(41)
                );
            }
        }
    }
}
