//! Края `i128` и `i64`, на которых арифметика паниковала или врала.
//!
//! Все три дефекта достижимы однострочным BSL-скриптом и жили в дереве до
//! работы над масштабом — найдены независимым ревью:
//!
//! * `neg` отрицал мантиссу обычным минусом, поэтому `-(-i128::MIN)`
//!   паниковал в debug и в release возвращал ИСХОДНОЕ отрицательное число;
//! * быстрый путь `rem` считал `a % b`, и `i128::MIN % -1` паниковал в
//!   обоих профилях;
//! * `pow_int` отрицал показатель, поэтому `Pow(2, i64::MIN)` паниковал в
//!   debug, а в release заворачивался обратно и уходил в бесконечную
//!   рекурсию до переполнения стека. Большой ПОЛОЖИТЕЛЬНЫЙ показатель той
//!   же функции просто не отвечал.

use bsl_number::{BslNumber, NumError};

/// `i128::MIN` как число: мантисса на нижнем краю малого яруса.
fn i128_min() -> BslNumber {
    BslNumber::from_i128(i128::MIN)
}

#[test]
fn negating_the_smallest_mantissa_gives_the_positive_value() {
    let n = i128_min();
    assert_eq!(n.to_canonical(), i128::MIN.to_string());

    let positive = n.neg();
    // Ответ не влезает в `i128` и обязан переехать в большой ярус, а не
    // остаться отрицательным.
    assert_eq!(
        positive.to_canonical(),
        "170141183460469231731687303715884105728"
    );
    assert_eq!(positive.neg().to_canonical(), i128::MIN.to_string());
}

#[test]
fn the_absolute_value_of_the_smallest_mantissa_is_positive() {
    assert_eq!(
        i128_min().abs().to_canonical(),
        "170141183460469231731687303715884105728"
    );
}

#[test]
fn the_remainder_of_the_smallest_mantissa_by_minus_one_is_zero() {
    let r = i128_min()
        .rem(&BslNumber::from_i64(-1))
        .expect("остаток определён");
    assert_eq!(r.to_canonical(), "0");
}

#[test]
fn an_extreme_exponent_is_a_clean_error_rather_than_a_panic_or_a_hang() {
    let two = BslNumber::from_i64(2);
    for exponent in [i64::MIN, i64::MAX, -i64::MAX] {
        let result = two.pow(&BslNumber::from_i64(exponent));
        assert!(
            matches!(result, Err(NumError::ScaleOverflow)),
            "показатель {exponent} должен давать ошибку, получено {result:?}"
        );
    }
}

#[test]
fn a_unit_base_is_exact_at_every_exponent_including_the_smallest() {
    // Предел показателя ±1 не касается — считать тут нечего. Но отрицание
    // `i64::MIN` всё равно срывалось на `checked_neg` и превращало точный
    // ответ в ошибку. `i64::MIN` чётен, поэтому `(-1)^i64::MIN` — единица.
    let one = BslNumber::from_i64(1);
    let minus_one = BslNumber::from_i64(-1);
    for exponent in [i64::MIN, i64::MAX, -i64::MAX, -3, 3, 0] {
        let e = BslNumber::from_i64(exponent);
        assert_eq!(
            one.pow(&e).expect("единица в любой степени").to_canonical(),
            "1",
            "1^{exponent}"
        );
        let expected = if exponent % 2 == 0 { "1" } else { "-1" };
        assert_eq!(
            minus_one
                .pow(&e)
                .expect("минус единица в любой степени")
                .to_canonical(),
            expected,
            "(-1)^{exponent}"
        );
    }
}

#[test]
fn ordinary_exponents_keep_working() {
    let two = BslNumber::from_i64(2);
    assert_eq!(
        two.pow(&BslNumber::from_i64(10)).unwrap().to_canonical(),
        "1024"
    );
    assert_eq!(
        two.pow(&BslNumber::from_i64(-2)).unwrap().to_canonical(),
        "0.25"
    );
    assert_eq!(
        BslNumber::from_i64(1)
            .pow(&BslNumber::from_i64(i64::MAX))
            .unwrap()
            .to_canonical(),
        "1",
        "единица в любой степени — единица, и считать её дёшево"
    );
}
