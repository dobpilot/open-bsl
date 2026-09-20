use open_bsl::Engine;

fn run_modes(source: &str) {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for bytecode in [false, true] {
            let module = engine.compile(source).unwrap();
            let module = if bytecode {
                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
            } else {
                module
            };
            engine.new_state().run(&module).unwrap();
        }
    }
}

#[test]
fn fixed_collections_are_shallow_immutable_snapshots_in_every_mode() {
    run_modes(
        r#"
ВложенныйМассив = Новый Массив;
ВложенныйМассив.Add(10);
ИсточникСтруктуры = Новый Структура("А,Б", 1, ВложенныйМассив);
ФиксСтруктура = Новый FixedStructure(ИсточникСтруктуры);
Если ФиксСтруктура.Count() <> 2 Или Не ФиксСтруктура.Property("А") Или ФиксСтруктура.Property("Нет") Или ФиксСтруктура.А <> 1 Тогда
    ВызватьИсключение "fixed structure";
КонецЕсли;
ИсточникСтруктуры.А = 2;
ВложенныйМассив.Add(20);
Если ФиксСтруктура.А <> 1 Или ФиксСтруктура.Б.Count() <> 2 Тогда ВызватьИсключение "shallow structure"; КонецЕсли;
ОшибкаБыла = Ложь;
Попытка ФиксСтруктура.А = 3; Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Или ФиксСтруктура.А <> 1 Тогда ВызватьИсключение "immutable structure"; КонецЕсли;
Если (Новый ФиксированнаяСтруктура).Count() <> 0 Тогда ВызватьИсключение "empty structure"; КонецЕсли;

ВложенныйМассивКарты = Новый Массив;
ВложенныйМассивКарты.Add(30);
ИсточникКарты = Новый Соответствие;
ИсточникКарты.Insert("А", 1);
ИсточникКарты.Insert(2, ВложенныйМассивКарты);
ФиксКарта = Новый FixedMap(ИсточникКарты);
Если ФиксКарта.Count() <> 2 Или ФиксКарта.Get("А") <> 1 Или ФиксКарта.Get("Нет") <> Неопределено Тогда
    ВызватьИсключение "fixed map";
КонецЕсли;
ИсточникКарты.Insert("А", 2);
ВложенныйМассивКарты.Add(40);
Если ФиксКарта.Get("А") <> 1 Или ФиксКарта.Get(2).Count() <> 2 Тогда ВызватьИсключение "shallow map"; КонецЕсли;
ОшибкаБыла = Ложь;
Попытка ФиксКарта.Insert("А", 3); Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Или ФиксКарта.Get("А") <> 1 Тогда ВызватьИсключение "immutable map"; КонецЕсли;

ОбычнаяСтруктураВыхода = Новый Структура("А", 1);
ЛокальныйВыход = 99;
Если Не ОбычнаяСтруктураВыхода.Property("А", ЛокальныйВыход) Или ЛокальныйВыход <> 1 Тогда ВызватьИсключение "local output"; КонецЕсли;
ЛокальныйВыход = 99;
Если ОбычнаяСтруктураВыхода.Свойство("Нет", ЛокальныйВыход) Или ЛокальныйВыход <> Неопределено Тогда ВызватьИсключение "missing output"; КонецЕсли;
ИндексныйВыход = Новый Массив;
ИндексныйВыход.Добавить(99);
Если Не ОбычнаяСтруктураВыхода.Свойство("А", ИндексныйВыход[0]) Или ИндексныйВыход[0] <> 1 Тогда ВызватьИсключение "indexed output"; КонецЕсли;
Если Не ОбычнаяСтруктураВыхода.Свойство("А", 99) Тогда ВызватьИсключение "discarded output"; КонецЕсли;
ФиксированныйВыход = 99;
Если Не ФиксСтруктура.Property("А", ФиксированныйВыход) Или ФиксированныйВыход <> 1 Тогда ВызватьИсключение "fixed output"; КонецЕсли;
"#,
    );
}

#[test]
fn property_writes_module_and_imported_variables_in_every_mode() {
    let source = r#"
Перем МодульныйВыход;
МодульныйВыход = 99;
СтруктураПробы = Новый Структура("А", 1);
Если Не СтруктураПробы.Свойство("А", МодульныйВыход) Или МодульныйВыход <> 1 Тогда ВызватьИсключение "module"; КонецЕсли;
Если Не СтруктураПробы.Property("А", Хранилище.Выход) Или Хранилище.Выход <> 1 Тогда ВызватьИсключение "imported"; КонецЕсли;
Возврат МодульныйВыход + Хранилище.Выход;
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("Хранилище", "Перем Выход Экспорт; Выход = 99;")
            .build()
            .unwrap();
        let compiled = engine.compile_entry(source).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
            } else {
                engine.compile_entry(source).unwrap()
            };
            assert_eq!(
                engine.new_state().run(&module).unwrap(),
                open_bsl::Value::number_from_i64(2)
            );
        }
    }
}

#[test]
fn fixed_collection_constructors_validate_the_measured_sources() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Объект = Новый ФиксированнаяСтруктура(1);",
        "Объект = Новый ФиксированноеСоответствие(1);",
        "Обычная = Новый Структура; Фикс = Новый ФиксированнаяСтруктура(Обычная); Еще = Новый ФиксированнаяСтруктура(Фикс);",
        "Обычное = Новый Соответствие; Фикс = Новый ФиксированноеСоответствие(Обычное); Еще = Новый ФиксированноеСоответствие(Фикс);",
    ] {
        let module = engine.compile(source).unwrap();
        assert!(engine.new_state().run(&module).is_err(), "{source}");
    }
    for source in [
        "Объект = Новый ФиксированнаяСтруктура(1, 2);",
        "Объект = Новый ФиксированноеСоответствие;",
    ] {
        assert!(engine.compile(source).is_err(), "{source}");
    }
}

#[test]
fn notification_accepts_only_the_measured_fixed_collection_methods() {
    run_modes(
        r#"
СтруктураПробы = Новый ФиксированнаяСтруктура;
КартаПробы = Новый Соответствие;
ФиксКартаПробы = Новый ФиксированноеСоответствие(КартаПробы);
Для Каждого ИмяМетода Из СтрРазделить("Количество,Count,Свойство,Property", ",") Цикл
    Описание = Новый NotifyDescription(ИмяМетода, СтруктураПробы);
КонецЦикла;
Для Каждого ИмяМетода Из СтрРазделить("Количество,Count,Получить,Get", ",") Цикл
    Описание = Новый NotifyDescription(ИмяМетода, ФиксКартаПробы);
КонецЦикла;
"#,
    );
}
