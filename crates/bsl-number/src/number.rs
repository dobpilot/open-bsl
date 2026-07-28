use std::borrow::Cow;
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

const fn pow10_i64_table() -> [i64; 19] {
    let mut table = [1i64; 19];
    let mut i = 1;
    while i < table.len() {
        table[i] = table[i - 1] * 10;
        i += 1;
    }
    table
}
static POW10_I64: [i64; 19] = pow10_i64_table();

#[inline]
fn scale_up_i64(m: i64, delta: i32) -> Option<i64> {
    if !(0..=18).contains(&delta) {
        return None;
    }
    m.checked_mul(POW10_I64[delta as usize])
}

#[inline]
fn scale_up_i128(m: i128, delta: i32) -> Option<i128> {
    // Ранний выход на РАВНЫХ масштабах. Профилирование флейм-графом
    // показало 11–15% времени на умножении мантиссы на `10^0`: у операндов
    // одного масштаба (а это подавляющее большинство сложений и сравнений)
    // `delta` всегда ноль. Быстрые пути в `add_sub` и `cmp` закрыли самые
    // горячие вызовы, но `div_to_scale` и прочие вызывающие по-прежнему
    // платили за 128-битное `checked_mul` на единицу.
    if delta == 0 {
        return Some(m);
    }
    if delta < 0 || delta > 38 {
        return None;
    }
    m.checked_mul(POW10[delta as usize])
}

/// То же для большого яруса; здесь ранний выход был с самого начала —
/// `delta <= 0` покрывает и ноль, — а `Cow` избавляет этот путь ещё и от
/// клонирования `BigInt`.
fn scale_up_big(m: &BigInt, delta: i32) -> Cow<'_, BigInt> {
    if delta <= 0 {
        return Cow::Borrowed(m);
    }
    Cow::Owned(m * BigInt::from(10u8).pow(delta as u32))
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

    #[inline]
    fn fast64_parts(&self) -> Option<(i64, i32)> {
        match self {
            BslNumber::Small { m, scale } if *scale <= 15 => {
                i64::try_from(m.get()).ok().map(|m| (m, *scale))
            }
            _ => None,
        }
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

    fn big_parts(&self) -> (Cow<'_, BigInt>, i32) {
        match self {
            BslNumber::Small { m, scale } => (Cow::Owned(BigInt::from(m.get())), *scale),
            BslNumber::Big(b) => (Cow::Borrowed(&b.m), b.scale),
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

        if let (Some((a, asc)), Some((b, bsc))) =
            (self.fast64_parts(), other.fast64_parts())
        {
            if let (Some(a), Some(b)) = (scale_up_i64(a, s - asc), scale_up_i64(b, s - bsc)) {
                let result = if negate {
                    a.checked_sub(b)
                } else {
                    a.checked_add(b)
                };
                if let Some(result) = result {
                    return Ok(BslNumber::small(result as i128, s));
                }
            }
        }

        if let (
            BslNumber::Small { m: am, scale: asc },
            BslNumber::Small { m: bm, scale: bsc },
        ) = (self, other)
        {
            // Самый частый путь (целые счётчики, значения одного масштаба):
            // не умножаем обе мантиссы на 10^0. У i128 даже такое
            // checked_mul компилируется в заметно дорогую широкую операцию.
            if asc == bsc {
                let r = if negate {
                    am.get().checked_sub(bm.get())
                } else {
                    am.get().checked_add(bm.get())
                };
                if let Some(r) = r {
                    return Ok(BslNumber::small(r, *asc));
                }
            }
            if let (Some(a), Some(b)) =
                (scale_up_i128(am.get(), s - asc), scale_up_i128(bm.get(), s - bsc))
            {
                let r = if negate { a.checked_sub(b) } else { a.checked_add(b) };
                if let Some(r) = r {
                    return Ok(BslNumber::small(r, s));
                }
            }
        }

        let (a, a_scale) = self.big_parts();
        let (b, b_scale) = other.big_parts();
        let am = scale_up_big(&a, s - a_scale);
        let bm = scale_up_big(&b, s - b_scale);
        Ok(BslNumber::big(
            if negate { &*am - &*bm } else { &*am + &*bm },
            s,
        ))
    }

    /// Умножение точное: масштаб результата — сумма масштабов операндов.
    /// Именно поэтому в 1С расходится n-body: масштаб растёт без границы.
    pub fn mul(&self, other: &Self) -> Result<Self, NumError> {
        let s = self.scale() + other.scale();
        check_scale(s)?;

        if let (Some((a, _)), Some((b, _))) = (self.fast64_parts(), other.fast64_parts()) {
            if let Some(result) = a.checked_mul(b) {
                return Ok(BslNumber::small(result as i128, s));
            }
        }

        if let (
            BslNumber::Small { m: am, scale: _ },
            BslNumber::Small { m: bm, scale: _ },
        ) = (self, other)
        {
            if let Some(r) = am.get().checked_mul(bm.get()) {
                return Ok(BslNumber::small(r, s));
            }
        }

        // ЗДЕСЬ БЫЛ БЫСТРЫЙ ПУТЬ «одна мантисса взаимно проста с 10, значит
        // произведение не кратно 10 — нормализацию можно пропустить». Он
        // НЕВЕРЕН, и вот почему: нормализация НЕ опускает масштаб ниже нуля
        // (инвариант про `100` = мантисса 100, а не 1e2), поэтому целое
        // число спокойно имеет мантиссу, кратную 10. `10^38 * 0,3` даёт
        // тогда мантиссу 3·10^38 при масштабе 1 вместо 3·10^37 при нуле —
        // ненормализованное представление, от которого зависят и хеш, и
        // равенство, а на них — `Соответствие` с числовыми ключами.
        // Регрессионный тест: `mul_normalizes_when_one_operand_goes_big`.
        let (a, _) = self.big_parts();
        let (b, _) = other.big_parts();
        Ok(BslNumber::big(&*a * &*b, s))
    }

    /// Горячий шаг числового `Для`: увеличить счётчик на единицу и сразу
    /// сравнить с заранее вычисленной верхней границей.
    #[inline]
    pub fn increment_and_le(&mut self, bound: &Self) -> Result<bool, NumError> {
        if let (
            BslNumber::Small {
                m: counter,
                scale: 0,
            },
            BslNumber::Small {
                m: limit,
                scale: 0,
            },
        ) = (&mut *self, bound)
        {
            if let Some(next) = counter.get().checked_add(1) {
                *counter = M128::new(next);
                return Ok(next <= limit.get());
            }
        }

        let next = self.add(&BslNumber::from_i64(1))?;
        let keep_going = next <= *bound;
        *self = next;
        Ok(keep_going)
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

        let (a, _) = self.big_parts();
        let (b, _) = other.big_parts();
        let (n, d) = if k >= 0 {
            (scale_up_big(&a, k), b)
        } else {
            (a, scale_up_big(&b, -k))
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
    /// форматированием (`ЧДЦ=N`) и будущими `Окр`/`Round`. Если `scale`
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

        let (m, _) = self.big_parts();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = div_half_up_big(&m, &divisor);
        BslNumber::big(q, target_scale)
    }

    /// Округление к заданному масштабу, ПОЛОВИНА К НУЛЮ.
    ///
    /// ИЗМЕРЕНО на платформе 8.3.27: `Окр(2.5,0,0)` -> 2, `Окр(3.5,0,0)` ->
    /// 3, `Окр(1.5,0,0)` -> 1, `Окр(-2.5,0,0)` -> -2. Тройка на 3,5 и
    /// отличает эту схему от половины-к-чётному, которая дала бы 4 — до
    /// замера здесь стояла именно она, и это было неверно.
    pub fn round_to_scale_half_down(&self, target_scale: i32) -> Self {
        let cur_scale = self.scale();
        if target_scale >= cur_scale {
            return self.clone();
        }
        let delta = cur_scale - target_scale;

        if let BslNumber::Small { m, .. } = self {
            if delta <= 38 {
                if let Some(q) = div_half_down_i128(m.get(), POW10[delta as usize]) {
                    return BslNumber::small(q, target_scale);
                }
            }
        }

        let (m, _) = self.big_parts();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = div_half_down_big(&m, &divisor);
        BslNumber::big(q, target_scale)
    }

    /// Округление К НУЛЮ (отбрасывание) — `Цел`, в отличие от
    /// `round_to_scale` (half-up, деление/`Окр`). `/` у знаковых целых
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

        let (m, _) = self.big_parts();
        let divisor = BigInt::from(10u8).pow(delta as u32);
        let q = &*m / &divisor;
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
    while scale > 0 && i128_is_divisible_by_10(m) {
        m /= 10;
        scale -= 1;
    }
    BslNumber::Small {
        m: M128::new(m),
        scale,
    }
}

#[inline]
fn i128_is_divisible_by_10(value: i128) -> bool {
    let magnitude = value.unsigned_abs();
    let low = magnitude as u64;
    let high = (magnitude >> 64) as u64;

    // 2^64 mod 10 = 6, therefore the decimal remainder of the full
    // 128-bit value can be computed with two constant 64-bit remainders.
    // This avoids compiler-rt's substantially more expensive `__modti3`
    // on the overwhelmingly common path where normalization stops after
    // the first check.
    ((high % 10) * 6 + low % 10).is_multiple_of(10)
}

fn normalize_big(mut m: BigInt, mut scale: i32) -> BigDec {
    if m.is_zero() {
        return BigDec {
            m: BigInt::zero(),
            scale: 0,
        };
    }
    let ten = BigInt::from(10u8);
    while scale > 0 && bigint_is_divisible_by_10(&m) {
        m /= &ten;
        scale -= 1;
    }
    BigDec { m, scale }
}

#[inline]
fn bigint_is_divisible_by_10(value: &BigInt) -> bool {
    let mut digits = value.iter_u64_digits();
    let Some(low) = digits.next() else {
        return true;
    };

    // BigInt uses base 2^64 limbs. Since 2^64 mod 10 = 6 and
    // 6^n mod 10 = 6 for every n >= 1, every limb above the lowest has
    // the same coefficient in the decimal remainder.
    let remainder = digits.fold(low % 10, |remainder, limb| {
        (remainder + (limb % 10) * 6) % 10
    });
    remainder == 0
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

// --- Округление half-down (половина к нулю на ничьей) ---------------------
//
// ИЗМЕРЕНО: третий аргумент `Окр` со значением 0. Никогда не умолчание (там
// half-up, тоже измерено) и никогда для деления. Отличается от half-up
// ровно на точной ничьей: 2,5 -> 2 и 3,5 -> 3 вместо 3 и 4.

fn div_half_down_i128(n: i128, d: i128) -> Option<i128> {
    let q = n.checked_div(d)?;
    let r = n % d;
    if r == 0 {
        return Some(q);
    }
    let rr = r.unsigned_abs();
    let dd = d.unsigned_abs();
    let doubled = rr.checked_mul(2);
    let bump = match doubled {
        // Ровно половина: не двигаем — остаёмся ближе к нулю.
        Some(x) if x == dd => false,
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

fn div_half_down_big(n: &BigInt, d: &BigInt) -> BigInt {
    let q = n / d;
    let r = n % d;
    if r.is_zero() {
        return q;
    }
    let doubled = r.abs() * 2u8;
    let da = d.abs();
    let bump = match doubled.cmp(&da) {
        Ordering::Equal => false,
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
        if let (Some((a, asc)), Some((b, bsc))) =
            (self.fast64_parts(), other.fast64_parts())
        {
            let scale = asc.max(bsc);
            if let (Some(a), Some(b)) =
                (scale_up_i64(a, scale - asc), scale_up_i64(b, scale - bsc))
            {
                return a.cmp(&b);
            }
        }
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
        let (a, a_scale) = self.big_parts();
        let (b, b_scale) = other.big_parts();
        let s = a_scale.max(b_scale);
        scale_up_big(&a, s - a_scale).cmp(&scale_up_big(&b, s - b_scale))
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

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;
    use num_traits::Zero;

    use super::{bigint_is_divisible_by_10, i128_is_divisible_by_10, BslNumber};

    #[test]
    fn fast_divisibility_by_ten_matches_i128_remainder() {
        let values = [
            i128::MIN,
            i128::MIN + 1,
            -(1i128 << 100),
            -10,
            -1,
            0,
            1,
            10,
            (1i128 << 64) - 6,
            1i128 << 100,
            i128::MAX - 7,
            i128::MAX,
        ];

        for value in values {
            assert_eq!(
                i128_is_divisible_by_10(value),
                value % 10 == 0,
                "value = {value}"
            );
        }
    }

    #[test]
    fn fast_i64_paths_promote_without_losing_decimal_exactness() {
        let max = BslNumber::from_i64(i64::MAX);
        let promoted = max.add(&BslNumber::from_i64(1)).unwrap();
        assert!(matches!(promoted, BslNumber::Small { .. }));
        assert_eq!(promoted.to_canonical(), "9223372036854775808");

        let sum = BslNumber::from_parts(1, 1)
            .add(&BslNumber::from_parts(2, 1))
            .unwrap();
        assert_eq!(sum.to_canonical(), "0.3");
    }

    #[test]
    fn limb_divisibility_by_ten_matches_bigint_remainder() {
        let ten = BigInt::from(10);
        for text in [
            "0",
            "1",
            "-10",
            "18446744073709551616",
            "340282366920938463463374607431768211450",
            "-99999999999999999999999999999999999999999999999999",
        ] {
            let value = text.parse::<BigInt>().unwrap();
            assert_eq!(
                bigint_is_divisible_by_10(&value),
                (&value % &ten).is_zero(),
                "value = {value}"
            );
        }
    }

}
