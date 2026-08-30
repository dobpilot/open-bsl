//! Замкнутое множество констант байт-кода.
//!
//! Таблица констант чанка обязана принимать только то, что представимо в
//! текстовом формате. Иначе образ собирается, проходит проверку и
//! исполняется, а отказ приходит от печати листинга — команды, к дефекту
//! отношения не имеющей. У объекта беда и вторая: он изменяем и живёт за
//! `Rc`, поэтому «константа» разделялась бы всеми исполнениями своей
//! `LoadConst`.

use bsl_bytecode::BytecodeConst;
use bsl_rt::BslValue;

/// Объект константой быть не может.
#[test]
fn an_object_is_not_a_bytecode_constant() {
    let object = BslValue::new_array(Vec::new());
    assert!(
        BytecodeConst::new(object).is_err(),
        "объект приняли в таблицу констант"
    );
}

/// Тип — тоже: текстовый формат его не представляет.
#[test]
fn a_type_value_is_not_a_bytecode_constant() {
    let value = BslValue::Type(bsl_rt::TypeRef::Native(bsl_rt::TypeId::String));
    assert!(
        BytecodeConst::new(value).is_err(),
        "значение-тип приняли в таблицу констант"
    );
}

/// Всё, что формат представляет, константой быть обязано.
#[test]
fn every_representable_value_becomes_a_constant() {
    let cases = vec![
        BslValue::Undefined,
        BslValue::Null,
        BslValue::Boolean(true),
        BslValue::number_from_i64(42),
        BslValue::Str(bsl_rt::BslString::from_str("строка")),
    ];
    for v in cases {
        assert!(
            BytecodeConst::new(v.clone()).is_ok(),
            "представимое значение {v:?} не приняли в константы"
        );
    }
}

/// Константа читается как `BslValue` без преобразования: тип прозрачен, и
/// это условие того, что горячий путь не подорожал.
#[test]
fn a_constant_dereferences_to_its_value() {
    let c = BytecodeConst::new(BslValue::number_from_i64(7)).expect("число — константа");
    assert_eq!(
        std::mem::size_of::<BytecodeConst>(),
        std::mem::size_of::<BslValue>(),
        "обёртка изменила представление — горячий путь подорожает"
    );
    assert!(matches!(&*c, BslValue::Number(_)));
}
