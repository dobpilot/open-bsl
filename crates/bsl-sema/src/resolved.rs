use bsl_number::BslNumber;
use bsl_rt::{BuiltinFn, ConstructorCode, FunctionCode, FunctionKind, LibraryKey};
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
    /// Ожидание обещания внутри асинхронного метода.
    Await(Box<RExpr>),
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
        args: Vec<ResolvedArg>,
    },
    /// Вызов экспортного метода чужого модуля: `link` — номер записи в
    /// `ResolvedProgram::links` вида `Function`. Режимы передачи параметров
    /// целевой функции копируются сюда при резолвинге: компилятор строит
    /// `ArgMode` по ним, не видя чужой `ResolvedProgram`.
    CallImported {
        link: u32,
        param_by_val: Vec<bool>,
        args: Vec<ResolvedArg>,
    },
    /// Чтение экспортной переменной чужого модуля: `link` — запись вида
    /// `Variable` в `ResolvedProgram::links`.
    ImportedVar(u32),
    /// Вызов встроенной функции по голому имени (`Sqrt(x)`, `Pow(x,y)`,
    /// `Message(x)`, ...). Всегда по значению — ни у одной встроенной
    /// функции параметров без `Знач` нет.
    CallBuiltinFn {
        builtin: BuiltinFn,
        args: Vec<RExpr>,
    },
    /// Глобальная функция статически подключённого runtime-компонента.
    /// Имя уже разрешено в стабильную пару «пакет, код функции»; локальный
    /// индекс `.requires` назначит компилятор.
    CallComponent {
        library: LibraryKey,
        function: FunctionCode,
        kind: FunctionKind,
        args: Vec<RExpr>,
    },
    /// `объект.Метод(args)`. Арность части методов (`Добавить`) зависит от
    /// типа получателя, который в динамически типизированном BSL не
    /// известен на этапе резолвинга — поэтому здесь `args` произвольной
    /// длины, а не проверяется здесь же (см. `bsl_rt::call_builtin_method`).
    CallMethod {
        obj: Box<RExpr>,
        /// Исходное имя метода. Применимость определяется фактическим
        /// типом получателя, поэтому закрытое перечисление здесь не нужно.
        method: String,
        /// Компиляция идёт с реестром компонентов: получатель может быть
        /// внешним, поэтому даже знакомое ядру имя нельзя специализировать.
        ///
        /// Булев сознательно, как и `ResolvedParam::by_val`: состояния
        /// ровно два, оба живут в генерируемом IR (без реестра вызов
        /// закрытый, с реестром — открытый, кроме доказанно ядровых
        /// приёмников), и назвать их типом значило бы добавить имя, а не
        /// убрать неоднозначность.
        open: bool,
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
    /// Конструктор статически подключённого runtime-компонента. Локальный
    /// индекс библиотеки назначается позднее по итоговой `.requires`.
    CreateObject {
        library: LibraryKey,
        constructor: ConstructorCode,
        args: Vec<RExpr>,
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
    /// `?(Условие, Тогда, Иначе)`. Хранится тремя выражениями, а не
    /// вызовом с тремя аргументами: оператор ЛЕНИВ (измерено), и
    /// кодогену нужно вкомпилировать переходы между ветвями, а не
    /// вычислить три значения.
    Ternary {
        cond: Box<RExpr>,
        then_expr: Box<RExpr>,
        else_expr: Box<RExpr>,
    },
    NewTextWriter {
        path: Box<RExpr>,
    },
    /// `Вычислить(<строка>)` — компилирует строку как ОДНО выражение (через
    /// внутреннюю обёртку `Возврат (<строка>);`, см. `bsl-vm`) и исполняет
    /// его в текущей области видимости верхнего уровня, возвращая значение.
    DynEval(Box<RExpr>),
}

/// Аргумент вызова ПОЛЬЗОВАТЕЛЬСКОЙ функции: либо выражение, либо
/// пропущенная позиция (`Ф(1, , 3)`), допустимая только там, где у
/// параметра объявлено значение по умолчанию — это проверяет резолвер.
///
/// Пропуск — свойство МЕСТА ВЫЗОВА, а не значение: значения «пропущено» в
/// BSL нет и в `bsl_rt::BslValue` ему больше ничего не соответствует.
/// Дальше по конвейеру пропуск живёт режимом `ArgMode::Default` в таблице
/// режимов инструкции `Call`, а вызванная функция узнаёт о нём из
/// метаданных кадра (см. пролог умолчаний в `bsl-bytecode::compiler` и
/// `Frame::param_aliases` в `bsl-vm`).
///
/// У ВСТРОЕННЫХ функций правило другое: объявленных умолчаний у них нет,
/// поэтому пропуск позиции резолвится в `Неопределено` (см.
/// `resolve_builtin_args`), и отдельный вариант им не нужен.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedArg {
    Value(RExpr),
    Default,
}

/// Разрешённая связь с экспортным символом ЧУЖОГО модуля конфигурации.
/// Номер записи в `ResolvedProgram::links` — будущий `LinkSlot` таблицы
/// `Program::links`; вид символа фиксируется на этапе резолвинга.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedLink {
    /// `func` — индекс ЧАНКА целевой программы (позиция функции + 1).
    Function { module: u32, func: u16 },
    /// `slot` — номер в `module_vars` целевого модуля.
    Variable { module: u32, slot: u16 },
}

/// Номер метки внутри одного тела модуля, процедуры или функции.
/// Имя свёрнуто в этот номер резолвером, поэтому компилятор
/// байт-кода больше не сравнивает строки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub u32);

/// Разрешённый оператор вместе со строкой исходника, с которой он
/// начинается.
///
/// Строка приходит из [`bsl_syntax::Stmt::line`] и дальше становится
/// таблицей строк образа. Ноль означает оператор, не пришедший из
/// текста, — строки у него нет вовсе.
///
/// У фрагмента `Выполнить`/`Вычислить` строки отсчитываются от начала
/// его СОБСТВЕННОГО текста, а не файла с вызовом: текст фрагмента —
/// значение времени исполнения и может не лежать ни в одном файле.
#[derive(Debug, Clone, PartialEq)]
pub struct RStmt {
    pub kind: RStmtKind,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RStmtKind {
    AssignLocal {
        slot: u32,
        value: RExpr,
    },
    /// Запись в переменную модуля — видна всем функциям и телу модуля.
    AssignModuleVar {
        slot: u32,
        value: RExpr,
    },
    /// Запись экспортной переменной чужого модуля конфигурации.
    AssignImportedVar {
        link: u32,
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
    /// Точка входа, не порождающая отдельной инструкции.
    Label(LabelId),
    /// Безусловный переход к метке того же тела.
    Goto(LabelId),
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
    /// Строка объявления параметра — она же строка инструкций его
    /// пролога в таблице строк образа. Ноль означает параметр, не
    /// пришедший из текста.
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFunction {
    pub name: String,
    /// Объявлена ли функция или процедура с модификатором `Асинх`.
    pub is_async: bool,
    pub params: Vec<ResolvedParam>,
    /// Слоты `0..params.len()` — параметры (в порядке объявления), дальше —
    /// остальные локальные переменные в порядке первого появления.
    pub locals: Vec<String>,
    pub body: Vec<RStmt>,
    /// Объявлена ли она `Процедура` (а не `Функция`).
    ///
    /// Процедура не возвращает значения, и платформа отвергает её вызов в
    /// позиции выражения ещё на компиляции — измерено на 8.3.27, строка
    /// «своя процедура выражением» в `measure-stmtcall.platform.txt`.
    /// Одного `Возврат` без значения для этого мало: функция без явного
    /// возврата — законная функция.
    pub is_procedure: bool,
    /// Содержит ли тело `Выполнить`/`Вычислить`.
    ///
    /// Такая функция не может быть скомпилирована «наглухо»: фрагмент
    /// видит её область видимости по ИМЕНАМ и может читать и менять её
    /// локальные, а имена после резолвинга исчезают — остаются номера
    /// слотов. Поэтому чанк такой функции обязан унести с собой таблицу
    /// «имя -> слот» (`Chunk::local_names`), а все остальные (99% кода)
    /// компилируются как раньше и ничего лишнего не несут.
    pub uses_dynamic: bool,
    /// Метод объявлен с модификатором `Экспорт` и виден другим модулям
    /// конфигурации. Внутри одного модуля признак ни на что не влияет.
    pub export: bool,
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
    /// Полное замыкание runtime-компонентов, фактически использованных
    /// модулем. `bsl-rt` всегда занимает нулевую позицию.
    pub requirements: Vec<bsl_rt::LibraryRequirement>,
    pub functions: Vec<ResolvedFunction>,
    pub top_level: Resolved,
    /// Имена переменных уровня модуля, в порядке объявления. Они же —
    /// ПЕРВЫЕ слоты `top_level.locals`: тело модуля обращается к ним как к
    /// обычным локальным, функции — через `RExpr::ModuleVar` с тем же
    /// номером, и совпадение номеров тут не совпадение, а инвариант.
    pub module_vars: Vec<String>,
    /// Параллельно `module_vars`: слот объявлен с модификатором `Экспорт`.
    /// Разнорегистровый дубль объявления экспортируется, если экспортным
    /// было хотя бы одно из объявлений.
    pub module_var_exports: Vec<bool>,
    /// Использованные связи с чужими модулями, в порядке появления и без
    /// повторов; номер записи — операнд `CallImported`/`ImportedVar`.
    pub links: Vec<ResolvedLink>,
}

/// Есть ли в теле `Выполнить`/`Вычислить` — обход по разрешённому дереву,
/// а не по AST: `Вычислить` к этому моменту уже стал `RExpr::DynEval`, а
/// `Выполнить` — `RStmtKind::Execute`, и различать их по имени функции больше
/// не нужно.
///
/// Ищет на ЛЮБОЙ глубине вложенности: `Выполнить` внутри `Если` внутри
/// цикла — такой же повод материализовать имена, как и на верхнем уровне
/// тела.
pub fn block_uses_dynamic(body: &[RStmt]) -> bool {
    body.iter().any(stmt_uses_dynamic)
}

/// Содержит ли динамический код хотя бы одно ВЫРАЖЕНИЕ УМОЛЧАНИЯ параметра.
/// Умолчания компилируются в тот же чанк прологом, поэтому
/// `Ф(а = Вычислить("..."))` обязан пометить чанк как использующий
/// динамику, даже если ТЕЛО функции его не содержит, — иначе `local_names`
/// не материализуются и вложенный фрагмент не увидит область видимости.
pub fn params_use_dynamic(params: &[ResolvedParam]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_uses_dynamic))
}

fn stmt_uses_dynamic(s: &RStmt) -> bool {
    match &s.kind {
        RStmtKind::Execute(_) => true,
        RStmtKind::AssignLocal { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignModuleVar { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignImportedVar { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignIndex { obj, index, value } => {
            expr_uses_dynamic(obj) || expr_uses_dynamic(index) || expr_uses_dynamic(value)
        }
        RStmtKind::AssignField { obj, value, .. } => {
            expr_uses_dynamic(obj) || expr_uses_dynamic(value)
        }
        RStmtKind::ExprStmt(e) => expr_uses_dynamic(e),
        RStmtKind::If {
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
        RStmtKind::While { cond, body } => expr_uses_dynamic(cond) || block_uses_dynamic(body),
        RStmtKind::ForNumeric { from, to, body, .. } => {
            expr_uses_dynamic(from) || expr_uses_dynamic(to) || block_uses_dynamic(body)
        }
        RStmtKind::ForEach { iter, body, .. } => {
            expr_uses_dynamic(iter) || block_uses_dynamic(body)
        }
        RStmtKind::Return(e) | RStmtKind::Raise(e) => e.as_ref().is_some_and(expr_uses_dynamic),
        RStmtKind::Try { body, except_body } => {
            block_uses_dynamic(body) || block_uses_dynamic(except_body)
        }
        RStmtKind::Break | RStmtKind::Continue | RStmtKind::Label(_) | RStmtKind::Goto(_) => false,
    }
}

fn expr_uses_dynamic(e: &RExpr) -> bool {
    match e {
        RExpr::DynEval(_) => true,
        RExpr::Await(expr) => expr_uses_dynamic(expr),
        RExpr::ModuleVar(_) => false,
        RExpr::Unary { expr, .. } => expr_uses_dynamic(expr),
        RExpr::Binary { lhs, rhs, .. } => expr_uses_dynamic(lhs) || expr_uses_dynamic(rhs),
        RExpr::Call { args, .. } | RExpr::CallImported { args, .. } => {
            args.iter().any(|a| match a {
                ResolvedArg::Value(e) => expr_uses_dynamic(e),
                ResolvedArg::Default => false,
            })
        }
        RExpr::ImportedVar(_) => false,
        RExpr::CallBuiltinFn { args, .. } | RExpr::CallComponent { args, .. } => {
            args.iter().any(expr_uses_dynamic)
        }
        RExpr::CallMethod { obj, args, .. } => {
            expr_uses_dynamic(obj) || args.iter().any(expr_uses_dynamic)
        }
        RExpr::Index { obj, index } => expr_uses_dynamic(obj) || expr_uses_dynamic(index),
        RExpr::Field { obj, .. } => expr_uses_dynamic(obj),
        RExpr::NewArray { dims } => dims.iter().any(expr_uses_dynamic),
        RExpr::NewStructure { values, .. } => values.iter().any(expr_uses_dynamic),
        RExpr::CreateObject { args, .. } => args.iter().any(expr_uses_dynamic),
        RExpr::EnumMember(_) | RExpr::EnumTypeRef(_) => false,
        RExpr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_dynamic(cond) || expr_uses_dynamic(then_expr) || expr_uses_dynamic(else_expr)
        }
        RExpr::NewTextWriter { path } => expr_uses_dynamic(path),
        RExpr::NewTypeDescription(names) => expr_uses_dynamic(names),
        RExpr::Number(_)
        | RExpr::Date(_)
        | RExpr::Bool(_)
        | RExpr::Undefined
        | RExpr::Null
        | RExpr::Local(_)
        | RExpr::Str(_)
        | RExpr::NewTable
        | RExpr::NewValueComparison
        | RExpr::NewMap => false,
    }
}
