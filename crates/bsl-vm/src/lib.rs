//! Цикл диспетчеризации VM: `match` в `loop`, без computed goto (в Rust его
//! нет и явные хвостовые вызовы не стабилизированы — не воюем с этим здесь,
//! оптимизация диспетчеризации приходит после профилирования, не раньше).
//!
//! Параметры без `Знач` передаются по ссылке, а указатель на регистр
//! вызывающего брать нельзя — рост общего стека значений (`Vec<BslValue>`)
//! его инвалидирует. Вместо указателя параметр хранит АБСОЛЮТНЫЙ ИНДЕКС в
//! этом стеке (см. `Frame::param_aliases`): индекс переживает любой рост
//! `Vec`, а времени жизни хватает, потому что в BSL нельзя сохранить ссылку
//! на переменную за пределы вызова.

use bsl_bytecode::{ArgMode, Instr, Program};
use bsl_rt::{BslValue, RtError};

/// Один активный вызов. Регистры кадра не хранятся отдельным `Vec` — все
/// кадры делят один сквозной стек значений (`Vm::stack`), кадр это лишь
/// окно в него, как в Lua.
struct Frame {
    func_id: usize,
    pc: usize,
    /// Абсолютные индексы в `Vm::stack` для параметров (длина — `n_params`
    /// вызванной функции). Для `Знач`-параметров и для параметров без
    /// `Знач`, но с не-переменным аргументом, это индекс материализованного
    /// значения (временный регистр вызывающего или свежая ячейка). Для
    /// параметров без `Знач` с голой переменной на месте вызова — это
    /// индекс самой переменной вызывающего: чтение/запись слота параметра
    /// напрямую видны вызывающему.
    param_aliases: Vec<usize>,
    /// Абсолютный индекс начала "собственных" регистров кадра (локалы
    /// сверх параметров + временные) — они всегда свежие, только что
    /// вытолкнутые в стек под этот вызов.
    own_base: usize,
    /// Абсолютный индекс, до которого укоротить `Vm::stack` при возврате —
    /// всё, что вызывающий вычислил ДО этого вызова, останется нетронутым;
    /// алиасы параметров, указывающие в более ранние кадры, не пострадают.
    call_start: usize,
    /// Регистр РОДИТЕЛЬСКОГО кадра, куда положить результат при возврате
    /// (не используется для самого нижнего/верхнего кадра).
    return_reg: u8,
}

impl Frame {
    #[inline]
    fn reg_index(&self, r: u8) -> usize {
        let r = r as usize;
        if r < self.param_aliases.len() {
            self.param_aliases[r]
        } else {
            self.own_base + (r - self.param_aliases.len())
        }
    }
}

/// Выполняет модуль с точки входа — операторов верхнего уровня (`chunks[0]`)
/// — и возвращает значение, которым он завершился (через `Возврат` на
/// верхнем уровне, что нетипично, но не запрещено; обычно — `Неопределено`).
pub fn run_program(program: &Program) -> Result<BslValue, RtError> {
    let mut stack: Vec<BslValue> = Vec::new();
    push_own_registers(&mut stack, &program.chunks[0]);
    let mut frames = vec![Frame {
        func_id: 0,
        pc: 0,
        param_aliases: Vec::new(),
        own_base: 0,
        call_start: 0,
        return_reg: 0,
    }];

    loop {
        let frame_idx = frames.len() - 1;
        let func_id = frames[frame_idx].func_id;
        let pc = frames[frame_idx].pc;
        let chunk = &program.chunks[func_id];

        if pc >= chunk.instrs.len() {
            // Неявный возврат: тело кончилось без `Возврат` — результат
            // Неопределено, как и `Возврат;` без выражения.
            match do_return_with_value(&mut frames, &mut stack, BslValue::Undefined) {
                Done(v) => return Ok(v),
                Continuing => continue,
            }
        }

        let instr = chunk.instrs[pc];
        match instr {
            Instr::Move { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = stack[s].clone();
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadConst { dst, k } => {
                let v = chunk.consts[k as usize].clone();
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadBool { dst, val } => {
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Boolean(val);
                frames[frame_idx].pc += 1;
            }
            Instr::LoadUndefined { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Undefined;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadNull { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Null;
                frames[frame_idx].pc += 1;
            }
            Instr::Add { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::add)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Sub { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::sub)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Mul { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::mul)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Div { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::div)?;
                frames[frame_idx].pc += 1;
            }
            Instr::And { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::and)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Or { dst, a, b } => {
                binop(&mut frames, &mut stack, frame_idx, dst, a, b, BslValue::or)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Neg { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = stack[s].neg()?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::Not { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = stack[s].not()?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::Eq { dst, a, b } => {
                let av = stack[frames[frame_idx].reg_index(a)].clone();
                let bv = stack[frames[frame_idx].reg_index(b)].clone();
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Boolean(av.eq_value(&bv));
                frames[frame_idx].pc += 1;
            }
            Instr::NotEq { dst, a, b } => {
                let av = stack[frames[frame_idx].reg_index(a)].clone();
                let bv = stack[frames[frame_idx].reg_index(b)].clone();
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Boolean(!av.eq_value(&bv));
                frames[frame_idx].pc += 1;
            }
            Instr::Lt { dst, a, b } => {
                cmp(&mut frames, &mut stack, frame_idx, dst, a, b, "<", |o| {
                    o.is_lt()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Gt { dst, a, b } => {
                cmp(&mut frames, &mut stack, frame_idx, dst, a, b, ">", |o| {
                    o.is_gt()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Le { dst, a, b } => {
                cmp(&mut frames, &mut stack, frame_idx, dst, a, b, "<=", |o| {
                    o.is_le()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Ge { dst, a, b } => {
                cmp(&mut frames, &mut stack, frame_idx, dst, a, b, ">=", |o| {
                    o.is_ge()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Jump { target } => {
                frames[frame_idx].pc = target as usize;
            }
            Instr::JumpIfFalse { cond, target } => {
                let c = frames[frame_idx].reg_index(cond);
                // Строгая булевость: не-`Булево` в условии — ошибка типа,
                // а не приведение к истинности.
                if stack[c].as_condition()? {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::Call {
                func,
                base,
                arg_modes,
                ret,
            } => {
                // Caller продвигается ЗА инструкцию Call сейчас — так, когда
                // callee вернётся, мы продолжим ровно со следующей.
                frames[frame_idx].pc += 1;

                let modes = &chunk.call_arg_modes[arg_modes as usize];
                let mut param_aliases = Vec::with_capacity(modes.len());
                for (i, mode) in modes.iter().enumerate() {
                    let idx = match mode {
                        ArgMode::Value => frames[frame_idx].reg_index(base + i as u8),
                        ArgMode::ByRefLocal(slot) => frames[frame_idx].reg_index(*slot),
                    };
                    param_aliases.push(idx);
                }

                let callee_chunk = &program.chunks[func as usize];
                let call_start = stack.len();
                let own_base = stack.len();
                push_own_registers(&mut stack, callee_chunk);

                frames.push(Frame {
                    func_id: func as usize,
                    pc: 0,
                    param_aliases,
                    own_base,
                    call_start,
                    return_reg: ret,
                });
            }
            Instr::Return { src } => {
                let value = match src {
                    Some(r) => {
                        let idx = frames[frame_idx].reg_index(r);
                        stack[idx].clone()
                    }
                    None => BslValue::Undefined,
                };
                match do_return_with_value(&mut frames, &mut stack, value) {
                    Done(v) => return Ok(v),
                    Continuing => continue,
                }
            }
        }
    }
}

/// Заводит "собственные" регистры чанка (сверх параметров) в конце стека —
/// используется и для верхнего уровня (0 параметров), и для вызовов.
fn push_own_registers(stack: &mut Vec<BslValue>, chunk: &bsl_bytecode::Chunk) {
    let n_own = (chunk.n_regs - chunk.n_params) as usize;
    stack.resize(stack.len() + n_own, BslValue::Undefined);
}

enum ReturnOutcome {
    Done(BslValue),
    Continuing,
}
use ReturnOutcome::{Continuing, Done};

fn do_return_with_value(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    value: BslValue,
) -> ReturnOutcome {
    let frame = frames.pop().expect("VM: возврат без активного кадра");
    stack.truncate(frame.call_start);
    match frames.last() {
        None => Done(value),
        Some(caller) => {
            let dst = caller.reg_index(frame.return_reg);
            stack[dst] = value;
            Continuing
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn binop(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
    f: impl Fn(&BslValue, &BslValue) -> Result<BslValue, RtError>,
) -> Result<(), RtError> {
    let av = stack[frames[frame_idx].reg_index(a)].clone();
    let bv = stack[frames[frame_idx].reg_index(b)].clone();
    let result = f(&av, &bv)?;
    let d = frames[frame_idx].reg_index(dst);
    stack[d] = result;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmp(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
    op: &'static str,
    f: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<(), RtError> {
    let av = stack[frames[frame_idx].reg_index(a)].clone();
    let bv = stack[frames[frame_idx].reg_index(b)].clone();
    let ord = av.compare(&bv, op)?;
    let d = frames[frame_idx].reg_index(dst);
    stack[d] = BslValue::Boolean(f(ord));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_bytecode::compile_program;
    use bsl_number::BslNumber;
    use bsl_sema::resolve_program;
    use bsl_syntax::parse;

    fn run_src(src: &str) -> BslValue {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let resolved = resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"));
        let program = compile_program(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"));
        run_program(&program).unwrap_or_else(|e| panic!("runtime error: {e:?}"))
    }

    fn run_src_err(src: &str) -> RtError {
        let prog = parse(src).unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        let program = compile_program(&resolved).unwrap();
        run_program(&program).unwrap_err()
    }

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    #[test]
    fn function_call_and_return_value() {
        let v = run_src("Функция Ф()\nВозврат 42;\nКонецФункции\nВозврат Ф();");
        assert_eq!(v, num("42"));
    }

    #[test]
    fn function_without_return_yields_undefined_and_is_callable_as_statement() {
        let v = run_src("Процедура П()\nx = 1;\nКонецПроцедуры\nП();\nВозврат Неопределено;");
        assert_eq!(v, BslValue::Undefined);
    }

    #[test]
    fn recursion_factorial() {
        let v = run_src(
            "Функция Факториал(n)\n\
             Если n <= 1 Тогда\n\
             Возврат 1;\n\
             КонецЕсли;\n\
             Возврат n * Факториал(n - 1);\n\
             КонецФункции\n\
             Возврат Факториал(5);",
        );
        assert_eq!(v, num("120"));
    }

    #[test]
    fn by_reference_parameter_mutates_callers_variable() {
        // Процедура П(а) а = 5 КонецПроцедуры — меняет переменную вызывающего,
        // т.к. параметры без Знач передаются по ссылке.
        let v = run_src(
            "Процедура П(а)\n\
             а = 5;\n\
             КонецПроцедуры\n\
             x = 1;\n\
             П(x);\n\
             Возврат x;",
        );
        assert_eq!(v, num("5"));
    }

    #[test]
    fn by_value_parameter_does_not_mutate_callers_variable() {
        let v = run_src(
            "Процедура П(Знач а)\n\
             а = 5;\n\
             КонецПроцедуры\n\
             x = 1;\n\
             П(x);\n\
             Возврат x;",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn by_reference_swap_via_two_parameters() {
        let v = run_src(
            "Процедура Обменять(а, б)\n\
             временная = а;\n\
             а = б;\n\
             б = временная;\n\
             КонецПроцедуры\n\
             x = 1;\n\
             y = 2;\n\
             Обменять(x, y);\n\
             Возврат x * 10 + y;",
        );
        // Было x=1,y=2 -> после обмена x=2,y=1 -> 2*10+1 = 21.
        assert_eq!(v, num("21"));
    }

    #[test]
    fn by_reference_argument_that_is_not_a_bare_variable_does_not_crash() {
        // Аргумент — выражение, не переменная: запись в параметр пишет в
        // одноразовую ячейку, наблюдаемого эффекта у вызывающего нет, но и
        // падать тут нечему.
        let v = run_src(
            "Процедура П(а)\n\
             а = 99;\n\
             КонецПроцедуры\n\
             x = 1;\n\
             П(x + 1);\n\
             Возврат x;",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn mutual_forward_calls_between_functions() {
        let v = run_src(
            "Функция ЧетноеЛи(n)\n\
             Если n = 0 Тогда\n\
             Возврат Истина;\n\
             КонецЕсли;\n\
             Возврат НечетноеЛи(n - 1);\n\
             КонецФункции\n\
             Функция НечетноеЛи(n)\n\
             Если n = 0 Тогда\n\
             Возврат Ложь;\n\
             КонецЕсли;\n\
             Возврат ЧетноеЛи(n - 1);\n\
             КонецФункции\n\
             Возврат ЧетноеЛи(6);",
        );
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn division_matches_oracle_27_digits_inside_a_function() {
        let v = run_src("Функция Ф()\nВозврат 1 / 3;\nКонецФункции\nВозврат Ф();");
        assert_eq!(v, num("0.333333333333333333333333333"));
    }

    #[test]
    fn while_and_for_loops_still_work_at_top_level() {
        let v = run_src(
            "sum = 0;\n\
             Для i = 0 По 10 Цикл\n\
             sum = sum + i;\n\
             КонецЦикла\n\
             Возврат sum;",
        );
        assert_eq!(v, num("55"));
    }

    #[test]
    fn strict_boolean_condition_is_a_runtime_error() {
        let err = run_src_err("Если 1 Тогда\nx = 1;\nКонецЕсли");
        assert!(matches!(
            err,
            RtError::TypeError {
                expected: "Булево",
                ..
            }
        ));
    }

    #[test]
    fn division_by_zero_is_a_runtime_error() {
        let err = run_src_err("x = 1 / 0;");
        assert!(matches!(err, RtError::Num(bsl_number::NumError::DivideByZero)));
    }
}
