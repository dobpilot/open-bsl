use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, Value};

#[test]
fn static_common_module_arguments_and_assignments_use_existing_instances() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "Приемник".into(),
                    source: "Функция Принять(Знач Модуль) Экспорт Возврат Модуль.Метка(); КонецФункции Функция Метка() Экспорт Возврат 7; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Named,
                },
                ModuleRecipe {
                    name: "Проба".into(),
                    source: "Функция Метка() Экспорт Возврат 42; КонецФункции
                        Функция Запуск() Экспорт
                            Ссылка = Проба;
                            Другая = Приемник;
                            Возврат Приемник.Принять(Ссылка) + Другая.Принять(Приемник);
                        КонецФункции
                        Функция Перекрытие(Проба) Экспорт Возврат Проба; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Named,
                },
            ],
            eager_init: false,
        })
        .configuration_roots(["Проба"])
        .build()
        .unwrap();
    let entry = engine
        .compile_entry("Возврат Проба.Запуск() + Проба.Перекрытие(1);")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::number_from_i64(50)
    );
}

#[test]
fn a_static_module_value_retains_its_target_in_the_rooted_catalog() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "Корень".into(),
                    source: "Функция Ссылка() Экспорт Возврат Цель; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Named,
                },
                ModuleRecipe {
                    name: "Цель".into(),
                    source: "Функция Метка() Экспорт Возврат 42; КонецФункции".into(),
                    imports: vec![],
                    visibility: ModuleVisibility::Named,
                },
            ],
            eager_init: false,
        })
        .configuration_roots(["Корень"])
        .build()
        .unwrap();
    let entry = engine
        .compile_entry("Получатель = Корень.Ссылка(); Возврат Получатель.Метка();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&entry).unwrap(),
        Value::number_from_i64(42)
    );
}
