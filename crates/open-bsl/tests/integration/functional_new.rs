use open_bsl::{Engine, Value};

fn text(value: Value) -> String {
    let Value::Str(value) = value else {
        panic!("ожидалась строка");
    };
    value.to_string()
}

#[test]
fn functional_new_accepts_type_name_and_parameter_array() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let result = engine
            .new_state()
            .exec(
                r#"
Перем Порядок;
Функция ТипПробы()
    Порядок = Порядок + "1";
    Возврат Тип("Массив");
КонецФункции
Функция ПараметрыПробы()
    Порядок = Порядок + "2";
    ПараметрыПробы = Новый Массив;
    ПараметрыПробы.Добавить(2);
    Возврат ПараметрыПробы;
КонецФункции

Порядок = "";
ЧерезТип = Новый(ТипПробы(), ПараметрыПробы());
ЧерезСтроку = Новый("мАсСиВ", 42);
Возврат Порядок + "|" + Строка(ЧерезТип.Количество()) + "|"
    + Строка(ЧерезСтроку.Количество());
"#,
            )
            .unwrap();
        assert_eq!(text(result), "12|2|0");
    }
}

#[test]
fn functional_new_uses_registered_component_constructor() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
Значение = Новый(Тип("Файл"));
Возврат Строка(ТипЗнч(Значение));
"#,
        )
        .unwrap();
    assert_eq!(text(result), "Файл");
}

#[test]
fn functional_new_errors_are_catchable() {
    let engine = Engine::builder().build().unwrap();
    let result = engine
        .new_state()
        .exec(
            r#"
Ошибок = 0;
Попытка
    Значение = Новый(42);
Исключение
    Ошибок = Ошибок + 1;
КонецПопытки;
Попытка
    Значение = Новый(Тип("Число"));
Исключение
    Ошибок = Ошибок + 1;
КонецПопытки;
Возврат Строка(Ошибок);
"#,
        )
        .unwrap();
    assert_eq!(text(result), "2");
}
