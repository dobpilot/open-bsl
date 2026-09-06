use super::prepare::{build_worker_engine, prepare_job};
use super::registry::{JobRegistry, RuntimeState, fail_resident_jobs};
use super::runtime::JobRuntimeShared;
use bsl_rt::{JobErrorDto, JobId, JobStateDto};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Condvar, Mutex};

/// Главный цикл worker: резиденты локальной FIFO чередуются бюджетными
/// квантами, задание с припаркованным host-вызовом (`Waiting`) не мешает
/// соседям, а когда все резиденты ждут — worker спит точно, до события
/// (`wake_epoch`, очередь, отмена, закрытие), без таймерного поллинга.
pub(super) fn worker_main(
    shared: &Arc<JobRuntimeShared>,
    progress: &std::sync::atomic::AtomicBool,
) {
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
pub(super) fn take_thread_residents() -> VecDeque<RunningJob> {
    RESIDENTS.with(|residents| std::mem::take(&mut *residents.borrow_mut()))
}

/// Режим общего цикла резидентов.
pub(super) enum DriveMode<'a> {
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
pub(super) fn drive_local(
    shared: &Arc<JobRuntimeShared>,
    engine: &crate::Engine,
    mode: DriveMode<'_>,
) {
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
pub(super) struct RunningJob {
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{engine, number, params};
    use super::super::*;
    use super::*;

    /// Закрытие runtime в окне «задание завершилось, право на terminal
    /// ещё не забрано» ОТКАТЫВАЕТ публикацию: claim не выдаётся, запись
    /// становится «Отменено», а данные в сеанс-получатель не попадают —
    /// как требует строка «shutdown — rollback» матрицы публикаций.
    #[test]
    fn a_shutdown_during_the_commit_window_rolls_back() {
        let engine = crate::Engine::builder()
            .common_module(
                "Служебный",
                "Процедура Пишет(Знач Адрес) Экспорт\n\
                     ПоместитьВоВременноеХранилище(\"из задания\", Адрес);\n\
                 КонецПроцедуры",
            )
            .build()
            .expect("движок с каталогом");
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Живой сеанс-получатель: публикация, если бы она случилась,
        // была бы видна в его mailbox.
        let caller_token = [7u8; 16];
        let caller = std::rc::Rc::new(std::cell::RefCell::new(bsl_rt::TempStorageSession::new(
            caller_token,
            bsl_rt::HostEnv::process().random(),
        )));
        engine
            .temp_hub()
            .register(caller_token, caller.borrow().mailbox());
        let address = format!(
            "e1cib/tempstorage/{}?seanceId={}",
            bsl_rt::uuid::format(&[9u8; 16]),
            bsl_rt::uuid::format(&caller_token)
        );
        let target =
            resolve_target(engine.catalog().unwrap(), "Служебный.Пишет").expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Пишет",
                target,
                params(&[bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&address))]),
                None,
                None,
                Some(caller_token),
                0,
            )
            .expect("задание принято");
        let gate = Arc::new(CommitWindowGate {
            target: snapshot.id,
            held: Mutex::new(true),
            released: Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        *COMMIT_WINDOW_GATE.lock().expect("шлюз без отравления") = Some(Arc::clone(&gate));
        // Ждём, пока задание отработает и встанет в окно перед claim.
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "задание не дошло до окна публикации"
            );
            std::thread::yield_now();
        }
        // Закрытие ИМЕННО в окне: право на terminal больше не выдаётся.
        // Само закрытие идёт в отдельном потоке — оно соединяет workers,
        // а наш worker стоит в шлюзе и выйдет только после его снятия.
        let closing = Arc::clone(&runtime.shared);
        let shutdown = std::thread::spawn(move || {
            let mut registry = closing.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Closed;
            drop(registry);
            closing.work_available.notify_all();
            closing.terminal_watch.notify_all();
        });
        shutdown.join().expect("поток закрытия");
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        let report = runtime.shutdown(Duration::from_secs(30));
        assert_eq!(report.detached_workers, 0);
        *COMMIT_WINDOW_GATE.lock().expect("шлюз без отравления") = None;

        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(
            done.state,
            JobStateDto::Canceled,
            "закрытие обязано откатить задание, а не завершить его успешно"
        );
        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let seen = caller
            .borrow()
            .get(&address, &mut shapes)
            .expect("чтение адреса");
        assert!(
            matches!(seen, bsl_rt::BslValue::Undefined),
            "write-set не имеет права попасть к получателю после закрытия: {seen:?}"
        );
    }

    /// Общая сцена окна helping-ожидания: задание принято без пула,
    /// очередь вычищена, запись переведена в Running; тестовый шлюз
    /// держит ожидание МЕЖДУ проверкой предиката и парковкой, задание
    /// завершается и notify уходит именно в этом окне.
    fn helping_window_scenario(first_change: bool) {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .shared
            .submit_shared(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        let id = snapshot.id;
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.queue.clear();
            registry
                .record_mut(id)
                .expect("живая запись")
                .snapshot
                .state = JobStateDto::Running;
        }
        let gate = Arc::new(HelpingWindowGate {
            target: id,
            held: Mutex::new(true),
            released: Condvar::new(),
            entered: std::sync::atomic::AtomicBool::new(false),
        });
        *HELPING_WINDOW_GATE.lock().expect("шлюз без отравления") = Some(Arc::clone(&gate));
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let shared = Arc::clone(&runtime.shared);
        std::thread::spawn(move || {
            // Сервис не `Send` из-за движка — поток строит свой.
            let worker_engine = self::engine();
            let service = WorkerJobService {
                shared,
                engine: worker_engine,
                session_token: [5; 16],
                profile_index: 0,
            };
            let state = if first_change {
                bsl_rt::BackgroundJobService::wait_first_change(
                    &service,
                    &[(id, JobStateDto::Running)],
                    None,
                )
                .map(|held| held[0].state)
            } else {
                bsl_rt::BackgroundJobService::wait_terminal(&service, &[id], None)
                    .map(|outcome| outcome.snapshots[0].state)
            };
            let _ = result_sender.send(state);
        });
        // Дождаться входа ожидания в окно.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !gate.entered.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "ожидание не вошло в окно шлюза"
            );
            std::thread::yield_now();
        }
        // Terminal transition и «теряемое» уведомление — именно в окне,
        // до начала парковки.
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.finish(id, JobStateDto::Completed, None, None);
        }
        runtime.shared.terminal_watch.notify_all();
        // Шлюз открывается: ожидание идёт к парковке и обязано
        // перечитать предикат под guard, а не уснуть до дедлайна.
        *gate.held.lock().expect("шлюз без отравления") = false;
        gate.released.notify_all();
        let state = result_receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("terminal-событие в окне не должно теряться парковкой")
            .expect("ожидание без ошибок");
        assert_eq!(state, JobStateDto::Completed);
        *HELPING_WINDOW_GATE.lock().expect("шлюз без отравления") = None;
    }

    /// Terminal-событие в окне «предикат проверен, парковка ещё не
    /// началась» не теряется ни одной из форм helping-ожидания: полный
    /// предикат перечитывается под guard, уходящим в `wait_timeout`.
    #[test]
    fn a_terminal_event_in_the_helping_window_is_not_lost() {
        helping_window_scenario(false);
        helping_window_scenario(true);
    }

    /// Helping-ожидание первого изменения паркуется на `terminal_watch`
    /// до события: за сотни миллисекунд тихого ожидания счётчик парковок
    /// растёт на единицы, а не на сотни двухмиллисекундных тиков — тест
    /// исключает периодические пробуждения без события.
    #[test]
    fn wait_first_change_parks_without_periodic_wakeups() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Задание принимается в реестр БЕЗ запуска пула (submit_shared не
        // спавнит потоки), очередь вычищается, а запись переводится в
        // Running вручную: helping-ожиданию некому помогать и не от кого
        // дождаться изменения — остаётся только спать.
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .shared
            .submit_shared(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.queue.clear();
            registry
                .record_mut(snapshot.id)
                .expect("живая запись")
                .snapshot
                .state = JobStateDto::Running;
        }
        let parks_before = HELPING_PARKS.load(std::sync::atomic::Ordering::Relaxed);
        let shared = Arc::clone(&runtime.shared);
        let id = snapshot.id;
        let waiter = std::thread::spawn(move || {
            // Сервис не `Send` из-за движка — поток строит свой.
            let worker_engine = self::engine();
            let service = WorkerJobService {
                shared,
                engine: worker_engine,
                session_token: [3; 16],
                profile_index: 0,
            };
            bsl_rt::BackgroundJobService::wait_first_change(
                &service,
                &[(id, JobStateDto::Running)],
                None,
            )
        });
        std::thread::sleep(Duration::from_millis(300));
        // Изменение публикуется штатной парой finish + notify — ожидание
        // просыпается от события, а не от таймера.
        {
            let mut registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            registry.finish(id, JobStateDto::Completed, None, None);
        }
        runtime.shared.terminal_watch.notify_all();
        let held = waiter
            .join()
            .expect("поток ожидания")
            .expect("ожидание без ошибок");
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].state, JobStateDto::Completed);
        let parks = HELPING_PARKS.load(std::sync::atomic::Ordering::Relaxed) - parks_before;
        assert!(
            parks <= 8,
            "за 300 мс тихого ожидания {parks} парковок — таймерный поллинг вернулся"
        );
    }

    /// Неперехваченная BSL-ошибка при закрытом сеансе-получателе: BSL-ошибка
    /// остаётся основной, отказ terminal commit — вторичной причиной в
    /// `cause`, а не молча проглоченным результатом `commit_staged`.
    #[test]
    fn a_failed_commit_after_a_bsl_error_becomes_a_secondary_cause() {
        let engine = crate::Engine::builder()
            .common_module(
                "Служебный",
                "Процедура ПишетИПадает(Знач Адрес) Экспорт\n\
                     ПоместитьВоВременноеХранилище(\"из задания\", Адрес);\n\
                     ВызватьИсключение \"падение задания\";\n\
                 КонецПроцедуры",
            )
            .build()
            .expect("движок с каталогом");
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Сеанс-вызыватель регистрируется в hub и закрывается ДО запуска
        // задания: staging пройдёт (он не смотрит в hub), а terminal
        // commit гарантированно встретит закрытый mailbox — без гонок.
        let caller_token = [7u8; 16];
        {
            let session =
                bsl_rt::TempStorageSession::new(caller_token, bsl_rt::HostEnv::process().random());
            engine.temp_hub().register(caller_token, session.mailbox());
        }
        let address = format!(
            "e1cib/tempstorage/{}?seanceId={}",
            bsl_rt::uuid::format(&[9u8; 16]),
            bsl_rt::uuid::format(&caller_token)
        );
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.ПишетИПадает")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.ПишетИПадает",
                target,
                params(&[bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&address))]),
                None,
                None,
                Some(caller_token),
                0,
            )
            .expect("задание принято");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime.snapshot(snapshot.id).expect("снимок");
        assert_eq!(done.state, JobStateDto::Failed);
        let error = done.error.as_ref().expect("ошибка задания");
        assert!(
            error.brief.contains("падение задания"),
            "основной обязана остаться BSL-ошибка: {}",
            error.brief
        );
        let mut causes = Vec::new();
        let mut tail = error.cause.as_deref();
        while let Some(cause) = tail {
            causes.push(cause.brief.clone());
            tail = cause.cause.as_deref();
        }
        assert!(
            causes
                .iter()
                .any(|cause| cause.contains("сеанс-получатель временного хранилища закрыт")),
            "отказ commit обязан быть вторичной причиной: {causes:?}"
        );
    }
}
