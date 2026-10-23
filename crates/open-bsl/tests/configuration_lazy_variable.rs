use open_bsl::{Engine, Value};

const SOURCE: &str = r#"
Функция Исправный() Экспорт
    Возврат 7;
КонецФункции

Функция Ошибочный() Экспорт
    Для Каждого Элемент Из Элемент Цикл
    КонецЦикла;
    Возврат 0;
КонецФункции
"#;

#[test]
fn self_referential_foreach_in_configuration_method_is_catchable_on_call() {
    let engine = Engine::builder()
        .common_module("Служебный", SOURCE)
        .configuration_roots(["Служебный"])
        .build()
        .unwrap();
    let good = engine
        .compile_entry("Возврат Служебный.Исправный();")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&good).unwrap(),
        Value::number_from_i64(7)
    );

    let bad = engine
        .compile_entry(
            "Попытка\nСлужебный.Ошибочный();\nВозврат 0;\nИсключение\nВозврат 1;\nКонецПопытки;",
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&bad).unwrap(),
        Value::number_from_i64(1)
    );

    let standalone = Engine::builder().build().unwrap();
    assert!(standalone.compile(SOURCE).is_err());
    assert!(
        Engine::builder()
            .common_module("Строгий", "Возврат НетТакойПеременной;")
            .configuration_roots(["Строгий"])
            .build()
            .is_err()
    );
}
