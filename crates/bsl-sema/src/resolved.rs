use bsl_number::BslNumber;
use bsl_rt::{BuiltinFn, BuiltinMethod};
use bsl_syntax::{BinaryOp, UnaryOp};

/// Выражение после резолвинга: идентификаторы заменены на индексы слотов,
/// числовой литерал уже распарсен в `BslNumber` (лексер гарантирует формат
/// `цифры[.цифры]`, парсинг не может провалиться на валидном токене).
#[derive(Debug, Clone, PartialEq)]
pub enum RExpr {
    Number(BslNumber),
    /// Литерал даты `'ГГГГММДД[ЧЧММСС]'`, уже разобранный в момент
    /// времени: дальше по конвейеру это обычная константа чанка, как и
    /// число.
    Date(bsl_rt::BslDate),
    Bool(bool),
    Undefined,
    Null,
    /// Пропущенный аргумент вызова (`Ф(1, , 3)`) на месте, где параметр
    /// объявлен со значением по умолчанию — компилируется в
    /// `bsl_rt::BslValue::Skipped`, который пролог параметров по умолчанию
    /// в вызванной функции (см. `bsl-bytecode::compiler::compile_chunk`)
    /// заменит разрешённым `ResolvedParam::default`. Само по себе не
    /// значение, с которым можно что-то делать — `Неопределено` для этого
    /// уже есть и означает другое (явно переданное "нет значения", а не
    /// "используй умолчание").
    Skipped,
    /// Слот в кадре текущей функции/скрипта.
    Local(u32),
    /// Переменная уровня модуля (`Перем` в начале файла). Отдельный вариант,
    /// а не `Local`: она живёт в кадре ВЕРХНЕГО УРОВНЯ, общем для всех
    /// функций, а не в кадре текущего вызова.
    ModuleVar(u32),
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
    /// Вызов встроенной функции по голому имени (`Sqrt(x)`, `Pow(x,y)`,
    /// `Message(x)`, ...). Всегда по значению — ни у одной встроенной
    /// функции параметров без `Знач` нет.
    CallBuiltinFn {
        builtin: BuiltinFn,
        args: Vec<RExpr>,
    },
    /// `объект.Метод(args)`. Арность части методов (`Добавить`) зависит от
    /// типа получателя, который в динамически типизированном BSL не
    /// известен на этапе резолвинга — поэтому здесь `args` произвольной
    /// длины, а не проверяется здесь же (см. `bsl_rt::call_builtin_method`).
    CallMethod {
        obj: Box<RExpr>,
        method: BuiltinMethod,
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
    /// `Новый ТаблицаЗначений()` — колонки заводятся отдельно, через
    /// `.Колонки.Добавить(имя)` (см. `CallMethod`), не в конструкторе.
    NewTable,
    NewTypeDescription(Box<RExpr>),
    NewValueComparison,
    /// `Новый Соответствие()` — пары ключ-значение заводятся отдельно, через
    /// `.Вставить(Ключ, Значение)`, не в конструкторе (как и `ТаблицаЗначений`
    /// выше).
    NewMap,
    /// `Новый ЗаписьТекста(Путь)` — путь вычисляется во время исполнения.
    /// Член платформенного перечисления — КОНСТАНТА времени компиляции.
    /// У платформы перечисление тоже не объект с полями: опечатка в имени
    /// члена там ошибка компиляции, а не рантайма.
    EnumMember(bsl_rt::EnumValue),
    /// Голое имя системного перечисления (`ВариантЗаписиДатыJSON`, без
    /// `.Член`) — тоже КОНСТАНТА времени компиляции, тем же рассуждением,
    /// что и `EnumMember` (см. doc comment там же и на
    /// `bsl_rt::BslValue::EnumType`).
    EnumTypeRef(bsl_rt::EnumKind),
    NewJsonReader,
    NewJsonWriter,
    /// `Новый НастройкиСериализацииJSON` — без аргументов, как `NewJsonReader`.
    NewJsonSerializerSettings,
    /// `?(Условие, Тогда, Иначе)`. Хранится тремя выражениями, а не
    /// вызовом с тремя аргументами: оператор ЛЕНИВ (измерено), и
    /// кодогену нужно вкомпилировать переходы между ветвями, а не
    /// вычислить три значения.
    Ternary {
        cond: Box<RExpr>,
        then_expr: Box<RExpr>,
        else_expr: Box<RExpr>,
    },
    NewTextDocument,
    NewSpreadDocument,
    NewXmlReader,
    NewXmlWriter,
    NewXmlWriterSettings {
        encoding: Box<RExpr>,
        version: Box<RExpr>,
        indent: Box<RExpr>,
    },
    NewJsonWriterSettings {
        line_break: Box<RExpr>,
        indent: Box<RExpr>,
    },
    NewTextWriter {
        path: Box<RExpr>,
    },
    /// `Новый ДвоичныеДанные(ИмяФайла)` — файл читается целиком в момент
    /// создания.
    NewBinaryData {
        path: Box<RExpr>,
    },
    /// `Новый БуферДвоичныхДанных(Размер[, ПорядокБайтов])` — размер
    /// обязателен, порядок необязателен и по умолчанию `LittleEndian`.
    /// Пропущенная позиция приходит `Undefined`, как у
    /// [`RExpr::NewJsonWriterSettings`].
    NewBinaryBuffer {
        size: Box<RExpr>,
        order: Box<RExpr>,
    },
    /// `Новый ПотокВПамяти([ЁмкостьЛибоБуфер])` — единственный аргумент
    /// необязателен и приходит `Undefined`, когда его нет.
    NewMemoryStream {
        arg: Box<RExpr>,
    },
    /// `Новый ФайловыйПоток(Имя, Режим[, Доступ])` — доступ необязателен и
    /// по умолчанию `ЧтениеИЗапись` (измерено).
    NewFileStream {
        path: Box<RExpr>,
        mode: Box<RExpr>,
        access: Box<RExpr>,
    },
    /// Голое имя `ФайловыеПотоки` как выражение. В отличие от
    /// [`RExpr::EnumTypeRef`] это НЕ константа: `ФайловыеПотоки =
    /// ФайловыеПотоки` платформа считает ложью (измерено), значит каждое
    /// обращение обязано строить новый объект.
    NewFileStreamsManager,
    /// `Вычислить(<строка>)` — компилирует строку как ОДНО выражение (через
    /// внутреннюю обёртку `Возврат (<строка>);`, см. `bsl-vm`) и исполняет
    /// его в текущей области видимости верхнего уровня, возвращая значение.
    DynEval(Box<RExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RStmt {
    AssignLocal {
        slot: u32,
        value: RExpr,
    },
    /// Запись в переменную модуля — видна всем функциям и телу модуля.
    AssignModuleVar {
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
    /// заводит на `body` защищённый диапазон (`начало_pc`, `конец_pc`,
    /// `handler_pc` = начало `except_body`) — ноль стоимости, пока
    /// исключение не брошено, как и описано в брифе.
    Try {
        body: Vec<RStmt>,
        except_body: Vec<RStmt>,
    },
    /// `ВызватьИсключение <выражение>;` — `None` внутри `Исключение` значит
    /// повторно бросить пойманное исключение (голая форма).
    Raise(Option<RExpr>),
    /// `Выполнить(<строка>);` — компилирует и исполняет строку как
    /// операторы в текущей области видимости верхнего уровня (см.
    /// `Instr::RunDynamic` в `bsl-vm` — там же обоснование ограничений:
    /// только верхний уровень, новые имена из фрагмента не переживают
    /// вызов).
    Execute(RExpr),
}

/// Параметр после резолвинга: имя больше не нужно (оно уже стало слотом
/// `0..params.len()` в `locals` той же функции), важен режим передачи и
/// значение по умолчанию, если оно объявлено.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedParam {
    /// `Знач`/`Val` — по значению; иначе (по умолчанию в BSL) — по ссылке.
    pub by_val: bool,
    /// `Ф(а = <выражение>)` — резолвится в той же области видимости, что и
    /// тело функции (слоты остальных параметров уже объявлены). `None` —
    /// параметр обязателен, пропустить его на вызове (`Ф(1, , 3)`) —
    /// ошибка резолвинга, не рантайма.
    pub default: Option<RExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFunction {
    pub name: String,
    pub params: Vec<ResolvedParam>,
    /// Слоты `0..params.len()` — параметры (в порядке объявления), дальше —
    /// остальные локальные переменные в порядке первого появления.
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
    /// Содержит ли тело `Выполнить`/`Вычислить`.
    ///
    /// Такая функция не может быть скомпилирована «наглухо»: фрагмент
    /// видит её область видимости по ИМЕНАМ и может читать и менять её
    /// локальные, а имена после резолвинга исчезают — остаются номера
    /// слотов. Поэтому чанк такой функции обязан унести с собой таблицу
    /// «имя -> слот» (`Chunk::local_names`), а все остальные (99% кода)
    /// компилируются как раньше и ничего лишнего не несут.
    pub uses_dynamic: bool,
}

/// Результат резолвинга скрипта верхнего уровня (без объявлений функций).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
    /// См. `ResolvedFunction::uses_dynamic` — то же самое для кода
    /// верхнего уровня.
    pub uses_dynamic: bool,
}

/// Результат резолвинга целого модуля: все объявления процедур/функций
/// (плоское пространство имён — в BSL нет вложенных процедур и замыканий)
/// плюс операторы верхнего уровня, которые могут их вызывать.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProgram {
    pub functions: Vec<ResolvedFunction>,
    pub top_level: Resolved,
    /// Имена переменных уровня модуля, в порядке объявления. Они же —
    /// ПЕРВЫЕ слоты `top_level.locals`: тело модуля обращается к ним как к
    /// обычным локальным, функции — через `RExpr::ModuleVar` с тем же
    /// номером, и совпадение номеров тут не совпадение, а инвариант.
    pub module_vars: Vec<String>,
}

/// Есть ли в теле `Выполнить`/`Вычислить` — обход по разрешённому дереву,
/// а не по AST: `Вычислить` к этому моменту уже стал `RExpr::DynEval`, а
/// `Выполнить` — `RStmt::Execute`, и различать их по имени функции больше
/// не нужно.
///
/// Ищет на ЛЮБОЙ глубине вложенности: `Выполнить` внутри `Если` внутри
/// цикла — такой же повод материализовать имена, как и на верхнем уровне
/// тела.
pub fn block_uses_dynamic(body: &[RStmt]) -> bool {
    body.iter().any(stmt_uses_dynamic)
}

fn stmt_uses_dynamic(s: &RStmt) -> bool {
    match s {
        RStmt::Execute(_) => true,
        RStmt::AssignLocal { value, .. } => expr_uses_dynamic(value),
        RStmt::AssignModuleVar { value, .. } => expr_uses_dynamic(value),
        RStmt::AssignIndex { obj, index, value } => {
            expr_uses_dynamic(obj) || expr_uses_dynamic(index) || expr_uses_dynamic(value)
        }
        RStmt::AssignField { obj, value, .. } => expr_uses_dynamic(obj) || expr_uses_dynamic(value),
        RStmt::ExprStmt(e) => expr_uses_dynamic(e),
        RStmt::If {
            cond,
            then_branch,
            elsif_branches,
            else_branch,
        } => {
            expr_uses_dynamic(cond)
                || block_uses_dynamic(then_branch)
                || elsif_branches
                    .iter()
                    .any(|(c, b)| expr_uses_dynamic(c) || block_uses_dynamic(b))
                || else_branch.as_deref().is_some_and(block_uses_dynamic)
        }
        RStmt::While { cond, body } => expr_uses_dynamic(cond) || block_uses_dynamic(body),
        RStmt::ForNumeric { from, to, body, .. } => {
            expr_uses_dynamic(from) || expr_uses_dynamic(to) || block_uses_dynamic(body)
        }
        RStmt::ForEach { iter, body, .. } => expr_uses_dynamic(iter) || block_uses_dynamic(body),
        RStmt::Return(e) | RStmt::Raise(e) => e.as_ref().is_some_and(expr_uses_dynamic),
        RStmt::Try { body, except_body } => {
            block_uses_dynamic(body) || block_uses_dynamic(except_body)
        }
        RStmt::Break | RStmt::Continue => false,
    }
}

fn expr_uses_dynamic(e: &RExpr) -> bool {
    match e {
        RExpr::DynEval(_) => true,
        RExpr::ModuleVar(_) => false,
        RExpr::Unary { expr, .. } => expr_uses_dynamic(expr),
        RExpr::Binary { lhs, rhs, .. } => expr_uses_dynamic(lhs) || expr_uses_dynamic(rhs),
        RExpr::Call { args, .. } | RExpr::CallBuiltinFn { args, .. } => {
            args.iter().any(expr_uses_dynamic)
        }
        RExpr::CallMethod { obj, args, .. } => {
            expr_uses_dynamic(obj) || args.iter().any(expr_uses_dynamic)
        }
        RExpr::Index { obj, index } => expr_uses_dynamic(obj) || expr_uses_dynamic(index),
        RExpr::Field { obj, .. } => expr_uses_dynamic(obj),
        RExpr::NewArray { dims } => dims.iter().any(expr_uses_dynamic),
        RExpr::NewStructure { values, .. } => values.iter().any(expr_uses_dynamic),
        RExpr::EnumMember(_)
        | RExpr::EnumTypeRef(_)
        | RExpr::NewJsonReader
        | RExpr::NewJsonWriter
        | RExpr::NewJsonSerializerSettings
        | RExpr::NewTextDocument
        | RExpr::NewSpreadDocument
        | RExpr::NewXmlReader
        | RExpr::NewXmlWriter
        | RExpr::NewFileStreamsManager => false,
        RExpr::NewXmlWriterSettings {
            encoding,
            version,
            indent,
        } => expr_uses_dynamic(encoding) || expr_uses_dynamic(version) || expr_uses_dynamic(indent),
        RExpr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_dynamic(cond) || expr_uses_dynamic(then_expr) || expr_uses_dynamic(else_expr)
        }
        RExpr::NewJsonWriterSettings { line_break, indent } => {
            expr_uses_dynamic(line_break) || expr_uses_dynamic(indent)
        }
        RExpr::NewTextWriter { path } | RExpr::NewBinaryData { path } => expr_uses_dynamic(path),
        RExpr::NewBinaryBuffer { size, order } => {
            expr_uses_dynamic(size) || expr_uses_dynamic(order)
        }
        RExpr::NewMemoryStream { arg } => expr_uses_dynamic(arg),
        RExpr::NewFileStream { path, mode, access } => {
            expr_uses_dynamic(path) || expr_uses_dynamic(mode) || expr_uses_dynamic(access)
        }
        RExpr::NewTypeDescription(names) => expr_uses_dynamic(names),
        RExpr::Number(_)
        | RExpr::Date(_)
        | RExpr::Bool(_)
        | RExpr::Undefined
        | RExpr::Null
        | RExpr::Skipped
        | RExpr::Local(_)
        | RExpr::Str(_)
        | RExpr::NewTable
        | RExpr::NewValueComparison
        | RExpr::NewMap => false,
    }
}
