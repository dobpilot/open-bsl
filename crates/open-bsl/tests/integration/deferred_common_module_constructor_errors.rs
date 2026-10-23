use open_bsl::{Engine, Error, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, Value};

fn recipe(visibility: ModuleVisibility) -> ModuleGraphRecipe {
    ModuleGraphRecipe {
        modules: vec![
            ModuleRecipe {
                name: "Дочерний".into(),
                source: "Функция Исправный() Экспорт Возврат 42; КонецФункции
                    Функция Ошибка() Экспорт Возврат Новый НеСуществующийТип(); КонецФункции"
                    .into(),
                imports: vec![],
                visibility,
            },
            ModuleRecipe {
                name: "Родитель".into(),
                source: "Функция Исправный() Экспорт Возврат 7; КонецФункции".into(),
                imports: vec![],
                visibility: ModuleVisibility::Named,
            },
        ],
        eager_init: false,
    }
}

#[test]
fn unsupported_constructor_does_not_prevent_unused_common_module_startup() {
    for visibility in [ModuleVisibility::Named, ModuleVisibility::Global] {
        let engine = Engine::builder()
            .defer_common_module_name_errors(true)
            .configuration(recipe(visibility))
            .build()
            .unwrap();
        let good = engine
            .compile_entry("Возврат Родитель.Исправный();")
            .unwrap();
        let bad = engine
            .compile_entry(
                "Попытка Возврат Дочерний.Исправный(); Исключение Возврат 99; КонецПопытки;",
            )
            .unwrap();
        let mut state = engine.new_state();
        assert_eq!(state.run(&good).unwrap(), Value::number_from_i64(7));
        assert!(
            matches!(state.run(&bad), Err(Error::Runtime(RtError::ModuleCompilation(ref message)))
            if message.contains("НеСуществующийТип"))
        );
    }
}

#[test]
fn unsupported_constructor_keeps_default_and_non_common_modules_strict() {
    assert!(
        Engine::builder()
            .configuration(recipe(ModuleVisibility::Named))
            .build()
            .is_err()
    );
    for visibility in [ModuleVisibility::Private, ModuleVisibility::Explicit] {
        assert!(
            Engine::builder()
                .defer_common_module_name_errors(true)
                .configuration(recipe(visibility))
                .build()
                .is_err()
        );
    }
}
