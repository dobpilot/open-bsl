//! Печать листинга не должна паниковать ни на какой публично собранной
//! программе: `Program` и `Chunk` — структуры с открытыми полями, поэтому
//! «такого байт-кода компилятор не выпускает» здесь не аргумент.

use bsl_bytecode::{Chunk, Instr, Program, TextError, write_program};

/// Пустой чанк с одной инструкцией: остальные поля — то, что получилось бы
/// у `Program`, собранной руками, а не компилятором.
fn chunk(instrs: Vec<Instr>) -> Chunk {
    Chunk {
        instrs,
        consts: Vec::new(),
        call_arg_modes: vec![Vec::new()],
        exception_ranges: Vec::new(),
        n_params: 0,
        param_by_val: Vec::new(),
        is_procedure: false,
        n_locals: 0,
        n_regs: 1,
        prop_cache: Vec::new(),
        method_cache: Vec::new(),
        local_names: Vec::new(),
        bundle_len: Vec::new(),
        touches_objects: false,
    }
}

fn program(instrs: Vec<Instr>) -> Program {
    Program {
        requirements: vec![bsl_bytecode::LibraryRequirement::bsl_rt()],
        chunks: vec![chunk(instrs)],
        names: Vec::new(),
        shapes: Vec::new(),
        top_level_locals: Vec::new(),
        module_vars: Vec::new(),
        module_base: 0,
        function_names: Vec::new(),
    }
}

/// Нулевой номер функции — это ссылка на `chunks[0]`, то есть на верхний
/// уровень, которого не вызывает никто: нумерация в `Call` начинается с
/// единицы (`function_names[i]` — это `chunks[i+1]`). Комментатор листинга
/// вычитал из номера единицу, поэтому на нуле печать уходила в переполнение
/// вместо ошибки.
#[test]
fn a_call_of_chunk_zero_is_an_error_not_a_panic() {
    let bad = program(vec![Instr::Call {
        func: 0,
        base: 0,
        arg_modes: 0,
        ret: 0,
    }]);
    match write_program(&bad, None) {
        Err(TextError::BadCallTarget { chunk, pc, func }) => {
            assert_eq!((chunk, pc, func), (0, 0, 0));
        }
        other => panic!("ожидалась типизированная ошибка, получено {other:?}"),
    }
}

/// Номер за концом таблицы функций тоже отвергается: напечатанный листинг
/// нельзя было бы разобрать обратно во что-то исполнимое.
#[test]
fn a_call_past_the_last_function_is_an_error() {
    let bad = program(vec![Instr::Call {
        func: 3,
        base: 0,
        arg_modes: 0,
        ret: 0,
    }]);
    assert!(matches!(
        write_program(&bad, None),
        Err(TextError::BadCallTarget { func: 3, .. })
    ));
}
