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

mod debug;
mod entry;

use entry::call_module_function_in_execution;
pub use entry::{
    call_module_function, call_module_function_with_registry_and_io, run_program,
    run_program_with_registry_and_io, run_repl_chunk_with_registry,
};
#[cfg(test)]
use entry::{call_module_function_with_host, run_program_with_host};
mod linking;

use debug::FrameValues;
pub use debug::{DebugAction, DebugHook, DebugPosition, DebugValues};
mod modules;

pub use modules::{CatalogContext, ROOT_MODULE, SessionModules};
use modules::{ModuleInitState, ModuleState, ModulesCtx, ensure_module_ready};
mod scheduler;
mod snippet;

use scheduler::{
    AsyncState, PromiseState, Task, TaskCompletion, TaskId, consume_scheduler_safe_point,
    crossed_scheduler_safe_point, resume_parked_task, take_frozen_ready, task_position,
};
pub use scheduler::{ExecutionWaker, SchedulerConfig};

#[cfg(test)]
use linking::link_verified;
use linking::{
    ComponentMethodMap, HostIo, LinkedComponents, component_prop_get, component_prop_set,
    link_components, resolve_component_method,
};

use snippet::run_dynamic_snippet;

use bsl_bytecode::{ArgMode, DynamicCompiler, Instr, Program};
use bsl_rt::{BslValue, RtError};
use std::io::Write;

struct Frame {
    /// Модуль, которому принадлежит `func_id`: `ROOT_MODULE` либо позиция
    /// в каталоге конфигурации. Кадры разных модулей чередуются в одном
    /// стеке кадров, а программа кадра резолвится драйвером по этому полю.
    module: u32,
    func_id: usize,
    pc: usize,
    /// Слоты параметров вызванной функции (длина — её `n_params`). Пуст у
    /// кадра, заведённого не инструкцией `Call`, — у чанка верхнего уровня,
    /// у фрагмента `Выполнить` и у вызова по имени из Rust: там параметры
    /// лежат обычными собственными регистрами кадра, а пропустить аргумент
    /// вызывающему просто нечем.
    param_aliases: Vec<ParamSlot>,
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
    /// Временные слоты параметров, созданные для передачи модульной либо
    /// импортированной переменной по ссылке: (индекс в стеке, модуль,
    /// слот). При возврате значения записываются обратно в `ModuleState`
    /// соответствующего модуля.
    module_copybacks: Vec<(usize, u32, usize)>,
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

/// Один слот параметра кадра.
///
/// Признак «аргумент передали» лежит РЯДОМ с индексом, а не отдельным
/// списком номеров: второй список того же множества пришлось бы держать в
/// согласии с первым, а такие копии здесь уже расходились (см. историю
/// `Instr::jump_target`).
#[derive(Clone, Copy)]
struct ParamSlot {
    /// Абсолютный индекс в `Vm::stack`. Для `Знач`-параметров и для
    /// параметров без `Знач`, но с не-переменным аргументом, это индекс
    /// материализованного значения (временный регистр вызывающего). Для
    /// параметров без `Знач` с голой переменной на месте вызова — индекс
    /// самой переменной вызывающего: чтение/запись слота параметра
    /// напрямую видны вызывающему.
    idx: usize,
    /// Аргумент на этом месте передали. `false` — позицию пропустили
    /// (`Ф(1, , 3)`), и значение слота обязан вычислить пролог умолчаний
    /// (`Instr::JumpIfNotSkipped` читает именно этот признак). Значения
    /// слота признак не касается: явно переданное `Неопределено` — это
    /// `provided: true`, и умолчание его не подменяет.
    provided: bool,
}

impl Frame {
    #[inline]
    fn reg_index(&self, r: u8) -> usize {
        let r = r as usize;
        if r < self.param_aliases.len() {
            self.param_aliases[r].idx
        } else {
            self.own_base + (r - self.param_aliases.len())
        }
    }
}

/// Аргументы подавляющего большинства встроенных вызовов помещаются сюда
/// без heap-аллокации. Более длинные вариативные вызовы используют `Vec`.
enum CallArgs {
    /// Ноль и один аргумент — подавляющее большинство вызовов методов
    /// (`Записать(строка)`, `Добавить(значение)`, `Количество()`). Отдельные
    /// варианты нужны, чтобы не строить и не ронять трёхэлементный массив
    /// `BslValue` там, где занят один слот: по профилю `csv_write` на это
    /// уходило заметное время в `CallArgs::load` и в `drop_glue`.
    None,
    One(BslValue),
    Inline {
        values: [BslValue; 3],
        len: usize,
    },
    Heap(Vec<BslValue>),
}

impl CallArgs {
    fn load(stack: &[BslValue], frame: &Frame, base: u8, count: u8) -> Result<Self, RtError> {
        if count == 0 {
            return Ok(CallArgs::None);
        }
        if count == 1 {
            return Ok(CallArgs::One(reg_load(stack, frame.reg_index(base))?));
        }
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
            CallArgs::None => &[],
            CallArgs::One(value) => std::slice::from_ref(value),
            CallArgs::Inline { values, len } => &values[..*len],
            CallArgs::Heap(values) => values,
        }
    }
}

/// Прогон без реестра — остался входом для собственных тестов VM:
/// production-путь (CLI и фасад) всюду ходит через `*_with_registry*`.
#[cfg(test)]
/// Выполняет `program.chunks[func_id]` с нуля, используя `stack` как
/// начальное содержимое регистров (уже дополненное/подготовленное
/// вызывающим), и возвращает значение и финальные модульные слоты. Этот
/// вход нужен тестам обратных вызовов по имени.
fn drive(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    drive_with(program, func_id, stack)
}

/// Предел числа одновременно активных кадров BSL. Кадры лежат в куче
/// (`Vec<Frame>` плюс общий стек значений), поэтому без предела
/// бесконечная рекурсия не падает, а молча растит память до OOM — процесс
/// умирает без диагностики и без шанса на `Попытка`. С пределом это
/// перехватываемая [`RtError::StackOverflow`].
// НЕ ИЗМЕРЕНО(EXEC.MAX_CALL_DEPTH) — какую глубину рекурсии допускает
// платформа и какой ошибкой отвечает на превышение; замер даёт только
// нижнюю границу (900 уровней обязаны работать).
const MAX_CALL_DEPTH: usize = 1000;

#[cfg(test)]
fn drive_with(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let mut env = bsl_rt::HostEnv::process();
    let linked = link_components(
        program,
        None,
        env.zone(),
        env.files(),
        env.random(),
        env.network(),
        env.background_jobs(),
        env.temp_storage(),
        env.message_sink(),
        bsl_bytecode::DynamicScope::ROOT,
    )?;
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut dynamic = tests::TestDynamic::bare();
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: Some(&mut env),
        dynamic: Some(&mut dynamic),
        dynamic_depth: &dynamic_depth,
    };
    let mut module_state = ModuleState::new(program);
    let (value, _) = drive_linked(
        program,
        func_id,
        stack,
        &linked,
        &mut host,
        &mut module_state,
        None,
    )?;
    Ok((value, module_state.slots))
}

/// Одноразовая подготовка прогона: таблица имён и форм.
///
/// Отдельная функция, а не первые строки `drive_linked`, и `inline(never)`
/// здесь — не украшение, а ИЗМЕРЕНИЕ. Пока этот пролог лежал в теле
/// `drive_linked`, положение горячего цикла зависело от его размера:
/// точка входа выровнена (`-align-all-functions=5` в `.cargo/config.toml`),
/// а цикл — на столько байтов дальше, сколько занял пролог. Любая правка
/// подготовки двигала цикл относительно границ декодера, и `empty_for`
/// платил за это десятками процентов при НЕИЗМЕННОМ числе инструкций:
/// именно так он подорожал 58 -> 83 млн тактов на `6c6b6a9` и ещё раз
/// 82 -> 92 при выносе фронтенда из VM. С вынесенным прологом цикл стоит
/// сразу за выровненным входом: `empty_for` вернулся к 58 млн тактов
/// (−30 % к базе), остальной набор — в пределах ±5 %, число инструкций
/// везде совпало.
#[inline(never)]
fn drive_prologue(program: &Program, linked: &LinkedComponents) -> bsl_rt::RuntimeShapes {
    // Затравлена формами/именами ЭТОЙ программы — см. `bsl_rt::RuntimeShapes`
    // doc comment про то, почему не общий на процесс синглтон: у вложенного
    // `Program` (см. `run_dynamic_snippet`) свои `names`/`shapes`, и рантайм-
    // расширения этой таблицы (`Вставить`/`Удалить` на структуре, меняющие
    // её форму) актуальны только для объектов внутри ОДНОГО такого вызова.
    // Типы компонентов и каталог их написаний приходят из реестра ОДНИМ
    // вызовом: промежуточного состояния «формы есть, типов ещё нет» не
    // существует. По ним `Тип("Имя")` находит то, чего нет в закрытом
    // реестре ядра (см. `TypeRef`).
    bsl_rt::RuntimeShapes::seeded(
        program.names.clone(),
        program.shapes.clone(),
        linked.registry,
    )
}

/// Результат продвижения сохраняемого запуска VM.
#[derive(Debug)]
pub enum ProgramPoll {
    Complete(BslValue, Vec<BslValue>),
    Runnable,
    Waiting,
}

/// Слот инлайн-кэша `GetProp`/`SetProp`: интернированная форма структуры
/// и номер слота поля в ней.
type PropCacheSlot = std::cell::RefCell<Option<(std::rc::Rc<bsl_rt::Shape>, u32)>>;

/// Слот инлайн-кэша `CallObjectMethod`: адрес статической таблицы методов
/// типа-получателя и разрешённый по ней дескриптор. Кэшируется дескриптор
/// целиком, чтобы попадание не платило отдельно за поиск арности.
type MethodCacheSlot =
    std::cell::RefCell<Option<(usize, Option<&'static bsl_rt::MethodDescriptor>)>>;

/// Инлайн-кэши одного запуска: по вектору ячеек на чанк, по одной ячейке
/// каждого вида на инструкцию.
struct RunCaches {
    prop: Vec<Vec<PropCacheSlot>>,
    method: Vec<Vec<MethodCacheSlot>>,
}

impl RunCaches {
    fn for_program(program: &Program) -> Self {
        let prop = program
            .chunks
            .iter()
            .map(|chunk| {
                std::iter::repeat_with(|| std::cell::RefCell::new(None))
                    .take(chunk.instrs.len())
                    .collect()
            })
            .collect();
        let method = program
            .chunks
            .iter()
            .map(|chunk| {
                std::iter::repeat_with(|| std::cell::RefCell::new(None))
                    .take(chunk.instrs.len())
                    .collect()
            })
            .collect();
        Self { prop, method }
    }
}

/// Состояние одного запуска программы, сохраняемое между вызовами `poll`.
/// Не содержит ссылок на host-сервисы и после завершения освобождает их для
/// следующего запуска того же `State`.
pub struct ProgramExecution {
    async_state: AsyncState,
    runtime_shapes: bsl_rt::RuntimeShapes,
    merge_linear: bool,
    /// Крючок отладчика, если прогон отлаживают.
    ///
    /// `None` — обычный прогон, и тогда единственная его цена на шаг —
    /// проверка `Option` во ВНЕШНЕМ цикле, вне `step`.
    debug: Option<Box<dyn DebugHook>>,
    root_result: Option<(BslValue, Vec<BslValue>)>,
    module_state: ModuleState,
    /// Кэши корневой программы этого запуска.
    caches: RunCaches,
    /// Кэши модулей каталога; индекс совпадает с `session_modules`.
    ///
    /// Параллельный вектор, а не третье поле `ModuleInstance`, — потому
    /// что в цикле `poll_linked` кэши текущего модуля держатся разделяемой
    /// ссылкой одновременно с мутабельным заимствованием
    /// `session_modules` через `ModulesCtx`. Общий ключ программы,
    /// линковки и кэшей сосредоточен в `CatalogContext::execution_parts`;
    /// кэши не входят в `ModuleInstance` и сохраняют прежнее владение.
    catalog_caches: Vec<RunCaches>,
    /// Экземпляры общих модулей каталога этого сеанса; у одиночной
    /// программы пуст.
    session_modules: SessionModules,
    /// Всегда квантовать, даже с одной BSL-задачей: фоновый owned-прогон
    /// чередуется с соседями по worker бюджетом poll, и неограниченный
    /// однозадачный fast path для него выключен. Обычный State остаётся
    /// с false и за проверку не платит.
    force_scheduled: bool,
    /// Кооперативная отмена: взводится другим потоком, проверяется на
    /// границах квантов. Латентность отмены ограничена квантом
    /// (`safe_points_per_quantum` safe points), а не одним safe point —
    /// более частая проверка стоила бы горячему циклу.
    cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    finished: bool,
}

impl ProgramExecution {
    fn new_linked(
        program: &Program,
        func_id: usize,
        stack: Vec<BslValue>,
        linked: &LinkedComponents<'_>,
        module_state: ModuleState,
        scheduler: SchedulerConfig,
    ) -> Self {
        let root = Task {
            frames: vec![Frame {
                module: ROOT_MODULE,
                func_id,
                pc: 0,
                param_aliases: Vec::new(),
                own_base: 0,
                call_start: 0,
                return_reg: 0,
                module_copybacks: Vec::new(),
                numeric_for_state: None,
            }],
            stack,
            current_exception: None,
            completion: TaskCompletion::Root,
            quantum_remaining: scheduler.safe_points_per_quantum,
        };
        let async_state = AsyncState::new(root, scheduler.safe_points_per_quantum);
        let runtime_shapes = drive_prologue(program, linked);
        Self {
            async_state,
            runtime_shapes,
            merge_linear: true,
            debug: None,
            root_result: None,
            module_state,
            caches: RunCaches::for_program(program),
            catalog_caches: Vec::new(),
            session_modules: SessionModules::default(),
            force_scheduled: false,
            cancel_flag: None,
            finished: false,
        }
    }

    /// Подключает сессионные экземпляры модулей каталога: по одному
    /// `ModuleInstance` на модуль, все в состоянии `NotStarted`. Вызывается
    /// один раз при создании конфигурационного запуска.
    ///
    /// `catalog` обязан быть тем же и неизменённым, что и в последующих
    /// `poll_configuration_*`: кэши и сессия строятся под его модули (см.
    /// контракт у [`Self::poll_with_registry_and_io`]).
    pub fn attach_catalog(&mut self, catalog: &bsl_bytecode::ConfigurationProgram) {
        self.session_modules = SessionModules::for_catalog(catalog);
        // Ячейки заводятся ЭНЕРГИЧНО для всех модулей каталога — так
        // требует спецификация образа («по ячейке на инструкцию до
        // исполнения первой инструкции»), и это сознательная цена: запуск
        // платит аллокацией, пропорциональной всему байт-коду каталога,
        // даже за модули, которых не коснётся. Ленивое заведение по
        // первому кадру модуля — изменение требования, не оптимизация на
        // месте (см. риски в `docs/plans/bsl-vm-refactor.md`).
        self.catalog_caches = catalog
            .modules
            .iter()
            .map(|module| RunCaches::for_program(&module.program))
            .collect();
    }

    /// Включает постоянное квантование — для фонового прогона, которым
    /// драйвер worker чередует несколько заданий (см.
    /// poll_configuration_with_budget).
    pub fn set_always_scheduled(&mut self, value: bool) {
        self.force_scheduled = value;
    }

    /// Подключает флаг кооперативной отмены: при взведённом флаге очередной
    /// квант возвращает неловимую `RtError::Canceled`, и драйвер фиксирует
    /// terminal-состояние «Отменено».
    pub fn set_cancel_flag(&mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        self.cancel_flag = Some(flag);
    }

    /// Подключает пробуждение драйвера: sink каждой последующей
    /// host-операции получает клон и зовёт его после доставки завершения.
    /// Драйвер подключает waker до первого poll — уже запущенные операции
    /// пробуждения не получают.
    pub fn set_host_waker(&mut self, waker: ExecutionWaker) {
        self.async_state.host_waker = Some(waker);
    }

    /// Планирует НЕленивую инициализацию модулей: тела выполняются до
    /// первой инструкции entry, в порядке `order` (post-order файлового
    /// графа — семантика расширения CLI `//@используй`; политика job
    /// остаётся ленивой до замера `JOB.MODULE.INIT`). Кадры кладутся в
    /// корневую задачу до первого poll; повторный вызов — ошибка контракта.
    ///
    /// # Errors
    ///
    /// `RtError::InvalidBytecode` при номере модуля вне каталога.
    pub fn schedule_eager_init(
        &mut self,
        catalog: &bsl_bytecode::ConfigurationProgram,
        order: &[u32],
    ) -> Result<(), RtError> {
        let root = self
            .async_state
            .tasks
            .first_mut()
            .and_then(Option::as_mut)
            .ok_or(RtError::InvalidBytecode(
                "инициализация планируется до первого poll",
            ))?;
        // Кадры исполняются с вершины стека: обратный порядок пуша даёт
        // прямой порядок исполнения.
        for module in order.iter().rev() {
            let instance = self
                .session_modules
                .instances
                .get_mut(*module as usize)
                .ok_or(RtError::InvalidBytecode(
                    "номер модуля инициализации вне каталога",
                ))?;
            if instance.init != ModuleInitState::NotStarted {
                continue;
            }
            instance.init = ModuleInitState::Initializing;
            let body = catalog
                .modules
                .get(*module as usize)
                .map(|m| &m.program)
                .ok_or(RtError::InvalidBytecode(
                    "номер модуля инициализации вне каталога",
                ))?;
            let chunk0 = at(&body.chunks, 0, "у модуля каталога нет тела")?;
            let call_start = root.stack.len();
            let own_base = root.stack.len();
            push_own_registers(&mut root.stack, chunk0);
            root.frames.push(Frame {
                module: *module,
                func_id: 0,
                pc: 0,
                param_aliases: Vec::new(),
                own_base,
                call_start,
                return_reg: 0,
                module_copybacks: Vec::new(),
                numeric_for_state: None,
            });
        }
        Ok(())
    }

    /// Создаёт отдельный запуск верхнего уровня и связывает его компоненты.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания до исполнения первой инструкции.
    pub fn start_with_registry(
        program: &Program,
        registry: &bsl_rt::RuntimeRegistry,
        host_env: &bsl_rt::HostEnv,
    ) -> Result<Self, RtError> {
        Self::start_with_registry_and_scheduler(
            program,
            registry,
            host_env,
            SchedulerConfig::default(),
        )
    }

    /// Создаёт запуск с явным квантом кооперативного планировщика.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания либо нулевого кванта.
    pub fn start_with_registry_and_scheduler(
        program: &Program,
        registry: &bsl_rt::RuntimeRegistry,
        host_env: &bsl_rt::HostEnv,
        scheduler: SchedulerConfig,
    ) -> Result<Self, RtError> {
        if scheduler.safe_points_per_quantum == 0 {
            return Err(RtError::DynamicError(
                "квант планировщика должен содержать хотя бы одну безопасную точку".into(),
            ));
        }
        let mut stack = Vec::new();
        push_own_registers(
            &mut stack,
            at(&program.chunks, 0, "в программе нет чанка верхнего уровня")?,
        );
        let linked = link_components(
            program,
            Some(registry),
            host_env.zone(),
            host_env.files(),
            host_env.random(),
            host_env.network(),
            host_env.background_jobs(),
            host_env.temp_storage(),
            host_env.message_sink(),
            bsl_bytecode::DynamicScope::ROOT,
        )?;
        Ok(Self::new_linked(
            program,
            0,
            stack,
            &linked,
            ModuleState::new(program),
            scheduler,
        ))
    }

    /// Ставит крючок отладчика на этот прогон.
    ///
    /// Заодно выключает сцепление линейных цепочек бандлов: при
    /// `merge_linear` `step` не возвращается во внешний цикл между
    /// бандлами, а крючок зовётся именно оттуда — иначе остановка
    /// приходила бы не на той инструкции, о которой просили.
    ///
    /// Разметку бандлов это НЕ отменяет: её снимает сборка образа со
    /// сведениями об отладке (`image::finalize_unbundled`), потому что
    /// пучкованность — свойство образа, а не прогона.
    pub fn set_debug_hook(&mut self, hook: Box<dyn DebugHook>) {
        self.debug = Some(hook);
        self.merge_linear = false;
    }

    /// Продвигает ранее созданный запуск, не сохраняя ссылок на host между
    /// вызовами. Конечный `host_slice` не блокирует ожидание completion;
    /// `usize::MAX` используется run-to-completion драйвером и ждёт первый.
    ///
    /// `program` обязана быть ТОЙ ЖЕ и НЕИЗМЕНЁННОЙ программой, что при
    /// старте: под неё построены таблицы запуска — инлайн-кэши
    /// (`RunCaches`) и слоты модульных переменных.
    /// Программа с другой геометрией отказывает посреди исполнения, а
    /// правка `instrs` на месте между poll'ами опаснее — тёплая ячейка
    /// кэша помнит форму и слот, но не имя поля, и при совпавшей форме
    /// молча вернёт слот прежнего свойства. Раньше это страховали сброс
    /// ячеек в `image::finalize` и проверка образа; теперь инвариант
    /// держится этим контрактом.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания или исполнения.
    #[allow(clippy::too_many_arguments)]
    pub fn poll_with_registry_and_io<'a>(
        &mut self,
        program: &Program,
        registry: &bsl_rt::RuntimeRegistry,
        stdout: &'a mut dyn Write,
        stderr: &'a mut dyn Write,
        dynamic: &'a mut dyn DynamicCompiler,
        host_env: &'a mut bsl_rt::HostEnv,
        host_slice: usize,
    ) -> Result<ProgramPoll, RtError> {
        let linked = link_components(
            program,
            Some(registry),
            host_env.zone(),
            host_env.files(),
            host_env.random(),
            host_env.network(),
            host_env.background_jobs(),
            host_env.temp_storage(),
            host_env.message_sink(),
            bsl_bytecode::DynamicScope::ROOT,
        )?;
        let dynamic_depth = std::cell::Cell::new(0);
        let mut host = HostIo {
            stdout,
            stderr,
            env: Some(host_env),
            dynamic: Some(dynamic),
            dynamic_depth: &dynamic_depth,
        };
        self.poll_linked(program, &linked, None, &mut host, host_slice, None)
    }

    /// Конфигурационный аналог [`Self::poll_with_registry_and_io`]:
    /// исполняет entry поверх каталога общих модулей. Entry и каждый модуль
    /// линкуются на каждый poll — так же, как одиночный путь
    /// перелинковывает свою программу. Перед первым poll должен быть
    /// вызван [`Self::attach_catalog`].
    ///
    /// `entry` и `catalog` обязаны быть теми же и неизменёнными, что при
    /// старте и в `attach_catalog`, — по той же причине, что у
    /// [`Self::poll_with_registry_and_io`]: таблицы запуска, включая
    /// инлайн-кэши модулей, построены под них.
    ///
    /// # Errors
    ///
    /// Ошибки связывания любого модуля каталога и ошибки исполнения.
    #[allow(clippy::too_many_arguments)]
    pub fn poll_configuration_with_registry_and_io<'a>(
        &mut self,
        entry: &Program,
        catalog: &bsl_bytecode::ConfigurationProgram,
        registry: &bsl_rt::RuntimeRegistry,
        stdout: &'a mut dyn Write,
        stderr: &'a mut dyn Write,
        dynamic: &'a mut dyn DynamicCompiler,
        host_env: &'a mut bsl_rt::HostEnv,
        host_slice: usize,
    ) -> Result<ProgramPoll, RtError> {
        self.poll_configuration_with_budget(
            entry, catalog, registry, stdout, stderr, dynamic, host_env, host_slice, None,
        )
    }

    /// То же с бюджетом квантов планировщика: после `max_quanta`
    /// исчерпанных квантов poll возвращает `Runnable`, не дожидаясь
    /// завершения. Драйвер worker пула чередует НЕСКОЛЬКО заданий на
    /// одном потоке именно этим бюджетом; `None` — без предела.
    ///
    /// # Errors
    ///
    /// Те же, что у [`Self::poll_configuration_with_registry_and_io`].
    #[allow(clippy::too_many_arguments)]
    pub fn poll_configuration_with_budget<'a>(
        &mut self,
        entry: &Program,
        catalog: &bsl_bytecode::ConfigurationProgram,
        registry: &bsl_rt::RuntimeRegistry,
        stdout: &'a mut dyn Write,
        stderr: &'a mut dyn Write,
        dynamic: &'a mut dyn DynamicCompiler,
        host_env: &'a mut bsl_rt::HostEnv,
        host_slice: usize,
        quanta_budget: Option<usize>,
    ) -> Result<ProgramPoll, RtError> {
        let linked = link_components(
            entry,
            Some(registry),
            host_env.zone(),
            host_env.files(),
            host_env.random(),
            host_env.network(),
            host_env.background_jobs(),
            host_env.temp_storage(),
            host_env.message_sink(),
            bsl_bytecode::DynamicScope::ROOT,
        )?;
        // Области динамического кода модулей нумеруются с единицы: ROOT
        // принадлежит entry, и пересечение областей склеило бы кэши
        // фрагментов разных модулей.
        let mut linked_modules = Vec::with_capacity(catalog.modules.len());
        for (i, module) in catalog.modules.iter().enumerate() {
            linked_modules.push(link_components(
                &module.program,
                Some(registry),
                host_env.zone(),
                host_env.files(),
                host_env.random(),
                host_env.network(),
                host_env.background_jobs(),
                host_env.temp_storage(),
                host_env.message_sink(),
                i as u64 + 1,
            )?);
        }
        let ctx = CatalogContext {
            catalog,
            linked: linked_modules,
        };
        let dynamic_depth = std::cell::Cell::new(0);
        let mut host = HostIo {
            stdout,
            stderr,
            env: Some(host_env),
            dynamic: Some(dynamic),
            dynamic_depth: &dynamic_depth,
        };
        self.poll_linked(
            entry,
            &linked,
            Some(&ctx),
            &mut host,
            host_slice,
            quanta_budget,
        )
    }

    fn poll_linked(
        &mut self,
        program: &Program,
        linked: &LinkedComponents<'_>,
        catalog: Option<&CatalogContext<'_>>,
        host: &mut HostIo<'_, '_>,
        host_slice: usize,
        mut quanta_budget: Option<usize>,
    ) -> Result<ProgramPoll, RtError> {
        if self.finished {
            return Err(RtError::DynamicError(
                "завершённый Execution нельзя опрашивать повторно".into(),
            ));
        }
        if host_slice == 0 {
            return Ok(ProgramPoll::Runnable);
        }
        let Self {
            async_state,
            runtime_shapes,
            merge_linear,
            debug,
            root_result,
            module_state,
            caches,
            catalog_caches,
            session_modules,
            force_scheduled,
            cancel_flag,
            finished,
            ..
        } = self;
        let mut host_remaining = host_slice;

        loop {
            // Замороженный синхронный вызов: пока его host-операция не
            // завершилась, никакая другая задача не исполняется — Pending
            // синхронного метода останавливает весь execution. Холодная
            // ветка вынесена: укладка этой функции несёт быстрый путь
            // пустого цикла, и лишние байты здесь стоили DSB (измерено на
            // `empty_for`).
            let next_ready = if async_state.sync_wait.is_none() {
                async_state.ready.pop_front()
            } else {
                take_frozen_ready(async_state)
            };
            let Some(task_id) = next_ready else {
                if async_state.has_pending_host_promises() {
                    // Отмена, пришедшая во время host-ожидания: без этой
                    // проверки резидент, ждущий медленный транспорт,
                    // отменялся бы только после доставки ответа.
                    if let Some(flag) = &cancel_flag
                        && flag.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        return Err(RtError::Canceled);
                    }
                    let block = host_slice == usize::MAX;
                    let accepted =
                        async_state.drain_completions(host_remaining, block, runtime_shapes)?;
                    host_remaining = host_remaining.saturating_sub(accepted);
                    if accepted != 0 {
                        continue;
                    }
                    return Ok(ProgramPoll::Waiting);
                }
                if let Some(result) = root_result.take() {
                    if async_state.has_live_tasks() {
                        return Err(RtError::DynamicError(
                            "выполнение остановлено: нет готовых задач".into(),
                        ));
                    }
                    *finished = true;
                    return Ok(ProgramPoll::Complete(result.0, result.1));
                }
                return Err(RtError::DynamicError(
                    "выполнение остановлено до завершения корневой задачи".into(),
                ));
            };
            // Кооперативная отмена: граница кванта — единственная точка
            // проверки; неловимость RtError::Canceled ведёт разматывание
            // мимо «Попытки» — как измерено на платформе.
            if let Some(flag) = &cancel_flag
                && flag.load(std::sync::atomic::Ordering::Relaxed)
            {
                return Err(RtError::Canceled);
            }
            let mut task = async_state
                .tasks
                .get_mut(task_id)
                .and_then(Option::take)
                .ok_or(RtError::InvalidBytecode(
                    "готовая очередь ссылается на отсутствующую задачу",
                ))?;
            // Пробуждение синхронного host-вызова: применение результата
            // вынесено (см. `resume_parked_task`) — по той же причине
            // укладки, что и `take_frozen_ready`.
            if async_state.sync_wait.is_some()
                && !resume_parked_task(
                    &mut task,
                    task_id,
                    async_state,
                    program,
                    catalog,
                    session_modules,
                )?
            {
                continue;
            }
            let scheduled = async_state.has_other_live_task() || *force_scheduled;
            if task.quantum_remaining == 0 || !scheduled {
                task.quantum_remaining = async_state.scheduler_quantum();
            }

            loop {
                // После инициализации пустой numeric-for не обращается к
                // регистрам. Обслуживаем его back-edge в компактном внешнем цикле,
                // не входя на каждой итерации в большой универсальный `step`.
                // Логических итераций по-прежнему столько же: цикл не сворачивается
                // в вычисление финального значения.
                // Под отладчиком быстрый back-edge выключен: он завершает
                // итерацию, не доходя до крючка, и точки останова в теле
                // цикла срабатывали бы один раз вместо каждой итерации.
                // Измерено: цикл в три миллиона витков давал ОДНУ
                // остановку. Отладка медленнее — это её цена, а не изъян.
                // Под отладчиком быстрый back-edge выключен: он завершает
                // итерацию, не доходя до крючка.
                //
                // Проверка стоит ЗДЕСЬ, а не локальным `bool`, снятым до
                // цикла, и это измерено, а не выбрано: с локальным
                // `empty_for` дешевле (+6,0 % против +9,0 %), но
                // `call_overhead` вдвое дороже (+1,79 % против +0,81 %), а
                // `pi_leibniz` вдвое (+0,40 % против +0,18 %). Пустой цикл
                // — микромерка ради самого этого быстрого пути; вызовы и
                // арифметика ближе к настоящему коду, и платить решено там,
                // где дешевле для них.
                let fast_numeric_for = debug.is_none() && {
                    let frame = task
                        .frames
                        .last_mut()
                        .expect("инвариант VM: drive всегда держит хотя бы один кадр");
                    match frame.numeric_for_state.as_mut() {
                        Some(state) if state.pc == frame.pc => match state.current.checked_add(1) {
                            Some(next) if next <= state.bound => {
                                state.current = next;
                                true
                            }
                            _ => false,
                        },
                        _ => false,
                    }
                };
                if fast_numeric_for {
                    if consume_scheduler_safe_point(
                        &mut task,
                        scheduled,
                        async_state.scheduler_quantum(),
                    ) {
                        async_state.tasks[task_id] = Some(task);
                        async_state.ready.push_back(task_id);
                        if let Some(budget) = quanta_budget.as_mut() {
                            *budget = budget.saturating_sub(1);
                            if *budget == 0 {
                                return Ok(ProgramPoll::Runnable);
                            }
                        }
                        break;
                    }
                    continue;
                }

                // Модуль верхнего кадра определяет программу, линковку и
                // состояние модульных переменных этого шага. Резолв стоит
                // ПОСЛЕ быстрого numeric-for: пустой цикл не должен платить
                // за ветку и чтение поля на каждом back-edge. У одиночной
                // программы ветка всегда предсказана: модуль — ROOT_MODULE.
                let cur_module = task
                    .frames
                    .last()
                    .expect("инвариант VM: drive всегда держит хотя бы один кадр")
                    .module;
                let (cur_program, cur_linked, cur_caches) = if cur_module == ROOT_MODULE {
                    (program, linked, &*caches)
                } else {
                    let Some(ctx) = catalog else {
                        return Err(RtError::InvalidBytecode(
                            "кадр модуля конфигурации без каталожного контекста",
                        ));
                    };
                    ctx.execution_parts(cur_module, &*catalog_caches)?
                };
                // `step` исполняет целый VLIW-бандл (см. `bsl_bytecode::bundle`),
                // так что проверка fast numeric-for выше происходит на границах
                // бандлов, а не на каждой инструкции. При ошибке члена
                // `pc` стоит на нём самом, и `unwind_to_handler` находит обработчик
                // как при поинструкционном исполнении; обработчик по построению
                // разметки — начало бандла.
                // Крючок отладчика — ВО ВНЕШНЕМ цикле, перед шагом.
                // Внутри `step` ему делать нечего: диспетчер живёт на
                // грани uop-кэша, и лишняя проверка там стоила бы больше,
                // чем отладчик даёт.
                if let Some(hook) = debug.as_mut() {
                    let frames: Vec<(u32, usize, usize)> = task
                        .frames
                        .iter()
                        .map(|f| (f.module, f.func_id, f.pc))
                        .collect();
                    let line = task.frames.last().and_then(|f| {
                        cur_program
                            .lines
                            .get(f.func_id)
                            .and_then(|rows| rows.get(f.pc))
                            .copied()
                    });
                    // Модульное состояние здесь КОРНЕВОЕ: изъятие в
                    // `scratch_state` для чужого модуля идёт ниже, уже
                    // после крючка. Вычисление в кадре чужого модуля
                    // каталога поэтому увидит корневые переменные — это
                    // ограничение, а не случайность, и снимать его надо
                    // отдельной работой.
                    let mut values = FrameValues {
                        task: &mut task,
                        program: cur_program,
                        linked: cur_linked,
                        host,
                        module_state,
                    };
                    let mut at = DebugPosition {
                        frames: &frames,
                        line,
                        values: &mut values,
                    };
                    if hook.before_instruction(&mut at) == DebugAction::Terminate {
                        async_state.tasks[task_id] = Some(task);
                        return Err(RtError::DynamicError("прогон прекращён отладчиком".into()));
                    }
                }
                let before = task_position(&task);
                // Состояние модульных переменных текущего модуля на время
                // шага изымается из сессии: `step` видит его обычным
                // `module_state`, а чужие модули достаёт через сессию, в
                // которой изъятая ячейка не встречается (self-link запрещён
                // периметром образа).
                let mut scratch_state = ModuleState { slots: Vec::new() };
                if cur_module != ROOT_MODULE {
                    std::mem::swap(
                        &mut scratch_state.slots,
                        &mut session_modules.instances[cur_module as usize].state.slots,
                    );
                }
                // Кадру модуля корневое состояние отдаётся отдельной
                // ссылкой: копибэк `ByRefModuleVar`, созданный корневым
                // кадром, пишется при возврате из модульного.
                let (step_state, step_root): (&mut ModuleState, Option<&mut ModuleState>) =
                    if cur_module == ROOT_MODULE {
                        (&mut *module_state, None)
                    } else {
                        (&mut scratch_state, Some(&mut *module_state))
                    };
                let mut modules_ctx = ModulesCtx {
                    session: session_modules,
                    catalog,
                    root_state: step_root,
                };
                let step_result = step(
                    &mut task.frames,
                    &mut task.stack,
                    cur_program,
                    cur_caches,
                    step_state,
                    &mut modules_ctx,
                    &mut task.current_exception,
                    runtime_shapes,
                    cur_linked,
                    host,
                    *merge_linear && !scheduled,
                    async_state,
                    task_id,
                );
                if cur_module != ROOT_MODULE {
                    std::mem::swap(
                        &mut scratch_state.slots,
                        &mut session_modules.instances[cur_module as usize].state.slots,
                    );
                }
                match step_result {
                    Ok(Step::Continue) => {
                        // Приостанавливающий метод припарковал задачу:
                        // `pc` стоит на его инструкции, бандл дорван
                        // пустыми повторами. Проверка обязана идти первой:
                        // requeue через safe point вернул бы задачу в
                        // готовые до завершения host-операции.
                        if async_state.sync_wait.is_some() {
                            async_state.tasks[task_id] = Some(task);
                            break;
                        }
                        if crossed_scheduler_safe_point(before, &task)
                            && consume_scheduler_safe_point(
                                &mut task,
                                scheduled,
                                async_state.scheduler_quantum(),
                            )
                        {
                            async_state.tasks[task_id] = Some(task);
                            async_state.ready.push_back(task_id);
                            if let Some(budget) = quanta_budget.as_mut() {
                                *budget = budget.saturating_sub(1);
                                if *budget == 0 {
                                    return Ok(ProgramPoll::Runnable);
                                }
                            }
                            break;
                        }
                        continue;
                    }
                    Ok(Step::Yield) => {
                        async_state.tasks[task_id] = Some(task);
                        async_state.ready.push_back(task_id);
                        if let Some(budget) = quanta_budget.as_mut() {
                            *budget = budget.saturating_sub(1);
                            if *budget == 0 {
                                return Ok(ProgramPoll::Runnable);
                            }
                        }
                        break;
                    }
                    Ok(Step::StartAsync(child_id)) => {
                        async_state.tasks[task_id] = Some(task);
                        // Async-callee исполняется немедленно до первого `Await`.
                        // Вызывающий продолжает сразу после него; задачи, уже
                        // стоявшие в FIFO, остаются за этой парой.
                        async_state.ready.push_front(task_id);
                        async_state.ready.push_front(child_id);
                        break;
                    }
                    Ok(Step::Suspend) => {
                        async_state.tasks[task_id] = Some(task);
                        break;
                    }
                    Ok(Step::Done(value)) => {
                        match task.completion {
                            TaskCompletion::Root => *root_result = Some((value, task.stack)),
                            TaskCompletion::Promise(promise_id) => {
                                async_state.resolve_promise(promise_id, Ok(value))?;
                            }
                            TaskCompletion::Detached => {}
                        }
                        break;
                    }
                    Err(e) => {
                        if !unwind_to_handler(
                            &mut task.frames,
                            &mut task.stack,
                            program,
                            catalog,
                            session_modules,
                            &e,
                            &mut task.current_exception,
                        ) {
                            match task.completion {
                                TaskCompletion::Root | TaskCompletion::Detached => return Err(e),
                                TaskCompletion::Promise(promise_id) => {
                                    async_state.resolve_promise(promise_id, Err(e))?;
                                    break;
                                }
                            }
                        }
                        // Иначе кадры/pc уже поправлены внутри unwind_to_handler —
                        // просто продолжаем цикл со следующей итерации.
                    }
                }
            }

            if root_result.is_some() && !async_state.has_live_tasks() {
                let result = root_result.take().expect("результат проверен выше");
                *finished = true;
                return Ok(ProgramPoll::Complete(result.0, result.1));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_linked(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
    linked: &LinkedComponents,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
    debug: Option<Box<dyn DebugHook>>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let owned_module_state = ModuleState {
        slots: std::mem::take(&mut module_state.slots),
    };
    let mut execution = ProgramExecution::new_linked(
        program,
        func_id,
        stack,
        linked,
        owned_module_state,
        SchedulerConfig::default(),
    );
    if let Some(hook) = debug {
        execution.set_debug_hook(hook);
    }
    let result = loop {
        match execution.poll_linked(program, linked, None, host, usize::MAX, None) {
            Ok(ProgramPoll::Complete(value, stack)) => break Ok((value, stack)),
            Ok(ProgramPoll::Runnable | ProgramPoll::Waiting) => continue,
            Err(error) => break Err(error),
        }
    };
    module_state.slots = execution.module_state.slots;
    result
}

enum Step {
    Continue,
    Yield,
    StartAsync(TaskId),
    Suspend,
    Done(BslValue),
}

// --- Доступ к байт-коду и регистрам ------------------------------------
//
// Ни одна паника в этом модуле не должна зависеть от ВХОДНЫХ ДАННЫХ. Всё,
// что инструкция читает по индексу — номер регистра, номер чанка,
// константы, формы, имени, аргумент builtin'а, — приходит из `Program`, а
// `Program` VM получает не только от собственного кодогена: её собирает
// `Выполнить`/`Вычислить` в рантайме, REPL `bsl-cli` по строке, и любой
// внешний пользователь публичных `run_program` и
// `run_repl_chunk_with_registry`. Поэтому такие обращения дают
// `RtError::InvalidBytecode`, а не роняют процесс.
//
// Остаются ровно два `expect()` — на `frames.pop()` в
// `do_return_with_value` и `unwind_to_handler`: это внутренние инварианты
// самого цикла диспетчеризации (стек кадров непуст, пока мы исполняем
// инструкцию), недостижимые никаким байт-кодом. Голых `unwrap()` вне
// тестов нет ни одного.

// Три следующие функции помечены `#[inline(always)]`, а не подсказкой
// `#[inline]`, намеренно. Это самые горячие операции цикла диспетчеризации,
// и решение инлайнера по ним оплачивает не тот код, который его сдвинул:
// коммит, добавивший к `match` в `step` один опкод для JSON, перевесил
// бюджет инлайнера, `reg_load` выехал наружу — и `pi_leibniz`, ни о каком
// JSON не знающий, замедлился на десятую часть (2,98 -> 3,16 млрд
// инструкций, 11% времени в вызовах `reg_load`). Пока диспетчер — одна
// огромная функция, размер которой меняется с каждым новым опкодом, такую
// связь надо разрывать явно.
#[inline(always)]
fn at<'a, T>(xs: &'a [T], i: usize, what: &'static str) -> Result<&'a T, RtError> {
    xs.get(i).ok_or_else(|| RtError::InvalidBytecode(what))
}

#[inline(always)]
fn reg_load(stack: &[BslValue], i: usize) -> Result<BslValue, RtError> {
    stack
        .get(i)
        .cloned()
        .ok_or_else(|| RtError::InvalidBytecode("чтение регистра за границей стека значений"))
}

#[inline(always)]
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

/// Первый заход в `Instr::NumericForNextI64`: счётчик и граница ещё не
/// сняты в `i64`. Отдельной функцией с `#[inline(never)]`, потому что на
/// цикл это выполняется однажды, а код занимает место ровно там, где
/// крутится тело цикла — а горячий путь диспетчера живёт на грани кеша
/// микроопераций (см. комментарий у `step_cold`).
///
/// `Ok(None)` означает, что цикл не уложился в `i64` и уже отработан
/// общим путём `numeric_for_next_regular`.
///
/// # Errors
///
/// Возвращает ошибку чтения регистра за границей стека значений или
/// ошибку общего пути.
#[inline(never)]
fn numeric_for_i64_start(
    stack: &mut [BslValue],
    counter_idx: usize,
    bound_idx: usize,
    pc: usize,
    frame_pc: &mut usize,
    target: i16,
) -> Result<Option<NumericForState>, RtError> {
    let counter_value = reg_load(stack, counter_idx)?;
    let bound_value = reg_load(stack, bound_idx)?;
    let pair = match (&counter_value, &bound_value) {
        (BslValue::Number(counter), BslValue::Number(bound)) => {
            counter.to_i64_exact().zip(bound.to_i64_exact())
        }
        _ => None,
    };
    let Some((current, bound)) = pair else {
        numeric_for_next_regular(stack, counter_idx, bound_idx, frame_pc, target)?;
        return Ok(None);
    };
    Ok(Some(NumericForState { pc, current, bound }))
}

/// Переполнение `i64` на инкременте счётчика: цикл дошёл до `i64::MAX` и
/// дальше считается общим путём. Вынесено по той же причине, что и
/// [`numeric_for_i64_start`], — недостижимый на практике код не должен
/// занимать место в теле горячего цикла.
///
/// # Errors
///
/// Возвращает ошибку записи регистра за границей стека значений или
/// ошибку общего пути.
#[inline(never)]
fn numeric_for_i64_overflow(
    stack: &mut [BslValue],
    counter_idx: usize,
    bound_idx: usize,
    current: i64,
    frame_pc: &mut usize,
    target: i16,
) -> Result<(), RtError> {
    reg_store(
        stack,
        counter_idx,
        BslValue::Number(bsl_number::BslNumber::from_i64(current)),
    )?;
    numeric_for_next_regular(stack, counter_idx, bound_idx, frame_pc, target)
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
/// Конструктор состояния запуска заводит её вместе со всеми остальными
/// ячейками программы.
#[inline]
fn prop_cache(caches: &RunCaches, func_id: usize, pc: usize) -> Result<&PropCacheSlot, RtError> {
    let chunk = at(&caches.prop, func_id, "номер чанка вне инлайн-кэша свойств")?;
    at(chunk, pc, "нет ячейки инлайн-кэша для инструкции")
}

/// Ячейка инлайн-кэша `CallObjectMethod` на позиции `pc` — см.
/// [`cached_component_method`].
#[inline]
fn method_cache(
    caches: &RunCaches,
    func_id: usize,
    pc: usize,
) -> Result<&MethodCacheSlot, RtError> {
    let chunk = at(
        &caches.method,
        func_id,
        "номер чанка вне инлайн-кэша методов",
    )?;
    at(chunk, pc, "нет ячейки кэша метода для инструкции")
}

/// Разрешение метода компонентного объекта с кэшем на позиции инструкции:
/// мономорфный сайт после первого вызова читает обработчик из своей ячейки
/// по одному сравнению адреса таблицы, не трогая карту мемоизации. Смена
/// типа получателя на том же сайте (полиморфизм) перечитывает карту и
/// перезаписывает ячейку; `None` кэшируется наравне с попаданием — тип без
/// имени в таблице не платит за строку и хэш на каждый вызов.
fn cached_component_method(
    caches: &RunCaches,
    func_id: usize,
    pc: usize,
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<&'static bsl_rt::MethodDescriptor>, RtError> {
    let slot = method_cache(caches, func_id, pc)?;
    let key = table.as_ptr() as usize;
    if let Some((cached_table, resolved)) = *slot.borrow()
        && cached_table == key
    {
        return Ok(resolved);
    }
    let resolved = resolve_component_method(map, table, name, program)?;
    *slot.borrow_mut() = Some((key, resolved));
    Ok(resolved)
}

/// Оригинальное написание имени поля — нужно строковому пути доступа
/// (`СтрокаТаблицыЗначений`, `КлючИЗначение`), у которого нет формы.
#[inline]
fn field_name(program: &Program, name: bsl_rt::NameId) -> Result<&str, RtError> {
    at(
        &program.names,
        name.index(),
        "идентификатор имени вне таблицы имён программы",
    )
    .map(|s| s.as_str())
}

/// Счётчики исполненных опкодов — только сборка `--features counters`.
///
/// Поток исполненных инструкций байт-кода НЕ зависит от кодогенерации:
/// инлайнинг и раскладка меняют время, но не последовательность
/// инструкций. Поэтому числа, снятые счётной сборкой, верны и для
/// release, а горячий цикл release остаётся нетронутым — под обычной
/// сборкой этого модуля не существует вовсе, вместе с крючком в `step`.
/// Именно поэтому `cfg` здесь не нарушает измеренный бюджет диспетчера.
#[cfg(feature = "counters")]
pub mod counters {
    use bsl_bytecode::{Chunk, Instr, OPCODE_COUNT, OPCODES};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Ярус представления операнда, различимый публичным API `BslNumber`.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Tier {
        /// Целое, помещающееся в `i64`: счётчики, индексы, смещения.
        Int64,
        /// Число, но не целое в `i64`: масштаб либо ширина мантиссы.
        Decimal,
        /// Не число вовсе.
        Other,
    }

    fn tier(v: &bsl_rt::BslValue) -> Tier {
        match v {
            bsl_rt::BslValue::Number(n) => {
                if n.to_i64_exact().is_some() {
                    Tier::Int64
                } else {
                    Tier::Decimal
                }
            }
            _ => Tier::Other,
        }
    }

    struct State {
        by_opcode: Vec<u64>,
        moves_removable: u64,
        /// Арифметика и сравнения: сколько раз ОБА операнда оказались
        /// целыми в `i64`, сколько — числами вне этого яруса, сколько —
        /// не числами. Это и есть ответ на вопрос, окупится ли
        /// специализация представления, — до того как её писать.
        arith: Vec<[u64; 3]>,
        /// Таблицы устранимости, по одной на чанк; ключ — его адрес.
        /// Чанки живут в `Program` весь прогон, поэтому адрес стабилен.
        tables: HashMap<usize, Rc<Vec<bool>>>,
    }

    thread_local! {
        static STATE: RefCell<State> = RefCell::new(State {
            by_opcode: vec![0; OPCODE_COUNT],
            moves_removable: 0,
            arith: vec![[0; 3]; OPCODE_COUNT],
            tables: HashMap::new(),
        });
    }

    /// Учесть одну исполненную инструкцию.
    pub fn tick(instr: &Instr, chunk: &Chunk, pc: usize, overlap: Option<usize>) {
        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            st.by_opcode[instr.opcode_index()] += 1;
            if !matches!(instr, Instr::Move { .. }) {
                return;
            }
            let key = std::ptr::from_ref(chunk) as usize;
            let table = match st.tables.get(&key) {
                Some(t) => Rc::clone(t),
                None => {
                    let t = Rc::new(bsl_bytecode::analysis::removable_copies(chunk, overlap));
                    st.tables.insert(key, Rc::clone(&t));
                    t
                }
            };
            if table.get(pc).copied().unwrap_or(false) {
                st.moves_removable += 1;
            }
        });
    }

    /// Учесть ярусы операндов арифметической инструкции.
    pub fn tick_arith(instr: &Instr, a: &bsl_rt::BslValue, b: &bsl_rt::BslValue) {
        let (ta, tb) = (tier(a), tier(b));
        let bucket = if ta == Tier::Other || tb == Tier::Other {
            2
        } else if ta == Tier::Int64 && tb == Tier::Int64 {
            0
        } else {
            1
        };
        STATE.with(|cell| cell.borrow_mut().arith[instr.opcode_index()][bucket] += 1);
    }

    /// Отчёт в TSV: строка на опкод, затем итоги по копиям.
    pub fn report() -> String {
        STATE.with(|cell| {
            let st = cell.borrow();
            let total: u64 = st.by_opcode.iter().sum();
            let mut out = String::from("# опкод\tисполнений\tдоля\n");
            let mut rows: Vec<(usize, u64)> = st
                .by_opcode
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, n)| *n > 0)
                .collect();
            rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            for (i, n) in rows {
                let share = if total == 0 {
                    0.0
                } else {
                    n as f64 * 100.0 / total as f64
                };
                out.push_str(&format!("{}\t{}\t{:.2}\n", OPCODES[i], n, share));
            }
            let moves = st.by_opcode[Instr::Move { dst: 0, src: 0 }.opcode_index()];
            out.push_str(&format!("# всего инструкций\t{total}\n"));
            out.push_str(&format!("# Move исполнено\t{moves}\n"));
            out.push_str(&format!("# Move устранимых\t{}\n", st.moves_removable));
            let share = if moves == 0 {
                0.0
            } else {
                st.moves_removable as f64 * 100.0 / moves as f64
            };
            out.push_str(&format!("# доля устранимых среди Move\t{share:.2}\n"));
            out.push_str("# арифметика: опкод\tоба Int64\tчисла вне Int64\tне числа\tдоля Int64\n");
            for (i, b) in st.arith.iter().enumerate() {
                let total = b[0] + b[1] + b[2];
                if total == 0 {
                    continue;
                }
                let share = b[0] as f64 * 100.0 / total as f64;
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{:.2}\n",
                    OPCODES[i], b[0], b[1], b[2], share
                ));
            }
            out
        })
    }
}

/// Выполняет один VLIW-бандл текущего (верхнего) кадра: от одной
/// инструкции (одиночный бандл) до `Chunk::bundle_len[pc]` подряд — без
/// возврата в `drive_with` между членами. При `merge_linear` исполнение
/// продолжается и через
/// границу бандла, пока `pc` идёт линейно: пробы `drive_with` имеют смысл
/// только там, куда `pc` попадает переходом, вызовом или разматыванием.
#[allow(clippy::too_many_arguments)]
fn step(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    caches: &RunCaches,
    module_state: &mut ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    current_exception: &mut Option<BslValue>,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    linked: &LinkedComponents,
    host: &mut HostIo<'_, '_>,
    merge_linear: bool,
    async_state: &mut AsyncState,
    task_id: TaskId,
) -> Result<Step, RtError> {
    let frame_idx = frames.len() - 1;
    let func_id = frames[frame_idx].func_id;
    let mut pc = frames[frame_idx].pc;
    let chunk = at(&program.chunks, func_id, "номер чанка вне таблицы функций")?;

    if pc >= chunk.instrs.len() {
        // Неявный возврат: тело кончилось без `Возврат` — результат
        // Неопределено, как и `Возврат;` без выражения.
        return Ok(
            match do_return_with_value(frames, stack, module_state, modules, BslValue::Undefined)? {
                Done(v) => Step::Done(v),
                Continuing => Step::Continue,
            },
        );
    }

    // Ширина VLIW-бандла на этой позиции (см. `bsl_bytecode::bundle`).
    // Члены бандла взаимно независимы, передача управления бывает только
    // хвостовым членом, поэтому кадр и чанк между членами можно не
    // перечитывать: `Call` и `Return` дальше хвоста не встречаются, а
    // `Return` к тому же выходит из функции ранним `return`. Ноль
    // Ноль в середине бандла и пустая таблица равнозначны одиночному
    // исполнению.
    // Разметке можно верить, потому что из файла она не читается — её
    // всегда пересчитывает `bundle::compute`; ошибка члена оставляет `pc`
    // на нём самом, и `Попытка` ищется ровно как при поинструкционном
    // исполнении.
    // «Сколько членов ЗА первым»: у одиночного бандла и в середине бандла
    // ноль — путь обычной инструкции оплачивает ровно одну загрузку `u8`
    // и вычитание, вся петлевая бухгалтерия лежит после исполнения члена.
    let mut extra = chunk
        .bundle_len()
        .get(pc)
        .copied()
        .unwrap_or(1)
        .saturating_sub(1);
    loop {
        // `pc < chunk.instrs.len()` проверено выше (для последующих
        // членов — перед переходом на них) — индексация здесь уже не
        // может выйти за границы.
        let instr = chunk.instrs[pc];
        #[cfg(feature = "counters")]
        {
            counters::tick(
                &instr,
                chunk,
                pc,
                bsl_bytecode::analysis::module_overlap(func_id, program.module_vars.len()),
            );
            // Ярусы операндов снимаются здесь же: специализация
            // представления по плану следует за динамической
            // статистикой, а не предшествует ей.
            if let Instr::Add { a, b, .. }
            | Instr::Sub { a, b, .. }
            | Instr::Mul { a, b, .. }
            | Instr::Div { a, b, .. }
            | Instr::Mod { a, b, .. }
            | Instr::Eq { a, b, .. }
            | Instr::NotEq { a, b, .. }
            | Instr::Lt { a, b, .. }
            | Instr::Gt { a, b, .. }
            | Instr::Le { a, b, .. }
            | Instr::Ge { a, b, .. } = instr
            {
                let ia = frames[frame_idx].reg_index(a);
                let ib = frames[frame_idx].reg_index(b);
                if let (Some(va), Some(vb)) = (stack.get(ia), stack.get(ib)) {
                    counters::tick_arith(&instr, va, vb);
                }
            }
        }
        match instr {
            Instr::GetModuleVar { dst, slot } => {
                // Номер в ОТДЕЛЬНОМ блоке модульных переменных, а не в
                // стеке: блок живёт всё исполнение и переносится явно.
                // Проверка границы остаётся, хотя её же делает и периметр:
                // байт-код может прийти и не от кодогена, а сторожить путь
                // дешевле, чем доказывать, что другого нет.
                if (slot as usize) >= program.module_vars.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер переменной модуля вне таблицы",
                    ));
                }
                let v = reg_load(&module_state.slots, slot as usize)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetModuleVar { slot, src } => {
                if (slot as usize) >= program.module_vars.len() {
                    return Err(RtError::InvalidBytecode(
                        "номер переменной модуля вне таблицы",
                    ));
                }
                let v = reg_load(stack, frames[frame_idx].reg_index(src))?;
                reg_store(&mut module_state.slots, slot as usize, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Move { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = reg_load(stack, s)?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::LoadConst { dst, k } => {
                let v = at(
                    &chunk.consts,
                    k as usize,
                    "номер константы вне таблицы констант чанка",
                )?
                .value()
                .clone();
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
            Instr::Add { dst, a, b } => {
                add_op(frames, stack, frame_idx, dst, a, b)?;
                frames[frame_idx].pc += 1;
            }
            Instr::AddConst { dst, src, k } => {
                add_const_op(program, frames, stack, frame_idx, dst, src, k)?;
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
            Instr::Mod { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::rem)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Div { dst, a, b } => {
                binop(frames, stack, frame_idx, dst, a, b, BslValue::div)?;
                frames[frame_idx].pc += 1;
            }
            Instr::Neg { dst, src } => {
                let s = frames[frame_idx].reg_index(src);
                let v = neg_op(&reg_load(stack, s)?)?;
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
                cmp(frames, stack, frame_idx, dst, a, b, "<", |o| o.is_lt())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Gt { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">", |o| o.is_gt())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Le { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, "<=", |o| o.is_le())?;
                frames[frame_idx].pc += 1;
            }
            Instr::Ge { dst, a, b } => {
                cmp(frames, stack, frame_idx, dst, a, b, ">=", |o| o.is_ge())?;
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
            Instr::JumpIfNotEqConst { src, k, target } => {
                let chunk = at(
                    &program.chunks,
                    frames[frame_idx].func_id,
                    "номер чанка вне таблицы функций",
                )?;
                let value = reg_load(stack, frames[frame_idx].reg_index(src))?;
                let constant = at(
                    &chunk.consts,
                    k as usize,
                    "номер константы вне таблицы констант чанка",
                )?;
                if value.eq_value(constant) {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::JumpIfNotLtConst { src, k, target } => {
                let chunk = at(
                    &program.chunks,
                    frames[frame_idx].func_id,
                    "номер чанка вне таблицы функций",
                )?;
                let value = reg_load(stack, frames[frame_idx].reg_index(src))?;
                let constant = at(
                    &chunk.consts,
                    k as usize,
                    "номер константы вне таблицы констант чанка",
                )?;
                if value.compare(constant, "<")?.is_lt() {
                    frames[frame_idx].pc += 1;
                } else {
                    frames[frame_idx].pc = target as usize;
                }
            }
            Instr::JumpIfNotSkipped { src, target } => {
                // Не условие пользовательского кода, а метаданные кадра:
                // передали ли аргумент на месте вызова (см. пролог
                // параметров по умолчанию в
                // `bsl-bytecode::compiler::compile_param_defaults`).
                // Содержимое слота при этом не читается вовсе — иначе
                // явно переданное `Неопределено` было бы неотличимо от
                // пропуска.
                //
                // `None` здесь означает РОВНО ОДНО: кадр заведён не
                // инструкцией `Call` (вызов по имени из Rust, фрагмент
                // `Выполнить`, верхний уровень), и пропустить аргумент
                // вызывающему было нечем. Случай «`src` за числом
                // параметров» сюда не доходит: он статический и отсечён
                // при связывании (`check_call_geometry`), иначе выглядел
                // бы точно так же — как переданный аргумент.
                let provided = frames[frame_idx]
                    .param_aliases
                    .get(src as usize)
                    .is_none_or(|slot| slot.provided);
                if provided {
                    frames[frame_idx].pc = target as usize;
                } else {
                    frames[frame_idx].pc += 1;
                }
            }
            Instr::NumericForNext {
                counter,
                bound,
                target,
            } => {
                let counter = frames[frame_idx].reg_index(counter);
                let bound = frames[frame_idx].reg_index(bound);
                numeric_for_next_regular(stack, counter, bound, &mut frames[frame_idx].pc, target)?;
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
                        ));
                    }
                    None => {
                        match numeric_for_i64_start(
                            stack,
                            counter_idx,
                            bound_idx,
                            pc,
                            &mut frames[frame_idx].pc,
                            target,
                        )? {
                            Some(state) => state,
                            None => return Ok(Step::Continue),
                        }
                    }
                };

                let Some(next) = state.current.checked_add(1) else {
                    numeric_for_i64_overflow(
                        stack,
                        counter_idx,
                        bound_idx,
                        state.current,
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
                let modes = at(
                    &chunk.call_arg_modes,
                    arg_modes as usize,
                    "номер набора режимов аргументов вне таблицы чанка",
                )?;
                let callee_chunk = at(
                    &program.chunks,
                    func as usize,
                    "номер вызываемого чанка вне таблицы функций",
                )?;

                if callee_chunk.is_async {
                    let mut child_stack = Vec::with_capacity(callee_chunk.n_regs as usize);
                    let mut param_aliases = Vec::with_capacity(modes.len());
                    for (i, mode) in modes.iter().enumerate() {
                        let (value, provided) = match mode {
                            ArgMode::Value => (
                                reg_load(stack, frames[frame_idx].reg_index(base + i as u8))?,
                                true,
                            ),
                            ArgMode::ByRefLocal(slot) => {
                                (reg_load(stack, frames[frame_idx].reg_index(*slot))?, true)
                            }
                            ArgMode::ByRefModuleVar(slot) => {
                                (reg_load(&module_state.slots, *slot as usize)?, true)
                            }
                            ArgMode::ByRefImportedVar(_) => {
                                return Err(RtError::InvalidBytecode(
                                    "режим byimport вне каталога конфигурации",
                                ));
                            }
                            ArgMode::Default => (BslValue::Undefined, false),
                        };
                        let idx = child_stack.len();
                        child_stack.push(value);
                        param_aliases.push(ParamSlot { idx, provided });
                    }
                    push_own_registers(&mut child_stack, callee_chunk);

                    let (completion, call_result) = if callee_chunk.is_procedure {
                        (TaskCompletion::Detached, BslValue::Undefined)
                    } else {
                        let (promise_id, promise) = async_state.new_promise()?;
                        (TaskCompletion::Promise(promise_id), promise)
                    };
                    let dst = frames[frame_idx].reg_index(ret);
                    reg_store(stack, dst, call_result)?;
                    frames[frame_idx].pc += 1;
                    let child_id = async_state.insert_task(Task {
                        frames: vec![Frame {
                            module: frames[frame_idx].module,
                            func_id: func as usize,
                            pc: 0,
                            param_aliases,
                            own_base: callee_chunk.n_params as usize,
                            call_start: 0,
                            return_reg: 0,
                            module_copybacks: Vec::new(),
                            numeric_for_state: None,
                        }],
                        stack: child_stack,
                        current_exception: None,
                        completion,
                        quantum_remaining: async_state.scheduler_quantum(),
                    });
                    return Ok(Step::StartAsync(child_id));
                }

                // Проверка глубины — ДО продвижения `pc` и до любых записей
                // в стек: в момент ошибки `pc` обязан стоять на сбойнувшей
                // инструкции, иначе `Попытка`, у которой этот `Call` —
                // последняя инструкция защищённого диапазона, его не поймает.
                if frames.len() >= MAX_CALL_DEPTH {
                    return Err(RtError::StackOverflow {
                        what: "слишком глубокая рекурсия вызовов",
                    });
                }

                // Caller продвигается ЗА инструкцию Call сейчас — так, когда
                // callee вернётся, мы продолжим ровно со следующей.
                frames[frame_idx].pc += 1;

                // `base + i` считается в `u8` без проверки: связывание уже
                // удостоверилось, что `base + modes.len() <= n_regs <= 255`
                // (`check_call_geometry`). Без той проверки номер
                // заворачивался, и аргумент становился алиасом чужого
                // регистра вызывающего.
                let mut param_aliases = Vec::with_capacity(modes.len());
                let mut module_copybacks = Vec::new();
                for (i, mode) in modes.iter().enumerate() {
                    let slot = match mode {
                        ArgMode::Value => ParamSlot {
                            idx: frames[frame_idx].reg_index(base + i as u8),
                            provided: true,
                        },
                        ArgMode::ByRefLocal(slot) => ParamSlot {
                            idx: frames[frame_idx].reg_index(*slot),
                            provided: true,
                        },
                        // Модульная переменная лежит по АБСОЛЮТНОМУ индексу
                        // (первые слоты кадра нулевого уровня), а не в
                        // кадре вызывающего: алиас указывает
                        // прямо туда, поэтому запись из вызванной функции
                        // видна и телу модуля, и другим функциям.
                        ArgMode::ByRefModuleVar(slot) => {
                            let module_slot = *slot as usize;
                            let value = reg_load(&module_state.slots, module_slot)?;
                            let idx = stack.len();
                            stack.push(value);
                            module_copybacks.push((idx, frames[frame_idx].module, module_slot));
                            ParamSlot {
                                idx,
                                provided: true,
                            }
                        }
                        // Импортированная переменная по ссылке появляется
                        // только внутри каталога конфигурации.
                        ArgMode::ByRefImportedVar(_) => {
                            return Err(RtError::InvalidBytecode(
                                "режим byimport вне каталога конфигурации",
                            ));
                        }
                        // Вызывающий в этот регистр ничего не вычислял, там
                        // лежит мусор от прошлого использования временного
                        // слота. Пролог умолчаний вызванной функции обязан
                        // записать туда значение раньше любого чтения — но
                        // «обязан» здесь про КОДОГЕН, а листинг байт-кода
                        // приходит и извне. Поэтому слот обнуляется явно:
                        // испорченный листинг даст `Неопределено`, а не
                        // случайное значение чужого выражения.
                        ArgMode::Default => {
                            let idx = frames[frame_idx].reg_index(base + i as u8);
                            reg_store(stack, idx, BslValue::Undefined)?;
                            ParamSlot {
                                idx,
                                provided: false,
                            }
                        }
                    };
                    param_aliases.push(slot);
                }

                let call_start = stack.len();
                let own_base = stack.len();
                push_own_registers(stack, callee_chunk);

                frames.push(Frame {
                    module: frames[frame_idx].module,
                    func_id: func as usize,
                    pc: 0,
                    param_aliases,
                    own_base,
                    call_start,
                    return_reg: ret,
                    module_copybacks,
                    numeric_for_state: None,
                });
            }
            Instr::Await { dst, promise } => {
                let value = reg_load(stack, frames[frame_idx].reg_index(promise))?;
                let Some((token, promise_id)) = value.promise_identity() else {
                    let dst = frames[frame_idx].reg_index(dst);
                    reg_store(stack, dst, value)?;
                    frames[frame_idx].pc += 1;
                    return Ok(Step::Yield);
                };
                if token != async_state.token {
                    return Err(RtError::DynamicError(
                        "обещание принадлежит другому запуску".into(),
                    ));
                }
                let promise_index = usize::try_from(promise_id.get()).map_err(|_| {
                    RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы")
                })?;
                let state =
                    async_state
                        .promises
                        .get_mut(promise_index)
                        .ok_or(RtError::InvalidBytecode(
                            "номер обещания вне таблицы запуска",
                        ))?;
                match state {
                    PromiseState::Pending { waiters } => {
                        if !waiters.contains(&task_id) {
                            waiters.push_back(task_id);
                        }
                        return Ok(Step::Suspend);
                    }
                    PromiseState::Ready(result) => {
                        let value = result.clone()?;
                        let dst = frames[frame_idx].reg_index(dst);
                        reg_store(stack, dst, value)?;
                        frames[frame_idx].pc += 1;
                        return Ok(Step::Yield);
                    }
                }
            }
            Instr::Return { src } => {
                let value = match src {
                    Some(r) => {
                        let idx = frames[frame_idx].reg_index(r);
                        reg_load(stack, idx)?
                    }
                    None => BslValue::Undefined,
                };
                return Ok(
                    match do_return_with_value(frames, stack, module_state, modules, value)? {
                        Done(v) => Step::Done(v),
                        Continuing => Step::Continue,
                    },
                );
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
                // Структура резолвится через инлайн-кэш этой ИНСТРУКЦИИ,
                // живущий в состоянии запуска: мономорфный сайт вызова после
                // первого попадания читает слот напрямую, без HashMap-
                // поиска в Shape::index. СтрокаТаблицыЗначений заводит
                // колонки в рантайме и не могла быть интернирована на
                // этапе компиляции — для неё (и только когда кэш-путь
                // говорит "это не такой объект") VM резолвит имя в текст
                // через Program::names и идёт по строковому пути.
                let v = if let Some(object) = ov.object_ref() {
                    let mut context =
                        bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                            runtime_shapes,
                            stdout: &mut *host.stdout,
                            stderr: &mut *host.stderr,
                            formatter: bsl_format::format_value,
                            zone: &linked.zone,
                            files: &linked.files,
                            random: &linked.random,
                            network: linked.network.as_ref(),
                            background_jobs: linked.background_jobs.as_ref(),
                            temp_storage: linked.temp_storage.as_ref(),
                            message_sink: linked.message_sink.as_ref(),
                            host_promises: None,
                            function_caller: None,
                        });
                    component_prop_get(
                        object,
                        &linked.component_properties,
                        name,
                        program,
                        &mut context,
                    )?
                } else {
                    match ov.get_field_cached(name, prop_cache(caches, func_id, pc)?) {
                        Err(RtError::NotAnObject) => {
                            ov.get_field_by_name(field_name(program, name)?)?
                        }
                        other => other?,
                    }
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            Instr::SetProp { obj, name, src } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let sv = reg_load(stack, frames[frame_idx].reg_index(src))?;
                if let Some(object) = ov.object_ref() {
                    let mut context =
                        bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                            runtime_shapes,
                            stdout: &mut *host.stdout,
                            stderr: &mut *host.stderr,
                            formatter: bsl_format::format_value,
                            zone: &linked.zone,
                            files: &linked.files,
                            random: &linked.random,
                            network: linked.network.as_ref(),
                            background_jobs: linked.background_jobs.as_ref(),
                            temp_storage: linked.temp_storage.as_ref(),
                            message_sink: linked.message_sink.as_ref(),
                            host_promises: None,
                            function_caller: None,
                        });
                    component_prop_set(
                        object,
                        &linked.component_properties,
                        name,
                        sv,
                        program,
                        &mut context,
                    )?;
                } else {
                    let имя = field_name(program, name)?;
                    match ov.set_field_cached(name, sv.clone(), prop_cache(caches, func_id, pc)?) {
                        Err(RtError::NotAnObject) => ov.set_field_by_name(имя, sv)?,
                        other => other?,
                    }
                }
                frames[frame_idx].pc += 1;
            }
            Instr::CallBuiltin {
                dst,
                builtin,
                base,
                count,
            } => {
                let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                let v = if builtin == bsl_rt::BuiltinFn::ErrorInfo {
                    current_error_info(current_exception.as_ref())?
                } else {
                    call_builtin_with_format(builtin, args.as_slice(), runtime_shapes, host)?
                };
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
                let v = if let Some(object) = ov.object_ref() {
                    let mut context =
                        bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                            runtime_shapes,
                            stdout: &mut *host.stdout,
                            stderr: &mut *host.stderr,
                            formatter: bsl_format::format_value,
                            zone: &linked.zone,
                            files: &linked.files,
                            random: &linked.random,
                            network: linked.network.as_ref(),
                            background_jobs: linked.background_jobs.as_ref(),
                            temp_storage: linked.temp_storage.as_ref(),
                            message_sink: linked.message_sink.as_ref(),
                            host_promises: Some(async_state),
                            function_caller: None,
                        });
                    object.call_method(method.primary_name(), args.as_slice(), &mut context)?
                } else {
                    bsl_rt::call_builtin_method_files(
                        method,
                        &ov,
                        args.as_slice(),
                        runtime_shapes,
                        linked.files.as_ref(),
                    )?
                };
                let d = frames[frame_idx].reg_index(dst);
                reg_store(stack, d, v)?;
                frames[frame_idx].pc += 1;
            }
            // Холодные опкоды: конструирование объектов, возбуждение
            // исключения, закрытие файла и динамическое исполнение. Тела
            // вынесены в `step_cold`, чтобы код диспетчера не разъезжался
            // по памяти вокруг горячего пути: он живёт на грани кеша
            // микроопераций процессора, и лишние окна на пути стоят до
            // полутора раз при том же числе исполненных инструкций.
            //
            // Граница проведена по стоимости тела, а не по редкости
            // опкода: `CallBuiltin` и `CallMethod`
            // остались здесь, потому что исполняются миллионами и лишний
            // вызов на каждый стоит дороже, чем занятое ими место (измерено
            // — `csv_write` теряет 7%, если унести и их). Открытый
            // `CallObjectMethod` встречается только у компонентных методов.
            //
            // Опкоды перечислены поимённо, а не `_`, чтобы `match`
            // остался исчерпывающим и новый опкод по-прежнему ломал
            // сборку, пока его не расклассифицируют.
            Instr::NewArray { .. }
            | Instr::NewStructure { .. }
            | Instr::NewTable { .. }
            | Instr::NewTypeDescription { .. }
            | Instr::NewValueComparison { .. }
            | Instr::NewMap { .. }
            | Instr::NewTextWriter { .. }
            | Instr::Raise { .. }
            | Instr::CallObjectMethod { .. }
            | Instr::GetObjectProp { .. }
            | Instr::SetObjectProp { .. }
            | Instr::CallComponent { .. }
            | Instr::CreateObject { .. }
            | Instr::CallImported { .. }
            | Instr::GetImportedVar { .. }
            | Instr::SetImportedVar { .. }
            | Instr::RunDynamic { .. } => {
                step_cold(
                    instr,
                    frames,
                    stack,
                    program,
                    caches,
                    current_exception,
                    linked,
                    host,
                    runtime_shapes,
                    module_state,
                    modules,
                    async_state,
                    task_id,
                    frame_idx,
                    func_id,
                    chunk,
                )?;
            }
            Instr::CollectionLen { dst, obj } => {
                let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
                let len = ov.collection_len()?;
                let d = frames[frame_idx].reg_index(dst);
                reg_store(
                    stack,
                    d,
                    BslValue::Number(bsl_number::BslNumber::from_i64(len as i64)),
                )?;
                frames[frame_idx].pc += 1;
            }
        }
        if extra == 0 {
            // Бандл кончился. Линейная цепочка бандлов продолжается прямо
            // здесь: смену кадра (`Call`) ловит первая
            // проверка, взятый переход — вторая, а `Return` вышел из
            // функции ранним `return` ещё в своей ветке. Всё, что
            // осталось, — обычный fallthrough, на котором пробам
            // `drive_with` делать нечего.
            if !merge_linear || frames.len() != frame_idx + 1 {
                break;
            }
            let next = frames[frame_idx].pc;
            if next != pc + 1 {
                break;
            }
            pc = next;
            if pc >= chunk.instrs.len() {
                break;
            }
            extra = chunk
                .bundle_len()
                .get(pc)
                .copied()
                .unwrap_or(1)
                .saturating_sub(1);
            continue;
        }
        extra -= 1;
        // Предыдущий член сам продвинул `pc` кадра.
        pc = frames[frame_idx].pc;
        if pc >= chunk.instrs.len() {
            break;
        }
    }
    Ok(Step::Continue)
}

/// Холодная половина диспетчера: опкоды, тело которых делает настоящую
/// работу — выделяет объект, зовёт встроенную функцию или метод, пишет
/// текст, исполняет динамический фрагмент. Вынесены из [`step`] отдельной
/// функцией с `#[inline(never)]`, потому что горячий цикл
/// диспетчеризации живёт на грани кеша микроопераций: измерено, что рост
/// `step` на три байта роняет пустой цикл BSL в полтора раза при том же
/// числе исполненных инструкций. Лишний вызов на такой опкод теряется на
/// фоне его собственной работы.
///
/// # Errors
///
/// Возвращает ошибку исполнения опкода, `RtError::Raised` от
/// `ВызватьИсключение`, а на горячем опкоде, который сюда попасть не
/// может, — `RtError::InvalidBytecode`.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn step_cold(
    instr: Instr,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
    caches: &RunCaches,
    current_exception: &Option<BslValue>,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    module_state: &mut ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    async_state: &mut AsyncState,
    task_id: TaskId,
    frame_idx: usize,
    func_id: usize,
    chunk: &bsl_bytecode::Chunk,
) -> Result<(), RtError> {
    match instr {
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
            let shape_rc = at(
                &program.shapes,
                shape as usize,
                "номер формы вне таблицы форм программы",
            )?
            .clone();
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
        Instr::NewTypeDescription { dst, names } => {
            let names = reg_load(stack, frames[frame_idx].reg_index(names))?;
            let value = BslValue::new_type_description(&names, runtime_shapes)?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewValueComparison { dst } => {
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, BslValue::new_value_comparison())?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewMap { dst } => {
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, BslValue::new_map())?;
            frames[frame_idx].pc += 1;
        }
        Instr::NewTextWriter { dst, path } => {
            let path = reg_load(stack, frames[frame_idx].reg_index(path))?;
            let files = host.env()?.files();
            let writer = BslValue::new_text_writer_with_files(&path, files.as_ref())?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, writer)?;
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
        Instr::GetObjectProp { dst, obj, name } => {
            let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
            let name_id = bsl_rt::NameId::from_index(name as u32);
            let value = if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                    runtime_shapes,
                    stdout: &mut *host.stdout,
                    stderr: &mut *host.stderr,
                    formatter: bsl_format::format_value,
                    zone: &linked.zone,
                    files: &linked.files,
                    random: &linked.random,
                    network: linked.network.as_ref(),
                    background_jobs: linked.background_jobs.as_ref(),
                    temp_storage: linked.temp_storage.as_ref(),
                    message_sink: linked.message_sink.as_ref(),
                    host_promises: None,
                    function_caller: None,
                });
                component_prop_get(
                    object,
                    &linked.component_properties,
                    name_id,
                    program,
                    &mut context,
                )?
            } else {
                match ov
                    .get_field_cached(name_id, prop_cache(caches, func_id, frames[frame_idx].pc)?)
                {
                    Err(RtError::NotAnObject) => {
                        ov.get_field_by_name(field_name(program, name_id)?)?
                    }
                    other => other?,
                }
            };
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::SetObjectProp { obj, name, src } => {
            let ov = reg_load(stack, frames[frame_idx].reg_index(obj))?;
            let value = reg_load(stack, frames[frame_idx].reg_index(src))?;
            let name_id = bsl_rt::NameId::from_index(name as u32);
            if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                    runtime_shapes,
                    stdout: &mut *host.stdout,
                    stderr: &mut *host.stderr,
                    formatter: bsl_format::format_value,
                    zone: &linked.zone,
                    files: &linked.files,
                    random: &linked.random,
                    network: linked.network.as_ref(),
                    background_jobs: linked.background_jobs.as_ref(),
                    temp_storage: linked.temp_storage.as_ref(),
                    message_sink: linked.message_sink.as_ref(),
                    host_promises: None,
                    function_caller: None,
                });
                component_prop_set(
                    object,
                    &linked.component_properties,
                    name_id,
                    value,
                    program,
                    &mut context,
                )?;
            } else {
                match ov.set_field_cached(
                    name_id,
                    value.clone(),
                    prop_cache(caches, func_id, frames[frame_idx].pc)?,
                ) {
                    Err(RtError::NotAnObject) => {
                        ov.set_field_by_name(field_name(program, name_id)?, value)?
                    }
                    other => other?,
                }
            }
            frames[frame_idx].pc += 1;
        }
        // Компонентные вызовы уехали сюда из горячего цикла: их тела —
        // самое крупное, что в нём лежало (машинерия `function_caller`
        // для обратных вызовов из компонентов), а стоимость холодного
        // перехода тонет в работе компонента. Горячему циклу важен
        // размер: он живёт на грани кеша микроопераций (см. комментарий
        // у списка холодных опкодов в `step`); вынос измерен A/B —
        // call_overhead с +12,3% до +4,3% к main, pi_leibniz с +12,1%
        // до +6,2%.
        Instr::CallComponent {
            dst, base, count, ..
        } => {
            let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
            let call = linked.function(func_id, frames[frame_idx].pc)?;
            // Окружение прогона едет и в обратный вызов: функция модуля,
            // позванная компонентом, обязана видеть те же часы и те же
            // аргументы, что и остальной код этого `State`.
            let HostIo {
                stdout: host_stdout,
                stderr: host_stderr,
                env: host_env,
                dynamic: host_dynamic,
                dynamic_depth: host_dynamic_depth,
            } = host;
            let mut function_caller =
                |name: &str,
                 call_args: Vec<BslValue>,
                 stdout: &mut dyn Write,
                 stderr: &mut dyn Write| {
                    let mut nested_host = HostIo {
                        stdout,
                        stderr,
                        env: host_env.as_deref_mut(),
                        // Функция модуля, позванная компонентом, вправе
                        // содержать `Выполнить`: компилятор фрагментов едет
                        // в обратный вызов вместе с потоками и окружением.
                        dynamic: host_dynamic.as_deref_mut(),
                        // Тот же счётчик вложенности, что у прогона: обратный
                        // вызов продолжает ту же сессию, а не открывает свою.
                        dynamic_depth: host_dynamic_depth,
                    };
                    call_module_function_in_execution(
                        program,
                        name,
                        call_args,
                        linked,
                        &mut nested_host,
                        module_state,
                    )
                };
            let mut context = bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                runtime_shapes,
                stdout: &mut **host_stdout,
                stderr: &mut **host_stderr,
                formatter: bsl_format::format_value,
                zone: &linked.zone,
                files: &linked.files,
                random: &linked.random,
                network: linked.network.as_ref(),
                background_jobs: linked.background_jobs.as_ref(),
                temp_storage: linked.temp_storage.as_ref(),
                message_sink: linked.message_sink.as_ref(),
                host_promises: None,
                function_caller: Some(&mut function_caller),
            });
            let value = call(&mut context, args.as_slice())?;
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::CreateObject {
            dst, base, count, ..
        } => {
            let args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
            let call = linked.constructor(func_id, frames[frame_idx].pc)?;
            let mut context = bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                runtime_shapes,
                stdout: &mut *host.stdout,
                stderr: &mut *host.stderr,
                formatter: bsl_format::format_value,
                zone: &linked.zone,
                files: &linked.files,
                random: &linked.random,
                network: linked.network.as_ref(),
                background_jobs: linked.background_jobs.as_ref(),
                temp_storage: linked.temp_storage.as_ref(),
                message_sink: linked.message_sink.as_ref(),
                host_promises: None,
                function_caller: None,
            });
            let value = call(&mut context, args.as_slice())?;
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::CallObjectMethod {
            result_required,
            dst,
            obj,
            method,
            base,
            count,
        } => {
            // Задача уже припаркована этим самым вызовом: `pc` остался на
            // инструкции, и внутри бандла `step` диспатчит её повторно,
            // пока не дойдёт до границы. Повторный вход — пустой: сам
            // `step` за парковку не платит ни байта (его тело на грани
            // кеша микроопераций), а паркует задачу арм `Continue`
            // планировщика.
            if async_state.sync_wait.is_some() {
                return Ok(());
            }
            let ov = at(
                stack,
                frames[frame_idx].reg_index(obj),
                "чтение объекта за границей стека значений",
            )?;
            // Аргументы открытого вызова кодоген кладёт в свежие временные
            // регистры — в стеке значений они лежат подряд, и обработчик
            // получает их срезом стека без поштучного клонирования (это
            // измеримая часть цены вызова: Rc-инкременты и сбросы на
            // каждый аргумент). Регистры-алиасы параметров смежности не
            // гарантируют — такая база уходит запасным путём с копиями.
            // Приёмник тоже заимствуется: обработчики не достают до стека VM
            // (их `CallContext` — без
            // канала обратного вызова), поэтому заём безопасен, а
            // `reg_store` идёт уже после того, как значение вычислено.
            let contiguous_args = base as usize >= frames[frame_idx].param_aliases.len();
            let fallback_args;
            let args: &[BslValue] = if count == 0 {
                &[]
            } else if contiguous_args {
                let start = frames[frame_idx].reg_index(base);
                stack
                    .get(start..start + count as usize)
                    .ok_or(RtError::InvalidBytecode(
                        "чтение аргументов за границей стека значений",
                    ))?
            } else {
                fallback_args = CallArgs::load(stack, &frames[frame_idx], base, count)?;
                fallback_args.as_slice()
            };
            let name_id = bsl_rt::NameId::from_index(method as u32);
            let value = if let Some(object) = ov.object_ref() {
                let mut context = bsl_rt::CallContext::interpreter(bsl_rt::InterpreterServices {
                    runtime_shapes,
                    stdout: &mut *host.stdout,
                    stderr: &mut *host.stderr,
                    formatter: bsl_format::format_value,
                    zone: &linked.zone,
                    files: &linked.files,
                    random: &linked.random,
                    network: linked.network.as_ref(),
                    background_jobs: linked.background_jobs.as_ref(),
                    temp_storage: linked.temp_storage.as_ref(),
                    message_sink: linked.message_sink.as_ref(),
                    host_promises: Some(async_state),
                    function_caller: None,
                });
                // Тип со статической таблицей методов идёт кэшем ячейки
                // этой инструкции поверх мемоизированного моста «номер
                // имени → обработчик»; промах и тип без таблицы —
                // строковым `call_method`, там единственный источник
                // текста ошибки о неизвестном методе.
                match cached_component_method(
                    caches,
                    func_id,
                    frames[frame_idx].pc,
                    &linked.component_methods,
                    object.method_table(),
                    name_id,
                    program,
                )? {
                    Some(descriptor) => {
                        descriptor.check_result_use(result_required)?;
                        descriptor.check_arity(count, object.type_descriptor().name)?;
                        match descriptor.invoke(object.as_dyn(), args, &mut context)? {
                            bsl_rt::CallOutcome::Ready(value) => value,
                            // Приостанавливающий метод: host-операция
                            // регистрируется, задача паркуется с `pc` на
                            // этой инструкции — ранний возврат без сдвига
                            // `pc`. `step` увидит установленный
                            // `sync_wait` и вернёт `Step::Suspend`;
                            // возобновляет планировщик
                            // (`resume_sync_host_call`), повторного входа
                            // в обработчик нет.
                            bsl_rt::CallOutcome::Pending(pending) => {
                                return async_state.begin_sync_host_call(task_id, dst, pending);
                            }
                        }
                    }
                    None => {
                        let method_name = field_name(program, name_id)?;
                        object.call_method(method_name, args, &mut context)?
                    }
                }
            } else {
                // Нативный получатель: обработчик по номеру имени из таблицы
                // связывания, строка нужна только тексту ошибки.
                let builtin =
                    linked
                        .builtin_method(name_id)
                        .ok_or_else(|| RtError::UnknownMethod {
                            method: field_name(program, name_id).unwrap_or("?").to_string(),
                            receiver: ov.type_name(),
                        })?;
                bsl_rt::call_builtin_method_files(
                    builtin,
                    ov,
                    args,
                    runtime_shapes,
                    linked.files.as_ref(),
                )?
            };
            let destination = frames[frame_idx].reg_index(dst);
            reg_store(stack, destination, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::RunDynamic { src, dst, is_eval } => {
            let code = reg_load(stack, frames[frame_idx].reg_index(src))?;
            let code = match code {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: if is_eval {
                            "Вычислить"
                        } else {
                            "Выполнить"
                        },
                    });
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
                linked,
                host,
                module_state,
            )?;
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, value)?;
            frames[frame_idx].pc += 1;
        }
        // Вызов экспортного метода чужого модуля. Протокол ленивой
        // инициализации: если какой-то из затрагиваемых модулей ещё не
        // инициализирован, `ensure_module_ready` пушит кадр его тела и
        // возвращает управление БЕЗ продвижения `pc` — после возврата тела
        // эта же инструкция исполняется повторно, уже с готовым модулем.
        Instr::CallImported {
            link_slot,
            base,
            arg_modes,
            ret,
        } => {
            let ctx = modules.catalog.ok_or(RtError::InvalidBytecode(
                "межмодульный опкод вне каталога конфигурации",
            ))?;
            let Some(&bsl_bytecode::LinkEntry::Function {
                module: target,
                func,
            }) = program.links.get(link_slot as usize)
            else {
                return Err(RtError::InvalidBytecode(
                    "CallImported ведёт мимо таблицы связей или на переменную",
                ));
            };
            let target = target.index() as u32;
            if ensure_module_ready(target, ctx, modules.session, frames, stack)? {
                return Ok(());
            }
            let modes = at(
                &chunk.call_arg_modes,
                arg_modes as usize,
                "номер набора режимов аргументов вне таблицы чанка",
            )?;
            // Все модули, чьи переменные уходят по ссылке, тоже должны быть
            // готовы до первого побочного действия: построение кадра ниже
            // уже пушит значения в стек и продвигает `pc`.
            for mode in modes {
                if let ArgMode::ByRefImportedVar(slot) = mode {
                    let Some(&bsl_bytecode::LinkEntry::Variable { module, .. }) =
                        program.links.get(*slot as usize)
                    else {
                        return Err(RtError::InvalidBytecode(
                            "byimport ведёт мимо таблицы связей или на функцию",
                        ));
                    };
                    if ensure_module_ready(
                        module.index() as u32,
                        ctx,
                        modules.session,
                        frames,
                        stack,
                    )? {
                        return Ok(());
                    }
                }
            }
            let callee_program = ctx.program(target)?;
            let callee_chunk = at(
                &callee_program.chunks,
                func as usize,
                "связь ведёт на несуществующий чанк модуля",
            )?;
            // Асинхронная цель межмодульного вызова не поддержана до замера
            // `JOB.ASYNC.TARGET`: семантика завершения не выведена логикой.
            if callee_chunk.is_async {
                return Err(RtError::DynamicError(
                    "асинхронная цель межмодульного вызова ещё не поддержана".into(),
                ));
            }
            if frames.len() >= MAX_CALL_DEPTH {
                return Err(RtError::StackOverflow {
                    what: "слишком глубокая рекурсия вызовов",
                });
            }
            frames[frame_idx].pc += 1;
            let mut param_aliases = Vec::with_capacity(modes.len());
            let mut module_copybacks = Vec::new();
            for (i, mode) in modes.iter().enumerate() {
                let slot = match mode {
                    ArgMode::Value => ParamSlot {
                        idx: frames[frame_idx].reg_index(base + i as u8),
                        provided: true,
                    },
                    ArgMode::ByRefLocal(slot) => ParamSlot {
                        idx: frames[frame_idx].reg_index(*slot),
                        provided: true,
                    },
                    // Модульная переменная ВЫЗЫВАЮЩЕГО модуля: значение
                    // копируется во временный слот, а при возврате кадра
                    // уходит обратно в состояние модуля-владельца.
                    ArgMode::ByRefModuleVar(slot) => {
                        let module_slot = *slot as usize;
                        let value = reg_load(&module_state.slots, module_slot)?;
                        let idx = stack.len();
                        stack.push(value);
                        module_copybacks.push((idx, frames[frame_idx].module, module_slot));
                        ParamSlot {
                            idx,
                            provided: true,
                        }
                    }
                    // Экспортная переменная ТРЕТЬЕГО модуля: то же, но
                    // состояние берётся из сессии (модуль готов — ensure
                    // выше; собственный модуль в связях запрещён периметром
                    // образа, так что изъятая ячейка не встретится).
                    ArgMode::ByRefImportedVar(slot) => {
                        let Some(&bsl_bytecode::LinkEntry::Variable {
                            module,
                            slot: var_slot,
                        }) = program.links.get(*slot as usize)
                        else {
                            return Err(RtError::InvalidBytecode(
                                "byimport ведёт мимо таблицы связей или на функцию",
                            ));
                        };
                        let owner = module.index() as u32;
                        let value = {
                            let instance = modules.session.instances.get(owner as usize).ok_or(
                                RtError::InvalidBytecode("связь ведёт мимо сессии модулей"),
                            )?;
                            reg_load(&instance.state.slots, var_slot as usize)?
                        };
                        let idx = stack.len();
                        stack.push(value);
                        module_copybacks.push((idx, owner, var_slot as usize));
                        ParamSlot {
                            idx,
                            provided: true,
                        }
                    }
                    ArgMode::Default => {
                        let idx = frames[frame_idx].reg_index(base + i as u8);
                        reg_store(stack, idx, BslValue::Undefined)?;
                        ParamSlot {
                            idx,
                            provided: false,
                        }
                    }
                };
                param_aliases.push(slot);
            }
            let call_start = stack.len();
            let own_base = stack.len();
            push_own_registers(stack, callee_chunk);
            frames.push(Frame {
                module: target,
                func_id: func as usize,
                pc: 0,
                param_aliases,
                own_base,
                call_start,
                return_reg: ret,
                module_copybacks,
                numeric_for_state: None,
            });
        }
        // Чтение экспортной переменной чужого модуля — с той же ленивой
        // инициализацией владельца.
        Instr::GetImportedVar { dst, link_slot } => {
            let ctx = modules.catalog.ok_or(RtError::InvalidBytecode(
                "межмодульный опкод вне каталога конфигурации",
            ))?;
            let Some(&bsl_bytecode::LinkEntry::Variable { module, slot }) =
                program.links.get(link_slot as usize)
            else {
                return Err(RtError::InvalidBytecode(
                    "импортная переменная ведёт мимо таблицы связей или на функцию",
                ));
            };
            let owner = module.index() as u32;
            if ensure_module_ready(owner, ctx, modules.session, frames, stack)? {
                return Ok(());
            }
            let value = {
                let instance = modules
                    .session
                    .instances
                    .get(owner as usize)
                    .ok_or(RtError::InvalidBytecode("связь ведёт мимо сессии модулей"))?;
                reg_load(&instance.state.slots, slot as usize)?
            };
            let d = frames[frame_idx].reg_index(dst);
            reg_store(stack, d, value)?;
            frames[frame_idx].pc += 1;
        }
        Instr::SetImportedVar { link_slot, src } => {
            let ctx = modules.catalog.ok_or(RtError::InvalidBytecode(
                "межмодульный опкод вне каталога конфигурации",
            ))?;
            let Some(&bsl_bytecode::LinkEntry::Variable { module, slot }) =
                program.links.get(link_slot as usize)
            else {
                return Err(RtError::InvalidBytecode(
                    "импортная переменная ведёт мимо таблицы связей или на функцию",
                ));
            };
            let owner = module.index() as u32;
            if ensure_module_ready(owner, ctx, modules.session, frames, stack)? {
                return Ok(());
            }
            let value = reg_load(stack, frames[frame_idx].reg_index(src))?;
            let instance = modules
                .session
                .instances
                .get_mut(owner as usize)
                .ok_or(RtError::InvalidBytecode("связь ведёт мимо сессии модулей"))?;
            reg_store(&mut instance.state.slots, slot as usize, value)?;
            frames[frame_idx].pc += 1;
        }
        _ => {
            return Err(RtError::InvalidBytecode(
                "горячий опкод попал в холодную половину диспетчера",
            ));
        }
    }
    Ok(())
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

fn do_return_with_value(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    module_state: &mut ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    value: BslValue,
) -> Result<ReturnOutcome, RtError> {
    let frame = frames
        .pop()
        .expect("инвариант VM: возврат исполняется только при непустом стеке кадров");
    for (stack_slot, target_module, module_slot) in &frame.module_copybacks {
        let value = reg_load(stack, *stack_slot)?;
        // `module_state` — состояние модуля ВОЗВРАЩАЮЩЕГОСЯ кадра (у
        // модульного кадра оно на время шага изъято из сессии драйвером);
        // корневое состояние приходит отдельной ссылкой, остальные модули —
        // через сессию.
        let slots = if *target_module == frame.module {
            &mut module_state.slots
        } else if *target_module == ROOT_MODULE {
            &mut modules
                .root_state
                .as_deref_mut()
                .ok_or(RtError::InvalidBytecode(
                    "копибэк в корневое состояние без ссылки на него",
                ))?
                .slots
        } else {
            &mut modules
                .session
                .instances
                .get_mut(*target_module as usize)
                .ok_or(RtError::InvalidBytecode(
                    "копибэк ссылается на модуль вне сессии",
                ))?
                .state
                .slots
        };
        reg_store(slots, *module_slot, value)?;
    }
    // Возврат из кадра инициализации модуля: тело модуля отработало,
    // экземпляр готов; результата у тела нет, и писать его некуда.
    if frame.module != ROOT_MODULE && frame.func_id == 0 {
        if let Some(instance) = modules.session.instances.get_mut(frame.module as usize) {
            instance.init = ModuleInitState::Ready;
        }
        stack.truncate(frame.call_start);
        return Ok(if frames.is_empty() {
            Done(value)
        } else {
            Continuing
        });
    }
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
    catalog: Option<&CatalogContext<'_>>,
    session: &mut SessionModules,
    err: &RtError,
    current_exception: &mut Option<BslValue>,
) -> bool {
    // Повреждённый образ не ловится `Попытка`: иначе битый байт-код ушёл бы
    // наружу с признаком успеха. `Link`, `StackOverflow`, `DynamicError` и
    // ошибки форматов приходят из пользовательских данных и остаются
    // ловимыми (см. `RtError::is_bsl_exception`).
    if !err.is_bsl_exception() {
        return false;
    }
    let mut first = true;
    loop {
        let frame_idx = frames.len() - 1;
        // Чанк кадра лежит в программе ЕГО модуля: кадры конфигурации
        // разматываются через каталог.
        let frame_program = if frames[frame_idx].module == ROOT_MODULE {
            program
        } else {
            match catalog.map(|ctx| ctx.program(frames[frame_idx].module)) {
                Some(Ok(p)) => p,
                _ => return false,
            }
        };
        let chunk = match frame_program.chunks.get(frames[frame_idx].func_id) {
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
        // Ошибка вылетела из тела модуля: инициализация не удалась, и
        // повторное касание модуля отвечает ловимой ошибкой, а не повторным
        // запуском тела — до замера `JOB.MODULE.INIT`.
        if frame.module != ROOT_MODULE
            && frame.func_id == 0
            && let Some(instance) = session.instances.get_mut(frame.module as usize)
        {
            instance.init = ModuleInitState::Failed;
        }
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

/// Снимает текущую ошибку задачи до выхода из обработчика. Платформенное
/// подробное представление содержит координаты модулей и стек вызовов;
/// open-bsl пока не хранит эквивалентную модель диагностики, поэтому это
/// СОЗНАТЕЛЬНОЕ ОТКЛОНЕНИЕ: снимок содержит только безопасный текст ошибки.
/// Этого достаточно для повторного броска и журналирования Connector.
fn current_error_info(current_exception: Option<&BslValue>) -> Result<BslValue, RtError> {
    let detail = match current_exception {
        Some(value) => bsl_format::format_value(value, None)?,
        // ИЗМЕРЕНО на 1С 8.3.27: вне обработчика функция возвращает объект,
        // а не `Неопределено`, и его подробное представление равно этой
        // строке (oracle `measure-error-info.bsl`).
        None => "Unexpected error".to_string(),
    };
    Ok(bsl_rt::new_error_info(bsl_rt::BslString::from_str(&detail)))
}

/// `Строка`/`Формат`/`Число`/`Message` перехватываются здесь, а не в
/// `bsl_rt::call_builtin_fn`: форматирование живёт в `bsl-format`, которое
/// зависит от `bsl-rt` (не наоборот) — `bsl-rt` физически не может
/// отформатировать число сам. Всё остальное уходит в `bsl-rt` как обычно.
///
/// `ПрочитатьJSON`/`ЗаписатьJSON` перехватываются здесь по той же причине,
/// но с другого конца: их функции восстановления и преобразования — это
/// вызов пользовательской функции ПО ИМЕНИ, то есть [`call_module_function`],
/// которой в `bsl-rt` быть не может (зависимость идёт в обратную сторону).
/// Поэтому `program` и `stack` доходят сюда: из них строится замыкание,
/// которое рантайм зовёт как обычную функцию.
///
/// Арность проверена в `bsl-sema` — но проверена для того байт-кода,
/// который родился из резолвинга. Здесь она перепроверяется один раз на
/// вызов, чтобы ни эта функция, ни `bsl_rt::call_builtin_fn` (тоже
/// индексирующая `args` напрямую) не паниковали на чанке, собранном мимо
/// резолвера.
fn call_builtin_with_format(
    builtin: bsl_rt::BuiltinFn,
    args: &[BslValue],
    runtime_shapes: &mut bsl_rt::RuntimeShapes,
    host: &mut HostIo<'_, '_>,
) -> Result<BslValue, RtError> {
    use bsl_rt::BuiltinFn;
    // Обычно проверка идёт по МАКСИМУМУ: резолвер добивает необязательные
    // позиции `Неопределено`. Вариадические `Мин`/`Макс`, напротив,
    // сохраняют фактическое число аргументов, поэтому им достаточно
    // измеренного минимума.
    let required = if builtin.is_variadic() {
        builtin.arity_range().0
    } else {
        builtin.arity_range().1
    };
    if args.len() < required {
        return Err(RtError::InvalidBytecode(
            "встроенной функции передано меньше аргументов, чем требует её арность",
        ));
    }
    match builtin {
        BuiltinFn::Message => {
            let text = bsl_format::format_value(&args[0], None)?;
            // Сеанс с внедрённым sink отдаёт владеющий DTO: историю
            // сообщений задания пишет runtime, представление выбирает
            // host. Без sink — прежний путь, строка в stdout сеанса.
            match host.env().ok().and_then(|env| env.message_sink()) {
                Some(sink) => sink
                    .enqueue(&bsl_rt::UserMessageDto::from_text(text))
                    .map_err(bsl_rt::HostError::raise)?,
                None => writeln!(host.stdout, "{text}")
                    .map_err(|error| RtError::IoError(error.to_string()))?,
            }
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
                    });
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
                    });
                }
            };
            let n = bsl_format::parse_number(&s.to_string(), &bsl_format::NumberFormat::default())?;
            Ok(BslValue::Number(n))
        }
        // Не `call_builtin_fn`: `ЗаполнитьЗначенияСвойств` читает таблицу
        // имён, и путь без контекста для неё кончается ошибкой.
        // Куда именно функция ходит наружу, говорит один источник истины —
        // `BuiltinFn::host_effect`.
        other => match other.host_effect() {
            // Часы, часы в миллисекундах и аргументы запуска отвечают из
            // окружения прогона, а не из состояния процесса.
            Some(bsl_rt::HostEffect::Env) => bsl_rt::call_builtin_env(other, host.env()?),
            // `ЗначениеВФайл`/`ЗначениеИзФайла` читают и пишут файл
            // целиком, а файловая система принадлежит прогону — как часы
            // и зона.
            Some(bsl_rt::HostEffect::Files) => {
                let files = host.env()?.files();
                bsl_rt::call_builtin_files(other, args, runtime_shapes, files.as_ref())
            }
            Some(bsl_rt::HostEffect::TempFiles) => {
                let env = host.env()?;
                let mut entropy = [0u8; 16];
                env.fill_random(&mut entropy);
                let files = env.files();
                bsl_rt::call_builtin_temp_file(other, args, files.as_ref(), &entropy)
            }
            // `Сообщить` перехвачен веткой выше — до сюда доходит только
            // то, что считает ответ по одним аргументам.
            Some(bsl_rt::HostEffect::Output) | None => {
                bsl_rt::call_builtin_fn_ctx(other, args, runtime_shapes)
            }
        },
    }
}

/// Тело инструкции `Add`.
///
/// Отдельной функцией, чтобы арифметика и конкатенация не раздували ветку
/// `step` диспетчерского цикла.
fn add_op(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    b: u8,
) -> Result<(), RtError> {
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    add_rhs_op(frames, stack, frame_idx, dst, a, &bv)
}

/// Тело `AddConst` вынесено из `step`: разворачивание двух проверок таблиц
/// в цикл диспетчеризации сдвигает код остальных опкодов, хотя они этой
/// инструкцией не пользуются.
#[inline(never)]
fn add_const_op(
    program: &Program,
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    src: u8,
    k: u16,
) -> Result<(), RtError> {
    let chunk = at(
        &program.chunks,
        frames[frame_idx].func_id,
        "номер чанка вне таблицы функций",
    )?;
    let value = at(
        &chunk.consts,
        k as usize,
        "номер константы вне таблицы констант чанка",
    )?;
    add_rhs_op(frames, stack, frame_idx, dst, src, value)
}

/// Общее тело `Add` и `AddConst`: правый операнд уже найден, но порядок и
/// все преобразования остаются прежними.
fn add_rhs_op(
    frames: &mut [Frame],
    stack: &mut [BslValue],
    frame_idx: usize,
    dst: u8,
    a: u8,
    bv: &BslValue,
) -> Result<(), RtError> {
    // Накопление строки в саму себя (`Текст = Текст + Кусок`
    // — приёмник и левый операнд один регистр) идёт особым
    // путём: значение ЗАБИРАЕТСЯ из регистра, а не копируется.
    // Регистр всё равно будет перезаписан результатом, зато
    // счётчик ссылок падает до единицы, и буфер дописывается
    // на месте вместо копирования всего накопленного.
    //
    // Условие на ОБЕ строки проверяется ДО того, как регистр
    // опустошён: иначе ошибка типа оставила бы переменную
    // затёртой, а её мог бы поймать `Попытка` и поехать
    // дальше с потерянным значением.
    let d = frames[frame_idx].reg_index(dst);
    let ia = frames[frame_idx].reg_index(a);
    let both_strings = matches!(
        (stack.get(ia), bv),
        (Some(BslValue::Str(_)), BslValue::Str(_))
    );
    if d == ia && both_strings {
        let av = std::mem::replace(&mut stack[ia], BslValue::Undefined);
        let (BslValue::Str(left), BslValue::Str(right)) = (av, bv) else {
            unreachable!("типы проверены выше")
        };
        stack[d] = BslValue::Str(left.append(right));
    } else if matches!(stack.get(ia), Some(BslValue::Str(_))) {
        // Строка СЛЕВА решает исход: правый операнд приводится к строке
        // и приклеивается, каким бы он ни был. Измерено на 8.3.27 по
        // всем типам сразу — и приведение оказалось ровно `Строка()`,
        // вместе с разделителями групп («Сумма: » + 1000.5 даёт
        // «Сумма: 1 000,5» с НЕРАЗРЫВНЫМ пробелом внутри). Поэтому
        // здесь именно `format_value`, а не своё представление числа.
        let av = reg_load(stack, ia)?;
        let BslValue::Str(left) = av else {
            unreachable!("тип проверен выше")
        };
        let right = bsl_format::format_value(bv, None)?;
        let joined = left.append(&bsl_rt::BslString::from_str(&right));
        reg_store(stack, d, BslValue::Str(joined))?;
    } else {
        // Тот же порядок, что и в `binop`: сначала как есть, приведение
        // — только после отказа. Строка слева сюда уже не попадает (её
        // разобрала ветка выше), поэтому подменить склейку арифметикой
        // этот повтор не может.
        let av = reg_load(stack, ia)?;
        let sum = match av.add(bv) {
            Ok(v) => v,
            Err(first) => {
                if needs_arith_coercion(&av) || needs_arith_coercion(bv) {
                    arith(&av)?.add(arith(bv)?.as_ref())?
                } else {
                    return Err(first);
                }
            }
        };
        reg_store(stack, d, sum)?;
    }
    Ok(())
}

/// Операнд арифметики после приведения.
///
/// Платформа тянет к числу и строку, и булево: `5 + "3"` даёт 8,
/// `Истина + 1` — 2, `-Истина` — минус единицу (всё измерено). Разбор
/// строки — тот же, что у `Число()`: с обрезкой пробелов, точкой ИЛИ
/// запятой и разделителями групп, так что `("" + 1000.5) - 0.5` честно
/// возвращает 1000.
///
/// `Cow` здесь не украшение: приведение нужно РЕДКО, а сложение чисел
/// лежит на самом горячем пути, и лишней копии значения на нём быть не
/// должно.
#[inline]
fn arith(v: &BslValue) -> Result<std::borrow::Cow<'_, BslValue>, RtError> {
    match v {
        BslValue::Str(s) => {
            let n = bsl_format::parse_number(&s.to_string(), &bsl_format::NumberFormat::default())
                .map_err(RtError::Num)?;
            Ok(std::borrow::Cow::Owned(BslValue::Number(n)))
        }
        // Истина — единица, Ложь — ноль.
        BslValue::Boolean(b) => Ok(std::borrow::Cow::Owned(BslValue::Number(
            bsl_number::BslNumber::from_i64(i64::from(*b)),
        ))),
        _ => Ok(std::borrow::Cow::Borrowed(v)),
    }
}

/// Тело инструкции `Neg`. Отдельной функцией по той же причине, что и
/// [`add_op`]: чтобы не раздувать ветку диспетчерского цикла.
fn neg_op(v: &BslValue) -> Result<BslValue, RtError> {
    // Тот же порядок, что в `binop`: сначала как есть. Унарный минус лежит
    // на горячем пути не меньше сложения (`flip = -flip` в цикле), и
    // строить `Cow` ради заведомого числа он не должен.
    match v.neg() {
        Ok(r) => Ok(r),
        Err(first) => {
            if needs_arith_coercion(v) {
                arith(v)?.neg()
            } else {
                Err(first)
            }
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
    let av = reg_load(stack, frames[frame_idx].reg_index(a))?;
    let bv = reg_load(stack, frames[frame_idx].reg_index(b))?;
    // Приведение строк и булевых к числу — свойство ВСЕЙ арифметики, а не
    // одного сложения: измерены `"5" - 1`, `"5" * 2`, `"5" / 2`,
    // `Истина - 1` и `Истина * 2`.
    //
    // Порядок здесь ради ЦЕНЫ: сначала операция пробуется как есть, и
    // только её отказ включает приведение. Пара чисел — подавляющее
    // большинство вызовов — не платит за приведение вообще ничего, даже
    // проверки тега. Повтор безопасен, потому что операции над значениями
    // чистые: неудачная попытка ничего не меняет.
    let result = match f(&av, &bv) {
        Ok(v) => v,
        Err(first) => {
            if needs_arith_coercion(&av) || needs_arith_coercion(&bv) {
                f(arith(&av)?.as_ref(), arith(&bv)?.as_ref())?
            } else {
                return Err(first);
            }
        }
    };
    let d = frames[frame_idx].reg_index(dst);
    reg_store(stack, d, result)?;
    Ok(())
}

/// Нужно ли значению приведение перед арифметикой. Вынесено, чтобы горячий
/// путь не звал [`arith`] ради заведомо чисел.
#[inline(always)]
fn needs_arith_coercion(v: &BslValue) -> bool {
    matches!(v, BslValue::Str(_) | BslValue::Boolean(_))
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
mod tests;
