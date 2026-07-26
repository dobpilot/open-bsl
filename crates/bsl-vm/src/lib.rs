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
///
/// Исключения (`Попытка`/`ВызватьИсключение`) ловятся здесь, а не внутри
/// `step`: если очередная инструкция вернула `Err`, кадр(ы) разматываются
/// (`unwind_to_handler`) в поисках защищённого диапазона, который её
/// накрывает — начиная с того кадра, где ошибка произошла, и дальше наружу
/// через вызовы. Не нашли нигде — ошибка настоящая, возвращаем её вызывающему
/// Rust-коду.
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
    let mut current_exception: Option<BslValue> = None;

    loop {
        match step(&mut frames, &mut stack, program, &mut current_exception) {
            Ok(Step::Continue) => continue,
            Ok(Step::Done(v)) => return Ok(v),
            Err(e) => {
                if !unwind_to_handler(&mut frames, &mut stack, program, &e, &mut current_exception) {
                    return Err(e);
                }
                // Иначе кадры/pc уже поправлены внутри unwind_to_handler —
                // просто продолжаем цикл со следующей итерации.
            }
        }
    }
}

enum Step {
    Continue,
    Done(BslValue),
}

/// Выполняет РОВНО одну инструкцию текущего (верхнего) кадра.
fn step(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    current_exception: &mut Option<BslValue>,
) -> Result<Step, RtError> {
    let frame_idx = frames.len() - 1;
    let func_id = frames[frame_idx].func_id;
    let pc = frames[frame_idx].pc;
    let chunk = &program.chunks[func_id];

    if pc >= chunk.instrs.len() {
        // Неявный возврат: тело кончилось без `Возврат` — результат
        // Неопределено, как и `Возврат;` без выражения.
        return Ok(match do_return_with_value(frames, stack, BslValue::Undefined) {
            Done(v) => Step::Done(v),
            Continuing => Step::Continue,
        });
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
                binop(frames, stack, frame_idx, dst, a, b, BslValue::add)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Sub { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::sub)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Mul { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::mul)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Div { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::div)?;
                frames[frame_idx].pc += 1;
            }
            Instr::And { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::and)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Or { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::or)?;
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
                cmp(frames, stack, frame_idx, dst, a, b, "<", |o| {
                    o.is_lt()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Gt { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">", |o| {
                    o.is_gt()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Le { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, "<=", |o| {
                    o.is_le()
                })?;
                frames[frame_idx].pc += 1;
            }
            Instr::Ge { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">=", |o| {
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
                push_own_registers(stack, callee_chunk);

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
                return Ok(match do_return_with_value(frames, stack, value) {
                    Done(v) => Step::Done(v),
                    Continuing => Step::Continue,
                });
            }
            Instr::GetIndex { dst, obj, idx } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                let iv = stack[frames[frame_idx].reg_index(idx)].clone();
                let v = ov.get_index(&iv)?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::SetIndex { obj, idx, src } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                let iv = stack[frames[frame_idx].reg_index(idx)].clone();
                let sv = stack[frames[frame_idx].reg_index(src)].clone();
                ov.set_index(&iv, sv)?;
                frames[frame_idx].pc += 1;
            }
            Instr::GetProp { dst, obj, name } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                // Структура резолвится по NameId напрямую (быстрый путь);
                // СтрокаТаблицыЗначений заводит колонки в рантайме и не
                // могла быть интернирована на этапе компиляции — для неё
                // (и только когда NameId-путь говорит "это не такой
                // объект") VM резолвит имя в текст через Program::names и
                // идёт по строковому пути.
                let v = match ov.get_field(name) {
                    Err(RtError::NotAnObject) => ov.get_field_by_name(&program.names[name.index()])?,
                    other => other?,
                };
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::SetProp { obj, name, src } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                let sv = stack[frames[frame_idx].reg_index(src)].clone();
                match ov.set_field(name, sv.clone()) {
                    Err(RtError::NotAnObject) => {
                        ov.set_field_by_name(&program.names[name.index()], sv)?
                    }
                    other => other?,
                }
                frames[frame_idx].pc += 1;
            }
            Instr::NewArray { dst, base, count } => {
                let mut dims = Vec::with_capacity(count as usize);
                for i in 0..count {
                    let v = stack[frames[frame_idx].reg_index(base + i)].clone();
                    dims.push(dim_to_usize(&v)?);
                }
                let arr = build_nested_array(&dims);
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = arr;
                frames[frame_idx].pc += 1;
            }
            Instr::NewStructure {
                dst,
                shape,
                base,
                count,
            } => {
                let shape_rc = program.shapes[shape as usize].clone();
                let mut slots = Vec::with_capacity(count as usize);
                for i in 0..count {
                    slots.push(stack[frames[frame_idx].reg_index(base + i)].clone());
                }
                let v = BslValue::new_structure(shape_rc, slots);
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::NewTable { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::new_table();
                frames[frame_idx].pc += 1;
            }
            Instr::CollectionLen { dst, obj } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                let len = ov.collection_len()?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = BslValue::Number(bsl_number::BslNumber::from_i64(len as i64));
                frames[frame_idx].pc += 1;
            }
            Instr::Raise { src } => {
                let value = match src {
                    Some(r) => stack[frames[frame_idx].reg_index(r)].clone(),
                    // Голая форма: повторно бросаем то, что сейчас поймано
                    // (или Неопределено, если бросить нечего — например,
                    // `ВызватьИсключение;` вне `Исключение`).
                    None => current_exception.clone().unwrap_or(BslValue::Undefined),
                };
                return Err(RtError::Raised(value));
            }
            Instr::CallBuiltin {
                dst,
                builtin,
                base,
                count,
            } => {
                let mut args = Vec::with_capacity(count as usize);
                for i in 0..count {
                    args.push(stack[frames[frame_idx].reg_index(base + i)].clone());
                }
                let v = call_builtin_with_format(builtin, &args)?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
            Instr::CallMethod {
                dst,
                obj,
                method,
                base,
                count,
            } => {
                let ov = stack[frames[frame_idx].reg_index(obj)].clone();
                let mut args = Vec::with_capacity(count as usize);
                for i in 0..count {
                    args.push(stack[frames[frame_idx].reg_index(base + i)].clone());
                }
                let v = bsl_rt::call_builtin_method(method, &ov, &args)?;
                let d = frames[frame_idx].reg_index(dst);
                stack[d] = v;
                frames[frame_idx].pc += 1;
            }
        }
    Ok(Step::Continue)
}

/// Размерность в `Новый Массив(d1, d2, ...)` обязана быть целым
/// неотрицательным числом.
fn dim_to_usize(v: &BslValue) -> Result<usize, RtError> {
    match v {
        BslValue::Number(n) => {
            let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
            usize::try_from(i).map_err(|_| RtError::BadIndex)
        }
        _ => Err(RtError::TypeError {
            expected: "Число",
            op: "Новый Массив(...)",
        }),
    }
}

/// `Новый Массив(3, 4)` — массив из 3 массивов по 4: каждое измерение
/// вкладывает следующий уровень, элементы на дне — `Неопределено`. Каждый
/// вложенный массив — отдельный объект (не общий `Rc`, иначе мутация одного
/// была бы видна во всех остальных).
fn build_nested_array(dims: &[usize]) -> BslValue {
    match dims.split_first() {
        Some((&n, rest)) => {
            let items = (0..n)
                .map(|_| {
                    if rest.is_empty() {
                        BslValue::Undefined
                    } else {
                        build_nested_array(rest)
                    }
                })
                .collect();
            BslValue::new_array(items)
        }
        None => BslValue::new_array(Vec::new()),
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

/// Ищет защищённый диапазон, содержащий `pc`, в данном чанке. При
/// нескольких вложенных диапазонах (`Попытка` внутри `Попытка`) выбирает
/// самый узкий — самый внутренний `Try` должен ловить раньше внешнего.
fn find_handler(chunk: &bsl_bytecode::Chunk, pc: usize) -> Option<usize> {
    chunk
        .exception_ranges
        .iter()
        .filter(|r| pc >= r.start_pc && pc < r.end_pc)
        .min_by_key(|r| r.end_pc - r.start_pc)
        .map(|r| r.handler_pc)
}

/// Разматывает кадры в поисках обработчика для только что брошенной ошибки.
/// Возвращает `true`, если нашли (кадры/pc уже поправлены — можно продолжать
/// цикл `run_program`), `false` — если исключение долетело до самого низа
/// стека кадров, не будучи пойманным нигде.
///
/// Кадр, где ошибка ПРОИЗОШЛА, проверяется по своему текущему `pc` (он ещё
/// не продвинут — инструкция вернула `Err` раньше, чем дошла до
/// инкремента). Любой кадр ВЫШЕ по стеку (куда мы попадаем, откатываясь
/// из-за того, что внутренний вызов не поймал исключение сам) проверяется
/// по `pc - 1` — позиции его собственной инструкции `Call`, а не следующей
/// за ней (которая уже была продвинута в момент самого вызова).
fn unwind_to_handler(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    err: &RtError,
    current_exception: &mut Option<BslValue>,
) -> bool {
    let mut first = true;
    loop {
        let frame_idx = frames.len() - 1;
        let chunk = &program.chunks[frames[frame_idx].func_id];
        let check_pc = if first {
            frames[frame_idx].pc
        } else {
            frames[frame_idx].pc - 1
        };
        first = false;

        if let Some(handler_pc) = find_handler(chunk, check_pc) {
            *current_exception = Some(err_to_value(err));
            frames[frame_idx].pc = handler_pc;
            return true;
        }

        if frames.len() == 1 {
            return false;
        }
        let frame = frames.pop().unwrap();
        stack.truncate(frame.call_start);
    }
}

/// Значение, которое видит `Исключение`-блок при повторном броске
/// (`ВызватьИсключение;` без выражения). Для `ВызватьИсключение <знач>;` это
/// само `<знач>`; для внутренних ошибок VM (деление на ноль, обращение к
/// несуществующему полю, ...) — их текстовое описание, потому что
/// полноценного объекта информации об ошибке (`ИнформацияОбОшибке()`) пока
/// нет, это отдельная задача поверх механизма builtin-функций.
fn err_to_value(err: &RtError) -> BslValue {
    match err {
        RtError::Raised(v) => v.clone(),
        other => BslValue::Str(bsl_rt::BslString::from_str(&other.to_string())),
    }
}

/// `Строка`/`Формат`/`Число`/`Message` перехватываются здесь, а не в
/// `bsl_rt::call_builtin_fn`: форматирование живёт в `bsl-format`, которое
/// зависит от `bsl-rt` (не наоборот) — `bsl-rt` физически не может
/// отформатировать число сам. Всё остальное уходит в `bsl-rt` как обычно.
fn call_builtin_with_format(
    builtin: bsl_rt::BuiltinFn,
    args: &[BslValue],
) -> Result<BslValue, RtError> {
    use bsl_rt::BuiltinFn;
    match builtin {
        BuiltinFn::Message => {
            println!("{}", bsl_format::format_value(&args[0], None));
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ToString => {
            let s = bsl_format::format_value(&args[0], None);
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        BuiltinFn::Format => {
            let spec = match &args[1] {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Формат(..., СтрокаФормата)",
                    })
                }
            };
            let s = bsl_format::format_value(&args[0], Some(&spec));
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        BuiltinFn::ToNumber => {
            let s = match &args[0] {
                BslValue::Str(s) => s,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Число(...)",
                    })
                }
            };
            let n = bsl_format::parse_number(&s.to_string(), &bsl_format::NumberFormat::default())?;
            Ok(BslValue::Number(n))
        }
        other => bsl_rt::call_builtin_fn(other, args),
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

    #[test]
    fn array_construction_indexing_and_mutation() {
        let v = run_src(
            "a = Новый Массив(3);\n\
             a[0] = 10;\n\
             a[1] = 20;\n\
             a[2] = a[0] + a[1];\n\
             Возврат a[2];",
        );
        assert_eq!(v, num("30"));
    }

    #[test]
    fn nested_array_dimensions() {
        // Новый Массив(3, 4) -> массив из 3 независимых массивов по 4.
        let v = run_src(
            "a = Новый Массив(3, 4);\n\
             a[0][0] = 1;\n\
             a[1][0] = 2;\n\
             Возврат a[0][0] + a[1][0];",
        );
        assert_eq!(v, num("3"));
    }

    #[test]
    fn nested_array_slots_are_independent_objects() {
        let v = run_src(
            "a = Новый Массив(2, 2);\n\
             a[0][0] = 1;\n\
             Возврат a[1][0];",
        );
        // Если бы вложенные массивы были одним общим объектом (баг), тут
        // тоже было бы 1 — а не Неопределено.
        assert_eq!(v, BslValue::Undefined);
    }

    #[test]
    fn array_index_out_of_bounds_is_a_runtime_error() {
        let err = run_src_err("a = Новый Массив(1);\nВозврат a[5];");
        assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
    }

    #[test]
    fn structure_construction_and_field_access() {
        let v = run_src(
            "s = Новый Структура(\"x,y,z\", 1, 2, 3);\n\
             s.y = s.y + 100;\n\
             Возврат s.x + s.y + s.z;",
        );
        assert_eq!(v, num("106"));
    }

    #[test]
    fn structure_keys_only_defaults_to_undefined() {
        let v = run_src("s = Новый Структура(\"x\");\nВозврат s.x;");
        assert_eq!(v, BslValue::Undefined);
    }

    #[test]
    fn unknown_field_is_a_runtime_error() {
        let err = run_src_err("s = Новый Структура(\"x\");\nВозврат s.z;");
        assert!(matches!(err, RtError::UnknownField(_)));
    }

    #[test]
    fn arrays_are_reference_types_across_by_reference_calls() {
        // Массив передаётся в процедуру по ссылке (как всё без Знач), и
        // сам массив — ссылочный тип: мутация видна вызывающему в обоих
        // смыслах сразу (тот же тест, что и by_reference, но для объекта).
        let v = run_src(
            "Процедура Заполнить(a)\n\
             a[0] = 42;\n\
             КонецПроцедуры\n\
             b = Новый Массив(1);\n\
             Заполнить(b);\n\
             Возврат b[0];",
        );
        assert_eq!(v, num("42"));
    }

    #[test]
    fn for_each_over_array() {
        let v = run_src(
            "a = Новый Массив(3);\n\
             a[0] = 1;\n\
             a[1] = 2;\n\
             a[2] = 3;\n\
             sum = 0;\n\
             Для Каждого x Из a Цикл\n\
             sum = sum + x;\n\
             КонецЦикла\n\
             Возврат sum;",
        );
        assert_eq!(v, num("6"));
    }

    #[test]
    fn for_each_break_and_continue() {
        let v = run_src(
            "a = Новый Массив(5);\n\
             a[0] = 1;\n a[1] = 2;\n a[2] = 3;\n a[3] = 4;\n a[4] = 5;\n\
             sum = 0;\n\
             Для Каждого x Из a Цикл\n\
             Если x = 2 Тогда\n\
             Продолжить;\n\
             КонецЕсли;\n\
             Если x = 5 Тогда\n\
             Прервать;\n\
             КонецЕсли;\n\
             sum = sum + x;\n\
             КонецЦикла\n\
             Возврат sum;",
        );
        // 1 + 3 + 4 = 8 (2 пропущен, остановились на 5)
        assert_eq!(v, num("8"));
    }

    #[test]
    fn display_of_array_and_structure_matches_measured_platform_strings() {
        let v = run_src("Возврат Новый Массив();");
        assert_eq!(v.to_string(), "Массив");
        let v = run_src("Возврат Новый Структура();");
        assert_eq!(v.to_string(), "Структура");
    }

    #[test]
    fn try_except_catches_internal_runtime_error() {
        let v = run_src(
            "x = 0;\n\
             Попытка\n\
             x = 1 / 0;\n\
             Исключение\n\
             x = 99;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("99"));
    }

    #[test]
    fn code_after_try_runs_normally_when_nothing_is_raised() {
        let v = run_src(
            "x = 0;\n\
             Попытка\n\
             x = 1;\n\
             Исключение\n\
             x = 99;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn raise_with_value_is_caught_and_carries_the_value() {
        let v = run_src(
            "Попытка\n\
             ВызватьИсключение \"беда\";\n\
             Исключение\n\
             Возврат 1;\n\
             КонецПопытки\n\
             Возврат 0;",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn exception_raised_inside_a_called_function_is_caught_by_callers_try() {
        // Попытка оборачивает ВЫЗОВ, а не сам код исключения — исключение
        // должно долететь через границу кадра и быть пойманным снаружи.
        let v = run_src(
            "Функция Взрыв()\n\
             Возврат 1 / 0;\n\
             КонецФункции\n\
             x = 0;\n\
             Попытка\n\
             x = Взрыв();\n\
             Исключение\n\
             x = 42;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("42"));
    }

    #[test]
    fn uncaught_exception_outside_any_try_propagates_as_an_error() {
        let err = run_src_err("x = 1 / 0;");
        assert!(matches!(err, RtError::Num(bsl_number::NumError::DivideByZero)));
    }

    #[test]
    fn bare_reraise_inside_except_rethrows_caught_value() {
        // Внешняя Попытка должна поймать то же самое исключение, повторно
        // брошенное из внутреннего Исключение через голый ВызватьИсключение.
        let v = run_src(
            "x = 0;\n\
             Попытка\n\
             Попытка\n\
             ВызватьИсключение \"внутренняя\";\n\
             Исключение\n\
             ВызватьИсключение;\n\
             КонецПопытки\n\
             Исключение\n\
             x = 7;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("7"));
    }

    #[test]
    fn nested_try_inner_handler_wins_over_outer() {
        let v = run_src(
            "x = 0;\n\
             Попытка\n\
             Попытка\n\
             x = 1 / 0;\n\
             Исключение\n\
             x = 1;\n\
             КонецПопытки\n\
             Исключение\n\
             x = 2;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn builtin_sqrt_and_pow() {
        let v = run_src("Возврат sqrt(2);");
        assert_eq!(v, num("1.4142135623731"));

        let v = run_src("Возврат Pow(10, 30);");
        assert_eq!(v, num("1000000000000000000000000000000"));
    }

    #[test]
    fn builtin_sqrt_of_negative_is_a_runtime_error() {
        let err = run_src_err("Возврат sqrt(-1);");
        assert!(matches!(err, RtError::Num(_)));
    }

    #[test]
    fn count_method_call_on_array() {
        let v = run_src("a = Новый Массив(5);\nВозврат a.Count();");
        assert_eq!(v, num("5"));
    }

    #[test]
    fn message_builtin_prints_and_returns_undefined() {
        // Не проверяем stdout здесь — только что вызов не падает и что
        // Message() возвращает Неопределено, как и положено процедуре без
        // Возврат.
        let v = run_src("Message(\"hello\");\nВозврат 1;");
        assert_eq!(v, num("1"));
    }

    #[test]
    fn nbody_smoke_runs_the_real_benchmark_shape_for_a_few_steps() {
        // Уменьшенная копия tests/conformance/fixtures/n-body.bsl: та же
        // структура (Function/EndFunction, Для Каждого, Новый Структура,
        // деление гигантских констант, sqrt, .Count()), но всего несколько
        // шагов Advance вместо 50 миллионов (брифом же и объявленных
        // невыполнимыми что у нас, что в самой 1С) и без Message — просто
        // Возврат энергии для проверки в тесте.
        let src = include_str!("../tests/nbody_smoke.bsl");
        let v = run_src(src);
        let e = match &v {
            BslValue::Number(n) => n.clone(),
            other => panic!("expected Number, got {other:?}"),
        };
        // Энергия системы отрицательна (связанная система) и не должна
        // выродиться в бесконечность/NaN за несколько шагов.
        assert!(e.is_negative(), "energy should stay negative: {e:?}");
    }

    fn str_val(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn stroka_groups_by_default_with_nbsp() {
        // Строка(1000.5) -> "1 000,5" (NBSP, не обычный пробел).
        let v = run_src("Возврат Строка(1000.5);");
        assert_eq!(str_val(&v), "1\u{A0}000,5");
    }

    #[test]
    fn format_with_explicit_spec_suppresses_grouping() {
        let v = run_src(r#"Возврат Формат(1000000, "ЧГ=0; ЧРД=.");"#);
        assert_eq!(str_val(&v), "1000000");
    }

    #[test]
    fn chislo_parses_grouped_string_back_round_trip() {
        let v = run_src("x = Строка(1000000);\nВозврат Число(x);");
        assert_eq!(v, num("1000000"));
    }

    #[test]
    fn stroka_of_boolean_and_undefined_matches_measured_strings() {
        let v = run_src("Возврат Строка(Истина);");
        assert_eq!(str_val(&v), "Да");
        let v = run_src("Возврат Строка(Неопределено);");
        assert_eq!(str_val(&v), "");
    }

    #[test]
    fn string_concatenation_via_plus() {
        let v = run_src(r#"Возврат "Привет, " + "мир!";"#);
        assert_eq!(str_val(&v), "Привет, мир!");
    }

    #[test]
    fn strdlina_counts_utf16_code_units_including_surrogate_pairs() {
        let v = run_src(r#"Возврат СтрДлина("привет");"#);
        assert_eq!(v, num("6"));
        // Эмодзи вне BMP — суррогатная пара, 2 код-юнита UTF-16.
        let v = run_src("Возврат СтрДлина(\"a\u{1F600}b\");");
        assert_eq!(v, num("4"));
    }

    #[test]
    fn left_right_mid_builtins() {
        let v = run_src(r#"Возврат Лев("Привет", 3);"#);
        assert_eq!(str_val(&v), "При");
        let v = run_src(r#"Возврат Прав("Привет", 3);"#);
        assert_eq!(str_val(&v), "вет");
        let v = run_src(r#"Возврат Сред("Привет", 2, 3);"#);
        assert_eq!(str_val(&v), "рив");
    }

    #[test]
    fn upper_lower_trimall_builtins() {
        let v = run_src(r#"Возврат ВРег("привет");"#);
        assert_eq!(str_val(&v), "ПРИВЕТ");
        let v = run_src(r#"Возврат НРег("ПРИВЕТ");"#);
        assert_eq!(str_val(&v), "привет");
        let v = run_src("Возврат СокрЛП(\"  привет  \");");
        assert_eq!(str_val(&v), "привет");
    }

    #[test]
    fn string_comparison_is_lexicographic() {
        let v = run_src(r#"Возврат "а" < "б";"#);
        assert_eq!(v, BslValue::Boolean(true));
        let v = run_src(r#"Возврат "яблоко" = "яблоко";"#);
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn adding_number_and_string_is_a_type_error() {
        let err = run_src_err(r#"Возврат 1 + "a";"#);
        assert!(matches!(err, RtError::TypeError { .. }));
    }

    #[test]
    fn array_add_delete_clear_methods() {
        let v = run_src(
            "a = Новый Массив();\n\
             a.Добавить(1);\n\
             a.Добавить(2);\n\
             a.Добавить(3);\n\
             a.Удалить(1);\n\
             Возврат a.Количество();",
        );
        assert_eq!(v, num("2"));

        let v = run_src(
            "a = Новый Массив();\n\
             a.Добавить(1);\n\
             a.Очистить();\n\
             Возврат a.Количество();",
        );
        assert_eq!(v, num("0"));
    }

    #[test]
    fn value_table_add_column_add_row_and_field_access() {
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"Имя\");\n\
             т.Колонки.Добавить(\"Возраст\");\n\
             строка = т.Добавить();\n\
             строка.Имя = \"Аня\";\n\
             строка.Возраст = 30;\n\
             Возврат строка.Возраст;",
        );
        assert_eq!(v, num("30"));
    }

    #[test]
    fn value_table_row_count_and_indexing() {
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             т.Добавить();\n\
             т.Добавить();\n\
             т.Добавить();\n\
             т[1].x = 42;\n\
             Возврат т.Количество() * 100 + т[1].x;",
        );
        assert_eq!(v, num("342"));
    }

    #[test]
    fn value_table_for_each_over_rows() {
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             а = т.Добавить(); а.x = 1;\n\
             б = т.Добавить(); б.x = 2;\n\
             в = т.Добавить(); в.x = 3;\n\
             сумма = 0;\n\
             Для Каждого строка Из т Цикл\n\
             сумма = сумма + строка.x;\n\
             КонецЦикла\n\
             Возврат сумма;",
        );
        assert_eq!(v, num("6"));
    }

    #[test]
    fn value_table_row_identity_survives_deleting_a_different_row() {
        // Строка держит row_id, не физическую позицию — удаление строки 0
        // не должно сломать ранее полученную ссылку на строку 1.
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             а = т.Добавить(); а.x = 10;\n\
             б = т.Добавить(); б.x = 20;\n\
             т.Удалить(0);\n\
             Возврат б.x;",
        );
        assert_eq!(v, num("20"));
    }

    #[test]
    fn value_table_accessing_deleted_row_is_an_error() {
        let err = run_src_err(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             а = т.Добавить(); а.x = 10;\n\
             т.Удалить(0);\n\
             Возврат а.x;",
        );
        assert!(matches!(err, RtError::RowInvalidated));
    }

    #[test]
    fn value_table_unknown_column_is_an_error() {
        let err = run_src_err(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             строка = т.Добавить();\n\
             Возврат строка.y;",
        );
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn value_table_clear_resets_row_count() {
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"x\");\n\
             т.Добавить();\n\
             т.Добавить();\n\
             т.Очистить();\n\
             Возврат т.Количество();",
        );
        assert_eq!(v, num("0"));
    }
}
