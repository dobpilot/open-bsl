use open_bsl::{Engine, Value};

#[test]
fn error_display_variants_preserve_native_presentation_whitespace() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english, presentation) in [
        ("Авто", "Auto", "Auto"),
        (
            "СообщениеОбОшибкеДляПользователя",
            "ErrorMessageForUser",
            "Message about error for user ",
        ),
        (
            "КраткоеПредставлениеОшибки",
            "BriefErrorDescription",
            "Short error presentation ",
        ),
        (
            "ПодробноеПредставлениеОшибки",
            "DetailErrorDescription",
            "Detailed error presentation ",
        ),
    ] {
        let source = format!(
            r#"Значение = ВариантОтображенияСообщенияОбОшибке.{russian};
            Возврат Значение = ErrorMessageDisplayVariant.{english}
                И Строка(Значение) = "{presentation}"
                И Строка(ТипЗнч(Значение)) = "ErrorMessageDisplayVariant";"#
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::Boolean(true)
        );
    }
}
