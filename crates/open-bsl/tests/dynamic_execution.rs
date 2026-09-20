use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, Value};

#[test]
fn dynamic_frames_keep_catalog_aliases_callbacks_and_async_code_alive() {
    let worker = r#"
Перем ЗначениеИзТела Экспорт;
Функция Прочитать()
    Возврат Данные.Ячейка;
КонецФункции
Процедура Записать(Значение)
    Данные.Ячейка = Значение;
КонецПроцедуры
Процедура Изменить(Первый, Второй) Экспорт
    Попытка
        Выполнить("Первый = 31;
        |Если Прочитать() <> 31 Или Второй <> 31 Тогда ВызватьИсключение ""нет общей ячейки""; КонецЕсли;
        |Записать(41);
        |Выполнить(""Если Первый <> 41 Тогда ВызватьИсключение """"нет вложенной ячейки""""; КонецЕсли;"");
        |ВызватьИсключение ""ожидаемая ошибка"";");
    Исключение
        Если Первый <> 41 Или Второй <> 41 Тогда ВызватьИсключение "потеря записи"; КонецЕсли;
    КонецПопытки;
    Первый = Первый + 1;
КонецПроцедуры
Асинх Функция Позже(Получатель) Экспорт
    Ждать 0;
    СтруктураПробы = Вычислить("Новый Структура(""ПолеФрагмента"", Прочитать())");
    Возврат Получатель.Добавить(СтруктураПробы.ПолеФрагмента);
КонецФункции
Функция Обещание(Получатель) Экспорт
    Возврат Вычислить("ЭтотОбъект.Позже(Получатель)");
КонецФункции
ЗначениеИзТела = Вычислить("Прочитать()");
"#;
    let entry = r#"
Перем Прибавка;
Функция Добавить(ЧислоПробы) Экспорт
    Возврат ЧислоПробы + Прибавка;
КонецФункции
Асинх Процедура Проверить()
    Если Работа.ЗначениеИзТела <> 1 Тогда ВызватьИсключение "тело или lazy init"; КонецЕсли;
    Работа.Изменить(Хранилище.Ячейка, Хранилище.Ячейка);
    Если Хранилище.Ячейка <> 42 Тогда ВызватьИсключение "аргумент"; КонецЕсли;
    ОбещаниеПробы = Работа.Обещание(ЭтотОбъект);
    Хранилище.Ячейка = 50;
    РезультатПробы = Ждать ОбещаниеПробы;
    Если РезультатПробы <> 57 Тогда ВызватьИсключение "каталог, контекст или callback"; КонецЕсли;
КонецПроцедуры
Прибавка = 7;
Проверить();
Возврат 1;
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        for eager_init in [false, true] {
            let engine = Engine::builder()
                .optimizations(optimizations)
                .configuration(ModuleGraphRecipe {
                    modules: vec![
                        ModuleRecipe {
                            name: "Хранилище".into(),
                            source: "Перем Ячейка Экспорт; Ячейка = 1;".into(),
                            imports: vec![],
                        },
                        ModuleRecipe {
                            name: "Работа".into(),
                            source: worker.into(),
                            imports: vec![("Данные".into(), "Хранилище".into())],
                        },
                    ],
                    eager_init,
                })
                .build()
                .unwrap();
            let compiled = engine.compile_entry(entry).unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                } else {
                    engine.compile_entry(entry).unwrap()
                };
                for _ in 0..2 {
                    assert_eq!(
                        engine.new_state().run(&module).unwrap(),
                        Value::number_from_i64(1)
                    );
                }
            }
        }
    }
}

#[test]
fn rejected_dynamic_await_preserves_the_callers_promise_and_local() {
    let source = r#"
Асинх Функция Позже()
    Ждать 0;
    Возврат 23;
КонецФункции
Асинх Процедура Проверить()
    ОбещаниеПробы = Позже();
    ЛокальнаяПробы = 1;
    ОшибкаБыла = Ложь;
    Попытка
        Выполнить("ЛокальнаяПробы = 11; ЛокальнаяПробы = Ждать ОбещаниеПробы;");
    Исключение
        ОшибкаБыла = Истина;
    КонецПопытки;
    Если Не ОшибкаБыла Или ЛокальнаяПробы <> 1 Тогда ВызватьИсключение "фрагмент исполнился"; КонецЕсли;
    Если Ждать ОбещаниеПробы <> 23 Тогда ВызватьИсключение "потеря обещания"; КонецЕсли;
КонецПроцедуры
Проверить();
Возврат 1;
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let compiled = engine.compile(source).unwrap();
        let restored = engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap();
        for module in [&compiled, &restored] {
            assert_eq!(
                engine.new_state().run(module).unwrap(),
                Value::number_from_i64(1)
            );
        }
    }
}
