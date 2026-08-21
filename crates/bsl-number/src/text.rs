use crate::number::Repr;
use num_bigint::BigInt;
use num_traits::Signed;

use crate::NumError;
use crate::number::BslNumber;

impl BslNumber {
    /// Каноническая форма: точка как разделитель, без группировки, без
    /// экспоненты. Соответствует `Формат(x, "ЧГ=0; ЧРД=.")` в 1С и
    /// разбирается обратно без потерь — на этом строится дифф-харнесс.
    pub fn to_canonical(&self) -> String {
        let (digits, neg, scale) = match &self.0 {
            Repr::Small { m, scale } => {
                let v = m.get();
                (v.unsigned_abs().to_string(), v < 0, *scale)
            }
            Repr::Big(b) => (b.m.abs().to_string(), b.m.is_negative(), b.scale),
        };

        let mut out = String::new();
        if neg {
            out.push('-');
        }

        if scale <= 0 {
            out.push_str(&digits);
            for _ in 0..(-scale) {
                out.push('0');
            }
            return out;
        }

        let scale = scale as usize;
        if digits.len() > scale {
            let split = digits.len() - scale;
            out.push_str(&digits[..split]);
            out.push('.');
            out.push_str(&digits[split..]);
        } else {
            out.push_str("0.");
            for _ in 0..(scale - digits.len()) {
                out.push('0');
            }
            out.push_str(&digits);
        }
        out
    }

    /// Разбор канонической формы. Экспоненты нет — в BSL числовых литералов
    /// с экспонентой не существует, поэтому автор n-body-теста и писал
    /// константы дробями вида `-103622044471123109/1000000000000000000`.
    pub fn parse_canonical(s: &str) -> Result<Self, NumError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(NumError::BadLiteral);
        }

        let (neg, body) = match s.as_bytes()[0] {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        if body.is_empty() {
            return Err(NumError::BadLiteral);
        }

        let (int_part, frac_part) = match body.split_once('.') {
            Some((a, b)) => (a, b),
            None => (body, ""),
        };

        if int_part.is_empty() && frac_part.is_empty() {
            return Err(NumError::BadLiteral);
        }
        if !int_part.bytes().all(|c| c.is_ascii_digit())
            || !frac_part.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(NumError::BadLiteral);
        }

        let mut digits = String::with_capacity(int_part.len() + frac_part.len());
        digits.push_str(int_part);
        digits.push_str(frac_part);
        let scale = frac_part.len() as i32;

        match digits.parse::<i128>() {
            Ok(v) => BslNumber::from_parts(if neg { -v } else { v }, scale),
            Err(_) => {
                let m: BigInt = digits.parse().map_err(|_| NumError::BadLiteral)?;
                BslNumber::from_big_parts(if neg { -m } else { m }, scale)
            }
        }
    }
}

impl std::fmt::Display for BslNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_canonical())
    }
}
