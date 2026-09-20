use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, Value};

#[test]
fn dynamic_imports_keep_aliases_visibility_arguments_and_nested_scopes() {
    let store = r#"
Перем Ячейка Экспорт;
Функция Прибавить(Первый, Второй = 2) Экспорт
    Первый = Первый + Второй;
    Возврат Первый;
КонецФункции
Функция Закрытая()
    Возврат 999;
КонецФункции
Асинх Функция Позже() Экспорт
    Ждать 0;
    Возврат Ячейка;
КонецФункции
Функция Обещание() Экспорт
    Возврат ЭтотОбъект.Позже();
КонецФункции
Ячейка = 10;
"#;
    let worker = r#"
Функция Читает()
    Возврат Вычислить("Данные.Ячейка");
КонецФункции
Функция Проверить() Экспорт
    Если Вычислить("Данные.Ячейка") <> 10 Тогда ВызватьИсключение "read"; КонецЕсли;
    Если Читает() <> 10 Или Вычислить("Читает()") <> 10 Тогда ВызватьИсключение "static function in dynamic"; КонецЕсли;
    Локальная = 3;
    Если Вычислить("Данные.Прибавить(Локальная)") <> 5 Или Локальная <> 5 Тогда ВызватьИсключение "byref/default"; КонецЕсли;
    Если Вычислить("Данные.Прибавить(Данные.Ячейка, 4)") <> 14 Тогда ВызватьИсключение "imported argument"; КонецЕсли;
    Попытка Выполнить("Данные.Ячейка = 21; ВызватьИсключение ""expected"";"); Исключение КонецПопытки;
    Если Вычислить("Вычислить(""Данные.Ячейка"")") <> 21 Тогда ВызватьИсключение "nested/exception"; КонецЕсли;
    Если Вычислить("Читает()") <> 21 Тогда ВызватьИсключение "cached links"; КонецЕсли;
    Отказов = 0;
    Попытка Ненужная = Вычислить("Хранилище.Ячейка"); Исключение Отказов = Отказов + 1; КонецПопытки;
    Попытка Ненужная = Вычислить("Посторонний.Прочитать()"); Исключение Отказов = Отказов + 1; КонецПопытки;
    Попытка Ненужная = Вычислить("Данные.Закрытая()"); Исключение Отказов = Отказов + 1; КонецПопытки;
    Если Отказов <> 3 Тогда ВызватьИсключение "visibility"; КонецЕсли;
    // Прямой async CallImported намеренно закрыт прежним JOB.ASYNC.TARGET.
    // Синхронная экспортная функция может вернуть обычное BSL-обещание.
    Попытка Ненужная = Вычислить("Данные.Позже()"); Исключение Отказов = Отказов + 1; КонецПопытки;
    Если Отказов <> 4 Тогда ВызватьИсключение "async target restriction"; КонецЕсли;
    Возврат Вычислить("Данные.Обещание()");
КонецФункции
Функция Затенить(Данные) Экспорт
    Возврат Вычислить("Данные.Ячейка");
КонецФункции
"#;
    let entry = r#"
Асинх Процедура ПроверитьВсе()
    ОбещаниеПробы = Вычислить("Работа.Проверить()");
    Если Ждать ОбещаниеПробы <> 21 Тогда ВызватьИсключение "async"; КонецЕсли;
    Если Работа.Затенить(Новый Структура("Ячейка", 71)) <> 71 Тогда ВызватьИсключение "shadowing"; КонецЕсли;
    Если Затенение.Прочитать() <> 72 Тогда ВызватьИсключение "module shadowing"; КонецЕсли;
    Если Смешанный.Прочитать() <> 42 Или Смешанный.Прочитать() <> 42 Тогда ВызватьИсключение "static/dynamic link tables"; КонецЕсли;
КонецПроцедуры
ПроверитьВсе();
Возврат 1;
"#;
    for optimized in [false, true] {
        for eager_init in [false, true] {
            let engine = Engine::builder()
                .optimizations(if optimized {
                    bsl_compiler::Optimizations::all()
                } else {
                    bsl_compiler::Optimizations::default()
                })
                .configuration(ModuleGraphRecipe {
                    modules: vec![
                        ModuleRecipe {
                            name: "Хранилище".into(),
                            source: store.into(),
                            imports: vec![],
                        },
                        ModuleRecipe {
                            name: "Работа".into(),
                            source: worker.into(),
                            imports: vec![("Данные".into(), "Хранилище".into())],
                        },
                        ModuleRecipe {
                            name: "Посторонний".into(),
                            source: "Функция Прочитать() Экспорт Возврат 999; КонецФункции".into(),
                            imports: vec![],
                        },
                        ModuleRecipe {
                            name: "Затенение".into(),
                            source: "Перем Данные; Функция Прочитать() Экспорт Возврат Вычислить(\"Данные.Ячейка\"); КонецФункции Данные = Новый Структура(\"Ячейка\", 72);".into(),
                            imports: vec![("Данные".into(), "Хранилище".into())],
                        },
                        ModuleRecipe {
                            name: "Смешанный".into(),
                            source: "Функция Статическая() Возврат Д.Ячейка; КонецФункции Функция Прочитать() Экспорт Возврат Вычислить(\"Статическая() + Д.Ячейка\"); КонецФункции".into(),
                            imports: vec![("Д".into(), "Хранилище".into())],
                        },
                    ],
                    eager_init,
                })
                .build()
                .unwrap();
            let module = engine.compile_entry(entry).unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
                } else {
                    module.clone()
                };
                let mut state = engine.new_state();
                for _ in 0..2 {
                    assert_eq!(state.run(&module).unwrap(), Value::number_from_i64(1));
                }
            }
            let image = engine.image_bytecode(&module, None).unwrap();
            let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } =
                bsl_bytecode::parse_image(&image).unwrap()
            else {
                panic!("ожидался каталог");
            };
            assert_eq!(catalog.modules[1].program.imports[0].alias, "Данные");
            assert!(
                catalog.modules[1].program.links.is_empty(),
                "импорт используется только в строках"
            );
            let loaded = Engine::builder()
                .configuration_image(catalog, eager_init)
                .build()
                .unwrap();
            let loaded_module = loaded.load_entry(entry.unwrap()).unwrap();
            assert_eq!(
                loaded.new_state().run(&loaded_module).unwrap(),
                Value::number_from_i64(1)
            );
        }
    }
}

#[test]
fn unused_declared_imports_keep_eager_initialization_order_after_loading() {
    #[derive(Clone, Default)]
    struct Output(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "Первый".into(),
                    source: "Перем Результат Экспорт; Сообщить(\"первый\"); Результат = 42;".into(),
                    imports: vec![("Зависимость".into(), "Второй".into())],
                },
                ModuleRecipe {
                    name: "Второй".into(),
                    source: "Сообщить(\"второй\");".into(),
                    imports: vec![],
                },
            ],
            eager_init: true,
        })
        .build()
        .unwrap();
    let module = engine.compile_entry("Возврат Первый.Результат;").unwrap();
    let image = engine.image_bytecode(&module, None).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } =
        bsl_bytecode::parse_image(&image).unwrap()
    else {
        panic!("каталог");
    };
    assert!(catalog.modules[0].program.links.is_empty());
    let loaded = Engine::builder()
        .configuration_image(catalog, true)
        .build()
        .unwrap();
    let output = Output::default();
    assert_eq!(
        loaded
            .state_builder()
            .stdout(output.clone())
            .build()
            .run(&loaded.load_entry(entry.unwrap()).unwrap())
            .unwrap(),
        Value::number_from_i64(42)
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "второй\nпервый\n"
    );
}
