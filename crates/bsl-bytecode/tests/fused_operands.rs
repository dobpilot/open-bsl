//! Индексы новых слитых инструкций — недоверенная часть образа.

mod support;

use bsl_bytecode::{Instr, image};
use bsl_number::BslNumber;
use bsl_rt::{BslValue, RtError};
use support::{chunk, program};

fn number() -> BslValue {
    BslValue::Number(BslNumber::from_i64(1))
}

#[test]
fn add_const_rejects_invalid_registers_and_constant() {
    let make = |dst, src, k| {
        let mut c = chunk(vec![Instr::AddConst { dst, src, k }]);
        c.consts.push(number());
        program(vec![c])
    };

    for broken in [make(1, 0, 0), make(0, 1, 0)] {
        assert!(matches!(
            image::verify(&broken),
            Err(RtError::InvalidBytecode(
                "регистр сложения с константой выходит за кадр"
            ))
        ));
    }
    assert!(matches!(
        image::verify(&make(0, 0, 1)),
        Err(RtError::InvalidBytecode(
            "номер константы сложения вне таблицы чанка"
        ))
    ));
}

#[test]
fn both_fused_jumps_reject_invalid_operands() {
    let bad_registers = [
        Instr::JumpIfNotEqConst {
            src: 1,
            k: 0,
            target: 1,
        },
        Instr::JumpIfNotLtConst {
            src: 1,
            k: 0,
            target: 1,
        },
    ];
    for instr in bad_registers {
        let mut c = chunk(vec![instr]);
        c.consts.push(number());
        assert!(matches!(
            image::verify(&program(vec![c])),
            Err(RtError::InvalidBytecode(
                "регистр условного перехода выходит за кадр"
            ))
        ));
    }

    let bad_constants = [
        Instr::JumpIfNotEqConst {
            src: 0,
            k: 1,
            target: 1,
        },
        Instr::JumpIfNotLtConst {
            src: 0,
            k: 1,
            target: 1,
        },
    ];
    for instr in bad_constants {
        let mut c = chunk(vec![instr]);
        c.consts.push(number());
        assert!(matches!(
            image::verify(&program(vec![c])),
            Err(RtError::InvalidBytecode(
                "номер константы условного перехода вне таблицы чанка"
            ))
        ));
    }
}
