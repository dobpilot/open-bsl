use open_bsl::{Engine, Error, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, Value};

fn recipe(visibility: ModuleVisibility) -> ModuleGraphRecipe {
    ModuleGraphRecipe {
        modules: vec![
            ModuleRecipe {
                name: "Дочерний".into(),
                source: include_str!(
                    "../../../docs/research/fixtures/deferred-common-function-errors/ChildModule.bsl"
                )
                .into(),
                imports: vec![],
                visibility,
            },
            ModuleRecipe {
                name: "Родитель".into(),
                source: "Функция Исправный() Экспорт Возврат 42; КонецФункции
                    Функция Дочерний() Экспорт Возврат Дочерний.Исправный(); КонецФункции"
                    .into(),
                imports: vec![],
                visibility: ModuleVisibility::Named,
            },
        ],
        eager_init: false,
    }
}

#[test]
fn unknown_global_function_matches_native_unused_and_accessed_child_behavior() {
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
                "Попытка Возврат Родитель.Дочерний();
                 Исключение Возврат 99; КонецПопытки;",
            )
            .unwrap();
        let mut state = engine.new_state();
        assert_eq!(state.run(&good).unwrap(), Value::number_from_i64(42));
        assert!(matches!(
            state.run(&bad),
            Err(Error::Runtime(RtError::ModuleCompilation(ref message)))
                if message.contains("СовсемНеизвестнаяФункция")
        ));
    }
}

#[test]
fn unknown_global_function_keeps_default_and_private_compilation_strict() {
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
