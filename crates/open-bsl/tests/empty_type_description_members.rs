use open_bsl::{Engine, Value};

#[test]
fn zero_argument_type_descriptions_have_no_allowed_types_in_either_language() {
    let engine = Engine::builder().build().unwrap();
    let actual = engine
        .new_state()
        .exec(
            r#"
Русский = Новый ОписаниеТипов();
Английский = New TypeDescription();
Возврат Русский.Типы().Количество() = 0
    И Английский.Types().Count() = 0
    И Не Русский.СодержитТип(Тип("Строка"))
    И Не Английский.ContainsType(Type("String"));
"#,
        )
        .unwrap();
    assert_eq!(actual, Value::Boolean(true));
}
