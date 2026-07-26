use bsl_number::BslNumber;
use bsl_syntax::{BinaryOp, UnaryOp};

/// Выражение после резолвинга: идентификаторы заменены на индексы слотов,
/// числовой литерал уже распарсен в `BslNumber` (лексер гарантирует формат
/// `цифры[.цифры]`, парсинг не может провалиться на валидном токене).
#[derive(Debug, Clone, PartialEq)]
pub enum RExpr {
    Number(BslNumber),
    Bool(bool),
    Undefined,
    Null,
    /// Слот в кадре текущей функции/скрипта.
    Local(u32),
    Unary {
        op: UnaryOp,
        expr: Box<RExpr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<RExpr>,
        rhs: Box<RExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RStmt {
    Assign {
        slot: u32,
        value: RExpr,
    },
    If {
        cond: RExpr,
        then_branch: Vec<RStmt>,
        elsif_branches: Vec<(RExpr, Vec<RStmt>)>,
        else_branch: Option<Vec<RStmt>>,
    },
    While {
        cond: RExpr,
        body: Vec<RStmt>,
    },
    /// Границы вычисляются один раз при входе в цикл — это гарантирует
    /// компилятор байт-кода (`from`/`to` компилируются один раз до тела),
    /// а не эта структура сама по себе.
    ForNumeric {
        slot: u32,
        from: RExpr,
        to: RExpr,
        body: Vec<RStmt>,
    },
    Break,
    Continue,
}

/// Результат резолвинга плоского скрипта верхнего уровня (без процедур —
/// те придут в M4). `locals` — таблица слотов в порядке первого появления
/// (оригинальное написание, для будущей отладочной информации).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
}
