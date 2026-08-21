use std::borrow::Cow;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::{DIV_SCALE, MAX_SCALE, NumError};

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
    if !(0..=38).contains(&delta) {
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
pub struct BslNumber(pub(crate) Repr);

/// Представление числа. ПРИВАТНО СОЗНАТЕЛЬНО: пока варианты были
/// публичными, внешний клиент мог взять обычное число, сопоставить
/// `Small { scale, .. }` и записать туда `i32::MIN` мимо проверяющих
/// конструкторов. После этого печать паниковала в debug на `-scale`, а в
/// release выдавала заведомо неверную строку. Инвариант «масштаб в
/// пределах `MAX_SCALE`» держится конструкторами, и обойти их теперь
/// нечем.
#[derive(Clone)]
pub(crate) enum Repr {
    Small { m: M128, scale: i32 },
    Big(Rc<BigDec>),
}

impl BslNumber {
    pub const ZERO: BslNumber = BslNumber(Repr::Small {
        m: M128([0, 0]),
        scale: 0,
    });

    #[inline]
    pub fn from_i64(v: i64) -> Self {
        BslNumber::small(v as i128, 0)
    }

    /// Приводит отрицательный масштаб к нулю, материализуя нули мантиссы.
    ///
    /// Отрицательный масштаб законен на ВХОДЕ, но храниться не должен.
    /// `normalize_small` снимает лишние нули только при `scale > 0`,
    /// поэтому `1000` легло бы как `{m: 1000, scale: 0}`, а `1×10³` — как
    /// `{m: 1, scale: -3}`: по `Eq` это одно значение, а хеш у них разный,
    /// то есть контракт `Hash` нарушен и `Соответствие` с числовым ключом
    /// теряло бы записи. Нормализация на входе закрывает это, не трогая ни
    /// `Hash`, ни порядок, ни арифметику.
    ///
    /// Арифметика отрицательных масштабов не порождает: `normalize_small`
    /// доводит масштаб только до нуля, — поэтому одного этого места
    /// достаточно.
    fn with_scale_raised(m: BigInt, scale: i32) -> Self {
        debug_assert!(scale < 0, "путь только для отрицательного масштаба");
        // Масштаб проверен пределом, значит `-scale <= MAX_SCALE` и в `u32`
        // влезает. `demote` зовётся напрямую, а не через `big`, чтобы не
        // возвращаться сюда же.
        let factor = BigInt::from(10u8).pow((-scale) as u32);
        demote(normalize_big(m * factor, 0))
    }

    /// Целое из `i128`: масштаб ноль, проверять нечего.
    #[inline]
    pub fn from_i128(m: i128) -> Self {
        BslNumber::small(m, 0)
    }

    /// Из BigInt-мантиссы и масштаба (используется парсером длинных
    /// литералов).
    ///
    /// # Errors
    ///
    /// [`NumError::ScaleOverflow`], если `|scale| > MAX_SCALE`.
    pub fn from_big_parts(m: BigInt, scale: i32) -> Result<Self, NumError> {
        check_scale(scale)?;
        Ok(BslNumber::big(m, scale))
    }

    /// Мантисса и масштаб: значение равно `m / 10^scale`.
    ///
    /// Проверяет масштаб, потому что предел [`MAX_SCALE`] — операционный
    /// инвариант арифметики, а не рекомендация: число, построенное за его
    /// пределами, ломало бы каждую операцию над собой. Предел СИММЕТРИЧЕН:
    /// отрицательный масштаб законен и нужен (`Формат` строит им степени
    /// десятки), но ограничен так же, как положительный, иначе разности
    /// масштабов выходят за `i32`.
    ///
    /// Отрицательный масштаб канонизируется на месте: `1×10³` хранится тем
    /// же представлением, что и `1000`, — иначе равные числа хешировались
    /// бы по-разному.
    ///
    /// # Errors
    ///
    /// [`NumError::ScaleOverflow`], если `|scale| > MAX_SCALE`.
    pub fn from_parts(m: i128, scale: i32) -> Result<Self, NumError> {
        check_scale(scale)?;
        Ok(BslNumber::small(m, scale))
    }

    #[inline]
    fn small(m: i128, scale: i32) -> Self {
        if scale < 0 {
            return Self::with_scale_raised(BigInt::from(m), scale);
        }
        normalize_small(m, scale)
    }

    fn big(m: BigInt, scale: i32) -> Self {
        if scale < 0 {
            return Self::with_scale_raised(m, scale);
        }
        demote(normalize_big(m, scale))
    }

    #[inline]
    fn fast64_parts(&self) -> Option<(i64, i32)> {
        match &self.0 {
            Repr::Small { m, scale } if *scale <= 15 => {
                i64::try_from(m.get()).ok().map(|m| (m, *scale))
            }
            _ => None,
        }
    }

    pub fn is_zero(&self) -> bool {
        match &self.0 {
            Repr::Small { m, .. } => m.get() == 0,
            Repr::Big(b) => b.m.is_zero(),
        }
    }

    pub fn is_negative(&self) -> bool {
        match &self.0 {
            Repr::Small { m, .. } => m.get() < 0,
            Repr::Big(b) => b.m.is_negative(),
        }
    }

    /// Целое ли значение (масштаб нулевой после нормализации).
    pub fn is_integer(&self) -> bool {
        self.scale() <= 0
    }

    pub fn scale(&self) -> i32 {
        match &self.0 {
            Repr::Small { scale, .. } => *scale,
            Repr::Big(b) => b.scale,
        }
    }

    fn big_parts(&self) -> (Cow<'_, BigInt>, i32) {
        match &self.0 {
            Repr::Small { m, scale } => (Cow::Owned(BigInt::from(m.get())), *scale),
            Repr::Big(b) => (Cow::Borrowed(&b.m), b.scale),
        }
    }

    // --- Точные операции --------------------------------------------------

    pub fn add(&self, other: &Self) -> Result<Self, NumError> {
        self.add_sub(other, false)
    }

    /// Прибавляет `other` к числу на месте.
    ///
    /// Целые счётчики меняются без создания и последующего уничтожения
    /// промежуточного `BslNumber`. Остальные представления проходят через
    /// обычное сложение, поэтому нормализация десятичного числа сохраняется.
    ///
    /// # Errors
    ///
    /// Возвращает [`NumError::ScaleOverflow`], если общий масштаб операндов
    /// превышает защитный предел.
    #[inline]
    pub fn add_assign(&mut self, other: &Self) -> Result<(), NumError> {
        if let (Repr::Small { m: left, scale: 0 }, Repr::Small { m: right, scale: 0 }) =
            (&mut self.0, &other.0)
            && let Some(sum) = left.get().checked_add(right.get())
        {
            *left = M128::new(sum);
            return Ok(());
        }

        *self = self.add(other)?;
        Ok(())
    }

    pub fn sub(&self, other: &Self) -> Result<Self, NumError> {
        self.add_sub(other, true)
    }

    fn add_sub(&self, other: &Self, negate: bool) -> Result<Self, NumError> {
        let s = self.scale().max(other.scale());
        check_scale(s)?;

        if let (Some((a, asc)), Some((b, bsc))) = (self.fast64_parts(), other.fast64_parts())
            && let (Some(a), Some(b)) = (scale_up_i64(a, s - asc), scale_up_i64(b, s - bsc))
        {
            let result = if negate {
                a.checked_sub(b)
            } else {
                a.checked_add(b)
            };
            if let Some(result) = result {
                return Ok(BslNumber::small(result as i128, s));
            }
        }

        if let (Repr::Small { m: am, scale: asc }, Repr::Small { m: bm, scale: bsc }) =
            (&self.0, &other.0)
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
            if let (Some(a), Some(b)) = (
                scale_up_i128(am.get(), s - asc),
                scale_up_i128(bm.get(), s - bsc),
            ) {
                let r = if negate {
                    a.checked_sub(b)
                } else {
                    a.checked_add(b)
                };
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
        // Проверяемое сложение, а не `+`: два больших масштаба переполняют
        // `i32` ДО того, как до них доберётся `check_scale`, и тогда в debug
        // это паника, а в release — молча неверный ответ (масштаб
        // заворачивается в отрицательный и проверку проходит).
        let s = self
            .scale()
            .checked_add(other.scale())
            .ok_or(NumError::ScaleOverflow)?;
        // Предел проверяется по РЕЗУЛЬТАТУ, а не по сырой сумме: нормализация
        // снимает хвостовые нули и опускает масштаб обратно. `4e-50001` на
        // `25e-50001` даёт сырые 100 002, но мантиссы дают 100, и после
        // снятия двух нулей выходит допустимое `1e-100000`. Проверка до
        // умножения отвергала такое произведение зря.
        //
        // Хранимый масштаб неотрицателен всегда (отрицательный канонизируется
        // в конструкторе), поэтому сумма не уходит вниз и переполнить `i32`
        // может только вверх — этим и занят `checked_add`.
        if let (Some((a, _)), Some((b, _))) = (self.fast64_parts(), other.fast64_parts())
            && let Some(result) = a.checked_mul(b)
        {
            return checked_result(BslNumber::small(result as i128, s));
        }

        if let (Repr::Small { m: am, scale: _ }, Repr::Small { m: bm, scale: _ }) =
            (&self.0, &other.0)
            && let Some(r) = am.get().checked_mul(bm.get())
        {
            return checked_result(BslNumber::small(r, s));
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
        checked_result(BslNumber::big(&*a * &*b, s))
    }

    /// Горячий шаг числового `Для`: увеличить счётчик на единицу и сразу
    /// сравнить с заранее вычисленной верхней границей.
    #[inline]
    pub fn increment_and_le(&mut self, bound: &Self) -> Result<bool, NumError> {
        if let (
            Repr::Small {
                m: counter,
                scale: 0,
            },
            Repr::Small { m: limit, scale: 0 },
        ) = (&mut self.0, &bound.0)
            && let Some(next) = counter.get().checked_add(1)
        {
            *counter = M128::new(next);
            return Ok(next <= limit.get());
        }

        let next = self.add(&BslNumber::from_i64(1))?;
        let keep_going = next <= *bound;
        *self = next;
        Ok(keep_going)
    }

    pub fn neg(&self) -> Self {
        match &self.0 {
            // У `i128::MIN` положительного двойника в `i128` НЕТ, поэтому
            // обычный минус там паникует в debug и возвращает то же
            // отрицательное число в release. Такой случай переезжает в
            // большой ярус, где предела нет.
            Repr::Small { m, scale } => match m.get().checked_neg() {
                Some(v) => BslNumber::small(v, *scale),
                None => BslNumber::big(-BigInt::from(m.get()), *scale),
            },
            Repr::Big(b) => BslNumber::big(-b.m.clone(), b.scale),
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

    /// Остаток от деления (`%`).
    ///
    /// Считается ТОЧНО, на общей шкале мантисс, а не через `div`: деление
    /// здесь округляет на 27-м знаке, и усечённое частное могло бы
    /// промахнуться на единицу там, где точное частное лежит к целому
    /// ближе, чем на этот знак. С общей шкалой ни округления, ни поправок
    /// не нужно вовсе: `a % b` целых мантисс и есть ответ.
    ///
    /// Знак результата — по ДЕЛИМОМУ, и это ИЗМЕРЕНО на 8.3.27 (замеры
    /// `MOD.NEGATIVE_LEFT`, `MOD.NEGATIVE_RIGHT`, `MOD.BOTH_NEGATIVE`):
    /// `-7 % 2` даёт -1, `7 % -2` даёт 1. Ровно так же ведёт себя `%` у
    /// целых в Rust, поэтому мантиссы делятся напрямую.
    ///
    /// Дробные операнды допустимы: `7.5 % 2` даёт 1.5, `7 % 2.5` — 2
    /// (замеры `MOD.FRACTIONAL_LEFT`/`MOD.FRACTIONAL_RIGHT`).
    ///
    /// # Errors
    ///
    /// [`NumError::DivideByZero`], если делитель ноль (измерено: платформа
    /// тоже отказывает), либо [`NumError::ScaleOverflow`] при выходе
    /// общей шкалы за предел.
    pub fn rem(&self, other: &Self) -> Result<Self, NumError> {
        if other.is_zero() {
            return Err(NumError::DivideByZero);
        }
        let s = self.scale().max(other.scale());
        check_scale(s)?;

        if let (Repr::Small { m: am, scale: asc }, Repr::Small { m: bm, scale: bsc }) =
            (&self.0, &other.0)
            && let (Some(a), Some(b)) = (
                scale_up_i128(am.get(), s - asc),
                scale_up_i128(bm.get(), s - bsc),
            )
            // Единственная переполняющая пара — `i128::MIN % -1`: частное
            // не влезает в `i128`, хотя остаток равен нулю. `checked_rem`
            // отдаёт её общему пути, где деление идёт в большом ярусе.
            && let Some(r) = a.checked_rem(b)
        {
            return Ok(BslNumber::small(r, s));
        }

        let (a, a_scale) = self.big_parts();
        let (b, b_scale) = other.big_parts();
        let am = scale_up_big(&a, s - a_scale);
        let bm = scale_up_big(&b, s - b_scale);
        Ok(BslNumber::big(&*am % &*bm, s))
    }

    fn div_to_scale(&self, other: &Self, target: i32) -> Result<Self, NumError> {
        // value = a.m * 10^(target + b.scale - a.scale) / b.m, округлить half-up
        //
        // Проверяемая арифметика по той же причине, что в `mul`: слагаемые
        // приходят из масштабов операндов, а `check_scale` здесь до правки
        // не звался вовсе.
        let k = target
            .checked_add(other.scale())
            .and_then(|k| k.checked_sub(self.scale()))
            .ok_or(NumError::ScaleOverflow)?;

        if let (Repr::Small { m: am, .. }, Repr::Small { m: bm, .. }) = (&self.0, &other.0) {
            let (n, d) = if k >= 0 {
                (scale_up_i128(am.get(), k), Some(bm.get()))
            } else {
                (Some(am.get()), scale_up_i128(bm.get(), -k))
            };
            if let (Some(n), Some(d)) = (n, d)
                && let Some(q) = div_half_up_i128(n, d)
            {
                return Ok(BslNumber::small(q, target));
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
        if exp.is_integer()
            && let Some(e) = exp.to_i64_exact()
        {
            return self.pow_int(e);
        }
        crate::float::via_f64_2(self, exp, f64::powf)
    }

    /// Защитный предел показателя степени — по смыслу тот же, что
    /// `MAX_SCALE` у масштаба.
    ///
    /// Без него `Pow(2, 9223372036854775807)` не отвечает вовсе: точный
    /// результат — число на квинтиллионы разрядов. А `Pow(2, i64::MIN)`
    /// был ещё хуже: отрицание `i64::MIN` заворачивалось в него же,
    /// рекурсия не заканчивалась и процесс падал по переполнению стека.
    /// Единица и ноль в основании считаются дёшево при любом показателе и
    /// под предел не попадают.
    const MAX_ABS_EXPONENT: i64 = MAX_SCALE as i64;

    fn pow_int(&self, e: i64) -> Result<Self, NumError> {
        if e == 0 {
            return Ok(BslNumber::from_i64(1));
        }
        // Основание ±1 отвечает точно при ЛЮБОМ показателе, и считать тут
        // нечего. Этот случай обязан уйти раньше отрицания показателя:
        // иначе `checked_neg(i64::MIN)` срывался и точная единица
        // превращалась в ошибку.
        if self.abs() == BslNumber::from_i64(1) {
            let negative_base = self.is_negative();
            return Ok(if negative_base && e % 2 != 0 {
                BslNumber::from_i64(-1)
            } else {
                BslNumber::from_i64(1)
            });
        }
        // `unsigned_abs` не спотыкается об `i64::MIN`, в отличие от `-e`.
        if e.unsigned_abs() > Self::MAX_ABS_EXPONENT.unsigned_abs() && !self.is_zero() {
            return Err(NumError::ScaleOverflow);
        }
        if e < 0 {
            if self.is_zero() {
                return Err(NumError::DivideByZero);
            }
            // `checked_neg` вместо `-e`: у `i64::MIN` положительного
            // двойника нет, и без проверки здесь начиналась бесконечная
            // рекурсия.
            let positive = e.checked_neg().ok_or(NumError::ScaleOverflow)?;
            let p = self.pow_int(positive)?;
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
        match &self.0 {
            Repr::Small { m, scale } => {
                let v = scale_up_i128(m.get(), -*scale)?;
                i64::try_from(v).ok()
            }
            Repr::Big(b) => scale_up_big(&b.m, -b.scale).to_i64(),
        }
    }

    /// Оценка сверху числа десятичных разрядов мантиссы.
    ///
    /// Нужна, чтобы решить, не бессмысленно ли округление: если сдвиг
    /// больше, чем разрядов, ответ — ноль, и строить `10^сдвиг` не надо.
    /// Оценка грубая намеренно (`log10(2) < 1/3`), зато без перевода
    /// числа в строку.
    fn mantissa_digits_at_most(&self) -> u64 {
        match &self.0 {
            Repr::Small { .. } => 39,
            Repr::Big(b) => b.m.bits() / 3 + 1,
        }
    }

    /// Округление ЗНАЧЕНИЯ (не только для показа) к заданному масштабу,
    /// половина-вверх-от-нуля — та же схема, что и у деления. Используется
    /// форматированием (`ЧДЦ=N`) и `Окр`/`Round`. Если `scale` не меньше
    /// текущего, значение не меняется: лишние дробные разряды — забота
    /// форматирования (нулями), не самого числа.
    pub fn round_to_scale(&self, target_scale: i32) -> Self {
        let cur_scale = self.scale();
        if target_scale >= cur_scale {
            return self.clone();
        }
        // Разность считается насыщающе, а огромный сдвиг отсекается: без
        // этого общий путь строит `10^delta`, и `Окр(1.5, -2000000000)` не
        // отвечает вовсе — измерено, процесс снимался по таймауту.
        let delta = cur_scale.saturating_sub(target_scale);
        if u64::from(delta.unsigned_abs()) > self.mantissa_digits_at_most() + 1 {
            return BslNumber::from_i128(0);
        }

        if let Repr::Small { m, .. } = &self.0
            && delta <= 38
            && let Some(q) = div_half_up_i128(m.get(), POW10[delta as usize])
        {
            return BslNumber::small(q, target_scale);
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
        // Разность считается насыщающе, а огромный сдвиг отсекается: без
        // этого общий путь строит `10^delta`, и `Окр(1.5, -2000000000)` не
        // отвечает вовсе — измерено, процесс снимался по таймауту.
        let delta = cur_scale.saturating_sub(target_scale);
        if u64::from(delta.unsigned_abs()) > self.mantissa_digits_at_most() + 1 {
            return BslNumber::from_i128(0);
        }

        if let Repr::Small { m, .. } = &self.0
            && delta <= 38
            && let Some(q) = div_half_down_i128(m.get(), POW10[delta as usize])
        {
            return BslNumber::small(q, target_scale);
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
        // Разность считается насыщающе, а огромный сдвиг отсекается: без
        // этого общий путь строит `10^delta`, и `Окр(1.5, -2000000000)` не
        // отвечает вовсе — измерено, процесс снимался по таймауту.
        let delta = cur_scale.saturating_sub(target_scale);
        if u64::from(delta.unsigned_abs()) > self.mantissa_digits_at_most() + 1 {
            return BslNumber::from_i128(0);
        }

        if let Repr::Small { m, .. } = &self.0
            && delta <= 38
        {
            let q = m.get() / POW10[delta as usize];
            return BslNumber::small(q, target_scale);
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
        return BslNumber(Repr::Small {
            m: M128::new(0),
            scale: 0,
        });
    }
    while scale > 0 && i128_is_divisible_by_10(m) {
        m = exact_div_by_10(m);
        scale -= 1;
    }
    BslNumber(Repr::Small {
        m: M128::new(m),
        scale,
    })
}

/// Обратное к пяти по модулю 2^128. Считается один раз и на бумаге:
/// `5 * INV5 = 1 (mod 2^128)`.
const INV5: i128 = 0xcccc_cccc_cccc_cccc_cccc_cccc_cccc_cccdu128 as i128;

/// Деление на десять для значения, про которое УЖЕ известно, что оно на
/// десять делится (это проверяет `i128_is_divisible_by_10` строкой выше в
/// единственном вызывающем).
///
/// Никакого деления здесь нет. Десятка раскладывается на два и пять:
/// делить на два — арифметический сдвиг (у чётного значения он точен и
/// для отрицательных), а на пять — умножение на обратное по модулю
/// 2^128. Умножение в дополнительном коде — это умножение в кольце вычетов,
/// поэтому знак разбирать не нужно: если частное представимо, оно и
/// получится.
///
/// Зачем: `normalize_small` снимает хвостовые нули по одному, и КАЖДЫЙ шаг
/// раньше был `__divti3` из compiler-rt — программное 128-битное деление
/// в десятки тактов. Здесь сдвиг и одно умножение.
///
/// Работает только при делимости: на 11 эта формула даст мусор, а не
/// частное с остатком. Отсюда и `debug_assert`.
#[inline]
fn exact_div_by_10(m: i128) -> i128 {
    debug_assert!(
        i128_is_divisible_by_10(m),
        "точное деление применено к неделящемуся"
    );
    (m >> 1).wrapping_mul(INV5)
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

/// Попытка вернуться в быстрый ярус после операции на `BigInt`.
fn demote(b: BigDec) -> BslNumber {
    match b.m.to_i128() {
        Some(v) => BslNumber(Repr::Small {
            m: M128::new(v),
            scale: b.scale,
        }),
        None => BslNumber(Repr::Big(Rc::new(b))),
    }
}

/// Предел масштаба симметричен.
///
/// Ограничение снизу не запрещает отрицательный масштаб — он законен и
/// нужен, — а держит РАЗНОСТИ масштабов в пределах `i32`: при
/// `|scale| <= MAX_SCALE` любая разность не превосходит `2 * MAX_SCALE`.
/// Без нижней границы `from_parts(1, i32::MIN)` строил число, на котором
/// `to_canonical` паниковал при вычислении `-scale`.
/// Пропускает уже нормализованное число, если его масштаб в пределах.
///
/// Нужна там, где сырой масштаб операции может выйти за предел, а
/// нормализация результата — вернуть его обратно.
fn checked_result(value: BslNumber) -> Result<BslNumber, NumError> {
    check_scale(value.scale())?;
    Ok(value)
}

fn check_scale(s: i32) -> Result<(), NumError> {
    if !(-MAX_SCALE..=MAX_SCALE).contains(&s) {
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

/// Деление 128 на 64 аппаратной инструкцией `div`.
///
/// # Safety
///
/// `d != 0` и `hi < d`. Нарушение любого из двух — исключение #DE, то
/// есть немедленная смерть процесса, а не ошибка: `div` не насыщается.
/// Оба условия обеспечивает единственный вызывающий ниже.
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn divq(hi: u64, lo: u64, d: u64) -> (u64, u64) {
    let (quotient, remainder): (u64, u64);
    unsafe {
        std::arch::asm!(
            "div {d}",
            d = in(reg) d,
            inlateout("rax") lo => quotient,
            inlateout("rdx") hi => remainder,
            options(pure, nomem, nostack),
        );
    }
    (quotient, remainder)
}

/// Частное и остаток от деления 128-битного на 64-битное.
///
/// Зачем своё, когда есть `/` и `%`: у compiler-rt эта операция общая —
/// её `u128_div_rem` разбирает все случаи и занимает 73 инструкции с
/// ветвлениями вокруг пяти аппаратных `div`. Здесь случай известен
/// заранее (делитель влезает в 64 бита), и хватает одного или двух `div`
/// без разбора.
#[cfg(target_arch = "x86_64")]
#[inline]
fn div_rem_u128_by_u64(n: u128, d: u64) -> (u128, u64) {
    let hi = (n >> 64) as u64;
    let lo = n as u64;
    if hi < d {
        // Частное влезает в 64 бита — одна инструкция.
        let (q, r) = unsafe { divq(hi, lo, d) };
        (q as u128, r)
    } else {
        // Школьное деление в две цифры: сначала старшая половина, остаток
        // от неё заведомо меньше делителя, поэтому второй `div` безопасен.
        let (q_hi, r) = unsafe { divq(0, hi, d) };
        let (q_lo, r2) = unsafe { divq(r, lo, d) };
        (((q_hi as u128) << 64) | q_lo as u128, r2)
    }
}

/// Деление с округлением half-up — тем самым, что у платформы.
///
/// `None` означает «не поместилось», и вызывающий уходит на большой ярус.
fn div_half_up_i128(n: i128, d: i128) -> Option<i128> {
    if d == 0 {
        return None;
    }
    // `unsigned_abs`, а не `abs`: у `i128::MIN` модуль в знаковый тип не
    // помещается.
    let dd = d.unsigned_abs();
    let nn = n.unsigned_abs();
    let negative = (n < 0) != (d < 0);

    let (q, r) = {
        #[cfg(target_arch = "x86_64")]
        {
            match u64::try_from(dd) {
                Ok(d64) => {
                    let (q, r) = div_rem_u128_by_u64(nn, d64);
                    (q, r as u128)
                }
                Err(_) => (nn / dd, nn % dd),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            (nn / dd, nn % dd)
        }
    };

    // Half-up считается по МОДУЛЯМ и означает «от нуля», а не «вверх»:
    // -0.5 даёт -1, как на платформе.
    let magnitude = if r != 0 && r * 2 >= dd {
        q.checked_add(1)?
    } else {
        q
    };
    if negative {
        // -2^127 представимо, 2^127 — нет, поэтому через беззнаковое.
        if magnitude > (1u128 << 127) {
            return None;
        }
        Some((magnitude as i128).wrapping_neg())
    } else {
        i128::try_from(magnitude).ok()
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
        if let (Some((a, asc)), Some((b, bsc))) = (self.fast64_parts(), other.fast64_parts()) {
            let scale = asc.max(bsc);
            if let (Some(a), Some(b)) = (scale_up_i64(a, scale - asc), scale_up_i64(b, scale - bsc))
            {
                return a.cmp(&b);
            }
        }
        if let (Repr::Small { m: a, scale: asc }, Repr::Small { m: b, scale: bsc }) =
            (&self.0, &other.0)
        {
            if asc == bsc {
                return a.get().cmp(&b.get());
            }
            let s = (*asc).max(*bsc);
            if let (Some(x), Some(y)) = (
                scale_up_i128(a.get(), s - asc),
                scale_up_i128(b.get(), s - bsc),
            ) {
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
        match &self.0 {
            Repr::Small { m, scale } => {
                // Малое и большое представление одного значения не пересекаются:
                // demote гарантирует, что всё влезающее в i128 лежит в Small.
                m.get().hash(state);
                scale.hash(state);
            }
            Repr::Big(b) => {
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
    use super::Repr;
    use num_bigint::BigInt;
    use num_traits::Zero;

    use super::{BslNumber, bigint_is_divisible_by_10, i128_is_divisible_by_10};

    /// Знак остатка — по ДЕЛИМОМУ, дробные операнды допустимы. Всё
    /// перечисленное измерено на 8.3.27, замеры `MOD.*`.
    #[test]
    fn remainder_takes_the_sign_of_the_dividend() {
        let cases = [
            ("7", "2", "1"),
            ("-7", "2", "-1"),
            ("7", "-2", "1"),
            ("-7", "-2", "-1"),
            ("7.5", "2", "1.5"),
            ("7", "2.5", "2"),
            ("0", "5", "0"),
            // Точная арифметика: 10^30 не помещается в i128, и остаток
            // обязан считаться на больших числах без потери разрядов.
            ("1000000000000000000000000000000", "7", "1"),
        ];
        for (a, b, want) in cases {
            let a = BslNumber::parse_canonical(a).expect("делимое");
            let b = BslNumber::parse_canonical(b).expect("делитель");
            assert_eq!(a.rem(&b).expect("остаток").to_canonical(), want);
        }
    }

    /// Остаток от деления на ноль — ошибка, а не ноль и не паника
    /// (измерено: платформа тоже отказывает).
    #[test]
    fn remainder_by_zero_is_an_error() {
        let a = BslNumber::parse_canonical("7").unwrap();
        let z = BslNumber::parse_canonical("0").unwrap();
        assert!(a.rem(&z).is_err());
    }

    /// Тождество `a = (a / b) * b + (a % b)` при усечении частного к нулю.
    /// Проверяется перебором, а не тремя примерами: именно здесь вылезла бы
    /// ошибка выравнивания масштабов.
    #[test]
    fn remainder_agrees_with_truncated_division() {
        for a in -25i64..=25 {
            for b in -7i64..=7 {
                if b == 0 {
                    continue;
                }
                let an = BslNumber::from_i64(a);
                let bn = BslNumber::from_i64(b);
                let r = an.rem(&bn).expect("остаток");
                let expected = BslNumber::from_i64(a % b);
                assert_eq!(r.to_canonical(), expected.to_canonical(), "{a} % {b}");
            }
        }
    }

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
        assert!(matches!(promoted.0, Repr::Small { .. }));
        assert_eq!(promoted.to_canonical(), "9223372036854775808");

        let sum = BslNumber::from_parts(1, 1)
            .unwrap()
            .add(&BslNumber::from_parts(2, 1).unwrap())
            .unwrap();
        assert_eq!(sum.to_canonical(), "0.3");
    }

    #[test]
    fn in_place_addition_matches_normal_addition_and_normalizes_decimals() {
        for (left, right) in [
            ("41", "1"),
            ("0.1", "0.9"),
            ("170141183460469231731687303715884105727", "1"),
        ] {
            let mut actual = BslNumber::parse_canonical(left).unwrap();
            let right = BslNumber::parse_canonical(right).unwrap();
            let expected = actual.add(&right).unwrap();
            actual.add_assign(&right).unwrap();
            assert_eq!(actual.to_canonical(), expected.to_canonical());
        }
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

#[cfg(test)]
mod exact_division_tests {
    use super::*;

    /// Точное деление обязано совпадать с обычным ВЕЗДЕ, где применимо.
    /// Проверяется не образцами, а перебором: границы диапазона, обе
    /// стороны нуля, все степени десятки и псевдослучайные величины.
    #[test]
    fn exact_division_by_ten_matches_ordinary_division() {
        let mut checked = 0;
        let mut sample = |m: i128| {
            if m % 10 == 0 {
                assert_eq!(exact_div_by_10(m), m / 10, "не сошлось на {m}");
                checked += 1;
            }
        };
        for k in 0..38u32 {
            let p = 10i128.pow(k);
            sample(p);
            sample(-p);
            sample(p * 10);
        }
        // Линейный конгруэнтный генератор: воспроизводимо и без зависимостей.
        let mut x: u128 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..20000 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = (x >> 1) as i128;
            sample(v - v % 10);
            sample(-(v - v % 10));
        }
        // Наибольшее кратное десяти, влезающее в i128, и наименьшее.
        sample(i128::MAX - i128::MAX % 10);
        // `identity_op` clippy 1.97 ошибочно считает `i128::MIN % 10`
        // равным `i128::MIN` (на самом деле −8; соседняя строка с `MAX`
        // линтер не смущает) — подсказке следовать нельзя.
        #[allow(clippy::identity_op)]
        sample(i128::MIN - i128::MIN % 10);
        assert!(checked > 20000, "перебор не набрал значений: {checked}");
    }
}

#[cfg(test)]
mod division_tests {
    use super::*;

    /// Эталон — прежняя реализация, слово в слово: частное и остаток
    /// обычными операторами, округление от нуля при |r| * 2 >= |d|.
    /// Новая версия отличается только СПОСОБОМ получить частное с
    /// остатком (аппаратный `div` вместо разбора случаев в compiler-rt),
    /// и обязана совпадать с ней всюду.
    fn reference(n: i128, d: i128) -> Option<i128> {
        let q = n.checked_div(d)?;
        let r = n % d;
        if r == 0 {
            return Some(q);
        }
        let rr = r.unsigned_abs();
        let dd = d.unsigned_abs();
        let bump = rr.checked_mul(2).map(|x| x >= dd).unwrap_or(true);
        if !bump {
            return Some(q);
        }
        if (n < 0) != (d < 0) {
            q.checked_sub(1)
        } else {
            q.checked_add(1)
        }
    }

    #[test]
    fn hardware_division_matches_the_previous_implementation() {
        let mut cases = 0;
        let mut check = |n: i128, d: i128| {
            assert_eq!(
                div_half_up_i128(n, d),
                reference(n, d),
                "разошлись на {n} / {d}"
            );
            cases += 1;
        };

        // Границы, знаки, нули, деление на единицу и само на себя.
        for &n in &[0i128, 1, -1, 5, -5, i128::MAX, i128::MIN, i128::MIN + 1] {
            for &d in &[1i128, -1, 2, -2, 3, 10, -10, i128::MAX, i128::MIN] {
                check(n, d);
            }
        }
        // Ровно на границе half-up: остаток строго в половину делителя.
        for &d in &[2i128, 4, 10, 100, 1 << 40] {
            for &sn in &[1i128, -1] {
                for k in 0..5i128 {
                    check(sn * (k * d + d / 2), d);
                    check(sn * (k * d + d / 2 - 1), d);
                    check(sn * (k * d + d / 2 + 1), d);
                }
            }
        }
        // Делитель В 64 бита и ЗА 64 бита — это разные ветви новой функции.
        let mut x: u128 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..20000 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let n = (x as i128).wrapping_mul(3);
            let small = ((x >> 64) as u64 | 1) as i128;
            check(n, small);
            check(n, -small);
            let big = (x | (1 << 100)) as i128;
            if big != 0 {
                check(n, big);
            }
        }
        assert!(cases > 50000, "перебор не набрал случаев: {cases}");
    }
}
