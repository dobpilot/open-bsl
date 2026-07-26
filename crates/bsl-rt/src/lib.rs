//! Минимальный рантайм-слой для M3: значения и арифметика/сравнение над
//! ними. Строки, даты, объекты и коллекции — начиная с M4/M5, здесь их
//! сознательно нет: `BslValue` растёт по мере готовности остальных слоёв,
//! а не заранее под все типы из брифа.

use std::cmp::Ordering;
use std::fmt;

use bsl_number::{BslNumber, NumError};

#[derive(Debug, Clone, PartialEq)]
pub enum BslValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(BslNumber),
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

    pub fn add(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("+")?.add(other.as_number("+")?)?,
        ))
    }

    pub fn sub(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("-")?.sub(other.as_number("-")?)?,
        ))
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

    pub fn neg(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("унарный -")?.neg()))
    }

    pub fn not(&self) -> RtResult<Self> {
        Ok(BslValue::Boolean(!self.as_bool("Не")?))
    }

    /// `И`/`ИЛИ` в 1С НЕ короткозамкнутые: оба операнда всегда вычислены до
    /// вызова этой функции (это делает вызывающий код в VM), здесь только
    /// комбинирование уже готовых булевых значений. Важно: `a? && b?` в
    /// Rust сам короткозамкнутый (`&&` не оценит правую часть, если левая
    /// уже `false`) — это ровно то, чего мы хотим избежать, поэтому обе
    /// стороны приводятся к `bool` до `&&`/`||`, отдельными выражениями.
    pub fn and(&self, other: &Self) -> RtResult<Self> {
        let a = self.as_bool("И")?;
        let b = other.as_bool("И")?;
        Ok(BslValue::Boolean(a && b))
    }

    pub fn or(&self, other: &Self) -> RtResult<Self> {
        let a = self.as_bool("ИЛИ")?;
        let b = other.as_bool("ИЛИ")?;
        Ok(BslValue::Boolean(a || b))
    }

    pub fn as_condition(&self) -> RtResult<bool> {
        self.as_bool("Условие")
    }

    /// Сравнение по значению для `=`/`<>`. Разнотипные значения просто не
    /// равны — это не ошибка (в отличие от `<`/`>`/... между разными типами).
    pub fn eq_value(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Undefined, BslValue::Undefined) => true,
            (BslValue::Null, BslValue::Null) => true,
            (BslValue::Boolean(a), BslValue::Boolean(b)) => a == b,
            (BslValue::Number(a), BslValue::Number(b)) => a == b,
            _ => false,
        }
    }

    pub fn compare(&self, other: &Self, op: &'static str) -> RtResult<Ordering> {
        match (self, other) {
            (BslValue::Number(a), BslValue::Number(b)) => Ok(a.cmp(b)),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op,
            }),
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
    fn logical_ops_do_not_need_short_circuit_evaluation_here() {
        assert_eq!(
            BslValue::Boolean(true).and(&BslValue::Boolean(false)).unwrap(),
            BslValue::Boolean(false)
        );
        assert_eq!(
            BslValue::Boolean(false).or(&BslValue::Boolean(true)).unwrap(),
            BslValue::Boolean(true)
        );
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
}
