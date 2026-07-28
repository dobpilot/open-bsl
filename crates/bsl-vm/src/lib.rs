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
    /// Активен только внутри доказанно пустого числового цикла. Его тело не
    /// может наблюдать счётчик, поэтому обычный `BslValue` материализуется
    /// лишь при выходе из цикла.
    numeric_for_state: Option<NumericForState>,
}

struct NumericForState {
    pc: usize,
    current: i64,
    bound: i64,
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

/// Аргументы подавляющего большинства встроенных вызовов помещаются сюда
/// без heap-аллокации. Более длинные вариативные вызовы используют `Vec`.
enum CallArgs {
    Inline {
        values: [BslValue; 3],
        len: usize,
    },
    Heap(Vec<BslValue>),
}

impl CallArgs {
    fn load(stack: &[BslValue], frame: &Frame, base: u8, count: u8) -> Result<Self, RtError> {
        if count <= 3 {
            let mut values = [
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
            ];
            for i in 0..count {
                let reg = base.checked_add(i).ok_or(RtError::InvalidBytecode(
                    "переполнение номера регистра аргумента",
                ))?;
                values[i as usize] = reg_load(stack, frame.reg_index(reg))?;
            }
            Ok(CallArgs::Inline {
                values,
                len: count as usize,
            })
        } else {
            let mut values = Vec::with_capacity(count as usize);
            for i in 0..count {
                let reg = base.checked_add(i).ok_or(RtError::InvalidBytecode(
                    "переполнение номера регистра аргумента",
                ))?;
                values.push(reg_load(stack, frame.reg_index(reg))?);
            }
            Ok(CallArgs::Heap(values))
        }
    }

    fn as_slice(&self) -> &[BslValue] {
        match self {
            CallArgs::Inline { values, len } => &values[..*len],
            CallArgs::Heap(values) => values,
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
    // Публичная точка входа — `Program` могли собрать и не нашим кодогеном
    // (это `pub fn`), поэтому даже отсутствие `chunks[0]` — ошибка, а не
    // паника.
    push_own_registers(
        &mut stack,
        at(&program.chunks, 0, "в программе нет чанка верхнего уровня")?,
    );
    let (value, _stack) = drive(program, 0, stack)?;
    Ok(value)
}

/// Для REPL (`bsl-cli`): исполняет один чанк с готовым стеком (уже
/// дополненным под `chunk.n_regs`) и возвращает и значение (`Возврат` в
/// строке, если был), и финальный стек — REPL сохраняет его целиком как
/// новое состояние сессии для следующей строки (в отличие от
/// `Выполнить`/`Вычислить` внутри уже работающего скрипта, REPL не
/// ограничен статически размеченным окружающим кадром — расти можно
/// сколько угодно, каждая строка это просто новый чанк поверх той же
/// растущей таблицы имён).
/// `locals` — имена слотов ЭТОЙ строки (все накопленные за сессию, включая
/// новые в этой строке) в порядке, СОВПАДАЮЩЕМ с раскладкой регистров
/// `chunk` (он был скомпилирован именно с этим списком как `all_locals`,
/// см. `bsl_bytecode::compile_snippet`). Передаётся дальше как
/// `Program::top_level_locals`, чтобы `Выполнить`/`Вычислить`, вызванные
/// ИЗНУТРИ этой строки, могли резолвить переменные REPL-сессии на те же
/// слоты — без этого они не видели бы вообще ничего (список был бы пуст)
/// и завели бы каждое имя как новую, независимую переменную.
pub fn run_repl_chunk(
    chunk: &bsl_bytecode::Chunk,
    names: Vec<String>,
    shapes: Vec<std::rc::Rc<bsl_rt::Shape>>,
    locals: Vec<String>,
    stack: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = Program {
        chunks: vec![chunk.clone()],
        names,
        shapes,
        top_level_locals: locals,
    };
    drive(&program, 0, stack)
}

/// Выполняет `program.chunks[func_id]` с нуля, используя `stack` как
/// начальное содержимое регистров (уже дополненное/подготовленное
/// вызывающим), и возвращает и значение, и финальный стек — нужен
/// `run_isolated` для `Выполнить`/`Вычислить`, которому важно прочитать
/// состояние регистров ПОСЛЕ завершения, а не только значение `Возврат`.
fn drive(
    program: &Program,
    func_id: usize,
    mut stack: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let mut frames = vec![Frame {
        func_id,
        pc: 0,
        param_aliases: Vec::new(),
        own_base: 0,
        call_start: 0,
        return_reg: 0,
        numeric_for_state: None,
    }];
    let mut current_exception: Option<BslValue> = None;
    // Кэш скомпилированных фрагментов `Выполнить`/`Вычислить` на всё это
    // исполнение — см. `SnippetCache`.
    let mut snippets: SnippetCache = SnippetCache::new();
    // Затравлена формами/именами ЭТОЙ программы — см. `bsl_rt::RuntimeShapes`
    // doc comment про то, почему не общий на процесс синглтон: у вложенного
    // `Program` (см. `run_dynamic_snippet`) свои `names`/`shapes`, и рантайм-
    // расширения этой таблицы (`Вставить`/`Удалить` на структуре, меняющие
    // её форму) актуальны только для объектов внутри ОДНОГО такого вызова.
    let mut runtime_shapes = bsl_rt::RuntimeShapes::seeded(program.names.clone(), program.shapes.clone());

    loop {
        // После инициализации пустой numeric-for не обращается к
        // регистрам. Обслуживаем его back-edge в компактном внешнем цикле,
        // не входя на каждой итерации в большой универсальный `step`.
        // Логических итераций по-прежнему столько же: цикл не сворачивается
        // в вычисление финального значения.
        let fast_numeric_for = {
            let frame = frames
                .last_mut()
                .expect("инвариант VM: drive всегда держит хотя бы один кадр");
            match frame.numeric_for_state.as_mut() {
                Some(state) if state.pc == frame.pc => {
                    match state.current.checked_add(1) {
                        Some(next) if next <= state.bound => {
                            state.current = next;
                            true
                        }
                        _ => false,
                    }
                }
                _ => false,
            }
        };
        if fast_numeric_for {
            continue;
        }

        match step(
            &mut frames,
            &mut stack,
            program,
            &mut current_exception,
            &mut runtime_shapes,
            &mut snippets,
        ) {
            Ok(Step::Continue) => continue,
            Ok(Step::Done(v)) => return Ok((v, stack)),
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

// --- Доступ к байт-коду и регистрам ------------------------------------
//
// Ни одна паника в этом модуле не должна зависеть от ВХОДНЫХ ДАННЫХ. Всё,
// что инструкция читает по индексу — номер регистра, номер чанка,
// константы, формы, имени, аргумент builtin'а, — приходит из `Program`, а
// `Program` VM получает не только от собственного кодогена: её собирает
// `Выполнить`/`Вычислить` в рантайме, REPL `bsl-cli` по строке, и любой
// внешний пользователь публичных `run_program`/`run_repl_chunk`. Поэтому
// такие обращения дают `RtError::InvalidBytecode`, а не роняют процесс.
//
// Остаются ровно два `expect()` — на `frames.pop()` в
// `do_return_with_value` и `unwind_to_handler`: это внутренние инварианты
// самого цикла диспетчеризации (стек кадров непуст, пока мы исполняем
// инструкцию), недостижимые никаким байт-кодом. Голых `unwrap()` вне
// тестов нет ни одного.

#[inline]
fn at<'a, T>(xs: &'a [T], i: usize, what: &'static str) -> Result<&'a T, RtError> {
    xs.get(i).ok_or(RtError::InvalidBytecode(what))
}

#[inline]
fn reg_load(stack: &[BslValue], i: usize) -> Result<BslValue, RtError> {
    stack
        .get(i)
        .cloned()
        .ok_or(RtError::InvalidBytecode("чтение регистра за границей стека значений"))
}

#[inline]
fn reg_store(stack: &mut [BslValue], i: usize, v: BslValue) -> Result<(), RtError> {
    match stack.get_mut(i) {
        Some(slot) => {
            *slot = v;
            Ok(())
        }
        None => Err(RtError::InvalidBytecode(
            "запись регистра за границей стека значений",
        )),
    }
}

/// Одновременно заимствует изменяемый счётчик и неизменяемую границу без
/// клонирования `BslValue` на каждой итерации числового цикла.
#[inline]
fn reg_pair_mut(
    stack: &mut [BslValue],
    mutable: usize,
    other: usize,
) -> Result<(&mut BslValue, &BslValue), RtError> {
    if mutable == other {
        return Err(RtError::InvalidBytecode(
            "счётчик и граница числового цикла используют один регистр",
        ));
    }
    if mutable >= stack.len() {
        return Err(RtError::InvalidBytecode(
            "счётчик числового цикла вне стека значений",
        ));
    }
    if other >= stack.len() {
        return Err(RtError::InvalidBytecode(
            "граница числового цикла вне стека значений",
        ));
    }
    if mutable < other {
        let (left, right) = stack.split_at_mut(other);
        Ok((&mut left[mutable], &right[0]))
    } else {
        let (left, right) = stack.split_at_mut(mutable);
        Ok((&mut right[0], &left[other]))
    }
}

#[inline]
fn numeric_for_next_regular(
    stack: &mut [BslValue],
    counter: usize,
    bound: usize,
    pc: &mut usize,
    target: i16,
) -> Result<(), RtError> {
    let (counter, bound) = reg_pair_mut(stack, counter, bound)?;
    if counter.increment_numeric_for_and_le(bound)? {
        *pc = target as usize;
    } else {
        *pc += 1;
    }
    Ok(())
}

/// Ячейка инлайн-кэша, отведённая под инструкцию на позиции `pc`.
/// `prop_cache` кодоген заводит длиной со всем `instrs`, но чанк мог
/// прийти и не от него.
#[inline]
fn prop_cache(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
) -> Result<&std::cell::RefCell<Option<(std::rc::Rc<bsl_rt::Shape>, u32)>>, RtError> {
    at(&chunk.prop_cache, pc, "нет ячейки инлайн-кэша для инструкции")
}

/// Оригинальное написание имени поля — нужно строковому пути доступа
/// (`СтрокаТаблицыЗначений`, `КлючИЗначение`), у которого нет формы.
#[inline]
fn field_name(program: &Program, name: bsl_rt::NameId) -> Result<&str, RtError> {
    at(&program.names, name.index(), "идентификатор имени вне таблицы имён программы")
        .map(|s| s.as_str())
}

/// Выполняет РОВНО одну инструкцию текущего (верхнего) кадра.
fn step(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    current_exception: &mut Option<BslValue>,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    snippets: &mut SnippetCache,
) -> Result<Step, RtError> {
    let frame_idx = frames.len() - 1;
    let func_id = frames[frame_idx].func_id;
    let pc = frames[frame_idx].pc;
    let chunk = at(&program.chunks, func_id, "номер чанка вне таблицы функций")?;

    if pc >= chunk.instrs.len() {
        // Неявный возврат: тело кончилось без `Возврат` — результат
        // Неопределено, как и `Возврат;` без выражения.
        return Ok(match do_return_with_value(frames, stack, BslValue::Undefined)? {
            Done(v) => Step::Done(v),
            Continuing => Step::Continue,
        });
    }

    // `pc < chunk.instrs.len()` проверено сразу выше — индексация здесь
    // уже не может выйти за границы.
    let instr = chunk.instrs[pc];
    match instr {
            Instr::Move { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadConst { dst, k } => {
                let v = at(&chunk.consts, k as usize, "номер константы вне таблицы констант чанка")?.clone();
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadBool { dst, val } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(val))?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadUndefined { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Undefined)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadNull { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Null)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadSkipped { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Skipped)?;
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
            Instr::Neg { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?.neg()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Not { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?.not()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Eq { dst, a, b } => {
                let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
                let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(av.eq_value(&bv)))?;
                frames[frame_idx].pc += 1;
            }
            Instr::NotEq { dst, a, b } => {
                let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
                let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Boolean(!av.eq_value(&bv)))?;
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
                if reg_load(stack, c)?.as_condition()? {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::JumpIfTrue { cond, target } => {
                let c = frames[frame_idx].reg_index(cond);
                if reg_load(stack, c)?.as_condition()? {
                    frames[frame_idx].pc = target as usize;
                } else {
                    frames[frame_idx].pc += 1;
                }
            }
            Instr::JumpIfNotSkipped { src, target } => {
                let s = frames[frame_idx].reg_index(src);
                // Не `as_condition`/строгая булевость — это не условие
                // пользовательского кода, а проверка внутреннего маркера
                // (см. пролог параметров по умолчанию в
                // `bsl-bytecode::compiler::compile_param_defaults`).
                if matches!(reg_load(stack, s)?, BslValue::Skipped) {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::NumericForNext {
                counter,
                bound,
                target,
            } => {
                let counter = frames[frame_idx].reg_index(counter);
                let bound = frames[frame_idx].reg_index(bound);
                numeric_for_next_regular(
                    stack,
                    counter,
                    bound,
                    &mut frames[frame_idx].pc,
                    target,
                )?;
            }
            Instr::NumericForNextI64 {
                counter,
                bound,
                target,
            } => {
                let counter_idx = frames[frame_idx].reg_index(counter);
                let bound_idx = frames[frame_idx].reg_index(bound);
                let state = match frames[frame_idx].numeric_for_state.take() {
                    Some(state) if state.pc == pc => state,
                    Some(_) => {
                        return Err(RtError::InvalidBytecode(
                            "перекрывающиеся скрытые состояния числовых циклов",
                        ))
                    }
                    None => {
                        let counter_value = reg_load(stack, counter_idx)?;
                        let bound_value = reg_load(stack, bound_idx)?;
                        let pair = match (&counter_value, &bound_value) {
                            (BslValue::Number(counter), BslValue::Number(bound)) => {
                                counter.to_i64_exact().zip(bound.to_i64_exact())
                            }
                            _ => None,
                        };
                        let Some((current, bound)) = pair else {
                            numeric_for_next_regular(
                                stack,
                                counter_idx,
                                bound_idx,
                                &mut frames[frame_idx].pc,
                                target,
                            )?;
                            return Ok(Step::Continue);
                        };
                        NumericForState { pc, current, bound }
                    }
                };

                let Some(next) = state.current.checked_add(1) else {
                    reg_store(
                        stack,
                        counter_idx,
                        BslValue::Number(bsl_number::BslNumber::from_i64(state.current)),
                    )?;
                    numeric_for_next_regular(
                        stack,
                        counter_idx,
                        bound_idx,
                        &mut frames[frame_idx].pc,
                        target,
                    )?;
                    return Ok(Step::Continue);
                };
                if next <= state.bound {
                    frames[frame_idx].numeric_for_state = Some(NumericForState {
                        current: next,
                        ..state
                    });
                    frames[frame_idx].pc = target as usize;
                } else {
                    reg_store(
                        stack,
                        counter_idx,
                        BslValue::Number(bsl_number::BslNumber::from_i64(next)),
                    )?;
                    frames[frame_idx].pc += 1;
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

                let modes = at(&chunk.call_arg_modes, arg_modes as usize, "номер набора режимов аргументов вне таблицы чанка")?;
                let mut param_aliases = Vec::with_capacity(modes.len());
                for (i, mode) in modes.iter().enumerate() {
                    let idx = match mode {
                        ArgMode::Value => frames[frame_idx].reg_index(base + i as u8),
                        ArgMode::ByRefLocal(slot) => frames[frame_idx].reg_index(*slot),
                    };
                    param_aliases.push(idx);
                }

                let callee_chunk = at(&program.chunks, func as usize, "номер вызываемого чанка вне таблицы функций")?;
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
                    numeric_for_state: None,
                });
            }
            Instr::Return { src } => {
                let value = match src {
                    Some(r) => {
                        let idx = frames[frame_idx].reg_index(r);
                        reg_load(stack, idx)?
                    }
                    None => BslValue::Undefined,
                };
                return Ok(match do_return_with_value(frames, stack, value)? {
                    Done(v) => Step::Done(v),
                    Continuing => Step::Continue,
                });
            }
            Instr::GetIndex { dst, obj, idx } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let iv = reg_load(stack, frames[frame_idx].reg_index(idx))?;
                let v = ov.get_index(&iv, &runtime_shapes.names)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetIndex { obj, idx, src } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let iv = reg_load(stack, frames[frame_idx].reg_index(idx))?;
                let sv = reg_load(stack, frames[frame_idx].reg_index(src))?;
                ov.set_index(&iv, sv)?;
                frames[frame_idx].pc += 1;
            }
            Instr::GetProp { dst, obj, name } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                // Структура резолвится через инлайн-кэш этой ИНСТРУКЦИИ
                // (см. Chunk::prop_cache): мономорфный сайт вызова после
                // первого попадания читает слот напрямую, без HashMap-
                // поиска в Shape::index. СтрокаТаблицыЗначений заводит
                // колонки в рантайме и не могла быть интернирована на
                // этапе компиляции — для неё (и только когда кэш-путь
                // говорит "это не такой объект") VM резолвит имя в текст
                // через Program::names и идёт по строковому пути.
                let v = match ov.get_field_cached(name, prop_cache(chunk, pc)?) {
                    Err(RtError::NotAnObject) => ov.get_field_by_name(field_name(program, name)?)?,
                    other => other?,
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetProp { obj, name, src } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let sv = reg_load(stack, frames[frame_idx].reg_index(src))?;
                match ov.set_field_cached(name, sv.clone(), prop_cache(chunk, pc)?) {
                    Err(RtError::NotAnObject) => {
                        ov.set_field_by_name(field_name(program, name)?, sv)?
                    }
                    other => other?,
                }
                frames[frame_idx].pc += 1;
            }
            Instr::NewArray { dst, base, count } => {
                let mut dims = Vec::with_capacity(count as usize);
                for i in 0..count {
                    let v = reg_load(stack, frames[frame_idx].reg_index(base + i))?;
                    dims.push(dim_to_usize(&v)?);
                }
                let arr = build_nested_array(&dims);
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, arr)?;
                frames[frame_idx].pc += 1;
            }
            Instr::NewStructure {
                dst,
                shape,
                base,
                count,
            } => {
                let shape_rc = at(&program.shapes, shape as usize, "номер формы вне таблицы форм программы")?.clone();
                let mut slots = Vec::with_capacity(count as usize);
                for i in 0..count {
                    slots.push(reg_load(stack, frames[frame_idx].reg_index(base + i))?);
                }
                let v = BslValue::new_structure(shape_rc, slots);
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::NewTable { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::new_table())?;
                frames[frame_idx].pc += 1;
            }
            Instr::NewMap { dst } => {
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::new_map())?;
                frames[frame_idx].pc += 1;
            }
            Instr::NewTextWriter { dst, path } => {
                let path = reg_load(stack, frames[frame_idx].reg_index(path))?;
                let writer = BslValue::new_text_writer(&path)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, writer)?;
                frames[frame_idx].pc += 1;
            }
            Instr::CollectionLen { dst, obj } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let len = ov.collection_len()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, BslValue::Number(bsl_number::BslNumber::from_i64(len as i64)))?;
                frames[frame_idx].pc += 1;
            }
            Instr::Raise { src } => {
                let value = match src {
                    Some(r) => reg_load(stack, frames[frame_idx].reg_index(r))?,
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
                let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                let v = call_builtin_with_format(builtin, args.as_slice())?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::CallMethod {
                dst,
                obj,
                method,
                base,
                count,
            } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                let v =
                    bsl_rt::call_builtin_method_ctx(method, &ov, args.as_slice(), runtime_shapes)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::WriteText { dst, obj, src } => {
                let obj_idx = frames[frame_idx].reg_index(obj);
                let src_idx = frames[frame_idx].reg_index(src);
                let v = {
                    let ov = at(stack, obj_idx, "чтение объекта за границей стека значений")?;
                    let sv = at(stack, src_idx, "чтение аргумента за границей стека значений")?;
                    ov.text_writer_write(sv)?
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::CloseText { dst, obj } => {
                let obj_idx = frames[frame_idx].reg_index(obj);
                let v = at(stack, obj_idx, "чтение объекта за границей стека значений")?
                    .text_writer_close()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::RunDynamic { src, dst, is_eval } => {
                let code = reg_load(stack, frames[frame_idx].reg_index(src))?;
                let code = match code {
                    BslValue::Str(s) => s.to_string(),
                    _ => {
                        return Err(RtError::TypeError {
                            expected: "Строка",
                            op: if is_eval { "Вычислить" } else { "Выполнить" },
                        })
                    }
                };
                // Область видимости фрагмента — материализованная таблица
                // имён ЭТОГО кадра (`Chunk::local_names`), а не только
                // верхнего уровня: `Выполнить` внутри процедуры видит её
                // локальные. Таблица есть у всех чанков, помеченных
                // `uses_dynamic` в `bsl-sema`, а `RunDynamic` эмитится
                // только в них — так что пустой она здесь быть не может,
                // кроме как у кадра вообще без локальных переменных.
                let value = run_dynamic_snippet(
                    &code,
                    is_eval,
                    program,
                    &chunk.local_names,
                    func_id,
                    stack,
                    &frames[frame_idx],
                    snippets,
                )?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, value)?;
                frames[frame_idx].pc += 1;
            }
        }
    Ok(Step::Continue)
}

/// Компилирует и исполняет `code` в контексте top-level переменных
/// программы (см. `Instr::RunDynamic`). `is_eval` заворачивает `code` в
/// `Возврат (...)`, чтобы получить значение выражения тем же путём, что
/// и обычный оператор `Возврат` — общий движок для `Выполнить`/`Вычислить`
/// без раздвоения семантики.
///
/// Изолированность: фрагмент исполняется на ОТДЕЛЬНОМ стеке — копии
/// текущих значений top-level переменных плюс новые слоты под то, что
/// фрагмент сам объявит. После исполнения обратно во внешний кадр
/// переносятся только значения уже существовавших до вызова слотов
/// (`0..old_count`) — их регистровые номера точно совпадают с тем, что
/// уже использует статический код вокруг, так что перезапись безопасна.
/// Новые имена, объявленные фрагментом, никуда не переносятся: чтобы это
/// сделать, статический код вокруг пришлось бы компилировать в режиме
/// материализованного кадра с именной таблицей (см. бриф) — это
/// отдельная, ещё не сделанная работа.
#[allow(clippy::too_many_arguments)]
fn run_dynamic_snippet(
    code: &str,
    is_eval: bool,
    program: &Program,
    scope_locals: &[String],
    scope_id: usize,
    stack: &mut [BslValue],
    frame: &Frame,
    snippets: &mut SnippetCache,
) -> Result<BslValue, RtError> {
    let compiled = snippets.get_or_compile(code, is_eval, scope_id, scope_locals, program)?;

    // Значения существующих переменных кадра переезжают во фрагмент по
    // НОМЕРУ СЛОТА: раскладка совпадает, потому что фрагмент резолвился
    // поверх ровно этого `scope_locals` (см. `resolve_snippet_stmts`).
    // `reg_index` здесь обязателен, а не голое `stack[i]`: у кадра функции
    // параметры — алиасы на слоты ВЫЗЫВАЮЩЕГО, и параметр по ссылке
    // обязан быть виден фрагменту тем же, чем он виден статическому коду.
    let old_count = scope_locals.len();
    let mut snippet_stack: Vec<BslValue> = (0..old_count)
        .map(|i| reg_load(stack, frame.reg_index(i as u8)))
        .collect::<Result<_, _>>()?;
    snippet_stack.resize(compiled.chunk.n_regs as usize, BslValue::Undefined);

    let snippet_program = Program {
        chunks: vec![compiled.chunk.clone()],
        names: program.names.clone(),
        shapes: compiled.shapes.clone(),
        top_level_locals: Vec::new(),
    };

    let (value, final_stack) = drive(&snippet_program, 0, snippet_stack)?;

    // Обратно переносятся ТОЛЬКО уже существовавшие слоты: их номера
    // совпадают с теми, что использует окружающий скомпилированный код.
    // Имена, объявленные самим фрагментом, получили слоты ЗА `old_count` и
    // никуда не переносятся — расширить статически размеченный кадр нечем.
    //
    // НЕ ИЗМЕРЕНО(EXEC.NEW_VARIABLE_SCOPE): `Выполнить` в 1С исполняется в
    // контексте текущей области видимости, так что новое имя, возможно, в
    // ней и остаётся. Если замер это подтвердит, потребуется кадр с именной
    // таблицей, а не эта перекладка по номерам слотов.
    for i in 0..old_count {
        let d = frame.reg_index(i as u8);
        reg_store(stack, d, reg_load(&final_stack, i)?)?;
    }

    Ok(value)
}

/// Один скомпилированный фрагмент. `shapes` — СОБСТВЕННЫЙ список форм
/// фрагмента: индексы `shape` внутри `chunk` ссылаются именно на него, а
/// не на `program.shapes` (был баг ровно на этом — `NewStructure` попадал
/// по чужому индексу).
struct CompiledSnippet {
    chunk: bsl_bytecode::Chunk,
    shapes: Vec<std::rc::Rc<bsl_rt::Shape>>,
}

/// Кэш «текст фрагмента -> скомпилированный чанк» на одно исполнение
/// (`drive`). Без него `Выполнить(...)` в цикле заново лексирует, парсит,
/// резолвит и компилирует одну и ту же строку на каждой итерации.
///
/// Ключ — пара (ОБЛАСТЬ ВИДИМОСТИ, текст), а не один текст: один и тот же
/// исходник, выполненный в разных кадрах, резолвится в РАЗНЫЕ номера
/// слотов, и переиспользовать чанк между ними нельзя. Область
/// идентифицируется номером чанка (`func_id`), потому что таблица имён
/// (`Chunk::local_names`) у чанка одна и та же на всех его вызовах.
/// `is_eval` тоже входит в ключ — от него зависит сам компилируемый текст
/// (`Возврат (...)` вместо операторов).
struct SnippetCache {
    entries: std::collections::HashMap<(usize, bool, String), std::rc::Rc<CompiledSnippet>>,
}

impl SnippetCache {
    fn new() -> Self {
        SnippetCache {
            entries: std::collections::HashMap::new(),
        }
    }

    fn get_or_compile(
        &mut self,
        code: &str,
        is_eval: bool,
        scope_id: usize,
        scope_locals: &[String],
        program: &Program,
    ) -> Result<std::rc::Rc<CompiledSnippet>, RtError> {
        let key = (scope_id, is_eval, code.to_string());
        if let Some(hit) = self.entries.get(&key) {
            return Ok(hit.clone());
        }
        let compiled = std::rc::Rc::new(compile_dynamic_snippet(
            code,
            is_eval,
            scope_locals,
            program,
        )?);
        self.entries.insert(key, compiled.clone());
        Ok(compiled)
    }
}

/// Лексика/парсинг/резолвинг/кодоген фрагмента. Любая ошибка на этом пути
/// — `RtError::DynamicError` В МОМЕНТ ИСПОЛНЕНИЯ, а не паника и не ошибка
/// сборки: текст фрагмента становится известен только сейчас, и кривой
/// текст обязан ловиться обычной `Попытка`.
fn compile_dynamic_snippet(
    code: &str,
    is_eval: bool,
    scope_locals: &[String],
    program: &Program,
) -> Result<CompiledSnippet, RtError> {
    // `is_eval` заворачивает выражение в `Возврат (...)`, чтобы получить
    // значение тем же путём, что и обычный `Возврат` — один движок на
    // `Выполнить` и `Вычислить`, без раздвоения семантики.
    let src = if is_eval {
        format!("Возврат ({code});")
    } else {
        code.to_string()
    };

    let parsed = bsl_syntax::parse(&src).map_err(|e| RtError::DynamicError(format!("{e:?}")))?;
    let mut stmts = Vec::with_capacity(parsed.items.len());
    for item in parsed.items {
        match item {
            bsl_syntax::Item::Stmt(s) => stmts.push(s),
            bsl_syntax::Item::VarDecl(vd) => stmts.push(bsl_syntax::Stmt::VarDecl(vd)),
            // НЕ ИЗМЕРЕНО(EXEC.PROC_DECLARATION): может ли фрагмент вообще
            // объявлять процедуры и функции. Взято «нет» — объявленную
            // процедуру было бы некуда деть: таблица чанков программы уже
            // скомпилирована.
            _ => {
                return Err(RtError::DynamicError(
                    "Выполнить/Вычислить не поддерживают объявление процедур/функций".to_string(),
                ))
            }
        }
    }

    let (all_locals, body) = bsl_sema::resolve_snippet_stmts(scope_locals, &stmts)
        .map_err(|e| RtError::DynamicError(format!("{e:?}")))?;
    let (chunk, _names, shapes) = bsl_bytecode::compile_snippet(&all_locals, &body, &program.names)
        .map_err(|e| RtError::DynamicError(format!("{e:?}")))?;

    Ok(CompiledSnippet { chunk, shapes })
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
/// `n_regs < n_params` кодоген не порождает (пиковое число регистров
/// включает параметры), но вычитание `u8` на таком чанке паниковало бы в
/// debug и молча заворачивалось в release — берём насыщающее, а промах по
/// регистру дальше поймает `reg_load`/`reg_store` уже как
/// `InvalidBytecode`.
fn push_own_registers(stack: &mut Vec<BslValue>, chunk: &bsl_bytecode::Chunk) {
    let n_own = chunk.n_regs.saturating_sub(chunk.n_params) as usize;
    stack.resize(stack.len() + n_own, BslValue::Undefined);
}

enum ReturnOutcome {
    Done(BslValue),
    Continuing,
}
use ReturnOutcome::{Continuing, Done};

/// `frames.pop().expect(...)` ниже — ВНУТРЕННИЙ ИНВАРИАНТ VM, а не входные
/// данные (см. классификацию в шапке модуля): `step`/`drive` вызывают эту
/// функцию только пока `frames` не пуст — сам факт того, что мы исполняем
/// инструкцию, это гарантирует. Никакой байт-код, корректный или нет, сюда
/// с пустым стеком кадров не приведёт.
fn do_return_with_value(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    value: BslValue,
) -> Result<ReturnOutcome, RtError> {
    let frame = frames
        .pop()
        .expect("инвариант VM: возврат исполняется только при непустом стеке кадров");
    match frames.last() {
        None => {
            // Самый верхний кадр завершился: НЕ усекаем стек — `drive`
            // возвращает его вызывающему как есть (нужно `run_isolated`,
            // чтобы прочитать финальные значения регистров после
            // `Выполнить`/`Вычислить`, а обычному `run_program` разницы
            // нет — он этот стек всё равно не читает после возврата).
            Ok(Done(value))
        }
        Some(caller) => {
            stack.truncate(frame.call_start);
            let dst = caller.reg_index(frame.return_reg);
            reg_store(stack, dst, value)?;
            Ok(Continuing)
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
        let chunk = match program.chunks.get(frames[frame_idx].func_id) {
            Some(c) => c,
            // Кадр с несуществующим чанком — не наше дело здесь: ошибку
            // уже несут наружу, обработчик в нём всё равно не найти.
            None => return false,
        };
        let check_pc = if first {
            frames[frame_idx].pc
        } else {
            // Кадр выше по стеку всегда стоит ЗА своей инструкцией `Call`,
            // так что `pc >= 1`; насыщение — страховка от того, что кадр
            // собрали не мы (см. классификацию паник в шапке модуля).
            frames[frame_idx].pc.saturating_sub(1)
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
        let frame = frames
            .pop()
            .expect("инвариант VM: `frames.len() >= 2` проверено строкой выше");
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
///
/// Арность проверена в `bsl-sema` — но проверена для того байт-кода,
/// который родился из резолвинга. Здесь она перепроверяется один раз на
/// вызов, чтобы ни эта функция, ни `bsl_rt::call_builtin_fn` (тоже
/// индексирующая `args` напрямую) не паниковали на чанке, собранном мимо
/// резолвера.
fn call_builtin_with_format(
    builtin: bsl_rt::BuiltinFn,
    args: &[BslValue],
) -> Result<BslValue, RtError> {
    use bsl_rt::BuiltinFn;
    // Проверка по МАКСИМУМУ, а не по минимуму: резолвер добивает
    // необязательные позиции `Неопределено` (см.
    // `BuiltinFn::arity_range`), так что корректный байт-код всегда
    // приносит полный набор — и `call_builtin_fn` может индексировать
    // `args[1]`/`args[2]` без проверок на каждой ветке.
    if args.len() < builtin.arity_range().1 {
        return Err(RtError::InvalidBytecode(
            "встроенной функции передано меньше аргументов, чем требует её арность",
        ));
    }
    match builtin {
        BuiltinFn::Message => {
            println!("{}", bsl_format::format_value(&args[0], None)?);
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ToString => {
            let s = bsl_format::format_value(&args[0], None)?;
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
            let s = bsl_format::format_value(&args[0], Some(&spec))?;
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
    let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    let result = f(&av, &bv)?;
    let d = frames[frame_idx].reg_index(dst);
    reg_store(stack, d, result)?;
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
    let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    let ord = av.compare(&bv, op)?;
    let d = frames[frame_idx].reg_index(dst);
    reg_store(stack, d, BslValue::Boolean(f(ord)))?;
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
    fn numeric_for_specialization_preserves_counter_assignment_and_final_value() {
        let v = run_src(
            "sum = 0;\n\
             Для i = 0 По 10 Цикл\n\
             sum = sum + i;\n\
             Если i = 2 Тогда i = 8; КонецЕсли;\n\
             КонецЦикла\n\
             Возврат sum * 100 + i;",
        );
        // Посещены 0, 1, 2, 9, 10; после последнего шага счётчик равен 11.
        assert_eq!(v, num("2211"));

        let v = run_src("Для i = 5 По 3 Цикл КонецЦикла; Возврат i;");
        assert_eq!(v, num("5"));

        // Пустое тело использует скрытый i64-счётчик, но наружу обязано
        // материализовать то же финальное BSL-значение.
        let v = run_src("Для i = 0 По 10 Цикл КонецЦикла; Возврат i;");
        assert_eq!(v, num("11"));

        // Значения, которые нельзя представить скрытым i64, автоматически
        // остаются на общем decimal/BigInt-пути.
        let v = run_src("Для i = 0.5 По 2.5 Цикл КонецЦикла; Возврат i;");
        assert_eq!(v, num("3.5"));
        let v = run_src(
            "Для i = 100000000000000000000 По 100000000000000000000 Цикл \
             КонецЦикла; Возврат i;",
        );
        assert_eq!(v, num("100000000000000000001"));
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
    fn short_circuit_and_skips_right_operand_on_false() {
        // Без ленивости `Неопределено.Свойство` бросил бы NotAnObject —
        // здесь этого не должно случиться, левый операнд уже решил результат.
        let v = run_src("Возврат Ложь И Неопределено.Свойство;");
        assert_eq!(v, BslValue::Boolean(false));
    }

    #[test]
    fn short_circuit_or_skips_right_operand_on_true() {
        let v = run_src("Возврат Истина ИЛИ Неопределено.Свойство;");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn short_circuit_still_enforces_strict_boolean_on_evaluated_operands() {
        // Левый операнд не решает результат -> правый вычисляется и обязан
        // быть `Булево`, как и левый.
        let err = run_src_err("Возврат Истина И 1;");
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));

        let err = run_src_err("Возврат Ложь ИЛИ 1;");
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));

        // ЛЕВЫЙ операнд идёт другим путём: он не «вычисляется по ходу», а
        // сам становится условием перехода (`JumpIfFalse`/`JumpIfTrue`,
        // см. кодоген `И`/`ИЛИ`), — и строгая булевость обязана держаться
        // и там, иначе `1 И Истина` тихо считалось бы истиной.
        let err = run_src_err("Возврат 1 И Истина;");
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));

        let err = run_src_err("Возврат 1 ИЛИ Истина;");
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));

        // И правый операнд не должен успеть вычислиться до этой ошибки:
        // если бы `1` привёлся к истине, `ИЛИ` замкнулось бы накоротко и
        // ошибки не было бы вовсе.
        let err = run_src_err("Возврат 1 ИЛИ Неопределено.Свойство;");
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));
    }

    #[test]
    fn short_circuit_chain_of_three_operands() {
        // Цепочка `А И Б И В`: если А уже Ложь, ни Б, ни В вычисляться не
        // должны.
        let v = run_src(
            "Возврат Ложь И Неопределено.Свойство И Неопределено.ДругоеСвойство;",
        );
        assert_eq!(v, BslValue::Boolean(false));
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
    fn get_prop_inline_cache_stays_correct_across_different_shapes() {
        // Один и тот же GetProp (внутри тела Для Каждого) видит структуры
        // ДВУХ разных форм подряд — если бы кэш слепо доверял старому
        // (форма, слот) без проверки Rc::ptr_eq, второе значение
        // прочиталось бы по слоту первой формы и оказалось бы неверным.
        let v = run_src(
            "a = Новый Массив(2);\n\
             a[0] = Новый Структура(\"x,y\", 10, 20);\n\
             a[1] = Новый Структура(\"y,x\", 200, 100);\n\
             сумма = 0;\n\
             Для Каждого elem Из a Цикл\n\
             сумма = сумма + elem.x;\n\
             КонецЦикла\n\
             Возврат сумма;",
        );
        // elem.x -> 10 (форма "x,y", x на слоте 0) + 100 (форма "y,x", x на слоте 1) = 110.
        assert_eq!(v, num("110"));
    }

    #[test]
    fn set_prop_inline_cache_stays_correct_across_different_shapes() {
        let v = run_src(
            "a = Новый Массив(2);\n\
             a[0] = Новый Структура(\"x,y\", 0, 0);\n\
             a[1] = Новый Структура(\"y,x\", 0, 0);\n\
             Для Каждого elem Из a Цикл\n\
             elem.x = 42;\n\
             КонецЦикла\n\
             Возврат a[0].x + a[1].x;",
        );
        assert_eq!(v, num("84"));
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
    fn structure_insert_adds_a_new_field_at_runtime() {
        // Задача 4 ревью: `ShapeTable` больше не только компиляционная —
        // `Вставить` заводит поле, которого не было в литерале `Новый
        // Структура(...)`, и оно сразу читается через `.y`.
        let v = run_src(
            "s = Новый Структура(\"x\", 1);\n\
             s.Вставить(\"y\", 2);\n\
             Возврат s.x + s.y;",
        );
        assert_eq!(v, num("3"));
    }

    #[test]
    fn structure_insert_on_existing_field_overwrites_value_not_shape() {
        let v = run_src(
            "s = Новый Структура(\"x\", 1);\n\
             s.Вставить(\"x\", 99);\n\
             Возврат s.x;",
        );
        assert_eq!(v, num("99"));
    }

    #[test]
    fn structure_delete_removes_field_and_shrinks_count() {
        let v = run_src(
            "s = Новый Структура(\"x,y\", 1, 2);\n\
             s.Удалить(\"x\");\n\
             Возврат s.Количество();",
        );
        assert_eq!(v, num("1"));
        let err = run_src_err(
            "s = Новый Структура(\"x,y\", 1, 2);\n\
             s.Удалить(\"x\");\n\
             Возврат s.x;",
        );
        assert!(matches!(err, RtError::UnknownField(_)));
    }

    #[test]
    fn structure_delete_missing_field_is_a_no_op() {
        let v = run_src(
            "s = Новый Структура(\"x\", 1);\n\
             s.Удалить(\"нетполя\");\n\
             Возврат s.Количество();",
        );
        assert_eq!(v, num("1"));
    }

    #[test]
    fn structure_property_returns_value_or_undefined_or_default() {
        let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"x\");");
        assert_eq!(v, num("5"));
        let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"y\");");
        assert_eq!(v, BslValue::Undefined);
        let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"y\", 42);");
        assert_eq!(v, num("42"));
    }

    #[test]
    fn structure_clear_resets_field_set_not_just_values() {
        let v = run_src(
            "s = Новый Структура(\"x,y\", 1, 2);\n\
             s.Очистить();\n\
             Возврат s.Количество();",
        );
        assert_eq!(v, num("0"));
        let err = run_src_err(
            "s = Новый Структура(\"x,y\", 1, 2);\n\
             s.Очистить();\n\
             Возврат s.x;",
        );
        assert!(matches!(err, RtError::UnknownField(_)));
    }

    #[test]
    fn structure_insert_in_a_loop_converges_on_one_shape_like_the_literal_path() {
        // Инвариант "формы интернируются глобально по набору ключей" не
        // должен ломаться рантайм-путём: одинаковый набор полей, заведённый
        // через `Вставить` в цикле на РАЗНЫХ структурах, обязан давать один
        // и тот же `Rc<Shape>`, что и прямой литерал с тем же набором —
        // иначе инлайн-кэш на горячий доступ к полю стал бы полиморфным.
        // Наблюдаем это косвенно: `Для Каждого` со смешанными "путём
        // Вставить" и "литералом" структурами по-прежнему читает верно.
        let v = run_src(
            "a = Новый Массив(2);\n\
             a[0] = Новый Структура();\n\
             a[0].Вставить(\"x\", 10);\n\
             a[0].Вставить(\"y\", 20);\n\
             a[1] = Новый Структура(\"x,y\", 100, 200);\n\
             сумма = 0;\n\
             Для Каждого elem Из a Цикл\n\
             сумма = сумма + elem.x;\n\
             КонецЦикла\n\
             Возврат сумма;",
        );
        assert_eq!(v, num("110"));
    }

    /// Чанк, собранный мимо кодогена, — ровно тот вход, ради которого
    /// индексация в `step` возвращает `InvalidBytecode` вместо паники.
    fn corrupt_program(instrs: Vec<Instr>) -> Program {
        Program {
            chunks: vec![bsl_bytecode::Chunk {
                instrs,
                consts: Vec::new(),
                call_arg_modes: Vec::new(),
                exception_ranges: Vec::new(),
                n_params: 0,
                n_locals: 1,
                n_regs: 1,
                local_names: Vec::new(),
                prop_cache: vec![std::cell::RefCell::new(None)],
            }],
            names: Vec::new(),
            shapes: Vec::new(),
            top_level_locals: Vec::new(),
        }
    }

    #[test]
    fn corrupt_bytecode_is_an_error_not_a_panic() {
        // Регистр за границей кадра.
        assert!(matches!(
            run_program(&corrupt_program(vec![Instr::Move { dst: 200, src: 0 }])),
            Err(RtError::InvalidBytecode(_))
        ));
        // Номер константы за границей таблицы констант.
        assert!(matches!(
            run_program(&corrupt_program(vec![Instr::LoadConst { dst: 0, k: 42 }])),
            Err(RtError::InvalidBytecode(_))
        ));
        // Номер вызываемого чанка за границей таблицы функций.
        assert!(matches!(
            run_program(&corrupt_program(vec![Instr::Call {
                func: 99,
                base: 0,
                arg_modes: 0,
                ret: 0,
            }])),
            Err(RtError::InvalidBytecode(_))
        ));
        // Номер формы за границей таблицы форм.
        assert!(matches!(
            run_program(&corrupt_program(vec![Instr::NewStructure {
                dst: 0,
                shape: 7,
                base: 0,
                count: 0,
            }])),
            Err(RtError::InvalidBytecode(_))
        ));
        // Программа вообще без чанка верхнего уровня.
        let mut empty = corrupt_program(Vec::new());
        empty.chunks.clear();
        assert!(matches!(
            run_program(&empty),
            Err(RtError::InvalidBytecode(_))
        ));
    }

    #[test]
    fn corrupt_bytecode_inside_a_call_unwinds_to_an_error_not_a_panic() {
        // Ошибка байт-кода внутри вызванного кадра проходит тем же путём
        // размотки, что и обычная `RtError`, и не роняет процесс на
        // `unwind_to_handler`.
        let mut program = corrupt_program(vec![
            Instr::Call { func: 1, base: 0, arg_modes: 0, ret: 0 },
            Instr::Return { src: Some(0) },
        ]);
        program.chunks[0].call_arg_modes = vec![Vec::new()];
        program.chunks[0].prop_cache = vec![std::cell::RefCell::new(None); 2];
        let mut callee = program.chunks[0].clone();
        callee.instrs = vec![Instr::Move { dst: 250, src: 0 }];
        program.chunks.push(callee);

        assert!(matches!(
            run_program(&program),
            Err(RtError::InvalidBytecode(_))
        ));
    }

    #[test]
    fn for_each_over_structure_yields_key_value_pairs_in_insertion_order() {
        let v = run_src(
            "с = Новый Структура(\"б,а,в\", 1, 2, 3);\n\
             рез = \"\";\n\
             Для Каждого киз Из с Цикл\n\
             рез = рез + киз.Ключ + Строка(киз.Значение);\n\
             КонецЦикла\n\
             Возврат рез;",
        );
        // Порядок — объявления, а не алфавитный и не хэшевый.
        assert_eq!(v, BslValue::Str(bsl_rt::BslString::from_str("б1а2в3")));
    }

    #[test]
    fn dictionary_structure_preserves_insertion_order_in_for_each() {
        // Больше `MAX_SHAPE_TRANSITIONS` вставок с динамическими именами —
        // структура заведомо ушла в словарный режим (см.
        // `bsl_rt::StructureStorage`), и `Для Каждого` по ней обязан
        // остаться детерминированным и совпасть с порядком вставки.
        let n = bsl_rt::MAX_SHAPE_TRANSITIONS + 10;
        let src = format!(
            "с = Новый Структура;\n\
             Для ном = 1 По {n} Цикл\n\
             с.Вставить(\"Поле\" + Строка(ном), ном);\n\
             КонецЦикла;\n\
             рез = \"\";\n\
             Для Каждого киз Из с Цикл\n\
             рез = рез + киз.Ключ + \"=\" + Строка(киз.Значение) + \";\";\n\
             КонецЦикла\n\
             Возврат рез;"
        );
        let expected: String = (1..=n).map(|i| format!("Поле{i}={i};")).collect();
        assert_eq!(
            run_src(&src),
            BslValue::Str(bsl_rt::BslString::from_str(&expected))
        );
    }

    #[test]
    fn dictionary_structure_still_answers_field_access_and_delete_from_script() {
        let n = bsl_rt::MAX_SHAPE_TRANSITIONS + 10;
        let src = format!(
            "с = Новый Структура;\n\
             Для ном = 1 По {n} Цикл\n\
             с.Вставить(\"Поле\" + Строка(ном), ном);\n\
             КонецЦикла;\n\
             с.Удалить(\"Поле1\");\n\
             с.Вставить(\"Поле2\", 200);\n\
             Возврат с.Количество() * 1000 + с.Поле2 + с.Свойство(\"Поле1\", 7);"
        );
        let expected = (n as i64 - 1) * 1000 + 200 + 7;
        assert_eq!(run_src(&src), num(&expected.to_string()));
    }

    #[test]
    fn map_insert_get_count_and_missing_key_returns_undefined() {
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(\"a\", 1);\n\
             м.Вставить(\"b\", 2);\n\
             Возврат м.Количество();",
        );
        assert_eq!(v, num("2"));

        let v = run_src("м = Новый Соответствие;\nм.Вставить(\"a\", 1);\nВозврат м.Получить(\"a\");");
        assert_eq!(v, num("1"));

        let v = run_src("м = Новый Соответствие;\nВозврат м.Получить(\"нет\");");
        assert_eq!(v, BslValue::Undefined);
    }

    #[test]
    fn map_key_hash_is_scale_independent() {
        // "Хеш числа обязан быть независим от масштаба" — м[1.0] и м[1.00]
        // должны быть ОДНИМ И ТЕМ ЖЕ ключом, а не двумя разными записями.
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(1.0, \"первое\");\n\
             м.Вставить(1.00, \"второе\");\n\
             Возврат м.Количество();",
        );
        assert_eq!(v, num("1"));
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(1.0, \"первое\");\n\
             Возврат м.Получить(1.00);",
        );
        assert_eq!(str_val(&v), "первое");
    }

    #[test]
    fn map_delete_removes_key_and_is_a_no_op_when_missing() {
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(\"a\", 1);\n\
             м.Удалить(\"a\");\n\
             м.Удалить(\"нет\");\n\
             Возврат м.Количество();",
        );
        assert_eq!(v, num("0"));
    }

    #[test]
    fn map_for_each_yields_key_value_pairs_in_insertion_order() {
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(\"a\", 1);\n\
             м.Вставить(\"b\", 2);\n\
             итог = \"\";\n\
             Для Каждого пара Из м Цикл\n\
             итог = итог + пара.Ключ + Строка(пара.Значение);\n\
             КонецЦикла\n\
             Возврат итог;",
        );
        assert_eq!(str_val(&v), "a1b2");
    }

    #[test]
    fn map_clear_resets_count_to_zero() {
        let v = run_src(
            "м = Новый Соответствие;\n\
             м.Вставить(\"a\", 1);\n\
             м.Очистить();\n\
             Возврат м.Количество();",
        );
        assert_eq!(v, num("0"));
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
    fn okrugl_rounds_half_up_in_decimal_not_f64() {
        // Окр(2.675, 2) обязан дать 2.68: ближайший f64 к 2.675 чуть
        // меньше самого числа, через f64 получилось бы 2.67.
        let v = run_src("Возврат Окр(2.675, 2);");
        assert_eq!(v, num("2.68"));
        // Второй аргумент необязателен — по умолчанию 0 разрядов.
        let v = run_src("Возврат Окр(2.4);");
        assert_eq!(v, num("2"));
        let v = run_src("Возврат Окр(2.5);");
        assert_eq!(v, num("3"));
    }

    #[test]
    fn okrugl_third_argument_selects_the_rounding_mode() {
        // ВСЁ НИЖЕ ИЗМЕРЕНО на платформе 8.3.27 (см. platform.tsv и якоря
        // NUM.ROUND.* в реестре). Режим 0 — половина К нулю, режим 1 — ОТ
        // нуля; до замера здесь было наоборот, и режим 0 считался
        // половиной к чётному.
        assert_eq!(run_src("Возврат Окр(2.5, 0, 0);"), num("2"));
        assert_eq!(run_src("Возврат Окр(3.5, 0, 0);"), num("3"));
        assert_eq!(run_src("Возврат Окр(-2.5, 0, 0);"), num("-2"));
        assert_eq!(run_src("Возврат Окр(2.5, 0, 1);"), num("3"));
        assert_eq!(run_src("Возврат Окр(3.5, 0, 1);"), num("4"));
        assert_eq!(run_src("Возврат Окр(-2.5, 0, 1);"), num("-3"));

        // Ничья не только на целых: Окр(2.675, 2, ...) — тоже с платформы.
        assert_eq!(run_src("Возврат Окр(2.675, 2, 0);"), num("2.67"));
        assert_eq!(run_src("Возврат Окр(2.675, 2, 1);"), num("2.68"));
        // Мимо ничьей режим ничего не меняет.
        assert_eq!(run_src("Возврат Окр(2.4, 0, 0);"), num("2"));
        assert_eq!(run_src("Возврат Окр(2.6, 0, 0);"), num("3"));

        // Опущенный третий аргумент — это режим 1, а НЕ 0: измерено, что
        // `Окр(2.5)` даёт 3, а `Окр(2.5, 0, 0)` даёт 2.
        assert_eq!(run_src("Возврат Окр(2.5);"), num("3"));
        assert_eq!(run_src("Возврат Окр(2.5);"), run_src("Возврат Окр(2.5, 0, 1);"));
        // И опущенное число разрядов — ноль.
        assert_eq!(run_src("Возврат Окр(2.675, 2);"), num("2.68"));

        // Неизвестный код режима платформа НЕ считает ошибкой и округляет
        // по умолчанию — измерено (`Окр(2.5, 0, 7)` -> 3).
        assert_eq!(run_src("Возврат Окр(2.5, 0, 7);"), num("3"));
    }

    #[test]
    fn cel_truncates_toward_zero_not_half_up() {
        let v = run_src("Возврат Цел(2.9);");
        assert_eq!(v, num("2"));
        let v = run_src("Возврат Цел(-2.9);");
        assert_eq!(v, num("-2"));
    }

    #[test]
    fn skipped_call_argument_uses_declared_default() {
        let v = run_src(
            "Функция Ф(а, б = 100)\n\
             Возврат а + б;\n\
             КонецФункции\n\
             Возврат Ф(1, );",
        );
        assert_eq!(v, num("101"));
    }

    #[test]
    fn skipped_call_argument_default_may_reference_earlier_parameter() {
        let v = run_src(
            "Функция Ф(а, б = а + 1, в = 100)\n\
             Возврат а + б + в;\n\
             КонецФункции\n\
             Возврат Ф(1, , 3);",
        );
        // б = а + 1 = 2 (пропущен), в = 3 (передан явно) -> 1 + 2 + 3 = 6.
        assert_eq!(v, num("6"));
    }

    #[test]
    fn skipped_call_argument_falls_back_to_default_when_all_optional_omitted() {
        let v = run_src(
            "Функция Ф(а, б = а + 1, в = 100)\n\
             Возврат а + б + в;\n\
             КонецФункции\n\
             Возврат Ф(1, ,);",
        );
        // б = а + 1 = 2, в = 100 (оба пропущены) -> 1 + 2 + 100 = 103.
        assert_eq!(v, num("103"));
    }

    #[test]
    fn explicit_argument_overrides_default_even_when_declared() {
        let v = run_src(
            "Функция Ф(а, б = 100)\n\
             Возврат а + б;\n\
             КонецФункции\n\
             Возврат Ф(1, 5);",
        );
        assert_eq!(v, num("6"));
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
    fn format_specifiers_beyond_the_measured_four_reach_the_formatter() {
        // Все ВЫБРАННЫЕ (не измеренные) значения проверены в bsl-format;
        // здесь — что ключи доходят до него через вызов из BSL и что
        // локаль действует на все три типа сразу.
        let v = run_src(r#"Возврат Формат(42, "ЧГ=0; ЧЦ=5; ЧВН=1");"#);
        assert_eq!(str_val(&v), "00042");
        let v = run_src(r#"Возврат Формат(0, "ЧН=пусто");"#);
        assert_eq!(str_val(&v), "пусто");
        let v = run_src(r#"Возврат Формат(1234, "ЧГ=0; ЧРД=.; ЧС=3");"#);
        assert_eq!(str_val(&v), "1.234");
        let v = run_src(r#"Возврат Формат(Ложь, "БЛ=неа");"#);
        assert_eq!(str_val(&v), "неа");
        let v = run_src(r#"Возврат Формат(1234.5, "Л=en");"#);
        assert_eq!(str_val(&v), "1,234.5");
        let v = run_src(r#"Возврат Формат(Дата(2024,1,15), "Л=en; ДФ='ММММ'");"#);
        assert_eq!(str_val(&v), "January");
    }

    #[test]
    fn unsupported_locale_is_a_catchable_error() {
        // НЕ ИЗМЕРЕНО(FMT.LOCALE.COVERAGE): незнакомая локаль — обычное
        // исключение времени исполнения, значит ловится Попыткой и не
        // роняет скрипт целиком.
        let err = run_src_err(r#"Возврат Формат(1, "Л=de_DE");"#);
        assert!(matches!(err, RtError::UnsupportedLocale(code) if code == "de_DE"));
        let v = run_src(
            "Попытка\n\
             р = Формат(1, \"Л=de_DE\");\n\
             Исключение\n\
             р = \"поймано\";\n\
             КонецПопытки;\n\
             Возврат р;",
        );
        assert_eq!(str_val(&v), "поймано");
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
    fn mid_without_length_runs_to_the_end_of_the_string() {
        // Третий аргумент необязателен (`BuiltinFn::arity_range`): резолвер
        // подставляет `Неопределено`, `Сред` читает его как "до конца".
        let v = run_src(r#"Возврат Сред("Привет", 4);"#);
        assert_eq!(str_val(&v), "вет");
    }

    #[test]
    fn strnaiti_returns_a_position_usable_by_sred_without_conversion() {
        let v = run_src(r#"Возврат СтрНайти("абвгд", "вг");"#);
        assert_eq!(v, num("3"));
        let v = run_src(r#"Возврат СтрНайти("абвгд", "яя");"#);
        assert_eq!(v, num("0"));
        // Позиция — в тех же код-юнитах, что считает СтрДлина: строка с
        // суррогатной парой сдвигает всё дальше на 2, не на 1.
        let v = run_src("Возврат СтрНайти(\"a\u{1F600}бв\", \"бв\");");
        assert_eq!(v, num("4"));
        // И этой позицией можно прямо резать строку.
        let v = run_src("Возврат Сред(\"a\u{1F600}бв\", СтрНайти(\"a\u{1F600}бв\", \"бв\"), 2);");
        assert_eq!(str_val(&v), "бв");
    }

    #[test]
    fn strzamenit_replaces_every_occurrence() {
        let v = run_src(r#"Возврат СтрЗаменить("а-б-в", "-", "+");"#);
        assert_eq!(str_val(&v), "а+б+в");
    }

    #[test]
    fn strrazdelit_and_strsoedinit_round_trip_through_an_array() {
        let v = run_src(r#"Возврат СтрРазделить("а,б,в", ",").Количество();"#);
        assert_eq!(v, num("3"));
        let v = run_src(r#"Возврат СтрРазделить("а,б,в", ",")[1];"#);
        assert_eq!(str_val(&v), "б");
        let v = run_src(r#"Возврат СтрСоединить(СтрРазделить("а,,б", ","), ",");"#);
        assert_eq!(str_val(&v), "а,,б");
    }

    #[test]
    fn one_sided_trims_and_line_helpers() {
        let v = run_src("Возврат СокрЛ(\"  а  \") + \"|\";");
        assert_eq!(str_val(&v), "а  |");
        let v = run_src("Возврат \"|\" + СокрП(\"  а  \");");
        assert_eq!(str_val(&v), "|  а");

        // Перевод строки внутри литерала лексер требует оформлять
        // продолжением через `|`, поэтому текст собирается из `Символ(10)` —
        // так же, как это пишут в реальном коде 1С.
        let v = run_src("пс = Символ(10);\nВозврат СтрЧислоСтрок(\"а\" + пс + \"б\" + пс + \"в\");");
        assert_eq!(v, num("3"));
        let v = run_src(
            "пс = Символ(10);\nВозврат СтрПолучитьСтроку(\"а\" + пс + \"б\" + пс + \"в\", 2);",
        );
        assert_eq!(str_val(&v), "б");
    }

    #[test]
    fn strshablon_substitutes_positional_values() {
        let v = run_src(r#"Возврат СтрШаблон("%1 и %2", "раз", "два");"#);
        assert_eq!(str_val(&v), "раз и два");
        // Меньше значений, чем позиций в шаблоне — пустая подстановка, не
        // ошибка резолвинга: арность у СтрШаблон вариативная.
        let v = run_src(r#"Возврат СтрШаблон("[%2]", "раз");"#);
        assert_eq!(str_val(&v), "[]");
        // Число подставляется своим строковым представлением.
        let v = run_src(r#"Возврат СтрШаблон("№%1", 7);"#);
        assert_eq!(str_val(&v), "№7");
    }

    #[test]
    fn simvol_and_kodsimvola_round_trip_and_agree_with_the_nbsp_measurement() {
        // Замер с платформы: разделитель групп разрядов — NBSP, код 160.
        let v = run_src(r#"Возврат КодСимвола(Символ(160));"#);
        assert_eq!(v, num("160"));
        // Позиция по умолчанию — первая.
        let v = run_src(r#"Возврат КодСимвола("абв");"#);
        assert_eq!(v, num(&(('а' as u32).to_string())));
        let v = run_src(r#"Возврат КодСимвола("абв", 2);"#);
        assert_eq!(v, num(&(('б' as u32).to_string())));
    }

    #[test]
    fn znachenie_zapolneno_covers_the_measured_cases() {
        // Измеренная часть: Неопределено/Null/пустая строка/ноль — Ложь.
        for src in [
            "Возврат ЗначениеЗаполнено(Неопределено);",
            "Возврат ЗначениеЗаполнено(NULL);",
            "Возврат ЗначениеЗаполнено(\"\");",
            "Возврат ЗначениеЗаполнено(0);",
        ] {
            assert_eq!(run_src(src), BslValue::Boolean(false), "{src}");
        }
        for src in [
            "Возврат ЗначениеЗаполнено(1);",
            "Возврат ЗначениеЗаполнено(\"а\");",
        ] {
            assert_eq!(run_src(src), BslValue::Boolean(true), "{src}");
        }
    }

    #[test]
    fn znachenie_zapolneno_enables_the_short_circuit_guard_idiom() {
        // Ради чего функция и нужна: правый операнд не должен вычисляться,
        // когда левый уже сказал "значения нет" (см. кодоген `И`).
        let v = run_src(
            "х = Неопределено;\n\
             Возврат ЗначениеЗаполнено(х) И х.Поле = 1;",
        );
        assert_eq!(v, BslValue::Boolean(false));
    }

    #[test]
    fn tipznch_compares_equal_across_different_values_of_one_type() {
        let v = run_src("Возврат ТипЗнч(1) = ТипЗнч(2);");
        assert_eq!(v, BslValue::Boolean(true));
        let v = run_src("Возврат ТипЗнч(1) = ТипЗнч(\"а\");");
        assert_eq!(v, BslValue::Boolean(false));
        // Именно та форма, ради которой заведён `Тип(...)`.
        let v = run_src("Возврат ТипЗнч(Новый Массив) = Тип(\"Массив\");");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn type_name_printed_by_stroka_matches_the_value_it_came_from() {
        // `Строка(Новый Массив)` -> "Массив" измерено на платформе; имя
        // типа обязано совпасть с ним, а не быть латинским `Array`.
        let v = run_src("Возврат Строка(ТипЗнч(Новый Массив));");
        assert_eq!(str_val(&v), "Массив");
        let v = run_src("Возврат Строка(ТипЗнч(1));");
        assert_eq!(str_val(&v), "Число");
        let v = run_src("Возврат Строка(ТипЗнч(\"а\"));");
        assert_eq!(str_val(&v), "Строка");
        // Английское имя на входе принимается, на выходе — всё равно русское.
        let v = run_src("Возврат Строка(Тип(\"Structure\"));");
        assert_eq!(str_val(&v), "Структура");
    }

    #[test]
    fn unknown_type_name_is_a_runtime_error_not_undefined() {
        let err = run_src_err("Возврат Тип(\"НетТакогоТипа\");");
        assert!(matches!(err, RtError::UnknownType(_)));
    }

    // --- Даты -----------------------------------------------------------

    fn date_str(src: &str) -> String {
        str_val(&run_src(src))
    }

    #[test]
    fn empty_date_literal_is_the_zero_of_the_epoch() {
        // Ради чего эпоха сдвинута на 0001-01-01: пустая дата — ноль, и
        // ЗначениеЗаполнено на ней даёт Ложь без отдельной константы.
        let v = run_src("Возврат ЗначениеЗаполнено('00010101');");
        assert_eq!(v, BslValue::Boolean(false));
        let v = run_src("Возврат ЗначениеЗаполнено('20240115');");
        assert_eq!(v, BslValue::Boolean(true));
        let v = run_src("Возврат '00010101' = Дата(1, 1, 1);");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn date_literal_and_constructor_agree_in_both_lengths() {
        let v = run_src("Возврат '20240115103000' = Дата(2024, 1, 15, 10, 30, 0);");
        assert_eq!(v, BslValue::Boolean(true));
        // Опущенное время — полночь.
        let v = run_src("Возврат '20240115' = Дата(2024, 1, 15);");
        assert_eq!(v, BslValue::Boolean(true));
        // Строковая форма конструктора.
        let v = run_src("Возврат Дата(\"20240115103000\") = '20240115103000';");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn nonexistent_calendar_literal_is_rejected_at_resolve_time() {
        // 30 февраля проходит лексер (цифры, длина 8), но не календарь —
        // и это ошибка КОМПИЛЯЦИИ, а не рантайма: литерал известен заранее.
        let prog = parse("Возврат '20240230';").unwrap();
        let err = resolve_program(&prog.items).unwrap_err();
        assert!(matches!(err, bsl_sema::SemaError::BadDateLiteral(_)));
    }

    #[test]
    fn date_arithmetic_is_in_seconds_both_ways() {
        // Дата - Дата -> число секунд.
        let v = run_src("Возврат Дата(2024, 1, 15) - Дата(2024, 1, 14);");
        assert_eq!(v, num("86400"));
        // Дата + Число -> дата, сдвинутая на N секунд.
        let v = run_src("Возврат Дата(2024, 1, 14) + 86400 = Дата(2024, 1, 15);");
        assert_eq!(v, BslValue::Boolean(true));
        // Дата - Число -> дата.
        let v = run_src("Возврат Дата(2024, 1, 15) - 86400 = Дата(2024, 1, 14);");
        assert_eq!(v, BslValue::Boolean(true));
        // Тип результата разный у двух форм вычитания — проверяем прямо.
        let v = run_src("Возврат Строка(ТипЗнч(Дата(2024,1,15) - Дата(2024,1,14)));");
        assert_eq!(str_val(&v), "Число");
        let v = run_src("Возврат Строка(ТипЗнч(Дата(2024,1,15) - 1));");
        assert_eq!(str_val(&v), "Дата");
    }

    #[test]
    fn dates_compare_by_moment_in_time() {
        let v = run_src("Возврат Дата(2024, 1, 14) < Дата(2024, 1, 15);");
        assert_eq!(v, BslValue::Boolean(true));
        let v = run_src("Возврат Дата(2024, 1, 15, 10, 0, 0) > Дата(2024, 1, 15, 9, 59, 59);");
        assert_eq!(v, BslValue::Boolean(true));
        let v = run_src("Возврат Дата(2024, 1, 15) = Дата(2024, 1, 15);");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn date_components_are_readable_individually() {
        let src = "д = Дата(2024, 2, 29, 13, 45, 30);\n";
        assert_eq!(run_src(&format!("{src}Возврат Год(д);")), num("2024"));
        assert_eq!(run_src(&format!("{src}Возврат Месяц(д);")), num("2"));
        assert_eq!(run_src(&format!("{src}Возврат День(д);")), num("29"));
        assert_eq!(run_src(&format!("{src}Возврат Час(д);")), num("13"));
        assert_eq!(run_src(&format!("{src}Возврат Минута(д);")), num("45"));
        assert_eq!(run_src(&format!("{src}Возврат Секунда(д);")), num("30"));
    }

    #[test]
    fn period_boundaries_and_weekday_from_script() {
        // 15 января 2024 — понедельник
        // (НЕ ИЗМЕРЕНО(DATE.WEEKDAY_NUMBERING), что пн = 1).
        assert_eq!(run_src("Возврат ДеньНедели(Дата(2024, 1, 15));"), num("1"));
        assert_eq!(
            date_str("Возврат Строка(НачалоДня(Дата(2024, 2, 17, 13, 45, 30)));"),
            "17.02.2024 00:00:00"
        );
        assert_eq!(
            date_str("Возврат Строка(КонецДня(Дата(2024, 2, 17)));"),
            "17.02.2024 23:59:59"
        );
        assert_eq!(
            date_str("Возврат Строка(НачалоМесяца(Дата(2024, 2, 17)));"),
            "01.02.2024 00:00:00"
        );
        // Високосный февраль — 29-е, не 28-е.
        assert_eq!(
            date_str("Возврат Строка(КонецМесяца(Дата(2024, 2, 17)));"),
            "29.02.2024 23:59:59"
        );
        assert_eq!(
            date_str("Возврат Строка(НачалоГода(Дата(2024, 7, 4)));"),
            "01.01.2024 00:00:00"
        );
        assert_eq!(
            date_str("Возврат Строка(КонецГода(Дата(2024, 7, 4)));"),
            "31.12.2024 23:59:59"
        );
        // Среда 17 января -> понедельник 15-го.
        assert_eq!(
            date_str("Возврат Строка(НачалоНедели(Дата(2024, 1, 17)));"),
            "15.01.2024 00:00:00"
        );
    }

    #[test]
    fn add_month_clamps_the_day_rather_than_failing() {
        // 31 января + 1 месяц -> 29 февраля
        // (НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP), см. add_months).
        assert_eq!(
            date_str("Возврат Строка(ДобавитьМесяц(Дата(2024, 1, 31), 1));"),
            "29.02.2024 00:00:00"
        );
        assert_eq!(
            date_str("Возврат Строка(ДобавитьМесяц(Дата(2024, 1, 15), -1));"),
            "15.12.2023 00:00:00"
        );
    }

    #[test]
    fn out_of_range_dates_are_errors_not_silent_wraparound() {
        let err = run_src_err("Возврат Дата(2024, 2, 30);");
        assert!(matches!(err, RtError::DateOutOfRange { .. }));
        let err = run_src_err("Возврат Дата(9999, 12, 31, 23, 59, 59) + 1;");
        assert!(matches!(err, RtError::DateOutOfRange { .. }));
        let err = run_src_err("Возврат Дата(1, 1, 1) - 1;");
        assert!(matches!(err, RtError::DateOutOfRange { .. }));
    }

    #[test]
    fn format_understands_df_and_dlf_keys() {
        let v = run_src("Возврат Формат(Дата(2024, 1, 15), \"ДФ='дд.ММ.гггг'\");");
        assert_eq!(str_val(&v), "15.01.2024");
        // Месяц и минута различаются регистром — обе в одном шаблоне.
        let v = run_src("Возврат Формат(Дата(2024, 1, 15, 9, 5, 0), \"ДФ='ММ/мм'\");");
        assert_eq!(str_val(&v), "01/05");
        let v = run_src("Возврат Формат(Дата(2024, 1, 15), \"ДЛФ=Д\");");
        assert_eq!(str_val(&v), "15.01.2024");
        let v = run_src("Возврат Формат(Дата(2024, 1, 15, 10, 30, 0), \"ДЛФ=В\");");
        assert_eq!(str_val(&v), "10:30:00");
        // Числовые ключи на дате не мешают и наоборот.
        let v = run_src("Возврат Формат(1000, \"ЧГ=0; ДФ='гггг'\");");
        assert_eq!(str_val(&v), "1000");
    }

    #[test]
    fn tipznch_of_a_date_is_the_localized_type_name() {
        let v = run_src("Возврат Строка(ТипЗнч(Дата(2024, 1, 15)));");
        assert_eq!(str_val(&v), "Дата");
        let v = run_src("Возврат ТипЗнч('20240115') = Тип(\"Дата\");");
        assert_eq!(v, BslValue::Boolean(true));
    }

    #[test]
    fn current_date_lands_inside_the_supported_range() {
        // Точное значение не проверить (оно зависит от часов машины), но
        // год обязан быть правдоподобным — это ловит перепутанную эпоху,
        // ради которой всё и затевалось.
        let v = run_src("Возврат Год(ТекущаяДата());");
        let year = match v {
            BslValue::Number(n) => n.to_i64_exact().unwrap(),
            other => panic!("ожидалось число, получено {other:?}"),
        };
        assert!((2020..=2200).contains(&year), "получен год {year}");
    }

    #[test]
    fn current_universal_date_in_milliseconds_runs_benchmark_style_script() {
        let v = run_src(
            "Процедура CalcНаСервере()\n\
             sum = 0.0;\n\
             flip = -1.0;\n\
             Для i = 1 По 100 Цикл\n\
             flip = -flip;\n\
             sum = sum + flip / (2 * i - 1);\n\
             КонецЦикла;\n\
             КонецПроцедуры\n\
             Т1 = ТекущаяУниверсальнаяДатаВМиллисекундах();\n\
             CalcНаСервере();\n\
             Возврат ТекущаяУниверсальнаяДатаВМиллисекундах() - Т1;",
        );
        let elapsed = match v {
            BslValue::Number(n) => n.to_i64_exact().unwrap(),
            other => panic!("ожидалось число миллисекунд, получено {other:?}"),
        };
        assert!(elapsed >= 0, "часы пошли назад: {elapsed} мс");
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

    // --- ТаблицаЗначений, волна 2 ---------------------------------------

    /// Таблица с колонками `имя`/`цена`/`кол` и тремя строками — общая
    /// затравка для тестов поиска/сортировки/итога.
    const GOODS: &str = "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"имя\");\n\
         т.Колонки.Добавить(\"цена\");\n\
         с1 = т.Добавить(); с1.имя = \"груша\"; с1.цена = 30;\n\
         с2 = т.Добавить(); с2.имя = \"яблоко\"; с2.цена = 10;\n\
         с3 = т.Добавить(); с3.имя = \"дыня\"; с3.цена = 20;\n";

    #[test]
    fn table_find_returns_a_row_or_undefined() {
        let v = run_src(&format!("{GOODS}Возврат т.Найти(\"дыня\").цена;"));
        assert_eq!(v, num("20"));
        // Не найдено — Неопределено, а не ошибка: это штатная проверка.
        let v = run_src(&format!("{GOODS}Возврат т.Найти(\"нет такого\");"));
        assert_eq!(v, BslValue::Undefined);
        // С явным списком колонок ищем только в них.
        let v = run_src(&format!("{GOODS}Возврат т.Найти(20, \"цена\").имя;"));
        assert_eq!(str_val(&v), "дыня");
        let v = run_src(&format!("{GOODS}Возврат т.Найти(20, \"имя\");"));
        assert_eq!(v, BslValue::Undefined);
        // Опечатка в имени колонки — ошибка, а не пустой результат.
        let err = run_src_err(&format!("{GOODS}Возврат т.Найти(20, \"опечатка\");"));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_find_rows_matches_every_field_of_the_search_structure() {
        let src = format!(
            "{GOODS}с4 = т.Добавить(); с4.имя = \"дыня\"; с4.цена = 99;\n\
             Возврат т.НайтиСтроки(Новый Структура(\"имя\", \"дыня\")).Количество();"
        );
        assert_eq!(run_src(&src), num("2"));

        // Два поля — оба обязаны совпасть.
        let src = format!(
            "{GOODS}с4 = т.Добавить(); с4.имя = \"дыня\"; с4.цена = 99;\n\
             Возврат т.НайтиСтроки(Новый Структура(\"имя,цена\", \"дыня\", 99)).Количество();"
        );
        assert_eq!(run_src(&src), num("1"));

        // Ничего не совпало — пустой массив, не Неопределено.
        let src = format!(
            "{GOODS}Возврат т.НайтиСтроки(Новый Структура(\"имя\", \"нет\")).Количество();"
        );
        assert_eq!(run_src(&src), num("0"));

        // Поле, которого нет среди колонок, — ошибка.
        let err = run_src_err(&format!(
            "{GOODS}Возврат т.НайтиСтроки(Новый Структура(\"опечатка\", 1));"
        ));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_sort_orders_ascending_and_descending() {
        let read = "рез = \"\";\n\
             Для Каждого с Из т Цикл рез = рез + с.имя + \";\"; КонецЦикла;\n\
             Возврат рез;";
        let v = run_src(&format!("{GOODS}т.Сортировать(\"цена\");\n{read}"));
        assert_eq!(str_val(&v), "яблоко;дыня;груша;");
        let v = run_src(&format!("{GOODS}т.Сортировать(\"цена Убыв\");\n{read}"));
        assert_eq!(str_val(&v), "груша;дыня;яблоко;");
        // Направление по умолчанию — возрастание, `Возр` его лишь называет.
        let v = run_src(&format!("{GOODS}т.Сортировать(\"имя Возр\");\n{read}"));
        assert_eq!(str_val(&v), "груша;дыня;яблоко;");
    }

    #[test]
    fn table_sort_is_stable_and_uses_the_second_key_only_on_ties() {
        let src = "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"группа\");\n\
             т.Колонки.Добавить(\"ном\");\n\
             а = т.Добавить(); а.группа = 1; а.ном = 1;\n\
             б = т.Добавить(); б.группа = 2; б.ном = 2;\n\
             в = т.Добавить(); в.группа = 1; в.ном = 3;\n\
             г = т.Добавить(); г.группа = 2; г.ном = 4;\n";
        let read = "рез = \"\";\n\
             Для Каждого с Из т Цикл рез = рез + Строка(с.ном); КонецЦикла;\n\
             Возврат рез;";
        // Сортировка по одной колонке: внутри группы — исходный порядок.
        let v = run_src(&format!("{src}т.Сортировать(\"группа\");\n{read}"));
        assert_eq!(str_val(&v), "1324");
        // Второй ключ работает только на равенстве первого.
        let v = run_src(&format!("{src}т.Сортировать(\"группа, ном Убыв\");\n{read}"));
        assert_eq!(str_val(&v), "3142");
    }

    #[test]
    fn table_sort_keeps_live_row_objects_pointing_at_their_own_row() {
        // Инвариант идентичности: строка держит row_id, а сортировка
        // переставляет физические позиции — объект, взятый ДО сортировки,
        // обязан остаться той же строкой.
        let v = run_src(&format!(
            "{GOODS}т.Сортировать(\"цена\");\n\
             Возврат с1.имя + \"=\" + Строка(с1.цена);"
        ));
        assert_eq!(str_val(&v), "груша=30");
        // И запись через него попадает в ту же строку, а не в чужую.
        let v = run_src(&format!(
            "{GOODS}т.Сортировать(\"цена\");\n\
             с1.цена = 99;\n\
             Возврат т.Найти(\"груша\").цена;"
        ));
        assert_eq!(v, num("99"));
    }

    #[test]
    fn table_sort_rejects_an_unknown_column_instead_of_silently_doing_nothing() {
        let err = run_src_err(&format!("{GOODS}т.Сортировать(\"опечатка\");"));
        assert!(matches!(err, RtError::UnknownColumn(_)));
        // Неизвестное направление — тоже ошибка.
        let err = run_src_err(&format!("{GOODS}т.Сортировать(\"цена Криво\");"));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_total_sums_the_column() {
        let v = run_src(&format!("{GOODS}Возврат т.Итог(\"цена\");"));
        assert_eq!(v, num("60"));
        // Нечисловые значения ИГНОРИРУЮТСЯ
        // (НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC) — см.
        // `ValueTableData::total`): колонка из одного текста даёт 0.
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"к\");\n\
             а = т.Добавить(); а.к = \"текст\";\n\
             б = т.Добавить(); б.к = 5;\n\
             Возврат т.Итог(\"к\");",
        );
        assert_eq!(v, num("5"));
        let err = run_src_err(&format!("{GOODS}Возврат т.Итог(\"опечатка\");"));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_sort_of_strings_is_case_insensitive_first() {
        // Фиксируем ВЫБРАННОЕ приближение коллации
        // (НЕ ИЗМЕРЕНО(TABLE.SORT.COLLATION), см.
        // `bsl_rt::table::collate`): регистр не делит слова на две группы,
        // как это было бы при сравнении по кодовым точкам, где все
        // заглавные идут раньше всех строчных.
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"с\");\n\
             Для Каждого з Из СтрРазделить(\"яблоко,Апельсин,арбуз\", \",\") Цикл\n\
             т.Добавить().с = з;\n\
             КонецЦикла;\n\
             т.Сортировать(\"с\");\n\
             рез = \"\";\n\
             Для Каждого с Из т Цикл рез = рез + с.с + \";\"; КонецЦикла;\n\
             Возврат рез;",
        );
        assert_eq!(str_val(&v), "Апельсин;арбуз;яблоко;");
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

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    #[test]
    fn table_copy_makes_an_independent_table() {
        // Копия — ДРУГАЯ таблица: правка её ячейки не видна в оригинале.
        let v = run_src(&format!(
            "{GOODS}к = т.Скопировать();\n\
             к[0].цена = 999;\n\
             Возврат т[0].цена;"
        ));
        assert_eq!(v, num("30"));
        let v = run_src(&format!("{GOODS}Возврат т.Скопировать().Количество();"));
        assert_eq!(v, num("3"));
    }

    #[test]
    fn table_copy_takes_only_the_listed_rows_and_columns() {
        let v = run_src(&format!(
            "{GOODS}строки = Новый Массив;\n\
             строки.Добавить(с3);\n\
             строки.Добавить(с1);\n\
             к = т.Скопировать(строки, \"имя\");\n\
             Возврат к[0].имя + \",\" + к[1].имя + \",\" + Строка(к.Колонки.Количество());"
        ));
        // Порядок строк копии — порядок МАССИВА, а не таблицы.
        assert_eq!(str_val(&v), "дыня,груша,1");
        // Колонки, не попавшей в список, в копии нет.
        let err = run_src_err(&format!(
            "{GOODS}к = т.Скопировать(Неопределено, \"имя\");\n\
             Возврат к[0].цена;"
        ));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_copy_columns_keeps_the_structure_and_drops_the_rows() {
        let v = run_src(&format!(
            "{GOODS}к = т.СкопироватьКолонки();\n\
             Возврат Строка(к.Количество()) + \",\" + Строка(к.Колонки.Количество());"
        ));
        assert_eq!(str_val(&v), "0,2");
        // Строку в пустую копию добавить можно — колонки на месте.
        let v = run_src(&format!(
            "{GOODS}к = т.СкопироватьКолонки(\"цена\");\n\
             к.Добавить().цена = 7;\n\
             Возврат к[0].цена;"
        ));
        assert_eq!(v, num("7"));
    }

    #[test]
    fn table_unload_and_load_column_round_trip() {
        let v = run_src(&format!(
            "{GOODS}м = т.ВыгрузитьКолонку(\"цена\");\n\
             Возврат Строка(м.Количество()) + \",\" + Строка(м[0]) + \",\" + Строка(м[2]);"
        ));
        assert_eq!(str_val(&v), "3,30,20");

        let v = run_src(&format!(
            "{GOODS}м = Новый Массив;\n\
             м.Добавить(1); м.Добавить(2); м.Добавить(3);\n\
             т.ЗагрузитьКолонку(м, \"цена\");\n\
             Возврат т.Итог(\"цена\");"
        ));
        assert_eq!(v, num("6"));
    }

    #[test]
    fn table_load_column_ignores_the_length_mismatch() {
        // НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH): фиксируем
        // ВЫБРАННОЕ — короткий массив меняет только начало колонки, длинный
        // не добавляет строк, число строк не меняется ни там, ни там.
        let v = run_src(&format!(
            "{GOODS}м = Новый Массив;\n\
             м.Добавить(1);\n\
             т.ЗагрузитьКолонку(м, \"цена\");\n\
             Возврат Строка(т.Количество()) + \",\" + Строка(т[0].цена) + \",\" + Строка(т[1].цена);"
        ));
        assert_eq!(str_val(&v), "3,1,10");

        let v = run_src(&format!(
            "{GOODS}м = Новый Массив;\n\
             Для i = 1 По 10 Цикл м.Добавить(i); КонецЦикла;\n\
             т.ЗагрузитьКолонку(м, \"цена\");\n\
             Возврат т.Количество();"
        ));
        assert_eq!(v, num("3"));
    }

    #[test]
    fn table_move_keeps_live_rows_valid() {
        // Инвариант 12: сдвиг переставляет строку, но живой объект строки
        // продолжает указывать на СВОИ данные, а не на чужие.
        let v = run_src(&format!(
            "{GOODS}т.Сдвинуть(с1, 2);\n\
             Возврат т[0].имя + \",\" + т[2].имя + \",\" + с1.имя;"
        ));
        assert_eq!(str_val(&v), "яблоко,груша,груша");
        // Индекс отражает новую позицию.
        let v = run_src(&format!(
            "{GOODS}т.Сдвинуть(0, 1);\n\
             Возврат т.Индекс(с1);"
        ));
        assert_eq!(v, num("1"));
    }

    #[test]
    fn table_move_past_the_edge_is_an_error() {
        // НЕ ИЗМЕРЕНО(TABLE.MOVE.OUT_OF_RANGE): взята ошибка, не зажатие.
        let err = run_src_err(&format!("{GOODS}т.Сдвинуть(с1, -1);"));
        assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
        let err = run_src_err(&format!("{GOODS}т.Сдвинуть(2, 1);"));
        assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
    }

    #[test]
    fn table_index_of_a_deleted_row_is_an_error() {
        let v = run_src(&format!("{GOODS}Возврат т.Индекс(с3);"));
        assert_eq!(v, num("2"));
        let err = run_src_err(&format!("{GOODS}т.Удалить(2);\nВозврат т.Индекс(с3);"));
        assert!(matches!(err, RtError::RowInvalidated));
        // Строка ЧУЖОЙ таблицы — ошибка метода, а не молчаливое «не нашли».
        let err = run_src_err(&format!(
            "{GOODS}другая = Новый ТаблицаЗначений();\n\
             другая.Колонки.Добавить(\"x\");\n\
             чужая = другая.Добавить();\n\
             Возврат т.Индекс(чужая);"
        ));
        assert!(matches!(err, RtError::MethodNotApplicable { .. }));
    }

    const COLLAPSE: &str = "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"группа\");\n\
         т.Колонки.Добавить(\"сумма\");\n\
         т.Колонки.Добавить(\"прочее\");\n\
         а = т.Добавить(); а.группа = \"б\"; а.сумма = 10; а.прочее = \"x\";\n\
         б = т.Добавить(); б.группа = \"а\"; б.сумма = 1; б.прочее = \"y\";\n\
         в = т.Добавить(); в.группа = \"б\"; в.сумма = 20; в.прочее = \"z\";\n";

    #[test]
    fn table_collapse_groups_and_sums() {
        let v = run_src(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
             рез = \"\";\n\
             Для Каждого с Из т Цикл рез = рез + с.группа + \"=\" + Строка(с.сумма) + \";\"; КонецЦикла;\n\
             Возврат рез;"
        ));
        // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.ROW_ORDER): порядок ПЕРВОГО ВХОЖДЕНИЯ,
        // поэтому «б» впереди «а», хотя по ключу было бы наоборот.
        assert_eq!(str_val(&v), "б=30;а=1;");
    }

    #[test]
    fn table_collapse_drops_the_columns_outside_both_lists() {
        // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.OTHER_COLUMNS): фиксируем ВЫБРАННОЕ —
        // колонка, не попавшая ни в группировку, ни в суммирование,
        // исчезает вместе со своими значениями.
        let v = run_src(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
             Возврат т.Колонки.Количество();"
        ));
        assert_eq!(v, num("2"));
        let err = run_src_err(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
             Возврат т[0].прочее;"
        ));
        assert!(matches!(err, RtError::UnknownColumn(_)));
    }

    #[test]
    fn table_collapse_without_summing_columns_leaves_unique_keys() {
        let v = run_src(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\");\n\
             Возврат Строка(т.Количество()) + \",\" + Строка(т.Колонки.Количество());"
        ));
        assert_eq!(str_val(&v), "2,1");
    }

    #[test]
    fn table_collapse_ignores_non_numeric_values_when_summing() {
        // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.NON_NUMERIC): то же решение, что у
        // `Итог` — нечисловое значение просто не входит в сумму.
        let v = run_src(
            "т = Новый ТаблицаЗначений();\n\
             т.Колонки.Добавить(\"г\");\n\
             т.Колонки.Добавить(\"с\");\n\
             а = т.Добавить(); а.г = \"к\"; а.с = 5;\n\
             б = т.Добавить(); б.г = \"к\"; б.с = \"текст\";\n\
             т.Свернуть(\"г\", \"с\");\n\
             Возврат т[0].с;",
        );
        assert_eq!(v, num("5"));
    }

    #[test]
    fn table_collapse_keeps_the_first_row_of_each_group_alive() {
        // Свёрнутая строка сохраняет row_id ПЕРВОЙ строки группы, поэтому
        // взятый до свёртки объект продолжает работать; строки, слитые в
        // неё, ведут себя как удалённые.
        let v = run_src(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
             Возврат а.сумма;"
        ));
        assert_eq!(v, num("30"));
        let err = run_src_err(&format!(
            "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
             Возврат в.сумма;"
        ));
        assert!(matches!(err, RtError::RowInvalidated));
    }

    #[test]
    fn table_wave3_methods_reject_unknown_columns() {
        for src in [
            "Возврат т.ВыгрузитьКолонку(\"опечатка\");",
            "т.Свернуть(\"опечатка\");",
            "к = т.Скопировать(Неопределено, \"опечатка\");",
        ] {
            let err = run_src_err(&format!("{GOODS}{src}"));
            assert!(
                matches!(err, RtError::UnknownColumn(_)),
                "ожидалась UnknownColumn на {src}, получено {err:?}"
            );
        }
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

    #[test]
    fn vychislit_evaluates_an_expression() {
        let v = run_src(r#"Возврат Вычислить("2+2");"#);
        assert_eq!(v, num("4"));
    }

    #[test]
    fn vychislit_sees_existing_top_level_variables() {
        let v = run_src("x = 10;\nВозврат Вычислить(\"x * 2\");");
        assert_eq!(v, num("20"));
    }

    #[test]
    fn vypolnit_mutates_an_existing_top_level_variable() {
        let v = run_src("x = 1;\nВыполнить(\"x = 5\");\nВозврат x;");
        assert_eq!(v, num("5"));
    }

    #[test]
    fn vypolnit_can_run_control_flow_and_read_it_back() {
        // Внутри строкового литерала BSL перенос строки без `|` — ошибка
        // лексера (см. multiline_string_requires_continuation_bar в
        // bsl-syntax), поэтому фрагмент для Выполнить пишем в одну строку.
        let v = run_src(
            r#"sum = 0;
Выполнить("Для i = 1 По 5 Цикл sum = sum + i; КонецЦикла");
Возврат sum;"#,
        );
        assert_eq!(v, num("15"));
    }

    #[test]
    fn vypolnit_new_local_does_not_leak_to_enclosing_scope() {
        // временная переменная снипета не должна пережить вызов и не
        // должна сломать окружающий скрипт — просто исчезает.
        let v = run_src(
            "x = 1;\n\
             Выполнить(\"временная = 99; x = x + временная;\");\n\
             Возврат x;",
        );
        assert_eq!(v, num("100"));
    }

    #[test]
    fn vypolnit_field_access_on_structure_created_by_static_code() {
        // Критическая проверка: интернер имён полей фрагмента засеян из
        // program.names, поэтому NameId для "y" совпадает с тем, что уже
        // использует статически созданная структура.
        let v = run_src(
            "s = Новый Структура(\"x,y\", 1, 2);\n\
             Выполнить(\"s.y = s.y + 100\");\n\
             Возврат s.y;",
        );
        assert_eq!(v, num("102"));
    }

    #[test]
    fn vypolnit_can_construct_a_new_structure_inside_the_snippet_itself() {
        // Регрессия: compile_snippet раньше не возвращал СВОЮ (локальную)
        // таблицу форм, и запуск шёл с чужим (внешним) списком форм —
        // NewStructure падал по индексу за границами, как только фрагмент
        // сам создавал структуру (а не просто читал уже существующую).
        // `""` внутри BSL-строкового литерала — экранирование кавычки
        // (доубление, не бэкслеш) для вложенного списка полей "a,b".
        let v = run_src(
            "Возврат Вычислить(\"Новый Структура(\"\"a,b\"\", 1, 2).b\");",
        );
        assert_eq!(v, num("2"));
    }

    #[test]
    fn vychislit_can_construct_and_use_new_values() {
        let v = run_src(r#"Возврат Вычислить("Новый Массив(3).Count()");"#);
        assert_eq!(v, num("3"));
    }

    #[test]
    fn dynamic_code_with_syntax_error_is_a_dynamic_error() {
        let err = run_src_err(r#"Выполнить("x = ");"#);
        assert!(matches!(err, RtError::DynamicError(_)));
    }

    #[test]
    fn dynamic_code_requires_a_string_argument() {
        let err = run_src_err("Выполнить(1);");
        assert!(matches!(err, RtError::TypeError { .. }));
    }

    #[test]
    fn vypolnit_inside_a_function_sees_and_changes_its_locals() {
        // Раньше это была `DynamicNotAtTopLevel`. Теперь чанк функции,
        // помеченной `uses_dynamic`, несёт таблицу «имя -> слот», и
        // фрагмент работает в области видимости ЭТОЙ функции.
        let v = run_src(
            "Функция Ф()\n\
             х = 1;\n\
             Выполнить(\"х = х + 41\");\n\
             Возврат х;\n\
             КонецФункции\n\
             Возврат Ф();",
        );
        assert_eq!(v, num("42"));
    }

    #[test]
    fn vychislit_inside_a_function_reads_its_locals() {
        let v = run_src(
            "Функция Ф()\n\
             а = 6;\n\
             б = 7;\n\
             Возврат Вычислить(\"а * б\");\n\
             КонецФункции\n\
             Возврат Ф();",
        );
        assert_eq!(v, num("42"));
    }

    #[test]
    fn dynamic_scope_is_the_frame_not_the_top_level() {
        // У функции и у верхнего уровня переменные с ОДНИМ именем — разные.
        // Фрагмент внутри функции обязан видеть её `х`, а не внешний.
        let v = run_src(
            "Функция Ф()\n\
             х = 5;\n\
             Возврат Вычислить(\"х\");\n\
             КонецФункции\n\
             х = 100;\n\
             Возврат Ф() * 1000 + х;",
        );
        assert_eq!(v, num("5100"));
    }

    #[test]
    fn dynamic_code_sees_parameters_including_by_reference_ones() {
        // Параметр по значению виден фрагменту как обычный слот.
        let v = run_src(
            "Функция Ф(Знач а)\n\
             Возврат Вычислить(\"а * 2\");\n\
             КонецФункции\n\
             Возврат Ф(21);",
        );
        assert_eq!(v, num("42"));

        // Параметр БЕЗ `Знач` — алиас на слот вызывающего (см.
        // `Frame::reg_index`): запись из фрагмента обязана быть видна
        // снаружи так же, как запись из обычного кода функции.
        let v = run_src(
            "Процедура П(а)\n\
             Выполнить(\"а = а + 1\");\n\
             КонецПроцедуры\n\
             х = 41;\n\
             П(х);\n\
             Возврат х;",
        );
        assert_eq!(v, num("42"));
    }

    #[test]
    fn dynamic_code_inside_a_loop_reuses_the_compiled_chunk() {
        // Кэш фрагментов — оптимизация, наблюдаемая только по времени, но
        // проверить можно то, что она не ломает семантику: одна и та же
        // строка, исполненная много раз, каждый раз работает с текущим
        // состоянием кадра, а не с запомненным при компиляции.
        let v = run_src(
            "сумма = 0;\n\
             Для ном = 1 По 5 Цикл\n\
             Выполнить(\"сумма = сумма + ном\");\n\
             КонецЦикла;\n\
             Возврат сумма;",
        );
        assert_eq!(v, num("15"));
    }

    #[test]
    fn dynamic_compile_error_is_catchable_at_runtime() {
        // Ошибка компиляции фрагмента — обычное исключение в момент
        // исполнения, а не паника: её можно поймать `Попытка`.
        let v = run_src(
            "рез = \"ок\";\n\
             Попытка\n\
             Выполнить(\"это ((( не код\");\n\
             рез = \"не сработало\";\n\
             Исключение\n\
             рез = \"поймано\";\n\
             КонецПопытки;\n\
             Возврат рез;",
        );
        assert_eq!(str_val(&v), "поймано");
    }

    #[test]
    fn dynamic_scope_marking_only_touches_functions_that_need_it() {
        // Таблица имён кадра materialized только у помеченных чанков —
        // остальные несут пустой список и ничего лишнего.
        let prog = parse(
            "Функция СВыполнить()\n\
             х = 1;\n\
             Выполнить(\"х = 2\");\n\
             Возврат х;\n\
             КонецФункции\n\
             Функция Обычная()\n\
             у = 1;\n\
             Возврат у;\n\
             КонецФункции\n\
             Возврат Обычная();",
        )
        .unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        assert!(resolved.functions[0].uses_dynamic);
        assert!(!resolved.functions[1].uses_dynamic);
        assert!(!resolved.top_level.uses_dynamic);

        let program = compile_program(&resolved).unwrap();
        assert!(!program.chunks[1].local_names.is_empty(), "помеченная функция");
        assert!(program.chunks[2].local_names.is_empty(), "обычная функция");
    }

    #[test]
    fn dynamic_marking_reaches_nested_statements() {
        // `Выполнить` под циклом под `Если` — такой же повод
        // материализовать имена, как и на верхнем уровне тела.
        let prog = parse(
            "Функция Ф(н)\n\
             Если н > 0 Тогда\n\
             Пока н > 0 Цикл\n\
             Выполнить(\"н = н - 1\");\n\
             КонецЦикла;\n\
             КонецЕсли;\n\
             Возврат н;\n\
             КонецФункции\n\
             Возврат Ф(3);",
        )
        .unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        assert!(resolved.functions[0].uses_dynamic);
        // И оно действительно работает насквозь.
        assert_eq!(run_src(
            "Функция Ф(Знач н)\n\
             Пока н > 0 Цикл\n\
             Выполнить(\"н = н - 1\");\n\
             КонецЦикла;\n\
             Возврат н;\n\
             КонецФункции\n\
             Возврат Ф(3);",
        ), num("0"));
    }

    #[test]
    fn vypolnit_declaring_procedures_is_rejected() {
        let err = run_src_err(r#"Выполнить("Процедура П() КонецПроцедуры");"#);
        assert!(matches!(err, RtError::DynamicError(_)));
    }

    #[test]
    fn text_writer_writes_utf8_and_flushes_on_close() {
        let path = std::env::temp_dir().join(format!(
            "onec-llvm-text-writer-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src = format!(
            "Файл = Новый ЗаписьТекста(\"{}\");\n\
             Файл.Записать(\"Привет\");\n\
             Файл.Записать(Символ(10));\n\
             Файл.Закрыть();",
            path.display()
        );

        run_src(&src);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Привет\n");
        std::fs::remove_file(path).unwrap();
    }
}
