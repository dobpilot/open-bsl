//! Числовой тип BSL, снятый с поведения 1С:Предприятие.
//!
//! Установленная семантика (все значения подтверждены на оракуле):
//!
//! | операция              | поведение                                  |
//! |-----------------------|--------------------------------------------|
//! | `+` `-` `*`           | точно, масштаб выводится из операндов      |
//! | `/`                   | 27 знаков после запятой, half-up           |
//! | `Pow` целый показатель| точно                                      |
//! | `Sqrt`, тригонометрия | через f64, 15 значащих разрядов            |
//! | хвостовые нули        | срезаются                                  |

mod float;
mod number;
mod text;

pub use number::BslNumber;

/// Знаков после запятой в результате деления. Замер: `1/3` → 27 троек.
pub const DIV_SCALE: i32 = 27;

/// Значащих разрядов на возврате из f64. Замер: `Sqrt(2)` → 1,4142135623731.
pub const F64_SIG: usize = 15;

/// Предел масштаба. В 1С границы нет, но неограниченный рост съедает память,
/// поэтому здесь стоит защитный барьер с внятной ошибкой вместо OOM.
pub const MAX_SCALE: i32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumError {
    /// Деление на ноль.
    DivideByZero,
    /// Аргумент вне области определения (`Sqrt(-1)`, `Log(0)`).
    Domain(&'static str),
    /// Значение не переводится в f64 без потери в бесконечность.
    NotFinite,
    /// Превышен защитный предел масштаба.
    ScaleOverflow,
    /// Строка не разбирается как число.
    BadLiteral,
}

impl std::fmt::Display for NumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumError::DivideByZero => write!(f, "деление на ноль"),
            NumError::Domain(op) => write!(f, "аргумент вне области определения: {op}"),
            NumError::NotFinite => write!(f, "значение не представимо в f64"),
            NumError::ScaleOverflow => write!(f, "превышен предел масштаба"),
            NumError::BadLiteral => write!(f, "некорректный числовой литерал"),
        }
    }
}

impl std::error::Error for NumError {}

pub type NumResult<T> = Result<T, NumError>;
