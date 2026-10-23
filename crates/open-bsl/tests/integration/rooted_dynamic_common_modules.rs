use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, Value};

#[test]
fn explicit_dynamic_roots_keep_visible_modules_and_exclude_unreached_private_managers() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "Корень".into(),
                    visibility: ModuleVisibility::Named,
                    imports: vec![],
                    source: "Функция Запуск() Экспорт Возврат Посредник.Вызвать(\"Цель\"); КонецФункции".into(),
                },
                ModuleRecipe {
                    name: "Посредник".into(),
                    visibility: ModuleVisibility::Named,
                    imports: vec![],
                    source: "Функция Вызвать(Имя) Экспорт Модуль = Вычислить(Имя); Возврат Модуль.Следующее(); КонецФункции".into(),
                },
                ModuleRecipe {
                    name: "Цель".into(),
                    visibility: ModuleVisibility::Named,
                    imports: vec![],
                    source: "Перем Счетчик; Функция Следующее() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Счетчик = 40;".into(),
                },
                ModuleRecipe {
                    name: "НеиспользуемыйМенеджер".into(),
                    visibility: ModuleVisibility::Private,
                    imports: vec![],
                    source: "Функция Ошибка() Экспорт Возврат НетПеременной; КонецФункции".into(),
                },
            ],
            eager_init: false,
        })
        .configuration_roots(["Корень", "Цель"])
        .build()
        .unwrap();
    let entry = engine
        .compile_entry("Возврат Корень.Запуск() + Корень.Запуск();")
        .unwrap();
    let mut state = engine.new_state();
    assert_eq!(state.run(&entry).unwrap(), Value::number_from_i64(83));
    let second = engine.new_state().run(&entry).unwrap();
    assert_eq!(second, Value::number_from_i64(83));
}
