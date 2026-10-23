use open_bsl::{Engine, Value};

#[test]
fn formatted_document_resolves_without_inventing_a_value() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile(
            "Процедура НеВызывается()\nД = Новый ФорматированныйДокумент;\nКонецПроцедуры\nВозврат 1;",
        )
        .unwrap();
    engine.compile("Д = Новый FormattedDocument();").unwrap();
    assert!(
        engine
            .compile("Д = Новый ФорматированныйДокумент(1);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nНовый ФорматированныйДокумент();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
