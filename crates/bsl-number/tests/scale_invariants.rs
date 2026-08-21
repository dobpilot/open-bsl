//! Масштаб не выходит за собственные пределы ни через конструктор, ни через
//! арифметику.
//!
//! До этих проверок `from_parts` принимал любой `i32`, а `mul` складывал
//! масштабы обычным `+` ДО `check_scale`. На числе с масштабом `i32::MAX`
//! это давало панику «attempt to add with overflow» в debug и молчаливый
//! `Ok(1)` в release — то есть неверный ответ.

use bsl_number::{BslNumber, MAX_SCALE, NumError};

#[test]
fn the_constructor_accepts_the_limit_and_refuses_the_next_value() {
    assert!(BslNumber::from_parts(1, MAX_SCALE).is_ok());
    assert!(matches!(
        BslNumber::from_parts(1, MAX_SCALE + 1),
        Err(NumError::ScaleOverflow)
    ));
    assert!(matches!(
        BslNumber::from_parts(1, i32::MAX),
        Err(NumError::ScaleOverflow)
    ));
}

#[test]
fn a_negative_scale_stays_legal() {
    // Отрицательный масштаб — часть модели: `Формат` строит им степени
    // десятки. Запрещать его без основания нельзя.
    let n = BslNumber::from_parts(1, -3).expect("отрицательный масштаб законен");
    assert_eq!(n.to_canonical(), "1000");
}

#[test]
fn multiplying_at_the_limit_is_an_error_and_never_a_panic_or_a_wrong_answer() {
    let a = BslNumber::from_parts(1, MAX_SCALE).expect("предел допустим");
    assert!(matches!(a.mul(&a), Err(NumError::ScaleOverflow)));
}

#[test]
fn division_scale_arithmetic_is_checked_too() {
    // `div_to_scale` считает `target + b.scale - a.scale`, и до правки
    // `check_scale` там не звался вовсе.
    let a = BslNumber::from_parts(1, MAX_SCALE).expect("предел допустим");
    let b = BslNumber::from_parts(1, -MAX_SCALE).expect("отрицательный предел допустим");
    match a.div(&b) {
        Ok(_) | Err(NumError::ScaleOverflow) => {}
        Err(other) => panic!("ожидались успех или ScaleOverflow, получено {other:?}"),
    }
}

#[test]
fn the_infallible_constructor_covers_whole_numbers() {
    assert_eq!(
        BslNumber::from_i128(1 << 100).to_canonical(),
        "1267650600228229401496703205376"
    );
}
