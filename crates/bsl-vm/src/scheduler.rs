use super::{CatalogContext, Frame, Program, SessionModules, reg_store, unwind_to_handler};
use bsl_rt::{BslValue, ExecutionToken, PromiseId, RtError};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

static NEXT_EXECUTION_TOKEN: AtomicU64 = AtomicU64::new(1);

pub(super) type TaskId = usize;

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

pub(super) struct Task {
    pub(super) frames: Vec<Frame>,
    pub(super) stack: Vec<BslValue>,
    pub(super) current_exception: Option<BslValue>,
    pub(super) completion: TaskCompletion,
    pub(super) quantum_remaining: usize,
}

#[derive(Clone, Copy)]
pub(super) enum TaskCompletion {
    Root,
    Promise(PromiseId),
    Detached,
}

pub(super) enum PromiseState {
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
pub(super) struct SyncWait {
    task_id: TaskId,
    promise_id: PromiseId,
    dst: u8,
}

struct HostPromise {
    handle: Box<dyn bsl_rt::RequestHandle>,
    mapper: bsl_rt::HttpResponseMapper,
}

pub(super) struct AsyncState {
    pub(super) token: ExecutionToken,
    scheduler_quantum: usize,
    pub(super) tasks: Vec<Option<Task>>,
    pub(super) ready: VecDeque<TaskId>,
    pub(super) promises: Vec<PromiseState>,
    host_promises: Vec<Option<HostPromise>>,
    completion_sender: mpsc::Sender<HostCompletion>,
    completion_receiver: mpsc::Receiver<HostCompletion>,
    pub(super) host_waker: Option<ExecutionWaker>,
    pub(super) sync_wait: Option<SyncWait>,
}

impl AsyncState {
    pub(super) fn new(root: Task, scheduler_quantum: usize) -> Self {
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

    pub(super) fn scheduler_quantum(&self) -> usize {
        self.scheduler_quantum
    }

    pub(super) fn new_promise(&mut self) -> Result<(PromiseId, BslValue), RtError> {
        let raw = u64::try_from(self.promises.len())
            .map_err(|_| RtError::DynamicError("слишком много обещаний в одном запуске".into()))?;
        let id = PromiseId::new(raw);
        self.promises.push(PromiseState::Pending {
            waiters: VecDeque::new(),
        });
        Ok((id, BslValue::new_promise(self.token, id)))
    }

    pub(super) fn insert_task(&mut self, task: Task) -> TaskId {
        let id = self.tasks.len();
        self.tasks.push(Some(task));
        id
    }

    pub(super) fn resolve_promise(
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

    pub(super) fn has_live_tasks(&self) -> bool {
        self.tasks.iter().any(Option::is_some)
    }

    pub(super) fn has_other_live_task(&self) -> bool {
        // Текущая задача вынута из `tasks` на время исполнения, поэтому
        // любой оставшийся `Some` означает настоящего конкурента за FIFO.
        self.tasks.iter().any(Option::is_some)
    }

    pub(super) fn has_pending_host_promises(&self) -> bool {
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

    pub(super) fn drain_completions(
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
    pub(super) fn begin_sync_host_call(
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

/// Изъятие пробуждённой замороженной задачи из готовых: при активной
/// парковке синхронного вызова исполняется только она сама, остальные
/// готовые ждут в очереди. Вынесено из `poll_linked` ради укладки его
/// горячего цикла.
#[inline(never)]
pub(super) fn take_frozen_ready(async_state: &mut AsyncState) -> Option<TaskId> {
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
pub(super) fn resume_parked_task(
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

#[derive(Clone, Copy)]
pub(super) struct TaskPosition {
    frame_depth: usize,
    func_id: usize,
    pc: usize,
}

pub(super) fn task_position(task: &Task) -> TaskPosition {
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

pub(super) fn crossed_scheduler_safe_point(before: TaskPosition, task: &Task) -> bool {
    let after = task_position(task);
    before.frame_depth != after.frame_depth
        || before.func_id != after.func_id
        || after.pc <= before.pc
}

pub(super) fn consume_scheduler_safe_point(
    task: &mut Task,
    scheduled: bool,
    quantum: usize,
) -> bool {
    if !scheduled {
        task.quantum_remaining = quantum;
        return false;
    }
    task.quantum_remaining -= 1;
    task.quantum_remaining == 0
}
