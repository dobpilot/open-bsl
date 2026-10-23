use open_bsl::{Engine, Error, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, RtError, Value};

#[test]
fn reached_server_constructor_accepts_aliases_and_rejects_unconnected_execution() {
    let engine = Engine::builder()
        .register_library(bsl_session::library())
        .build()
        .unwrap();
    for name in [
        "АдминистрированиеСервера",
        "ServerAdministration",
        "serveradministration",
    ] {
        for arguments in ["", "\"127.0.0.1\"", "\"127.0.0.1\", 1545"] {
            let module = engine
                .compile(&format!("Возврат Новый {name}({arguments});"))
                .unwrap();
            let mut state = engine.new_state();
            assert!(
                matches!(state.run(&module), Err(Error::Runtime(RtError::DynamicError(message)))
                if message == "транспорт администрирования сервера ras не подключён")
            );
        }
        assert!(
            engine
                .compile(&format!("Возврат Новый {name}(\"127.0.0.1\", 1545, 1);"))
                .is_err()
        );
    }
}

#[test]
fn unused_ras_constructor_in_common_module_does_not_prevent_a_valid_export() {
    let engine = Engine::builder().register_library(bsl_session::library())
        .configuration(ModuleGraphRecipe {
            modules: vec![ModuleRecipe {
                name: "АдминистрированиеКластераRAS".into(),
                source: "Функция Исправный() Экспорт Возврат 42; КонецФункции Функция АгентСервера() Экспорт Возврат Новый АдминистрированиеСервера(); КонецФункции".into(),
                imports: vec![],
                visibility: ModuleVisibility::Named,
            }],
            eager_init: false,
        }).build().unwrap();
    let entry = engine
        .compile_entry("Возврат АдминистрированиеКластераRAS.Исправный();")
        .unwrap();
    let mut state = engine.new_state();
    assert_eq!(state.run(&entry).unwrap(), Value::number_from_i64(42));
    let reached = engine
        .compile_entry("Возврат АдминистрированиеКластераRAS.АгентСервера();")
        .unwrap();
    assert!(state.run(&reached).is_err());
}
