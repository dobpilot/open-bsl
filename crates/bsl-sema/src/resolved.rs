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
    /// `func` — индекс в `ResolvedProgram::functions` (не в таблице чанков
    /// байт-кода — там будет сдвиг на 1 из-за чанка верхнего уровня).
    /// `args.len()` всегда равно числу параметров функции: в M4 вызов с
    /// пропущенными/меньшим числом аргументов — ошибка резолвинга, а не
    /// заполнение значениями по умолчанию (см. `SemaError::Unsupported`).
    Call {
        func: u32,
        args: Vec<RExpr>,
    },
    Str(String),
    Index {
        obj: Box<RExpr>,
        index: Box<RExpr>,
    },
    Field {
        obj: Box<RExpr>,
        name: String,
    },
    /// `Новый Массив(d1, d2, ...)` — каждое измерение вкладывает следующий
    /// уровень массивов (`Новый Массив(3, 4)` — массив из 3 массивов по 4).
    NewArray {
        dims: Vec<RExpr>,
    },
    /// `Новый Структура("x,y,z", ...)`. Список полей — обязательно строковый
    /// литерал на месте вызова (см. `SemaError::Unsupported` иначе):
    /// динамические формы, зависящие от рантайм-значения строки, отложены.
    /// `values.len() == keys.len()` всегда — если аргументов-значений не
    /// было вовсе, `values` заполнен `RExpr::Undefined`.
    NewStructure {
        keys: Vec<String>,
        values: Vec<RExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RStmt {
    AssignLocal {
        slot: u32,
        value: RExpr,
    },
    AssignIndex {
        obj: RExpr,
        index: RExpr,
        value: RExpr,
    },
    AssignField {
        obj: RExpr,
        name: String,
        value: RExpr,
    },
    /// Вызов процедуры/функции как оператор — результат отбрасывается.
    ExprStmt(RExpr),
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
    /// `iter` вычисляется один раз (как и границы `Для`) — компилятор
    /// байт-кода превращает это в индексный цикл поверх длины коллекции.
    ForEach {
        slot: u32,
        iter: RExpr,
        body: Vec<RStmt>,
    },
    Break,
    Continue,
    /// `Неопределено` при отсутствии выражения — функция без `Возврат`
    /// возвращает `Неопределено` (совпадает с неявным возвратом в конце тела).
    Return(Option<RExpr>),
    /// `Попытка ... Исключение ... КонецПопытки`. Компилятор байт-кода
    /// заводит на `body` защищённый диапазон (начало_pc, конец_pc,
    /// handler_pc = начало `except_body`) — ноль стоимости, пока
    /// исключение не брошено, как и описано в брифе.
    Try {
        body: Vec<RStmt>,
        except_body: Vec<RStmt>,
    },
    /// `ВызватьИсключение <выражение>;` — `None` внутри `Исключение` значит
    /// повторно бросить пойманное исключение (голая форма).
    Raise(Option<RExpr>),
}

/// Параметр после резолвинга: имя больше не нужно (оно уже стало слотом
/// 0..params.len() в `locals` той же функции), важен только режим передачи.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedParam {
    /// `Знач`/`Val` — по значению; иначе (по умолчанию в BSL) — по ссылке.
    pub by_val: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFunction {
    pub name: String,
    pub params: Vec<ResolvedParam>,
    /// Слоты 0..params.len() — параметры (в порядке объявления), дальше —
    /// остальные локальные переменные в порядке первого появления.
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
}

/// Результат резолвинга скрипта верхнего уровня (без объявлений функций).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
}

/// Результат резолвинга целого модуля: все объявления процедур/функций
/// (плоское пространство имён — в BSL нет вложенных процедур и замыканий)
/// плюс операторы верхнего уровня, которые могут их вызывать.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProgram {
    pub functions: Vec<ResolvedFunction>,
    pub top_level: Resolved,
}
