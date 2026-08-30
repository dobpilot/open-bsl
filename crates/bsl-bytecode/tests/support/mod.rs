//! Программы, собранные РУКАМИ, а не скомпилированные из BSL.
//!
//! Тесты представления и текстового формата обходятся без фронтенда
//! намеренно: `bsl-bytecode` не зависит ни от лексера, ни от резолвера, и
//! проверять его через них значило бы завести эту зависимость обратно —
//! пусть и только в тестах. Заодно программу здесь можно довести до
//! состояния, которого кодоген не выпускает: испорченный листинг правят
//! руками, и именно такой вход парсер обязан отвергать.

// Модуль включается в КАЖДЫЙ тестовый бинарник, а нужен каждому свой
// набор сборщиков: без этого неиспользованный в конкретном файле сборщик
// был бы предупреждением там, где он совершенно уместен.
#![allow(dead_code)]

use std::rc::Rc;

use bsl_bytecode::{
    ArgMode, BytecodeConst, Chunk, ExceptionRange, Instr, LibraryRequirement, Program, image,
};
use bsl_rt::{BslValue, NameId, Shape, ShapeTable};

/// Чанк с этими инструкциями и пустыми таблицами. Кэши создаются длиной с
/// код: VM индексирует их номером инструкции.
#[must_use]
/// Константа обвязки. Тесты кладут только представимое, поэтому отказ
/// проверяемого преобразования здесь — ошибка самого теста.
pub fn konst(value: bsl_rt::BslValue) -> BytecodeConst {
    BytecodeConst::new(value).expect("тест положил непредставимую константу")
}

pub fn chunk(instrs: Vec<Instr>) -> Chunk {
    // Производные таблицы здесь не заполняются: их ставит
    // `image::finalize`, и снаружи крейта они закрыты.
    let mut c = Chunk::new();
    c.instrs = instrs;
    c.n_regs = 1;
    c
}

/// Программа из этих чанков: нулевой — верхний уровень, остальные должны
/// быть подписаны в `function_names`.
#[must_use]
pub fn program(chunks: Vec<Chunk>) -> Program {
    Program {
        requirements: vec![LibraryRequirement::bsl_rt()],
        chunks,
        names: Vec::new(),
        shapes: Vec::new(),
        top_level_locals: Vec::new(),
        module_vars: Vec::new(),
        exported_module_vars: Vec::new(),
        module_base: 0,
        links: Vec::new(),
        function_names: Vec::new(),
        exported_functions: Vec::new(),
    }
}

/// Одна интернированная форма с этими полями — как её завёл бы кодоген.
#[must_use]
pub fn shapes(fields: &[NameId]) -> Vec<Rc<Shape>> {
    let mut table = ShapeTable::new();
    table.intern(fields);
    table.into_shapes()
}

/// Программа, у которой НЕПУСТА каждая секция листинга со счётчиком:
/// требования, имена, формы, локальные верхнего уровня, переменные модуля,
/// функции, константы, режимы аргументов, обработчики, имена слотов и код.
///
/// Одна программа на всё: секции проверяются по первому вхождению, поэтому
/// заполнен именно нулевой чанк.
#[must_use]
pub fn every_section() -> Program {
    let field = NameId::from_index(0);
    let mut top = chunk(vec![
        Instr::LoadConst { dst: 0, k: 0 },
        Instr::NewStructure {
            dst: 1,
            shape: 0,
            base: 0,
            count: 0,
        },
        Instr::GetProp {
            dst: 2,
            obj: 1,
            name: field,
        },
        Instr::Call {
            func: 1,
            base: 3,
            arg_modes: 0,
            ret: 0,
        },
        Instr::Return { src: None },
    ]);
    top.consts = vec![BytecodeConst::new(BslValue::number_from_i64(1)).expect("число — константа")];
    top.call_arg_modes = vec![vec![ArgMode::Value]];
    top.exception_ranges = vec![ExceptionRange {
        start_pc: 0,
        end_pc: 3,
        handler_pc: 4,
    }];
    top.local_names = vec!["х".to_string()];
    top.n_locals = 1;
    top.n_regs = 4;

    let mut callee = chunk(vec![Instr::Return { src: None }]);
    callee.n_params = 1;
    callee.param_by_val = vec![true];
    callee.n_regs = 1;

    let mut p = program(vec![top, callee]);
    p.requirements
        .push(LibraryRequirement::new("bsl-test-host", "1.2.3"));
    p.names = vec!["Поле".to_string()];
    p.shapes = shapes(&[field]);
    p.top_level_locals = vec!["х".to_string()];
    p.module_vars = vec!["Общая".to_string()];
    p.exported_module_vars = vec![true];
    p.function_names = vec!["Ф".to_string()];
    p.exported_functions = vec![true];
    // Разметка бандлов производная, но у ОБРАЗЦА она должна быть настоящей:
    // печать помечает ею многочленные бандлы, а разбор считает её заново, и
    // без неё round-trip разошёлся бы на комментарии.
    image::finalize(&mut p);
    p
}
