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

/// Все секции листинга, несущие счётчик. Список закрыт: если появится
/// двенадцатая, тест обязан упасть на её отсутствии в образце, а не
/// молча её пропустить.
const COUNTED_SECTIONS: [&str; 11] = [
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

/// Заменяет счётчик первой найденной секции `section` на невозможный.
/// `None` — такой секции в листинге нет.
fn with_absurd_count(text: &str, section: &str) -> Option<String> {
    let mut out = String::new();
    let mut hit = false;
    for line in text.lines() {
        let head = line.trim_start();
        if !hit && head.starts_with(section) && head[section.len()..].starts_with(' ') {
            out.push_str(&line[..line.len() - head.len()]);
            out.push_str(section);
            out.push_str(" 18446744073709551615");
            hit = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    hit.then_some(out)
}

#[test]
fn an_absurd_count_is_caught_by_the_counter_check_in_every_section() {
    let text = listing();
    assert!(
        parse_program(&text).is_ok(),
        "целый листинг обязан читаться"
    );

    for section in COUNTED_SECTIONS {
        let broken = with_absurd_count(&text, section)
            .unwrap_or_else(|| panic!("в образце нет секции {section}"));
        let error = parse_program(&broken)
            .err()
            .unwrap_or_else(|| panic!("{section} с невозможным счётчиком обязан быть ошибкой"));

        // Именно РАННЯЯ проверка счётчика, а не случайный сбой дальше по
        // разбору: у `.shapes`, например, нет `Vec::with_capacity`, и без
        // этой привязки тест прошёл бы, ничего не доказав — парсер всё
        // равно споткнулся бы позже, приняв следующую директиву за запись.
        let text = error.to_string();
        assert!(
            text.contains("строк осталось"),
            "{section}: ожидалась диагностика счётчика, получено «{text}»"
        );
    }
}
