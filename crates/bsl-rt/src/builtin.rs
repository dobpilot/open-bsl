use crate::{BslValue, RtResult};

/// Встроенные функции, вызываемые по голому имени (`Sqrt(x)`, `Pow(x,y)`,
/// ...). Разрешаются регистронезависимо (`sqrt` == `Sqrt`), без перевода на
/// русский — в реальной 1С у математических функций нет русских синонимов
/// (в отличие от ключевых слов), что и подтверждает сам n-body: `sqrt(...)`
/// написан строчными буквами прямо в "русском" файле.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFn {
    Sqrt,
    Pow,
    Ln,
    Log10,
    Exp,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    /// Побочный эффект — печать в stdout. Заглушка на месте настоящего
    /// вывода/UI.
    Message,
    /// `Строка(x)` = `Формат(x, Неопределено)` — форматная строка по
    /// умолчанию. Форматирование живёт в `bsl-format` (более высокий
    /// слой, чем этот крейт), поэтому `call_builtin_fn` ниже для этого
    /// варианта не вызывается в реальном пайплайне — VM перехватывает его
    /// раньше (см. `bsl-vm`); здесь только выделено имя-идентификатор.
    ToString,
    /// `Формат(x, spec)` — с явной форматной строкой.
    Format,
    /// `Число(строка)` — обратный разбор форматированной строки в число.
    ToNumber,

    /// `СтрДлина`/`StrLen` — длина в код-юнитах UTF-16, не в символах.
    StrLen,
    /// `Лев`/`Left(строка, длина)`.
    Left,
    /// `Прав`/`Right(строка, длина)`.
    Right,
    /// `Сред`/`Mid(строка, начало, длина)` — пока только с тремя
    /// аргументами (без опускания длины до конца строки): необязательные
    /// аргументы появятся вместе со значениями по умолчанию в вызовах.
    Mid,
    Upper,
    Lower,
    /// `СокрЛП`/`TrimAll` — обрезка пробелов с обеих сторон.
    TrimAll,
}

impl BuiltinFn {
    pub fn lookup(name: &str) -> Option<Self> {
        Some(match name.to_uppercase().as_str() {
            "SQRT" => BuiltinFn::Sqrt,
            "POW" => BuiltinFn::Pow,
            "LOG" => BuiltinFn::Ln,
            "LOG10" => BuiltinFn::Log10,
            "EXP" => BuiltinFn::Exp,
            "SIN" => BuiltinFn::Sin,
            "COS" => BuiltinFn::Cos,
            "TAN" => BuiltinFn::Tan,
            "ASIN" => BuiltinFn::Asin,
            "ACOS" => BuiltinFn::Acos,
            "ATAN" => BuiltinFn::Atan,
            "MESSAGE" | "СООБЩИТЬ" => BuiltinFn::Message,
            "STRING" | "СТРОКА" => BuiltinFn::ToString,
            "FORMAT" | "ФОРМАТ" => BuiltinFn::Format,
            "NUMBER" | "ЧИСЛО" => BuiltinFn::ToNumber,
            "STRLEN" | "СТРДЛИНА" => BuiltinFn::StrLen,
            "LEFT" | "ЛЕВ" => BuiltinFn::Left,
            "RIGHT" | "ПРАВ" => BuiltinFn::Right,
            "MID" | "СРЕД" => BuiltinFn::Mid,
            "UPPER" | "ВРЕГ" => BuiltinFn::Upper,
            "LOWER" | "НРЕГ" => BuiltinFn::Lower,
            "TRIMALL" | "СОКРЛП" => BuiltinFn::TrimAll,
            _ => return None,
        })
    }

    pub fn arity(self) -> usize {
        match self {
            BuiltinFn::Pow | BuiltinFn::Format | BuiltinFn::Left | BuiltinFn::Right => 2,
            BuiltinFn::Mid => 3,
            _ => 1,
        }
    }
}

/// Методы объектов, вызываемые как `а.Метод()`. Пока только `Количество`
/// для `Массив`/`Структура` — остальные (`Добавить`, `Вставить`, ...)
/// приходят волнами, как и описано в брифе для `ТаблицаЗначений`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMethod {
    Count,
}

impl BuiltinMethod {
    pub fn lookup(name: &str) -> Option<Self> {
        Some(match name.to_uppercase().as_str() {
            "COUNT" | "КОЛИЧЕСТВО" => BuiltinMethod::Count,
            _ => return None,
        })
    }
}

pub fn call_builtin_fn(f: BuiltinFn, args: &[BslValue]) -> RtResult<BslValue> {
    match f {
        BuiltinFn::Sqrt => args[0].sqrt(),
        BuiltinFn::Pow => args[0].pow(&args[1]),
        BuiltinFn::Ln => args[0].ln(),
        BuiltinFn::Log10 => args[0].log10(),
        BuiltinFn::Exp => args[0].exp(),
        BuiltinFn::Sin => args[0].sin(),
        BuiltinFn::Cos => args[0].cos(),
        BuiltinFn::Tan => args[0].tan(),
        BuiltinFn::Asin => args[0].asin(),
        BuiltinFn::Acos => args[0].acos(),
        BuiltinFn::Atan => args[0].atan(),
        BuiltinFn::Message => {
            println!("{}", args[0]);
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ToString | BuiltinFn::Format | BuiltinFn::ToNumber => {
            unreachable!(
                "форматозависимые builtin'ы (Строка/Формат/Число) перехватываются в bsl-vm, \
                 у которого есть доступ к bsl-format — сюда попадать не должны"
            )
        }
        BuiltinFn::StrLen => Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
            args[0].str_len()? as i64,
        ))),
        BuiltinFn::Left => args[0].str_left(&args[1]),
        BuiltinFn::Right => args[0].str_right(&args[1]),
        BuiltinFn::Mid => args[0].str_mid(&args[1], &args[2]),
        BuiltinFn::Upper => args[0].str_upper(),
        BuiltinFn::Lower => args[0].str_lower(),
        BuiltinFn::TrimAll => args[0].str_trim_all(),
    }
}

pub fn call_builtin_method(m: BuiltinMethod, obj: &BslValue) -> RtResult<BslValue> {
    match m {
        BuiltinMethod::Count => {
            let len = obj.collection_len()?;
            Ok(BslValue::Number(bsl_number::BslNumber::from_i64(len as i64)))
        }
    }
}
