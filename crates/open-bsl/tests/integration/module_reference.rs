use open_bsl::{Engine, Value};

const SOURCE: &str = r#"
Перем ЧислоВызовов;
Процедура Обработать(Результат, Данные) Экспорт
    ЧислоВызовов = ЧислоВызовов + 1;
КонецПроцедуры
Функция Открытая() Экспорт
    Возврат ЭтотОбъект;
КонецФункции
Процедура Служебная()
    ЧислоВызовов = ЧислоВызовов + 1;
КонецПроцедуры
Функция СсылкаИзФункции()
    Возврат Вычислить("ThisObject");
КонецФункции
Функция Перекрытие(ЭтотОбъект)
    Возврат ЭтотОбъект;
КонецФункции
Асинх Процедура ПроверитьАсинх(Ожидаемая)
    Если ЭтотОбъект <> Ожидаемая Или Вычислить("ThisObject") <> Ожидаемая Тогда
        ВызватьИсключение "async identity";
    КонецЕсли;
КонецПроцедуры
ЧислоВызовов = 0;
// Первое чтение происходит внутри динамического фрагмента.
Ссылка = Вычислить("ThisObject");
Если Ссылка <> ЭтотОбъект Или Ссылка <> ThisObject() Или Ссылка <> СсылкаИзФункции() Тогда
    ВызватьИсключение "identity";
КонецЕсли;
Другая = Неопределено;
Выполнить("Другая = Вычислить(""ЭтотОбъект()"");");
Если Другая <> Ссылка Тогда ВызватьИсключение "dynamic identity"; КонецЕсли;
Попытка Выполнить("ВызватьИсключение ""проверка восстановления"";"); Исключение КонецПопытки;
Если ЭтотОбъект <> Ссылка Тогда ВызватьИсключение "identity after exception"; КонецЕсли;
ПроверитьАсинх(Ссылка);
Если ТипЗнч(Ссылка) <> Тип("BSLModule") Или ТипЗнч(Ссылка) <> Тип("МодульBSL") Тогда
    ВызватьИсключение "type";
КонецЕсли;
Описание = Новый ОписаниеОповещения("оБрАбОтАтЬ", ЭтотОбъект);
Если Описание.Модуль <> Ссылка Тогда ВызватьИсключение "receiver"; КонецЕсли;
ОписаниеФункции = Новый NotifyDescription("Открытая", Ссылка);
ОшибкаБыла = Ложь;
Попытка Описание = Новый NotifyDescription("Служебная", Ссылка); Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Или ЧислоВызовов <> 0 Тогда ВызватьИсключение "exports"; КонецЕсли;
Если Перекрытие(17) <> 17 Тогда ВызватьИсключение "shadowing"; КонецЕсли;
Ключи = Новый Соответствие;
Ключи.Вставить(Ссылка, 42);
Если Ключи.Получить(ЭтотОбъект) <> 42 Тогда ВызватьИсключение "key"; КонецЕсли;
Возврат Ссылка;
"#;

#[test]
fn current_module_reference_keeps_identity_and_exports_in_every_execution_mode() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        assert!(engine.compile("Возврат ThisObject(1);").is_err());
        assert!(engine.compile("Возврат Новый МодульBSL();").is_err());
        let module = engine.compile(SOURCE).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
            } else {
                engine.compile(SOURCE).unwrap()
            };
            let mut state = engine.new_state();
            let first = state.run(&module).unwrap();
            let second = state.run(&module).unwrap();
            assert_ne!(first, second, "два запуска разделяют экземпляр модуля");
            let object = first.object_ref().unwrap();
            assert!(object.has_method("ОБРАБОТАТЬ"));
            assert!(object.has_method("открытая"));
            assert!(!object.has_method("Служебная"));
            assert!(!object.has_method("Несуществующая"));
            assert_ne!(first, Value::Undefined);
        }
    }
}

#[test]
fn a_module_variable_can_shadow_the_global_property_without_changing_the_other_alias() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    assert_eq!(state.exec("Перем ЭтотОбъект; ЭтотОбъект = 17; Если ТипЗнч(ThisObject) <> Тип(\"BSLModule\") Тогда ВызватьИсключение \"alias\"; КонецЕсли; Возврат ЭтотОбъект;").unwrap(), Value::number_from_i64(17));
}

#[test]
fn catalog_modules_keep_separate_references_across_calls_and_dynamic_fragments() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let module_source = r#"
Перем Сохраненная;
Функция Ссылка() Экспорт
    Если ЭтотОбъект <> Сохраненная Тогда ВызватьИсключение "module body identity"; КонецЕсли;
    Возврат Вычислить("ThisObject");
КонецФункции
Процедура Обработчик(Результат, Данные) Экспорт
КонецПроцедуры
Процедура Скрытый()
КонецПроцедуры
Сохраненная = Вычислить("ЭтотОбъект");
"#;
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("Первый", module_source)
            .common_module("Второй", module_source)
            .build()
            .unwrap();
        let entry = engine.compile_entry(r#"
А = Первый.Ссылка(); Б = Второй.Ссылка();
Если А <> Первый.Ссылка() Или Б <> Второй.Ссылка() Или А = Б Или А = ЭтотОбъект Или Б = ЭтотОбъект Тогда
    ВызватьИсключение "catalog identities";
КонецЕсли;
Описание = Новый ОписаниеОповещения("Обработчик", А);
Если Описание.Модуль <> А Тогда ВызватьИсключение "catalog receiver"; КонецЕсли;
Возврат А;
"#).unwrap();
        let mut state = engine.new_state();
        let first = state.run(&entry).unwrap();
        let second = state.run(&entry).unwrap();
        assert_ne!(first, second);
    }
}
