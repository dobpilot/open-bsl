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
//! Точки входа лежат по началам VLIW-бандлов (см. `bsl_bytecode::bundle`):
//! карта `entries[pc]` даёт смещение в машинном коде. Внутрь бандла
//! интерпретатор войти не может — цели переходов и обработчики `Попытки`
//! по построению разметки начинают бандл, — поэтому члены бандла не несут
//! ни пролога, ни точки входа, а их тела лежат вплотную, без
//! fallthrough-переходов. Интерпретатор перед очередным шагом смотрит,
//! есть ли вход для текущего `pc`, и если есть — прыгает туда. Нативный
//! код исполняет столько инструкций, сколько умеет, и возвращает `pc`, на
//! котором надо продолжить обычным путём: вызовы, возвраты, `Выполнить`,
//! исключения и всё прочее, чего JIT не умеет, остаются интерпретатору.
//! Отсюда и безопасность частичной поддержки: неизвестная инструкция — не
//! ошибка, а выход (в том числе в середину бандла — интерпретатор дошагает
//! одиночными до следующего начала).

mod mem;
mod x64;

use crate::{
    CallArgs, ComponentMethodMap, ComponentPropertyMap, Frame, HostIo, LinkedComponents, add_op,
    at, binop, cached_component_method, call_builtin_with_format, cmp, component_prop_get,
    component_prop_set, field_name, neg_op, numeric_for_next_regular, prop_cache, reg_load,
    reg_store,
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
    /// Таблицы связывания для открытого `CallObjectMethod`: «номер имени
    /// → встроенный метод» для нативных получателей и карта мемоизации
    /// методов компонентных объектов. Разрешать имя строкой на каждом
    /// вызове слишком дорого. Указатели на данные `LinkedComponents`,
    /// которыми владеет `drive_with`, — живут дольше нативного вызова.
    builtin_methods: *const Option<bsl_rt::BuiltinMethod>,
    builtin_methods_len: usize,
    component_methods: *const ComponentMethodMap,
    /// То же для свойств: у них нет ячейки инструкции, и мемоизация
    /// «таблица типа, номер имени → обработчик» — единственный быстрый
    /// путь.
    component_properties: *const ComponentPropertyMap,
}

/// Скомпилированный чанк: машинный код и карта входов.
pub struct CompiledChunk {
    code: ExecutableBuffer,
    /// `entries[pc]` — смещение в машинном коде или `None`, если на эту
    /// инструкцию входить нельзя: она не скомпилирована, лежит в середине
    /// бандла или цепочка от неё короче `MIN_RUN`.
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
        linked: &LinkedComponents<'_>,
    ) -> Option<Result<usize, RtError>> {
        let offset = (*self.entries.get(pc)?)?;
        let mut error: Option<RtError> = None;
        let mut ctx = JitCtx {
            frames,
            stack,
            program,
            error: &mut error,
            runtime_shapes,
            builtin_methods: linked.builtin_methods.as_ptr(),
            builtin_methods_len: linked.builtin_methods.len(),
            component_methods: &linked.component_methods,
            component_properties: &linked.component_properties,
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
    // Смещения ТЕЛА всех скомпилированных инструкций (за прологом, если
    // он есть): цели переходов внутри чанка, в отличие от точек входа,
    // порогом не ограничены.
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

    // Начало бандла на этой позиции? Пустая таблица (чанк, собранный
    // мимо кодогена) читается как «все бандлы одиночные» — прологи
    // повсюду, прежнее поведение.
    let is_bundle_start = |pc: usize| chunk.bundle_len.get(pc).is_none_or(|&w| w >= 1);

    for (pc, instr) in chunk.instrs.iter().enumerate() {
        let Some(op) = compile_instr(instr) else {
            continue;
        };
        compiled_any = true;
        // Пролог и точка входа — только у НАЧАЛА бандла. Внутрь бандла
        // интерпретатор не входит: цели переходов и обработчики `Попытки`
        // начинают бандл по построению разметки, а после выхода нативного
        // кода на середине бандла `step` дошагает одиночными до следующего
        // начала. Переходы внутри чанка целятся в тело, минуя пролог, —
        // членам бандла пролог не нужен вовсе.
        if is_bundle_start(pc) {
            if run_len[pc] >= MIN_RUN {
                entries[pc] = Some(asm.here());
            }
            prologue(&mut asm);
        }
        offsets[pc] = Some(asm.here());
        // Если следующая инструкция скомпилирована и НЕ начинает бандл, её
        // тело ляжет вплотную за нашим — fallthrough-переход не нужен,
        // падаем насквозь. Это главный машинный выигрыш разметки: внутри
        // бандла код течёт линейно, без `jmp` между членами.
        let next_is_inline = pc + 1 < chunk.instrs.len()
            && !is_bundle_start(pc + 1)
            && compile_instr(&chunk.instrs[pc + 1]).is_some();
        match op {
            Compiled::Call { func, args } => {
                emit_call(&mut asm, func, pc as u32, args);
                asm.test_rax_rax();
                error_patches.push(asm.jcc(Cond::NotZero));
                if !next_is_inline {
                    // На следующую инструкцию — переходом в её ТЕЛО: за
                    // нами лежит либо её пролог, либо чужой код.
                    let fallthrough = asm.jmp();
                    jumps.push((fallthrough, pc + 1));
                }
            }
            Compiled::Branch { func, args, target } => {
                emit_call(&mut asm, func, pc as u32, args);
                asm.cmp_rax_imm8(FAILED as i8);
                error_patches.push(asm.jcc(Cond::Zero));
                asm.cmp_rax_imm8(JUMPED as i8);
                let taken = asm.jcc(Cond::Zero);
                jumps.push((taken, target));
                if !next_is_inline {
                    let fallthrough = asm.jmp();
                    jumps.push((fallthrough, pc + 1));
                }
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

    // Цель перехода — ТЕЛО инструкции: `offsets` хранит позицию сразу за
    // прологом (у членов бандла пролога и нет). Инструкция, которую мы не
    // компилировали (и позиция за концом чанка), целью быть не может: туда
    // ставится выход с нужным pc.
    for (patch, target) in jumps {
        let to = match offsets.get(target).copied().flatten() {
            Some(offset) => offset,
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
        // `Сообщить` обязан писать в поток конкретного `State`, три
        // функции окружения — отвечать из его часов, случайности и
        // аргументов запуска, а две файловые — читать и писать через его
        // файловую систему. Ничего этого у JIT-шима нет: он работает с
        // sink-потоками и без окружения. Все шесть остаются
        // интерпретатору; остальные builtin'ы ни того, ни другого не
        // трогают. Список — ОБЪЕДИНЕНИЕ `Сообщить`, входа
        // `bsl_rt::call_builtin_env` и входа `bsl_rt::call_builtin_files`.
        Instr::CallBuiltin {
            builtin:
                bsl_rt::BuiltinFn::Message
                | bsl_rt::BuiltinFn::CurrentDate
                | bsl_rt::BuiltinFn::CurrentUniversalDateInMilliseconds
                | bsl_rt::BuiltinFn::CommandLineArguments
                // Файловая система — тоже возможность ПРОГОНА, и до
                // сокращённого контекста шимов она не доезжает.
                | bsl_rt::BuiltinFn::ValueToFile
                | bsl_rt::BuiltinFn::ValueFromFile,
            ..
        } => None,
        Instr::CallBuiltin { .. } => s(shim_call_builtin, [0, 0, 0]),
        // У этих троих операндов тоже больше трёх либо среди них есть не
        // число (`NameId`, номер метода), поэтому они, как и CallBuiltin,
        // читают свою инструкцию сами.
        Instr::GetProp { .. } => s(shim_get_prop, [0, 0, 0]),
        Instr::SetProp { .. } => s(shim_set_prop, [0, 0, 0]),
        Instr::CallMethod { .. } => s(shim_call_method, [0, 0, 0]),
        // Открытые двойники троих закрытых выше: нативный получатель идёт
        // тем же инлайн-кэшем и таблицей связывания, что и в интерпретаторе,
        // компонентный — через sink-контекст, как у закрытых (методы и
        // свойства официальных компонентов не пишут в stdout; расхождение
        // поймала бы `the_jit_agrees_with_the_interpreter_on_every_script`).
        // С реестром в эти опкоды компилируется каждое обращение программы,
        // и выход в интерпретатор на каждом из них съедал целые чанки.
        Instr::GetObjectProp { .. } => s(shim_get_object_prop, [0, 0, 0]),
        Instr::SetObjectProp { .. } => s(shim_set_object_prop, [0, 0, 0]),
        Instr::CallObjectMethod { .. } => s(shim_call_object_method, [0, 0, 0]),
        _ => None,
    }
}

// --- Шимы ---------------------------------------------------------------

/// Обвязка, одинаковая у всех: развернуть контекст, выполнить тело,
/// превратить `Result` в код возврата.
///
/// На пути успеха `frame.pc` не трогается вовсе: пока работает натив,
/// его никто не читает — продолжение передаётся через `rax` на выходе, а
/// шимы, которым нужна собственная позиция, получают точный `pc`
/// аргументом. Единственный потребитель `frame.pc` во время нативного
/// исполнения — поиск обработчика `Попытка` после ошибки, поэтому точный
/// `pc` сбойнувшей инструкции пишется один раз в холодной ветке. Это тот
/// же инвариант, что у интерпретатора: в момент ошибки `pc` стоит на
/// сбойной инструкции — по нему и ищется объемлющая `Попытка`. Ошибись
/// здесь, и `Попытка` ловила бы не то, что ловит в обычном режиме.
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
    match body(frames, stack, program, frame_idx, shapes) {
        Ok(code) => code,
        Err(e) => {
            frames[frame_idx].pc = pc as usize;
            unsafe { *ctx.error = Some(e) };
            FAILED
        }
    }
}

/// Как [`run_shim`], но телу дополнительно отдаётся карта мемоизации
/// свойств: шимы свойств разрешают имя через неё, как интерпретатор.
unsafe fn run_prop_shim(
    ctx: *mut JitCtx,
    pc: u32,
    body: impl FnOnce(
        &mut Vec<Frame>,
        &mut Vec<BslValue>,
        &Program,
        usize,
        &mut bsl_rt::RuntimeShapes,
        &ComponentPropertyMap,
    ) -> Result<u64, RtError>,
) -> u64 {
    let properties = unsafe { &*(&*ctx).component_properties };
    unsafe {
        run_shim(ctx, pc, |frames, stack, program, idx, shapes| {
            body(frames, stack, program, idx, shapes, properties)
        })
    }
}

macro_rules! prop_shim {
    ($name:ident, |$frames:ident, $stack:ident, $program:ident, $idx:ident, $shapes:ident, $props:ident, $pc:ident| $body:block) => {
        extern "C" fn $name(ctx: *mut JitCtx, pc_arg: u32, _a: u32, _b: u32, _c: u32) -> u64 {
            unsafe {
                run_prop_shim(
                    ctx,
                    pc_arg,
                    |$frames, $stack, $program, $idx, $shapes, $props| {
                        let $pc: u32 = pc_arg;
                        $body
                    },
                )
            }
        }
    };
}

macro_rules! shim {
    ($name:ident, |$frames:ident, $stack:ident, $program:ident, $idx:ident, $shapes:ident, $pc:ident, $a:ident, $b:ident, $c:ident| $body:block) => {
        #[allow(dead_code)]
        extern "C" fn $name(ctx: *mut JitCtx, pc_arg: u32, $a: u32, $b: u32, $c: u32) -> u64 {
            unsafe {
                run_shim(ctx, pc_arg, |$frames, $stack, $program, $idx, $shapes| {
                    // Гигиена макроса не даёт телу видеть `pc_arg` — своя
                    // позиция привязывается к имени с места вызова.
                    let $pc: u32 = pc_arg;
                    // Глушим «не использовано» для шимов, которым часть
                    // параметров не нужна; ссылку на формы по значению брать
                    // нельзя — она не Copy.
                    let _ = (&$shapes, $program, $pc, $a, $b, $c);
                    $body
                })
            }
        }
    };
}

shim!(shim_move, |frames,
                  stack,
                  program,
                  idx,
                  shapes,
                  _pc,
                  dst,
                  src,
                  _c| {
    let s = frames[idx].reg_index(src as u8);
    let v = reg_load(stack, s)?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_load_const, |frames,
                        stack,
                        program,
                        idx,
                        shapes,
                        _pc,
                        dst,
                        k,
                        _c| {
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

shim!(shim_load_bool, |frames,
                       stack,
                       program,
                       idx,
                       shapes,
                       _pc,
                       dst,
                       val,
                       _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Boolean(val != 0))?;
    Ok(OK)
});

shim!(shim_load_undefined, |frames,
                            stack,
                            program,
                            idx,
                            shapes,
                            _pc,
                            dst,
                            _b,
                            _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Undefined)?;
    Ok(OK)
});

shim!(shim_load_null, |frames,
                       stack,
                       program,
                       idx,
                       shapes,
                       _pc,
                       dst,
                       _b,
                       _c| {
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Null)?;
    Ok(OK)
});

shim!(shim_add, |frames,
                 stack,
                 program,
                 idx,
                 shapes,
                 _pc,
                 dst,
                 a,
                 b| {
    add_op(frames, stack, idx, dst as u8, a as u8, b as u8)?;
    Ok(OK)
});

macro_rules! binop_shim {
    ($name:ident, $f:path) => {
        shim!($name, |frames,
                      stack,
                      program,
                      idx,
                      shapes,
                      _pc,
                      dst,
                      a,
                      b| {
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
        shim!($name, |frames,
                      stack,
                      program,
                      idx,
                      shapes,
                      _pc,
                      dst,
                      a,
                      b| {
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
shim!(shim_eq, |frames,
                stack,
                program,
                idx,
                shapes,
                _pc,
                dst,
                a,
                b| {
    let av = reg_load(stack, frames[idx].reg_index(a as u8))?;
    let bv = reg_load(stack, frames[idx].reg_index(b as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, BslValue::Boolean(av.eq_value(&bv)))?;
    Ok(OK)
});

shim!(shim_not_eq, |frames,
                    stack,
                    program,
                    idx,
                    shapes,
                    _pc,
                    dst,
                    a,
                    b| {
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

shim!(shim_neg, |frames,
                 stack,
                 program,
                 idx,
                 shapes,
                 _pc,
                 dst,
                 src,
                 _c| {
    let v = reg_load(stack, frames[idx].reg_index(src as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, neg_op(&v)?)?;
    Ok(OK)
});

shim!(shim_not, |frames,
                 stack,
                 program,
                 idx,
                 shapes,
                 _pc,
                 dst,
                 src,
                 _c| {
    let v = reg_load(stack, frames[idx].reg_index(src as u8))?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v.not()?)?;
    Ok(OK)
});

// Условие читается через `as_condition`, а не «всё, что не ложь, —
// истина»: у 1С не-`Булево` в условии это ОШИБКА ТИПА, и приведения к
// истинности нет. Тем же вызовом это делает и интерпретатор.
shim!(shim_jump_if_false, |frames,
                           stack,
                           program,
                           idx,
                           shapes,
                           _pc,
                           cond,
                           _b,
                           _c| {
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
shim!(shim_get_index, |frames,
                       stack,
                       program,
                       idx,
                       shapes,
                       _pc,
                       dst,
                       obj,
                       index| {
    let ov = reg_load(stack, frames[idx].reg_index(obj as u8))?;
    let iv = reg_load(stack, frames[idx].reg_index(index as u8))?;
    let v = ov.get_index(&iv, &shapes.names)?;
    let d = frames[idx].reg_index(dst as u8);
    reg_store(stack, d, v)?;
    Ok(OK)
});

shim!(shim_set_index, |frames,
                       stack,
                       program,
                       idx,
                       shapes,
                       _pc,
                       obj,
                       index,
                       src| {
    let ov = reg_load(stack, frames[idx].reg_index(obj as u8))?;
    let iv = reg_load(stack, frames[idx].reg_index(index as u8))?;
    let sv = reg_load(stack, frames[idx].reg_index(src as u8))?;
    ov.set_index(&iv, sv)?;
    Ok(OK)
});

shim!(shim_call_builtin, |frames,
                          stack,
                          program,
                          idx,
                          shapes,
                          pc,
                          _a,
                          _b,
                          _c| {
    let (_chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
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
    let mut stdout = std::io::sink();
    let mut stderr = std::io::sink();
    // Ни потоков, ни окружения: то и другое принадлежит `State`, которого
    // здесь нет. Функции, которым они нужны, сюда не компилируются (см.
    // список исключений выше), и `None` — не заглушка, а запись этого
    // контракта: если исключение когда-нибудь протухнет, будет ошибка, а
    // не молча другое время.
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: None,
    };
    let v = call_builtin_with_format(builtin, args.as_slice(), shapes, &mut host)?;
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

/// Своя инструкция шима — по точному `pc`, который нативный код передал
/// аргументом (`frame.pc` на горячем пути не поддерживается — см.
/// `run_shim`). Возвращает и чанк — он нужен и под инлайн-кэш свойства, и
/// под таблицу констант.
fn own_instr<'a>(
    frames: &[Frame],
    program: &'a Program,
    idx: usize,
    pc: usize,
) -> Result<(&'a bsl_bytecode::Chunk, Instr), RtError> {
    let chunk = at(
        &program.chunks,
        frames[idx].func_id,
        "номер чанка вне таблицы функций",
    )?;
    Ok((chunk, *at(&chunk.instrs, pc, "инструкция вне чанка")?))
}

prop_shim!(shim_get_prop, |frames,
                           stack,
                           program,
                           idx,
                           shapes,
                           props,
                           pc| {
    let (chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
    let Instr::GetProp { dst, obj, name } = instr else {
        return Err(RtError::InvalidBytecode(
            "шим свойства вызван не на своей инструкции",
        ));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    // Инлайн-кэш — ячейка ЭТОЙ инструкции, ровно как у интерпретатора:
    // отдельного кэша у JIT-а нет и быть не должно, иначе мономорфный
    // сайт грелся бы дважды и по-разному.
    let v = if let Some(object) = ov.object_ref() {
        // Legacy-байткод не помечает компонентное свойство открытым.
        // Мигрированные официальные объекты не используют IO в свойствах;
        // новый байткод получает настоящий CallContext через интерпретатор.
        let mut stdout = std::io::sink();
        let mut stderr = std::io::sink();
        let mut context = bsl_rt::CallContext::new(
            shapes,
            &mut stdout,
            &mut stderr,
            bsl_format::format_value,
            None,
        );
        component_prop_get(object, props, name, program, &mut context)?
    } else {
        match ov.get_field_cached(name, prop_cache(chunk, pc as usize)?) {
            Err(RtError::NotAnObject) => ov.get_field_by_name(field_name(program, name)?)?,
            other => other?,
        }
    };
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

prop_shim!(shim_set_prop, |frames,
                           stack,
                           program,
                           idx,
                           shapes,
                           props,
                           pc| {
    let (chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
    let Instr::SetProp { obj, name, src } = instr else {
        return Err(RtError::InvalidBytecode(
            "шим свойства вызван не на своей инструкции",
        ));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let sv = reg_load(stack, frames[idx].reg_index(src))?;
    // `Значение` перехватывается ТАК ЖЕ, как в ветке интерпретатора: ему
    // нужно форматирование из `bsl-format`.
    if let Some(object) = ov.object_ref() {
        let mut stdout = std::io::sink();
        let mut stderr = std::io::sink();
        let mut context = bsl_rt::CallContext::new(
            shapes,
            &mut stdout,
            &mut stderr,
            bsl_format::format_value,
            None,
        );
        component_prop_set(object, props, name, sv, program, &mut context)?;
    } else {
        match ov.set_field_cached(name, sv.clone(), prop_cache(chunk, pc as usize)?) {
            Err(RtError::NotAnObject) => ov.set_field_by_name(field_name(program, name)?, sv)?,
            other => other?,
        }
    }
    Ok(OK)
});

shim!(shim_call_method, |frames,
                         stack,
                         program,
                         idx,
                         shapes,
                         pc,
                         _a,
                         _b,
                         _c| {
    let (_chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
    let Instr::CallMethod {
        dst,
        obj,
        method,
        base,
        count,
    } = instr
    else {
        return Err(RtError::InvalidBytecode(
            "шим встроенного метода вызван не на своей инструкции",
        ));
    };
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let args = CallArgs::load(stack, &frames[idx], base, count)?;
    let v = if let Some(object) = ov.object_ref() {
        let mut stdout = std::io::sink();
        let mut stderr = std::io::sink();
        let mut context = bsl_rt::CallContext::new(
            shapes,
            &mut stdout,
            &mut stderr,
            bsl_format::format_value,
            None,
        );
        object.call_method(method.primary_name(), args.as_slice(), &mut context)?
    } else {
        bsl_rt::call_builtin_method_ctx(method, &ov, args.as_slice(), shapes)?
    };
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

// Открытые двойники свойств: тела зеркалят `shim_get_prop`/`shim_set_prop`,
// отличаются только паттерном инструкции и тем, что номер имени лежит в
// операнде как `u16`. Sink вместо настоящего stdout — то же допущение, что
// у закрытых шимов выше: свойства и методы официальных компонентов не
// пишут в поток вывода, а расхождение с интерпретатором поймала бы
// `the_jit_agrees_with_the_interpreter_on_every_script`.
prop_shim!(shim_get_object_prop, |frames,
                                  stack,
                                  program,
                                  idx,
                                  shapes,
                                  props,
                                  pc| {
    let (chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
    let Instr::GetObjectProp { dst, obj, name } = instr else {
        return Err(RtError::InvalidBytecode(
            "шим открытого свойства вызван не на своей инструкции",
        ));
    };
    let name_id = bsl_rt::NameId::from_index(name as u32);
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let v = if let Some(object) = ov.object_ref() {
        let mut stdout = std::io::sink();
        let mut stderr = std::io::sink();
        let mut context = bsl_rt::CallContext::new(
            shapes,
            &mut stdout,
            &mut stderr,
            bsl_format::format_value,
            None,
        );
        component_prop_get(object, props, name_id, program, &mut context)?
    } else {
        match ov.get_field_cached(name_id, prop_cache(chunk, pc as usize)?) {
            Err(RtError::NotAnObject) => ov.get_field_by_name(field_name(program, name_id)?)?,
            other => other?,
        }
    };
    let d = frames[idx].reg_index(dst);
    reg_store(stack, d, v)?;
    Ok(OK)
});

prop_shim!(shim_set_object_prop, |frames,
                                  stack,
                                  program,
                                  idx,
                                  shapes,
                                  props,
                                  pc| {
    let (chunk, instr) = own_instr(frames, program, idx, pc as usize)?;
    let Instr::SetObjectProp { obj, name, src } = instr else {
        return Err(RtError::InvalidBytecode(
            "шим открытого свойства вызван не на своей инструкции",
        ));
    };
    let name_id = bsl_rt::NameId::from_index(name as u32);
    let ov = reg_load(stack, frames[idx].reg_index(obj))?;
    let sv = reg_load(stack, frames[idx].reg_index(src))?;
    if let Some(object) = ov.object_ref() {
        let mut stdout = std::io::sink();
        let mut stderr = std::io::sink();
        let mut context = bsl_rt::CallContext::new(
            shapes,
            &mut stdout,
            &mut stderr,
            bsl_format::format_value,
            None,
        );
        component_prop_set(object, props, name_id, sv, program, &mut context)?;
    } else {
        match ov.set_field_cached(name_id, sv.clone(), prop_cache(chunk, pc as usize)?) {
            Err(RtError::NotAnObject) => ov.set_field_by_name(field_name(program, name_id)?, sv)?,
            other => other?,
        }
    }
    Ok(OK)
});

/// Открытый двойник `shim_call_method`. Написан без макроса `shim!`: ему
/// одному нужна таблица связывания «номер имени → встроенный метод» из
/// `JitCtx`, а расширять сигнатуру всех шимов ради одного поля незачем.
extern "C" fn shim_call_object_method(
    ctx: *mut JitCtx,
    pc_arg: u32,
    _a: u32,
    _b: u32,
    _c: u32,
) -> u64 {
    let (table_ptr, table_len, component_methods) = unsafe {
        let context = &*ctx;
        (
            context.builtin_methods,
            context.builtin_methods_len,
            &*context.component_methods,
        )
    };
    // Пустая таблица возможна только у `Vec::new()` — указатель у него
    // невисячий, `from_raw_parts` с нулевой длиной корректен.
    let table = unsafe { std::slice::from_raw_parts(table_ptr, table_len) };
    unsafe {
        run_shim(ctx, pc_arg, |frames, stack, program, idx, shapes| {
            let (chunk, instr) = own_instr(frames, program, idx, pc_arg as usize)?;
            let Instr::CallObjectMethod {
                dst,
                obj,
                method,
                base,
                count,
            } = instr
            else {
                return Err(RtError::InvalidBytecode(
                    "шим открытого метода вызван не на своей инструкции",
                ));
            };
            let name_id = bsl_rt::NameId::from_index(method as u32);
            let ov = at(
                stack,
                frames[idx].reg_index(obj),
                "чтение объекта за границей стека значений",
            )?;
            // Приёмник заимствуется, аргументы идут срезом стека — как в
            // ветке интерпретатора (см. `Instr::CallObjectMethod` в
            // `step_cold`): temp-регистры аргументов смежны, обработчики
            // до стека VM не достают.
            let contiguous_args = base as usize >= frames[idx].param_aliases.len();
            let fallback_args;
            let args: &[BslValue] = if count == 0 {
                &[]
            } else if contiguous_args {
                let start = frames[idx].reg_index(base);
                stack
                    .get(start..start + count as usize)
                    .ok_or(RtError::InvalidBytecode(
                        "чтение аргументов за границей стека значений",
                    ))?
            } else {
                fallback_args = CallArgs::load(stack, &frames[idx], base, count)?;
                fallback_args.as_slice()
            };
            let v = if let Some(object) = ov.object_ref() {
                let mut stdout = std::io::sink();
                let mut stderr = std::io::sink();
                let mut context = bsl_rt::CallContext::new(
                    shapes,
                    &mut stdout,
                    &mut stderr,
                    bsl_format::format_value,
                    None,
                );
                // Тот же кэш ячейки инструкции поверх мемоизированного
                // моста, что у интерпретатора: без моста каждый вызов
                // конвертированного типа шёл бы строковым сканом таблицы,
                // и `--jit` проигрывал бы интерпретатору в разы (измерено
                // на xml_parse до этой правки).
                match cached_component_method(
                    chunk,
                    pc_arg as usize,
                    component_methods,
                    object.method_table(),
                    name_id,
                    program,
                )? {
                    Some(call) => call(object.as_dyn(), args, &mut context)?,
                    None => {
                        object.call_method(field_name(program, name_id)?, args, &mut context)?
                    }
                }
            } else {
                let builtin = table
                    .get(name_id.index())
                    .copied()
                    .flatten()
                    .ok_or_else(|| RtError::UnknownMethod {
                        method: field_name(program, name_id).unwrap_or("?").to_string(),
                        receiver: ov.type_name(),
                    })?;
                bsl_rt::call_builtin_method_ctx(builtin, ov, args, shapes)?
            };
            let d = frames[idx].reg_index(dst);
            reg_store(stack, d, v)?;
            Ok(OK)
        })
    }
}

shim!(
    shim_numeric_for_next,
    |frames, stack, program, idx, shapes, pc, counter, bound, target| {
        let counter_idx = frames[idx].reg_index(counter as u8);
        let bound_idx = frames[idx].reg_index(bound as u8);
        // Своя позиция — из аргумента шима: `frame.pc` на горячем пути не
        // поддерживается (см. `run_shim`).
        let here = pc as usize;
        let mut next = here;
        numeric_for_next_regular(stack, counter_idx, bound_idx, &mut next, target as i16)?;
        Ok(if next == here + 1 { OK } else { JUMPED })
    }
);

shim!(shim_jump_if_true, |frames,
                          stack,
                          program,
                          idx,
                          shapes,
                          _pc,
                          cond,
                          _b,
                          _c| {
    let c = frames[idx].reg_index(cond as u8);
    Ok(if reg_load(stack, c)?.as_condition()? {
        JUMPED
    } else {
        OK
    })
});
