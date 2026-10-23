use open_bsl::{Engine, Value};

#[test]
fn style_colors_resolves_but_does_not_fabricate_colors() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Значение = ЦветаСтиля;").unwrap();
    engine.compile("Значение = StyleColors;").unwrap();

    let result = engine
        .new_state()
        .exec("Попытка\nЗначение = ЦветаСтиля;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;")
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn style_fonts_resolves_but_does_not_fabricate_fonts() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Значение = ШрифтыСтиля;").unwrap();
    engine.compile("Значение = StyleFonts;").unwrap();

    let result = engine
        .new_state()
        .exec("Попытка\nЗначение = ШрифтыСтиля;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;")
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn web_colors_resolves_but_does_not_fabricate_colors() {
    let engine = Engine::builder().build().unwrap();
    engine.compile("Значение = WebЦвета.Зеленый;").unwrap();
    engine.compile("Значение = WebColors.Green;").unwrap();

    let result = engine
        .new_state()
        .exec("Попытка\nЗначение = WebЦвета.Зеленый;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;")
        .unwrap();
    assert_eq!(result, Value::Boolean(true));

    assert_eq!(
        engine
            .new_state()
            .exec("Перем WebЦвета; WebЦвета = 17; Возврат WebЦвета;")
            .unwrap(),
        Value::Number(open_bsl::BslNumber::from_i64(17))
    );
}
