use open_bsl::{Engine, Value};

#[test]
fn static_calls_preserve_imported_arguments_and_initialize_their_owners() {
    let store = "Перем Ячейка Экспорт; Ячейка = 1;";
    let source = r#"
Процедура Записать(Первый, Второй)
    Если Первый <> 1 Тогда ВызватьИсключение "lazy initialization"; КонецЕсли;
    Первый = 10;
    Если Второй <> 10 Или Хранилище.Ячейка <> 10 Тогда ВызватьИсключение "shared cell"; КонецЕсли;
    Хранилище.Ячейка = 20;
    Если Первый <> 20 Или Второй <> 20 Тогда ВызватьИсключение "module write"; КонецЕсли;
    Вложенный(Хранилище.Ячейка);
    Если Первый <> 21 Или Второй <> 21 Тогда ВызватьИсключение "nested cell"; КонецЕсли;
КонецПроцедуры
Процедура Вложенный(Аргумент)
    Аргумент = Аргумент + 1;
КонецПроцедуры
Процедура Сбой(Аргумент, Знач Копия, Добавка = 7)
    Аргумент = Аргумент + Добавка;
    Копия = -1;
    ВызватьИсключение "ожидаемая ошибка";
КонецПроцедуры
Функция Запуск() Экспорт
    Записать(Хранилище.Ячейка, Хранилище.Ячейка);
    Попытка Сбой(Хранилище.Ячейка, Хранилище.Ячейка, ); Исключение КонецПопытки;
    Возврат Хранилище.Ячейка;
КонецФункции
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        for eager_init in [false, true] {
            let engine = Engine::builder()
                .optimizations(optimizations)
                .configuration(open_bsl::ModuleGraphRecipe {
                    modules: vec![
                        open_bsl::ModuleRecipe {
                            name: "Хранилище".into(),
                            source: store.into(),
                            imports: vec![],
                        },
                        open_bsl::ModuleRecipe {
                            name: "Работа".into(),
                            source: source.into(),
                            imports: vec![("Хранилище".into(), "Хранилище".into())],
                        },
                    ],
                    eager_init,
                })
                .build()
                .unwrap();
            for entry in [
                format!("{source}\nВозврат Запуск();"),
                "Возврат Работа.Запуск();".into(),
            ] {
                let compiled = engine.compile_entry(&entry).unwrap();
                for bytecode in [false, true] {
                    let module = if bytecode {
                        engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                    } else {
                        engine.compile_entry(&entry).unwrap()
                    };
                    assert_eq!(
                        engine.new_state().run(&module).unwrap(),
                        Value::number_from_i64(28)
                    );
                }
            }
        }
    }
}

#[test]
fn static_imported_async_arguments_keep_the_existing_snapshot_contract() {
    let source = r#"
Перем ОбещаниеРаботы;
Асинх Функция Снимок(Первый, Второй)
    Если Первый <> 7 Или Второй <> 7 Тогда ВызватьИсключение "active alias snapshot"; КонецЕсли;
    Первый = 9;
    Если Второй <> 7 Или Хранилище.Ячейка <> 7 Тогда ВызватьИсключение "private async parameters"; КонецЕсли;
    Хранилище.Ячейка = 20;
    Ждать 0;
    Возврат Первый + Второй;
КонецФункции
Процедура Мост(Аргумент)
    Если Аргумент <> 1 Тогда ВызватьИсключение "lazy async owner"; КонецЕсли;
    Аргумент = 7;
    ОбещаниеРаботы = Снимок(Хранилище.Ячейка, Хранилище.Ячейка);
    Если Аргумент <> 20 Тогда ВызватьИсключение "async module write"; КонецЕсли;
КонецПроцедуры
Асинх Процедура Проверить()
    Мост(Хранилище.Ячейка);
    Результат = Ждать ОбещаниеРаботы;
    Если Результат <> 16 Или Хранилище.Ячейка <> 20 Тогда ВызватьИсключение "async result"; КонецЕсли;
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
            .common_module("Хранилище", "Перем Ячейка Экспорт; Ячейка = 1;")
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
                Value::number_from_i64(1)
            );
        }
    }
}

#[test]
fn failed_imported_argument_initialization_does_not_enter_the_static_callee() {
    let source = r#"
Перем Вызовов;
Процедура Записать(Первый, Второй)
    Вызовов = Вызовов + 1;
    Первый = 100;
КонецПроцедуры
Вызовов = 0;
Ошибок = 0;
Для НомерПопытки = 1 По 2 Цикл
    Попытка
        Записать(ПервыйМодуль.Ячейка, ВторойМодуль.Ячейка);
    Исключение
        Ошибок = Ошибок + 1;
    КонецПопытки;
КонецЦикла;
Если Вызовов <> 0 Или Ошибок <> 2 Тогда ВызватьИсключение "callee before initialization"; КонецЕсли;
Возврат ПервыйМодуль.Ячейка;
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("ПервыйМодуль", "Перем Ячейка Экспорт; Ячейка = 1;")
            .common_module(
                "ВторойМодуль",
                "Перем Ячейка Экспорт; ВызватьИсключение \"ошибка инициализации\";",
            )
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
                Value::number_from_i64(1)
            );
        }
    }
}

#[test]
fn imported_variables_and_nested_calls_share_the_same_parameter_cell() {
    let store = "Перем Ячейка Экспорт; Процедура Увеличить(Аргумент) Экспорт Аргумент = Аргумент + 1; КонецПроцедуры Ячейка = 1;";
    let worker = r#"
Функция Ссылка() Экспорт Возврат ЭтотОбъект; КонецФункции
Процедура Проверить(Первый, Второй) Экспорт
    Первый = 10;
    Если Второй <> 10 Или Хранилище.Ячейка <> 10 Тогда ВызватьИсключение "read"; КонецЕсли;
    Хранилище.Ячейка = 20;
    Если Первый <> 20 Или Второй <> 20 Тогда ВызватьИсключение "write"; КонецЕсли;
    Хранилище.Увеличить(Хранилище.Ячейка);
    Если Первый <> 21 Или Второй <> 21 Тогда ВызватьИсключение "nested"; КонецЕсли;
    Второй = 42;
КонецПроцедуры
"#;
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .configuration(open_bsl::ModuleGraphRecipe {
                modules: vec![
                    open_bsl::ModuleRecipe {
                        name: "Хранилище".into(),
                        source: store.into(),
                        imports: vec![],
                    },
                    open_bsl::ModuleRecipe {
                        name: "Работа".into(),
                        source: worker.into(),
                        imports: vec![("Хранилище".into(), "Хранилище".into())],
                    },
                ],
                eager_init: false,
            })
            .build()
            .unwrap();
        for receiver in ["Работа", "СсылкаРаботы"] {
            let source = format!(
                "СсылкаРаботы = Работа.Ссылка(); {receiver}.Проверить(Хранилище.Ячейка, Хранилище.Ячейка); Возврат Хранилище.Ячейка;"
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
                    Value::number_from_i64(42)
                );
            }
        }
    }
}

#[test]
fn an_active_module_argument_observes_async_tasks_and_keeps_its_writes() {
    let source = r#"
Перем Общая, ОбещаниеРаботы;
Асинх Функция Ребенок() Экспорт
    Если Общая <> 10 Тогда ВызватьИсключение "child module read"; КонецЕсли;
    Общая = 20;
    Ждать 0;
    Если Общая <> 21 Тогда ВызватьИсключение "parent write"; КонецЕсли;
    Общая = 30;
    Возврат 0;
КонецФункции
Процедура Мост(Аргумент)
    Аргумент = 10;
    ОбещаниеРаботы = ЭтотОбъект.Ребенок();
    Если Аргумент <> 20 Тогда ВызватьИсключение "child write"; КонецЕсли;
    Аргумент = 21;
КонецПроцедуры
Асинх Процедура Проверить()
    Мост(Общая);
    Ждать ОбещаниеРаботы;
    Если Общая <> 30 Тогда ВызватьИсключение "final module"; КонецЕсли;
КонецПроцедуры
Общая = 1;
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
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
            } else {
                engine.compile(source).unwrap()
            };
            assert_eq!(
                engine.new_state().run(&module).unwrap(),
                Value::number_from_i64(1)
            );
        }
    }
}
