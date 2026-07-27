use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::{NumError, DIV_SCALE, MAX_SCALE};

// --- Упакованная мантисса -------------------------------------------------
//
// У `i128` выравнивание 16 байт, из-за чего enum с таким полем раздувается
// до 32 байт. Пара `u64` даёт то же содержимое при выравнивании 8, и
// `BslNumber` укладывается в 24 байта. Для таблиц значений на миллионы
// ячеек эта треть памяти существенна.

#[derive(Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct M128([u64; 2]);

impl M128 {
    #[inline]
    fn new(v: i128) -> Self {
        let u = v as u128;
        M128([u as u64, (u >> 64) as u64])
    }
    #[inline]
    pub(crate) fn get(self) -> i128 {
        (((self.0[1] as u128) << 64) | self.0[0] as u128) as i128
    }
}

const fn pow10_table() -> [i128; 39] {
    let mut t = [1i128; 39];
    let mut i = 1;
    while i < 39 {
        t[i] = t[i - 1] * 10;
        i += 1;
    }
    t
}
static POW10: [i128; 39] = pow10_table();

#[inline]
fn scale_up_i128(m: i128, delta: i32) -> Option<i128> {
    if delta < 0 || delta > 38 {
        return None;
    }
    m.checked_mul(POW10[delta as usize])
}

fn scale_up_big(m: &BigInt, delta: i32) -> BigInt {
    if delta <= 0 {
        return m.clone();
    }
    m * BigInt::from(10u8).pow(delta as u32)
}

// --- Большой ярус ---------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub struct BigDec {
    pub m: BigInt,
    pub scale: i32,
}

// --- Число ----------------------------------------------------------------

/// Десятичное число BSL.
///
/// Два яруса: `Small` держит мантиссу в `i128` (38 разрядов) без аллокации,
/// `Big` — произвольной длины. После каждой операции выполняется попытка
/// вернуться в `Small`, иначе однажды ушедшее наверх значение навсегда
/// осталось бы на медленном пути.
#[derive(Clone)]
pub enum BslNumber {
    Small { m: M128, scale: i32 },
    Big(Rc<BigDec>),
}

impl BslNumber {
    pub const ZERO: BslNumber = BslNumber::Small {
        m: M128([0, 0]),
        scale: 0,
    };

    #[inline]
    pub fn from_i64(v: i64) -> Self {
        BslNumber::small(v as i128, 0)
    }


    /// Из BigInt-мантиссы и масштаба (используется парсером длинных литералов).
    pub fn from_big_parts(m: BigInt, scale: i32) -> Self {
        BslNumber::big(m, scale)
    }

    /// Мантисса и масштаб: значение равно `m / 10^scale`.
    pub fn from_parts(m: i128, scale: i32) -> Self {
        BslNumber::small(m, scale)
    }

    #[inline]
    fn small(m: i128, scale: i32) -> Self {
        normalize_small(m, scale)
    }

    fn big(m: BigInt, scale: i32) -> Self {
        demote(normalize_big(m, scale))
    }

    pub fn is_zero(&self) -> bool {
        match self {
            BslNumber::Small { m, .. } => m.get() == 0,
            BslNumber::Big(b) => b.m.is_zero(),
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            BslNumber::Small { m, .. } => m.get() < 0,
            BslNumber::Big(b) => b.m.is_negative(),
        }
    }

    /// Целое ли значение (масштаб нулевой после нормализации).
    pub fn is_integer(&self) -> bool {
        self.scale() <= 0
    }

    pub fn scale(&self) -> i32 {
        match self {
            BslNumber::Small { scale, .. } => *scale,
            BslNumber::Big(b) => b.scale,
        }
    }

    pub(crate) fn to_big(&self) -> BigDec {
        match self {
            BslNumber::Small { m, scale } => BigDec {
                m: BigInt::from(m.get()),
                scale: *scale,
            },
            BslNumber::Big(b) => (**b).clone(),
        }
    }

    // --- Точные операции --------------------------------------------------

    pub fn add(&self, other: &Self) -> Result<Self, NumError> {
        self.add_sub(other, false)
    }

    pub fn sub(&self, other: &Self) -> Result<Self, NumError> {
        self.add_sub(other, true)
    }

    fn add_sub(&self, other: &Self, negate: bool) -> Result<Self, NumError> {
        let s = self.scale().max(other.scale());
        check_scale(s)?;

        if let (
            BslNumber::Small { m: am, scale: asc },
            BslNumber::Small { m: bm, scale: bsc },
        ) = (self, other)
        {
            if let (Some(a), Some(b)) =
                (scale_up_i128(am.get(), s - asc), scale_up_i128(bm.get(), s - bsc))
            {
                let r = if negate { a.checked_sub(b) } else { a.checked_add(b) };
                if let Some(r) = r {
                    return Ok(BslNumber::small(r, s));
                }
            }
        }

        let a = self.to_big();
        let b = other.to_big();
        let am = scale_up_big(&a.m, s - a.scale);
        let bm = scale_up_big(&b.m, s - b.scale);
        Ok(BslNumber::big(if negate { am - bm } else { am + bm }, s))
    }

    /// Умножение точное: масштаб результата — сумма масштабов операндов.
    /// Именно поэтому в 1С расходится n-body: масштаб растёт без границы.
    pub fn mul(&self, other: &Self) -> Result<Self, NumError> {
        let s = self.scale() + other.scale();
        check_scale(s)?;

        if let (
            BslNumber::Small { m: am, scale: _ },
            BslNumber::Small { m: bm, scale: _ },
        ) = (self, other)
        {
            if let Some(r) = am.get().checked_mul(bm.get()) {
                return Ok(BslNumber::small(r, s));
            }
        }

        let a = self.to_big();
        let b = other.to_big();
        Ok(BslNumber::big(a.m * b.m, s))
    }

    pub fn neg(&self) -> Self {
        match self {
            BslNumber::Small { m, scale } => BslNumber::small(-m.get(), *scale),
            BslNumber::Big(b) => BslNumber::big(-b.m.clone(), b.scale),
        }
    }

    pub fn abs(&self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }

    // --- Деление: 27 знаков после запятой, half-up ------------------------

    pub fn div(&self, other: &Self) -> Result<Self, NumError> {
        if other.is_zero() {
            return Err(NumError::DivideByZero);
        }
        self.div_to_scale(other, DIV_SCALE)
    }

    fn div_to_scale(&self, other: &Self, target: i32) -> Result<Self, NumError> {
        // value = a.m * 10^(target + b.scale - a.scale) / b.m, округлить half-up
        let k = target + other.scale() - self.scale();

        if let (
            BslNumber::Small { m: am, .. },
            BslNumber::Small { m: bm, .. },
        ) = (self, other)
        {
            let (n, d) = if k >= 0 {
                (scale_up_i128(am.get(), k), Some(bm.get()))
            } else {
                (Some(am.get()), scale_up_i128(bm.get(), -k))
            };
            if let (Some(n), Some(d)) = (n, d) {
                if let Some(q) = div_half_up_i128(n, d) {
                    return Ok(BslNumber::small(q, target));
                }
            }
        }

        let a = self.to_big();
        let b = other.to_big();
        let (n, d) = if k >= 0 {
            (scale_up_big(&a.m, k), b.m.clone())
        } else {
            (a.m.clone(), scale_up_big(&b.m, -k))
        };
        Ok(BslNumber::big(div_half_up_big(&n, &d), target))
    }

    /// Возведение в степень. Целый показатель — точно (проверено: `Pow(10,30)`
    /// даёт единицу и тридцать нулей, а не мусор из double).
    /// Дробный показатель уходит в f64.
    pub fn pow(&self, exp: &Self) -> Result<Self, NumError> {
        if exp.is_integer() {
            if let Some(e) = exp.to_i64_exact() {
                return self.pow_int(e);
            }
        }
        crate::float::via_f64_2(self, exp, f64::powf)
    }

    fn pow_int(&self, e: i64) -> Result<Self, NumError> {
        if e == 0 {
            return Ok(BslNumber::from_i64(1));
        }
        if e < 0 {
            if self.is_zero() {
                return Err(NumError::DivideByZero);
            }
            let p = self.pow_int(-e)?;
            return BslNumber::from_i64(1).div(&p);
        }
        // Бинарное возведение в степень, точное.
        let mut result = BslNumber::from_i64(1);
        let mut base = self.clone();
        let mut n = e as u64;
        while n > 0 {
            if n & 1 == 1 {
                result = result.mul(&base)?;
            }
            n >>= 1;
            if n > 0 {
                base = base.mul(&base)?;
            }
        }
        Ok(result)
    }

    pub fn to_i64_exact(&self) -> Option<i64> {
        if self.scale() > 0 {
            return None;
        }
        match self {
            BslNumber::Small { m, scale } => {
                let v = scale_up_i128(m.get(), -*scale)?;
                i64::try_from(v).ok()
            }
            BslNumber::Big(b) => scale_up_big(&b.m, -b.scale).to_i64(),
        }
    }

    /// Округление ЗНАЧЕНИЯ (не только для показа) к заданному масштабу,
    /// половина-вверх-от-нуля — та же схема, что и у деления. Используется
    /// форматированием (`ЧДЦ=N`) и будущими `Округл`/`Round`. Если `scale`
    /// не меньше текущего, значение не меняется: досыпать лишние дробные
    /// разряды — забота форматирования (нулями), не самого числа.
    pub fn round_to_scale(&self, target_scale: i32) -> Self {
        let cur_scale = self.scale();
        if target_scale >= cur_scale {
            return self.clone();
        }
        let delta = cur_scale - target_scale;

        if let BslNumber::Small { m, .. } = self {
            if delta <= 38 {
                if let Some(q) = div_half_up_i128(m.get(), POW10[delta as usize]) {
                    return BslNumber::small(q, target_scale);
                }
            }
        }

        let b = self.to_big();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = div_half_up_big(&b.m, &divisor);
        BslNumber::big(q, target_scale)
    }

    /// Округление к заданному масштабу, ПОЛОВИНА-К-ЧЁТНОМУ. Отдельная
    /// функция, а не параметр `round_to_scale`: half-up снят с платформы
    /// точной ничьей (см. `div_half_up_i128`) и трогать его нельзя, а
    /// half-even ничем не подтверждён и существует только под явно
    /// запрошенный третий аргумент `Округл` (см. `BslValue::round`).
    pub fn round_to_scale_half_even(&self, target_scale: i32) -> Self {
        let cur_scale = self.scale();
        if target_scale >= cur_scale {
            return self.clone();
        }
        let delta = cur_scale - target_scale;

        if let BslNumber::Small { m, .. } = self {
            if delta <= 38 {
                if let Some(q) = div_half_even_i128(m.get(), POW10[delta as usize]) {
                    return BslNumber::small(q, target_scale);
                }
            }
        }

        let b = self.to_big();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = div_half_even_big(&b.m, &divisor);
        BslNumber::big(q, target_scale)
    }

    /// Округление К НУЛЮ (отбрасывание) — `Цел`, в отличие от
    /// `round_to_scale` (half-up, деление/`Округл`). `/` у знаковых целых
    /// в Rust и у `BigInt` в `num-bigint` УЖЕ усечение к нулю, поэтому
    /// достаточно голого целочисленного деления без поправок на знак —
    /// не нужна отдельная ветка для отрицательных, как в
    /// `div_half_up_i128`/`div_half_up_big`.
    pub fn trunc_to_scale(&self, target_scale: i32) -> Self {
        let cur_scale = self.scale();
        if target_scale >= cur_scale {
            return self.clone();
        }
        let delta = cur_scale - target_scale;

        if let BslNumber::Small { m, .. } = self {
            if delta <= 38 {
                let q = m.get() / POW10[delta as usize];
                return BslNumber::small(q, target_scale);
            }
        }

        let b = self.to_big();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = &b.m / &divisor;
        BslNumber::big(q, target_scale)
    }
}

// --- Нормализация ---------------------------------------------------------
//
// Хвостовые нули срезаются: измерено `1.10 * 1.00` = `1.1`. Это удерживает
// значения в быстром ярусе и делает хеш независимым от представления.
// Масштаб ниже нуля не опускаем: `100` остаётся мантиссой 100, а не 1e2.

fn normalize_small(mut m: i128, mut scale: i32) -> BslNumber {
    if m == 0 {
        return BslNumber::Small {
            m: M128::new(0),
            scale: 0,
        };
    }
    while scale > 0 && m % 10 == 0 {
        m /= 10;
        scale -= 1;
    }
    BslNumber::Small {
        m: M128::new(m),
        scale,
    }
}

fn normalize_big(mut m: BigInt, mut scale: i32) -> BigDec {
    if m.is_zero() {
        return BigDec {
            m: BigInt::zero(),
            scale: 0,
        };
    }
    let ten = BigInt::from(10u8);
    while scale > 0 && (&m % &ten).is_zero() {
        m /= &ten;
        scale -= 1;
    }
    BigDec { m, scale }
}

/// Попытка вернуться в быстрый ярус после операции на BigInt.
fn demote(b: BigDec) -> BslNumber {
    match (&b.m).to_i128() {
        Some(v) => BslNumber::Small {
            m: M128::new(v),
            scale: b.scale,
        },
        None => BslNumber::Big(Rc::new(b)),
    }
}

fn check_scale(s: i32) -> Result<(), NumError> {
    if s > MAX_SCALE {
        Err(NumError::ScaleOverflow)
    } else {
        Ok(())
    }
}

// --- Округление half-up (от нуля на ничьей) -------------------------------
//
// Измерено на 1/2^28 = 0.0000000037252902984619140625 — ровно 28 знаков
// с пятёркой на конце. Платформа дала ...063, то есть вверх.
// Half-even и truncate дали бы ...062.

fn div_half_up_i128(n: i128, d: i128) -> Option<i128> {
    let q = n.checked_div(d)?;
    let r = n % d;
    if r == 0 {
        return Some(q);
    }
    let rr = (r.unsigned_abs()) as u128;
    let dd = (d.unsigned_abs()) as u128;
    let bump = rr.checked_mul(2).map(|x| x >= dd).unwrap_or(true);
    if !bump {
        return Some(q);
    }
    let neg = (n < 0) != (d < 0);
    if neg {
        q.checked_sub(1)
    } else {
        q.checked_add(1)
    }
}

fn div_half_up_big(n: &BigInt, d: &BigInt) -> BigInt {
    let q = n / d;
    let r = n % d;
    if r.is_zero() {
        return q;
    }
    let bump = (r.abs() * 2u8) >= d.abs();
    if !bump {
        return q;
    }
    let neg = n.is_negative() != d.is_negative();
    if neg {
        q - BigInt::one()
    } else {
        q + BigInt::one()
    }
}

// --- Округление half-even (к чётному на ничьей) ---------------------------
//
// НЕ ИЗМЕРЕНО на платформе — существует только как ЯВНО ЗАПРОШЕННЫЙ третий
// аргумент `Округл`, никогда как умолчание и никогда для деления (там
// half-up подтверждён точной ничьей, см. выше). Отличается от half-up
// ровно на точной ничьей: 2,5 -> 2 и 3,5 -> 4 вместо 3 и 4.

fn div_half_even_i128(n: i128, d: i128) -> Option<i128> {
    let q = n.checked_div(d)?;
    let r = n % d;
    if r == 0 {
        return Some(q);
    }
    let rr = r.unsigned_abs();
    let dd = d.unsigned_abs();
    let doubled = rr.checked_mul(2);
    let bump = match doubled {
        // Ровно половина: округляем только если частное нечётное — чтобы
        // результат оказался чётным.
        Some(x) if x == dd => q % 2 != 0,
        Some(x) => x > dd,
        // Переполнение удвоения означает, что остаток заведомо больше
        // половины делителя.
        None => true,
    };
    if !bump {
        return Some(q);
    }
    let neg = (n < 0) != (d < 0);
    if neg {
        q.checked_sub(1)
    } else {
        q.checked_add(1)
    }
}

fn div_half_even_big(n: &BigInt, d: &BigInt) -> BigInt {
    let q = n / d;
    let r = n % d;
    if r.is_zero() {
        return q;
    }
    let doubled = r.abs() * 2u8;
    let da = d.abs();
    let bump = match doubled.cmp(&da) {
        Ordering::Equal => !(&q % 2u8).is_zero(),
        Ordering::Greater => true,
        Ordering::Less => false,
    };
    if !bump {
        return q;
    }
    let neg = n.is_negative() != d.is_negative();
    if neg {
        q - BigInt::one()
    } else {
        q + BigInt::one()
    }
}

// --- Сравнение и хеш ------------------------------------------------------
//
// Сравнение по значению, не по представлению: 1.0 = 1.00. Поскольку
// хвостовые нули срезаются при нормализации, равные значения имеют
// идентичное представление, и хеш согласован с равенством автоматически.

impl PartialEq for BslNumber {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for BslNumber {}

impl PartialOrd for BslNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BslNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        if let (
            BslNumber::Small { m: a, scale: asc },
            BslNumber::Small { m: b, scale: bsc },
        ) = (self, other)
        {
            if asc == bsc {
                return a.get().cmp(&b.get());
            }
            let s = (*asc).max(*bsc);
            if let (Some(x), Some(y)) =
                (scale_up_i128(a.get(), s - asc), scale_up_i128(b.get(), s - bsc))
            {
                return x.cmp(&y);
            }
        }
        let a = self.to_big();
        let b = other.to_big();
        let s = a.scale.max(b.scale);
        scale_up_big(&a.m, s - a.scale).cmp(&scale_up_big(&b.m, s - b.scale))
    }
}

impl Hash for BslNumber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            BslNumber::Small { m, scale } => {
                // Малое и большое представление одного значения не пересекаются:
                // demote гарантирует, что всё влезающее в i128 лежит в Small.
                m.get().hash(state);
                scale.hash(state);
            }
            BslNumber::Big(b) => {
                b.m.hash(state);
                b.scale.hash(state);
            }
        }
    }
}

impl std::fmt::Debug for BslNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}
