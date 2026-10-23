use open_bsl::{Engine, Value};

#[test]
fn empty_type_description_uses_the_measured_zero_argument_form() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Русский = Новый ОписаниеТипов();
Английский = New TypeDescription();
Возврат ТипЗнч(Русский) = Тип("ОписаниеТипов")
    И ТипЗнч(Русский) = ТипЗнч(Английский);
"#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}
