use open_bsl::{Engine, Value};

#[test]
fn date_only_point_has_native_value_equality_and_compare() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    let result = state.exec("Первый = Новый МоментВремени(Дата(2024,1,2)); Второй = Новый PointInTime(\"20240102\", Неопределено); Поздний = Новый МоментВремени(Дата(2024,1,3)); Возврат Первый = Второй И Первый.Сравнить(Второй) = 0 И Первый.Compare(Поздний) = -1 И Поздний.Сравнить(Первый) = 1 И Первый.Дата = Дата(2024,1,2) И Первый.Ссылка = Неопределено И Строка(ТипЗнч(Первый)) = \"Момент времени\";").unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn point_properties_are_read_only_and_less_than_is_not_a_primitive_comparison() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for code in [
        "МоментПробы.Дата = Дата(2025,1,1);",
        "МоментПробы.Ссылка = Неопределено;",
        "Результат = МоментПробы < МоментПробы;",
    ] {
        let source = format!(
            "МоментПробы = Новый МоментВремени(Дата(2024,1,2)); Попытка {code} Исключение Возврат МоментПробы.Дата = Дата(2024,1,2); КонецПопытки; Возврат Ложь;"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
}
