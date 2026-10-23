use open_bsl::{Engine, Value};

#[test]
fn bin_dir_resolves_without_claiming_the_interpreter_directory() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Возврат КаталогПрограммы(1);").is_err());
    assert_eq!(
        engine
            .new_state()
            .exec(
                r#"
Попытка
    Р = BinDir();
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
