use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, Value};

#[test]
fn host_calls_a_private_handler_without_exposing_it_to_bsl() {
    let engine = Engine::builder()
        .configuration(ModuleGraphRecipe {
            modules: vec![ModuleRecipe {
                visibility: ModuleVisibility::Named,
                name: "Служебный".into(),
                source: "Перем Счетчик; Счетчик = 0; Процедура Обработчик(Значение) Счетчик = Счетчик + Значение; КонецПроцедуры Функция Прочитать() Экспорт Возврат Счетчик; КонецФункции".into(),
                imports: Vec::new(),
            }],
            eager_init: false,
        })
        .build()
        .unwrap();
    let mut state = engine.new_state();
    state
        .call_configuration_handler("сЛуЖеБнЫй", "оБрАбОтЧиК", vec![Value::number_from_i64(2)])
        .unwrap();
    assert_eq!(
        state
            .call_configuration_handler("Служебный", "Прочитать", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(2)
    );

    assert!(engine.compile_entry("Служебный.Обработчик(1);").is_err());
}
