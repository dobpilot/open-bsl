use open_bsl::{Engine, Value};

#[test]
fn unary_plus_preserves_numbers_checks_types_and_evaluates_once() {
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
Перем Вызовы;

Функция ЧислоСПобочнымЭффектом()
    Вызовы = Вызовы + 1;
    Возврат 7;
КонецФункции

Вызовы = 0;
Значение = +ЧислоСПобочнымЭффектом();
Ошибок = 0;
Попытка
    СтроковоеЗначение = +"текст";
Исключение
    Ошибок = Ошибок + 1;
КонецПопытки;
Возврат Строка(Значение) + "|" + Строка(Вызовы) + "|"
    + Строка(Ошибок) + "|" + Строка(+-5 * 2);
"#,
            )
            .unwrap();
        let Value::Str(result) = result else {
            panic!("ожидалась строка");
        };
        assert_eq!(result.to_string(), "7|1|1|-10");
    }
}
