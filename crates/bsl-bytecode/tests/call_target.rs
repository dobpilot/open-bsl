//! Печать листинга не должна паниковать ни на какой публично собранной
//! программе: `Program` и `Chunk` — структуры с открытыми полями, поэтому
//! «такого байт-кода компилятор не выпускает» здесь не аргумент.

mod support;

use bsl_bytecode::{Instr, TextError, write_program};
use support::{chunk, program};

fn one_call(func: u16) -> bsl_bytecode::Program {
    program(vec![chunk(vec![Instr::Call {
        func,
        base: 0,
        arg_modes: 0,
        ret: 0,
    }])])
}

/// Нулевой номер функции — это ссылка на `chunks[0]`, то есть на верхний
/// уровень, которого не вызывает никто: нумерация в `Call` начинается с
/// единицы (`function_names[i]` — это `chunks[i+1]`). Комментатор листинга
/// вычитал из номера единицу, поэтому на нуле печать уходила в переполнение
/// вместо ошибки.
#[test]
fn a_call_of_chunk_zero_is_an_error_not_a_panic() {
    match write_program(&one_call(0), None) {
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
    assert!(matches!(
        write_program(&one_call(3), None),
        Err(TextError::BadCallTarget { func: 3, .. })
    ));
}

/// Номер в границах таблицы ИМЁН, но без своего чанка — тоже ссылка в
/// никуда: имя и тело связаны сдвигом на единицу, и наличие подписи ничего
/// не говорит о наличии кода. Такой листинг разобрался бы обратно, а VM
/// отвергла бы программу уже на исполнении (`Instr::Call` проверяет
/// `chunks.get(func)`), то есть печать выпустила бы заведомо неисполнимое.
#[test]
fn a_named_function_without_a_chunk_is_an_error() {
    let mut bad = one_call(1);
    bad.function_names = vec!["Ф".to_string()];
    assert_eq!(bad.chunks.len(), 1, "тела у функции нет по построению");
    assert!(matches!(
        write_program(&bad, None),
        Err(TextError::BadCallTarget { func: 1, .. })
    ));

    // А с телом — печатается и читается обратно.
    bad.chunks.push(chunk(vec![Instr::Return { src: None }]));
    let text = write_program(&bad, None).expect("целая программа обязана печататься");
    assert!(bsl_bytecode::parse_program(&text).is_ok());
}
