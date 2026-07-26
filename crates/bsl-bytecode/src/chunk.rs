use bsl_rt::BslValue;

use crate::instr::{ArgMode, Instr};

/// Скомпилированное тело одной функции/процедуры либо скрипта верхнего
/// уровня. `n_regs` — пиковое число регистров кадра (параметры + локалы +
/// максимум одновременно живых временных) — столько `BslValue` резервирует
/// VM при входе в кадр.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub instrs: Vec<Instr>,
    pub consts: Vec<BslValue>,
    /// Режимы аргументов для каждой инструкции `Call` в этом чанке,
    /// индексируются полем `arg_modes` той инструкции.
    pub call_arg_modes: Vec<Vec<ArgMode>>,
    /// Параметры занимают слоты `0..n_params` (см. `bsl-sema`), это нужно
    /// VM, чтобы отличить слоты-алиасы (параметры) от собственных слотов
    /// кадра при вычислении абсолютного индекса в стеке значений.
    pub n_params: u8,
    pub n_locals: u8,
    pub n_regs: u8,
}

/// Весь скомпилированный модуль: `chunks[0]` — скрипт верхнего уровня,
/// `chunks[1..]` — объявленные `Процедура`/`Функция` в порядке резолвинга
/// (индекс `i` в `ResolvedProgram::functions` соответствует `chunks[i+1]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub chunks: Vec<Chunk>,
}
