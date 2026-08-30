//! Замкнутое множество констант байт-кода.
//!
//! Таблица констант чанка обязана принимать только то, что представимо в
//! текстовом формате. Иначе образ собирается, проходит проверку и
//! исполняется, а отказ приходит от печати листинга — команды, к дефекту
//! отношения не имеющей. У объекта беда и вторая: он изменяем и живёт за
//! `Rc`, поэтому «константа» разделялась бы всеми исполнениями своей
//! `LoadConst`.

mod support;

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

/// Каждое представимое значение — из перечисленных в спецификации, а не
/// из удобных — становится константой.
fn representable_cases() -> Vec<BslValue> {
    let member = bsl_rt::lookup_member(bsl_rt::EnumKind::JsonValueType, "Строка")
        .expect("член перечисления для проверки");
    vec![
        BslValue::Undefined,
        BslValue::Null,
        BslValue::Boolean(true),
        BslValue::number_from_i64(42),
        BslValue::Str(bsl_rt::BslString::from_str("строка")),
        BslValue::Date(bsl_rt::BslDate::from_seconds(1_700_000_000).expect("дата для проверки")),
        BslValue::Enum(member),
        BslValue::EnumType(bsl_rt::EnumKind::JsonValueType),
    ]
}

#[test]
fn every_representable_value_becomes_a_constant() {
    for v in representable_cases() {
        assert!(
            BytecodeConst::new(v.clone()).is_ok(),
            "представимое значение {v:?} не приняли в константы"
        );
    }
}

/// Главный инвариант: проверяемое преобразование и ПЕЧАТЬ согласны о
/// представимости на каждом значении.
///
/// Классификация в проекте одна именно ради этого. Прежде их было две —
/// исчерпывающая у печати и с ветвью-заглушкой у преобразования, — и
/// новый вариант `BslValue` обновил бы только первую: печать объявила бы
/// его непредставимым, преобразование молча приняло бы. Тест ловит такое
/// расхождение на всех значениях, которые умеет построить.
#[test]
fn the_checked_conversion_and_printing_agree_on_representability() {
    let mut cases = representable_cases();
    cases.push(BslValue::new_array(Vec::new()));
    cases.push(BslValue::Type(bsl_rt::TypeRef::Native(
        bsl_rt::TypeId::String,
    )));
    for v in cases {
        let accepted = BytecodeConst::new(v.clone()).is_ok();
        // Печать той же константы: программа с одним чанком, одной
        // константой и `Возврат`.
        let mut program = support::program(vec![support::chunk(vec![
            bsl_bytecode::Instr::LoadConst { dst: 0, k: 0 },
            bsl_bytecode::Instr::Return { src: None },
        ])]);
        program.chunks[0].n_regs = 1;
        program.chunks[0].consts = vec![BytecodeConst::transient(v.clone())];
        bsl_bytecode::image::finalize(&mut program);
        let printable = !matches!(
            bsl_bytecode::write_program(&program, None),
            Err(bsl_bytecode::TextError::Unrepresentable(_))
        );
        assert_eq!(
            accepted, printable,
            "преобразование и печать разошлись на {v:?}: приняли {accepted}, печатается {printable}"
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
