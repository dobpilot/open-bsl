//! JIT: байт-код чанка -> машинный код x86-64.
//!
//! # Что это за JIT
//!
//! Шаблонный (template JIT), а не оптимизирующий. Для каждой инструкции
//! порождается вызов «шима» — маленькой `extern "C"` обёртки, которая
//! зовёт ТЕ ЖЕ функции, что и соответствующая ветка `step`: `add_op`,
//! `binop`, `cmp`, `eq_value`, `as_condition`, `reg_load`/`reg_store`.
//! Смысл операций JIT не переписывает — он убирает диспетчеризацию.
//!
//! Совсем без дублирования не выходит: обвязку вокруг общего вызова
//! (какой регистр читать, куда класть результат) шим повторяет за веткой
//! интерпретатора. Это ровно то место, где режимы могут разъехаться, и
//! оно уже разъезжалось: `=` здесь сперва сравнивал через порядок, а не
//! через `eq_value`, и `ТипЗнч(1) = ТипЗнч(2)` под `--jit` падало ошибкой
//! типа. Поэтому обвязка держится не обещанием, а тестом
//! `the_jit_agrees_with_the_interpreter_on_every_script`: он гоняет весь
//! корпус фикстур обоими режимами и сравнивает вывод вместе с кодом
//! возврата. Ту ошибку он поймал на первом прогоне.
//!
//! # Что именно выигрывается
//!
//! Из горячего цикла уходят: выборка инструкции, `match` по её коду,
//! проверка границ таблицы инструкций, возврат в цикл `drive` и повторный
//! поиск чанка по номеру функции. Переходы становятся настоящими
//! `jmp`/`jcc`: тело цикла крутится внутри одной нативной функции, ни
//! разу не возвращаясь в интерпретатор.
//!
//! # Как устроен вход и выход
//!
//! У чанка не одна точка входа, а по одной на каждую скомпилированную
//! инструкцию: карта `entries[pc]` даёт смещение в машинном коде.
//! Интерпретатор перед очередным шагом смотрит, есть ли вход для текущего
//! `pc`, и если есть — прыгает туда. Нативный код исполняет столько
//! инструкций, сколько умеет, и возвращает `pc`, на котором надо
//! продолжить обычным путём: вызовы, возвраты, `Выполнить`, исключения и
//! всё прочее, чего JIT не умеет, остаются интерпретатору. Отсюда и
//! безопасность частичной поддержки: неизвестная инструкция — не ошибка,
//! а выход.

mod mem;
mod x64;

use crate::{
    add_op, at, binop, call_builtin_with_format, cmp, field_name, neg_op, numeric_for_next_regular,
    prop_cache, reg_load, reg_store, CallArgs, Frame,
};
use bsl_bytecode::{Chunk, Instr, Program};
use bsl_rt::{BslValue, RtError};
use mem::ExecutableBuffer;
use x64::{Assembler, Cond, Reg};

/// Значение, которым нативный код сообщает «случилась ошибка»: сама
/// ошибка лежит в `JitCtx::error`. Обычный выход возвращает `pc`, а он
/// всегда меньше длины чанка, так что путаницы быть не может.
const JIT_ERROR: u64 = u64::MAX;

/// Есть ли JIT на этой сборке. На x86-64 Linux — да; проверка нужна
/// вызывающему, чтобы не заводить пустых структур там, где её нет.
pub const AVAILABLE: bool = true;

/// Коды возврата шима.
const OK: u64 = 0;
const JUMPED: u64 = 1;
const FAILED: u64 = 2;

/// Всё, что нужно шиму, одним указателем: соглашение о вызовах даёт шесть
/// регистров под аргументы, и тратить их на контекст расточительно.
///
/// Поля — сырые указатели на то, чем владеет `drive`. Пока работает
/// нативный код, `frames` и `stack` не могут переехать: инструкции,
/// меняющие их длину (вызов, возврат), JIT не компилирует и выходит на
/// них в интерпретатор.
#[repr(C)]
pub struct JitCtx {
    frames: *mut Vec<Frame>,
    stack: *mut Vec<BslValue>,
    program: *const Program,
    error: *mut Option<RtError>,
    /// Нужна `GetIndex`: индексация по строке ищет имя поля в этой
    /// таблице. Она переживает весь прогон и живёт в `drive_with`, как и
    /// остальное здесь.
    runtime_shapes: *mut bsl_rt::RuntimeShapes,
}

/// Скомпилированный чанк: машинный код и карта входов.
pub struct CompiledChunk {
    code: ExecutableBuffer,
    /// `entries[pc]` — смещение в машинном коде или `None`, если на эту
    /// инструкцию входить нельзя (она не скомпилирована).
    entries: Vec<Option<usize>>,
}

impl CompiledChunk {
    /// Исполняет чанк нативно, начиная с байт-кодовой позиции `pc`.
    ///
    /// Возвращает `None`, если на `pc` входа нет. Иначе — `pc`, на
    /// котором интерпретатор должен продолжить, либо ошибку.
    pub(crate) fn run(
        &self,
        pc: usize,
        frames: &mut Vec<Frame>,
        stack: &mut Vec<BslValue>,
        program: &Program,
        runtime_shapes: &mut bsl_rt::RuntimeShapes,
    ) -> Option<Result<usize, RtError>> {
        let offset = (*self.entries.get(pc)?)?;
        let mut error: Option<RtError> = None;
        let mut ctx = JitCtx {
            frames,
            stack,
            program,
            error: &mut error,
            runtime_shapes,
        };
        // Переход в отображённую страницу. Безопасность держится на том,
        // что код туда положил `compile` из этого же файла, а указатели в
        // контексте живут дольше вызова.
        let entry: extern "C" fn(*mut JitCtx) -> u64 =
            unsafe { std::mem::transmute(self.code.entry_at(offset)) };
        let result = entry(&mut ctx);
        if result == JIT_ERROR {
            return Some(Err(error.unwrap_or(RtError::InvalidBytecode(
                "JIT сообщил об ошибке, но не оставил её",
            ))));
        }
        Some(Ok(result as usize))
    }
}

/// Компилирует чанк целиком. `None` — если в нём не нашлось ни одной
/// инструкции, которую мы умеем, либо ядро не дало исполняемую страницу.
pub fn compile(chunk: &Chunk) -> Option<CompiledChunk> {
    let mut asm = Assembler::new();
    let mut entries: Vec<Option<usize>> = vec![None; chunk.instrs.len()];
    // Смещения ВСЕХ скомпилированных инструкций: цели переходов внутри
    // чанка, в отличие от точек входа, порогом не ограничены.
    let mut offsets: Vec<Option<usize>> = vec![None; chunk.instrs.len()];
    // Переходы между инструкциями: (место патча, целевой pc). Патчатся
    // после того, как известны смещения всех инструкций.
    let mut jumps: Vec<(x64::Patch, usize)> = Vec::new();
    let mut error_patches: Vec<x64::Patch> = Vec::new();
    let mut compiled_any = false;

    // Длина непрерывной цепочки скомпилированных инструкций, начиная с
    // каждой позиции: по ней решается, стоит ли делать позицию точкой
    // входа — см. MIN_RUN.
    let mut run_len = vec![0usize; chunk.instrs.len() + 1];
    for pc in (0..chunk.instrs.len()).rev() {
        run_len[pc] = if compile_instr(&chunk.instrs[pc]).is_some() {
            run_len[pc + 1] + 1
        } else {
            0
        };
    }

    for (pc, instr) in chunk.instrs.iter().enumerate() {
        let Some(op) = compile_instr(instr) else {
            continue;
        };
        compiled_any = true;
        let offset = asm.here();
        // Код порождаем всегда — на него могут прыгнуть изнутри чанка,
        // и там цепочка уже неважна. Точкой ВХОДА из интерпретатора
        // позиция становится, только если отсюда есть что исполнять.
        if run_len[pc] >= MIN_RUN {
            entries[pc] = Some(offset);
        }
        offsets[pc] = Some(offset);
        // Пролог у КАЖДОЙ инструкции: войти в чанк можно на любой из них,
        // а значит каждая обязана уметь оказаться первой. Платится он
        // один раз на вход в нативный код: переходы внутри чанка целятся
        // за пролог, в тело.
        prologue(&mut asm);
        match op {
            Compiled::Call { func, args } => {
                emit_call(&mut asm, func, pc as u32, args);
                asm.test_rax_rax();
                error_patches.push(asm.jcc(Cond::NotZero));
                // На следующую инструкцию — переходом в её ТЕЛО, а не
                // «просто дальше»: сразу за нами лежит её пролог, который
                // повторно сохранил бы регистры.
                let fallthrough = asm.jmp();
                jumps.push((fallthrough, pc + 1));
            }
            Compiled::Branch { func, args, target } => {
                emit_call(&mut asm, func, pc as u32, args);
                asm.cmp_rax_imm8(FAILED as i8);
                error_patches.push(asm.jcc(Cond::Zero));
                asm.cmp_rax_imm8(JUMPED as i8);
                let taken = asm.jcc(Cond::Zero);
                jumps.push((taken, target));
                let fallthrough = asm.jmp();
                jumps.push((fallthrough, pc + 1));
            }
            Compiled::Goto(target) => {
                // Безусловный переход — без единого вызова: ради этого
                // JIT и затевался. Тело цикла крутится внутри одной
                // нативной функции, не возвращаясь в `drive`.
                let always = asm.jmp();
                jumps.push((always, target));
            }
        }
    }

    if !compiled_any {
        return None;
    }

    // Общий выход. `bail` ждёт номер позиции в rax — его кладёт тот, кто
    // сюда прыгает.
    let bail_target = asm.here();
    epilogue(&mut asm);
    let error_target = asm.here();
    asm.mov_r_imm64(Reg::Rax, JIT_ERROR);
    epilogue(&mut asm);

    // Цель перехода — ТЕЛО инструкции, то есть её вход БЕЗ пролога.
    // Инструкция, которую мы не компилировали (и позиция за концом
    // чанка), целью быть не может: туда ставится выход с нужным pc.
    for (patch, target) in jumps {
        let to = match offsets.get(target).copied().flatten() {
            Some(offset) => offset + PROLOGUE_LEN,
            None => {
                let exit = asm.here();
                asm.mov_r_imm32(Reg::Rax, target as u32);
                let to_bail = asm.jmp();
                asm.patch(to_bail, bail_target);
                exit
            }
        };
        asm.patch(patch, to);
    }
    for patch in error_patches {
        asm.patch(patch, error_target);
    }

    let code = ExecutableBuffer::new(&asm.finish())?;
    Some(CompiledChunk { code, entries })
}

/// Длина пролога в байтах — переходы внутрь чанка целятся ЗА него.
/// Проверяется тестом `the_prologue_length_matches_what_is_emitted`.
/// Сколько инструкций подряд должно быть скомпилировано, чтобы на первую
/// из них имело смысл входить из интерпретатора.
///
/// Обоснование — из порождаемого кода, а не из бенчмарка. Вход стоит
/// пролога, эпилога и вызова через `CompiledChunk::run` с постройкой
/// контекста; шаг интерпретатора — выборки инструкции и `match`. На
/// цепочке в ОДНУ инструкцию первое заведомо дороже второго, поэтому
/// двойка. Разницу между двумя и тремя на этой машине измерить не
/// удалось: разброс медиан у `table_sort` между повторами доходит до 37%,
/// что больше любого ожидаемого здесь эффекта.
const MIN_RUN: usize = 2;

const PROLOGUE_LEN: usize = 9; // push+push+sub rsp,imm8+mov = 1+1+4+3

/// push rbp; push rbx; sub rsp, 8; mov rbx, rdi
///
/// `rbx` держит контекст всё время работы: он сохраняется вызываемым, а
/// значит переживает вызовы шимов. `sub rsp, 8` доводит выравнивание до
/// 16 байт к моменту `call`, как требует System V.
fn prologue(asm: &mut Assembler) {
    asm.push(Reg::Rbp);
    asm.push(Reg::Rbx);
    asm.sub_rsp(8);
    asm.mov_rr(Reg::Rbx, Reg::Rdi);
}

/// add rsp, 8; pop rbx; pop rbp; ret — результат уже лежит в rax.
fn epilogue(asm: &mut Assembler) {
    asm.add_rsp(8);
    asm.pop(Reg::Rbx);
    asm.pop(Reg::Rbp);
    asm.ret();
}

/// Единственная форма вызова: контекст, pc и три числовых аргумента.
/// Одинаковая у всех шимов — иначе кодогену пришлось бы знать сигнатуру
/// каждого.
type ShimFn = extern "C" fn(*mut JitCtx, u32, u32, u32, u32) -> u64;

fn emit_call(asm: &mut Assembler, func: ShimFn, pc: u32, args: [u32; 3]) {
    asm.mov_rr(Reg::Rdi, Reg::Rbx);
    asm.mov_r_imm32(Reg::Rsi, pc);
    asm.mov_r_imm32(Reg::Rdx, args[0]);
    asm.mov_r_imm32(Reg::Rcx, args[1]);
    asm.mov_r8d_imm32(args[2]);
    asm.mov_r_imm64(Reg::Rax, func as *const () as u64);
    asm.call_r(Reg::Rax);
}

/// Во что превращается одна инструкция.
enum Compiled {
    /// Вызов шима: 0 — дальше, иначе ошибка.
    Call { func: ShimFn, args: [u32; 3] },
    /// Вызов шима, решающий, прыгать ли: 1 — на цель, 0 — дальше, 2 —
    /// ошибка.
    Branch {
        func: ShimFn,
        args: [u32; 3],
        target: usize,
    },
    /// Безусловный переход, без вызова.
    Goto(usize),
}

/// Таблица «инструкция -> машинный код». `None` означает «JIT этого не
/// умеет» и приводит к выходу в интерпретатор, а не к ошибке.
///
/// Цели переходов у нас АБСОЛЮТНЫЕ (`pc = target`), а не относительные —
/// как и в интерпретаторе; перепутать было бы легко, поэтому здесь без
/// арифметики.
fn compile_instr(instr: &Instr) -> Option<Compiled> {
    let s = |func: ShimFn, args: [u32; 3]| Some(Compiled::Call { func, args });
    match *instr {
        Instr::Jump { target } => Some(Compiled::Goto(target as usize)),
        Instr::JumpIfFalse { cond, target } => Some(Compiled::Branch {
            func: shim_jump_if_false,
            args: [cond as u32, 0, 0],
            target: target as usize,
        }),
        Instr::JumpIfTrue { cond, target } => Some(Compiled::Branch {
            func: shim_jump_if_true,
            args: [cond as u32, 0, 0],
            target: target as usize,
        }),
        // Инструкция, ЗАМЫКАЮЩАЯ числовой цикл. Без неё тело цикла
        // компилировалось, а шаг счётчика — нет, и нативный код выходил в
        // интерпретатор на КАЖДОЙ итерации: весь смысл нативных переходов
        // при этом пропадал.
        Instr::NumericForNext {
            counter,
            bound,
            target,
        } => Some(Compiled::Branch {
            func: shim_numeric_for_next,
            args: [counter as u32, bound as u32, target as u32],
            target: target as usize,
        }),
        Instr::Move { dst, src } => s(shim_move, [dst as u32, src as u32, 0]),
        Instr::LoadConst { dst, k } => s(shim_load_const, [dst as u32, k as u32, 0]),
        Instr::LoadBool { dst, val } => s(shim_load_bool, [dst as u32, val as u32, 0]),
        Instr::LoadUndefined { dst } => s(shim_load_undefined, [dst as u32, 0, 0]),
        Instr::LoadNull { dst } => s(shim_load_null, [dst as u32, 0, 0]),
        Instr::Add { dst, a, b } => s(shim_add, [dst as u32, a as u32, b as u32]),
        Instr::Sub { dst, a, b } => s(shim_sub, [dst as u32, a as u32, b as u32]),
        Instr::Mul { dst, a, b } => s(shim_mul, [dst as u32, a as u32, b as u32]),
        Instr::Div { dst, a, b } => s(shim_div, [dst as u32, a as u32, b as u32]),
        Instr::Mod { dst, a, b } => s(shim_rem, [dst as u32, a as u32, b as u32]),
        Instr::Neg { dst, src } => s(shim_neg, [dst as u32, src as u32, 0]),
        Instr::Not { dst, src } => s(shim_not, [dst as u32, src as u32, 0]),
        Instr::Eq { dst, a, b } => s(shim_eq, [dst as u32, a as u32, b as u32]),
        Instr::NotEq { dst, a, b } => s(shim_not_eq, [dst as u32, a as u32, b as u32]),
        Instr::Lt { dst, a, b } => s(shim_lt, [dst as u32, a as u32, b as u32]),
        Instr::Gt { dst, a, b } => s(shim_gt, [dst as u32, a as u32, b as u32]),
        Instr::Le { dst, a, b } => s(shim_le, [dst as u32, a as u32, b as u32]),
        Instr::Ge { dst, a, b } => s(shim_ge, [dst as u32, a as u32, b as u32]),
        Instr::GetIndex { dst, obj, idx } => {
            s(shim_get_index, [dst as u32, obj as u32, idx as u32])
        }
        Instr::SetIndex { obj, idx, src } => {
            s(shim_set_index, [obj as u32, idx as u32, src as u32])
        }
        // Операндов четыре (приёмник, номер встроенной функции, начало
        // аргументов, их число), а в форме вызова шима их три, и один из
        // них — не число, а перечисление. Поэтому этот шим читает свою
        // инструкцию сам: по `pc` он и так её находит, а `match` по одному
        // известному варианту стоит несравнимо дешевле, чем разбор всей
        // таблицы кодов операций в `step`.
        Instr::CallBuiltin { .. } => s(shim_call_builtin, [0, 0, 0]),
        // У этих троих операндов тоже больше трёх либо среди них есть не
        // число (`NameId`, номер метода), поэтому они, как и CallBuiltin,
        // читают свою инструкцию сами.
        Instr::GetProp { .. } => s(shim_get_prop, [0, 0, 0]),
        Instr::SetProp { .. } => s(shim_set_prop, [0, 0, 0]),
        Instr::CallMethod { .. } => s(shim_call_method, [0, 0, 0]),
        _ => None,
    }
}

// --- Шимы ---------------------------------------------------------------

/// Обвязка, одинаковая у всех: развернуть контекст, зафиксировать `pc`,
/// выполнить тело, превратить `Result` в код возврата.
///
/// `pc` ставится ДО работы и не увеличивается после. Так же ведёт себя и
/// интерпретатор: он двигает `pc` уже после успеха, поэтому в момент
/// ошибки там стоит позиция сбойнувшей инструкции — по ней обработчик
/// исключений и ищет объемлющую `Попытка`. Ошибись здесь, и `Попытка`
/// ловила бы не то, что ловит в обычном режиме.
///
/// # Safety
///
/// `ctx` — указатель, переданный нативным кодом; он действителен ровно
/// столько, сколько исполняется чанк.
unsafe fn run_shim(
    ctx: *mut JitCtx,
    pc: u32,
    body: impl FnOnce(
        &mut Vec<Frame>,
        &mut Vec<BslValue>,
        &Program,
        usize,
        &mut bsl_rt::RuntimeShapes,
    ) -> Result<u64, RtError>,
) -> u64 {
    let ctx = unsafe { &mut *ctx };
    let frames = unsafe { &mut *ctx.frames };
    let stack = unsafe { &mut *ctx.stack };
    let program = unsafe { &*ctx.program };
    let shapes = unsafe { &mut *ctx.runtime_shapes };
    let frame_idx = frames.len() - 1;
    frames[frame_idx].pc = pc as usize;
    match body(frames, stack, program, frame_idx, shapes) {
        Ok(code) => {
            frames[frame_idx].pc = pc as usize + 1;
            code
        }
        Err(e) => {
            unsafe { *ctx.error = Some(e) };
            FAILED
        }
    }
}

macro_rules! shim {
    ($name:ident, |$frames:ident, $stack:ident, $program:ident, $idx:ident, $shapes:ident, $a:ident, $b:ident, $c:ident| $body:block) => {
        extern "C" fn $name(ctx: *mut JitCtx, pc: u32, $a: u32, $b: u32, $c: u32) -> u64 {
            unsafe {
                run_shim(ctx, pc, |$frames, $stack, $program, $idx, $shapes| {
                    // Глушим «не использовано» для шимов, которым часть
                    // параметров не нужна; ссылку на формы по значению брать
                    // нельзя — она не Copy.
                    let _ = (&$shapes, $program, $a, $b, $c);
                    $body
                })
            }
        }
    };
}

shim!(shim_move, |frames, stack, program, idx, shapes, dst, src, _c| {
    let s = frames[idx].reg_index(src as u8);
    let v = reg_load(stack, s)?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_load_const, |frames, stack, program, idx, shapes, dst, k, _c| {
    let chunk = at(
        &program.chunks,
        frames[idx].func_id,
        "номер чанка вне таблицы функций",
    )?;
    let v = at(
        &chunk.consts,
        k as usize,
        "номер константы вне таблицы констант чанка",
    )?
    .clone();
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_load_bool, |frames, stack, program, idx, shapes, dst, val, _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Boolean(val != 0))?;
    Ok(OK)
});

shim!(shim_load_undefined, |frames, stack, program, idx, shapes, dst, _b, _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Undefined)?;
    Ok(OK)
});

shim!(shim_load_null, |frames, stack, program, idx, shapes, dst, _b, _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Null)?;
    Ok(OK)
});

shim!(shim_add, |frames, stack, program, idx, shapes, dst, a, b| {
    add_op(frames, stack, idx, dst as u8, a as u8, b as u8)?;
    Ok(OK)
});

macro_rules! binop_shim {
    ($name:ident, $f:path) => {
        shim!($name, |frames, stack, program, idx, shapes, dst, a, b| {
            binop(frames, stack, idx, dst as u8, a as u8, b as u8, $f)?;
            Ok(OK)
        });
    };
}

binop_shim!(shim_sub, BslValue::sub);
binop_shim!(shim_mul, BslValue::mul);
binop_shim!(shim_div, BslValue::div);
binop_shim!(shim_rem, BslValue::rem);

macro_rules! cmp_shim {
    ($name:ident, $op:literal, $f:expr) => {
        shim!($name, |frames, stack, program, idx, shapes, dst, a, b| {
            cmp(frames, stack, idx, dst as u8, a as u8, b as u8, $op, $f)?;
            Ok(OK)
        });
    };
}

// `=` и `<>` — СТРУКТУРНОЕ равенство (`eq_value`), а не сравнение с
// порядком: сравнивать можно только числа, строки и даты, а на равенство
// — что угодно, включая `Тип` и `Неопределено`. Здесь сначала стояло
// `cmp(.., Ordering::Equal)`, и `ТипЗнч(1) = ТипЗнч(2)` под `--jit`
// падало ошибкой типа там, где интерпретатор печатал «Да». Поймал это
// `the_jit_agrees_with_the_interpreter_on_every_script` на первом же
// прогоне — ради этого он и написан.
shim!(shim_eq, |frames, stack, program, idx, shapes, dst, a, b| {
    let av = reg_load(stack, frames[idx].reg_index(a as u8))?;
    let bv = reg_load(stack, frames[idx].reg_index(b as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Boolean(av.eq_value(&bv)))?;
    Ok(OK)
});

shim!(shim_not_eq, |frames, stack, program, idx, shapes, dst, a, b| {
    let av = reg_load(stack, frames[idx].reg_index(a as u8))?;
    let bv = reg_load(stack, frames[idx].reg_index(b as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Boolean(!av.eq_value(&bv)))?;
    Ok(OK)
});
// Предикаты — БУКВАЛЬНО те же выражения, что в ветках `step`. Написать
// здесь равносильное, но другое (`o == Ordering::Less` вместо `o.is_lt()`)
// значило бы завести место, где расхождение возможно и незаметно.
cmp_shim!(shim_lt, "<", std::cmp::Ordering::is_lt);
cmp_shim!(shim_gt, ">", std::cmp::Ordering::is_gt);
cmp_shim!(shim_le, "<=", std::cmp::Ordering::is_le);
cmp_shim!(shim_ge, ">=", std::cmp::Ordering::is_ge);

shim!(shim_neg, |frames, stack, program, idx, shapes, dst, src, _c| {
    let v = reg_load(stack, frames[idx].reg_index(src as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, neg_op(&v)?)?;
    Ok(OK)
});

shim!(shim_not, |frames, stack, program, idx, shapes, dst, src, _c| {
    let v = reg_load(stack, frames[idx].reg_index(src as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v.not()?)?;
    Ok(OK)
});

// Условие читается через `as_condition`, а не «всё, что не ложь, —
// истина»: у 1С не-`Булево` в условии это ОШИБКА ТИПА, и приведения к
// истинности нет. Тем же вызовом это делает и интерпретатор.
shim!(shim_jump_if_false, |frames, stack, program, idx, shapes, cond, _b, _c| {
    let c = frames[idx].reg_index(cond as u8);
    Ok(if reg_load(stack, c)?.as_condition()? {
        OK
    } else {
        JUMPED
    })
});

// Шаг счётчика делает та же `numeric_for_next_regular`, что и ветка
// интерпретатора: она сама решает, куда поставить pc — на цель или на
// следующую инструкцию. Мы отдаём ей ЛОКАЛЬНЫЙ pc и по нему же узнаём,
// прыгнули или вышли из цикла; сравнение именно с «шагом вперёд», а не с
// целью, потому что цель у пустого цикла может совпасть с pc + 1.
shim!(shim_get_index, |frames, stack, program, idx, shapes, dst, obj, index| {
    let ov = reg_load(stack, frames[idx].reg_index(obj as u8))?;
    let iv = reg_load(stack, frames[idx].reg_index(index as u8))?;
    let v = ov.get_index(&iv, &shapes.names)?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_set_index, |frames, stack, program, idx, shapes, obj, index, src| {
    let ov = reg_load(stack, frames[idx].reg_index(obj as u8))?;
    let iv = reg_load(stack, frames[idx].reg_index(index as u8))?;
    let sv = reg_load(stack, frames[idx].reg_index(src as u8))?;
    ov.set_index(&iv, sv)?;
    Ok(OK)
});

shim!(shim_call_builtin, |frames, stack, program, idx, shapes, _a, _b, _c| {
    let (_chunk, _pc, instr) = own_instr(frames, program, idx)?;
    let Instr::CallBuiltin {
        dst,
        builtin,
        base,
        count,
    } = instr
    else {
        return Err(RtError::InvalidBytecode(
            "шим встроенной функции вызван не на своей инструкции",
        ));
    };
    let args = CallArgs::load(stack, &frames[idx], base, count)?;
    let v = call_builtin_with_format(builtin, args.as_slice(), shapes)?;
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

/// Своя инструкция шима: та, на которой стоит `pc` кадра. Возвращает и
/// чанк — он нужен и под инлайн-кэш свойства, и под таблицу констант.
fn own_instr<'a>(
    frames: &[Frame],
    program: &'a Program,
    idx: usize,
) -> Result<(&'a bsl_bytecode::Chunk, usize, Instr), RtError> {
    let pc = frames[idx].pc;
    let chunk = at(
        &program.chunks,
        frames[idx].func_id,
        "номер чанка вне таблицы функций",
    )?;
    Ok((chunk, pc, *at(&chunk.instrs, pc, "инструкция вне чанка")?))
}

shim!(shim_get_prop, |frames, stack, program, idx, shapes, _a, _b, _c| {
    let (chunk, pc, instr) = own_instr(frames, program, idx)?;
    let Instr::GetProp { dst, obj, name } = instr else {
        return Err(RtError::InvalidBytecode("шим свойства вызван не на своей инструкции"));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    // Инлайн-кэш — ячейка ЭТОЙ инструкции, ровно как у интерпретатора:
    // отдельного кэша у JIT-а нет и быть не должно, иначе мономорфный
    // сайт грелся бы дважды и по-разному.
    let v = match ov.get_field_cached(name, prop_cache(chunk, pc)?) {
        Err(RtError::NotAnObject) => ov.get_field_by_name(field_name(program, name)?)?,
        other => other?,
    };
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_set_prop, |frames, stack, program, idx, shapes, _a, _b, _c| {
    let (chunk, pc, instr) = own_instr(frames, program, idx)?;
    let Instr::SetProp { obj, name, src } = instr else {
        return Err(RtError::InvalidBytecode("шим свойства вызван не на своей инструкции"));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let sv = reg_load(stack, frames[idx].reg_index(src))?;
    // `Значение` перехватывается ТАК ЖЕ, как в ветке интерпретатора: ему
    // нужно форматирование из `bsl-format`.
    if !crate::set_spread_value(&ov, field_name(program, name)?, &sv)? {
        match ov.set_field_cached(name, sv.clone(), prop_cache(chunk, pc)?) {
            Err(RtError::NotAnObject) => ov.set_field_by_name(field_name(program, name)?, sv)?,
            other => other?,
        }
    }
    Ok(OK)
});

shim!(shim_call_method, |frames, stack, program, idx, shapes, _a, _b, _c| {
    let (_chunk, _pc, instr) = own_instr(frames, program, idx)?;
    let Instr::CallMethod {
        dst,
        obj,
        method,
        base,
        count,
    } = instr
    else {
        return Err(RtError::InvalidBytecode("шим метода вызван не на своей инструкции"));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let args = CallArgs::load(stack, &frames[idx], base, count)?;
    // `Вывести` перехватывается ТАК ЖЕ, как в ветке интерпретатора: ему
    // нужно форматирование из `bsl-format`. Иначе режимы разъедутся —
    // ровно это и поймал `the_jit_agrees_with_the_interpreter_on_every_script`.
    let v = if method == bsl_rt::BuiltinMethod::OutputArea {
        crate::output_area(&ov, args.as_slice())?
    } else {
        bsl_rt::call_builtin_method_ctx(method, &ov, args.as_slice(), shapes)?
    };
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_numeric_for_next, |frames, stack, program, idx, shapes, counter, bound, target| {
    let counter_idx = frames[idx].reg_index(counter as u8);
    let bound_idx = frames[idx].reg_index(bound as u8);
    let here = frames[idx].pc;
    let mut pc = here;
    numeric_for_next_regular(stack, counter_idx, bound_idx, &mut pc, target as i16)?;
    Ok(if pc == here + 1 { OK } else { JUMPED })
});

shim!(shim_jump_if_true, |frames, stack, program, idx, shapes, cond, _b, _c| {
    let c = frames[idx].reg_index(cond as u8);
    Ok(if reg_load(stack, c)?.as_condition()? {
        JUMPED
    } else {
        OK
    })
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prologue_length_matches_what_is_emitted() {
        // Переходы внутрь чанка целятся ЗА пролог по КОНСТАНТЕ. Разъедется
        // она с кодогеном — управление попадёт в середину инструкции.
        let mut asm = Assembler::new();
        prologue(&mut asm);
        assert_eq!(asm.here(), PROLOGUE_LEN);
    }
}
