use open_bsl::{Engine, Value};

#[test]
fn button_representation_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Возврат Строка(ТипЗнч(ОтображениеКнопки.Авто)) + ":"
    + Строка(ОтображениеКнопки.Авто) + ":"
    + Строка(ОтображениеКнопки.Картинка) + ":"
    + Строка(ОтображениеКнопки.КартинкаИТекст) + ":"
    + Строка(ОтображениеКнопки.Текст) + ":"
    + Строка(ОтображениеКнопки.КартинкаИТекст = ButtonRepresentation.PictureAndText);
"#,
        )
        .unwrap();
    assert_eq!(
        value,
        Value::Str("ОтображениеКнопки:Авто:Картинка:Картинка и текст:Текст:Да".into())
    );
}
