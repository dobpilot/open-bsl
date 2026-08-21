//! Счётчики секций листинга — тоже недостоверный вход.
//!
//! Каждый из них уходил прямо в `Vec::with_capacity`, поэтому
//! `.handlers 18446744073709551615` в правленом руками листинге валил
//! процесс с «capacity overflow» и кодом 101 — в debug и в release
//! одинаково, — вместо `TextError`.

use bsl_bytecode::{compile_program, parse_program, write_program};

fn listing() -> String {
    // Программа с непустыми секциями: константы, имена, обработчик,
    // объявленная функция и режимы аргументов.
    let src = concat!(
        "Функция Ф(а = 3, б = 7)\n",
        "Возврат а + б;\n",
        "КонецФункции\n",
        "Перем М;\n",
        "Попытка\n",
        "С = Новый Структура(\"Поле\", 1);\n",
        "Сообщить(С.Поле + Ф(1,));\n",
        "Исключение\n",
        "Сообщить(\"поймано\");\n",
        "КонецПопытки;\n",
    );
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    let program = compile_program(&resolved).expect("кодоген");
    write_program(&program, None).expect("печать")
}

#[test]
fn an_absurd_section_count_is_an_error_and_never_a_crash() {
    let text = listing();
    assert!(
        parse_program(&text).is_ok(),
        "целый листинг обязан читаться"
    );

    let sections = [
        ".requires",
        ".names",
        ".shapes",
        ".top-locals",
        ".module-vars",
        ".functions",
        ".consts",
        ".argmodes",
        ".handlers",
        ".localnames",
        ".code",
    ];
    let mut seen = 0;
    for section in sections {
        let mut broken = String::new();
        let mut hit = false;
        for line in text.lines() {
            let head = line.trim_start();
            if !hit && head.starts_with(section) && head[section.len()..].starts_with(' ') {
                let indent = &line[..line.len() - head.len()];
                broken.push_str(indent);
                broken.push_str(section);
                broken.push_str(" 18446744073709551615");
                hit = true;
            } else {
                broken.push_str(line);
            }
            broken.push('\n');
        }
        if !hit {
            continue;
        }
        seen += 1;
        assert!(
            parse_program(&broken).is_err(),
            "{section} с невозможным счётчиком обязан быть ошибкой"
        );
    }
    assert!(
        seen >= 8,
        "проверено секций: {seen}, ожидалось не меньше восьми"
    );
}
