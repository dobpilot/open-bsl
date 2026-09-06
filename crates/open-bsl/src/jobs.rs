//! Нативный runtime фоновых заданий: конфигурация, реестр, admission.
//!
//! Архитектурные инварианты (план фоновых заданий,
//! `docs/archive/plans/background-jobs.md`):
//! реестр хранит только владеющие `Send`-DTO и никогда не вызывает внешний
//! код под своим локом; мьютекс — синхронный `std::sync::Mutex` с короткими
//! секциями и не пересекает `await`; занятый пул означает FIFO, а не
//! ошибку; первый terminal transition выигрывает ровно один раз.

mod prepare;
mod registry;

#[cfg(test)]
pub(crate) use prepare::resolve_target;
pub(crate) use prepare::runtime_for_engine;
use prepare::{TargetTable, WorkerRecipe, build_worker_engine, prepare_job};
mod runtime;

use runtime::JobRuntimeShared;
pub(crate) use runtime::random_uuid;
pub use runtime::{
    BackgroundJobConfig, BackgroundStateFactory, HostProfileId, JobIdSource, JobRuntime,
    JobTimeSource, ShutdownReport,
};

use registry::{JobRegistry, RuntimeState, fail_resident_jobs};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::{
    HostError, JobErrorDto, JobId, JobSnapshotDto, JobStateDto, SerializedValueGraph,
    UserMessageDto,
};

// --- Пул workers -------------------------------------------------------

#[cfg(test)]
use std::sync::{Condvar, Mutex};

/// Главный цикл worker: резиденты локальной FIFO чередуются бюджетными
/// квантами, задание с припаркованным host-вызовом (`Waiting`) не мешает
/// соседям, а когда все резиденты ждут — worker спит точно, до события
/// (`wake_epoch`, очередь, отмена, закрытие), без таймерного поллинга.
fn worker_main(shared: &Arc<JobRuntimeShared>, progress: &std::sync::atomic::AtomicBool) {
    // Каталог разбирается один раз на worker; программы разделяются между
    // сеансами этого worker и не покидают его поток.
    let engine = match build_worker_engine(&shared.recipe) {
        Ok(engine) => engine,
        Err(error) => {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Broken;
            let text = format!("worker не разобрал рецепт каталога: {error}");
            fail_resident_jobs(&mut registry, &text);
            drop(registry);
            shared.terminal_watch.notify_all();
            return;
        }
    };
    // Резиденты живут в пуле ПОТОКА, а не в кадре: вложенное ожидание
    // внутри кванта резидента обслуживает тот же пул и потому способно
    // доводить своих соседей (см. `drive_local`).
    drive_local(shared, &engine, DriveMode::Worker(progress));
}

thread_local! {
    /// Резиденты ЭТОГО потока. Пул общий для цикла worker и всех
    /// вложенных helping-драйверов: задание, припаркованное на
    /// host-операции, закреплено за своим потоком, и довести его может
    /// ТОЛЬКО этот поток. Пока набор жил в кадре `worker_main`, вложенное
    /// ожидание соседа-резидента (родитель ждёт ребёнка, который сам
    /// припаркован на HTTP этого же worker) не имело к нему доступа и
    /// парковалось навсегда.
    static RESIDENTS: RefCell<VecDeque<RunningJob>> = const { RefCell::new(VecDeque::new()) };
}

/// Забирает резидентов потока — путь очистки после паники worker:
/// `Drop`-гарды переводят их в `Failed`, вечного `Running` не остаётся.
fn take_thread_residents() -> VecDeque<RunningJob> {
    RESIDENTS.with(|residents| std::mem::take(&mut *residents.borrow_mut()))
}

/// Режим общего цикла резидентов.
enum DriveMode<'a> {
    /// Цикл worker: живёт до закрытия runtime, отмечает прогресс для
    /// супервизора.
    Worker(&'a std::sync::atomic::AtomicBool),
    /// Helping-ожидание: выход по собственному предикату либо дедлайну.
    /// Ожидающий поток не простаивает — он двигает пул своего потока и
    /// подбирает задания из глобальной очереди.
    Until {
        /// Наблюдаемые задания — для тестового шлюза окна парковки.
        ids: &'a [JobId],
        /// Предикат завершения, вычисляемый ПОД локом реестра: там же,
        /// где принимается решение спать, — потерянных пробуждений нет.
        done: &'a dyn Fn(&JobRegistry) -> bool,
        deadline: Option<std::time::Instant>,
    },
}

/// Общий цикл резидентов worker и helping-ожиданий. Локальная FIFO
/// потока: runnable чередуются бюджетными квантами, waiting опрашиваются
/// вместе с ними (host-completions подбирает их собственный poll), новое
/// глобальное задание извлекается, только когда локально нет
/// runnable-резидентов, — по плану фоновых заданий.
///
/// В режиме `Until` тот же цикл обслуживает вложенное ожидание: пока цель
/// не достигнута, поток двигает СВОИХ резидентов (включая припаркованных
/// на host-операциях соседей — иначе их некому довести) и помогает
/// глобальной очереди, а спит только когда двигать нечего.
fn drive_local(shared: &Arc<JobRuntimeShared>, engine: &crate::Engine, mode: DriveMode<'_>) {
    let mark_progress = || {
        if let DriveMode::Worker(progress) = &mode {
            progress.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    };
    let deadline = match &mode {
        DriveMode::Until { deadline, .. } => *deadline,
        DriveMode::Worker(_) => None,
    };
    // Снимок wake_epoch на момент последнего начала опроса ждущих
    // резидентов: спим, только если с тех пор не было ни одного
    // host-события. Событие между опросом и сном поднимает счётчик под
    // локом — пропущенных пробуждений нет.
    let mut seen_epoch = 0u64;
    loop {
        // Тестовое окно «предикат ещё не перечитан»: держится БЕЗ лока
        // реестра, иначе завершающий задание тест встал бы на нём сам.
        helping_window_pause(&mode);
        let next_global = {
            let mut registry = shared.registry.lock().expect("реестр без отравления");
            loop {
                let closed = matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken);
                match &mode {
                    DriveMode::Until { ids, done, .. } => {
                        // Цель и недоступность runtime проверяются ПОД тем
                        // же локом, под которым принимается решение спать.
                        if closed || done(&registry) {
                            return;
                        }
                        let _ = ids;
                    }
                    DriveMode::Worker(_) => {
                        if closed {
                            // Закрытие на границе кванта — и есть
                            // кооперативная точка: резиденты завершаются
                            // «Отменено». Сначала terminal transitions под
                            // локом, а сами резиденты дропаются ПОСЛЕ его
                            // отпускания: Drop-гард берёт этот же мьютекс.
                            drop(registry);
                            let end = shared.time_source.wall_now();
                            let residents = take_thread_residents();
                            {
                                let mut registry =
                                    shared.registry.lock().expect("реестр без отравления");
                                for job in residents.iter() {
                                    registry.finish(job.id, JobStateDto::Canceled, end, None);
                                }
                            }
                            drop(residents);
                            shared.terminal_watch.notify_all();
                            return;
                        }
                    }
                }
                if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
                    return;
                }
                let (has_residents, runnable_locally) = RESIDENTS.with(|residents| {
                    let residents = residents.borrow();
                    (
                        !residents.is_empty(),
                        residents.iter().any(|job| !job.waiting),
                    )
                });
                if runnable_locally {
                    // Есть чем заняться — глобальную очередь не трогаем.
                    break None;
                }
                if let Some(id) = registry.queue.pop_front() {
                    break Some(id);
                }
                // Двигать нечего: сон до события. В режиме worker это
                // `work_available` (новая работа и host-события), в
                // helping-ожидании — `terminal_watch`, который будят и
                // terminal-переходы чужих заданий, и host-completions, и
                // отмена, и закрытие.
                if has_residents && registry.wake_epoch != seen_epoch {
                    seen_epoch = registry.wake_epoch;
                    // Неизвестно, чьё завершение пришло: каждый ждущий
                    // резидент опрашивается заново (его собственный poll
                    // подберёт доставленные завершения либо снова уснёт).
                    RESIDENTS.with(|residents| {
                        for job in residents.borrow_mut().iter_mut() {
                            job.waiting = false;
                        }
                    });
                    break None;
                }
                let condvar = match &mode {
                    DriveMode::Worker(_) => &shared.work_available,
                    DriveMode::Until { .. } => &shared.terminal_watch,
                };
                HELPING_PARKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                match deadline {
                    None => {
                        registry = condvar.wait(registry).expect("реестр без отравления");
                    }
                    Some(deadline) => {
                        let left = deadline.saturating_duration_since(std::time::Instant::now());
                        let (guard, _) = condvar
                            .wait_timeout(registry, left)
                            .expect("реестр без отравления");
                        registry = guard;
                    }
                }
            }
        };
        if let Some(id) = next_global {
            match start_job(shared, engine, id) {
                None => continue,
                Some(Ok(job)) => RESIDENTS.with(|residents| residents.borrow_mut().push_back(job)),
                Some(Err(error)) => {
                    finish_job(shared, id, JobStateDto::Failed, Some(error));
                    // Отказ подготовки — тоже доведённое до terminal
                    // задание: супервизор обязан считать это прогрессом.
                    mark_progress();
                    continue;
                }
            }
        }
        // Резидент ИЗВЛЕКАЕТСЯ из пула на время кванта: вложенное
        // ожидание внутри этого кванта увидит только соседей и не станет
        // опрашивать задание, чей квант уже на стеке.
        let Some(mut job) = RESIDENTS.with(|residents| {
            let mut residents = residents.borrow_mut();
            let index = residents.iter().position(|job| !job.waiting)?;
            residents.remove(index)
        }) else {
            continue;
        };
        // Паника BSL-исполнения ловится на границе кванта и роняет только
        // это задание; соседи-резиденты продолжают.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job.poll(engine)));
        match outcome {
            Ok(Ok(bsl_vm::ProgramPoll::Complete(..))) => {
                // Право на terminal transition забирается ДО публикации:
                // поломка runtime, выигравшая гонку, оставляет задание
                // `Failed` БЕЗ публикации (rollback по матрице), а
                // выигравший драйвер публикует и завершает сам.
                commit_window_pause(job.id);
                if claim_terminal(shared, job.id) {
                    if commit_staged(shared, &job) {
                        finish_job(shared, job.id, JobStateDto::Completed, None);
                    } else {
                        // Успешный job при закрытом сеансе вызывателя — Failed:
                        // частичной публикации нет (план, JOB.TEMP.CALLER_CLOSE_RACE).
                        finish_job(
                            shared,
                            job.id,
                            JobStateDto::Failed,
                            Some(JobErrorDto::from_text(
                                "сеанс-получатель временного хранилища закрыт",
                            )),
                        );
                    }
                }
                mark_progress();
            }
            Ok(Ok(bsl_vm::ProgramPoll::Runnable)) => {
                job.waiting = false;
                RESIDENTS.with(|residents| residents.borrow_mut().push_back(job));
            }
            Ok(Ok(bsl_vm::ProgramPoll::Waiting)) => {
                job.waiting = true;
                RESIDENTS.with(|residents| residents.borrow_mut().push_back(job));
            }
            Ok(Err(JobPollError::Canceled)) => {
                finish_job(shared, job.id, JobStateDto::Canceled, None);
                mark_progress();
            }
            Ok(Err(JobPollError::Failed(error))) => {
                // Неперехваченная BSL-ошибка ПУБЛИКУЕТ write-set — измерено
                // JOB.TEMP.FAILURE; неудача публикации остаётся вторичной
                // причиной в cause-цепочке, основная ошибка BSL важнее.
                // Публикация — только с забранным claim (см. выше).
                if claim_terminal(shared, job.id) {
                    let error = if commit_staged(shared, &job) {
                        error
                    } else {
                        with_commit_failure_cause(error)
                    };
                    finish_job(shared, job.id, JobStateDto::Failed, Some(error));
                }
                mark_progress();
            }
            Err(_) => {
                finish_job(
                    shared,
                    job.id,
                    JobStateDto::Failed,
                    Some(JobErrorDto::from_text(
                        "исполнение задания прервано паникой",
                    )),
                );
                mark_progress();
            }
        }
    }
}

/// Маршрут сообщений задания: неблокирующий sink его сеанса. Сначала DTO
/// добавляется к записи реестра под коротким локом (бюджет
/// `max_message_bytes_per_job`), затем — уже без лока — уходит внешнему
/// sink представления, если host его зарегистрировал. Backpressure
/// внешнего sink не отменяет уже записанную историю и не повторяется
/// скрыто: `Сообщить()` возвращает ловимую ошибку, повторный вызов
/// создаёт новое сообщение.
struct JobMessageRoute {
    shared: Arc<JobRuntimeShared>,
    id: JobId,
}

impl bsl_rt::UserMessageSink for JobMessageRoute {
    fn enqueue(&self, message: &UserMessageDto) -> Result<(), HostError> {
        {
            let mut registry = self.shared.registry.lock().expect("реестр без отравления");
            registry.push_message(self.id, message.clone())?;
        }
        if let Some(display) = &self.shared.message_display {
            display.enqueue(message)?;
        }
        Ok(())
    }

    /// Остаток кумулятивного бюджета `max_message_bytes_per_job` этой
    /// записи: им ограничивается сериализация `КлючДанных` и
    /// `ИдентификаторНазначения` ДО крупной аллокации. Для уже
    /// terminal-записи предела нет — `enqueue` всё равно ответит
    /// `JobExpired`, и подменять его ошибкой бюджета нельзя.
    fn message_bytes_left(&self) -> Option<usize> {
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        let record = registry.record(self.id)?;
        Some(
            self.shared
                .config
                .max_message_bytes_per_job
                .saturating_sub(record.message_bytes),
        )
    }
}

/// Резидент worker: изолированный сеанс одного задания с pollable
/// VM-прогоном. Начатый резидент закреплён за своим worker — `Rc`-графы
/// его сеанса поток не покидают.
///
/// Drop-гард: резидент, дропнутый БЕЗ terminal transition — разматывание
/// паники worker, замена worker супервизором, — завершает свою запись
/// `Failed`, будит ожидающих и не оставляет задание вечным `Running`.
/// Host-handles отменяет дроп VM (`AsyncState::drop`), staging
/// откатывается дропом сеанса, его кредиты возвращает `StagingBudget`.
/// Штатные пути уже сделали terminal transition к моменту дропа, и гард
/// для них — no-op. Ни один дроп резидента не происходит под локом
/// реестра — гард берёт его сам.
struct RunningJob {
    id: JobId,
    shared: Arc<JobRuntimeShared>,
    state: crate::State,
    module: crate::Module,
    vm: bsl_vm::ProgramExecution,
    /// Последний poll вернул `Waiting`: задание ждёт host-completion и
    /// runnable-резидентом не считается.
    waiting: bool,
}

impl Drop for RunningJob {
    fn drop(&mut self) {
        let end = self.shared.time_source.wall_now();
        // `lock()` без `expect`: гард срабатывает и при разматывании
        // паники — отравленный мьютекс здесь не повод для двойной паники.
        let Ok(mut registry) = self.shared.registry.lock() else {
            return;
        };
        let finished = registry.finish(
            self.id,
            JobStateDto::Failed,
            end,
            Some(JobErrorDto::from_text(
                "исполнение задания прервано сбоем worker",
            )),
        );
        drop(registry);
        if finished {
            self.shared.terminal_watch.notify_all();
        }
    }
}

/// Гард окна запуска: между `Queued -> Running` и передачей записи под
/// Drop-гард готового резидента задание не должно застрять вечным
/// `Running`, если подготовка сеанса паникует (паника worker вне границы
/// задания). Взведённый гард завершает запись `Failed`; штатные выходы
/// его разряжают.
struct StartGuard<'a> {
    shared: &'a Arc<JobRuntimeShared>,
    id: JobId,
    armed: bool,
}

impl Drop for StartGuard<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let end = self.shared.time_source.wall_now();
        let Ok(mut registry) = self.shared.registry.lock() else {
            return;
        };
        let finished = registry.finish(
            self.id,
            JobStateDto::Failed,
            end,
            Some(JobErrorDto::from_text(
                "запуск задания прерван сбоем worker",
            )),
        );
        drop(registry);
        if finished {
            self.shared.terminal_watch.notify_all();
        }
    }
}

/// Стартует резидента: `Queued -> Running`, entry-программа цели,
/// изолированный сеанс, pollable-прогон с постоянным квантованием.
/// `None` — запись уже terminal (например, отменена до старта).
fn start_job(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    id: JobId,
) -> Option<Result<RunningJob, JobErrorDto>> {
    let mut guard = StartGuard {
        shared,
        id,
        armed: true,
    };
    let begin = shared.time_source.wall_now();
    // `Queued -> Running` — ПЕРВОЕ изменение статуса по семантике
    // менеджерного `ОжидатьЗавершенияВыполнения`: ожидающих будим.
    let mut started = false;
    let taken = {
        let mut registry = shared.registry.lock().expect("реестр без отравления");
        let record = registry.record_mut(id);
        match record {
            None => None,
            Some(record) => {
                record.snapshot.state = JobStateDto::Running;
                record.snapshot.begin = begin;
                started = true;
                Some((
                    record.target,
                    Arc::clone(&record.snapshot.params),
                    Arc::clone(&record.cancel_requested),
                    record.caller_token,
                    record.profile_index,
                ))
            }
        }
    };
    if started {
        shared.terminal_watch.notify_all();
    }
    let Some((target, params, cancel_requested, caller_token, profile_index)) = taken else {
        guard.armed = false;
        return None;
    };
    let prepared = prepare_job(
        shared,
        engine,
        id,
        target,
        &params,
        caller_token,
        profile_index,
    );
    guard.armed = false;
    Some(prepared.map(|(state, module, mut vm)| {
        vm.set_cancel_flag(cancel_requested);
        // Пробуждение из потока транспорта: поднять wake_epoch под
        // локом и разбудить пул. Сон worker сверяет счётчик со
        // своим снимком, поэтому пробуждение не теряется, даже
        // если пришло между опросом резидентов и засыпанием.
        let waker_shared = Arc::clone(shared);
        vm.set_host_waker(std::sync::Arc::new(move || {
            let mut registry = waker_shared.registry.lock().expect("реестр без отравления");
            registry.wake_epoch += 1;
            drop(registry);
            waker_shared.work_available.notify_all();
            // Helping-ожидание спит на `terminal_watch`: завершение
            // host-операции — его событие тоже (появился резидент,
            // которого стало можно двигать).
            waker_shared.terminal_watch.notify_all();
        }));
        RunningJob {
            id,
            shared: Arc::clone(shared),
            state,
            module,
            vm,
            waiting: false,
        }
    }))
}

/// Исход неудачного кванта: отмена — не ошибка BSL, снимок получает
/// состояние «Отменено» без `ИнформацияОбОшибке`.
enum JobPollError {
    Canceled,
    Failed(JobErrorDto),
}

impl RunningJob {
    /// Один бюджетный квант задания. Поток не блокируется: completions
    /// подбираются конечным срезом без ожидания первого.
    fn poll(&mut self, engine: &crate::Engine) -> Result<bsl_vm::ProgramPoll, JobPollError> {
        let catalog = engine
            .catalog()
            .expect("worker строится только с каталогом");
        self.vm
            .poll_configuration_with_budget(
                &self.module.program,
                catalog,
                engine.registry(),
                &mut self.state.host.stdout,
                &mut self.state.host.stderr,
                &mut self.state.dynamic,
                &mut self.state.host.env,
                1024,
                Some(1),
            )
            .map_err(|error| match error {
                bsl_rt::RtError::Canceled => JobPollError::Canceled,
                other => JobPollError::Failed(JobErrorDto::from_text(other.to_string())),
            })
    }
}

/// Публикация write-set задания по измеренной terminal-матрице: commit
/// после успеха И неперехваченной BSL-ошибки (`JOB.TEMP.FAILURE`),
/// rollback — просто дроп staging — после отмены (`JOB.TEMP.CANCEL`),
/// паники и инфраструктурных сбоев. `false` — сеанс вызывателя уже
/// закрыт и публикация не состоялась.
// НЕ ИЗМЕРЕНО(JOB.TEMP.CALLER_CLOSE_RACE): гонка закрытия сеанса
// вызывателя с terminal commit на платформе не замерена; выбрано «без
// частичной публикации»: успешный BSL-job при закрытом получателе
// становится Failed, что закреплено тестом
// `a_closed_caller_session_fails_the_publishing_job`.
fn commit_staged(shared: &Arc<JobRuntimeShared>, job: &RunningJob) -> bool {
    let Some(session) = job.state.host.env.temp_storage() else {
        return true;
    };
    let mut session = session.borrow_mut();
    let Some(caller) = session.caller() else {
        return true;
    };
    let writes = session.take_staged();
    if writes.is_empty() {
        return true;
    }
    shared.temp_hub.commit(caller, writes)
}

/// Пристраивает отказ terminal commit в конец cause-цепочки ошибки:
/// при неперехваченной BSL-ошибке и закрытом сеансе-получателе основная
/// ошибка остаётся BSL-ошибкой, а закрытый mailbox виден вторичной
/// причиной, не подменяя её (измеренный порядок публикаций —
/// `JOB.TEMP.FAILURE`, гонка закрытия — `JOB.TEMP.CALLER_CLOSE_RACE`).
fn with_commit_failure_cause(mut error: JobErrorDto) -> JobErrorDto {
    let mut tail = &mut error;
    while tail.cause.is_some() {
        tail = tail.cause.as_mut().expect("проверено условием цикла");
    }
    tail.cause = Some(Box::new(JobErrorDto::from_text(
        "сеанс-получатель временного хранилища закрыт",
    )));
    error
}

/// Тестовый шлюз окна «задание завершилось, право ещё не забрано»:
/// проба закрывает runtime ровно здесь и проверяет, что публикация не
/// состоялась (`a_shutdown_during_the_commit_window_rolls_back`).
#[cfg(test)]
struct CommitWindowGate {
    target: JobId,
    held: Mutex<bool>,
    released: Condvar,
    entered: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
static COMMIT_WINDOW_GATE: Mutex<Option<Arc<CommitWindowGate>>> = Mutex::new(None);

/// Пауза перед взятием claim. Вне тестов и без шлюза — пустышка.
fn commit_window_pause(id: JobId) {
    #[cfg(not(test))]
    let _ = id;
    #[cfg(test)]
    {
        let gate = COMMIT_WINDOW_GATE
            .lock()
            .expect("шлюз без отравления")
            .clone();
        let Some(gate) = gate else {
            return;
        };
        if gate.target != id {
            return;
        }
        gate.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut held = gate.held.lock().expect("шлюз без отравления");
        while *held {
            held = gate.released.wait(held).expect("шлюз без отравления");
        }
    }
}

/// Забирает право на terminal transition под локом реестра. Отказ при
/// закрытом runtime завершает запись «Отменено» прямо там же, поэтому
/// ожидающих будим в любом случае.
fn claim_terminal(shared: &Arc<JobRuntimeShared>, id: JobId) -> bool {
    let end = shared.time_source.wall_now();
    let granted = shared
        .registry
        .lock()
        .expect("реестр без отравления")
        .claim_terminal(id, end);
    if !granted {
        shared.terminal_watch.notify_all();
    }
    granted
}

/// Terminal transition резидента с пробуждением ожидающих.
fn finish_job(
    shared: &Arc<JobRuntimeShared>,
    id: JobId,
    state: JobStateDto,
    error: Option<JobErrorDto>,
) {
    let end = shared.time_source.wall_now();
    let mut registry = shared.registry.lock().expect("реестр без отравления");
    registry.finish(id, state, end, error);
    drop(registry);
    shared.terminal_watch.notify_all();
}

/// Счётчик парковок helping-ожидания первого изменения — пробник для
/// теста `wait_first_change_parks_without_periodic_wakeups`: ожидание
/// спит на `terminal_watch` до события, и за сотни миллисекунд тихого
/// ожидания счётчик растёт на единицы, а не на сотни таймерных тиков.
/// Счётчик парковок helping-ожидания — пробник для теста
/// `wait_first_change_parks_without_periodic_wakeups`: ожидание спит до
/// события, и за сотни миллисекунд тихого ожидания счётчик растёт на
/// единицы, а не на сотни таймерных тиков.
static HELPING_PARKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Тестовый шлюз окна helping-ожидания: удерживает поток МЕЖДУ проверкой
/// предиката и парковкой — ровно там, где terminal-уведомление терялось
/// бы без повторной проверки под guard. Тест завершает задание в этом
/// окне и убеждается, что ожидание возвращается сразу, а не спит до
/// дедлайна (`a_terminal_event_in_the_helping_window_is_not_lost`).
#[cfg(test)]
struct HelpingWindowGate {
    /// Пауза срабатывает только для ожиданий, среди целей которых это
    /// задание, — параллельные тесты с другими заданиями не задеваются.
    target: JobId,
    held: Mutex<bool>,
    released: Condvar,
    entered: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
static HELPING_WINDOW_GATE: Mutex<Option<Arc<HelpingWindowGate>>> = Mutex::new(None);

/// Пауза в окне «предикат проверен, парковка ещё не началась». Вне
/// тестов и без выставленного шлюза — пустышка. Держится БЕЗ лока
/// реестра: вызывается перед взятием guard парковки.
fn helping_window_pause(mode: &DriveMode<'_>) {
    #[cfg(not(test))]
    let _ = mode;
    #[cfg(test)]
    {
        let DriveMode::Until { ids, .. } = mode else {
            return;
        };
        let gate = HELPING_WINDOW_GATE
            .lock()
            .expect("шлюз без отравления")
            .clone();
        let Some(gate) = gate else {
            return;
        };
        if !ids.contains(&gate.target) {
            return;
        }
        gate.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut held = gate.held.lock().expect("шлюз без отравления");
        while *held {
            held = gate.released.wait(held).expect("шлюз без отравления");
        }
    }
}

/// Сервис worker-сеансов: тот же общий реестр, но ожидание не блокирует
/// пул — поток-родитель ПОМОГАЕТ: пока свои задания не terminal, он
/// исполняет чужие из глобальной FIFO. Это разблокирует вложенное
/// ожидание при полностью занятом пуле (два родителя, ждущие детей,
/// доводят их сами); helping не меняет ABI и наблюдаемой семантики
/// ожидания.
pub(crate) struct WorkerJobService {
    pub shared: Arc<JobRuntimeShared>,
    /// Клон worker-движка: живёт только в потоке этого worker.
    pub engine: crate::Engine,
    /// Token хранилища СВОЕГО job-сеанса: дочерние задания публикуются в
    /// него, а не в сеанс исходного foreground — транзитивного повышения
    /// capability нет.
    // НЕ ИЗМЕРЕНО(JOB.TEMP.NESTED_CAPABILITY): может ли дочерний job
    // платформы писать по адресу исходного foreground-сеанса, не замерено;
    // выбрана capability только непосредственного родителя — теснее
    // некуда, расширить после замера легче, чем сузить.
    pub session_token: [u8; 16],
    /// Host-профиль этого job-сеанса: дочерние задания наследуют его —
    /// другого профиля сервису сеанса не передать.
    pub profile_index: u32,
}

impl bsl_rt::BackgroundJobService for WorkerJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, HostError> {
        self.shared
            .submit_by_name_shared(
                method_name,
                params,
                key,
                description,
                Some(self.session_token),
                self.profile_index,
            )
            .map(Arc::new)
    }

    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshot(id)
    }

    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshots()
    }

    fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        {
            let registry = self.shared.registry.lock().expect("реестр без отравления");
            JobRuntimeShared::live_guard(&registry)?;
            for id in ids {
                JobRuntimeShared::known_guard(&registry, *id)?;
            }
        }
        // Ожидание — драйвер пула СВОЕГО потока: он двигает и соседей
        // (в том числе припаркованных на host-операциях, довести которых
        // может только этот поток), и глобальную очередь, а спит лишь
        // когда двигать нечего. Предикат вычисляется под тем же локом,
        // под которым принимается решение спать.
        let done = |registry: &JobRegistry| JobRuntimeShared::all_terminal(registry, ids);
        drive_local(
            &self.shared,
            &self.engine,
            DriveMode::Until {
                ids,
                done: &done,
                deadline,
            },
        );
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        // Закрытие runtime во время ожидания — ловимая ошибка, а не
        // молчаливый таймаут.
        JobRuntimeShared::live_guard(&registry)?;
        Ok(bsl_rt::JobWaitOutcome {
            completed: JobRuntimeShared::all_terminal(&registry, ids),
            snapshots: JobRuntimeShared::held_snapshots(&registry, ids)?,
        })
    }

    fn cancel(&self, id: JobId) -> Result<(), HostError> {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end)
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.shared.take_messages(id, remove)
    }

    fn graph_limits(&self) -> bsl_rt::GraphLimits {
        graph_limits_for(&self.shared.config)
    }

    fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        // Worker не блокируется впустую: ожидание двигает пул своего
        // потока и глобальную очередь (см. `drive_local`).
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let ids: Vec<JobId> = jobs.iter().map(|(id, _)| *id).collect();
        {
            let registry = self.shared.registry.lock().expect("реестр без отравления");
            JobRuntimeShared::live_guard(&registry)?;
            for id in &ids {
                JobRuntimeShared::known_guard(&registry, *id)?;
            }
        }
        let done = |registry: &JobRegistry| {
            // Вытеснение во время ожидания прекращает сон: внешняя
            // проверка ниже вернёт по нему ловимую `JobExpired`.
            let Ok(held) = JobRuntimeShared::held_snapshots(registry, &ids) else {
                return true;
            };
            let (any_active, any_changed, all_terminal, any_failed) =
                JobRuntimeShared::first_change_flags(jobs, &held);
            if !any_active {
                return true;
            }
            match deadline {
                Some(_) => any_changed,
                None => all_terminal || any_failed,
            }
        };
        drive_local(
            &self.shared,
            &self.engine,
            DriveMode::Until {
                ids: &ids,
                done: &done,
                deadline,
            },
        );
        let registry = self.shared.registry.lock().expect("реестр без отравления");
        JobRuntimeShared::live_guard(&registry)?;
        JobRuntimeShared::held_snapshots(&registry, &ids)
    }
}

/// Мост сервиса: `Rc`-обёртка над разделяемым runtime — то, что native
/// внедряет в `HostEnv` каждого сеанса движка с каталогом.
pub(crate) struct EngineJobService {
    pub runtime: Arc<JobRuntime>,
    /// Token временного хранилища сеанса-вызывателя: задания публикуют
    /// write-set'ы в его mailbox.
    pub caller_token: [u8; 16],
    /// Host-профиль, выбранный `StateBuilder::host_profile`: задания
    /// этого сеанса и их потомки строят host-окружение по нему.
    pub profile_index: u32,
}

impl bsl_rt::BackgroundJobService for EngineJobService {
    fn submit(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<bsl_rt::JobKeyDto>>,
        description: Option<String>,
    ) -> Result<Arc<JobSnapshotDto>, HostError> {
        self.runtime
            .submit_by_name_with_caller(
                method_name,
                params,
                key,
                description,
                Some(self.caller_token),
                self.profile_index,
            )
            .map(Arc::new)
    }

    fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.runtime.snapshot(id)
    }

    fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.runtime.snapshots()
    }

    fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        self.runtime.shared.wait_terminal_blocking(ids, timeout)
    }

    fn cancel(&self, id: JobId) -> Result<(), HostError> {
        self.runtime.cancel(id)
    }

    fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.runtime.shared.take_messages(id, remove)
    }

    fn graph_limits(&self) -> bsl_rt::GraphLimits {
        graph_limits_for(&self.runtime.shared.config)
    }

    fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        self.runtime.shared.wait_first_change(jobs, timeout)
    }
}

/// Бюджет сериализации графов параметров и ключа: admission всё равно
/// отвергнет запись больше `max_single_job_record_bytes`, поэтому
/// сериализатор останавливается на нём до крупной аллокации.
fn graph_limits_for(config: &BackgroundJobConfig) -> bsl_rt::GraphLimits {
    let default_limit = bsl_rt::GraphLimits::default().max_bytes;
    bsl_rt::GraphLimits {
        max_bytes: config.max_single_job_record_bytes.min(default_limit),
    }
}
