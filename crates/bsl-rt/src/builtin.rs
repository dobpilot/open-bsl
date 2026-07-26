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
    /// вывода/UI (и `Формат`, который ещё не готов — `Строка()` через
    /// `Display` тут используется впрямую).
    Message,
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
            _ => return None,
        })
    }

    pub fn arity(self) -> usize {
        match self {
            BuiltinFn::Pow => 2,
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
