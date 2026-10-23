use open_bsl::{Engine, Value};

#[test]
fn number_based_declension_uses_measured_arity_without_fabricating_forms() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Возврат ПолучитьСклоненияСтрокиПоЧислу(\"товар\");")
            .is_err()
    );
    assert!(
        engine
            .compile(
                "Возврат ПолучитьСклоненияСтрокиПоЧислу(\"товар\", 2, \"\", \"\", \"\", 0, 0);"
            )
            .is_err()
    );
    for source in [
        "Возврат ПолучитьСклоненияСтрокиПоЧислу(\"товар\", 2);",
        "Возврат ПолучитьСклоненияСтрокиПоЧислу(\"товар\", 2, \"\", \"Л=ru_RU\", \"\");",
        "Возврат GetStringDeclensionsByNumber(\"товар\", 2, \"\", \"Л=ru_RU\", \"\", 0);",
    ] {
        assert!(engine.compile(source).is_ok(), "{source}");
        let result = engine
            .new_state()
            .exec(&format!(
                "Попытка\n{source}\nИсключение\nВозврат Истина;\nКонецПопытки;"
            ))
            .unwrap();
        assert_eq!(result, Value::Boolean(true));
    }
}
