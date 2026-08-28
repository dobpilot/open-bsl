use crate::number::Repr;
use num_bigint::BigInt;
use num_traits::Signed;

use crate::NumError;
use crate::number::BslNumber;

/// Верхняя оценка числа десятичных цифр мантиссы по её длине в БИТАХ,
/// считаемая в аккумуляторе шириной `ACC_BITS`.
///
/// log10(2) ≈ 0.30103: столько десятичных цифр приходится на бит, плюс
/// единица за округление вверх. Ширина аккумулятора — ПАРАМЕТР, а не
/// свойство цели: производственный путь всегда зовёт эту функцию со
/// 128 битами, поэтому расчёт одинаков на любой платформе, а тест может
/// подать 32 и увидеть, что произошло бы в `usize`-арифметике на
/// `wasm32` — там произведение «биты × 30103» не помещается уже при
/// 142 675 битах, и оценка перестала бы быть верхней ровно там, где она
/// нужна, на больших мантиссах.
fn decimal_digits_from_bits_in(bits: u64, acc_bits: u32) -> u128 {
    let ceiling = if acc_bits >= 128 {
        u128::MAX
    } else {
        (1u128 << acc_bits) - 1
    };
    // Умножение в узком аккумуляторе насыщается — ровно это и делало
    // прежнюю оценку заниженной на 32-битной цели.
    let product = u128::from(bits).saturating_mul(30_103).min(ceiling);
    product / 100_000 + 1
}

/// Оценка цифр по битам на производственном пути: аккумулятор ВСЕГДА
/// 128-битный, независимо от разрядности цели.
fn decimal_digits_from_bits(bits: u64) -> u128 {
    decimal_digits_from_bits_in(bits, u128::BITS)
}

/// Число десятичных цифр в записи `value` (у нуля — одна).
fn decimal_digits(value: u128) -> usize {
    if value == 0 {
        return 1;
    }
    value.ilog10() as usize + 1
}

impl BslNumber {
    /// Каноническая форма: точка как разделитель, без группировки, без
    /// экспоненты. Соответствует `Формат(x, "ЧГ=0; ЧРД=.")` в 1С и
    /// разбирается обратно без потерь — на этом строится дифф-харнесс.
    /// Длина канонической записи В БАЙТАХ, посчитанная без материализации
    /// строки. Для `Small` — ТОЧНАЯ (число десятичных цифр `i128`
    /// считается арифметикой), для `Big` — верхняя оценка: печатать
    /// мантиссу произвольной величины ради длины значило бы выделить
    /// ровно то, что бюджет собирается отвергнуть. Формула повторяет
    /// ветви [`BslNumber::to_canonical`] и обязана оставаться с ними в
    /// согласии — это закреплено тестом `canonical_len_bound_is_exact`.
    #[must_use]
    pub fn canonical_len_bound(&self) -> usize {
        let (digits, neg, scale) = match &self.0 {
            Repr::Small { m, scale } => {
                let v = m.get();
                (decimal_digits(v.unsigned_abs()) as u128, v < 0, *scale)
            }
            // log10(2) ≈ 0.30103: столько десятичных цифр приходится на
            // бит мантиссы, плюс единица за округление вверх. Счёт идёт
            // в `u128`: на 32-битной цели произведение `биты × 30103`
            // насыщало бы `usize` уже на 143 тысячах бит, и оценка
            // переставала бы быть верхней ровно там, где она нужна.
            Repr::Big(b) => (
                decimal_digits_from_bits(b.m.bits()),
                b.m.is_negative(),
                b.scale,
            ),
        };
        let sign = u128::from(neg);
        let scale_abs = u128::from(scale.unsigned_abs());
        let total = if scale <= 0 {
            sign + digits + scale_abs
        } else if digits > scale_abs {
            // «цифры до точки» + точка + «цифры после».
            sign + digits + 1
        } else {
            // «0.» + ведущие нули + цифры.
            sign + 2 + scale_abs
        };
        // Значение, не представимое в `usize`, заведомо не помещается ни
        // в какой бюджет: насыщение здесь ведёт к честному отказу по
        // месту, а не к заниженной оценке.
        usize::try_from(total).unwrap_or(usize::MAX)
    }

    /// ЭВРИСТИЧЕСКАЯ оценка пиковой памяти, которую занимает печать
    /// числа, включая внутренние буферы перевода мантиссы в десятичную
    /// запись.
    ///
    /// Статус важен: для `Small` величина точна (печать идёт из `u128` и
    /// стоит доли сотни байт), а для `Big` это НЕ доказанная верхняя
    /// граница, а измеренная закономерность с запасом. Перевод выполняет
    /// `num-bigint` рекурсивным делением, его внутренние буферы наружу
    /// не описаны, и вывести границу из алгоритма нельзя — она снята
    /// счётчиком живой памяти на машине разработки: отношение пика к
    /// длине результата 1,9 на 64 цифрах, 4,9 на тысяче и 5,4–5,5 на
    /// 4 К … 256 К цифр, то есть выходит на полку. Множитель 12 берётся
    /// с двукратным запасом к этой полке, слагаемое покрывает мелкие
    /// величины, где полка ещё не достигнута.
    ///
    /// Отсюда следствие для вызывающих: бюджет памяти, опирающийся на
    /// эту величину, строгой гарантией для `Big`-чисел НЕ является.
    /// Сторожем служит тест `the_charged_print_peak_covers_the_real_one`
    /// в `bsl-rt`: он сверяет списанное с фактическим пиком и упадёт,
    /// если поведение библиотеки изменится. Строгая граница потребовала
    /// бы собственного форматтера с формально ограниченной рабочей
    /// памятью — отдельное решение, не принятое.
    ///
    /// Оценка линейна СОЗНАТЕЛЬНО: квадратичная формула переполнялась бы
    /// на 32-битной цели (`wasm32`) уже на 65 536 цифрах и там же
    /// переставала бы расти.
    ///
    /// Нужна тем, кто обязан списать память ДО печати: иначе задание
    /// занимает кратно больше оплаченного.
    #[must_use]
    pub fn canonical_print_peak_bound(&self) -> usize {
        let len = self.canonical_len_bound();
        match &self.0 {
            Repr::Small { .. } => len,
            Repr::Big(_) => len.saturating_mul(12).saturating_add(8 << 10),
        }
    }

    pub fn to_canonical(&self) -> String {
        use std::fmt::Write as _;

        // Цифры пишутся СРАЗУ в результат: промежуточная строка цифр
        // держала бы вторую копию числа, а у `Big` мантисса произвольной
        // величины — на ней пик памяти удваивался бы против учтённого.
        // Ёмкость берётся из той же оценки, которой пользуется бюджет.
        let mut out = String::with_capacity(self.canonical_len_bound());
        let (neg, scale) = match &self.0 {
            Repr::Small { m, scale } => (m.get() < 0, *scale),
            Repr::Big(b) => (b.m.is_negative(), b.scale),
        };
        if neg {
            out.push('-');
        }
        let digits_at = out.len();
        match &self.0 {
            Repr::Small { m, .. } => {
                let _ = write!(out, "{}", m.get().unsigned_abs());
            }
            Repr::Big(b) => {
                let _ = write!(out, "{}", b.m.magnitude());
            }
        }
        let digits_len = out.len() - digits_at;

        if scale <= 0 {
            for _ in 0..(-scale) {
                out.push('0');
            }
            return out;
        }

        let scale = scale as usize;
        if digits_len > scale {
            // Точка встаёт на своё место сдвигом внутри уже выделенного
            // буфера — второй аллокации не нужно.
            out.insert(digits_at + digits_len - scale, '.');
        } else {
            // Префикс собирается ЦЕЛИКОМ и вставляется одним сдвигом:
            // посимвольная вставка двигала бы весь хвост на каждом нуле,
            // а масштаб доходит до `MAX_SCALE` — это миллиарды
            // перемещённых байтов на ровном месте.
            let padding = scale - digits_len;
            let mut prefix = String::with_capacity(2 + padding);
            prefix.push_str("0.");
            for _ in 0..padding {
                prefix.push('0');
            }
            out.insert_str(digits_at, &prefix);
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

#[cfg(test)]
mod length_tests {
    use crate::number::BslNumber;

    /// Расчёт длины без материализации совпадает с фактической записью
    /// для `Small` и не занижает её для `Big`: на этом равенстве держится
    /// списание бюджета памяти ДО печати.
    /// Оценка цифр по битам верна и ЗА 32-битной границей — и это
    /// проверяется НА ТОМ ЖЕ коде, которым считает производственный
    /// путь: ширина аккумулятора у него параметр, поэтому тест подаёт 32
    /// и видит ровно то, что произошло бы на `wasm32`.
    ///
    /// Ограничение честно: воспроизвести дефект «расчёт идёт в `usize`»
    /// на 64-битном хосте нельзя, а исполняемого 32-битного target'а в
    /// репозитории нет (`cargo check --target wasm32` только собирает).
    /// Поэтому тест сторожит ФОРМУЛУ: узкий аккумулятор обязан занижать,
    /// широкий — совпадать с точной арифметикой, а производственный путь
    /// — с широким.
    #[test]
    fn digits_from_bits_survives_the_32_bit_boundary() {
        use super::{decimal_digits_from_bits, decimal_digits_from_bits_in};

        // Порог, за которым 32-битное умножение насыщается.
        let boundary = u64::from(u32::MAX) / 30_103;
        for bits in [1, 1_000, boundary - 1, boundary] {
            assert_eq!(
                decimal_digits_from_bits_in(bits, 32),
                decimal_digits_from_bits(bits),
                "до границы обе разрядности обязаны совпадать ({bits} бит)"
            );
        }
        // Сразу за порогом насыщение съедает меньше единицы деления, и
        // расхождение появляется только дальше — проверяем там, где оно
        // заведомо есть.
        for bits in [boundary * 2, boundary * 8, 1 << 40] {
            let wide = decimal_digits_from_bits(bits);
            let narrow = decimal_digits_from_bits_in(bits, 32);
            assert_eq!(
                wide,
                u128::from(bits) * 30_103 / 100_000 + 1,
                "широкий расчёт разошёлся с точным на {bits} битах"
            );
            assert!(
                narrow < wide,
                "узкий аккумулятор обязан занижать за границей ({bits} бит): \
                 {narrow} против {wide}"
            );
            // И именно поэтому производственный путь обязан быть широким.
            assert!(
                wide as f64 >= bits as f64 * std::f64::consts::LOG10_2,
                "оценка занижена на {bits} битах"
            );
        }
    }

    /// Печать числа с масштабом у предела даёт верный результат.
    ///
    /// Утверждения о времени здесь СОЗНАТЕЛЬНО нет: порог по настенным
    /// часам зависит от машины и нагрузки, а репозиторий требует для
    /// любых утверждений о скорости чередующегося A/B — тесту это не
    /// место. Защита от квадратичности структурная и живёт в самом коде:
    /// префикс дробной части собирается целиком и вставляется ОДНИМ
    /// сдвигом (см. `to_canonical`), а не по нулю за раз.
    #[test]
    fn a_huge_scale_prints_correctly() {
        let mut text = String::from("0.");
        text.push_str(&"0".repeat(90_000));
        text.push_str(&"7".repeat(9_000));
        let value = BslNumber::parse_canonical(&text).expect("число с большим масштабом");
        let printed = value.to_canonical();
        assert_eq!(printed, text, "печать при большом масштабе исказила запись");
        assert!(
            value.canonical_len_bound() >= printed.len(),
            "оценка длины занижена на большом масштабе"
        );
    }

    #[test]
    fn canonical_len_bound_is_exact() {
        let mut cases: Vec<BslNumber> = vec![
            BslNumber::ZERO,
            BslNumber::from_i64(1),
            BslNumber::from_i64(-1),
            BslNumber::from_i64(i64::MAX),
            BslNumber::from_i64(i64::MIN),
        ];
        for text in [
            "0.1",
            "-0.1",
            "0.0000001",
            "-0.0000001",
            "123.456",
            "-123.456",
            "1000",
            "-1000",
            "0.000000000000000000000000001",
            "123456789012345678901234567890123456789012345678901234567890.123456789",
        ] {
            cases.push(BslNumber::parse_canonical(text).expect("разбор числа"));
        }
        for value in cases {
            let printed = value.to_canonical().len();
            let bound = value.canonical_len_bound();
            assert!(
                bound >= printed,
                "оценка занижена для {}: {bound} < {printed}",
                value.to_canonical()
            );
            // Для представимых `i128` расчёт обязан быть точным.
            if printed <= 40 {
                assert_eq!(
                    bound,
                    printed,
                    "расчёт не точен для {}",
                    value.to_canonical()
                );
            }
        }
    }
}
