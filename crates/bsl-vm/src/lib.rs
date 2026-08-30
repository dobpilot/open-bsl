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

/// Компиляция байт-кода в машинный код x86-64. Включается ТОЛЬКО ключом
/// `--jit`: по умолчанию исполнение идёт интерпретатором, и любой отказ
/// JIT-а (неподдержанная инструкция, ядро не дало исполняемую страницу,
/// другая архитектура) молча возвращает на него же.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub(crate) mod jit;

/// На всех прочих платформах JIT-а нет, и `--jit` там просто ничего не
/// меняет. Заглушка, а не `cfg` на каждом месте использования: условная
/// компиляция, размазанная по циклу диспетчеризации, читается хуже.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
pub(crate) mod jit {
    pub const AVAILABLE: bool = false;
    #[allow(dead_code)]
    pub(crate) enum NativeOutcome {
        Continue { pc: usize },
        Yield { pc: usize },
    }
    pub struct CompiledChunk;
    impl CompiledChunk {
        #[allow(clippy::too_many_arguments)]
        pub(crate) fn run(
            &self,
            _pc: usize,
            _frames: &mut Vec<crate::Frame>,
            _stack: &mut Vec<bsl_rt::BslValue>,
            _program: &bsl_bytecode::Program,
            _runtime_shapes: &mut bsl_rt::RuntimeShapes,
            _linked: &crate::LinkedComponents<'_>,
            _quantum_remaining: &mut usize,
        ) -> Option<Result<NativeOutcome, bsl_rt::RtError>> {
            None
        }
    }
    pub fn compile(
        _chunk: &bsl_bytecode::Chunk,
        _builtin_methods: &[Option<bsl_rt::BuiltinMethod>],
        _scheduled: bool,
    ) -> Option<CompiledChunk> {
        None
    }
}

use bsl_bytecode::{ArgMode, DynamicCompiler, Instr, Program};
use bsl_rt::{BslValue, ExecutionToken, PromiseId, RtError};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

static NEXT_EXECUTION_TOKEN: AtomicU64 = AtomicU64::new(1);

type TaskId = usize;

/// Настройка кооперативного планировщика одного запуска VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// Сколько scheduler safe points исполняет задача до перехода в конец
    /// FIFO-очереди. При единственной живой задаче счётчик отключён.
    pub safe_points_per_quantum: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            safe_points_per_quantum: 1_024,
        }
    }
}

struct Task {
    frames: Vec<Frame>,
    stack: Vec<BslValue>,
    current_exception: Option<BslValue>,
    completion: TaskCompletion,
    quantum_remaining: usize,
}

struct ModuleState {
    slots: Vec<BslValue>,
}

impl ModuleState {
    fn new(program: &Program) -> Self {
        Self {
            slots: vec![BslValue::Undefined; program.module_vars.len()],
        }
    }
}

/// Номер модуля кадра. `ROOT_MODULE` — программа, переданная в poll
/// (одиночная либо entry конфигурации); остальные номера — позиции в
/// каталоге. Сравнение с sentinel дешевле `Option<u32>` в горячем цикле.
const ROOT_MODULE: u32 = u32::MAX;

/// Состояние инициализации общего модуля в ОДНОМ сеансе. Политика ленивая:
/// тело модуля исполняется при первом обращении к его символу; момент и
/// повторная попытка после ошибки уточняются замером `JOB.MODULE.INIT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleInitState {
    NotStarted,
    Initializing,
    Ready,
    Failed,
}

/// Экземпляр общего модуля в сеансе: его переменные и стадия
/// инициализации. Между сеансами и потоками не разделяется.
struct ModuleInstance {
    state: ModuleState,
    init: ModuleInitState,
}

/// Сессионные экземпляры всех модулей каталога, индекс — `ModuleId`.
/// У одиночной программы пуст.
#[derive(Default)]
pub struct SessionModules {
    instances: Vec<ModuleInstance>,
}

impl SessionModules {
    fn for_catalog(catalog: &bsl_bytecode::ConfigurationProgram) -> Self {
        Self {
            instances: catalog
                .modules
                .iter()
                .map(|module| ModuleInstance {
                    state: ModuleState::new(&module.program),
                    init: ModuleInitState::NotStarted,
                })
                .collect(),
        }
    }
}

/// Каталожный контекст одного poll: программы модулей и их связанные
/// компонентные таблицы. Живёт не дольше poll, как `LinkedComponents`.
pub struct CatalogContext<'a> {
    catalog: &'a bsl_bytecode::ConfigurationProgram,
    linked: Vec<LinkedComponents<'a>>,
}

/// Модульный контекст шага: сессия, каталог и корневое состояние одним
/// указателем. Горячий `step` получает его вместо трёх отдельных
/// параметров — регистровое давление в цикле диспетчеризации измеримо
/// (A/B чередованием: +10% `call_overhead` на трёх параметрах).
struct ModulesCtx<'a, 'b> {
    session: &'a mut SessionModules,
    catalog: Option<&'a CatalogContext<'b>>,
    /// Корневое состояние, когда текущий кадр — модульный (его собственное
    /// состояние на время шага изъято из сессии); `None` у корневого кадра.
    root_state: Option<&'a mut ModuleState>,
}

impl<'a> CatalogContext<'a> {
    fn program(&self, module: u32) -> Result<&'a Program, RtError> {
        self.catalog
            .modules
            .get(module as usize)
            .map(|m| &m.program)
            .ok_or(RtError::InvalidBytecode(
                "номер модуля кадра вне каталога конфигурации",
            ))
    }

    fn linked(&self, module: u32) -> Result<&LinkedComponents<'a>, RtError> {
        self.linked
            .get(module as usize)
            .ok_or(RtError::InvalidBytecode(
                "номер модуля кадра вне таблиц линковки",
            ))
    }
}

#[derive(Clone, Copy)]
enum TaskCompletion {
    Root,
    Promise(PromiseId),
    Detached,
}

enum PromiseState {
    Pending { waiters: VecDeque<TaskId> },
    Ready(Result<BslValue, RtError>),
}

struct HostCompletion {
    token: ExecutionToken,
    promise_id: PromiseId,
    result: Result<bsl_rt::HttpWireResponse, bsl_rt::NetworkError>,
}

struct VmHttpSink {
    token: ExecutionToken,
    promise_id: PromiseId,
    sender: mpsc::Sender<HostCompletion>,
    /// Пробуждение драйвера этого execution: транспорт зовёт его после
    /// доставки завершения, чтобы спящий без таймера драйвер (worker пула
    /// заданий) опросил execution немедленно.
    waker: Option<ExecutionWaker>,
}

impl bsl_rt::HttpCompletionSink for VmHttpSink {
    fn complete(self: Box<Self>, result: Result<bsl_rt::HttpWireResponse, bsl_rt::NetworkError>) {
        let _ = self.sender.send(HostCompletion {
            token: self.token,
            promise_id: self.promise_id,
            result,
        });
        if let Some(waker) = &self.waker {
            waker();
        }
    }
}

/// Пробуждение драйвера executions: зовётся из потока транспорта после
/// доставки каждого host-завершения. Драйвер, спящий в ожидании событий,
/// просыпается и опрашивает свои executions — таймерного поллинга нет.
pub type ExecutionWaker = std::sync::Arc<dyn Fn() + Send + Sync>;

/// Парковка синхронного host-вызова: задача `task_id` ждёт обещание
/// `promise_id`, а его результат при пробуждении ложится в регистр `dst`
/// инструкции, на которой остановлен `pc` задачи. Пока поле занято,
/// планировщик не запускает другие задачи: Pending синхронного метода
/// замораживает весь execution — в отличие от `Await`, уступающего
/// соседним задачам.
struct SyncWait {
    task_id: TaskId,
    promise_id: PromiseId,
    dst: u8,
}

struct HostPromise {
    handle: Box<dyn bsl_rt::RequestHandle>,
    mapper: bsl_rt::HttpResponseMapper,
}

struct AsyncState {
    token: ExecutionToken,
    scheduler_quantum: usize,
    tasks: Vec<Option<Task>>,
    ready: VecDeque<TaskId>,
    promises: Vec<PromiseState>,
    host_promises: Vec<Option<HostPromise>>,
    completion_sender: mpsc::Sender<HostCompletion>,
    completion_receiver: mpsc::Receiver<HostCompletion>,
    host_waker: Option<ExecutionWaker>,
    sync_wait: Option<SyncWait>,
}

impl AsyncState {
    fn new(root: Task, scheduler_quantum: usize) -> Self {
        let token = ExecutionToken::new(NEXT_EXECUTION_TOKEN.fetch_add(1, Ordering::Relaxed));
        let (completion_sender, completion_receiver) = mpsc::channel();
        Self {
            token,
            scheduler_quantum,
            tasks: vec![Some(root)],
            ready: VecDeque::from([0]),
            promises: Vec::new(),
            host_promises: Vec::new(),
            completion_sender,
            completion_receiver,
            host_waker: None,
            sync_wait: None,
        }
    }

    fn scheduler_quantum(&self) -> usize {
        self.scheduler_quantum
    }

    fn new_promise(&mut self) -> Result<(PromiseId, BslValue), RtError> {
        let raw = u64::try_from(self.promises.len())
            .map_err(|_| RtError::DynamicError("слишком много обещаний в одном запуске".into()))?;
        let id = PromiseId::new(raw);
        self.promises.push(PromiseState::Pending {
            waiters: VecDeque::new(),
        });
        Ok((id, BslValue::new_promise(self.token, id)))
    }

    fn insert_task(&mut self, task: Task) -> TaskId {
        let id = self.tasks.len();
        self.tasks.push(Some(task));
        id
    }

    fn resolve_promise(
        &mut self,
        promise_id: PromiseId,
        result: Result<BslValue, RtError>,
    ) -> Result<(), RtError> {
        let index = usize::try_from(promise_id.get()).map_err(|_| {
            RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы")
        })?;
        let state = self
            .promises
            .get_mut(index)
            .ok_or(RtError::InvalidBytecode(
                "номер обещания вне таблицы запуска",
            ))?;
        let waiters = match std::mem::replace(state, PromiseState::Ready(result)) {
            PromiseState::Pending { waiters } => waiters,
            PromiseState::Ready(_) => {
                return Err(RtError::InvalidBytecode("обещание завершено повторно"));
            }
        };
        self.ready.extend(waiters);
        Ok(())
    }

    fn has_live_tasks(&self) -> bool {
        self.tasks.iter().any(Option::is_some)
    }

    fn has_other_live_task(&self) -> bool {
        // Текущая задача вынута из `tasks` на время исполнения, поэтому
        // любой оставшийся `Some` означает настоящего конкурента за FIFO.
        self.tasks.iter().any(Option::is_some)
    }

    fn has_pending_host_promises(&self) -> bool {
        self.host_promises.iter().any(Option::is_some)
    }

    fn accept_completion(
        &mut self,
        completion: HostCompletion,
        runtime_shapes: &mut bsl_rt::RuntimeShapes,
    ) -> Result<(), RtError> {
        if completion.token != self.token {
            return Ok(());
        }
        let index = usize::try_from(completion.promise_id.get()).map_err(|_| {
            RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы")
        })?;
        let pending = self
            .host_promises
            .get_mut(index)
            .and_then(Option::take)
            .ok_or(RtError::InvalidBytecode(
                "завершение ссылается на отсутствующую host-операцию",
            ))?;
        let result = (pending.mapper)(completion.result, runtime_shapes);
        self.resolve_promise(completion.promise_id, result)
    }

    fn drain_completions(
        &mut self,
        limit: usize,
        block_for_first: bool,
        runtime_shapes: &mut bsl_rt::RuntimeShapes,
    ) -> Result<usize, RtError> {
        if limit == 0 || !self.has_pending_host_promises() {
            return Ok(0);
        }
        let mut accepted = 0;
        if block_for_first {
            let completion = self.completion_receiver.recv().map_err(|_| {
                RtError::DynamicError("канал завершений host-операций закрыт".into())
            })?;
            self.accept_completion(completion, runtime_shapes)?;
            accepted += 1;
        }
        while accepted < limit {
            match self.completion_receiver.try_recv() {
                Ok(completion) => {
                    self.accept_completion(completion, runtime_shapes)?;
                    accepted += 1;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(RtError::DynamicError(
                        "канал завершений host-операций закрыт".into(),
                    ));
                }
            }
        }
        Ok(accepted)
    }
}

impl AsyncState {
    /// Общая регистрация внешней HTTP-операции: обещание, sink с токеном
    /// исполнения и отменяемый handle. Возвращает и номер, и значение
    /// обещания: async-путь отдаёт значение BSL-коду, sync-путь паркует
    /// задачу по номеру.
    fn spawn_host_operation(
        &mut self,
        client: Arc<dyn bsl_rt::HttpClient>,
        request: bsl_rt::HttpWireRequest,
        mapper: bsl_rt::HttpResponseMapper,
        error_mapper: bsl_rt::HttpErrorMapper,
    ) -> Result<(PromiseId, BslValue), RtError> {
        let (promise_id, promise) = self.new_promise()?;
        let sink = Box::new(VmHttpSink {
            token: self.token,
            promise_id,
            sender: self.completion_sender.clone(),
            waker: self.host_waker.clone(),
        });
        let handle = match client.submit(request, sink) {
            Ok(handle) => handle,
            Err(error) => {
                self.promises.pop();
                return Err(error_mapper(error));
            }
        };
        let index = usize::try_from(promise_id.get()).map_err(|_| {
            RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы")
        })?;
        self.host_promises.resize_with(index + 1, || None);
        self.host_promises[index] = Some(HostPromise { handle, mapper });
        Ok((promise_id, promise))
    }

    /// Запускает host-операцию приостанавливающего метода и паркует
    /// задачу: результат ляжет в регистр `dst` остановленной инструкции,
    /// а до его прихода execution заморожен целиком (см. [`SyncWait`]).
    /// Ошибка запуска транспорта возвращается как обычная ловимая ошибка
    /// вызова — парковки тогда не происходит.
    fn begin_sync_host_call(
        &mut self,
        task_id: TaskId,
        dst: u8,
        pending: bsl_rt::PendingHostCall,
    ) -> Result<(), RtError> {
        match pending {
            bsl_rt::PendingHostCall::HttpSync {
                client,
                request,
                mapper,
                error_mapper,
            } => {
                let (promise_id, _promise) =
                    self.spawn_host_operation(client, request, mapper, error_mapper)?;
                let index = usize::try_from(promise_id.get()).map_err(|_| {
                    RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы")
                })?;
                let Some(PromiseState::Pending { waiters }) = self.promises.get_mut(index) else {
                    return Err(RtError::InvalidBytecode(
                        "свежее обещание синхронного вызова уже завершено",
                    ));
                };
                waiters.push_back(task_id);
                self.sync_wait = Some(SyncWait {
                    task_id,
                    promise_id,
                    dst,
                });
                Ok(())
            }
        }
    }
}

impl bsl_rt::HttpPromiseSpawner for AsyncState {
    fn spawn_http(
        &mut self,
        client: Arc<dyn bsl_rt::HttpClient>,
        request: bsl_rt::HttpWireRequest,
        mapper: bsl_rt::HttpResponseMapper,
        error_mapper: bsl_rt::HttpErrorMapper,
    ) -> Result<BslValue, RtError> {
        self.spawn_host_operation(client, request, mapper, error_mapper)
            .map(|(_, promise)| promise)
    }
}

impl Drop for AsyncState {
    fn drop(&mut self) {
        for pending in self.host_promises.iter_mut().filter_map(Option::as_mut) {
            pending.handle.cancel();
        }
    }
}

/// Один активный вызов. Регистры кадра не хранятся отдельным `Vec` — все
/// кадры делят один сквозной стек значений (`Vm::stack`), кадр — это лишь
/// окно в него, как в Lua.
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
///
/// Компилятора динамического кода у этого входа нет: `Выполнить` и
/// `Вычислить` дают ловимую [`RtError::DynamicError`]. Прогон с
/// динамическим кодом идёт через `*_and_io` — там компилятор фрагментов
/// передаётся явно (см. [`bsl_bytecode::DynamicCompiler`]).
///
/// # Errors
///
/// Возвращает [`RtError`], если выполнение завершилось неперехваченным исключением или
/// программа содержит некорректный байт-код.
pub fn run_program(program: &Program) -> Result<BslValue, RtError> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut env = bsl_rt::HostEnv::process();
    run_program_with_host(
        program,
        None,
        JitMode::Off,
        &mut stdout,
        &mut stderr,
        None,
        &mut env,
    )
}

/// Исполняет программу с выводом в потоки, принадлежащие host-приложению.
/// `Сообщить` пишет только в `stdout`; библиотечный API возвращает ошибки и
/// не печатает их в `stderr` автоматически.
///
/// Это ЕДИНСТВЕННАЯ полная форма запуска: JIT — не суффикс имени, а
/// параметр `jit`, потому что это ось возможностей, а не отдельная функция.
/// Формы без ввода-вывода (`*_with_registry`, с JIT и без) удалены: в
/// workspace их никто не звал, а нужный им путь — это `jit` со стандартными
/// потоками процесса.
///
/// `dynamic` — компилятор `Выполнить`/`Вычислить` этого прогона. VM
/// динамический код только исполняет: текст, вид операции и описание
/// области видимости уходят сюда, а обратно приходит готовый чанк.
///
/// # Errors
///
/// До первой инструкции возвращает [`RtError::Link`], если требуемый пакет,
/// версия или код функции отсутствует; далее — те же ошибки, что
/// [`run_program`], включая ошибку записи в пользовательский поток.
pub fn run_program_with_registry_and_io<'a>(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
    jit: JitMode,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<BslValue, RtError> {
    run_program_with_host(
        program,
        Some(registry),
        jit,
        stdout,
        stderr,
        Some(dynamic),
        host_env,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_program_with_host<'a>(
    program: &Program,
    registry: Option<&bsl_rt::RuntimeRegistry>,
    jit_mode: JitMode,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: Option<&'a mut dyn DynamicCompiler>,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<BslValue, RtError> {
    let mut stack = Vec::new();
    push_own_registers(
        &mut stack,
        at(&program.chunks, 0, "в программе нет чанка верхнего уровня")?,
    );
    let linked = link_components(
        program,
        registry,
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
        dynamic,
        dynamic_depth: &dynamic_depth,
    };
    let mut module_state = ModuleState::new(program);
    let (value, _) = drive_linked(
        program,
        0,
        stack,
        jit_mode,
        &linked,
        &mut host,
        &mut module_state,
    )?;
    Ok(value)
}

/// Исполняет чанк REPL с каталогом компонентов: фрагмент, скомпилированный
/// с реестром, несёт `CreateObject`/`CallComponent`, и его требования
/// связываются перед исполнением. Стек предыдущего чанка передаётся внутрь
/// и возвращается наружу — так в сессии живут накопленные переменные.
///
/// # Errors
///
/// Возвращает [`RtError`] при неперехваченном исключении или некорректных
/// таблицах имён, форм и регистров чанка, плюс ошибку
/// связывания компонентов.
// Семь параметров сверх `unit` — это состояние REPL-сессии, разложенное по
// местам: локали, стек и требования собираются вызывающим по одному, а
// потоки, JIT и окружение — сервисы прогона. Чанк, имена и формы, чей
// инвариант связан позицией, теперь приезжают одним [`SnippetUnit`].
#[allow(clippy::too_many_arguments)]
pub fn run_repl_chunk_with_registry<'a>(
    unit: &bsl_bytecode::SnippetUnit,
    locals: Vec<String>,
    stack: Vec<BslValue>,
    requirements: Vec<bsl_bytecode::LibraryRequirement>,
    registry: &bsl_rt::RuntimeRegistry,
    jit: JitMode,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = Program {
        requirements,
        chunks: vec![unit.chunk.clone()],
        names: unit.names.clone(),
        shapes: unit.shapes.clone(),
        top_level_locals: locals,
        function_names: Vec::new(),
        exported_functions: Vec::new(),
        module_vars: Vec::new(),
        exported_module_vars: Vec::new(),
        module_base: 0,
        links: Vec::new(),
    };
    let linked = link_components(
        &program,
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
    let mut module_state = ModuleState::new(&program);
    drive_linked(
        &program,
        0,
        stack,
        jit,
        &linked,
        &mut host,
        &mut module_state,
    )
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
    drive_with(program, func_id, stack, JitMode::Off)
}

/// Host-сервисы одного прогона: куда писать и откуда брать то, чего нет в
/// аргументах BSL-функции (аргументы запуска, часы, случайность).
///
/// Вывод и окружение ходят вместе, потому что оба принадлежат ПРОГОНУ, а
/// не программе: `Program` сериализуется, а это — нет.
struct HostIo<'a, 'd> {
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    /// `None` — у вызывающего окружения нет: так работает шим JIT, у
    /// которого нет и потоков. Функции, которым окружение нужно, туда не
    /// компилируются, поэтому `None` — запись контракта, а не заглушка.
    env: Option<&'a mut bsl_rt::HostEnv>,
    /// Компилятор динамического кода. Он тоже принадлежит ПРОГОНУ, а не
    /// программе, и лежит здесь по той же причине, что и вывод: VM его
    /// зовёт, но не реализует (см. `bsl_bytecode::dynamic`).
    ///
    /// `None` — прогон запущен входом без компилятора фрагментов
    /// (`run_program`, `call_module_function`): тогда `Выполнить` и
    /// `Вычислить` дают ловимую динамическую ошибку, а не тихо ничего.
    dynamic: Option<&'a mut (dyn DynamicCompiler + 'd)>,
    /// Текущая вложенность `Выполнить`/`Вычислить` ЭТОГО прогона. Раньше
    /// была потоковой (`thread_local`) и делилась между сессиями одного
    /// потока; теперь принадлежит прогону. Вложенный `drive`
    /// переиспользует тот же `HostIo`, а обратный вызов функции модуля
    /// строит новый с тем же счётчиком (см. `DynamicDepthGuard`).
    dynamic_depth: &'a std::cell::Cell<usize>,
}

impl HostIo<'_, '_> {
    /// Окружение прогона или ошибка, если его нет.
    ///
    /// # Errors
    ///
    /// [`RtError::InvalidBytecode`], если сюда дошла функция окружения из
    /// контекста без окружения — то есть если список исключений JIT
    /// разошёлся с `bsl_rt::call_builtin_env`.
    fn env(&mut self) -> Result<&mut bsl_rt::HostEnv, RtError> {
        self.env.as_deref_mut().ok_or(RtError::InvalidBytecode(
            "функция окружения вызвана там, где окружения прогона нет",
        ))
    }

    /// Компилятор фрагментов этого прогона или ловимая ошибка.
    ///
    /// # Errors
    ///
    /// [`RtError::DynamicError`], если прогон запущен входом без
    /// компилятора: динамический код в таком прогоне недоступен, и узнаётся
    /// это только сейчас — значит, ошибка обычная, ловимая `Попытка`.
    fn dynamic(&mut self) -> Result<&mut dyn DynamicCompiler, RtError> {
        match self.dynamic.as_deref_mut() {
            Some(compiler) => Ok(compiler),
            None => Err(RtError::DynamicError(
                "Выполнить/Вычислить недоступны: прогон запущен без компилятора динамического кода"
                    .to_string(),
            )),
        }
    }
}

struct LinkedComponents<'a> {
    /// Каталог компонентов прогона. `None` — прогон без реестра: базовый
    /// рантайм и ничего сверх него.
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    /// Чей нулевой чанк исполняется: `DynamicScope::ROOT` у самого модуля
    /// и номер фрагмента (`DynamicUnit::scope`) у программы, собранной
    /// вокруг фрагмента. Половина ключа, по которому хост кэширует
    /// скомпилированные фрагменты, — и приезжает она СНАРУЖИ, потому что
    /// раздаёт номера хост, а не VM.
    scope: u64,

    functions: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    constructors: Vec<Vec<Option<bsl_rt::ComponentCall>>>,
    /// Обработчики встроенных методов по номеру имени программы. Открытый
    /// `CallObjectMethod` исполняется миллионами, и поиск по строке на
    /// каждом вызове недопустим: измерено флеймграфом, до трёх четвертей
    /// времени `csv_write` уходило в `to_uppercase` внутри
    /// `BuiltinMethod::lookup`. Таблица строится один раз при связывании.
    builtin_methods: Vec<Option<bsl_rt::BuiltinMethod>>,
    /// Мемоизация «(статическая таблица типа, номер имени) → обработчик»
    /// для компонентных объектов со статическими таблицами методов
    /// (см. `ObjectProtocol::method_table`): разрешение по строке
    /// происходит один раз на пару, дальше — целочисленный поиск по хешу.
    /// `None` запоминает и промахи, чтобы типы без таблиц не платили за
    /// строку на каждом вызове. Рантайм однопоточный, `RefCell` достаточно.
    component_methods: ComponentMethodMap,
    /// То же для статических таблиц СВОЙСТВ (`ObjectProtocol::property_table`).
    /// Ячейки инструкции у свойств пока нет: она — производная таблица
    /// `Chunk`, её добавление стоит правок формата байт-кода, а выигрыш не
    /// измерен.
    component_properties: ComponentPropertyMap,
    /// Есть ли в реестре библиотека, объявившая
    /// `ObjectContextNeed::Full`, — то есть такая, чьи методы и
    /// свойства вправе читать зону прогона и писать в потоки вывода.
    ///
    /// Сокращённый контекст JIT-шимов ни того, ни другого не даёт, и для
    /// ТАКОЙ программы нативный путь за объектные опкоды не берётся
    /// вовсе. Решение принимается один раз на чанк, при компиляции.
    /// Проверять получателя на каждом обращении было бы точнее, и это
    /// измерено: тела шимов делят кодогенерацию с горячим `step`, и
    /// проверка в них стоила `empty_for` 58 -> 83 млн тактов при
    /// НЕИЗМЕННОЙ семантике интерпретатора.
    ///
    /// Плата приходится только на программу, которая такой компонент
    /// зарегистрировала, и не на отдельные опкоды, а на ВЕСЬ чанк,
    /// который их содержит: гранулярность решения — чанк. Движок с
    /// компонентами дерева не теряет ничего.
    interpreter_only_objects: bool,
    /// Часовой пояс прогона — единственная возможность окружения, которая
    /// нужна КОМПОНЕНТАМ, а значит и обеим ветвям исполнения. Лежит здесь,
    /// а не в `HostIo`, потому что до JIT-шимов доезжает только связанное
    /// состояние: добавить окружение аргументом `CompiledChunk::run`
    /// значило бы вернуть тот самый лишний аргумент во внешнем цикле
    /// диспетчеризации, который при переносе окружения стоил `empty_for`
    /// +61 % тактов.
    zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
    files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    random: bsl_rt::RandomHandle,
    network: Option<std::rc::Rc<dyn bsl_rt::HttpClientFactory>>,
    background_jobs: Option<std::rc::Rc<dyn bsl_rt::BackgroundJobService>>,
    temp_storage: Option<std::rc::Rc<std::cell::RefCell<bsl_rt::TempStorageSession>>>,
    message_sink: Option<std::rc::Rc<dyn bsl_rt::UserMessageSink>>,
}

impl LinkedComponents<'_> {
    fn function(&self, func_id: usize, pc: usize) -> Result<bsl_rt::ComponentCall, RtError> {
        self.functions
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "CallComponent не связан с функцией реестра",
            ))
    }

    fn constructor(&self, func_id: usize, pc: usize) -> Result<bsl_rt::ComponentCall, RtError> {
        self.constructors
            .get(func_id)
            .and_then(|chunk| chunk.get(pc))
            .and_then(|slot| *slot)
            .ok_or(RtError::InvalidBytecode(
                "CreateObject не связан с конструктором реестра",
            ))
    }

    /// Обработчик встроенного метода по номеру имени — без строковых
    /// операций на вызове. `None` — имя не из таблицы ядра: для нативного
    /// получателя это ошибка «метод не применим».
    fn builtin_method(&self, name: bsl_rt::NameId) -> Option<bsl_rt::BuiltinMethod> {
        self.builtin_methods.get(name.index()).copied().flatten()
    }
}

/// Карта мемоизации «(статическая таблица типа, номер имени) → дескриптор».
/// Хранится дескриптор, а не голый обработчик: рантаймная проверка арности
/// метода (арм `CallObjectMethod`, шим JIT) читает из него `arity`.
type ComponentMethodMap = std::cell::RefCell<
    std::collections::HashMap<(usize, u32), Option<&'static bsl_rt::MethodDescriptor>>,
>;

/// Разрешение метода компонентного объекта по статической таблице типа и
/// номеру имени. Строка разбирается один раз на пару «таблица, имя»;
/// установившийся режим — поиск по хешу от двух целых, промахи тоже
/// запоминаются. `None` — имени в таблице нет (или таблица пустая):
/// вызывающий уходит в строковый `call_method`, чтобы текст ошибки остался
/// одним, у самого типа. Свободная функция, а не метод: тем же разрешением
/// пользуется шим открытого метода в JIT, у которого карта приходит сырым
/// указателем из `JitCtx`.
/// Карта мемоизации «(статическая таблица типа, номер имени) → пара
/// обработчиков свойства».
type ComponentPropertyMap = std::cell::RefCell<
    std::collections::HashMap<
        (usize, u32),
        Option<(bsl_rt::PropertyGet, Option<bsl_rt::PropertySet>)>,
    >,
>;

/// Разрешение свойства компонентного объекта — зеркало
/// [`resolve_component_method`]: строка разбирается один раз на пару
/// «таблица, имя», промахи запоминаются тоже. `None` — имени в таблице нет
/// (или таблица пустая), и вызывающий уходит строковым путём, где у типа
/// остаётся единственный источник текста ошибки.
fn resolve_component_property(
    map: &ComponentPropertyMap,
    table: &'static [bsl_rt::PropertyDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<(bsl_rt::PropertyGet, Option<bsl_rt::PropertySet>)>, RtError> {
    let key = (table.as_ptr() as usize, name.index() as u32);
    if let Some(resolved) = map.borrow().get(&key) {
        return Ok(*resolved);
    }
    let written = field_name(program, name)?;
    let resolved = table
        .iter()
        .find(|descriptor| {
            descriptor
                .names
                .iter()
                .any(|candidate| bsl_rt::folded_eq(candidate, written))
        })
        .map(|descriptor| (descriptor.get, descriptor.set));
    map.borrow_mut().insert(key, resolved);
    Ok(resolved)
}

/// Чтение свойства компонентного объекта: таблица типа — быстрым путём,
/// промах и тип без таблицы — строковым `get_property`. Вынесено из арма и
/// помечено `#[inline(never)]`: тело крупное, а горячий цикл живёт на
/// грани кеша микроопераций (см. комментарий у `step_cold`).
#[inline(never)]
fn component_prop_get(
    object: &bsl_rt::ObjectRef,
    properties: &ComponentPropertyMap,
    name: bsl_rt::NameId,
    program: &Program,
    context: &mut bsl_rt::CallContext<'_>,
) -> Result<BslValue, RtError> {
    let table = object.property_table();
    if !table.is_empty()
        && let Some((get, _)) = resolve_component_property(properties, table, name, program)?
    {
        return get(object.as_dyn(), context);
    }
    object.get_property(field_name(program, name)?, context)
}

/// Запись свойства компонентного объекта — двойник `component_prop_get`.
#[inline(never)]
fn component_prop_set(
    object: &bsl_rt::ObjectRef,
    properties: &ComponentPropertyMap,
    name: bsl_rt::NameId,
    value: BslValue,
    program: &Program,
    context: &mut bsl_rt::CallContext<'_>,
) -> Result<(), RtError> {
    let table = object.property_table();
    if !table.is_empty()
        && let Some((_, set)) = resolve_component_property(properties, table, name, program)?
    {
        return match set {
            Some(set) => set(object.as_dyn(), value, context),
            None => Err(RtError::PropertyReadOnly {
                property: field_name(program, name)?.to_string(),
                receiver: object.type_descriptor().name,
            }),
        };
    }
    object.set_property(field_name(program, name)?, value, context)
}

fn resolve_component_method(
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<&'static bsl_rt::MethodDescriptor>, RtError> {
    let key = (table.as_ptr() as usize, name.index() as u32);
    if let Some(resolved) = map.borrow().get(&key) {
        return Ok(*resolved);
    }
    let upper = field_name(program, name)?.to_uppercase();
    let resolved = table.iter().find(|descriptor| {
        descriptor
            .names()
            .iter()
            .any(|candidate| candidate.to_uppercase() == upper)
    });
    map.borrow_mut().insert(key, resolved);
    Ok(resolved)
}

#[allow(clippy::too_many_arguments)]
fn link_components<'a>(
    program: &Program,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
    files: std::rc::Rc<dyn bsl_rt::FileSystem>,
    random: bsl_rt::RandomHandle,
    network: Option<std::rc::Rc<dyn bsl_rt::HttpClientFactory>>,
    background_jobs: Option<std::rc::Rc<dyn bsl_rt::BackgroundJobService>>,
    temp_storage: Option<std::rc::Rc<std::cell::RefCell<bsl_rt::TempStorageSession>>>,
    message_sink: Option<std::rc::Rc<dyn bsl_rt::UserMessageSink>>,
    scope: u64,
) -> Result<LinkedComponents<'a>, RtError> {
    bsl_bytecode::image::verify(program)?;
    // Список собирается ОДИН РАЗ на программу: у обычного движка он пуст,
    // и нативный путь остаётся ровно таким, каким был.
    let interpreter_only_objects =
        registry.is_some_and(bsl_rt::RuntimeRegistry::has_full_context_objects);
    let Some(core) = program.requirements.first() else {
        return Err(RtError::Link(
            "в требованиях отсутствует bsl-rt".to_string(),
        ));
    };
    if core.package != bsl_rt::PACKAGE_NAME || core.version != bsl_rt::PACKAGE_VERSION {
        return Err(RtError::Link(format!(
            "необходим {}={}, исполнитель предоставляет {}={}",
            core.package,
            core.version,
            bsl_rt::PACKAGE_NAME,
            bsl_rt::PACKAGE_VERSION
        )));
    }

    for requirement in &program.requirements[1..] {
        let Some(registry) = registry else {
            return Err(RtError::Link(format!(
                "необходим пакет {}={}, но реестр компонентов не предоставлен",
                requirement.package, requirement.version
            )));
        };
        let Some(library) = registry.library_by_package(&requirement.package) else {
            return Err(RtError::Link(format!(
                "необходим пакет {}={}, но он не зарегистрирован",
                requirement.package, requirement.version
            )));
        };
        if library.version() != requirement.version {
            return Err(RtError::Link(format!(
                "для {} требуется {}, зарегистрирована версия {}",
                requirement.package,
                requirement.version,
                library.version()
            )));
        }
    }

    let mut functions = Vec::with_capacity(program.chunks.len());
    let mut constructors = Vec::with_capacity(program.chunks.len());
    for chunk in &program.chunks {
        let mut function_slots = vec![None; chunk.instrs.len()];
        let mut constructor_slots = vec![None; chunk.instrs.len()];
        for (pc, instruction) in chunk.instrs.iter().enumerate() {
            match instruction {
                Instr::CallComponent {
                    library,
                    function,
                    count,
                    ..
                } => {
                    let requirement = program.requirements.get(*library as usize).ok_or(
                        RtError::InvalidBytecode("индекс библиотеки вне таблицы requirements"),
                    )?;
                    let Some(registry) = registry else {
                        return Err(RtError::Link(format!(
                            "функция {}/{} требует реестр компонентов",
                            requirement.package, function
                        )));
                    };
                    let library_descriptor = registry
                        .library_by_package(&requirement.package)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "необходим пакет {}={}, но он не зарегистрирован",
                                requirement.package, requirement.version
                            ))
                        })?;
                    let descriptor = library_descriptor
                        .functions()
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *function)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "компонент {} не содержит функцию с кодом {}",
                                requirement.package, function
                            ))
                        })?;
                    if !descriptor.arity.accepts(*count) {
                        return Err(RtError::InvalidBytecode(
                            "арность CallComponent не совпадает с дескриптором",
                        ));
                    }
                    function_slots[pc] = Some(descriptor.call);
                }
                Instr::CreateObject {
                    library,
                    constructor,
                    count,
                    ..
                } => {
                    let requirement = program.requirements.get(*library as usize).ok_or(
                        RtError::InvalidBytecode("индекс библиотеки вне таблицы requirements"),
                    )?;
                    let library_descriptor = match registry {
                        Some(registry) => *registry
                            .library_by_package(&requirement.package)
                            .ok_or_else(|| {
                                RtError::Link(format!(
                                    "необходим пакет {}={}, но он не зарегистрирован",
                                    requirement.package, requirement.version
                                ))
                            })?,
                        None if requirement.package == bsl_rt::PACKAGE_NAME => {
                            bsl_rt::core_library()
                        }
                        None => {
                            return Err(RtError::Link(format!(
                                "конструктор {}/{} требует реестр компонентов",
                                requirement.package, constructor
                            )));
                        }
                    };
                    let descriptor = library_descriptor
                        .constructors()
                        .iter()
                        .find(|descriptor| descriptor.code.get() == *constructor)
                        .ok_or_else(|| {
                            RtError::Link(format!(
                                "компонент {} не содержит конструктор с кодом {}",
                                requirement.package, constructor
                            ))
                        })?;
                    if !descriptor.arity.accepts(*count) {
                        return Err(RtError::InvalidBytecode(
                            "арность CreateObject не совпадает с дескриптором",
                        ));
                    }
                    constructor_slots[pc] = Some(descriptor.call);
                }
                _ => {}
            }
        }
        functions.push(function_slots);
        constructors.push(constructor_slots);
    }
    // Диспетчеризация открытых методов: карта «номер имени → обработчик»
    // строится здесь один раз, чтобы `CallObjectMethod` не искал метод по
    // строке на каждом вызове. Сама `lookup` дешёвая (хеш-карта на
    // процесс), поэтому связывание фрагмента `Выполнить` она не утяжеляет.
    let builtin_methods = program
        .names
        .iter()
        .map(|name| bsl_rt::BuiltinMethod::lookup(name))
        .collect();
    Ok(LinkedComponents {
        registry,
        scope,
        interpreter_only_objects,
        zone,
        files,
        random,
        network,
        background_jobs,
        temp_storage,
        message_sink,
        functions,
        constructors,
        builtin_methods,
        component_methods: std::cell::RefCell::new(std::collections::HashMap::new()),
        component_properties: std::cell::RefCell::new(std::collections::HashMap::new()),
    })
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

/// Предел вложенности `Выполнить`/`Вычислить` друг в друге. В отличие от
/// кадров BSL, каждый уровень динамического кода — это настоящий вложенный
/// `drive` на стеке Rust (плюс разбор и компиляция фрагмента), поэтому
/// предел защищает стек процесса, а не память: без него рекурсия через
/// `Выполнить` валит процесс переполнением стека, минуя `Попытка`.
// НЕ ИЗМЕРЕНО(EXEC.DYNAMIC_DEPTH) — сколько уровней допускает платформа;
// замер даёт только нижнюю границу (40 уровней обязаны работать).
const MAX_DYNAMIC_DEPTH: usize = 64;

/// Вход в очередной уровень динамического кода; выход — в `Drop`, чтобы
/// счётчик не съезжал ни на одном из путей ошибки.
///
/// Счётчик — не потоковый, а поле [`HostIo`] прогона: две сессии в одном
/// потоке (например, вложенный `Engine` за обратным вызовом функции) не
/// делят вложенность `Выполнить`. Вложенный `drive` переиспользует тот же
/// `HostIo`, а обратный вызов функции модуля строит новый — но с тем же
/// `dynamic_depth` родителя, — так что уровень протаскивается через
/// прогон, а не через поток.
struct DynamicDepthGuard<'a> {
    depth: &'a std::cell::Cell<usize>,
}

impl<'a> DynamicDepthGuard<'a> {
    fn enter(depth: &'a std::cell::Cell<usize>) -> Result<Self, RtError> {
        if depth.get() >= MAX_DYNAMIC_DEPTH {
            Err(RtError::StackOverflow {
                what: "слишком глубокая вложенность Выполнить/Вычислить",
            })
        } else {
            depth.set(depth.get() + 1);
            Ok(DynamicDepthGuard { depth })
        }
    }
}

impl Drop for DynamicDepthGuard<'_> {
    fn drop(&mut self) {
        self.depth.set(self.depth.get() - 1);
    }
}

/// Включён ли JIT. Отдельный тип, а не `bool`: у вызова `drive(.., true)`
/// на месте вызова не видно, что именно включается.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JitMode {
    Off,
    On,
}

#[cfg(test)]
fn drive_with(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
    jit_mode: JitMode,
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
        jit_mode,
        &linked,
        &mut host,
        &mut module_state,
    )?;
    Ok((value, module_state.slots))
}

/// Одноразовая подготовка прогона: таблица имён и форм плюс место под
/// скомпилированные чанки.
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
fn drive_prologue(
    program: &Program,
    jit_mode: JitMode,
    linked: &LinkedComponents,
) -> (
    bsl_rt::RuntimeShapes,
    Vec<Option<Option<jit::CompiledChunk>>>,
) {
    // Затравлена формами/именами ЭТОЙ программы — см. `bsl_rt::RuntimeShapes`
    // doc comment про то, почему не общий на процесс синглтон: у вложенного
    // `Program` (см. `run_dynamic_snippet`) свои `names`/`shapes`, и рантайм-
    // расширения этой таблицы (`Вставить`/`Удалить` на структуре, меняющие
    // её форму) актуальны только для объектов внутри ОДНОГО такого вызова.
    // Типы компонентов и каталог их написаний приходят из реестра ОДНИМ
    // вызовом: промежуточного состояния «формы есть, типов ещё нет» не
    // существует. По ним `Тип("Имя")` находит то, чего нет в закрытом
    // реестре ядра (см. `TypeRef`).
    let runtime_shapes = bsl_rt::RuntimeShapes::seeded(
        program.names.clone(),
        program.shapes.clone(),
        linked.registry,
    );
    // Скомпилированные чанки. Внешний `None` — «ещё не пробовали»,
    // внутренний — «пробовали, JIT отказался»: компилировать чанк заново
    // на каждом входе в него стоило бы дороже любого выигрыша.
    let native: Vec<Option<Option<jit::CompiledChunk>>> =
        if jit_mode == JitMode::On && jit::AVAILABLE {
            (0..program.chunks.len()).map(|_| None).collect()
        } else {
            Vec::new()
        };
    (runtime_shapes, native)
}

/// Результат продвижения сохраняемого запуска VM.
#[derive(Debug)]
pub enum ProgramPoll {
    Complete(BslValue, Vec<BslValue>),
    Runnable,
    Waiting,
}

/// Состояние одного запуска программы, сохраняемое между вызовами `poll`.
/// Не содержит ссылок на host-сервисы и после завершения освобождает их для
/// следующего запуска того же `State`.
pub struct ProgramExecution {
    async_state: AsyncState,
    runtime_shapes: bsl_rt::RuntimeShapes,
    native: Vec<Option<Option<jit::CompiledChunk>>>,
    native_scheduled: Vec<Option<Option<jit::CompiledChunk>>>,
    merge_linear: bool,
    root_result: Option<(BslValue, Vec<BslValue>)>,
    module_state: ModuleState,
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
        jit_mode: JitMode,
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
        let (runtime_shapes, native) = drive_prologue(program, jit_mode, linked);
        let native_scheduled = (0..native.len()).map(|_| None).collect();
        // Без JIT `step` может сцеплять линейные цепочки бандлов, не
        // возвращаясь сюда; JIT требует возврата после каждого бандла.
        let merge_linear = native.is_empty();
        Self {
            async_state,
            runtime_shapes,
            native,
            native_scheduled,
            merge_linear,
            root_result: None,
            module_state,
            session_modules: SessionModules::default(),
            force_scheduled: false,
            cancel_flag: None,
            finished: false,
        }
    }

    /// Подключает сессионные экземпляры модулей каталога: по одному
    /// `ModuleInstance` на модуль, все в состоянии `NotStarted`. Вызывается
    /// один раз при создании конфигурационного запуска.
    pub fn attach_catalog(&mut self, catalog: &bsl_bytecode::ConfigurationProgram) {
        self.session_modules = SessionModules::for_catalog(catalog);
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
        jit_mode: JitMode,
        host_env: &bsl_rt::HostEnv,
    ) -> Result<Self, RtError> {
        Self::start_with_registry_and_scheduler(
            program,
            registry,
            jit_mode,
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
        jit_mode: JitMode,
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
            jit_mode,
            &linked,
            ModuleState::new(program),
            scheduler,
        ))
    }

    /// Продвигает ранее созданный запуск, не сохраняя ссылок на host между
    /// вызовами. Конечный `host_slice` не блокирует ожидание completion;
    /// `usize::MAX` используется run-to-completion драйвером и ждёт первый.
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
            native,
            native_scheduled,
            merge_linear,
            root_result,
            module_state,
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
                let fast_numeric_for = {
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
                let (cur_program, cur_linked) = if cur_module == ROOT_MODULE {
                    (program, linked)
                } else {
                    let Some(ctx) = catalog else {
                        return Err(RtError::InvalidBytecode(
                            "кадр модуля конфигурации без каталожного контекста",
                        ));
                    };
                    (ctx.program(cur_module)?, ctx.linked(cur_module)?)
                };
                // Нативный путь. Он не обязан ничего исполнить: если на текущей
                // позиции входа нет, управление просто идёт в `step`, и это же
                // происходит при любом отказе JIT-а.
                if cur_module == ROOT_MODULE && !native.is_empty() {
                    let native_slots = if scheduled {
                        &mut *native_scheduled
                    } else {
                        &mut *native
                    };
                    let (fid, pc) = {
                        let frame = task
                            .frames
                            .last()
                            .expect("инвариант VM: drive всегда держит хотя бы один кадр");
                        (frame.func_id, frame.pc)
                    };
                    if let Some(slot) = native_slots.get_mut(fid) {
                        if slot.is_none() {
                            // Чанк, ТРОГАЮЩИЙ объекты, нативному пути не отдаётся,
                            // если реестр несёт библиотеку «только интерпретатор»:
                            // её обработчикам нужен полный контекст, а у шимов он
                            // сокращённый. Решение принимается один раз на чанк —
                            // сам `jit` при этом не меняется ни на строку, и это
                            // не педантизм: любая правка его кода сдвигает укладку
                            // и бьёт по горячему циклу интерпретатора (`empty_for`
                            // платил 58 -> 83 млн тактов при неизменном числе
                            // инструкций).
                            *slot = Some(
                                program
                                    .chunks
                                    .get(fid)
                                    .filter(|chunk| {
                                        !chunk.touches_objects || !linked.interpreter_only_objects
                                    })
                                    .and_then(|chunk| {
                                        jit::compile(chunk, &linked.builtin_methods, scheduled)
                                    }),
                            );
                        }
                        // Два слоя внутри уже найденной ячейки: пробовали ли этот
                        // чанк и вышло ли. Повторно искать его в таблице незачем.
                        if let Some(Some(code)) = slot.as_ref()
                            && let Some(outcome) = code.run(
                                pc,
                                &mut task.frames,
                                &mut task.stack,
                                program,
                                runtime_shapes,
                                linked,
                                &mut task.quantum_remaining,
                            )
                        {
                            match outcome {
                                Ok(jit::NativeOutcome::Continue { pc: next_pc }) => {
                                    if let Some(frame) = task.frames.last_mut() {
                                        frame.pc = next_pc;
                                    }
                                    continue;
                                }
                                Ok(jit::NativeOutcome::Yield { pc: next_pc }) => {
                                    if let Some(frame) = task.frames.last_mut() {
                                        frame.pc = next_pc;
                                    }
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
                                            TaskCompletion::Root | TaskCompletion::Detached => {
                                                return Err(e);
                                            }
                                            TaskCompletion::Promise(promise_id) => {
                                                async_state.resolve_promise(promise_id, Err(e))?;
                                                break;
                                            }
                                        }
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                }

                // `step` исполняет целый VLIW-бандл (см. `bsl_bytecode::bundle`),
                // так что проверки fast numeric-for и JIT-входа выше происходят на
                // границах бандлов, а не на каждой инструкции. При ошибке члена
                // `pc` стоит на нём самом, и `unwind_to_handler` находит обработчик
                // как при поинструкционном исполнении; обработчик по построению
                // разметки — начало бандла.
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

/// Изъятие пробуждённой замороженной задачи из готовых: при активной
/// парковке синхронного вызова исполняется только она сама, остальные
/// готовые ждут в очереди. Вынесено из `poll_linked` ради укладки его
/// горячего цикла.
#[inline(never)]
fn take_frozen_ready(async_state: &mut AsyncState) -> Option<TaskId> {
    let frozen = async_state.sync_wait.as_ref()?.task_id;
    let position = async_state
        .ready
        .iter()
        .position(|&candidate| candidate == frozen)?;
    async_state.ready.remove(position)
}

/// Возобновление задачи, пробуждённой из парковки синхронного вызова:
/// результат обещания — в регистр назначения, `pc` — за инструкцию;
/// ошибка транспорта или материализации разматывается с `pc` на самой
/// инструкции (ловимость — как у блокирующего пути). `Ok(true)` — задача
/// продолжает исполнение (обычное или в найденном обработчике), включая
/// чужую задачу, пробуждённую без парковки; `Ok(false)` — задача
/// завершилась ошибкой обещания и умерла; `Err` — неперехваченная ошибка
/// корневой задачи. Вынесено из `poll_linked` ради укладки.
#[inline(never)]
fn resume_parked_task(
    task: &mut Task,
    task_id: TaskId,
    async_state: &mut AsyncState,
    program: &Program,
    catalog: Option<&CatalogContext<'_>>,
    session_modules: &mut SessionModules,
) -> Result<bool, RtError> {
    if async_state
        .sync_wait
        .as_ref()
        .is_none_or(|wait| wait.task_id != task_id)
    {
        return Ok(true);
    }
    let wait = async_state
        .sync_wait
        .take()
        .expect("проверено строкой выше");
    let Err(error) = resume_sync_host_call(task, async_state, &wait) else {
        return Ok(true);
    };
    if unwind_to_handler(
        &mut task.frames,
        &mut task.stack,
        program,
        catalog,
        session_modules,
        &error,
        &mut task.current_exception,
    ) {
        return Ok(true);
    }
    match task.completion {
        TaskCompletion::Root | TaskCompletion::Detached => Err(error),
        TaskCompletion::Promise(promise_id) => {
            async_state.resolve_promise(promise_id, Err(error))?;
            Ok(false)
        }
    }
}

/// Применяет итог завершённого синхронного host-вызова к припаркованной
/// задаче: значение — в регистр назначения, `pc` — за инструкцию. Ошибка
/// возвращается вызывающему для обычного разматывания: `pc` задачи стоит
/// на самой инструкции вызова.
#[inline(never)]
fn resume_sync_host_call(
    task: &mut Task,
    async_state: &mut AsyncState,
    wait: &SyncWait,
) -> Result<(), RtError> {
    let index = usize::try_from(wait.promise_id.get())
        .map_err(|_| RtError::InvalidBytecode("номер обещания не помещается в индекс таблицы"))?;
    let state = async_state
        .promises
        .get_mut(index)
        .ok_or(RtError::InvalidBytecode(
            "номер обещания вне таблицы запуска",
        ))?;
    // Результат забирается насовсем: копия ответа в таблице обещаний
    // больше никому не нужна — ждала ровно одна задача.
    let result = match std::mem::replace(state, PromiseState::Ready(Ok(BslValue::Undefined))) {
        PromiseState::Ready(result) => result,
        PromiseState::Pending { .. } => {
            return Err(RtError::InvalidBytecode(
                "пробуждение синхронного вызова с незавершённым обещанием",
            ));
        }
    };
    let value = result?;
    let frame = task
        .frames
        .last_mut()
        .ok_or(RtError::InvalidBytecode("замороженная задача без кадра"))?;
    let destination = frame.reg_index(wait.dst);
    reg_store(&mut task.stack, destination, value)?;
    frame.pc += 1;
    Ok(())
}

fn drive_linked(
    program: &Program,
    func_id: usize,
    stack: Vec<BslValue>,
    jit_mode: JitMode,
    linked: &LinkedComponents,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let owned_module_state = ModuleState {
        slots: std::mem::take(&mut module_state.slots),
    };
    let mut execution = ProgramExecution::new_linked(
        program,
        func_id,
        stack,
        jit_mode,
        linked,
        owned_module_state,
        SchedulerConfig::default(),
    );
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

#[derive(Clone, Copy)]
struct TaskPosition {
    frame_depth: usize,
    func_id: usize,
    pc: usize,
}

fn task_position(task: &Task) -> TaskPosition {
    let frame = task
        .frames
        .last()
        .expect("инвариант VM: готовая задача всегда имеет кадр");
    TaskPosition {
        frame_depth: task.frames.len(),
        func_id: frame.func_id,
        pc: frame.pc,
    }
}

fn crossed_scheduler_safe_point(before: TaskPosition, task: &Task) -> bool {
    let after = task_position(task);
    before.frame_depth != after.frame_depth
        || before.func_id != after.func_id
        || after.pc <= before.pc
}

fn consume_scheduler_safe_point(task: &mut Task, scheduled: bool, quantum: usize) -> bool {
    if !scheduled {
        task.quantum_remaining = quantum;
        return false;
    }
    task.quantum_remaining -= 1;
    task.quantum_remaining == 0
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
/// `prop_cache` кодоген заводит длиной со всем `instrs`, но чанк мог
/// прийти и не от него.
#[inline]
fn prop_cache(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
) -> Result<&bsl_bytecode::PropCacheSlot, RtError> {
    at(
        &chunk.prop_cache,
        pc,
        "нет ячейки инлайн-кэша для инструкции",
    )
}

/// Ячейка инлайн-кэша `CallObjectMethod` на позиции `pc` — см.
/// [`cached_component_method`].
#[inline]
fn method_cache(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
) -> Result<&bsl_bytecode::MethodCacheSlot, RtError> {
    at(
        &chunk.method_cache,
        pc,
        "нет ячейки кэша метода для инструкции",
    )
}

/// Разрешение метода компонентного объекта с кэшем на позиции инструкции:
/// мономорфный сайт после первого вызова читает обработчик из своей ячейки
/// по одному сравнению адреса таблицы, не трогая карту мемоизации. Смена
/// типа получателя на том же сайте (полиморфизм) перечитывает карту и
/// перезаписывает ячейку; `None` кэшируется наравне с попаданием — тип без
/// имени в таблице не платит за строку и хэш на каждый вызов.
fn cached_component_method(
    chunk: &bsl_bytecode::Chunk,
    pc: usize,
    map: &ComponentMethodMap,
    table: &'static [bsl_rt::MethodDescriptor],
    name: bsl_rt::NameId,
    program: &Program,
) -> Result<Option<&'static bsl_rt::MethodDescriptor>, RtError> {
    let slot = method_cache(chunk, pc)?;
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
/// возврата в `drive_with` между членами. При `merge_linear` (чисто
/// интерпретаторный режим, без JIT) исполнение продолжается и через
/// границу бандла, пока `pc` идёт линейно: пробы `drive_with` имеют смысл
/// только там, куда `pc` попадает переходом, вызовом или разматыванием.
#[allow(clippy::too_many_arguments)]
fn step(
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    program: &Program,
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
    // (середина бандла — сюда возвращается JIT, отказавшийся от
    // инструкции) и пустая таблица равнозначны одиночному исполнению.
    // Разметке можно верить, потому что из файла она не читается — её
    // всегда пересчитывает `bundle::compute`; ошибка члена оставляет `pc`
    // на нём самом, и `Попытка` ищется ровно как при поинструкционном
    // исполнении.
    // «Сколько членов ЗА первым»: у одиночного бандла и в середине бандла
    // ноль — путь обычной инструкции оплачивает ровно одну загрузку `u8`
    // и вычитание, вся петлевая бухгалтерия лежит после исполнения члена.
    let mut extra = chunk
        .bundle_len
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
                // АБСОЛЮТНЫЙ индекс: модульные переменные лежат подряд,
                // начиная с `module_base`. У обычной программы база нулевая
                // — это первые слоты кадра верхнего уровня, а он стоит в
                // самом низу стека и живёт всё исполнение. Проверка границы
                // обязательна — байт-код может прийти и не от кодогена.
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
                        // `module_base + slot` (первые слоты кадра нулевого
                        // уровня), а не в кадре вызывающего: алиас указывает
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
                // Структура резолвится через инлайн-кэш этой ИНСТРУКЦИИ
                // (см. Chunk::prop_cache): мономорфный сайт вызова после
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
                    match ov.get_field_cached(name, prop_cache(chunk, pc)?) {
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
                    match ov.set_field_cached(name, sv.clone(), prop_cache(chunk, pc)?) {
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
            // Бандл кончился. Без JIT линейная цепочка бандлов
            // продолжается прямо здесь: смену кадра (`Call`) ловит первая
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
                .bundle_len
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
                match ov.get_field_cached(name_id, prop_cache(chunk, frames[frame_idx].pc)?) {
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
                    prop_cache(chunk, frames[frame_idx].pc)?,
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
                        // Обратный вызов функции модуля из компонента всегда
                        // шёл интерпретатором (форма `..._with_host` зашивала
                        // `Off`); параметр эту семантику сохраняет.
                        JitMode::Off,
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
                    chunk,
                    frames[frame_idx].pc,
                    &linked.component_methods,
                    object.method_table(),
                    name_id,
                    program,
                )? {
                    Some(descriptor) => {
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

/// Исполняет `code` в контексте переменных текущего кадра (см.
/// `Instr::RunDynamic`).
///
/// Компилирует НЕ эта функция: текст, вид операции и описание области
/// уходят компилятору хоста (`bsl_bytecode::DynamicCompiler`), а обратно
/// приходит готовый `DynamicUnit`. Здесь остаётся ровно механика
/// исполнения — перенос значений внутрь фрагмента и обратно.
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
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
) -> Result<BslValue, RtError> {
    // Предел вложенности — на входе, до обращения к хосту: компиляция
    // фрагмента рекурсивна так же, как его исполнение, и тоже расходует
    // стек Rust.
    let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;

    // Что фрагмент знает о функциях модуля, в порядке `chunks[1..]`: имя,
    // арность, вид объявления и режимы параметров. Всё заимствовано у
    // программы — запрос собирается на каждом `RunDynamic`, в том числе
    // когда фрагмент уже лежит в кэше хоста.
    let functions: Vec<bsl_bytecode::DynamicSignature<'_>> = program
        .function_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let chunk = program.chunks.get(i + 1);
            bsl_bytecode::DynamicSignature {
                name,
                arity: chunk.map_or(0, |c| c.n_params as usize),
                is_procedure: chunk.is_some_and(|c| c.is_procedure),
                is_async: chunk.is_some_and(|c| c.is_async),
                param_by_val: chunk.map_or(&[][..], |c| &c.param_by_val),
                param_has_default: chunk.map_or(&[][..], |c| &c.param_has_default),
            }
        })
        .collect();
    let request = bsl_bytecode::DynamicRequest {
        source: code,
        kind: if is_eval {
            bsl_bytecode::DynamicKind::Eval
        } else {
            bsl_bytecode::DynamicKind::Execute
        },
        scope: bsl_bytecode::DynamicScope {
            // Чанки 1.. — процедуры и функции ИСХОДНОГО модуля: во
            // фрагмент они едут как есть, таблицы локальных у них те же, и
            // область у них корневая на любой глубине вложенности. Своя
            // таблица только у нулевого чанка — вот он и берёт номер того,
            // чей он: модуля или фрагмента вокруг.
            program: if scope_id == 0 {
                linked.scope
            } else {
                bsl_bytecode::DynamicScope::ROOT
            },
            chunk: scope_id as u32,
        },
        caller_is_async: at(
            &program.chunks,
            scope_id,
            "номер чанка динамического вызова вне таблицы функций",
        )?
        .is_async,
        locals: scope_locals,
        module_vars: &program.module_vars,
        functions: &functions,
        names: &program.names,
        requirements: &program.requirements,
    };
    // Неудача любой фазы фронтенда — обычное исключение В МОМЕНТ
    // ИСПОЛНЕНИЯ, а не паника: текст фрагмента становится известен только
    // сейчас, и кривой текст обязан ловиться `Попытка`. Поэтому контракт
    // хоста и отдаёт текст ошибки, а не готовый `RtError`: каким видом
    // ошибки станет неудача, решает VM, а не хост.
    let compiled = host
        .dynamic()?
        .compile(&request)
        .map_err(RtError::DynamicError)?;

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

    // Чанки функций едут во фрагмент КАК ЕСТЬ, только нулевой заменён на
    // сам фрагмент: измерено, что `Вычислить("Удвоить(21)")` на платформе
    // работает, а `Call func=N` у нас индексирует ровно `chunks[N]`.
    // Поэтому нумерация обязана совпасть с исходной программой.
    let mut chunks = program.chunks.clone();
    for chunk in &mut chunks {
        remap_chunk_libraries(chunk, &program.requirements, &compiled.requirements)?;
    }
    if chunks.is_empty() {
        chunks.push(compiled.chunk.clone());
    } else {
        chunks[0] = compiled.chunk.clone();
    }
    // Разметка бандлов фрагмента остаётся ПУСТОЙ (поинструкционное
    // исполнение). Прежний расчёт в `compile_snippet` звал `compute` с
    // `overlap = None` по ложной посылке «у фрагмента `module_base != 0`»:
    // у верхнего уровня он нулевой, модульные слоты накладываются на первые
    // `n_module` регистров, и разметка делала ЛОЖНОЕ утверждение о
    // независимости членов. Верный `overlap` известен только здесь
    // (`aliased`, `n_module`), но пересчитывать его на КАЖДОМ `Выполнить` —
    // измеренные +9.6 % на eval-в-цикле ради соундности доказательства,
    // которое над фрагментами ни в рантайме, ни в тестах не проверяется
    // (`bundle::verify` идёт по статическому корпусу). Пустой вектор не
    // делает никакого утверждения — он и сонадёжен, и бесплатен; фрагмент
    // одноразовый, потеря пакетной диспетчеризации на нём незначима.
    let snippet_program = Program {
        requirements: compiled.requirements.clone(),
        chunks,
        // СОБСТВЕННАЯ таблица фрагмента, не `program.names`: она — префикс
        // (те же имена, в том же порядке, значит те же `NameId`) плюс,
        // возможно, новые поля, которых в статическом коде не было (см.
        // doc comment `bsl_bytecode::DynamicUnit`). Старые `GetProp`/`SetProp` — и
        // статического кода вокруг, и вложенных вызовов функций программы
        // (см. ниже про `chunks`) — по-прежнему резолвятся: их `NameId`
        // меньше длины `program.names` и указывают на тот же префикс.
        names: compiled.names.clone(),
        shapes: compiled.shapes.clone(),
        top_level_locals: Vec::new(),
        function_names: program.function_names.clone(),
        exported_functions: program.exported_functions.clone(),
        module_vars: program.module_vars.clone(),
        exported_module_vars: program.exported_module_vars.clone(),
        module_base: 0,
        links: Vec::new(),
    };

    let snippet_linked = link_components(
        &snippet_program,
        linked.registry,
        std::rc::Rc::clone(&linked.zone),
        std::rc::Rc::clone(&linked.files),
        linked.random.clone(),
        linked.network.as_ref().map(std::rc::Rc::clone),
        linked.background_jobs.as_ref().map(std::rc::Rc::clone),
        linked.temp_storage.as_ref().map(std::rc::Rc::clone),
        linked.message_sink.as_ref().map(std::rc::Rc::clone),
        compiled.scope.get(),
    )?;
    let (value, final_stack) = drive_linked(
        &snippet_program,
        0,
        snippet_stack,
        JitMode::Off,
        &snippet_linked,
        host,
        module_state,
    )?;

    // Обратно переносятся ТОЛЬКО уже существовавшие слоты: их номера
    // совпадают с теми, что использует окружающий скомпилированный код.
    // Имена, объявленные самим фрагментом, получили слоты ЗА `old_count` и
    // никуда не переносятся — расширить статически размеченный кадр нечем.
    //
    // ИЗМЕРЕНО на 8.3.27: платформа ведёт себя ТАК ЖЕ — имя, впервые
    // созданное внутри `Выполнить`, вызов не переживает. Выбор оказался
    // верным, кадр с именной таблицей не нужен.
    //
    for i in 0..old_count {
        let d = frame.reg_index(i as u8);
        reg_store(stack, d, reg_load(&final_stack, i)?)?;
    }

    Ok(value)
}

/// Вызывает процедуру или функцию модуля ПО ИМЕНИ — узкая точка входа для
/// рантайма, которому надо позвать пользовательский код по строке, пришедшей
/// из данных (функция восстановления `ПрочитатьJSON`, обработчик события и
/// далее в том же духе). Машинерия под ней та же, что у изолированного
/// исполнения `Выполнить`/`Вычислить` (см. `run_dynamic_snippet`), только
/// без текстовой прослойки: чанк уже скомпилирован, компилировать и
/// кэшировать нечего.
///
/// Имя ищется регистронезависимо (через `to_uppercase`, как сравнивает
/// идентификаторы `bsl-sema`) по [`Program::function_names`], куда входят и
/// процедуры, и функции.
///
/// `stack` — стек значений вызывающего: из него по [`Program::module_base`]
/// читается блок модульных переменных и в него же он возвращается после
/// вызова, поэтому запись в модульную переменную из вызванной процедуры
/// переживает вызов ровно так же, как при обычном `Call`.
///
/// Аргументы передаются ПО ЗНАЧЕНИЮ, даже для параметров без `Знач`:
/// алиасить нечего — вызов приходит не с места вызова в BSL, а изнутри
/// рантайма, и «переменной вызывающего» здесь не существует. Наблюдать то,
/// что вызванный код записал в свой параметр, позволяет второй элемент
/// возвращаемой пары — ФИНАЛЬНЫЕ значения слотов параметров, длиной
/// `n_params` и в порядке объявления (это будущий канал для `Отказ`).
/// Первый элемент — значение `Возврат`; у процедуры и у функции, дошедшей
/// до конца тела без `Возврат`, это `Неопределено`.
///
/// # Errors
///
/// - [`RtError::DynamicError`] — имени нет в модуле либо число аргументов
///   не совпало с числом параметров. И то, и другое приходит из
///   пользовательских данных, поэтому это перехватываемая `Попытка` ошибка,
///   а не паника. Аргументов обязано быть РОВНО `n_params`: пропустить
///   позицию, как это делает `Ф(1, , 3)` в BSL, отсюда нельзя — режим
///   аргумента живёт в инструкции `Call`, а этот вызов её не проходит.
/// - [`RtError::StackOverflow`] — превышена вложенность динамических
///   вызовов: вызов по имени — такой же вложенный `drive` на стеке Rust,
///   как `Выполнить`, и рекурсия через него не должна валить процесс мимо
///   `Попытка`.
/// - [`RtError::DynamicError`] — вызванный код содержит
///   `Выполнить`/`Вычислить`: компилятора фрагментов у этого входа нет,
///   он есть у [`call_module_function_with_registry_and_io`].
/// - Любая ошибка самого исполнения, не перехваченная внутри вызванного
///   кода, а также [`RtError::InvalidBytecode`], если `program`
///   рассогласована (имя функции есть, чанка под него нет) или `stack`
///   короче блока модульных переменных.
pub fn call_module_function(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
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
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: Some(&mut env),
        dynamic: None,
        dynamic_depth: &dynamic_depth,
    };
    call_module_function_with_host(program, stack, name, args, JitMode::Off, &linked, &mut host)
}

/// Вызывает функцию модуля с реестром компонентов и потоками текущего
/// host-состояния.
///
/// `jit` — ось возможностей, как у [`run_program_with_registry_and_io`]:
/// тонкая обёртка [`call_module_function`] передаёт [`JitMode::Off`], а
/// полная форма принимает его параметром. Стоит ли фасаду когда-то давать
/// сюда что-то кроме `Off`, решается замером, а не формой API.
///
/// # Errors
///
/// Помимо ошибок [`call_module_function`] возвращает ошибку связывания,
/// если модулю недоступен требуемый компонент или его точная версия.
#[allow(clippy::too_many_arguments)]
pub fn call_module_function_with_registry_and_io<'a>(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    registry: &bsl_rt::RuntimeRegistry,
    jit: JitMode,
    stdout: &'a mut dyn Write,
    stderr: &'a mut dyn Write,
    dynamic: &'a mut dyn DynamicCompiler,
    host_env: &'a mut bsl_rt::HostEnv,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
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
    call_module_function_with_host(program, stack, name, args, jit, &linked, &mut host)
}

fn call_module_function_with_host(
    program: &Program,
    stack: &mut [BslValue],
    name: &str,
    args: Vec<BslValue>,
    jit: JitMode,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let mut module_state = ModuleState {
        slots: (0..program.module_vars.len())
            .map(|i| reg_load(stack, program.module_base as usize + i))
            .collect::<Result<_, _>>()?,
    };
    let result = call_module_function_in_execution(
        program,
        name,
        args,
        jit,
        linked,
        host,
        &mut module_state,
    );
    for (i, value) in module_state.slots.into_iter().enumerate() {
        reg_store(stack, program.module_base as usize + i, value)?;
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn call_module_function_in_execution(
    program: &Program,
    name: &str,
    args: Vec<BslValue>,
    jit: JitMode,
    linked: &LinkedComponents<'_>,
    host: &mut HostIo<'_, '_>,
    module_state: &mut ModuleState,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let _depth = DynamicDepthGuard::enter(host.dynamic_depth)?;

    let upper = name.to_uppercase();
    let index = program
        .function_names
        .iter()
        .position(|n| n.to_uppercase() == upper)
        .ok_or_else(|| {
            RtError::DynamicError(format!(
                "Процедура или функция «{name}» в модуле не найдена"
            ))
        })?;
    // `function_names[i]` — это `chunks[i + 1]`: нулевой чанк занят
    // операторами верхнего уровня.
    let func_id = index + 1;
    let chunk = at(&program.chunks, func_id, "номер чанка вне таблицы функций")?;
    let n_params = chunk.n_params as usize;
    if args.len() != n_params {
        return Err(RtError::DynamicError(format!(
            "Неверное число аргументов при вызове «{name}»: передано {}, а параметров {n_params}",
            args.len()
        )));
    }

    // Кадр вызванной функции строится так же, как его строит `drive`:
    // параметры — слоты `0..n_params`, собственные регистры сразу за ними.
    // Алиасов на слоты вызывающего у этого кадра нет (`drive` заводит его с
    // пустым `Frame::param_aliases`), поэтому аргументы просто лежат
    // значениями в начале стека.
    let mut call_stack = args;
    push_own_registers(&mut call_stack, chunk);
    let (value, final_stack) = drive_linked(
        program,
        func_id,
        call_stack,
        jit,
        linked,
        host,
        module_state,
    )?;

    // Финальные значения слотов параметров: верхний кадр при возврате стек
    // не усекает (см. `do_return_with_value`), поэтому они всё ещё на
    // месте — в слотах `0..n_params`.
    let mut final_params = Vec::with_capacity(n_params);
    for i in 0..n_params {
        final_params.push(reg_load(&final_stack, i)?);
    }
    Ok((value, final_params))
}

fn remap_chunk_libraries(
    chunk: &mut bsl_bytecode::Chunk,
    from: &[bsl_bytecode::LibraryRequirement],
    to: &[bsl_bytecode::LibraryRequirement],
) -> Result<(), RtError> {
    for instruction in &mut chunk.instrs {
        let library = match instruction {
            Instr::CallComponent { library, .. } | Instr::CreateObject { library, .. } => library,
            _ => continue,
        };
        let requirement = from.get(*library as usize).ok_or(RtError::InvalidBytecode(
            "индекс библиотеки вне таблицы requirements",
        ))?;
        let target = to
            .iter()
            .position(|candidate| candidate.package == requirement.package)
            .ok_or(RtError::InvalidBytecode(
                "компонент чанка отсутствует в объединённых requirements",
            ))?;
        *library = target.try_into().map_err(|_| {
            RtError::InvalidBytecode("индекс библиотеки не помещается в операнд u8")
        })?;
    }
    // Единственное место вне `bsl-bytecode`, где разметка считается не
    // через `image::finalize`, и это объявленное исключение, а не
    // недосмотр. Финализация ставит разметку ВСЕЙ программе, а здесь
    // чанки едут во фрагмент по одному: нулевой будет заменён самим
    // фрагментом (его разметка остаётся пустой — поинструкционное
    // исполнение), а у остальных пересечения с модульными слотами нет по
    // определению `module_overlap`, поэтому `None` для них и есть верный
    // ответ.
    chunk.bundle_len = bsl_bytecode::bundle::compute(chunk, None);
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

/// `frames.pop().expect(...)` ниже — ВНУТРЕННИЙ ИНВАРИАНТ VM, а не входные
/// данные (см. классификацию в шапке модуля): `step`/`drive` вызывают эту
/// функцию только пока `frames` не пуст — сам факт того, что мы исполняем
/// инструкцию, это гарантирует. Никакой байт-код, корректный или нет, сюда
/// с пустым стеком кадров не приведёт.
/// Готовит модуль каталога к обращению. `Ok(true)` означает, что кадр
/// тела модуля запушен и текущая инструкция должна исполниться повторно
/// после его возврата; `pc` вызывающего при этом не продвинут.
///
/// # Errors
///
/// Ловимая ошибка при циклической инициализации и при обращении к модулю,
/// чьё тело уже завершилось ошибкой; политика повторного запуска — за
/// замером `JOB.MODULE.INIT`.
fn ensure_module_ready(
    target: u32,
    ctx: &CatalogContext<'_>,
    session: &mut SessionModules,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
) -> Result<bool, RtError> {
    let instance = session
        .instances
        .get_mut(target as usize)
        .ok_or(RtError::InvalidBytecode("связь ведёт мимо сессии модулей"))?;
    match instance.init {
        ModuleInitState::Ready => Ok(false),
        ModuleInitState::NotStarted => {
            if frames.len() >= MAX_CALL_DEPTH {
                return Err(RtError::StackOverflow {
                    what: "слишком глубокая рекурсия вызовов",
                });
            }
            instance.init = ModuleInitState::Initializing;
            let body = ctx.program(target)?;
            let chunk0 = at(&body.chunks, 0, "у модуля каталога нет тела")?;
            let call_start = stack.len();
            let own_base = stack.len();
            push_own_registers(stack, chunk0);
            frames.push(Frame {
                module: target,
                func_id: 0,
                pc: 0,
                param_aliases: Vec::new(),
                own_base,
                call_start,
                return_reg: 0,
                module_copybacks: Vec::new(),
                numeric_for_state: None,
            });
            Ok(true)
        }
        ModuleInitState::Initializing => Err(RtError::DynamicError(
            "циклическая инициализация общего модуля".into(),
        )),
        ModuleInitState::Failed => Err(RtError::DynamicError(
            "инициализация общего модуля завершилась ошибкой".into(),
        )),
    }
}

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
        // `BuiltinFn::host_effect`; по нему же JIT решает, чего не
        // компилировать.
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
/// Отдельной функцией, потому что её зовут ДВОЕ: ветка `step`
/// интерпретатора и шим JIT-а. Второй реализации сложения строк в
/// проекте быть не должно — при расхождении режимов `--jit` и обычного
/// не сработал бы ни один существующий тест.
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
/// [`add_op`]: её зовут и интерпретатор, и шим JIT-а.
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
