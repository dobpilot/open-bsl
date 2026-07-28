//! Подсветка BSL для REPL: разбор строки на куски и раскраска их ANSI.
//!
//! Своя развёртка, а не `bsl_syntax::Lexer`, ровно по одной причине:
//! подсвечивать надо ВВОДИМУЮ строку, то есть заведомо недописанную.
//! Лексер на `"незакрытой` возвращает `UnterminatedString` и не отдаёт
//! ничего — а подсветка обязана показать хоть что-то на каждом нажатии
//! клавиши. Здесь незакрытый литерал просто тянется до конца строки.
//!
//! Классификация имён (ключевое слово / встроенное / прочее) идёт через те
//! же таблицы, что и настоящий разбор (`bsl_syntax::keywords`,
//! `bsl_rt::BuiltinFn`/`BuiltinMethod`), поэтому подсветка не может
//! разойтись с языком: добавленная функция подсвечивается сама.
//!
//! ГЛАВНЫЙ ИНВАРИАНТ: раскраска не меняет текст. Снять ANSI-коды с
//! результата — получить ровно исходную строку (тест
//! `colorize_never_changes_the_text_itself`). Иначе курсор в терминале
//! разъедется с содержимым строки.

/// Что за кусок строки. `Punct` — всё остальное, включая пробелы: своего
/// цвета у них нет, но покрытие строки обязано быть сплошным.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    Comment,
    Str,
    Date,
    Number,
    Keyword,
    Builtin,
    Ident,
    Punct,
}

impl Piece {
    /// ANSI-код цвета; `None` — печатать как есть.
    fn color(self) -> Option<&'static str> {
        match self {
            Piece::Comment => Some("90"), // серый
            Piece::Str => Some("32"),     // зелёный
            Piece::Date => Some("32"),    // зелёный: тоже литерал
            Piece::Number => Some("33"),  // жёлтый
            Piece::Keyword => Some("35"), // пурпурный
            Piece::Builtin => Some("36"), // голубой
            Piece::Ident | Piece::Punct => None,
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Разбор строки на куски `(начало, конец, что это)` в БАЙТОВЫХ смещениях.
/// Куски идут подряд и покрывают строку целиком, без дыр и наложений — на
/// этом держится склейка обратно в `colorize`.
pub fn pieces(line: &str) -> Vec<(usize, usize, Piece)> {
    let bytes = line.as_bytes();
    let mut out: Vec<(usize, usize, Piece)> = Vec::new();
    let mut i = 0;

    // Один шаг вперёд по символам, чтобы не резать UTF-8 посередине.
    let next_char = |i: usize| line[i..].chars().next();

    while i < line.len() {
        let start = i;
        let c = match next_char(i) {
            Some(c) => c,
            None => break,
        };

        let piece = if c == '/' && bytes.get(i + 1) == Some(&b'/') {
            // Комментарий — до конца строки, внутри ничего не разбирается.
            i = line.len();
            Piece::Comment
        } else if c == '"' {
            i += 1;
            // `""` внутри литерала — экранированная кавычка, а не конец:
            // тот же разбор, что и в лексере.
            while i < line.len() {
                let ch = next_char(i).unwrap();
                i += ch.len_utf8();
                if ch == '"' {
                    if next_char(i) == Some('"') {
                        i += 1;
                        continue;
                    }
                    break;
                }
            }
            Piece::Str
        } else if c == '\'' {
            // Литерал даты `'20240115'`. Незакрытый — до конца строки.
            i += 1;
            while i < line.len() {
                let ch = next_char(i).unwrap();
                i += ch.len_utf8();
                if ch == '\'' {
                    break;
                }
            }
            Piece::Date
        } else if c.is_ascii_digit() {
            while let Some(ch) = next_char(i) {
                if ch.is_ascii_digit() || ch == '.' {
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            Piece::Number
        } else if is_ident_start(c) {
            while let Some(ch) = next_char(i) {
                if is_ident_continue(ch) {
                    i += ch.len_utf8();
                } else {
                    break;
                }
            }
            classify_ident(&line[start..i])
        } else {
            i += c.len_utf8();
            Piece::Punct
        };

        // Соседние одинаковые куски склеиваются: незачем открывать и
        // закрывать цвет вокруг каждого пробела.
        match out.last_mut() {
            Some((_, end, last)) if *last == piece && *end == start => *end = i,
            _ => out.push((start, i, piece)),
        }
    }
    out
}

/// Имя — ключевое слово, встроенная функция/метод или обычный идентификатор.
/// Метод и функция дают один цвет: с точки зрения читающего строку это
/// одинаково «имя из языка, а не моё».
fn classify_ident(ident: &str) -> Piece {
    if bsl_syntax::lookup_keyword(ident).is_some() {
        Piece::Keyword
    } else if bsl_rt::BuiltinFn::lookup(ident).is_some()
        || bsl_rt::BuiltinMethod::lookup(ident).is_some()
    {
        Piece::Builtin
    } else {
        Piece::Ident
    }
}

/// Строка с ANSI-кодами. Текст не меняется — только вставляются коды.
pub fn colorize(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 32);
    for (start, end, piece) in pieces(line) {
        match piece.color() {
            Some(code) => {
                out.push_str("\x1b[");
                out.push_str(code);
                out.push('m');
                out.push_str(&line[start..end]);
                out.push_str("\x1b[0m");
            }
            None => out.push_str(&line[start..end]),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Снять коды — получить исходный текст. Проверяется на каждом тесте
    /// ниже, но отдельно — на нарочно кривом вводе.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // `[..m` — всё, что мы вообще порождаем.
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn kinds(line: &str) -> Vec<(&str, Piece)> {
        pieces(line)
            .into_iter()
            .map(|(s, e, p)| (&line[s..e], p))
            .collect()
    }

    #[test]
    fn colorize_never_changes_the_text_itself() {
        for line in [
            "",
            "х = 1;",
            "Если СтрДлина(с) > 0 Тогда Сообщить(\"да\"); КонецЕсли;",
            "т.Свернуть(\"группа\", \"сумма\");",
            "д = '20240115';",
            "// комментарий с \"кавычками\" и 'датой'",
            "незакрытая = \"строка",
            "незакрытая = 'дата",
            "эмодзи = \"\u{1F600}\";",
            "1.5 + .5 + 1..2",
            "\"\"\"экранированная\"\"\"",
        ] {
            assert_eq!(strip_ansi(&colorize(line)), line, "вход: {line:?}");
        }
    }

    #[test]
    fn pieces_cover_the_line_without_gaps() {
        let line = "Для i = 1 По 10 Цикл Сообщить(i); КонецЦикла // всё";
        let mut expected_start = 0;
        for (start, end, _) in pieces(line) {
            assert_eq!(start, expected_start, "дыра или наложение в {line:?}");
            assert!(end > start);
            expected_start = end;
        }
        assert_eq!(expected_start, line.len());
    }

    #[test]
    fn keywords_builtins_and_plain_names_get_different_pieces() {
        let k = kinds("Если СтрДлина(х) > 0 Тогда");
        assert!(k.contains(&("Если", Piece::Keyword)));
        assert!(k.contains(&("СтрДлина", Piece::Builtin)));
        assert!(k.contains(&("х", Piece::Ident)));
        assert!(k.contains(&("0", Piece::Number)));
        // Регистр не важен — как и в самом языке.
        assert!(kinds("если х тогда").contains(&("если", Piece::Keyword)));
        // Метод объекта — тот же цвет, что и встроенная функция.
        assert!(kinds("т.Свернуть()").contains(&("Свернуть", Piece::Builtin)));
    }

    #[test]
    fn literals_swallow_their_content() {
        assert_eq!(
            kinds("с = \"текст // не комментарий\";"),
            // Соседние `Punct` склеены в один кусок — цвет у них общий
            // (никакой), и дробить строку ради этого незачем.
            vec![
                ("с", Piece::Ident),
                (" = ", Piece::Punct),
                ("\"текст // не комментарий\"", Piece::Str),
                (";", Piece::Punct),
            ]
        );
        // Экранированная кавычка не закрывает литерал.
        assert_eq!(kinds("\"а\"\"б\""), vec![("\"а\"\"б\"", Piece::Str)]);
        // Незакрытый литерал тянется до конца строки, а не роняет разбор.
        assert_eq!(kinds("\"хвост"), vec![("\"хвост", Piece::Str)]);
    }

    #[test]
    fn a_comment_swallows_the_rest_of_the_line() {
        assert_eq!(
            kinds("х = 1; // Если \"строка\""),
            vec![
                ("х", Piece::Ident),
                (" = ", Piece::Punct),
                ("1", Piece::Number),
                ("; ", Piece::Punct),
                ("// Если \"строка\"", Piece::Comment),
            ]
        );
    }

    #[test]
    fn date_literal_is_its_own_piece() {
        assert!(kinds("д = '20240115';").contains(&("'20240115'", Piece::Date)));
    }
}
