//! Автодополнение BSL для REPL: что предлагать в этом месте строки.
//!
//! Вся логика — чистые функции над `(строка, позиция курсора)`, без единого
//! типа из rustyline: она так проверяется тестами, а склейка с редактором
//! живёт в `repl.rs` и занимает десяток строк.
//!
//! Источники имён — те же таблицы, по которым идёт настоящий разбор
//! (`bsl_rt::BUILTIN_FN_NAMES`, `bsl_rt::BUILTIN_METHOD_NAMES`,
//! `bsl_syntax::KEYWORD_SPELLINGS`, `bsl_sema::NEW_TYPES` плюс реестр
//! компонентов движка — его конструкторы и глобальные функции приходят в
//! [`SessionNames`]), поэтому предложить то, чего интерпретатор не знает,
//! автодополнение не может: за этим следят тесты в тех же крейтах.

/// Что уместно в текущей позиции. Место определяется одним символом слева
/// (`.`) или одним словом слева (`Новый`) — большего разбора здесь не
/// нужно, а меньший давал бы заведомо неуместные предложения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// После точки: методы объекта и имена полей/колонок.
    Member,
    /// После `Новый`: только конструируемые типы.
    NewType,
    /// Всё остальное: встроенные функции, ключевые слова, переменные сессии.
    Expression,
}

/// Имена, которые знает только текущая сессия REPL: её переменные, поля,
/// осевшие в интернере (колонки таблиц, поля структур), и имена из реестра
/// компонентов её движка — конструкторы для `Новый` и глобальные функции.
#[derive(Debug, Default, Clone)]
pub struct SessionNames {
    pub locals: Vec<String>,
    pub fields: Vec<String>,
    pub constructors: Vec<String>,
    pub functions: Vec<String>,
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Начало слова, которое сейчас дополняется. Совпадает с `pos`, если слева
/// от курсора не имя (тогда дополняем «с пустого места» — это законно,
/// список просто будет полным).
pub fn word_start(line: &str, pos: usize) -> usize {
    line[..pos]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_continue(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(pos)
}

/// Где мы находимся — по тексту ЛЕВЕЕ дополняемого слова.
pub fn place_at(line: &str, word_start: usize) -> Place {
    let before = line[..word_start].trim_end();
    if before.ends_with('.') {
        return Place::Member;
    }
    // Последнее слово слева: `Новый Мас|` — это `Новый`.
    let last_word: String = before
        .chars()
        .rev()
        .take_while(|c| is_ident_continue(*c))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    match bsl_syntax::lookup_keyword(&last_word) {
        Some(bsl_syntax::Keyword::New) => Place::NewType,
        _ => Place::Expression,
    }
}

/// Все имена, уместные в этом месте, — ещё без отбора по префиксу.
fn names_for(place: Place, session: &SessionNames) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    match place {
        Place::Member => {
            out.extend(
                bsl_rt::BUILTIN_METHOD_NAMES
                    .iter()
                    .map(|(n, _)| n.to_string()),
            );
            out.extend(session.fields.iter().cloned());
        }
        Place::NewType => {
            out.extend(bsl_sema::NEW_TYPES.iter().map(|n| n.to_string()));
            out.extend(session.constructors.iter().cloned());
        }
        Place::Expression => {
            out.extend(bsl_rt::BUILTIN_FN_NAMES.iter().map(|(n, _)| n.to_string()));
            out.extend(session.functions.iter().cloned());
            for (ru, en, _) in bsl_syntax::KEYWORD_SPELLINGS {
                out.push(ru.to_string());
                if ru != en {
                    out.push(en.to_string());
                }
            }
            out.extend(session.locals.iter().cloned());
        }
    }
    out
}

/// Кандидаты для позиции `pos`: `(с какого байта заменять, чем)`.
///
/// Отбор по префиксу РЕГИСТРОНЕЗАВИСИМЫЙ — как и всё сопоставление имён в
/// BSL: набранное `стрн` обязано найти `СтрНайти`. Подставляется при этом
/// каноническое написание из таблицы, а не то, что набрал пользователь:
/// иначе дополнение закрепляло бы случайный регистр.
///
/// Одинаковые имена схлопываются (переменная сессии может совпасть с
/// именем поля), порядок — по алфавиту без учёта регистра, чтобы список не
/// зависел от порядка таблиц.
pub fn candidates(line: &str, pos: usize, session: &SessionNames) -> (usize, Vec<String>) {
    let start = word_start(line, pos);
    let prefix = line[start..pos].to_uppercase();
    let place = place_at(line, start);

    let mut names: Vec<String> = names_for(place, session)
        .into_iter()
        .filter(|n| n.to_uppercase().starts_with(&prefix))
        .collect();
    names.sort_by_key(|n| (n.to_uppercase(), n.clone()));
    names.dedup();
    (start, names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> SessionNames {
        SessionNames {
            locals: vec!["таблица".to_string(), "сумма".to_string()],
            fields: vec!["цена".to_string()],
            constructors: vec!["ЧтениеXML".to_string(), "XMLReader".to_string()],
            functions: vec!["ПрочитатьJSON".to_string()],
        }
    }

    fn names(line: &str) -> Vec<String> {
        candidates(line, line.len(), &session()).1
    }

    #[test]
    fn word_start_finds_the_beginning_of_the_typed_name() {
        assert_eq!(word_start("СтрН", 4 * 2), 0); // кириллица — два байта на букву
        assert_eq!(word_start("х = Стр", "х = Стр".len()), "х = ".len());
        // Слева не имя — дополняем с пустого места.
        assert_eq!(word_start("х = ", 4), 4);
        assert_eq!(word_start("", 0), 0);
    }

    #[test]
    fn prefix_matching_ignores_case_but_inserts_the_canonical_spelling() {
        let found = names("стрн");
        assert!(found.contains(&"СтрНайти".to_string()), "{found:?}");
        // Префикс отбирает, а не сортирует: чужого в списке нет.
        assert!(!found.contains(&"Сообщить".to_string()), "{found:?}");
        // Английские написания ищутся так же.
        assert!(names("strf").contains(&"StrFind".to_string()));
    }

    #[test]
    fn after_a_dot_only_methods_and_field_names_are_offered() {
        let found = names("т.");
        assert_eq!(place_at("т.", "т.".len()), Place::Member);
        assert!(found.contains(&"Свернуть".to_string()), "{found:?}");
        assert!(found.contains(&"цена".to_string()), "поле из интернера");
        // Встроенных функций и ключевых слов после точки быть не должно:
        // `т.Если` и `т.СтрДлина` — не выражения BSL.
        assert!(!found.contains(&"СтрДлина".to_string()));
        assert!(!found.contains(&"Если".to_string()));
        // Пробелы вокруг точки ничего не меняют.
        assert_eq!(place_at("т . ", "т . ".len()), Place::Member);
    }

    #[test]
    fn after_new_only_constructible_types_are_offered() {
        let found = names("х = Новый ");
        assert_eq!(found, {
            let mut all: Vec<String> = bsl_sema::NEW_TYPES.iter().map(|s| s.to_string()).collect();
            all.extend(session().constructors);
            all.sort_by_key(|n| (n.to_uppercase(), n.clone()));
            all
        });
        assert!(names("х = Новый Табл").contains(&"ТаблицаЗначений".to_string()));
        // Конструкторы реестра предлагаются наравне с базовыми типами.
        assert!(names("х = Новый Чтен").contains(&"ЧтениеXML".to_string()));
        // `New` — то же самое ключевое слово.
        assert!(names("x = New Val").contains(&"ValueTable".to_string()));
    }

    #[test]
    fn in_an_expression_builtins_keywords_and_session_variables_are_offered() {
        assert!(names("Сооб").contains(&"Сообщить".to_string()));
        assert!(names("Ес").contains(&"Если".to_string()));
        assert!(names("таб").contains(&"таблица".to_string()));
        // Глобальные функции компонентов приходят из реестра сессии.
        assert!(names("Прочитать").contains(&"ПрочитатьJSON".to_string()));
        // Методы объектов в выражении не предлагаются — они бессмысленны
        // без получателя.
        assert!(!names("Сверн").contains(&"Свернуть".to_string()));
    }

    #[test]
    fn candidates_are_unique_and_sorted() {
        let mut session = session();
        // Переменная сессии, совпадающая со встроенным именем, не должна
        // задваивать строку в списке.
        session.locals.push("Сообщить".to_string());
        let (_, names) = candidates("Сооб", "Сооб".len(), &session);
        assert_eq!(names, vec!["Сообщить".to_string()]);

        let (_, names) = candidates("", 0, &session);
        let mut sorted = names.clone();
        sorted.sort_by_key(|n| (n.to_uppercase(), n.clone()));
        assert_eq!(names, sorted, "список обязан быть отсортирован");
    }

    #[test]
    fn replacement_starts_at_the_word_not_at_the_cursor() {
        let line = "х = СтрД";
        let (start, names) = candidates(line, line.len(), &session());
        assert_eq!(start, "х = ".len());
        assert!(names.contains(&"СтрДлина".to_string()));
    }
}
