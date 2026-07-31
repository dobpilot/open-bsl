use bsl_number::BslNumber;

fn n(s: &str) -> BslNumber {
    BslNumber::parse_canonical(s).unwrap()
}
fn c(x: &BslNumber) -> String {
    x.to_canonical()
}

#[test]
fn division_27_digits() {
    assert_eq!(c(&n("1").div(&n("3")).unwrap()), "0.333333333333333333333333333");
    assert_eq!(c(&n("2").div(&n("3")).unwrap()), "0.666666666666666666666666667");
    assert_eq!(c(&n("10").div(&n("3")).unwrap()), "3.333333333333333333333333333");
}

#[test]
fn division_half_up_on_exact_tie() {
    // 1/2^28 = 0.0000000037252902984619140625 ровно, 28 знаков, пятёрка в хвосте.
    // Платформа дала ...063 -> округление от нуля. Half-even дал бы ...062.
    assert_eq!(
        c(&n("1").div(&n("268435456")).unwrap()),
        "0.000000003725290298461914063"
    );
}

#[test]
fn multiplication_is_exact() {
    assert_eq!(c(&n("0.1").add(&n("0.2")).unwrap()), "0.3");
    assert_eq!(c(&n("1.10").mul(&n("1.00")).unwrap()), "1.1");
    // Перенос множителей 2 и 5 между операндами всё равно обязан срезать
    // ноль произведения; взаимно простая с 10 мантисса нуля не создаёт.
    assert_eq!(c(&n("0.2").mul(&n("0.5")).unwrap()), "0.1");
    assert_eq!(c(&n("0.3").mul(&n("0.5")).unwrap()), "0.15");
    let big = n("123456789123456789").mul(&n("987654321987654321")).unwrap();
    assert_eq!(c(&big), "121932631356500531347203169112635269");
}

#[test]
fn numeric_for_step_has_integer_fast_path_and_decimal_fallback() {
    let mut integer = n("10");
    assert!(integer.increment_and_le(&n("11")).unwrap());
    assert_eq!(c(&integer), "11");
    assert!(!integer.increment_and_le(&n("11")).unwrap());
    assert_eq!(c(&integer), "12");

    let mut decimal = n("0.5");
    assert!(decimal.increment_and_le(&n("2.5")).unwrap());
    assert_eq!(c(&decimal), "1.5");
}

#[test]
fn pow_integer_exponent_is_exact() {
    // Через f64 вышло бы 1000000000000000019884624838656.
    assert_eq!(c(&n("10").pow(&n("30")).unwrap()), "1000000000000000000000000000000");
}

#[test]
fn sqrt_goes_through_f64_15_significant() {
    assert_eq!(c(&n("2").sqrt().unwrap()), "1.4142135623731");
}

/// ЗАМЕР 8.3.27, четырнадцать точек. Модель «15 значащих, half-up от
/// значения f64» воспроизводит ТРИНАДЦАТЬ из них побайтово — включая
/// разные магнитуды, `Exp`, `Log` и `Sin`. Проверены и отвергнуты:
/// half-down от f64, half-up от кратчайшей записи f64, усечение — каждая
/// из них расходится не меньше чем на той же точке, а некоторые хуже.
#[test]
fn f64_return_matches_the_platform_on_every_measured_point() {
    // Sqrt по магнитудам.
    assert_eq!(c(&n("3").sqrt().unwrap()), "1.73205080756888");
    assert_eq!(c(&n("5").sqrt().unwrap()), "2.23606797749979");
    assert_eq!(c(&n("7").sqrt().unwrap()), "2.64575131106459");
    assert_eq!(c(&n("0.5").sqrt().unwrap()), "0.707106781186548");
    assert_eq!(c(&n("0.3").sqrt().unwrap()), "0.547722557505166");
    assert_eq!(c(&n("1000000.7").sqrt().unwrap()), "1000.00034999994");
    assert_eq!(c(&n("0.0000007").sqrt().unwrap()), "0.000836660026534076");
    assert_eq!(c(&n("2").sqrt().unwrap()), "1.4142135623731");
    assert_eq!(c(&n("200").sqrt().unwrap()), "14.142135623731");
    assert_eq!(c(&n("0.0002").sqrt().unwrap()), "0.014142135623731");
    // Не только Sqrt: тот же возврат из f64 у остальных.
    assert_eq!(c(&n("1").exp().unwrap()), "2.71828182845905");
    assert_eq!(c(&n("2").ln().unwrap()), "0.693147180559945");
    assert_eq!(c(&n("1").sin().unwrap()), "0.841470984807897");
}

/// `НЕ ИЗМЕРЕНО(SQRT.SMALL_ARG)` — точнее, измерено, но не объяснено.
/// ЧЕТЫРНАДЦАТАЯ ТОЧКА расходится: платформа даёт
/// `0.141421356237309`, мы `0.14142135623731`.
///
/// Причина не в схеме округления, а в самом квадратном корне. Точный
/// double от IEEE-sqrt(0.02) равен
/// `0.1414213562373095034452...` — шестнадцатая значащая цифра `5` со
/// «хвостом» `03445`, то есть строго больше половины, и ЛЮБАЯ схема
/// округления к 15 значащим даёт `...310`. Чтобы получить `...309`,
/// платформа должна была начать с ДРУГОГО double — на один ulp меньше.
///
/// То есть 1С считает корень не тем же способом, что IEEE `sqrt`, и
/// расходится с ним в последнем разряде на редких входах: на остальных
/// тринадцати измеренных точках расхождения нет. Воспроизводить это
/// означало бы эмулировать их арифметику вслепую, поэтому ЗДЕСЬ ОСТАВЛЕНО
/// РАСХОЖДЕНИЕ — задокументированное, а не забытое.
#[test]
fn sqrt_of_002_differs_from_the_platform_by_one_ulp() {
    assert_eq!(c(&n("0.02").sqrt().unwrap()), "0.14142135623731");
    // Платформа: 0.141421356237309 (см. SQRT.SMALL_ARG в реестре).
}

#[test]
fn equality_ignores_representation() {
    assert_eq!(n("1.0"), n("1.00"));
    assert_eq!(n("1.0"), n("1"));
}

#[test]
fn scale_grows_without_bound() {
    // Модель n-body: масштаб растёт линейно, границы нет.
    let mut x = n("1.0000000001");
    for _ in 0..100 {
        x = x.mul(&n("1.0000000001")).unwrap();
    }
    assert!(x.scale() > 500, "scale = {}", x.scale());
}

#[test]
fn size_is_24_bytes() {
    assert_eq!(std::mem::size_of::<BslNumber>(), 24);
}

#[test]
fn round_to_scale_is_half_up() {
    // 2.675 -> 2.68 при округлении до 2 знаков половина-вверх (а не 2.67,
    // как дало бы округление через f64 из-за двоичного приближения — брифом
    // явно указано, что Round/Int обязаны остаться в decimal).
    assert_eq!(c(&n("2.675").round_to_scale(2)), "2.68");
    assert_eq!(c(&n("1.005").round_to_scale(2)), "1.01");
    // Масштаб не меньше текущего — значение не меняется.
    assert_eq!(c(&n("1.5").round_to_scale(5)), "1.5");
    assert_eq!(c(&n("123").round_to_scale(0)), "123");
    assert_eq!(c(&n("-2.675").round_to_scale(2)), "-2.68");
}

#[test]
fn trunc_to_scale_is_toward_zero_not_half_up() {
    // `Цел` отбрасывает дробную часть в СТОРОНУ НУЛЯ — не half-up, как
    // `round_to_scale`/`Окр`: 2.9 -> 2, а не 3, и симметрично для
    // отрицательных: -2.9 -> -2, а не -3.
    assert_eq!(c(&n("2.9").trunc_to_scale(0)), "2");
    assert_eq!(c(&n("-2.9").trunc_to_scale(0)), "-2");
    assert_eq!(c(&n("2.675").trunc_to_scale(2)), "2.67");
    assert_eq!(c(&n("123").trunc_to_scale(0)), "123");
}

/// ИЗМЕРЕНО на платформе 8.3.27 через `Окр(х, 0, 0)`: половина уходит К
/// НУЛЮ, а не к чётному. Решающая точка — 3,5: к чётному дало бы 4.
#[test]
fn round_to_scale_half_down_keeps_the_tie_closer_to_zero() {
    assert_eq!(c(&n("2.5").round_to_scale_half_down(0)), "2");
    assert_eq!(c(&n("3.5").round_to_scale_half_down(0)), "3");
    assert_eq!(c(&n("1.5").round_to_scale_half_down(0)), "1");
    assert_eq!(c(&n("0.5").round_to_scale_half_down(0)), "0");
    assert_eq!(c(&n("-2.5").round_to_scale_half_down(0)), "-2");
    assert_eq!(c(&n("-3.5").round_to_scale_half_down(0)), "-3");

    // Мимо ничьей обе схемы совпадают — тоже измерено:
    // Окр(2.4,0,0) = 2, Окр(2.6,0,0) = 3.
    for s in ["2.4", "2.6", "-2.4", "-2.6", "123"] {
        assert_eq!(
            c(&n(s).round_to_scale_half_down(0)),
            c(&n(s).round_to_scale(0)),
            "не на ничьей режимы обязаны совпасть: {s}"
        );
    }

    // Ничья не только на нуле разрядов: Окр(2.675, 2, 0) = 2,67 против
    // 2,68 у half-up — тоже с платформы.
    assert_eq!(c(&n("2.675").round_to_scale_half_down(2)), "2.67");
    assert_eq!(c(&n("2.675").round_to_scale(2)), "2.68");

    // И на большом ярусе (мантисса за пределами i128).
    let big = n("25000000000000000000000000000000000000000.5");
    assert_eq!(
        c(&big.round_to_scale_half_down(0)),
        "25000000000000000000000000000000000000000"
    );
    assert_eq!(
        c(&big.round_to_scale(0)),
        "25000000000000000000000000000000000000001"
    );
}

/// Нормализация не опускает масштаб ниже нуля, поэтому ЦЕЛОЕ число вполне
/// может иметь мантиссу, кратную 10 (`100` — это мантисса 100). Значит
/// «одна мантисса взаимно проста с 10» НЕ доказывает, что произведение не
/// кратно 10, и пропускать нормализацию на этом основании нельзя.
///
/// Тест бьёт именно по большому пути умножения: малый путь нормализует сам,
/// поэтому операнд взят такой, что `i128`-умножение переполняется.
#[test]
fn mul_normalizes_when_one_operand_goes_big() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let ten_pow_38 = n(&format!("1{}", "0".repeat(38)));
    let product = ten_pow_38.mul(&n("0.3")).unwrap();
    let expected = n(&format!("3{}", "0".repeat(37)));

    assert_eq!(c(&product), format!("3{}", "0".repeat(37)));

    // Главное следствие ненормализованного представления: от него зависят
    // равенство и хеш, а на них держится `Соответствие` с числовыми ключами.
    let hash = |x: &bsl_number::BslNumber| {
        let mut h = DefaultHasher::new();
        x.hash(&mut h);
        h.finish()
    };
    assert_eq!(product, expected, "равенство зависит от представления");
    assert_eq!(hash(&product), hash(&expected), "хеш зависит от представления");
}
