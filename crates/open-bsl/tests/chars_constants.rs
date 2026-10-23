use open_bsl::{Engine, Value};

#[test]
fn all_character_constants_match_the_native_codes_through_both_spellings() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english, code) in [
        ("ВК", "CR", 13),
        ("ПС", "LF", 10),
        ("ПФ", "FF", 12),
        ("Таб", "Tab", 9),
        ("ВТаб", "VTab", 11),
        ("НПП", "NBSp", 160),
    ] {
        let source = format!(
            "Возврат Символы.{russian} = Chars.{english} \
             И СтрДлина(Символы.{russian}) = 1 \
             И КодСимвола(Chars.{english}) = {code};"
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::Boolean(true)
        );
    }
}

#[test]
fn local_and_module_variables_shadow_both_character_constant_names() {
    let engine = Engine::builder().build().unwrap();
    for (name, field) in [("Символы", "ПС"), ("Chars", "LF")] {
        let source = format!(
            r#"
            Перем {name};
            Функция ЛокальнаяПроба({name})
                Возврат {name}.{field};
            КонецФункции
            {name} = Новый Структура("{field}", "module");
            Возврат {name}.{field} = "module"
                И ЛокальнаяПроба(Новый Структура("{field}", "local")) = "local";
            "#
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::Boolean(true)
        );
    }
}
