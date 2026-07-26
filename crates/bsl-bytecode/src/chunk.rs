use bsl_rt::BslValue;

use crate::instr::Instr;

/// Скомпилированный скрипт верхнего уровня. `n_regs` — пиковое число
/// регистров кадра (локалы + максимум временных одновременно живых) —
/// столько `BslValue` заводит VM при старте.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub instrs: Vec<Instr>,
    pub consts: Vec<BslValue>,
    pub n_locals: u8,
    pub n_regs: u8,
}
