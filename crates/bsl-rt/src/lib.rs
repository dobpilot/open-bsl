//! Рантайм-слой: значения, арифметика/сравнение, коллекции (`Массив`,
//! `Структура`, `Соответствие`, `ТаблицаЗначений`), строки UTF-16
//! (`Str`/`BslString`), даты (`Date`/`BslDate`, эпоха — `0001-01-01`, см.
//! модуль `date`) и типы как значения (`Type`/`TypeId`). `BslValue` растёт
//! по мере готовности остальных слоёв, а не заранее под все типы из брифа.

mod builtin;
mod date;
mod interner;
mod locale;
mod map;
mod object;
pub mod open_questions;
mod runtime_shapes;
mod shape;
mod string;
mod table;
mod types;

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::rc::Rc;

use bsl_number::{BslNumber, NumError};

pub use builtin::{
    call_builtin_fn, call_builtin_method, call_builtin_method_ctx, BuiltinFn, BuiltinMethod,
    BUILTIN_FN_NAMES, BUILTIN_METHOD_NAMES,
};
pub use date::{
    format_long as format_date_long, format_pattern as format_date_pattern, BslDate, DateBoundary,
    DatePart, DEFAULT_PATTERN as DEFAULT_DATE_PATTERN,
};
pub use interner::{NameId, NameInterner};
pub use locale::{Locale, NBSP};
pub use map::MapData;
pub use object::{BslObject, StructureStorage};
pub use runtime_shapes::RuntimeShapes;
pub use shape::{Shape, ShapeTable, MAX_SHAPE_TRANSITIONS};
pub use string::{BslString, MAX_TEMPLATE_ARGS};
pub use table::ValueTableData;
pub use types::TypeId;

#[derive(Debug, Clone)]
pub enum BslValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(BslNumber),
    Str(BslString),
    /// Момент времени с разрешением 1 секунда. Отсчёт — от `0001-01-01`,
    /// НЕ от Unix-эпохи: пустая дата (`'00010101'`) обязана быть нулём, а
    /// не отрицательным числом, иначе `ЗначениеЗаполнено` и сравнения с
    /// пустой датой пришлось бы писать через отдельную константу. Подробно
    /// — в модуле `date`.
    Date(BslDate),
    /// Тип как ЗНАЧЕНИЕ (`ТипЗнч(х)`, `Тип("Массив")`) — не тег этого
    /// перечисления, а полноценное значение, которое можно сравнить,
    /// положить в переменную и напечатать (`Строка(ТипЗнч(1))` -> `Число`).
    /// `TypeId` — `Copy` в один байт, поэтому вариант ничего не добавляет
    /// к размеру `BslValue`.
    Type(TypeId),
    Object(Rc<BslObject>),
    /// Пропущенный позиционный аргумент (`Ф(1, , 3)`) — ТОЛЬКО как
    /// временное значение параметра сразу при входе в функцию/процедуру, до
    /// того как пролог параметров по умолчанию (см.
    /// `bsl-bytecode::compiler`, генерация `JumpUnlessSkipped`) заменит его
    /// резолвнутым значением по умолчанию из объявления. Отличается от
    /// `Неопределено`: `Ф(Знач а = 1)` вызванная как `Ф()` должна дать `а`
    /// значение `1`, а не `Неопределено`, а `Ф(,)` внутри тела, если бы
    /// прологу не удалось подставить дефолт, должна была бы упасть, а не
    /// молча дать `Неопределено` — снаружи этого значения не видно ни при
    /// каких обстоятельствах, весь остальной рантайм трактует его как
    /// ошибку типа (см. `as_number`/`as_str`/... через generic `_ => Err`).
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RtError {
    Num(NumError),
    /// Операция получила значение не того типа — например, `Если 1 Тогда`:
    /// в BSL условия строго булевы, никакой truthiness.
    TypeError {
        expected: &'static str,
        op: &'static str,
    },
    /// Индексация значения, которое не индексируется (не `Массив`).
    NotIndexable,
    IndexOutOfBounds {
        index: i64,
        len: usize,
    },
    /// Индекс — не целое неотрицательное число.
    BadIndex,
    /// Доступ к полю значения, у которого полей нет (не `Структура`).
    NotAnObject,
    /// Обращение к полю, которого нет в форме структуры.
    UnknownField(NameId),
    /// `ВызватьИсключение <значение>;` — значение, с которым бросили.
    Raised(BslValue),
    /// Обращение к `СтрокаТаблицы`, чья строка уже удалена (`row_id` не
    /// резолвится в `id_to_pos`) — не тихое чтение чужих данных.
    RowInvalidated,
    /// Обращение к несуществующей колонке `ТаблицыЗначений`/`СтрокиТаблицы`.
    UnknownColumn(String),
    /// `Тип("ОпечаткаВИмени")` — такого типа в реестре нет.
    UnknownType(String),
    /// Дата вышла за `0001-01-01 .. 9999-12-31` — при построении
    /// (`Дата(0, 1, 1)`), при сдвиге (`Дата + огромное число`) или при
    /// `ДобавитьМесяц`. Заворачивать в другой конец диапазона нельзя:
    /// молчаливое `9999-12-31 + 1 сутки = 0001-01-01` дало бы неверные
    /// сравнения там, где ожидалась ошибка.
    DateOutOfRange { op: &'static str },
    /// Метод объекта существует, но не для этого типа получателя, либо
    /// вызван не с тем числом аргументов для этого типа (некоторые методы,
    /// например `Добавить`, полиморфны: означают разное в зависимости от
    /// типа получателя, и арность из-за этого проверяется в рантайме, не
    /// на этапе резолвинга).
    MethodNotApplicable {
        method: &'static str,
        receiver: &'static str,
    },
    /// Ошибка лексера/парсера/резолвинга/компиляции строки, переданной в
    /// `Выполнить`/`Вычислить` — текст уже отформатирован тем слоем, что
    /// её обнаружил (`bsl-syntax`/`bsl-sema`/`bsl-bytecode`), сюда попадает
    /// как есть: `bsl-rt` не знает про эти крейты (обратная зависимость).
    DynamicError(String),
    /// Инструкция сослалась на то, чего в её `Program`/`Chunk` нет: номер
    /// регистра за границей стека кадра, номер чанка/константы/формы/имени
    /// за границей таблицы, недостача аргументов у builtin'а. Корректный
    /// кодоген такого не порождает — но VM исполняет и байт-код, собранный
    /// в рантайме (`Выполнить`/`Вычислить`, REPL `bsl-cli`), и предъявленный
    /// извне через публичный `run_program`/`run_repl_chunk`, поэтому это
    /// ошибка, а не паника: уронить процесс на кривом входе — не вариант.
    InvalidBytecode(&'static str),
    /// Ошибка открытия, записи или закрытия файла.
    IoError(String),
    /// `Формат(x, "Л=de_DE")` — локаль, которой здесь нет. Поддержаны
    /// русская и английская, см. `Locale` (там же метка о том, что набор
    /// кодов НЕ ИЗМЕРЕН).
    UnsupportedLocale(String),
}

impl From<NumError> for RtError {
    fn from(e: NumError) -> Self {
        RtError::Num(e)
    }
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RtError::Num(e) => write!(f, "{e}"),
            RtError::TypeError { expected, op } => {
                write!(f, "ожидался тип «{expected}» для операции «{op}»")
            }
            RtError::NotIndexable => write!(f, "значение не поддерживает индексацию"),
            RtError::IndexOutOfBounds { index, len } => {
                write!(f, "индекс {index} вне границ (длина {len})")
            }
            RtError::BadIndex => write!(f, "индекс должен быть целым неотрицательным числом"),
            RtError::NotAnObject => write!(f, "значение не поддерживает доступ к полям"),
            RtError::UnknownField(_) => write!(f, "поле не найдено в структуре"),
            RtError::Raised(v) => write!(f, "{v}"),
            RtError::RowInvalidated => write!(f, "строка таблицы значений больше не существует"),
            RtError::UnknownColumn(name) => write!(f, "колонка «{name}» не найдена"),
            RtError::UnknownType(name) => write!(f, "тип «{name}» не определён"),
            RtError::DateOutOfRange { op } => write!(
                f,
                "результат «{op}» вне диапазона дат (0001-01-01 .. 9999-12-31)"
            ),
            RtError::MethodNotApplicable { method, receiver } => {
                write!(f, "метод «{method}» не применим к «{receiver}»")
            }
            RtError::DynamicError(msg) => write!(f, "{msg}"),
            RtError::InvalidBytecode(what) => write!(f, "некорректный байт-код: {what}"),
            RtError::IoError(msg) => write!(f, "ошибка файлового ввода-вывода: {msg}"),
            RtError::UnsupportedLocale(code) => write!(
                f,
                "локаль «{code}» не поддержана: есть только ru и en"
            ),
        }
    }
}

impl std::error::Error for RtError {}

pub type RtResult<T> = Result<T, RtError>;

impl BslValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            BslValue::Undefined => "Неопределено",
            BslValue::Null => "Null",
            BslValue::Boolean(_) => "Булево",
            BslValue::Number(_) => "Число",
            BslValue::Str(_) => "Строка",
            BslValue::Date(_) => "Дата",
            BslValue::Type(_) => "Тип",
            BslValue::Object(o) => match &**o {
                BslObject::Array(_) => "Массив",
                BslObject::Structure(_) => "Структура",
                BslObject::ValueTable(_) => "ТаблицаЗначений",
                BslObject::TableColumns(_) => "КоллекцияКолонокТаблицыЗначений",
                BslObject::TableRow(_, _) => "СтрокаТаблицыЗначений",
                BslObject::Map(_) => "Соответствие",
                BslObject::KeyValuePair(_, _) => "КлючИЗначение",
                BslObject::TextWriter(_) => "ЗаписьТекста",
            },
            BslValue::Skipped => "Skipped",
        }
    }

    fn as_number(&self, op: &'static str) -> RtResult<&BslNumber> {
        match self {
            BslValue::Number(n) => Ok(n),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op,
            }),
        }
    }

    fn as_bool(&self, op: &'static str) -> RtResult<bool> {
        match self {
            BslValue::Boolean(b) => Ok(*b),
            _ => Err(RtError::TypeError {
                expected: "Булево",
                op,
            }),
        }
    }

    /// `+` между двумя строками — конкатенация (реальная 1С считает это
    /// перегрузкой того же оператора, не отдельной функцией). Любая другая
    /// комбинация типов идёт по числовому пути и получает его же ошибку
    /// типа, если не подходит.
    pub fn add(&self, other: &Self) -> RtResult<Self> {
        if let (BslValue::Str(a), BslValue::Str(b)) = (self, other) {
            return Ok(BslValue::Str(a.concat(b)));
        }
        // `Дата + Число` — сдвиг на N СЕКУНД (не дней: разрешение типа —
        // секунда, и `Дата - Дата` симметрично отдаёт секунды).
        if let BslValue::Date(d) = self {
            let secs = Self::whole_seconds(other, "+")?;
            return Self::shifted(*d, secs, "+");
        }
        Ok(BslValue::Number(
            self.as_number("+")?.add(other.as_number("+")?)?,
        ))
    }

    pub fn sub(&self, other: &Self) -> RtResult<Self> {
        if let BslValue::Date(a) = self {
            return match other {
                // `Дата - Дата` -> Число секунд между ними.
                BslValue::Date(b) => Ok(BslValue::Number(BslNumber::from_i64(a.diff_seconds(*b)))),
                // `Дата - Число` -> Дата.
                _ => {
                    let secs = Self::whole_seconds(other, "-")?;
                    Self::shifted(*a, -secs, "-")
                }
            };
        }
        Ok(BslValue::Number(
            self.as_number("-")?.sub(other.as_number("-")?)?,
        ))
    }

    /// Слагаемое к дате обязано быть ЦЕЛЫМ числом секунд: у типа нет
    /// разрешения мельче секунды, и тихо отбрасывать дробную часть значит
    /// делать `Дата + 0.4` неотличимым от `Дата + 0`.
    fn whole_seconds(v: &Self, op: &'static str) -> RtResult<i64> {
        v.as_number(op)?.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Число (целое количество секунд)",
            op,
        })
    }

    /// Выход за границы `0001-01-01 .. 9999-12-31` — ошибка, а не тихое
    /// заворачивание в другой конец диапазона.
    fn shifted(d: BslDate, secs: i64, op: &'static str) -> RtResult<Self> {
        d.shift_seconds(secs)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op })
    }

    pub fn mul(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("*")?.mul(other.as_number("*")?)?,
        ))
    }

    pub fn div(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("/")?.div(other.as_number("/")?)?,
        ))
    }

    /// Специализированный шаг числового `Для`, сохраняющий проверку типов,
    /// если тело цикла переприсвоило переменную-счётчик.
    #[inline]
    pub fn increment_numeric_for_and_le(&mut self, bound: &Self) -> RtResult<bool> {
        let bound = bound.as_number("Для")?;
        match self {
            BslValue::Number(counter) => Ok(counter.increment_and_le(bound)?),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op: "Для",
            }),
        }
    }

    pub fn neg(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("унарный -")?.neg()))
    }

    pub fn not(&self) -> RtResult<Self> {
        Ok(BslValue::Boolean(!self.as_bool("Не")?))
    }

    // --- Трансцендентные функции (через f64 в bsl-number) ------------------

    pub fn sqrt(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sqrt")?.sqrt()?))
    }

    pub fn pow(&self, exp: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Pow")?.pow(exp.as_number("Pow")?)?))
    }

    pub fn ln(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log")?.ln()?))
    }

    pub fn log10(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log10")?.log10()?))
    }

    pub fn exp(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Exp")?.exp()?))
    }

    /// `Окр(Число, ЧислоРазрядов, Режим)` — decimal, НЕ через `f64`:
    /// `Окр(2.675, 2)` обязан дать `2.68`, а не `2.67` (ближайший `f64` к
    /// `2.675` чуть меньше самого числа).
    ///
    /// Все три аргумента здесь всегда есть: недостающие подставляет
    /// `bsl-sema::resolver::resolve_call` литеральным `0` (см. там же, почему
    /// не вариативная арность).
    ///
    /// Кодировка режимов ИЗМЕРЕНА на платформе: `0` — половина к нулю,
    /// `1` — половина от нуля, опущенный аргумент — как `1`, а НЕ как `0`.
    ///
    /// Неизвестный код (`Окр(2.5, 0, 7)`) платформа не считает ошибкой и
    /// округляет по умолчанию — измерено, поэтому здесь тоже не ошибка.
    /// Раньше тут стояло исключение, и это было расхождение.
    pub fn round(&self, digits: &Self, mode: &Self) -> RtResult<Self> {
        let n = self.as_number("Окр")?;
        // Опущенное число разрядов — ноль (`Окр(2.5)` округляет до целого).
        let scale = match digits {
            BslValue::Undefined => 0,
            other => Self::round_arg_as_i32(other)?,
        };
        // Опущенный третий аргумент приходит `Неопределено` (см.
        // `bsl-sema::resolver::resolve_call`): подставлять вместо него `0`
        // нельзя — это ДРУГОЙ режим.
        let mode = match mode {
            BslValue::Undefined => bsl_number::DEFAULT_ROUND_MODE,
            other => match Self::round_arg_as_i32(other)? {
                0 => bsl_number::RoundMode::HalfDown,
                1 => bsl_number::RoundMode::HalfUp,
                _ => bsl_number::DEFAULT_ROUND_MODE,
            },
        };
        Ok(BslValue::Number(match mode {
            bsl_number::RoundMode::HalfUp => n.round_to_scale(scale),
            bsl_number::RoundMode::HalfDown => n.round_to_scale_half_down(scale),
        }))
    }

    fn round_arg_as_i32(v: &Self) -> RtResult<i32> {
        v.as_number("Окр")?
            .to_i64_exact()
            .and_then(|s| i32::try_from(s).ok())
            .ok_or(RtError::TypeError {
                expected: "Число (целое)",
                op: "Окр",
            })
    }

    /// `Цел(Число)` — отбрасывание дробной части К НУЛЮ (не half-up, в
    /// отличие от `Окр` выше): `Цел(2.9) = 2`, `Цел(-2.9) = -2`.
    pub fn trunc(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Цел")?.trunc_to_scale(0)))
    }

    pub fn sin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sin")?.sin()?))
    }

    pub fn cos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Cos")?.cos()?))
    }

    pub fn tan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Tan")?.tan()?))
    }

    pub fn asin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ASin")?.asin()?))
    }

    pub fn acos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ACos")?.acos()?))
    }

    pub fn atan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ATan")?.atan()?))
    }

    /// `И`/`ИЛИ` в 1С короткозамкнутые, поэтому у `BslValue` больше нет
    /// `and`/`or`: комбинирование живёт в потоке управления кодогена
    /// (`Instr::JumpIfFalse`/`JumpIfTrue` в `bsl-bytecode::compiler`), а не
    /// здесь, — правый операнд физически не вычисляется, если левый уже
    /// решил результат.
    pub fn as_condition(&self) -> RtResult<bool> {
        self.as_bool("Условие")
    }

    /// Сравнение по значению для `=`/`<>`. Разнотипные значения просто не
    /// равны — это не ошибка (в отличие от `<`/`>`/... между разными
    /// типами). `Массив`/`Структура` — ссылочные типы: равенство — это
    /// тождество объекта (`Rc::ptr_eq`), а не структурное сравнение
    /// содержимого (см. `PartialEq` ниже, эта функция ему делегирует).
    pub fn eq_value(&self, other: &Self) -> bool {
        self == other
    }

    /// Сравнение строк — упорядочивание код-юнитов UTF-16, без учёта
    /// локали (настоящая коллация для `Сортировать` в `ТаблицаЗначений` —
    /// отдельная, ещё не сделанная задача).
    pub fn compare(&self, other: &Self, op: &'static str) -> RtResult<Ordering> {
        match (self, other) {
            (BslValue::Number(a), BslValue::Number(b)) => Ok(a.cmp(b)),
            (BslValue::Str(a), BslValue::Str(b)) => Ok(a.cmp(b)),
            // Даты сравниваются по значению (моменту времени) — секунды от
            // общей эпохи, поэтому это обычное сравнение `i64`.
            (BslValue::Date(a), BslValue::Date(b)) => Ok(a.cmp(b)),
            _ => Err(RtError::TypeError {
                expected: "Число, Строка или Дата",
                op,
            }),
        }
    }

    fn as_str(&self, op: &'static str) -> RtResult<&BslString> {
        match self {
            BslValue::Str(s) => Ok(s),
            _ => Err(RtError::TypeError {
                expected: "Строка",
                op,
            }),
        }
    }

    fn as_usize(&self, op: &'static str) -> RtResult<usize> {
        let n = self.as_number(op)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    // --- Строки ---------------------------------------------------------

    pub fn str_len(&self) -> RtResult<usize> {
        Ok(self.as_str("СтрДлина")?.len_utf16())
    }

    pub fn str_left(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("Лев")?.left(len.as_usize("Лев")?)))
    }

    pub fn str_right(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("Прав")?.right(len.as_usize("Прав")?)))
    }

    /// `Сред(Строка, Начало[, Длина])` — `Неопределено` на месте длины
    /// (аргумент опущен, см. `BuiltinFn::arity_range`) значит "до конца
    /// строки".
    pub fn str_mid(&self, start: &Self, len: &Self) -> RtResult<Self> {
        let s = self.as_str("Сред")?;
        let start = start.as_usize("Сред")?;
        let len = match len {
            BslValue::Undefined => s.len_utf16(),
            other => other.as_usize("Сред")?,
        };
        Ok(BslValue::Str(s.substring(start, len)))
    }

    pub fn str_upper(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("ВРег")?.to_uppercase()))
    }

    pub fn str_lower(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("НРег")?.to_lowercase()))
    }

    pub fn str_trim_all(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛП")?.trim()))
    }

    pub fn str_trim_left(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛ")?.trim_start()))
    }

    pub fn str_trim_right(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрП")?.trim_end()))
    }

    /// `СтрНайти(Строка, Подстрока)` — позиция в КОД-ЮНИТАХ UTF-16, 1-based,
    /// `0` если не найдено. Те же единицы, что считает `СтрДлина`, чтобы
    /// результат можно было передать в `Сред`/`Лев` без пересчёта.
    pub fn str_find(&self, needle: &Self) -> RtResult<Self> {
        let pos = self.as_str("СтрНайти")?.find(needle.as_str("СтрНайти")?);
        Ok(BslValue::Number(BslNumber::from_i64(pos as i64)))
    }

    pub fn str_replace(&self, from: &Self, to: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СтрЗаменить")?.replace(
            from.as_str("СтрЗаменить")?,
            to.as_str("СтрЗаменить")?,
        )))
    }

    /// `СтрРазделить(Строка, Разделитель)` -> `Массив` строк.
    pub fn str_split(&self, sep: &Self) -> RtResult<Self> {
        let parts = self.as_str("СтрРазделить")?.split(sep.as_str("СтрРазделить")?);
        Ok(BslValue::new_array(parts.into_iter().map(BslValue::Str).collect()))
    }

    /// `СтрСоединить(Массив, Разделитель)`. Не-строковые элементы массива
    /// приводятся к строке через `Display` — у 1С `СтрСоединить` тоже
    /// принимает массив любых значений, а не только строк. ВНИМАНИЕ:
    /// приведение здесь идёт МИМО `bsl-format` (этот крейт ниже него
    /// слоем), поэтому число получает каноническую форму, а не
    /// локализованную с NBSP-группировкой — см. `Display for BslNumber`.
    /// Это осознанное расхождение уровня слоёв, а не забытая локализация:
    /// массив строк (обычный случай) оно не затрагивает вовсе.
    pub fn str_join(&self, sep: &Self) -> RtResult<Self> {
        let sep = sep.as_str("СтрСоединить")?;
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let parts: Vec<BslString> = items
                        .borrow()
                        .iter()
                        .map(|v| match v {
                            BslValue::Str(s) => s.clone(),
                            other => BslString::from_str(&other.to_string()),
                        })
                        .collect();
                    Ok(BslValue::Str(BslString::join(&parts, sep)))
                }
                _ => Err(RtError::TypeError {
                    expected: "Массив",
                    op: "СтрСоединить",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Массив",
                op: "СтрСоединить",
            }),
        }
    }

    pub fn str_line_count(&self) -> RtResult<Self> {
        let n = self.as_str("СтрЧислоСтрок")?.line_count();
        Ok(BslValue::Number(BslNumber::from_i64(n as i64)))
    }

    pub fn str_get_line(&self, n: &Self) -> RtResult<Self> {
        let s = self.as_str("СтрПолучитьСтроку")?;
        let n = n.as_usize("СтрПолучитьСтроку")?;
        Ok(BslValue::Str(s.line_at(n)))
    }

    /// `СтрШаблон(Шаблон, З1, ..., З10)` — значения уже дополнены до
    /// `MAX_TEMPLATE_ARGS` штук `Неопределено` резолвером (см.
    /// `BuiltinFn::arity_range`); хвостовые `Неопределено` отбрасываются
    /// здесь, чтобы `%3` при двух реально переданных значениях дал пусто, а
    /// не строковое представление `Неопределено` (оно, впрочем, тоже
    /// пустое — но полагаться на это совпадение не нужно).
    pub fn str_template(&self, values: &[Self]) -> RtResult<Self> {
        let tmpl = self.as_str("СтрШаблон")?;
        let end = values
            .iter()
            .rposition(|v| !matches!(v, BslValue::Undefined))
            .map(|i| i + 1)
            .unwrap_or(0);
        let vals: Vec<BslString> = values[..end]
            .iter()
            .map(|v| match v {
                BslValue::Str(s) => s.clone(),
                other => BslString::from_str(&other.to_string()),
            })
            .collect();
        Ok(BslValue::Str(tmpl.template(&vals)))
    }

    /// `Символ(Код)`.
    pub fn char_from_code(&self) -> RtResult<Self> {
        let code = self.as_number("Символ")?;
        let code = code
            .to_i64_exact()
            .and_then(|c| u32::try_from(c).ok())
            .and_then(BslString::from_char_code)
            .ok_or(RtError::TypeError {
                expected: "Код символа (целое в диапазоне Unicode)",
                op: "Символ",
            })?;
        Ok(BslValue::Str(code))
    }

    /// `КодСимвола(Строка[, Позиция])` — позиция по умолчанию `1`.
    pub fn char_code(&self, pos: &Self) -> RtResult<Self> {
        let s = self.as_str("КодСимвола")?;
        let pos = match pos {
            BslValue::Undefined => 1,
            other => other.as_usize("КодСимвола")?,
        };
        let code = s.char_code_at(pos).ok_or(RtError::IndexOutOfBounds {
            index: pos as i64,
            len: s.len_utf16(),
        })?;
        Ok(BslValue::Number(BslNumber::from_i64(code as i64)))
    }

    // --- Даты -------------------------------------------------------------

    fn as_date(&self, op: &'static str) -> RtResult<BslDate> {
        match self {
            BslValue::Date(d) => Ok(*d),
            _ => Err(RtError::TypeError {
                expected: "Дата",
                op,
            }),
        }
    }

    /// Компонента `Дата(...)` — целое число, помещающееся в календарный
    /// диапазон. Нецелое (`Дата(2024.5, 1, 1)`) — ошибка, а не усечение.
    fn date_part(v: &Self, op: &'static str) -> RtResult<i64> {
        v.as_number(op)?.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Число (целое)",
            op,
        })
    }

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])` — шестиместная
    /// форма, и `Дата("ГГГГММДД[ЧЧММСС]")` — строковая.
    ///
    /// Обе формы живут в одной функции, потому что в 1С это одна и та же
    /// встроенная функция с перегрузкой по типу первого аргумента, а не две
    /// разных. Опущенные `Час`/`Минута`/`Секунда` приходят сюда как
    /// `Неопределено` (см. `BuiltinFn::arity_range`) и означают ноль.
    pub fn make_date(args: &[BslValue]) -> RtResult<Self> {
        // Строковая форма: `Дата("20240115103000")`. Остальные позиции при
        // ней обязаны быть пустыми — `Дата("20240115", 2, 3)` бессмысленно.
        if let BslValue::Str(s) = &args[0] {
            if args[1..].iter().any(|a| !matches!(a, BslValue::Undefined)) {
                return Err(RtError::TypeError {
                    expected: "Дата(«ГГГГММДДЧЧММСС») без остальных аргументов",
                    op: "Дата",
                });
            }
            return BslDate::parse_digits(&s.to_string())
                .map(BslValue::Date)
                .ok_or(RtError::DateOutOfRange { op: "Дата" });
        }

        let year = Self::date_part(&args[0], "Дата")?;
        let month = Self::date_part(&args[1], "Дата")?;
        let day = Self::date_part(&args[2], "Дата")?;
        // Час/минута/секунда необязательны — опущенные значат ноль.
        let mut time = [0i64; 3];
        for (i, slot) in time.iter_mut().enumerate() {
            *slot = match &args[3 + i] {
                BslValue::Undefined => 0,
                other => Self::date_part(other, "Дата")?,
            };
        }
        let fits = |v: i64| u32::try_from(v).ok();
        let built = (|| {
            BslDate::from_civil(
                year,
                fits(month)?,
                fits(day)?,
                fits(time[0])?,
                fits(time[1])?,
                fits(time[2])?,
            )
        })();
        built
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op: "Дата" })
    }

    /// `ТекущаяДата()`.
    ///
    /// ОТКЛОНЕНИЕ, о котором надо знать: возвращается момент по UTC, а не
    /// по локальной зоне машины. Дата в 1С — наивный локальный момент без
    /// зоны (см. модуль `date`), а в `std` нет способа узнать смещение
    /// локальной зоны; тащить ради этого `chrono`/`libc` значит тащить
    /// целую модель времени с зонами, которой в типе всё равно нет.
    /// Наблюдаемо это только как сдвиг на смещение зоны, и только у
    /// `ТекущаяДата` — все остальные функции работают с датами, которые им
    /// дали.
    pub fn current_date() -> RtResult<Self> {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        BslDate::from_seconds(secs + date::UNIX_EPOCH_SECONDS)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op: "ТекущаяДата" })
    }

    /// `ТекущаяУниверсальнаяДатаВМиллисекундах()` — целое число
    /// миллисекунд от Unix-эпохи в UTC.
    pub fn current_universal_date_in_milliseconds() -> RtResult<Self> {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let millis = i64::try_from(millis).map_err(|_| RtError::DateOutOfRange {
            op: "ТекущаяУниверсальнаяДатаВМиллисекундах",
        })?;
        Ok(BslValue::Number(BslNumber::from_i64(millis)))
    }

    /// `Год`/`Месяц`/`День`/`Час`/`Минута`/`Секунда`/`ДеньНедели` — все
    /// возвращают `Число`, поэтому одна функция с селектором вместо шести
    /// почти одинаковых.
    pub fn date_component(&self, part: DatePart) -> RtResult<Self> {
        let d = self.as_date(part.op())?;
        let c = d.to_civil();
        let n = match part {
            DatePart::Year => c.year,
            DatePart::Month => c.month as i64,
            DatePart::Day => c.day as i64,
            DatePart::Hour => c.hour as i64,
            DatePart::Minute => c.minute as i64,
            DatePart::Second => c.second as i64,
            DatePart::Weekday => d.weekday() as i64,
        };
        Ok(BslValue::Number(BslNumber::from_i64(n)))
    }

    /// `НачалоДня`/`КонецДня`/`НачалоМесяца`/... — тоже один селектор:
    /// все семь границ отличаются только тем, что округляют.
    pub fn date_boundary(&self, which: DateBoundary) -> RtResult<Self> {
        let d = self.as_date(which.op())?;
        Ok(BslValue::Date(match which {
            DateBoundary::StartOfDay => d.start_of_day(),
            DateBoundary::EndOfDay => d.end_of_day(),
            DateBoundary::StartOfMonth => d.start_of_month(),
            DateBoundary::EndOfMonth => d.end_of_month(),
            DateBoundary::StartOfYear => d.start_of_year(),
            DateBoundary::EndOfYear => d.end_of_year(),
            DateBoundary::StartOfWeek => d.start_of_week(),
        }))
    }

    /// `ДобавитьМесяц(Дата, Количество)` — про зажатие дня см.
    /// `BslDate::add_months` (там же пометка НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP)).
    pub fn add_month(&self, count: &Self) -> RtResult<Self> {
        let d = self.as_date("ДобавитьМесяц")?;
        let n = Self::date_part(count, "ДобавитьМесяц")?;
        d.add_months(n)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ДобавитьМесяц",
            })
    }

    // --- Проверки значения и типа ----------------------------------------

    /// `ЗначениеЗаполнено(Значение)`.
    ///
    /// ИЗМЕРЕНО (из брифа): `Ложь` для `Неопределено`, `Null`, пустой
    /// строки, нуля и пустой даты.
    ///
    /// Всё остальное — три открытых вопроса,
    /// НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BOOLEAN),
    /// НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BLANK_STRING) и
    /// НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.EMPTY_COLLECTION), каждый со своей веткой
    /// ниже. Ошибиться здесь дорого: это главный потребитель
    /// короткозамкнутых `И`/`ИЛИ` (`Если ЗначениеЗаполнено(Х) И Х.Поле = 1`),
    /// поэтому спорные ветки помечены поимённо, а не «примерно так».
    pub fn is_filled(&self) -> RtResult<bool> {
        Ok(match self {
            BslValue::Undefined | BslValue::Null => false,
            // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BOOLEAN): считается ли `Ложь`
            // незаполненным. Взято
            // «булево заполнено всегда» — иначе `ЗначениеЗаполнено(Флаг)`
            // нельзя было бы отличить от «флага нет вовсе», а именно ради
            // этого различия функция и существует.
            BslValue::Boolean(_) => true,
            BslValue::Number(n) => !n.is_zero(),
            // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BLANK_STRING): пусты ли строки из
            // одних пробелов. Взято «да,
            // пусты» (сравнение после `СокрЛП`) — это поведение, которого
            // ждут от защитной проверки введённого пользователем текста.
            BslValue::Str(s) => s.trim().len_utf16() > 0,
            // ИЗМЕРЕНО (из брифа): пустая дата не заполнена. Ровно ради
            // этой строчки эпоха и сдвинута на `0001-01-01` — «пусто» это
            // просто ноль, без отдельной константы.
            BslValue::Date(d) => !d.is_empty(),
            // Тип — всегда значение, «пустого типа» не существует.
            BslValue::Type(_) => true,
            BslValue::Object(o) => match &**o {
                // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.EMPTY_COLLECTION): пустая
                // коллекция. Взято «пустая — не
                // заполнена» (по числу элементов), потому что для
                // коллекций это единственное содержательное прочтение;
                // альтернатива («объект есть — значит заполнено») делает
                // проверку тождественно истинной и бесполезной.
                BslObject::Array(_)
                | BslObject::Structure(_)
                | BslObject::Map(_)
                | BslObject::ValueTable(_)
                | BslObject::TableColumns(_) => self.collection_len()? > 0,
                // У строки таблицы и пары ключ-значение «длины» нет: сам
                // факт существования объекта и есть заполненность.
                BslObject::TableRow(..)
                | BslObject::KeyValuePair(..)
                | BslObject::TextWriter(..) => true,
            },
            BslValue::Skipped => {
                return Err(RtError::TypeError {
                    expected: "Значение",
                    op: "ЗначениеЗаполнено",
                })
            }
        })
    }

    /// `ТипЗнч(Значение)` -> `Тип`.
    pub fn type_of(&self) -> RtResult<Self> {
        let id = match self {
            BslValue::Undefined => TypeId::Undefined,
            BslValue::Null => TypeId::Null,
            BslValue::Boolean(_) => TypeId::Boolean,
            BslValue::Number(_) => TypeId::Number,
            BslValue::Str(_) => TypeId::String,
            BslValue::Date(_) => TypeId::Date,
            BslValue::Type(_) => TypeId::Type,
            BslValue::Object(o) => match &**o {
                BslObject::Array(_) => TypeId::Array,
                BslObject::Structure(_) => TypeId::Structure,
                BslObject::Map(_) => TypeId::Map,
                BslObject::ValueTable(_) => TypeId::ValueTable,
                BslObject::TableColumns(_) => TypeId::ValueTableColumns,
                BslObject::TableRow(..) => TypeId::ValueTableRow,
                BslObject::KeyValuePair(..) => TypeId::KeyAndValue,
                BslObject::TextWriter(..) => {
                    return Err(RtError::TypeError {
                        expected: "Зарегистрированный тип",
                        op: "ТипЗнч",
                    })
                }
            },
            BslValue::Skipped => {
                return Err(RtError::TypeError {
                    expected: "Значение",
                    op: "ТипЗнч",
                })
            }
        };
        Ok(BslValue::Type(id))
    }

    /// `Тип("ИмяТипа")` -> `Тип`. Неизвестное имя — ошибка (в 1С тоже
    /// исключение, а не `Неопределено`: опечатка в имени типа обязана
    /// падать сразу, а не молча делать сравнение вечно ложным).
    pub fn type_by_name(&self) -> RtResult<Self> {
        let name = self.as_str("Тип")?.to_string();
        TypeId::lookup(&name)
            .map(BslValue::Type)
            .ok_or(RtError::UnknownType(name))
    }

    // --- Коллекции ----------------------------------------------------

    pub fn new_array(items: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Array(std::cell::RefCell::new(items))))
    }

    pub fn new_structure(shape: Rc<Shape>, slots: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Structure(std::cell::RefCell::new(
            StructureStorage::new(shape, slots),
        ))))
    }

    pub fn new_table() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueTable(ValueTableData::new())))
    }

    pub fn new_map() -> Self {
        BslValue::Object(Rc::new(BslObject::Map(std::cell::RefCell::new(MapData::new()))))
    }

    /// Создаёт объект `ЗаписьТекста` и открывает файл для буферизованной
    /// записи UTF-8.
    ///
    /// Существующий файл обрезается до нулевой длины.
    ///
    /// # Errors
    ///
    /// Возвращает [`RtError::TypeError`], если путь не является строкой,
    /// или [`RtError::IoError`], если файл невозможно создать.
    pub fn new_text_writer(path: &BslValue) -> RtResult<Self> {
        let path = path.as_str("Новый ЗаписьТекста")?.to_string();
        let file = std::fs::File::create(&path)
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::Object(Rc::new(BslObject::TextWriter(
            std::cell::RefCell::new(Some(std::io::BufWriter::new(file))),
        ))))
    }

    /// Записывает строку в буфер объекта `ЗаписьТекста`.
    ///
    /// UTF-16-представление [`BslString`] кодируется непосредственно в
    /// UTF-8 без промежуточного [`String`].
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для нестрокового аргумента, ошибку
    /// применимости для другого объекта либо [`RtError::IoError`] при
    /// записи в закрытый файл или ошибке файловой системы.
    pub fn text_writer_write(&self, text: &BslValue) -> RtResult<Self> {
        let text = text.as_str("Записать")?;
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    let mut writer = writer.borrow_mut();
                    text.write_utf8(
                        writer
                        .as_mut()
                        .ok_or_else(|| RtError::IoError("файл уже закрыт".to_string()))?,
                    )
                        .map_err(|e| RtError::IoError(e.to_string()))?;
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Записать",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Записать",
                receiver: self.type_name(),
            }),
        }
    }

    /// Сбрасывает буфер и закрывает `ЗаписьТекста`.
    ///
    /// Повторный вызов безопасен и возвращает `Неопределено`.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку применимости для другого объекта либо
    /// [`RtError::IoError`], если буфер не удалось сбросить.
    pub fn text_writer_close(&self) -> RtResult<Self> {
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    if let Some(mut writer) = writer.borrow_mut().take() {
                        writer.flush().map_err(|e| RtError::IoError(e.to_string()))?;
                    }
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Закрыть",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Закрыть",
                receiver: self.type_name(),
            }),
        }
    }

    /// Индекс должен быть целым неотрицательным числом — `1С` использует
    /// `Число` для индексов, отдельного целочисленного типа нет.
    fn index_as_usize(idx: &BslValue) -> RtResult<usize> {
        let n = idx.as_number("[]").map_err(|_| RtError::BadIndex)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    /// `names` нужен единственной ветке — `Структура`: её поля хранятся
    /// идентификаторами (`NameId`), а `Для Каждого` обязан отдать ключ
    /// пользовательскому коду СТРОКОЙ (`КлючИЗначение.Ключ`). Тащить сюда
    /// интернер целиком дешевле, чем держать в каждой структуре ещё и
    /// строковые имена рядом с формой.
    pub fn get_index(&self, idx: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let v = v.borrow();
                    let i = Self::index_as_usize(idx)?;
                    v.get(i)
                        .cloned()
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: v.len() })
                }
                BslObject::ValueTable(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let row_id = {
                        let d = data.borrow();
                        d.row_id_at(i)
                            .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: d.row_count() })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))))
                }
                // ПОЗИЦИОННЫЙ, не по ключу: `Для Каждого` компилируется в
                // общий для всех коллекций протокол `CollectionLen` + рост
                // числового индекса `0..len` через эту же функцию (см.
                // `bsl-bytecode::compiler::RStmt::ForEach`) — компилятор не
                // знает на этапе компиляции, что `idx` в рантайме окажется
                // `Соответствие`, и не может эмитить для него другой путь.
                // Доступ ПО КЛЮЧУ у `Соответствие` поэтому сознательно НЕ
                // здесь, а в `.Получить(Ключ)` (`map_get`) — если бы `[]`
                // тоже читал по ключу, `м[0]` было бы неразрешимо
                // неоднозначно между "0-я по счёту пара" и "значение по
                // ключу 0" для карты с целочисленными ключами.
                BslObject::Map(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let data = data.borrow();
                    let (k, v) = data
                        .entry_at(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: data.len() })?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(k, v))))
                }
                // `Для Каждого КиЗ Из Структура` — тот же протокол, что и у
                // `Соответствие` (`CollectionLen` + позиционный обход), и та
                // же пара `Ключ`/`Значение` на выходе. Порядок — вставки, в
                // обоих режимах хранения (`StructureStorage::entry_at`).
                BslObject::Structure(s) => {
                    let i = Self::index_as_usize(idx)?;
                    let s = s.borrow();
                    let (n, v) = s
                        .entry_at(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: s.len() })?;
                    let key = names.name(n).ok_or(RtError::UnknownField(n))?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(
                        BslValue::Str(BslString::from_str(key)),
                        v,
                    ))))
                }
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn set_index(&self, idx: &BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    let slot = v
                        .get_mut(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len })?;
                    *slot = val;
                    Ok(())
                }
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    /// Длина коллекции — используется и `Для Каждого` (компилируется в
    /// индексный цикл поверх этой длины), и `Количество()`.
    pub fn collection_len(&self) -> RtResult<usize> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => Ok(v.borrow().len()),
                BslObject::Structure(s) => Ok(s.borrow().len()),
                BslObject::ValueTable(data) => Ok(data.borrow().row_count()),
                BslObject::TableColumns(data) => Ok(data.borrow().column_names.len()),
                BslObject::TableRow(..) => Err(RtError::NotIndexable),
                BslObject::Map(data) => Ok(data.borrow().len()),
                BslObject::KeyValuePair(..) => Err(RtError::NotIndexable),
                BslObject::TextWriter(..) => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn get_field(&self, name: NameId) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow().get(name).ok_or(RtError::UnknownField(name))
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field(&self, name: NameId, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    if s.borrow_mut().set(name, val) {
                        Ok(())
                    } else {
                        Err(RtError::UnknownField(name))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Рантайм-мутация формы структуры ---------------------------------
    //
    // `Вставить`/`Удалить`/`Свойство` (двухаргументная форма) — в отличие
    // от `get_field`/`set_field` выше, которые лишь ЧИТАЮТ уже готовую
    // форму, эти три меняют её: `ShapeTable` здесь больше не голая таблица
    // компиляции, а рантайм-контекст (`RuntimeShapes`, см. одноимённый
    // модуль), поэтому и подписи ниже берут `&mut ShapeTable`, а не
    // работают в изоляции. Инлайн-кэш `GetProp`/`SetProp` (`Rc::ptr_eq` на
    // `s.shape`) сам заметит смену формы после любой из них — ничего
    // специально инвалидировать не нужно.

    /// `Структура.Вставить(Ключ, Значение)`. Поле уже есть — просто новое
    /// значение на том же слоте, форма не меняется (у 1С `Вставить`
    /// повторного поля — это не ошибка и не дубликат, а перезапись).
    pub fn structure_insert(&self, field: NameId, val: BslValue, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().insert(field, val, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Удалить(Ключ)`. Поля нет — no-op (симметрично
    /// `MapData::remove`, см. его doc comment): убрать то, чего и так нет,
    /// не повод падать.
    pub fn structure_delete(&self, field: NameId, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().remove(field, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Свойство(Ключ)` / `Структура.Свойство(Ключ,
    /// ЗначениеПоУмолчанию)`.
    ///
    /// ОТКЛОНЕНИЕ ОТ РЕАЛЬНОЙ 1С: настоящая сигнатура — `Свойство(Ключ,
    /// Значение)`, где `Значение` — параметр ПО ССЫЛКЕ (выходной), а
    /// возвращает функция `Булево` (найдено ли поле). Такой контракт для
    /// ВСТРОЕННОГО метода потребовал бы протаскивать режим аргумента
    /// (`ArgMode::ByRefLocal`) через `CallMethod` — сейчас это умеет только
    /// `Call` для пользовательских процедур/функций (см.
    /// `bsl-bytecode::instr::Instr::Call`), `CallMethod` вычисляет все
    /// аргументы как значения. Ради одного метода вводить второй протокол
    /// аргументов не стали — вместо этого второй аргумент трактуется как
    /// значение по умолчанию (безопасный geттер: нет поля — нет и ошибки).
    pub fn structure_property(&self, field: NameId, default: Option<BslValue>) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => Ok(s
                    .borrow()
                    .get(field)
                    .unwrap_or_else(|| default.unwrap_or(BslValue::Undefined))),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Очистить()` — сбрасывает набор полей целиком (форма
    /// становится пустой), не только значения на месте: у 1С `Очистить()`
    /// на структуре убирает и сами поля, следующий `Свойство`/`.Х` их уже
    /// не найдёт.
    pub fn structure_clear(&self, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().clear(shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Вставить(Ключ, Значение)`.
    pub fn map_insert(&self, key: BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => {
                    data.borrow_mut().insert(key, val);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Получить(Ключ)` — `Неопределено`, если ключа нет, не
    /// ошибка (соответствует `MapData::get`/реальной 1С).
    pub fn map_get(&self, key: &BslValue) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => Ok(data.borrow().get(key).unwrap_or(BslValue::Undefined)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `GetProp` (см. брифовский план оптимизаций: "слот
    /// хранит (shape_ptr, slot_idx)"). `cache` — одна ячейка НА КОНКРЕТНУЮ
    /// инструкцию в чанке (см. `Chunk::prop_cache` в `bsl-bytecode`),
    /// живёт между исполнениями этой инструкции. Промах — обычный поиск
    /// по `Shape::index` плюс запись в кэш; форма меняется редко (обычно
    /// вообще никогда для данной инструкции — иначе откуда там структура
    /// другой формы), так что кэш почти всегда мономорфный.
    ///
    /// Держим `Rc<Shape>` целиком, а не голый указатель: так кэш не может
    /// протухнуть на чужой адрес, если форма где-то освободится — он сам
    /// продлевает ей жизнь, пока висит в кэше.
    ///
    /// Словарная структура (`StructureStorage::Dictionary`) формы не имеет
    /// вообще, поэтому ВСЕГДА промахивается мимо кэша и идёт в `HashMap` —
    /// и, что важнее, НЕ ТРОГАЕТ ячейку кэша. Если бы словарный объект
    /// затирал её (хоть чем — своим отсутствием формы, `None`), то шейповые
    /// объекты на том же сайте вызова теряли бы быстрый путь после каждого
    /// прохода словарного, то есть навсегда в смешанном цикле.
    pub fn get_field_cached(
        &self,
        name: NameId,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &*s.borrow() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref() {
                            if Rc::ptr_eq(cached_shape, shape) {
                                return Ok(slots[*slot as usize].clone());
                            }
                        }
                        match shape.index.get(&name) {
                            Some(&slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                Ok(slots[slot as usize].clone())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    StructureStorage::Dictionary { values, .. } => {
                        values.get(&name).cloned().ok_or(RtError::UnknownField(name))
                    }
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `SetProp` — см. `get_field_cached`.
    pub fn set_field_cached(
        &self,
        name: NameId,
        val: BslValue,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &mut *s.borrow_mut() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref() {
                            if Rc::ptr_eq(cached_shape, shape) {
                                slots[*slot as usize] = val;
                                return Ok(());
                            }
                        }
                        match shape.index.get(&name).copied() {
                            Some(slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                slots[slot as usize] = val;
                                Ok(())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    // Кэш не трогаем — см. `get_field_cached`.
                    StructureStorage::Dictionary { values, .. } => match values.get_mut(&name) {
                        Some(slot) => {
                            *slot = val;
                            Ok(())
                        }
                        None => Err(RtError::UnknownField(name)),
                    },
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Резолвинг поля/псевдо-свойства по ИМЕНИ (не `NameId`) — нужен для
    /// объектов, чьи "поля" известны только в рантайме: колонки
    /// `СтрокиТаблицыЗначений` заводятся через `.Колонки.Добавить(имя)`, а
    /// не как статичная форма структуры, поэтому по ним нельзя
    /// интернировать `NameId` на этапе компиляции. `Структура` в эту
    /// функцию не заходит — у неё есть более быстрый путь через
    /// `get_field`/`NameId`, здесь она просто не находится.
    pub fn get_field_by_name(&self, name: &str) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    if name.eq_ignore_ascii_case("Колонки") || name.eq_ignore_ascii_case("Columns")
                    {
                        Ok(BslValue::Object(Rc::new(BslObject::TableColumns(data.clone()))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::TableRow(data, row_id) => {
                    let data = data.borrow();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.get_cell(*row_id, col).ok_or(RtError::RowInvalidated)
                }
                // `КлючИЗначение.Ключ`/`.Значение` — те же два имени всегда,
                // ни разу не интернированные как `Shape`/`NameId`, потому
                // что этот объект заводится ЦЕЛИКОМ в рантайме
                // (`get_index` на `Соответствие`, см. `map.rs`), а не
                // компилируется из литерала полей.
                BslObject::KeyValuePair(k, v) => {
                    if name.eq_ignore_ascii_case("Ключ") || name.eq_ignore_ascii_case("Key") {
                        Ok(k.clone())
                    } else if name.eq_ignore_ascii_case("Значение") || name.eq_ignore_ascii_case("Value")
                    {
                        Ok(v.clone())
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field_by_name(&self, name: &str, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableRow(data, row_id) => {
                    let mut data = data.borrow_mut();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.set_cell(*row_id, col, val).ok_or(RtError::RowInvalidated)
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Методы, полиморфные по типу получателя --------------------------
    //
    // `Добавить`/`Удалить`/`Очистить` в реальной 1С означают разное в
    // зависимости от типа получателя (элемент массива, строка таблицы,
    // колонка, ...) — то же имя метода, разное поведение и разная арность.
    // Резолвинг имени в `bsl-sema` не может знать заранее, каким объектом
    // оказится `obj` в рантайме (BSL — динамически типизированный), поэтому
    // диспетчеризация и проверка арности — здесь, в рантайме, а не на этапе
    // компиляции.

    /// `Массив.Добавить(значение)`.
    pub fn push_element(&self, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().push(val);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `ТаблицаЗначений.Добавить()` -> новая строка.
    pub fn table_add_row(&self) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    let row_id = data.borrow_mut().add_row();
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))))
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Таблица.Колонки.Добавить(имя)`.
    pub fn table_add_column(&self, name: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableColumns(data) => {
                    let name = name.as_str("Колонки.Добавить")?.to_string();
                    data.borrow_mut().add_column(&name);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Удалить(индекс)` / `ТаблицаЗначений.Удалить(индекс)`.
    pub fn delete_element(&self, idx: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    if i >= len {
                        return Err(RtError::IndexOutOfBounds { index: i as i64, len });
                    }
                    v.remove(i);
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    let mut d = data.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = d.row_count();
                    d.delete_row_at(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len })
                }
                // `Соответствие.Удалить(Ключ)` — по значению ключа, не по
                // позиции (в отличие от Array/ValueTable выше): в этом
                // случае `idx` в имени параметра функции вводит в
                // заблуждение, но сигнатура (`&BslValue`) уже общая для
                // всех получателей, менять её ради одного случая не стоит.
                BslObject::Map(data) => {
                    data.borrow_mut().remove(idx);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Удалить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Удалить",
                receiver: self.type_name(),
            }),
        }
    }

    // --- ТаблицаЗначений, волна 2 ----------------------------------------

    /// Общий доступ к данным таблицы для методов волны 2 — все они
    /// применимы только к самой `ТаблицаЗначений`, не к строке и не к
    /// коллекции колонок.
    fn as_table(&self, method: &'static str) -> RtResult<&Rc<std::cell::RefCell<ValueTableData>>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => Ok(data),
                _ => Err(RtError::MethodNotApplicable {
                    method,
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: self.type_name(),
            }),
        }
    }

    /// Разбор списка колонок `"Кол1, Кол2"` в индексы. Пустая строка (или
    /// отсутствующий аргумент) — пустой список, что для `Найти` значит
    /// «искать во всех колонках».
    fn column_indices(data: &ValueTableData, spec: &str) -> RtResult<Vec<usize>> {
        spec.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|name| {
                data.column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))
            })
            .collect()
    }

    /// `Найти(Значение[, Колонки])` -> `СтрокаТаблицыЗначений` либо
    /// `Неопределено`, если ничего не нашлось (не ошибка — это штатный
    /// способ проверить наличие).
    pub fn table_find(&self, value: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Найти")?;
        let cols = match columns {
            BslValue::Undefined => Vec::new(),
            other => {
                let spec = other.as_str("Найти")?.to_string();
                Self::column_indices(&data.borrow(), &spec)?
            }
        };
        let found = data.borrow().find(value, &cols);
        Ok(match found {
            Some(row_id) => BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))),
            None => BslValue::Undefined,
        })
    }

    /// `НайтиСтроки(СтруктураПоиска)` -> `Массив` строк таблицы.
    ///
    /// Имена полей структуры — это имена колонок, поэтому нужен интернер:
    /// поля хранятся `NameId`, а колонки — строками (они заведены в
    /// рантайме через `.Колонки.Добавить`, см. `get_field_by_name`).
    pub fn table_find_rows(&self, criteria: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        let data = self.as_table("НайтиСтроки")?;
        let BslValue::Object(o) = criteria else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };
        let BslObject::Structure(s) = &**o else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };

        let pairs = {
            let s = s.borrow();
            let d = data.borrow();
            let mut pairs = Vec::with_capacity(s.len());
            for i in 0..s.len() {
                let (field, want) = s.entry_at(i).ok_or(RtError::NotAnObject)?;
                let name = names.name(field).ok_or(RtError::UnknownField(field))?;
                let col = d
                    .column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                pairs.push((col, want));
            }
            pairs
        };

        let ids = data.borrow().find_rows(&pairs);
        Ok(BslValue::new_array(
            ids.into_iter()
                .map(|id| BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), id))))
                .collect(),
        ))
    }

    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`. Живые объекты
    /// `СтрокаТаблицыЗначений` переживают сортировку — см.
    /// `ValueTableData::sort`.
    pub fn table_sort(&self, spec: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сортировать")?;
        let spec = spec.as_str("Сортировать")?.to_string();
        let keys = {
            let d = data.borrow();
            table::parse_sort_spec(&spec, |name| d.column_index(name))
                .map_err(RtError::UnknownColumn)?
        };
        data.borrow_mut().sort(&keys);
        Ok(())
    }

    /// `Итог("Колонка")` -> `Число`. Про нечисловые значения см.
    /// `ValueTableData::total` (НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC)).
    pub fn table_total(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Итог")?;
        let name = column.as_str("Итог")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::Number(d.total(col)?))
    }

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    /// Строка ЭТОЙ таблицы: разворачивает объект `СтрокаТаблицыЗначений` в
    /// текущую позицию. Строка чужой таблицы — ошибка метода, а не «не
    /// найдено»: спутать таблицы легко, и молчаливый `-1` в ответ прятал бы
    /// эту ошибку до самого конца.
    fn row_position(
        data: &Rc<std::cell::RefCell<ValueTableData>>,
        row: &BslValue,
        method: &'static str,
    ) -> RtResult<usize> {
        let BslValue::Object(o) = row else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        let BslObject::TableRow(owner, row_id) = &**o else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        if !Rc::ptr_eq(owner, data) {
            return Err(RtError::MethodNotApplicable {
                method,
                receiver: "СтрокаТаблицыЗначений другой таблицы",
            });
        }
        data.borrow().pos_of(*row_id).ok_or(RtError::RowInvalidated)
    }

    /// Список колонок `"Кол1, Кол2"` -> индексы; `Неопределено` или пустая
    /// строка -> ВСЕ колонки в их порядке. Разворачивать «все» здесь, а не
    /// в `ValueTableData`, нарочно: слой данных не должен знать, что пустой
    /// список для `Скопировать` значит «все», а для `Найти` — «любая».
    fn columns_or_all(
        data: &ValueTableData,
        spec: &BslValue,
        method: &'static str,
    ) -> RtResult<Vec<usize>> {
        let all = || (0..data.column_names.len()).collect::<Vec<usize>>();
        match spec {
            BslValue::Undefined => Ok(all()),
            other => {
                let spec = other.as_str(method)?.to_string();
                if spec.trim().is_empty() {
                    return Ok(all());
                }
                Self::column_indices(data, &spec)
            }
        }
    }

    /// `Скопировать([Строки], [Колонки])` -> новая `ТаблицаЗначений`.
    ///
    /// `Строки` — `Массив` строк ЭТОЙ таблицы (порядок массива и есть
    /// порядок строк копии) либо `Неопределено` — тогда все строки в
    /// текущем порядке.
    pub fn table_copy(&self, rows: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Скопировать")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "Скопировать")?;
        let positions: Vec<usize> = match rows {
            BslValue::Undefined => (0..data.borrow().row_count()).collect(),
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let items = items.borrow();
                    let mut out = Vec::with_capacity(items.len());
                    for row in items.iter() {
                        out.push(Self::row_position(data, row, "Скопировать")?);
                    }
                    out
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: "Скопировать",
                    })
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: "Скопировать",
                })
            }
        };
        let copy = data.borrow().copy_of(&positions, &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `СкопироватьКолонки([Колонки])` -> пустая таблица той же структуры.
    /// Это `Скопировать` без единой строки, а не отдельный алгоритм.
    pub fn table_copy_columns(&self, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("СкопироватьКолонки")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "СкопироватьКолонки")?;
        let copy = data.borrow().copy_of(&[], &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `ВыгрузитьКолонку(Колонка)` -> `Массив` значений в текущем порядке
    /// строк.
    pub fn table_unload_column(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("ВыгрузитьКолонку")?;
        let name = column.as_str("ВыгрузитьКолонку")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::new_array(d.unload_column(col)))
    }

    /// `ЗагрузитьКолонку(Массив, Колонка)`. Про несовпадение длин — см.
    /// `ValueTableData::load_column`
    /// (НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH)).
    pub fn table_load_column(&self, values: &BslValue, column: &BslValue) -> RtResult<()> {
        let data = self.as_table("ЗагрузитьКолонку")?;
        let name = column.as_str("ЗагрузитьКолонку")?.to_string();
        let BslValue::Object(o) = values else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let BslObject::Array(items) = &**o else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let col = data
            .borrow()
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        let values = items.borrow().clone();
        data.borrow_mut().load_column(col, &values);
        Ok(())
    }

    /// `Сдвинуть(Строка, Смещение)` — `Строка` это либо объект строки, либо
    /// её индекс. Целевая позиция вне таблицы — `IndexOutOfBounds`, а не
    /// зажатие в границы.
    ///
    /// НЕ ИЗМЕРЕНО(TABLE.MOVE.OUT_OF_RANGE): падает ли платформа или молча
    /// зажимает. Взята ошибка: `Сдвинуть(ПерваяСтрока, -1)`, тихо ничего не
    /// сделавший, — та же категория беды, что и `Сортировать("Опечатка")`,
    /// молча ничего не отсортировавшая.
    pub fn table_move(&self, row: &BslValue, offset: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сдвинуть")?;
        let from = match row {
            BslValue::Number(_) => {
                let i = Self::index_as_usize(row)?;
                let len = data.borrow().row_count();
                if i >= len {
                    return Err(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    });
                }
                i
            }
            other => Self::row_position(data, other, "Сдвинуть")?,
        };
        let BslValue::Number(n) = offset else {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Сдвинуть",
            });
        };
        let offset = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        let len = data.borrow().row_count();
        data.borrow_mut()
            .move_row(from, offset)
            .map(|_| ())
            .ok_or(RtError::IndexOutOfBounds {
                index: from as i64 + offset,
                len,
            })
    }

    /// `Индекс(Строка)` -> `Число`, позиция строки (с нуля, как у
    /// `Получить`/`Удалить`).
    pub fn table_index_of(&self, row: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Индекс")?;
        let pos = Self::row_position(data, row, "Индекс")?;
        Ok(BslValue::Number(BslNumber::from_i64(pos as i64)))
    }

    /// `Свернуть(КолонкиГруппировки[, КолонкиСуммирования])` — группировка
    /// на месте. Три неизмеренных решения (судьба прочих колонок, порядок
    /// строк, нечисловые значения) описаны у `ValueTableData::collapse`.
    pub fn table_collapse(&self, group: &BslValue, sum: &BslValue) -> RtResult<()> {
        let data = self.as_table("Свернуть")?;
        let (group_cols, sum_cols) = {
            let d = data.borrow();
            let group_cols = Self::column_indices(&d, &group.as_str("Свернуть")?.to_string())?;
            let sum_cols = match sum {
                BslValue::Undefined => Vec::new(),
                other => Self::column_indices(&d, &other.as_str("Свернуть")?.to_string())?,
            };
            (group_cols, sum_cols)
        };
        data.borrow_mut().collapse(&group_cols, &sum_cols)?;
        Ok(())
    }

    /// `Массив.Очистить()` / `ТаблицаЗначений.Очистить()` /
    /// `Соответствие.Очистить()`.
    pub fn clear_collection(&self) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().clear();
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Очистить",
                receiver: self.type_name(),
            }),
        }
    }
}

/// Ручная реализация вместо `derive`: `Массив`/`Структура` — ссылочные
/// типы, `=` для них — тождество объекта (`Rc::ptr_eq`), а не структурное
/// сравнение содержимого (в отличие от `Число`/`Строка`/`Булево`).
impl PartialEq for BslValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Undefined, BslValue::Undefined) => true,
            (BslValue::Null, BslValue::Null) => true,
            (BslValue::Boolean(a), BslValue::Boolean(b)) => a == b,
            (BslValue::Number(a), BslValue::Number(b)) => a == b,
            (BslValue::Str(a), BslValue::Str(b)) => a == b,
            // Дата — тип ЗНАЧЕНИЯ, как число и строка: равенство по
            // моменту времени, а не по тождеству объекта.
            (BslValue::Date(a), BslValue::Date(b)) => a == b,
            // Тип — значение, а не объект: два `ТипЗнч(...)` от разных
            // значений одного типа равны (`ТипЗнч(1) = ТипЗнч(2)`), иначе
            // проверка типа была бы бесполезна.
            (BslValue::Type(a), BslValue::Type(b)) => a == b,
            (BslValue::Object(a), BslValue::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// `Eq` — все варианты `PartialEq::eq` выше рефлексивны (десятичные числа
/// без NaN-подобных значений, указатели сравниваются с самими собой),
/// значит `Eq` держится честно, не только формально для `HashMap`.
impl Eq for BslValue {}

/// Нужен ключу `Соответствие` (`HashMap<BslValue, BslValue>` в `map.rs`).
/// Согласован с `PartialEq` выше пункт-в-пункт: `Число` хэширует через
/// `BslNumber` (см. его `impl Hash` — нормализация уже сделала хэш
/// независимым от масштаба представления, `1.0` и `1.00` дают одно и то же
/// (`m`, `scale`)), `Строка` — по содержимому (`BslString` производит
/// `Hash` от `Rc<[u16]>`, тоже по значению, не по адресу), `Массив`/
/// `Структура`/... — по адресу `Rc`, ровно как их `==` через `Rc::ptr_eq`.
impl Hash for BslValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BslValue::Undefined | BslValue::Null => {}
            BslValue::Boolean(b) => b.hash(state),
            BslValue::Number(n) => n.hash(state),
            BslValue::Str(s) => s.hash(state),
            BslValue::Date(d) => d.hash(state),
            BslValue::Type(t) => t.hash(state),
            BslValue::Object(o) => Rc::as_ptr(o).hash(state),
            BslValue::Skipped => {}
        }
    }
}

impl fmt::Display for BslValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BslValue::Undefined => write!(f, ""),
            BslValue::Null => write!(f, "Null"),
            BslValue::Boolean(true) => write!(f, "Да"),
            BslValue::Boolean(false) => write!(f, "Нет"),
            BslValue::Number(n) => write!(f, "{n}"),
            BslValue::Str(s) => write!(f, "{s}"),
            // Формат по умолчанию (`date::DEFAULT_PATTERN`) — НЕ ИЗМЕРЕН,
            // см. там же.
            BslValue::Date(d) => write!(f, "{d}"),
            // Локализованное имя типа: `Строка(ТипЗнч(Новый Массив))` даёт
            // то же `Массив`, что и `Строка(Новый Массив)`.
            BslValue::Type(t) => write!(f, "{t}"),
            BslValue::Object(o) => match &**o {
                BslObject::Array(_) => write!(f, "Массив"),
                BslObject::Structure(_) => write!(f, "Структура"),
                BslObject::ValueTable(_) => write!(f, "ТаблицаЗначений"),
                BslObject::TableColumns(_) => write!(f, "КоллекцияКолонокТаблицыЗначений"),
                BslObject::TableRow(_, _) => write!(f, "СтрокаТаблицыЗначений"),
                BslObject::Map(_) => write!(f, "Соответствие"),
                BslObject::KeyValuePair(_, _) => write!(f, "КлючИЗначение"),
                BslObject::TextWriter(_) => write!(f, "ЗаписьТекста"),
            },
            // Никогда не должно реально дойти до печати (см. doc comment
            // на варианте) — но `Display` обязан быть тотальным.
            BslValue::Skipped => write!(f, "<Skipped>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    #[test]
    fn strict_boolean_condition_rejects_numbers() {
        // Если 1 Тогда — ошибка, не приведение.
        let err = num("1").as_condition().unwrap_err();
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));
    }

    #[test]
    fn equality_by_value_across_representations() {
        assert!(num("1.0").eq_value(&num("1.00")));
    }

    #[test]
    fn display_matches_measured_platform_strings() {
        assert_eq!(BslValue::Boolean(true).to_string(), "Да");
        assert_eq!(BslValue::Boolean(false).to_string(), "Нет");
        assert_eq!(BslValue::Undefined.to_string(), "");
    }

    #[test]
    fn array_index_get_set_roundtrip() {
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        assert_eq!(arr.get_index(&num("1"), &NameInterner::new()).unwrap(), num("2"));
        arr.set_index(&num("1"), num("99")).unwrap();
        assert_eq!(arr.get_index(&num("1"), &NameInterner::new()).unwrap(), num("99"));
        assert_eq!(arr.collection_len().unwrap(), 3);
    }

    #[test]
    fn array_out_of_bounds_is_an_error() {
        let arr = BslValue::new_array(vec![num("1")]);
        assert!(matches!(
            arr.get_index(&num("5"), &NameInterner::new()).unwrap_err(),
            RtError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn arrays_and_structures_are_reference_types() {
        // b = a делает b тем же объектом, что и a: мутация через одну
        // переменную видна через другую (Rc, не глубокое копирование).
        let a = BslValue::new_array(vec![num("1")]);
        let b = a.clone();
        b.set_index(&num("0"), num("42")).unwrap();
        assert_eq!(a.get_index(&num("0"), &NameInterner::new()).unwrap(), num("42"));
        assert!(a.eq_value(&b));

        let c = BslValue::new_array(vec![num("42")]);
        assert!(!a.eq_value(&c), "структурно равные, но разные объекты — не равны");
    }

    #[test]
    fn structure_field_get_set_by_interned_name() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let y = names.intern("y");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x, y]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1"), num("2")]);
        assert_eq!(s.get_field(x).unwrap(), num("1"));
        s.set_field(y, num("99")).unwrap();
        assert_eq!(s.get_field(y).unwrap(), num("99"));
    }

    #[test]
    fn unknown_field_is_an_error() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let z = names.intern("z");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1")]);
        assert!(matches!(s.get_field(z).unwrap_err(), RtError::UnknownField(_)));
    }

    // --- Словарный режим структуры ------------------------------------
    //
    // Порог `MAX_SHAPE_TRANSITIONS` — единственная защита от того, что
    // `Вставить` с динамическим именем в цикле навсегда интернирует форму
    // на каждой итерации (см. doc comment на константе). Тесты ниже
    // фиксируют и сам переход, и то, ради чего он затеян: таблица форм
    // после него перестаёт расти.

    /// Пустая структура плюс рантайм-контекст форм — общая затравка для
    /// тестов деградации.
    fn fresh_structure() -> (BslValue, RuntimeShapes) {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let empty = rt.shapes.empty();
        (BslValue::new_structure(empty, Vec::new()), rt)
    }

    fn is_dictionary(v: &BslValue) -> bool {
        match v {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    matches!(&*s.borrow(), StructureStorage::Dictionary { .. })
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// `Вставить("Поле<i>", i)` — ровно тот путь, на котором форма уходит
    /// вглубь: каждый ключ новый, так что каждый переход заводил бы форму.
    fn insert_generated_fields(s: &BslValue, rt: &mut RuntimeShapes, count: u32) {
        for i in 0..count {
            let f = rt.names.intern(&format!("Поле{i}"));
            s.structure_insert(f, num(&i.to_string()), &mut rt.shapes).unwrap();
        }
    }

    #[test]
    fn structure_degrades_to_dictionary_after_threshold_inserts() {
        let (s, mut rt) = fresh_structure();

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS);
        assert!(
            !is_dictionary(&s),
            "ровно {MAX_SHAPE_TRANSITIONS} переходов должны укладываться в порог"
        );

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 1);
        assert!(is_dictionary(&s), "переход за порог обязан деградировать объект");
    }

    #[test]
    fn dictionary_structure_field_get_set_roundtrip() {
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 5);
        assert!(is_dictionary(&s));

        // Поля, заведённые ДО деградации (перенесённые из слотов), и после
        // неё — читаются и пишутся одинаково.
        let first = rt.names.intern("Поле0");
        let last = rt.names.intern(&format!("Поле{}", MAX_SHAPE_TRANSITIONS + 4));
        assert_eq!(s.get_field(first).unwrap(), num("0"));
        assert_eq!(
            s.get_field(last).unwrap(),
            num(&(MAX_SHAPE_TRANSITIONS + 4).to_string())
        );

        s.set_field(first, num("777")).unwrap();
        s.set_field(last, num("888")).unwrap();
        assert_eq!(s.get_field(first).unwrap(), num("777"));
        assert_eq!(s.get_field(last).unwrap(), num("888"));

        let missing = rt.names.intern("НетТакогоПоля");
        assert!(matches!(s.get_field(missing).unwrap_err(), RtError::UnknownField(_)));
        assert!(matches!(
            s.set_field(missing, num("1")).unwrap_err(),
            RtError::UnknownField(_)
        ));
    }

    #[test]
    fn dictionary_structure_delete_keeps_order_of_the_rest() {
        let total = MAX_SHAPE_TRANSITIONS + 3;
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, total);
        assert!(is_dictionary(&s));

        let victim = rt.names.intern("Поле1");
        s.structure_delete(victim, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);

        // Ожидаемый порядок — исходный без удалённого: `order` теряет ровно
        // один элемент, остальные не переставляются.
        let expected: Vec<String> = (0..total)
            .filter(|i| *i != 1)
            .map(|i| format!("Поле{i}"))
            .collect();
        let actual: Vec<String> = (0..expected.len())
            .map(|i| match s.get_index(&num(&i.to_string()), &rt.names).unwrap() {
                BslValue::Object(o) => match &*o {
                    BslObject::KeyValuePair(k, _) => k.to_string(),
                    other => panic!("ожидался КлючИЗначение, получено {other:?}"),
                },
                other => panic!("ожидался объект, получено {other:?}"),
            })
            .collect();
        assert_eq!(actual, expected);

        // Удаление отсутствующего — no-op и в словарном режиме тоже.
        let missing = rt.names.intern("НетТакогоПоля");
        s.structure_delete(missing, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);
    }

    #[test]
    fn shape_table_stops_growing_after_degradation() {
        // Суть всей задачи: без порога здесь было бы 10_000 бессмертных
        // форм со списками имён нарастающей длины — квадратичная память.
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, 10_000);

        assert!(is_dictionary(&s));
        assert_eq!(s.collection_len().unwrap(), 10_000);
        // Пустая форма + по одной на каждый разрешённый переход, и ни одной
        // сверх того.
        assert_eq!(rt.shapes.len(), MAX_SHAPE_TRANSITIONS as usize + 1);
    }

    #[test]
    fn dictionary_structure_does_not_poison_inline_cache_for_shaped_objects() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        // `Поле0` — первое имя и у `shaped`, и у сгенерированной серии, так
        // что оба объекта проходят через одну и ту же форму `[Поле0]`.
        let x = rt.names.intern("Поле0");

        let shaped = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        shaped.structure_insert(x, num("1"), &mut rt.shapes).unwrap();

        let dict = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        insert_generated_fields(&dict, &mut rt, MAX_SHAPE_TRANSITIONS + 2);
        assert!(is_dictionary(&dict));
        assert!(!is_dictionary(&shaped));
        assert_eq!(dict.get_field(x).unwrap(), num("0"));

        // Один и тот же сайт вызова (одна ячейка кэша) видит оба объекта.
        let cache: std::cell::RefCell<Option<(Rc<Shape>, u32)>> = std::cell::RefCell::new(None);

        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("1"));
        let filled = cache.borrow().clone();
        assert!(filled.is_some(), "шейповый объект обязан заполнить кэш");

        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("0"));
        let after_dict = cache.borrow().clone();
        match (&filled, &after_dict) {
            (Some((a, ai)), Some((b, bi))) => {
                assert!(Rc::ptr_eq(a, b), "словарный объект затёр форму в кэше");
                assert_eq!(ai, bi, "словарный объект затёр слот в кэше");
            }
            _ => panic!("словарный объект обнулил ячейку кэша"),
        }

        // И быстрый путь для шейпового объекта по-прежнему работает.
        shaped.set_field_cached(x, num("42"), &cache).unwrap();
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
        // Запись через тот же сайт в словарный объект тоже не портит кэш.
        dict.set_field_cached(x, num("43"), &cache).unwrap();
        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("43"));
        let after_dict_set = cache.borrow().clone();
        match (&filled, &after_dict_set) {
            (Some((a, _)), Some((b, _))) => assert!(Rc::ptr_eq(a, b)),
            _ => panic!("словарный объект обнулил ячейку кэша при записи"),
        }
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
    }

    #[test]
    fn display_matches_measured_platform_strings_for_collections() {
        // Строка(Новый Массив) -> "Массив" (измерено на платформе).
        assert_eq!(BslValue::new_array(vec![]).to_string(), "Массив");
    }

    #[test]
    fn builtin_math_functions_lookup_and_call() {
        assert_eq!(BuiltinFn::lookup("sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("Sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("СООБЩИТЬ"), Some(BuiltinFn::Message));
        assert_eq!(
            BuiltinFn::lookup("ТекущаяУниверсальнаяДатаВМиллисекундах"),
            Some(BuiltinFn::CurrentUniversalDateInMilliseconds)
        );
        assert_eq!(
            BuiltinFn::CurrentUniversalDateInMilliseconds.arity_range(),
            (0, 0)
        );
        assert_eq!(BuiltinFn::lookup("НетТакойФункции"), None);
        assert_eq!(BuiltinFn::Pow.arity_range(), (2, 2));
        assert_eq!(BuiltinFn::Sqrt.arity_range(), (1, 1));
        // Необязательный аргумент — диапазон, а не одно число.
        assert_eq!(BuiltinFn::Mid.arity_range(), (2, 3));
        assert_eq!(BuiltinFn::StrTemplate.arity_range(), (1, 11));

        let v = call_builtin_fn(BuiltinFn::Sqrt, &[num("2")]).unwrap();
        assert_eq!(v, num("1.4142135623731"));
    }

    #[test]
    fn builtin_method_count_on_array() {
        assert_eq!(BuiltinMethod::lookup("count"), Some(BuiltinMethod::Count));
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        let v = call_builtin_method(BuiltinMethod::Count, &arr, &[]).unwrap();
        assert_eq!(v, num("3"));
    }
}
