use open_bsl::{Engine, Value};

#[test]
fn string_declension_resolves_without_fabricating_forms() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Р = ПолучитьСклоненияСтроки();").is_err());
    assert!(
        engine
            .compile("Р = ПолучитьСклоненияСтроки(\"Иван\", \"\", \"\", \"\");")
            .is_err()
    );
    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Р = GetStringDeclensions("Иван", "ПЛ=Мужской", "ПД=Дательный");
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
