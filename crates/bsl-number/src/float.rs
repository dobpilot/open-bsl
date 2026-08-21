use crate::number::BslNumber;
use crate::{F64_SIG, NumError};

/// Функции, которые 1С считает в двойной точности.
///
/// Установлено измерением: `Sqrt(2)` даёт `1,4142135623731`, а не 27 знаков,
/// как деление. Это 15 значащих разрядов — то есть платформа уходит в f64,
/// считает и возвращается в decimal, отбрасывая мусорные разряды.
///
/// Кажущееся противоречие с `Sqrt(0.02)` = `0,141421356237309` снимается тем,
/// что десятичные цифры у обоих корней одинаковые, но f64-приближения ложатся
/// по разные стороны от границы округления 16-го разряда.
impl BslNumber {
    pub fn to_f64(&self) -> f64 {
        // Через каноническую строку: медленно, зато корректно округляется
        // и не теряет разрядов на больших мантиссах.
        self.to_canonical().parse::<f64>().unwrap_or(f64::NAN)
    }

    /// Обратный переход: 15 значащих разрядов, затем нормализация.
    /// Это единственное место, задающее точность трансцендентных функций.
    ///
    /// # Errors
    ///
    /// Возвращает [`NumError::NotFinite`], если `x` не является конечным числом или его
    /// нельзя преобразовать в десятичное представление.
    pub fn from_f64(x: f64) -> Result<Self, NumError> {
        if !x.is_finite() {
            return Err(NumError::NotFinite);
        }
        if x == 0.0 {
            return Ok(BslNumber::ZERO);
        }
        // {:.14e} — это 15 значащих разрядов в научной записи.
        let sci = format!("{:.*e}", F64_SIG - 1, x);
        let (mant, exp) = sci.split_once('e').ok_or(NumError::NotFinite)?;
        let exp: i32 = exp.parse().map_err(|_| NumError::NotFinite)?;

        let neg = mant.starts_with('-');
        let mant = mant.trim_start_matches('-');
        let (int_part, frac_part) = match mant.split_once('.') {
            Some((a, b)) => (a, b),
            None => (mant, ""),
        };
        let digits = format!("{}{}", int_part, frac_part);
        // Значение = digits / 10^frac_len * 10^exp
        let scale = frac_part.len() as i32 - exp;
        let s = if neg { format!("-{}", digits) } else { digits };
        let m: i128 = s.parse().map_err(|_| NumError::NotFinite)?;
        BslNumber::from_parts(m, scale)
    }

    pub fn sqrt(&self) -> Result<Self, NumError> {
        if self.is_negative() {
            return Err(NumError::Domain("f64"));
        }
        via_f64(self, f64::sqrt)
    }

    pub fn ln(&self) -> Result<Self, NumError> {
        if self.is_negative() || self.is_zero() {
            return Err(NumError::Domain("f64"));
        }
        via_f64(self, f64::ln)
    }

    pub fn log10(&self) -> Result<Self, NumError> {
        if self.is_negative() || self.is_zero() {
            return Err(NumError::Domain("f64"));
        }
        via_f64(self, f64::log10)
    }

    pub fn exp(&self) -> Result<Self, NumError> {
        via_f64(self, f64::exp)
    }

    pub fn sin(&self) -> Result<Self, NumError> {
        via_f64(self, f64::sin)
    }
    pub fn cos(&self) -> Result<Self, NumError> {
        via_f64(self, f64::cos)
    }
    pub fn tan(&self) -> Result<Self, NumError> {
        via_f64(self, f64::tan)
    }
    pub fn asin(&self) -> Result<Self, NumError> {
        via_f64(self, f64::asin)
    }
    pub fn acos(&self) -> Result<Self, NumError> {
        via_f64(self, f64::acos)
    }
    pub fn atan(&self) -> Result<Self, NumError> {
        via_f64(self, f64::atan)
    }
}

pub(crate) fn via_f64(x: &BslNumber, f: impl Fn(f64) -> f64) -> Result<BslNumber, NumError> {
    let d = x.to_f64();
    if !d.is_finite() {
        return Err(NumError::NotFinite);
    }
    let r = f(d);
    if !r.is_finite() {
        return Err(NumError::Domain("f64"));
    }
    BslNumber::from_f64(r)
}

pub(crate) fn via_f64_2(
    a: &BslNumber,
    b: &BslNumber,
    f: impl Fn(f64, f64) -> f64,
) -> Result<BslNumber, NumError> {
    let (x, y) = (a.to_f64(), b.to_f64());
    if !x.is_finite() || !y.is_finite() {
        return Err(NumError::NotFinite);
    }
    let r = f(x, y);
    if !r.is_finite() {
        return Err(NumError::Domain("f64"));
    }
    BslNumber::from_f64(r)
}
