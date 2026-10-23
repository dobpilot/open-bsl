use open_bsl::{Engine, Value};

fn text(value: Value) -> String {
    let Value::Str(value) = value else {
        panic!("ожидалась строка");
    };
    value.to_string()
}

#[test]
fn formatted_string_works_through_the_facade_in_both_spellings() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
Части = Новый Массив;
Части.Добавить("в");
Части.Добавить(New FormattedString("г"));
Значение = Новый ФорматированнаяСтрока("а", Новый ФорматированнаяСтрока("б"), Части);
Возврат Строка(Значение) + "|" + Строка(ТипЗнч(Значение)) + "|"
    + Строка(ЗначениеЗаполнено(Значение));
"#,
        )
        .unwrap();

    assert_eq!(text(result), "абвг|Форматированная строка|Да");
}

#[test]
fn formatted_string_preserves_formatting_for_equality() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
Первое = Новый ФорматированнаяСтрока("текст", , , , "первая");
Второе = Новый ФорматированнаяСтрока("текст", , , , "вторая");
Возврат Строка(Первое) + "|" + Строка(Первое = Второе);
"#,
        )
        .unwrap();

    assert_eq!(text(result), "текст|Нет");
}

#[test]
fn formatted_string_rejects_empty_and_unsupported_content() {
    let engine = Engine::builder().build().unwrap();
    assert!(
        engine
            .compile("Значение = Новый ФорматированнаяСтрока();")
            .is_err()
    );

    let result = engine
        .new_state()
        .exec(
            r#"
ОшибкаБыла = Ложь;
Попытка
    Значение = Новый ФорматированнаяСтрока(1);
Исключение
    ОшибкаБыла = Истина;
КонецПопытки;
Возврат ОшибкаБыла;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
