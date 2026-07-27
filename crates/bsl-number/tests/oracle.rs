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
fn pow_integer_exponent_is_exact() {
    // Через f64 вышло бы 1000000000000000019884624838656.
    assert_eq!(c(&n("10").pow(&n("30")).unwrap()), "1000000000000000000000000000000");
}

#[test]
fn sqrt_goes_through_f64_15_significant() {
    assert_eq!(c(&n("2").sqrt().unwrap()), "1.4142135623731");
}

/// ОТКРЫТО. Платформа даёт 0,141421356237309, наша модель "15 значащих с
/// округлением" даёт 0.14142135623731. У sqrt(2) и sqrt(0.02) одинаковая
/// последовательность цифр, и оба f64 округляются ВВЕРХ на 16-м разряде:
///   f64(sqrt 2)    = 1.41421356237309514547
///   f64(sqrt 0.02) = 0.14142135623730950345
/// Ни 15 значащих, ни 13 знаков после точки, ни усечение не описывают обе
/// точки. Нужны доизмерения — см. OPEN QUESTIONS в промте.
#[test]
#[ignore]
fn sqrt_small_argument_unresolved() {
    assert_eq!(c(&n("0.02").sqrt().unwrap()), "0.141421356237309");
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
    // `round_to_scale`/`Округл`: 2.9 -> 2, а не 3, и симметрично для
    // отрицательных: -2.9 -> -2, а не -3.
    assert_eq!(c(&n("2.9").trunc_to_scale(0)), "2");
    assert_eq!(c(&n("-2.9").trunc_to_scale(0)), "-2");
    assert_eq!(c(&n("2.675").trunc_to_scale(2)), "2.67");
    assert_eq!(c(&n("123").trunc_to_scale(0)), "123");
}

#[test]
fn round_to_scale_half_even_differs_from_half_up_only_on_exact_ties() {
    // НЕ ИЗМЕРЕНО на платформе — этот режим существует только под явно
    // запрошенный третий аргумент `Округл` (см. `bsl_number::RoundMode`);
    // тест фиксирует саму схему, не то, что 1С её так называет.
    assert_eq!(c(&n("2.5").round_to_scale_half_even(0)), "2");
    assert_eq!(c(&n("3.5").round_to_scale_half_even(0)), "4");
    assert_eq!(c(&n("-2.5").round_to_scale_half_even(0)), "-2");
    assert_eq!(c(&n("-3.5").round_to_scale_half_even(0)), "-4");
    assert_eq!(c(&n("0.5").round_to_scale_half_even(0)), "0");
    assert_eq!(c(&n("1.5").round_to_scale_half_even(0)), "2");

    // Мимо ничьей обе схемы совпадают.
    for s in ["2.4", "2.6", "-2.4", "-2.6", "2.675", "123"] {
        assert_eq!(
            c(&n(s).round_to_scale_half_even(2)),
            c(&n(s).round_to_scale(2)),
            "не на ничьей half-even обязан совпасть с half-up: {s}"
        );
    }

    // И на большом ярусе (BigInt) тоже: 2.5 с мантиссой за пределами i128.
    let big = n("25000000000000000000000000000000000000000.5");
    assert_eq!(
        c(&big.round_to_scale_half_even(0)),
        "25000000000000000000000000000000000000000"
    );
    assert_eq!(
        c(&big.round_to_scale(0)),
        "25000000000000000000000000000000000000001"
    );
}
