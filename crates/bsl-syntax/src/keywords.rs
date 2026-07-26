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

/// Регистронезависимый поиск ключевого слова по идентификатору.
/// `None` значит, что это обычный идентификатор, а не ключевое слово.
pub fn lookup(ident: &str) -> Option<Keyword> {
    let upper = ident.to_uppercase();
    Some(match upper.as_str() {
        "ЕСЛИ" | "IF" => Keyword::If,
        "ТОГДА" | "THEN" => Keyword::Then,
        "ИНАЧЕЕСЛИ" | "ELSIF" => Keyword::ElsIf,
        "ИНАЧЕ" | "ELSE" => Keyword::Else,
        "КОНЕЦЕСЛИ" | "ENDIF" => Keyword::EndIf,
        "ДЛЯ" | "FOR" => Keyword::For,
        "ПО" | "TO" => Keyword::To,
        "КАЖДОГО" | "EACH" => Keyword::Each,
        "ИЗ" | "IN" => Keyword::In,
        "ЦИКЛ" | "DO" => Keyword::Do,
        "КОНЕЦЦИКЛА" | "ENDDO" => Keyword::EndDo,
        "ПОКА" | "WHILE" => Keyword::While,
        "ПРОЦЕДУРА" | "PROCEDURE" => Keyword::Procedure,
        "КОНЕЦПРОЦЕДУРЫ" | "ENDPROCEDURE" => Keyword::EndProcedure,
        "ФУНКЦИЯ" | "FUNCTION" => Keyword::Function,
        "КОНЕЦФУНКЦИИ" | "ENDFUNCTION" => Keyword::EndFunction,
        "ВОЗВРАТ" | "RETURN" => Keyword::Return,
        "ПЕРЕМ" | "VAR" => Keyword::Var,
        "ЗНАЧ" | "VAL" => Keyword::Val,
        "ЭКСПОРТ" | "EXPORT" => Keyword::Export,
        "ПРЕРВАТЬ" | "BREAK" => Keyword::Break,
        "ПРОДОЛЖИТЬ" | "CONTINUE" => Keyword::Continue,
        "ПОПЫТКА" | "TRY" => Keyword::Try,
        "ИСКЛЮЧЕНИЕ" | "EXCEPT" => Keyword::Except,
        "КОНЕЦПОПЫТКИ" | "ENDTRY" => Keyword::EndTry,
        "ВЫЗВАТЬИСКЛЮЧЕНИЕ" | "RAISE" => Keyword::Raise,
        "НОВЫЙ" | "NEW" => Keyword::New,
        "НЕ" | "NOT" => Keyword::Not,
        "И" | "AND" => Keyword::And,
        "ИЛИ" | "OR" => Keyword::Or,
        "ИСТИНА" | "TRUE" => Keyword::True,
        "ЛОЖЬ" | "FALSE" => Keyword::False,
        "НЕОПРЕДЕЛЕНО" | "UNDEFINED" => Keyword::Undefined,
        "NULL" => Keyword::Null,
        "ВЫПОЛНИТЬ" | "EXECUTE" => Keyword::Execute,
        _ => return None,
    })
}
