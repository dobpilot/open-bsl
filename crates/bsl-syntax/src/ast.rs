//! Синтаксическое дерево BSL. Позиции на узлах ВЫРАЖЕНИЙ сюда по-прежнему
//! не добавлены: они нужны только диагностикам парсера, а у тех свои span.
//!
//! У ОПЕРАТОРА позиция есть — номер строки, с которой он начинается
//! ([`Stmt::line`]). Она доходит до образа байт-кода и становится там
//! таблицей строк, без которой отладчику нечего сказать редактору: точка
//! останова приходит строкой, стек возвращается строками. Это и есть то
//! «точечно, а не заранее на каждый вариант», о чём говорила прежняя
//! редакция этой заметки.

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
    /// Строка объявления параметра, считая с единицы.
    ///
    /// Нужна таблице строк образа: пролог умолчаний (`JumpIfNotSkipped`
    /// и вычисление самого умолчания) выпускается ДО тела функции и ни
    /// из какого оператора не приходит, а из пользовательского текста —
    /// приходит. Без этой строки у тех инструкций её взять негде.
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    pub name: String,
    /// Метод объявлен с модификатором `Асинх`/`Async`.
    pub is_async: bool,
    pub params: Vec<Param>,
    pub export: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    /// Метод объявлен с модификатором `Асинх`/`Async`.
    pub is_async: bool,
    pub params: Vec<Param>,
    pub export: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub names: Vec<String>,
    /// Объявление завершено модификатором `Экспорт`/`Export`.
    ///
    /// Модификатор относится ко всем именам списка. Осмыслен только на уровне
    /// модуля; внутри тела метода sema отвергает его как ошибку компиляции.
    pub export: bool,
}

/// Допустимая цель присваивания: переменная, элемент по индексу или поле.
///
/// Цель — ПУТЬ (`тела[0].вх`), а не просто имя, но не любое выражение:
/// `1 = 2` и `Ф() = 3` целями не бывают. Раньше это был `Expr`, а
/// ограничение держалось соглашением — парсер строил только эти три формы,
/// а резолвер нёс запасную ветку «присваивание поддержано только...» для
/// всех остальных. Ветка была недостижима из разбора и достижима из
/// программно собранного AST, то есть проверяла не то, чем была. Теперь
/// недопустимой цели просто нет в типе, а разбор отвечает синтаксической
/// ошибкой — там, где ей и место.
#[derive(Debug, Clone, PartialEq)]
pub enum LValue {
    /// Имя переменной: локальной, модульной или заводимой этим же
    /// присваиванием — что именно, решает резолвер.
    Name(String),
    Index {
        obj: Expr,
        index: Expr,
    },
    Field {
        obj: Expr,
        name: String,
    },
}

impl LValue {
    /// Постфиксная цепочка слева от `=`, если она может быть целью.
    ///
    /// `None` — выражение целью быть не может; вызывающий (парсер) знает
    /// позицию и превращает это в ошибку разбора.
    #[must_use]
    pub fn from_expr(expr: Expr) -> Option<LValue> {
        match expr {
            Expr::Ident(name) => Some(LValue::Name(name)),
            Expr::Index { obj, index } => Some(LValue::Index {
                obj: *obj,
                index: *index,
            }),
            Expr::Field { obj, name } => Some(LValue::Field { obj: *obj, name }),
            _ => None,
        }
    }
}

/// Оператор вместе со строкой исходника, с которой он начинается.
///
/// Строка, а не байтовое смещение: в образ уходит именно она. Образ
/// обязан описывать себя сам, не требуя, чтобы исходный файл дожил до
/// запуска и не изменился, — а колонок протокол отладки не требует.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    /// Номер строки, считая с единицы. Ноль означает оператор, не
    /// пришедший из текста: такие собирает не парсер (см.
    /// [`Stmt::synthetic`]), и строки у них нет вовсе — это честнее, чем
    /// приписать чужую.
    pub line: u32,
}

impl Stmt {
    /// Оператор без позиции — для собранных не из исходного текста.
    pub fn synthetic(kind: StmtKind) -> Self {
        Self { kind, line: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Assign {
        target: LValue,
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
    /// Метка `~Имя:`. Строка ещё не свёрнута по регистру.
    Label(String),
    /// Безусловный переход `Перейти ~Имя` / `Goto ~Name`.
    Goto(String),
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
    /// Остаток от деления. Приоритет тот же, что у `Mul`/`Div`
    /// (измерено: `2 + 6 % 4` даёт 4, а `10 % 3 * 2` — 2).
    Mod,
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
    /// `Ждать`/`Await` приостанавливает текущий асинхронный метод до
    /// завершения обещания.
    Await(Box<Expr>),
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
