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
