//! Таблица ключевых слов BSL. Язык двуязычный: `Если`/`If`, `Функция`/`Function`
//! и т.д. — оба написания резолвятся в один и тот же вариант `Keyword` один
//! раз при лексинге, дальше по конвейеру разницы между языками уже нет.
//!
//! Слова вроде `const` в BSL не зарезервированы — в таблицу их не добавляем.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    If,
    Then,
    ElsIf,
    Else,
    EndIf,
    For,
    To,
    Each,
    In,
    Do,
    EndDo,
    While,
    Procedure,
    EndProcedure,
    Function,
    EndFunction,
    Return,
    Var,
    Val,
    Export,
    Break,
    Continue,
    Try,
    Except,
    EndTry,
    Raise,
    New,
    Not,
    And,
    Or,
    True,
    False,
    Undefined,
    Null,
    Execute,
}

/// Написания каждого ключевого слова: `(русское, английское)` в
/// КАНОНИЧЕСКОМ регистре — том, в котором их принято писать в коде и в
/// котором их предлагает автодополнение REPL. Поиск (`lookup`) идёт по
/// этой же таблице, так что второго списка, который мог бы разъехаться с
/// первым, не существует.
///
/// У `Null` оба написания совпадают — как и в самом языке.
pub const SPELLINGS: &[(&str, &str, Keyword)] = &[
    ("Если", "If", Keyword::If),
    ("Тогда", "Then", Keyword::Then),
    ("ИначеЕсли", "ElsIf", Keyword::ElsIf),
    ("Иначе", "Else", Keyword::Else),
    ("КонецЕсли", "EndIf", Keyword::EndIf),
    ("Для", "For", Keyword::For),
    ("По", "To", Keyword::To),
    ("Каждого", "Each", Keyword::Each),
    ("Из", "In", Keyword::In),
    ("Цикл", "Do", Keyword::Do),
    ("КонецЦикла", "EndDo", Keyword::EndDo),
    ("Пока", "While", Keyword::While),
    ("Процедура", "Procedure", Keyword::Procedure),
    ("КонецПроцедуры", "EndProcedure", Keyword::EndProcedure),
    ("Функция", "Function", Keyword::Function),
    ("КонецФункции", "EndFunction", Keyword::EndFunction),
    ("Возврат", "Return", Keyword::Return),
    ("Перем", "Var", Keyword::Var),
    ("Знач", "Val", Keyword::Val),
    ("Экспорт", "Export", Keyword::Export),
    ("Прервать", "Break", Keyword::Break),
    ("Продолжить", "Continue", Keyword::Continue),
    ("Попытка", "Try", Keyword::Try),
    ("Исключение", "Except", Keyword::Except),
    ("КонецПопытки", "EndTry", Keyword::EndTry),
    ("ВызватьИсключение", "Raise", Keyword::Raise),
    ("Новый", "New", Keyword::New),
    ("Не", "Not", Keyword::Not),
    ("И", "And", Keyword::And),
    ("Или", "Or", Keyword::Or),
    ("Истина", "True", Keyword::True),
    ("Ложь", "False", Keyword::False),
    ("Неопределено", "Undefined", Keyword::Undefined),
    ("Null", "Null", Keyword::Null),
    ("Выполнить", "Execute", Keyword::Execute),
];

/// Регистронезависимый поиск ключевого слова по идентификатору.
/// `None` значит, что это обычный идентификатор, а не ключевое слово.
pub fn lookup(ident: &str) -> Option<Keyword> {
    let upper = ident.to_uppercase();
    SPELLINGS
        .iter()
        .find(|(ru, en, _)| ru.to_uppercase() == upper || en.to_uppercase() == upper)
        .map(|(_, _, kw)| *kw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_spelling_resolves_to_its_own_keyword() {
        for (ru, en, kw) in SPELLINGS {
            assert_eq!(lookup(ru), Some(*kw), "русское написание {ru}");
            assert_eq!(lookup(en), Some(*kw), "английское написание {en}");
            // Регистр не значим ни в одном из двух языков.
            assert_eq!(lookup(&ru.to_uppercase()), Some(*kw));
            assert_eq!(lookup(&en.to_lowercase()), Some(*kw));
        }
    }

    #[test]
    fn spellings_are_unique() {
        let mut all: Vec<String> = SPELLINGS
            .iter()
            .flat_map(|(ru, en, _)| [ru.to_uppercase(), en.to_uppercase()])
            .collect();
        all.sort();
        all.dedup();
        // `Null` даёт одно и то же написание дважды — оно и есть
        // единственный законный повтор.
        assert_eq!(all.len(), SPELLINGS.len() * 2 - 1);
    }

    #[test]
    fn ordinary_identifiers_are_not_keywords() {
        for ident in ["x", "СуммаИтого", "Массив", "Сообщить"] {
            assert_eq!(lookup(ident), None, "{ident}");
        }
    }
}
