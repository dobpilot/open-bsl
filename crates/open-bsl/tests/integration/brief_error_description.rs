use open_bsl::{Engine, Value};

#[test]
fn brief_error_description_reads_the_saved_error_snapshot() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Попытка
    ВызватьИсключение "ошибка для пробы";
Исключение
    Ошибка = ИнформацияОбОшибке();
КонецПопытки;
Возврат КраткоеПредставлениеОшибки(Ошибка) = BriefErrorDescription(Ошибка)
    И КраткоеПредставлениеОшибки(Ошибка) = Ошибка.Описание;
"#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
    assert!(
        engine
            .new_state()
            .exec("Возврат КраткоеПредставлениеОшибки(Неопределено);")
            .is_err()
    );
}
