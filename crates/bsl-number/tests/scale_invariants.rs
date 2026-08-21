//! Масштаб не выходит за собственные пределы ни через конструктор, ни через
//! арифметику, ни через публичные методы округления.
//!
//! История дефектов, которые эти тесты держат:
//!
//! * `mul` складывал масштабы обычным `+` ДО `check_scale`, а `div_to_scale`
//!   считал `target + b.scale - a.scale` и `check_scale` не звал вовсе. На
//!   числе с масштабом `i32::MAX` это давало панику «attempt to add with
//!   overflow» в debug и молчаливый неверный ответ в release;
//! * `check_scale` ограничивал масштаб только сверху, поэтому
//!   `from_parts(1, i32::MIN)` строил число, на котором `to_canonical`
//!   паниковал при вычислении `-scale`;
//! * `round_to_scale` и соседи публичны и берут произвольный `i32`, а
//!   разность `cur_scale - target_scale` считали без защиты.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bsl_number::{BslNumber, MAX_SCALE, NumError};

fn hash_of(n: &BslNumber) -> u64 {
    let mut h = DefaultHasher::new();
    n.hash(&mut h);
    h.finish()
}

// --- Границы конструктора ------------------------------------------------

#[test]
fn the_constructor_accepts_both_limits_and_refuses_the_next_value_on_each_side() {
    assert!(BslNumber::from_parts(1, MAX_SCALE).is_ok());
    assert!(BslNumber::from_parts(1, -MAX_SCALE).is_ok());
    assert!(matches!(
        BslNumber::from_parts(1, MAX_SCALE + 1),
        Err(NumError::ScaleOverflow)
    ));
    assert!(matches!(
        BslNumber::from_parts(1, -MAX_SCALE - 1),
        Err(NumError::ScaleOverflow)
    ));
}

#[test]
fn the_extreme_i32_values_are_refused_by_both_constructors() {
    // `i32::MIN` — не просто «очень мало»: на нём паникует даже унарный
    // минус, поэтому число с таким масштабом нельзя было ни напечатать,
    // ни сравнить.
    for scale in [i32::MIN, i32::MAX] {
        assert!(
            matches!(
                BslNumber::from_parts(1, scale),
                Err(NumError::ScaleOverflow)
            ),
            "from_parts должен отвергать масштаб {scale}"
        );
        assert!(
            matches!(
                BslNumber::from_big_parts(num_bigint::BigInt::from(1), scale),
                Err(NumError::ScaleOverflow)
            ),
            "from_big_parts должен отвергать масштаб {scale}"
        );
    }
}

#[test]
fn a_negative_scale_stays_legal_inside_the_bound() {
    // Отрицательный масштаб — часть модели: им строит степени десятки
    // `Формат`. Ограничение — не запрет.
    let n = BslNumber::from_parts(1, -3).expect("отрицательный масштаб законен");
    assert_eq!(n.to_canonical(), "1000");
    assert!(BslNumber::from_parts(7, -MAX_SCALE).is_ok());
}

#[test]
fn to_canonical_never_panics_on_any_constructible_number() {
    for scale in [-MAX_SCALE, -1000, -1, 0, 1, 1000, MAX_SCALE] {
        let n = BslNumber::from_parts(123, scale).expect("масштаб в пределах");
        let text = n.to_canonical();
        assert!(
            !text.is_empty(),
            "пустое представление при масштабе {scale}"
        );
    }
}

// --- Арифметика ----------------------------------------------------------

#[test]
fn multiplying_at_the_limit_is_an_error_and_never_a_panic_or_a_wrong_answer() {
    let a = BslNumber::from_parts(1, MAX_SCALE).expect("предел допустим");
    assert!(matches!(a.mul(&a), Err(NumError::ScaleOverflow)));
}

#[test]
fn division_gives_the_exact_documented_result() {
    // Конкретный ответ, а не «успех или ошибка»: `DIV_SCALE` — 27 разрядов,
    // и 1/8 в нём представимо точно.
    let one = BslNumber::from_i128(1);
    let eight = BslNumber::from_i128(8);
    assert_eq!(one.div(&eight).expect("деление").to_canonical(), "0.125");

    let three = BslNumber::from_i128(3);
    assert_eq!(
        one.div(&three).expect("деление").to_canonical(),
        "0.333333333333333333333333333"
    );

    assert!(matches!(
        one.div(&BslNumber::from_i128(0)),
        Err(NumError::DivideByZero)
    ));
}

#[test]
fn dividing_at_the_scale_limits_gives_a_definite_answer() {
    // `div_to_scale` считает `target + b.scale - a.scale`, и на краях эта
    // арифметика обязана быть проверяемой. Ответ здесь КОНКРЕТНЫЙ, а не
    // «успех или ошибка»: 10^-MAX_SCALE делить на 10^MAX_SCALE — это
    // 10^(-2*MAX_SCALE), что в 27 разрядах `DIV_SCALE` округляется в ноль.
    let tiny = BslNumber::from_parts(1, MAX_SCALE).expect("предел допустим");
    let huge = BslNumber::from_parts(1, -MAX_SCALE).expect("предел допустим");
    assert_eq!(tiny.div(&huge).expect("деление").to_canonical(), "0");

    // Обратное деление представимо целым числом, и ответ здесь тоже
    // конкретный: 10^(2*MAX_SCALE), то есть единица и двести тысяч нулей.
    let quotient = huge.div(&tiny).expect("деление").to_canonical();
    assert_eq!(quotient.len(), 2 * MAX_SCALE as usize + 1);
    assert!(quotient.starts_with('1'));
    assert!(quotient[1..].bytes().all(|b| b == b'0'));
}

#[test]
fn rounding_to_an_extreme_target_scale_neither_panics_nor_lies() {
    // Методы публичные и берут произвольный `i32`, включая `i32::MIN`.
    let n = BslNumber::from_parts(12345, 3).expect("масштаб в пределах");
    assert_eq!(n.round_to_scale(i32::MAX).to_canonical(), "12.345");
    assert_eq!(n.round_to_scale(i32::MIN).to_canonical(), "0");
    assert_eq!(n.round_to_scale_half_down(i32::MIN).to_canonical(), "0");
    assert_eq!(n.trunc_to_scale(i32::MIN).to_canonical(), "0");
}

// --- Согласованность Eq/Ord/Hash ----------------------------------------

#[test]
fn equal_values_stay_equal_ordered_and_hashed_alike_across_scales() {
    // Одно и то же число, записанное разными масштабами, обязано быть
    // равным, одинаково хешироваться и не быть ни больше, ни меньше себя.
    let pairs = [
        (
            BslNumber::from_i128(1000),
            BslNumber::from_parts(1, -3).unwrap(),
        ),
        (
            BslNumber::from_i128(5),
            BslNumber::from_parts(500, 2).unwrap(),
        ),
    ];
    for (a, b) in pairs {
        assert_eq!(a, b, "{a:?} и {b:?} должны быть равны");
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
        assert_eq!(hash_of(&a), hash_of(&b), "равные числа — равные хеши");
    }

    let small = BslNumber::from_parts(1, MAX_SCALE).expect("предел допустим");
    let large = BslNumber::from_parts(1, -MAX_SCALE).expect("предел допустим");
    assert!(small < large, "на краях порядок обязан оставаться верным");
    assert_eq!(large.cmp(&small), std::cmp::Ordering::Greater);
}

#[test]
fn the_infallible_constructor_covers_whole_numbers() {
    assert_eq!(
        BslNumber::from_i128(1 << 100).to_canonical(),
        "1267650600228229401496703205376"
    );
}

#[test]
fn rounding_to_a_negative_scale_keeps_the_hash_contract() {
    // Достижимо из BSL: `Окр(24, -1)` кладёт результат с масштабом -1, а
    // `20` — с масштабом 0. Значения равны, поэтому и хеши обязаны
    // совпадать: иначе `Соответствие` с числовым ключом хранит две записи
    // под одним ключом. Проверено скриптом — до правки «ключей: 2».
    let rounded = BslNumber::from_i128(24).round_to_scale(-1);
    let direct = BslNumber::from_i128(20);
    assert_eq!(rounded, direct, "Окр(24, -1) равно 20");
    assert_eq!(rounded.to_canonical(), direct.to_canonical());
    assert_eq!(
        hash_of(&rounded),
        hash_of(&direct),
        "равные числа обязаны хешироваться одинаково"
    );

    // То же для усечения и для округления половины к нулю.
    for produced in [
        BslNumber::from_i128(24).trunc_to_scale(-1),
        BslNumber::from_i128(25).round_to_scale_half_down(-1),
    ] {
        assert_eq!(produced, direct);
        assert_eq!(hash_of(&produced), hash_of(&direct));
    }
}
