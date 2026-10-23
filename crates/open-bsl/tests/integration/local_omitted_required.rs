use open_bsl::{Engine, Value};

#[test]
fn explicit_local_omissions_preserve_required_and_defaulted_positions() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
Функция Проба(Первый, Второй = 2, Третий)
    Возврат Первый = Неопределено И Второй = 2 И Третий = Неопределено;
КонецФункции

Возврат Проба(, , );
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn absent_required_trailing_position_still_fails_resolution() {
    let engine = Engine::builder().build().unwrap();
    let error = engine
        .compile("Функция Проба(Первый, Второй) Возврат Первый; КонецФункции\nПроба(1);")
        .err()
        .expect("обязательный второй параметр не передан");
    assert!(
        error
            .to_string()
            .contains("пропущен обязательный аргумент на позиции 2")
    );
}
