//! Синтаксическое дерево BSL. Позиции (span) сюда сознательно не добавлены —
//! в M2 они не нужны никому, кроме диагностик парсера (у тех есть свои span);
//! если sema/VM позже понадобятся позиции на узлах AST, их можно добавить
//! точечно, а не заранее на каждый вариант.

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Procedure(ProcDecl),
    Function(FuncDecl),
    VarDecl(VarDecl),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    /// `Знач`/`Val` — параметр передаётся по значению, а не по ссылке.
    pub by_val: bool,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub export: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub export: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// LValue — путь (`bodies[0].vx`), а не просто имя, поэтому `target` это
    /// `Expr`, ограниченный постфиксной цепочкой (проверяется парсером, не
    /// типом: сама цепочка строится тем же кодом, что и обычные постфиксы).
    Assign {
        target: Expr,
        value: Expr,
    },
    /// Вызов процедуры/функции как оператор.
    ExprStmt(Expr),
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        elsif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    /// Границы вычисляются один раз; переменная цикла жива после `КонецЦикла`
    /// (гарантируется тем, что это обычная переменная области видимости, а не
    /// что-то отдельное — семантика уровня sema/VM, не парсера).
    ForNumeric {
        var: String,
        from: Expr,
        to: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Try {
        body: Vec<Stmt>,
        except_body: Vec<Stmt>,
    },
    Raise(Option<Expr>),
    VarDecl(VarDecl),
    /// `Выполнить(СтрокаКода)`.
    Execute(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Сырой текст числа — не парсится в `BslNumber` здесь: это работа слоя
    /// над парсером, у парсера нет и не должно быть зависимости на bsl-number.
    Number(String),
    Str(String),
    /// Цифры даты без кавычек, как из лексера.
    Date(String),
    Bool(bool),
    Undefined,
    Null,
    Ident(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `args` содержит `None` на месте пропущенного аргумента: `Ф(1, , 3)`.
    Call {
        callee: Box<Expr>,
        args: Vec<Option<Expr>>,
    },
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        obj: Box<Expr>,
        name: String,
    },
    New {
        type_name: String,
        args: Vec<Expr>,
    },
    /// `?(Условие, ЕслиИстина, ЕслиЛожь)`.
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}
