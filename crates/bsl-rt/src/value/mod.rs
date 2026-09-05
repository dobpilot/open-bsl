//! Представление значения BSL и его операции.

mod collections;
mod dates;
mod strings;
mod tables;

use crate::RandomHandle;
use crate::{
    BslDate, BslNumber, BslObject, BslString, ByteStreamProtocol, EnumKind, EnumValue,
    ExecutionToken, ObjectProtocol, ObjectRef, PromiseId, PromiseValue, RtError, RtResult,
    RuntimeShapes, TypeId, TypeRef, bindata, uuid,
};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::rc::Rc;

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
    /// `TypeRef` — `Copy` в размер указателя, поэтому вариант ничего не
    /// добавляет к размеру `BslValue`.
    Type(TypeRef),
    /// Член платформенного перечисления (`ТипЗначенияJSON.Строка`). Как и
    /// `Type`, это ЗНАЧЕНИЕ, а не тег: его можно сравнить, положить в
    /// переменную и напечатать. `Copy` в один байт — размер `BslValue` не
    /// растёт (см. модуль `enums`).
    Enum(EnumValue),
    /// Голое имя системного перечисления как ВЫРАЖЕНИЕ (`Вычислить("ВариантЗаписиДатыJSON")`,
    /// без `.Член`) — ИЗМЕРЕНО (`JSON.DATE_VARIANT_EN_NAMES`, «Т+»): платформа
    /// принимает такое выражение, а не отвергает его как обращение к
    /// неопределённой переменной. Что именно возвращает `Строка()`/`ТипЗнч()`
    /// от этого значения — `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`; здесь это
    /// самостоятельный вариант (не `Type`/`Enum` — семантически ни то, ни
    /// другое: не тип и не конкретный член), `Copy` в один байт, как и они.
    EnumType(EnumKind),
    Object(Rc<BslObject>),
}

/// `TypeId` перечисления, к которому принадлежит член, — используется и
/// для `ТипЗнч()` конкретного члена (`BslValue::Enum`), и для голого имени
/// перечисления как выражения (`BslValue::EnumType`, см. doc comment на
/// самом варианте).
fn enum_kind_type_id(kind: EnumKind) -> TypeId {
    match kind {
        EnumKind::JsonValueType => TypeId::JsonValueType,
        EnumKind::JsonLineBreak => TypeId::JsonLineBreak,
        EnumKind::JsonEscapeCharacters => TypeId::JsonEscapeCharacters,
        EnumKind::JsonDateFormat => TypeId::JsonDateFormat,
        EnumKind::JsonDateWritingVariant => TypeId::JsonDateWritingVariant,
        EnumKind::XmlNodeType => TypeId::XmlNodeType,
        EnumKind::DomNodeType => TypeId::DomNodeType,
        EnumKind::SpreadFileType => TypeId::SpreadFileType,
        EnumKind::DrawingKind => TypeId::DrawingKind,
        EnumKind::PageOrientation => TypeId::PageOrientation,
        EnumKind::TextEncoding => TypeId::TextEncoding,
        EnumKind::StringEncodingMethod => TypeId::StringEncodingMethod,
        EnumKind::SortDirection => TypeId::SortDirection,
        EnumKind::BackgroundJobState => TypeId::BackgroundJobState,
        EnumKind::MessageStatus => TypeId::MessageStatus,
        EnumKind::ErrorCategory => TypeId::ErrorCategory,
        EnumKind::DateFractions => TypeId::DateFractions,
        EnumKind::HashFunction => TypeId::HashFunction,
        EnumKind::ByteOrder => TypeId::ByteOrder,
        EnumKind::FileOpenMode => TypeId::FileOpenMode,
        EnumKind::FileAccess => TypeId::FileAccess,
        EnumKind::StreamPosition => TypeId::StreamPosition,
        EnumKind::XsComponentType => TypeId::XsComponentType,
        EnumKind::XsForm => TypeId::XsForm,
        EnumKind::XsSimpleTypeVariety => TypeId::XsSimpleTypeVariety,
        EnumKind::XsModelGroupKind => TypeId::XsModelGroupKind,
        EnumKind::XsDerivationMethod => TypeId::XsDerivationMethod,
        EnumKind::XsValueConstraint => TypeId::XsValueConstraint,
        EnumKind::XsWhitespaceHandling => TypeId::XsWhitespaceHandling,
        EnumKind::XmlForm => TypeId::XmlForm,
        EnumKind::XdtoFacetKind => TypeId::XdtoFacetKind,
        EnumKind::DomXPathResultType => TypeId::DomXPathResultType,
        EnumKind::SearchDirection => TypeId::SearchDirection,
        EnumKind::ZipRestorePathsMode => TypeId::ZipRestorePathsMode,
        EnumKind::PdfAttachmentRelation => TypeId::PdfAttachmentRelation,
        EnumKind::ArchiveFileType => TypeId::ArchiveFileType,
        EnumKind::ZipCompressionMethod => TypeId::ZipCompressionMethod,
        EnumKind::ZipCompressionLevel => TypeId::ZipCompressionLevel,
        EnumKind::ZipStorePathMode => TypeId::ZipStorePathMode,
        EnumKind::ZipSubDirProcessingMode => TypeId::ZipSubDirProcessingMode,
        EnumKind::ZipEncryptionMethod => TypeId::ZipEncryptionMethod,
        EnumKind::ZipFileNamesEncoding => TypeId::ZipFileNamesEncoding,
        EnumKind::ByteOrderMarkUse => TypeId::ByteOrderMarkUse,
    }
}

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
            BslValue::Enum(e) => e.enum_name(),
            // ИЗМЕРЕНО (проба `JSON.ENUM.BARE_NAME`): голое имя
            // перечисления печатается МЕТАТИПОМ — `Перечисление` плюс
            // русское написание, слитно.
            BslValue::EnumType(k) => k.meta_ru_name(),
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.type_descriptor().name,
                BslObject::Array(_) => "Массив",
                BslObject::Structure(_) => "Структура",
                // Служебное имя ЭТОЙ реализации: в 1С такое значение всегда
                // имеет настоящий тип (ссылка, список значений, ...), но
                // без базы и без реверса разметки материализовать его нечем.
                BslObject::VstrOpaque(_) => "НепрозрачноеЗначение",
                BslObject::ValueTable(_) => "ТаблицаЗначений",
                BslObject::TableColumns(_) => "КоллекцияКолонокТаблицыЗначений",
                BslObject::TableColumn(..) => "КолонкаТаблицыЗначений",
                BslObject::TableRow(_, _) => "СтрокаТаблицыЗначений",
                BslObject::TypeDescription(_) => "ОписаниеТипов",
                BslObject::ValueComparison => "СравнениеЗначений",
                BslObject::Map(_) => "Соответствие",
                BslObject::KeyValuePair(_, _) => "КлючИЗначение",
                BslObject::TextWriter(_) => "ЗаписьТекста",
                // У двоичных данных имя ЗНАЧЕНИЯ платформой не наблюдаемо:
                // `Строка(ДД)` отдаёт дамп байтов, а не имя (измерено,
                // проба `BIN.STR`). Эта строка живёт только в
                // диагностике самой реализации — в тексте `RtError`, — и
                // написана слитно по образцу соседей.
                BslObject::BinaryData(_) => "ДвоичныеДанные",
                // А вот у БУФЕРА имя значения наблюдаемо и измерено:
                // `Строка(Буфер)` печатает именно «БуферДвоичныхДанных»
                // (слитно), в отличие от имени типа «Буфер двоичных
                // данных» в `types.rs` и в отличие от соседа сверху,
                // который печатается дампом байтов.
                BslObject::BinaryBuffer(_) => "БуферДвоичныхДанных",
                // Имя ЗНАЧЕНИЯ здесь не наблюдаемо: `Строка(УИД)` печатает
                // саму каноническую форму, а не имя (фикстура `uuid`).
                // Строка ниже живёт в диагностике `RtError`.
                BslObject::Uuid(_) => "УникальныйИдентификатор",
            },
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

    /// `%` — остаток от деления. Дат это не касается: `Дата % Число`
    /// платформе неизвестно так же, как и нам, и общая числовая ветка ниже
    /// отвергнет её сама.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если операнд не число, либо
    /// [`RtError::Num`] при делении на ноль.
    pub fn rem(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("%")?.rem(other.as_number("%")?)?,
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

    /// `Не` приводит операнд по тем же правилам, что и любое другое
    /// условие: `Не 1` даёт «Нет», а `Не "Ложь"` — «Да» (измерено,
    /// `COND.NOT_NUMBER` и `COND.NOT_STRING`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если операнд к условию не приводится.
    pub fn not(&self) -> RtResult<Self> {
        Ok(BslValue::Boolean(!self.as_condition()?))
    }

    // --- Трансцендентные функции (через f64 в bsl-number) ------------------

    pub fn sqrt(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sqrt")?.sqrt()?))
    }

    pub fn pow(&self, exp: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("Pow")?.pow(exp.as_number("Pow")?)?,
        ))
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
    /// Условие приводится к булеву, и это ИЗМЕРЕНО, а не выведено: до
    /// замеров здесь стояла строгая булевость с комментарием «никакой
    /// truthiness», и она оказалась неверной. Платформа принимает
    ///
    /// * булево — как есть;
    /// * ЧИСЛО: ноль — ложь, любое другое (в том числе отрицательное и
    ///   дробное) — истина (замеры `TERNARY.CONDITION_ZERO`,
    ///   `TERNARY.CONDITION_NEGATIVE`, `COND.IF_NUMBER_ONE`);
    /// * СТРОКУ — но только словами, и это не «непустая строка истинна»:
    ///   «абв», «0» и «1» отвергаются наравне с пустой (замеры
    ///   `TERNARY.CONDITION_WORD_OTHER`, `..._STRING_ZERO`, `..._STRING_ONE`).
    ///
    /// Всё остальное — `Неопределено`, `Null`, дата, коллекция — ошибка.
    /// Правило одно на все условия языка: `Если`, `Пока`, `И`, `ИЛИ`, `Не`
    /// и `?()` идут сюда же, и на платформе они тоже ведут себя одинаково
    /// (замеры `COND.*`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если значение к условию не приводится.
    pub fn as_condition(&self) -> RtResult<bool> {
        match self {
            BslValue::Boolean(b) => Ok(*b),
            BslValue::Number(n) => Ok(!n.is_zero()),
            BslValue::Str(s) => condition_word(&s.to_string()).ok_or(RtError::TypeError {
                expected: "Булево",
                op: "Условие",
            }),
            _ => Err(RtError::TypeError {
                expected: "Булево",
                op: "Условие",
            }),
        }
    }

    /// Сравнение по значению для `=`/`<>`. Разнотипные значения просто не
    /// равны — это не ошибка (в отличие от `<`/`>`/... между разными
    /// типами). `Массив`/`Структура` — ссылочные типы: равенство — это
    /// тождество объекта (`Rc::ptr_eq`), а не структурное сравнение
    /// содержимого.
    ///
    /// ОДНО исключение, и оно измерено: булево сравнивается с числом
    /// ЧИСЛЕННО, `Истина` как единица, `Ложь` как ноль. Именно численно, а
    /// не «по истинности»: `Истина = 2` — ЛОЖЬ (замер `EQ.BOOL.TRUE_TWO`).
    /// Строка при этом не приводится никак: `"5" = 5` ложно.
    ///
    /// Поэтому же эта функция БОЛЬШЕ НЕ совпадает с [`PartialEq`]: там
    /// отношение строже, и так и должно быть. Платформа различает эти два
    /// отношения — измерено на самом наблюдаемом их следствии: `Истина` и
    /// `1` остаются РАЗНЫМИ ключами `Соответствие` (замеры
    /// `EQ.MAP.BOOL_KEY_BY_NUMBER` и `EQ.MAP.BOTH_KEYS`), и `Найти` булево
    /// по единице тоже не находит. Ослабить заодно `PartialEq` было бы
    /// нельзя ещё и технически: с ним согласован `Hash`, и соответствие
    /// сломалось бы молча.
    pub fn eq_value(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Boolean(b), BslValue::Number(n))
            | (BslValue::Number(n), BslValue::Boolean(b)) => {
                *n == BslNumber::from_i64(i64::from(*b))
            }
            _ => self == other,
        }
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

    pub(super) fn as_str(&self, op: &'static str) -> RtResult<&BslString> {
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

    // --- Проверки значения и типа ----------------------------------------

    /// `ЗначениеЗаполнено(Значение)`.
    ///
    /// ИЗМЕРЕНО (из брифа): `Ложь` для `Неопределено`, `Null`, пустой
    /// строки, нуля и пустой даты.
    ///
    /// Всё остальное — три открытых вопроса,
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BOOLEAN)`,
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BLANK_STRING)` и
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.EMPTY_COLLECTION)`, каждый со своей веткой
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
            // Член перечисления — тоже всегда значение.
            BslValue::Enum(_) => true,
            // Голое имя перечисления — тем же рассуждением: значение есть,
            // «пустого» варианта нет (не измерено отдельно).
            BslValue::EnumType(_) => true,
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.is_filled()?,
                // Непрозрачное значение — всегда «что-то»: судить о его
                // заполненности, не материализуя вид, нечем.
                BslObject::VstrOpaque(_) => true,
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
                // ИЗМЕРЕНО (пробы `BIN.IS_FILLED`/`BIN.EMPTY`): двоичные
                // данные считаются заполненными по ДЛИНЕ — 13 байт «Да»,
                // ноль байт «Нет». Это тот же критерий, что у коллекций,
                // а не «объект есть — значит заполнен».
                BslObject::BinaryData(bytes) => !bytes.is_empty(),
                // У буфера тот же критерий — ПО РАЗМЕРУ, не по содержимому:
                // измерено, что четырёхбайтовый НУЛЕВОЙ буфер считается
                // заполненным («Да»), а буфер нулевого размера — нет.
                BslObject::BinaryBuffer(d) => !d.borrow().is_empty(),
                // ИЗМЕРЕНО (фикстура `uuid`): нулевой идентификатор не
                // заполнен, любой другой — заполнен.
                BslObject::Uuid(b) => *b != [0; 16],
                // У строки таблицы и пары ключ-значение «длины» нет: сам
                // факт существования объекта и есть заполненность.
                BslObject::TableRow(..)
                | BslObject::TableColumn(..)
                | BslObject::TypeDescription(..)
                | BslObject::ValueComparison
                | BslObject::KeyValuePair(..)
                | BslObject::TextWriter(..) => true,
            },
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
            // Тип члена перечисления — само перечисление.
            BslValue::Enum(e) => enum_kind_type_id(e.kind()),
            // ИЗМЕРЕНО (проба `JSON.ENUM.BARE_NAME`): `ТипЗнч()` голого
            // имени перечисления — отдельный МЕТАТИП, печатающийся как
            // `Перечисление<Имя>`, а не тип членов.
            BslValue::EnumType(k) => TypeId::EnumMeta(*k),
            BslValue::Object(o) => match &**o {
                // Тип объекта компонента — его дескриптор, и только он:
                // ни у официального типа, ни у host-типа больше нет
                // строки в закрытом реестре `TypeId` ядра.
                BslObject::Extension(object) => {
                    return Ok(BslValue::Type(TypeRef::Object(object.type_descriptor())));
                }
                BslObject::VstrOpaque(_) => TypeId::VstrOpaque,
                BslObject::Array(_) => TypeId::Array,
                BslObject::Structure(_) => TypeId::Structure,
                BslObject::Map(_) => TypeId::Map,
                BslObject::ValueTable(_) => TypeId::ValueTable,
                BslObject::TableColumns(_) => TypeId::ValueTableColumns,
                BslObject::TableColumn(..) => TypeId::ValueTableColumn,
                BslObject::TableRow(..) => TypeId::ValueTableRow,
                BslObject::TypeDescription(_) => TypeId::TypeDescription,
                BslObject::ValueComparison => TypeId::ValueComparison,
                BslObject::KeyValuePair(..) => TypeId::KeyAndValue,
                BslObject::TextWriter(..) => {
                    return Err(RtError::TypeError {
                        expected: "Зарегистрированный тип",
                        op: "ТипЗнч",
                    });
                }
                BslObject::BinaryData(..) => TypeId::BinaryData,
                BslObject::BinaryBuffer(..) => TypeId::BinaryDataBuffer,
                BslObject::Uuid(..) => TypeId::Uuid,
            },
        };
        Ok(BslValue::Type(TypeRef::Native(id)))
    }

    /// `Тип("ИмяТипа")` -> `Тип`. Неизвестное имя — ошибка (в 1С тоже
    /// исключение, а не `Неопределено`: опечатка в имени типа обязана
    /// падать сразу, а не молча делать сравнение вечно ложным).
    pub fn type_by_name(&self) -> RtResult<Self> {
        let name = self.as_str("Тип")?.to_string();
        // Имя ищется в таблице ядра: типы компонентов пока опознаются
        // через свои `TypeId` там же (см. `TypeRef`).
        TypeId::lookup(&name)
            .map(|id| BslValue::Type(TypeRef::Native(id)))
            .ok_or(RtError::UnknownType(name))
    }

    /// Создаёт целое число BSL без прямой зависимости компонента от
    /// внутреннего числового крейта runtime.
    pub fn number_from_i64(value: i64) -> Self {
        Self::Number(BslNumber::from_i64(value))
    }

    /// Заворачивает реализацию статически подключённого компонента в
    /// ссылочное значение BSL.
    pub fn new_object(object: impl ObjectProtocol + 'static) -> Self {
        BslValue::Object(Rc::new(BslObject::Extension(ObjectRef::new(object))))
    }

    /// Создаёт непрозрачное обещание, принадлежащее конкретному запуску.
    #[must_use]
    pub fn new_promise(execution_token: ExecutionToken, promise_id: PromiseId) -> Self {
        Self::new_object(PromiseValue::new(execution_token, promise_id))
    }

    /// Идентификаторы обещания либо `None` для значения другого типа.
    #[must_use]
    pub fn promise_identity(&self) -> Option<(ExecutionToken, PromiseId)> {
        let promise = self.object_ref()?.downcast_ref::<PromiseValue>()?;
        Some((promise.execution_token(), promise.promise_id()))
    }

    /// Возвращает расширяемый объект, не раскрывая legacy-представление.
    pub fn object_ref(&self) -> Option<&ObjectRef> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::Extension(object) => Some(object),
                _ => None,
            },
            _ => None,
        }
    }

    /// Возвращает байтовую потоковую возможность внешнего объекта.
    pub fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        self.object_ref()?.byte_stream()
    }

    /// Байты `ДвоичныеДанные` без копирования.
    pub fn binary_data_bytes(&self) -> Option<&[u8]> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryData(bytes) => Some(bytes),
                _ => None,
            },
            _ => None,
        }
    }

    /// Создаёт `БуферДвоичныхДанных` с готовыми байтами и малым порядком.
    pub fn binary_buffer_of(bytes: Vec<u8>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryBuffer(Rc::new(
            std::cell::RefCell::new(bindata::BinBufData::new(bytes, bindata::ByteOrder::Little)),
        ))))
    }

    /// Размер буфера либо `None` для значения другого типа.
    pub fn binary_buffer_len(&self) -> Option<usize> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().len()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Снимок байтов буфера.
    pub fn binary_buffer_bytes(&self) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().to_vec()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Копирует ограниченный отрезок буфера; позиция за концом даёт пустой
    /// отрезок, как чтение потока.
    pub fn binary_buffer_slice(&self, offset: u64, count: usize) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().with_bytes(|bytes| {
                    let Ok(start) = usize::try_from(offset) else {
                        return Vec::new();
                    };
                    let start = start.min(bytes.len());
                    let end = start.saturating_add(count).min(bytes.len());
                    bytes[start..end].to_vec()
                })),
                _ => None,
            },
            _ => None,
        }
    }

    /// Записывает отрезок в существующий буфер без изменения его размера.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для не-буфера или ошибку границ, если отрезок
    /// не помещается в буфер.
    pub fn binary_buffer_write(&self, offset: usize, bytes: &[u8]) -> RtResult<()> {
        let buffer = match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => buffer,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "БуферДвоичныхДанных",
                        op: "Записать",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: "Записать",
                });
            }
        };
        let end = offset.checked_add(bytes.len()).ok_or(RtError::BadIndex)?;
        if end > buffer.borrow().len() {
            return Err(RtError::IndexOutOfBounds {
                index: i64::try_from(end).unwrap_or(i64::MAX),
                len: buffer.borrow().len(),
            });
        }
        buffer.borrow().with_bytes_mut(|target| {
            target[offset..end].copy_from_slice(bytes);
        });
        Ok(())
    }

    /// `Новый ОписаниеТипов("Тип1, Тип2")`.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если аргумент не строка либо содержит имя
    /// незарегистрированного типа.
    /// Имена ищутся там же, где их ищет `Тип("Имя")`: сперва в таблице
    /// ядра, потом среди типов компонентов этого прогона. Без второго
    /// шага `Новый ОписаниеТипов("ЧтениеJSON")` перестал бы работать,
    /// когда компонентные типы ушли из закрытого реестра `TypeId`.
    pub fn new_type_description(names: &BslValue, rt: &RuntimeShapes) -> RtResult<Self> {
        let names = names.as_str("Новый ОписаниеТипов")?.to_string();
        let mut types: Vec<TypeRef> = Vec::new();
        for name in names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let ty = rt
                .resolve_type(name)
                .ok_or_else(|| RtError::UnknownType(name.to_string()))?;
            if !types.contains(&ty) {
                types.push(ty);
            }
        }
        Ok(BslValue::Object(Rc::new(BslObject::TypeDescription(types))))
    }

    pub fn new_value_comparison() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueComparison))
    }

    /// `Закрыть()` — полиморфен по получателю: `ЗаписьТекста` сбрасывает
    /// буфер и ничего не возвращает, `ЗаписьJSON` отдаёт накопленный
    /// текст.
    ///
    /// Разведение делает сам объект: общая диспетчеризация метода не знает его
    /// конкретный тип заранее.
    ///
    /// # Errors
    ///
    /// Ошибку ввода-вывода либо неприменимость метода к получателю.
    pub fn close_object(&self) -> RtResult<BslValue> {
        self.text_writer_close()
    }

    /// Создаёт объект `ЗаписьТекста` и открывает файл для буферизованной
    /// записи UTF-8 С МЕТКОЙ ПОРЯДКА БАЙТОВ.
    ///
    /// `ЗаписьТекста` над файловой системой прогона. Файл открывается ЗДЕСЬ,
    /// в конструкторе, и объект дальше держит только `FileHandle` — поэтому
    /// файловая система нужна ему лишь на время построения (BORROW), и VM
    /// передаёт её ссылкой из окружения (`host.env()?.files()`), как уже
    /// делает для `Новый ДвоичныеДанные`.
    ///
    /// BOM — ИЗМЕРЕНО на 8.3.27: файл, созданный `Новый ЗаписьТекста(Путь)`
    /// без прочих аргументов, начинается с `EF BB BF`. Отключается он
    /// шестым аргументом конструктора, которого здесь пока нет. Существующий
    /// файл обрезается до нулевой длины.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файл невозможно создать.
    pub fn new_text_writer_with_files(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ЗаписьТекста")?.to_string();
        // `File::create` = открыть-или-создать с обрезанием.
        let handle = files
            .open(
                &path,
                crate::FileOpenOptions::write(crate::FileCreate::OpenOrCreate).truncate(true),
            )
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        let mut buffered = std::io::BufWriter::new(handle);
        std::io::Write::write_all(&mut buffered, &[0xef, 0xbb, 0xbf])
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::Object(Rc::new(BslObject::TextWriter(
            std::cell::RefCell::new(Some(buffered)),
        ))))
    }

    /// Записывает строку в буфер объекта `ЗаписьТекста`.
    ///
    /// UTF-16-представление [`BslString`] кодируется непосредственно в
    /// UTF-8 без промежуточного [`String`], а перевод строки разворачивается
    /// в CRLF — см. [`BslString::write_utf8_crlf`].
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
                    text.write_utf8_crlf(
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
                    let mut slot = writer.borrow_mut();
                    // Сброс НА МЕСТЕ, а писатель снимается только ПОСЛЕ
                    // успеха: прежде `take()` забирал буфер ДО `flush()`, и на
                    // отказе `?` уносил ошибку наружу с уже опустевшим слотом
                    // — накопленный текст терялся, а повторный `Закрыть()`
                    // находил `None` и врал успехом при незаписанном тексте.
                    // Теперь на отказе слот цел, и повторный `Закрыть()`
                    // пробует снова.
                    if let Some(writer) = slot.as_mut() {
                        // Сброс буфера в дескриптор, затем ЯВНОЕ закрытие
                        // дескриптора: оба на `?` оставляют слот целым, так
                        // что повторный `Закрыть()` пробует снова (закон
                        // `close` из ABI-G0). `BufWriter` на `Drop` молча
                        // проглотил бы отказ — здесь он наблюдаем.
                        writer
                            .flush()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                        writer
                            .get_mut()
                            .close()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                    }
                    *slot = None;
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

    /// Двоичные данные из готовых байтов — общий конструктор для чтения
    /// файла, `РазделитьДвоичныеДанные` и `СоединитьДвоичныеДанные`.
    pub fn binary_data_of(bytes: impl Into<Rc<[u8]>>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryData(bytes.into())))
    }

    /// Байты значения `ДвоичныеДанные`; для любого другого значения —
    /// ошибка типа с указанием операции, которая его потребовала.
    fn as_binary_data(&self, op: &'static str) -> RtResult<&Rc<[u8]>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::BinaryData(bytes) => Ok(bytes),
                _ => Err(RtError::TypeError {
                    expected: "ДвоичныеДанные",
                    op,
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op,
            }),
        }
    }

    /// `Новый ДвоичныеДанные(ИмяФайла)` — файл читается ЦЕЛИКОМ в память
    /// сразу, как и на платформе (размер известен немедленно, а `Размер()`
    /// после удаления файла продолжает отвечать).
    ///
    /// Конструктор без аргументов, с числом вместо имени файла и с двумя
    /// аргументами платформа отвергает (пробы `BIN.NEW.NOARG`,
    /// `BIN.NEW.NUMARG`, `BIN.NEW.TWOARGS`) — ровно один строковый
    /// аргумент, и это проверяет резолвер.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файла нет, он недоступен или это каталог (пробы
    /// `BIN.NEW.MISSING`, `BIN.NEW.DIR` — платформа в обоих случаях
    /// бросает исключение).
    pub(crate) fn new_binary_data(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ДвоичныеДанные")?.to_string();
        let bytes = files
            .read(&path)
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::binary_data_of(bytes))
    }

    /// `Новый БуферДвоичныхДанных(Размер[, ПорядокБайтов])`.
    ///
    /// Размер обязателен и фиксирует буфер навсегда — роста у него нет;
    /// байты нулевые, порядок по умолчанию `LittleEndian` (измерено).
    /// Пропущенный второй аргумент приходит сюда как
    /// [`BslValue::Undefined`].
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если размер не целое неотрицательное число
    /// (платформа отвергает `-1`, `2.5` и строку `"4"`, а `0` принимает),
    /// если порядок байтов не член `ПорядокБайтов`, а также если буфер
    /// такого размера не удалось разместить в памяти: отказом это лучше,
    /// чем падением процесса на числе из пользовательского текста.
    pub fn new_binary_buffer(size: &BslValue, order: &BslValue) -> RtResult<Self> {
        bindata::new_binary_buffer(size, order)
    }

    /// `Новый УникальныйИдентификатор([СтрокаЛибоУИД])`. Без аргумента —
    /// случайный идентификатор версии 4, со строкой — разбор канонической
    /// формы `8-4-4-4-12` (регистр цифр безразличен), с другим
    /// идентификатором — равная копия (обе формы измерены фикстурой
    /// `uuid`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если аргумент не строка, не идентификатор и
    /// не `Неопределено`, либо строка не в канонической форме.
    pub(crate) fn new_uuid(arg: &BslValue, random: &RandomHandle) -> RtResult<Self> {
        let bytes = match arg {
            BslValue::Undefined => {
                let mut bytes = [0u8; 16];
                random.fill(&mut bytes);
                uuid::v4_from_bytes(bytes)
            }
            BslValue::Str(s) => uuid::parse(&s.to_string())?,
            // Конструктор от другого идентификатора платформа принимает и
            // отдаёт равное значение (измерено фикстурой `uuid`).
            BslValue::Object(o) => match &**o {
                BslObject::Uuid(b) => *b,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Новый УникальныйИдентификатор",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op: "Новый УникальныйИдентификатор",
                });
            }
        };
        Ok(BslValue::Object(Rc::new(BslObject::Uuid(bytes))))
    }

    /// `ДвоичныеДанные.Размер()` — число байтов.
    ///
    /// # Errors
    ///
    /// [`RtError::MethodNotApplicable`], если получатель — не двоичные
    /// данные.
    pub fn binary_data_size(&self) -> RtResult<Self> {
        let bytes = self
            .as_binary_data("Размер")
            .map_err(|_| RtError::MethodNotApplicable {
                method: "Размер",
                receiver: self.type_name(),
            })?;
        Ok(BslValue::Number(BslNumber::from_i64(bytes.len() as i64)))
    }

    /// `РазделитьДвоичныеДанные(Данные, РазмерЧасти)` -> `Массив` частей.
    ///
    /// ИЗМЕРЕНО на 8.3.27 (пробы `BIN.SPLIT.*`) на 13 байтах: по 5 — три
    /// части 5, 5, 3; по 3 — пять частей 3, 3, 3, 3, 1; по 100 и по
    /// 10 000 000 000 — одна часть целиком; по 10 — две части 10 и 3.
    /// То есть хвост КОРОЧЕ, а не дополняется, и размер части больше
    /// целого — не ошибка.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если первый аргумент не двоичные данные
    /// (проба `BIN.SPLIT.BADARG`) либо размер части не целое положительное
    /// число, влезающее в 64 бита без знака: ноль, отрицательное, дробное
    /// и даже числовая СТРОКА `"5"` платформой отвергнуты (пробы
    /// `BIN.SPLIT.ZERO`, `.NEGATIVE`, `.FRACTIONAL`, `.STRSIZE`), а
    /// верхняя граница снята фикстурой `binary-data` с точностью до
    /// единицы: `2^64-1` принимается, `2^64` — уже ошибка.
    pub fn binary_data_split(&self, part_size: &BslValue) -> RtResult<Self> {
        const OP: &str = "РазделитьДвоичныеДанные";
        let bad_size = || RtError::TypeError {
            expected: "Целое положительное число не больше 2^64-1",
            op: OP,
        };
        let bytes = self.as_binary_data(OP)?;
        let size = part_size.as_number(OP).map_err(|_| bad_size())?;
        if !size.is_integer()
            || size.is_negative()
            || size.is_zero()
            || *size > binary_split_max_part()
        {
            return Err(bad_size());
        }
        // Размер части шире `usize` ошибкой НЕ является, пока он в
        // пределах `2^64-1`: платформа на 10^10 и на `2^64-1` одинаково
        // отдаёт одну часть целиком, и насыщение до `usize::MAX` даёт
        // ровно это.
        let size = size
            .to_i64_exact()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(usize::MAX);
        // ПУСТЫЕ данные — краевой случай, где `chunks` расходится с
        // платформой: он не даёт ни одной части, а платформа отдаёт массив
        // из ОДНОЙ пустой части (измерено фикстурой `binary-data`, строка
        // «разбиение пустых»). Пустое на входе — пустое на выходе, но
        // обёрнутое.
        if bytes.is_empty() {
            return Ok(BslValue::new_array(vec![BslValue::binary_data_of(
                Vec::new(),
            )]));
        }
        Ok(BslValue::new_array(
            bytes
                .chunks(size)
                .map(BslValue::binary_data_of)
                .collect::<Vec<_>>(),
        ))
    }

    /// `СоединитьДвоичныеДанные(Массив)` -> склеенные данные в порядке
    /// массива (ИЗМЕРЕНО, проба `BIN.COMBINE.ORDER`: три элемента по 13
    /// байт дают 39 байт, и дамп идёт в порядке массива).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если аргумент не массив (проба
    /// `BIN.COMBINE.NOTARRAY`) или его элемент — не двоичные данные:
    /// платформа отвергает и строку, и `Неопределено` (пробы
    /// `BIN.COMBINE.BADELEM`, `BIN.COMBINE.UNDEF`). Пустой массив
    /// ошибкой НЕ является — он даёт пустые двоичные данные (проба
    /// `BIN.COMBINE.EMPTY`).
    pub fn binary_data_combine(&self) -> RtResult<Self> {
        const OP: &str = "СоединитьДвоичныеДанные";
        let items = match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => items,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: OP,
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: OP,
                });
            }
        };
        let items = items.borrow();
        let mut out = Vec::new();
        for item in items.iter() {
            out.extend_from_slice(item.as_binary_data(OP)?);
        }
        Ok(BslValue::binary_data_of(out))
    }

    /// Индекс должен быть целым неотрицательным числом — `1С` использует
    /// `Число` для индексов, отдельного целочисленного типа нет.
    fn index_as_usize(idx: &BslValue) -> RtResult<usize> {
        let n = idx.as_number("[]").map_err(|_| RtError::BadIndex)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }
}

/// Ручная реализация вместо `derive`: `Массив`/`Структура` — ссылочные
/// типы, `=` для них — тождество объекта (`Rc::ptr_eq`), а не структурное
/// сравнение содержимого (в отличие от `Число`/`Строка`/`Булево`).
/// Строка как условие: набор слов ИЗМЕРЕН перебором на 8.3.27, а не взят
/// из документации. Принимаются ровно шесть написаний, регистр не важен, а
/// пробелы по краям обрезаются («" Истина "» проходит). «yes», «no» и «Y`»
/// платформа НЕ принимает — то есть это не «любое разумное слово», а
/// закрытый список; `None` здесь и означает отказ.
fn condition_word(s: &str) -> Option<bool> {
    let w = s.trim().to_uppercase();
    match w.as_str() {
        "ИСТИНА" | "ДА" | "TRUE" => Some(true),
        "ЛОЖЬ" | "НЕТ" | "FALSE" => Some(false),
        _ => None,
    }
}

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
            // Члены перечисления сравниваются как значения — на этом
            // держится весь потоковый разбор JSON (`Если Т =
            // ТипЗначенияJSON.ИмяСвойства Тогда`).
            (BslValue::Enum(a), BslValue::Enum(b)) => a == b,
            // Голое имя перечисления — тоже значение, тем же рассуждением.
            (BslValue::EnumType(a), BslValue::EnumType(b)) => a == b,
            // Один и тот же объект равен себе при любом виде — для
            // непрозрачных значений это быстрый путь: чтение интернирует
            // повторяющиеся ссылки в один объект, и сравнение текстов до
            // них просто не доходит.
            (BslValue::Object(a), BslValue::Object(b)) if Rc::ptr_eq(a, b) => true,
            (BslValue::Object(a), BslValue::Object(b)) => match (&**a, &**b) {
                // Внешние объекты равны по объявленному ключу «то же
                // состояние, то же место»: обёртку строит каждое обращение
                // к коллекции, и тождества обёрток для измеренных равенств
                // мало. Без ключа — только тождество (быстрый путь выше).
                (BslObject::Extension(x), BslObject::Extension(y)) => {
                    if !std::ptr::eq(x.type_descriptor(), y.type_descriptor()) {
                        return false;
                    }
                    if let Some(equal) = x.value_eq(y) {
                        return equal;
                    }
                    match (x.identity_key(), y.identity_key()) {
                        (Some(kx), Some(ky)) => kx == ky,
                        _ => false,
                    }
                }
                // Непрозрачные значения внутреннего формата равны ПО
                // ТЕКСТУ: две ссылки на один объект базы, прочитанные из
                // разных строк, равны — так ведёт себя платформа
                // (измерено, проба `REF.CAT.RT`).
                (BslObject::VstrOpaque(x), BslObject::VstrOpaque(y)) => x == y,
                // Двоичные данные — тоже значение, а не ссылка: ИЗМЕРЕНО
                // (пробы `BIN.EQ`/`BIN.EQ.DIFF`), что два `Новый
                // ДвоичныеДанные` от ОДНОГО файла равны, а от разных — нет.
                (BslObject::BinaryData(x), BslObject::BinaryData(y)) => x == y,
                // УИД — значение: два идентификатора с одними байтами
                // равны, откуда бы они ни пришли.
                (BslObject::Uuid(x), BslObject::Uuid(y)) => x == y,
                // Узлы DOM — ССЫЛКИ на место в дереве: обёртка каждый раз
                _ => false,
            },
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
            BslValue::Enum(e) => e.hash(state),
            BslValue::EnumType(k) => k.hash(state),
            // Непрозрачное значение хэширует текст, двоичные данные —
            // байты: оба согласованы со своим равенством ПО СОДЕРЖИМОМУ в
            // `PartialEq` выше, иначе ключ `Соответствие` терялся бы.
            BslValue::Object(o) => match &**o {
                BslObject::VstrOpaque(text) => text.hash(state),
                BslObject::BinaryData(bytes) => bytes.hash(state),
                // УИД — значение: хэшируем байты, ровно как `PartialEq` их
                // сравнивает (воспроизведение 3 — иначе два равных УИД дают
                // два ключа `Соответствия`). Совпадение с хэшем `BinaryData`
                // из тех же байтов законно: это коллизия, равными их
                // `PartialEq` не делает (кросс-тип уходит в `_ => false`).
                BslObject::Uuid(bytes) => bytes.hash(state),
                // Внешний объект хэширует то же, чем равняется, и в ТОМ ЖЕ
                // порядке правил, что `PartialEq` выше: сперва содержимое
                // (`value_eq` у типов-значений хэширует представление), затем
                // ключ места (`identity_key`), и только при чистом тождестве —
                // адрес обёртки. Обратный порядок ломал бы ключ `Соответствия`
                // для типа, реализующего ОБА метода: он равнялся бы по
                // `value_eq`, а хэшировался по чужому `identity_key`.
                BslObject::Extension(object) => {
                    std::ptr::from_ref(object.type_descriptor()).hash(state);
                    if object.value_eq(object).is_some() {
                        object.display().hash(state);
                    } else if let Some(key) = object.identity_key() {
                        key.hash(state);
                    } else {
                        Rc::as_ptr(o).hash(state);
                    }
                }
                _ => Rc::as_ptr(o).hash(state),
            },
        }
    }
}

/// Наибольший размер части, который принимает `РазделитьДвоичныеДанные`.
///
/// ИЗМЕРЕНО фикстурой `binary-data` с точностью до единицы: `2^64-1`
/// платформа принимает (и отдаёт одну часть целиком), `2^64` — уже
/// ошибка. То есть счётчик у неё 64-битный БЕЗ знака, а не `i64`: `2^63`
/// тоже проходит.
fn binary_split_max_part() -> BslNumber {
    BslNumber::from_i128(u64::MAX as i128)
}

/// Сколько байтов попадает в строковое представление `ДвоичныеДанные`.
///
/// ИЗМЕРЕНО (проба `BIN.STR.LONG`): у значения в 303 байта `Строка()`
/// печатает ровно 256 пар, за которыми СРАЗУ, без разделяющего пробела,
/// идёт многоточие. Ровно на границе (255/256/257 байт) поведение
/// закреплено фикстурой `binary-data`.
const BINARY_DATA_DISPLAY_LIMIT: usize = 256;

/// Строковое представление `ДвоичныеДанные` — не имя типа, а САМИ БАЙТЫ:
/// шестнадцатеричные пары в ВЕРХНЕМ регистре через пробел, не более
/// [`BINARY_DATA_DISPLAY_LIMIT`] штук, с многоточием у длинного значения
/// (измерено, пробы `BIN.STR`, `BIN.STR.LONG`, `BIN.EMPTY`: у пустых
/// данных представление — пустая строка).
fn binary_data_display(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let shown = bytes.len().min(BINARY_DATA_DISPLAY_LIMIT);
    let mut out = String::with_capacity(shown * 3 + 3);
    for (i, b) in bytes[..shown].iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    if bytes.len() > shown {
        out.push_str("...");
    }
    out
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
            BslValue::Enum(e) => write!(f, "{}", e.display_text()),
            // `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`: по умолчанию — то же имя,
            // что стоит слева от точки в исходном тексте (симметрично
            // `type_name`).
            BslValue::EnumType(k) => write!(f, "{}", k.meta_ru_name()),
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => write!(f, "{}", object.display()),
                BslObject::VstrOpaque(_) => write!(f, "НепрозрачноеЗначение"),
                BslObject::Array(_) => write!(f, "Массив"),
                BslObject::Structure(_) => write!(f, "Структура"),
                BslObject::ValueTable(_) => write!(f, "ТаблицаЗначений"),
                BslObject::TableColumns(_) => write!(f, "КоллекцияКолонокТаблицыЗначений"),
                BslObject::TableColumn(..) => write!(f, "КолонкаТаблицыЗначений"),
                BslObject::TableRow(_, _) => write!(f, "СтрокаТаблицыЗначений"),
                BslObject::TypeDescription(_) => write!(f, "ОписаниеТипов"),
                BslObject::ValueComparison => write!(f, "СравнениеЗначений"),
                BslObject::Map(_) => write!(f, "Соответствие"),
                BslObject::KeyValuePair(_, _) => write!(f, "КлючИЗначение"),
                BslObject::TextWriter(_) => write!(f, "ЗаписьТекста"),
                // УИД печатается своей канонической формой, а не именем
                // типа: `Строка(УИД)` — это и есть его строка (фикстура
                // `uuid`, эталон с платформы).
                BslObject::Uuid(b) => write!(f, "{}", uuid::format(b)),
                // Единственный объект, который печатается СОДЕРЖИМЫМ, а не
                // именем: см. `binary_data_display`.
                BslObject::BinaryData(bytes) => write!(f, "{}", binary_data_display(bytes)),
                // Буфер, в отличие от двоичных данных, печатается ИМЕНЕМ, а
                // не содержимым (измерено): дампа байтов у него нет.
                BslObject::BinaryBuffer(_) => write!(f, "БуферДвоичныхДанных"),
            },
        }
    }
}
