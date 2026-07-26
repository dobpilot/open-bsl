use std::rc::Rc;

use bsl_rt::{BslValue, Shape};

use crate::instr::{ArgMode, Instr};

/// `(начало_pc, конец_pc, обработчик_pc)` на функцию — ноль стоимости, пока
/// исключение не брошено, как и описано в брифе (JVM/CPython делают так
/// же). `end_pc` эксклюзивно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionRange {
    pub start_pc: usize,
    pub end_pc: usize,
    pub handler_pc: usize,
}

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
    /// Диапазоны `Попытка`/`КонецПопытки` в этом чанке — при нескольких
    /// вложенных диапазонах, содержащих один pc, VM выбирает самый узкий.
    pub exception_ranges: Vec<ExceptionRange>,
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
///
/// `names`/`shapes` — общие для всего модуля таблицы, интернированные один
/// раз при компиляции (см. `bsl_rt::NameInterner`/`ShapeTable`): доступ к
/// полю ссылается на них по индексу (`NameId`/индекс формы), а не хранит
/// строку или список полей в каждой инструкции.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub chunks: Vec<Chunk>,
    pub names: Vec<String>,
    pub shapes: Vec<Rc<Shape>>,
    /// Слоты верхнего уровня по именам, в порядке — нужно `Выполнить`/
    /// `Вычислить` (см. `bsl-vm`), чтобы резолвить уже объявленные
    /// top-level переменные фрагмента на те же слоты, что и в статическом
    /// коде, а не заводить их заново.
    pub top_level_locals: Vec<String>,
}
