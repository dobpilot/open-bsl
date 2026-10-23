use open_bsl::{Engine, Value};

#[test]
fn error_display_variants_preserve_native_presentation_whitespace() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english, presentation) in [
        ("Авто", "Auto", "Авто"),
        (
            "СообщениеОбОшибкеДляПользователя",
            "ErrorMessageForUser",
            "Сообщение об ошибке для пользователя ",
        ),
        (
            "КраткоеПредставлениеОшибки",
            "BriefErrorDescription",
            "Краткое представление ошибки ",
        ),
        (
            "ПодробноеПредставлениеОшибки",
            "DetailErrorDescription",
            "Подробное представление ошибки ",
        ),
    ] {
        let source = format!(
            r#"Значение = ВариантОтображенияСообщенияОбОшибке.{russian};
            Возврат Значение = ErrorMessageDisplayVariant.{english}
                И Строка(Значение) = "{presentation}"
                И Строка(ТипЗнч(Значение)) = "ВариантОтображенияСообщенияОбОшибке";"#
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::Boolean(true)
        );
    }
}
