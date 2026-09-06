use super::registry::{AdmissionError, JobRegistry, RuntimeState, fail_all_resident};
use super::{TargetTable, WorkerRecipe, take_thread_residents, worker_main};
use bsl_rt::{
    GlobalStagingBudget, HostError, HostErrorCode, JobId, JobKeyDto, JobSnapshotDto, JobStateDto,
    SerializedValueGraph, UserMessageDto,
};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Конфигурация фонового runtime — одна публичная структура; scheduler
/// настраивается своей `SchedulerConfig` и здесь не дублируется.
///
/// Значения по умолчанию подтверждены нагрузочной сессией 2026-08-27
/// (см. «Нагрузочные и регрессионные проверки» плана фоновых заданий).
#[derive(Debug, Clone)]
pub struct BackgroundJobConfig {
    /// Число OS-потоков пула; `None` — `available_parallelism`, минимум 1.
    pub workers: Option<usize>,
    pub max_inflight_jobs: usize,
    pub max_live_payload_bytes: usize,
    /// История: намеренное расширение open-bsl — 10 000 против
    /// документированных 1 000 у платформы.
    pub max_history_jobs: usize,
    pub max_history_bytes: usize,
    pub max_single_job_record_bytes: usize,
    pub max_error_bytes_per_job: usize,
    pub max_message_bytes_per_job: usize,
    pub max_staged_temp_bytes_per_job: usize,
    pub max_live_staged_temp_bytes: usize,
    pub shutdown_timeout: Duration,
}

impl Default for BackgroundJobConfig {
    fn default() -> Self {
        Self {
            workers: None,
            max_inflight_jobs: 1_024,
            max_live_payload_bytes: 256 << 20,
            max_history_jobs: 10_000,
            max_history_bytes: 256 << 20,
            max_single_job_record_bytes: 64 << 20,
            max_error_bytes_per_job: 1 << 20,
            max_message_bytes_per_job: 4 << 20,
            max_staged_temp_bytes_per_job: 64 << 20,
            max_live_staged_temp_bytes: 256 << 20,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl BackgroundJobConfig {
    /// Проверка согласованности — скрытых clamp нет: несогласованная
    /// конфигурация отвергается сборкой движка.
    ///
    /// # Errors
    ///
    /// Текст первого нарушения.
    pub fn validate(&self) -> Result<(), String> {
        if self.workers == Some(0) {
            return Err("число workers не может быть нулевым".to_string());
        }
        if self.max_staged_temp_bytes_per_job > self.max_live_staged_temp_bytes {
            return Err("staging одного задания больше глобального staging-бюджета".to_string());
        }
        if self.max_single_job_record_bytes > self.max_history_bytes {
            return Err("одна запись истории больше всего бюджета истории".to_string());
        }
        // `checked_add`: публичная конфигурация — переполнение суммы
        // бюджетов отвергается как несогласованность, а не паникует.
        match self
            .max_error_bytes_per_job
            .checked_add(self.max_message_bytes_per_job)
        {
            Some(sum) if sum <= self.max_single_job_record_bytes => {}
            _ => {
                return Err("бюджеты ошибки и сообщений не помещаются в запись истории".to_string());
            }
        }
        if self.max_inflight_jobs == 0 {
            return Err("max_inflight_jobs не может быть нулевым".to_string());
        }
        Ok(())
    }

    /// Фактическое число workers.
    #[must_use]
    pub fn effective_workers(&self) -> usize {
        self.workers
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .unwrap_or(1)
            })
            .max(1)
    }
}

/// Источник времени runtime: wall-часы для `Начало`/`Конец` снимков.
/// Монотонных deadlines и таймеров здесь нет — пул спит на condvar до
/// события (`wake_epoch`), а не по таймеру; тестовая реализация
/// подставляет ручные значения.
pub trait JobTimeSource: Send + Sync {
    fn wall_now(&self) -> Option<bsl_rt::BslDate>;
}

/// Системные wall-часы. Снимки получают наивное UTC-время: локальная
/// зона сеанса и точный формат платформенных `Начало`/`Конец` уточняются
/// замером `JOB.STATE.SNAPSHOT`. Wall clock не clamp'ится при скачке
/// назад — монотонные длительности придут отдельным источником.
struct SystemJobTime;

impl JobTimeSource for SystemJobTime {
    fn wall_now(&self) -> Option<bsl_rt::BslDate> {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        bsl_rt::BslDate::from_seconds(unix as i64 + bsl_rt::UNIX_EPOCH_SECONDS)
    }
}

/// Источник идентификаторов заданий: стандартный — OS random + UUID v4,
/// тестовый — детерминированная последовательность.
pub trait JobIdSource: Send + Sync {
    fn next_id(&self) -> JobId;
}

// --- Host-профили сеансов заданий --------------------------------------

/// Непрозрачный идентификатор host-профиля фоновых заданий, выданный
/// [`crate::EngineBuilder::register_host_profile`]. Привязан к движку,
/// зарегистрировавшему профиль: идентификатор чужого движка отвергается
/// при выборе профиля ошибкой, без fallback на process-профиль.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostProfileId {
    pub(crate) engine: u64,
    pub(crate) index: u32,
}

/// Фабрика host-окружений сеансов фоновых заданий. Вызывается в потоке
/// worker для каждого задания своего профиля; `Send + Sync` — одна
/// фабрика разделяется всем пулом, а непереносимые сервисы сеанса
/// (`Rc`-обёртки ФС, часов, сети) строятся внутри вызова и поток worker
/// не покидают. Foreground-сервисы вызывающего `State` — его `files`,
/// `network`, часы и вывод — в worker не копируются: профиль и есть их
/// именованная замена.
pub trait BackgroundStateFactory: Send + Sync {
    /// Настраивает сеанс задания. Builder приходит с process-default
    /// сервисами worker-движка (стандартные компоненты плюс
    /// пользовательские библиотеки родительского движка); фабрика
    /// ограничивает или подменяет их — например `deny_network` или
    /// изолированная файловая система.
    ///
    /// # Errors
    ///
    /// Текст причины: задание завершается `Failed` с кодом
    /// `HostProfileUnavailable`, worker остаётся жив.
    fn configure(&self, builder: crate::StateBuilder) -> Result<crate::StateBuilder, String>;
}

/// Разделяемое состояние runtime: реестр под синхронным мьютексом и два
/// условия — «есть работа» для workers и «есть terminal» для ожидающих.
/// Ни одно внешнее действие под локом не выполняется.
pub(crate) struct JobRuntimeShared {
    pub registry: Mutex<JobRegistry>,
    pub work_available: Condvar,
    pub terminal_watch: Condvar,
    pub recipe: WorkerRecipe,
    pub id_source: Arc<dyn JobIdSource>,
    pub time_source: Arc<dyn JobTimeSource>,
    pub targets: TargetTable,
    /// Реестр mailbox'ов временного хранилища родительского движка —
    /// публикации write-set'ов заданий идут сюда.
    pub temp_hub: Arc<bsl_rt::TempStorageHub>,
    /// Копия конфигурации для чтения лимитов без лока реестра.
    pub config: BackgroundJobConfig,
    /// Фабрики host-профилей движка: индекс записи + 1 — это
    /// `JobRecord::profile_index`; 0 — системный профиль без фабрики.
    pub profiles: Arc<[Arc<dyn BackgroundStateFactory>]>,
    /// Глобальный staging-бюджет временного хранилища всех живых заданий
    /// (`max_live_staged_temp_bytes`).
    pub staging_global: Arc<GlobalStagingBudget>,
    /// Внешний sink представления сообщений заданий, если host его
    /// зарегистрировал; история записи пишется до него и не теряется при
    /// его backpressure.
    pub message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
    /// OS-потоки пула. Список живёт в разделяемом состоянии, потому что
    /// соседей спавнит бутстраппер с потока первого worker; порядок локов
    /// всюду один — сначала этот список, затем реестр.
    pub threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
    /// Тестовый шлюз бутстрапа: пока держится, первый worker не спавнит
    /// соседей и не берёт работу — тест наблюдает возврат первого
    /// admission до запуска пула. Продакшен-путь шлюз не выставляет.
    pub bootstrap_gate: Mutex<Option<Arc<BootstrapGate>>>,
}

/// Тестовый шлюз бутстрапа пула: флаг «удержан» под мьютексом и condvar
/// освобождения.
pub(crate) type BootstrapGate = (Mutex<bool>, Condvar);

impl JobRuntimeShared {
    /// Все перечисленные задания terminal (неизвестные считаются
    /// вытесненными и потому terminal).
    pub(super) fn all_terminal(registry: &JobRegistry, ids: &[JobId]) -> bool {
        ids.iter().all(|id| {
            registry
                .snapshot(*id)
                .is_none_or(|snapshot| snapshot.state.is_terminal())
        })
    }

    /// Admission нового задания в общий реестр. Пул не поднимает: это
    /// обязанность внешнего `JobRuntime::submit`; вложенный submit из
    /// worker приходит, когда потоки уже работают.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn submit_shared(
        &self,
        method_name: &str,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let id = self.id_source.next_id();
        let mut registry = self.registry.lock().expect("реестр без отравления");
        let snapshot = registry
            .admit(
                id,
                method_name.to_string(),
                target,
                params,
                key,
                description,
                caller_token,
                profile_index,
            )
            .map_err(|error| match error {
                AdmissionError::ResourceLimit(text) => {
                    HostError::new(HostErrorCode::ResourceLimit, text)
                }
                AdmissionError::DuplicateKey => HostError::new(
                    HostErrorCode::InvalidCall,
                    "задание с таким ключом уже активно",
                ),
                AdmissionError::Unavailable(state) => unavailable_error(state),
            })?;
        if registry.state == RuntimeState::Cold {
            registry.state = RuntimeState::Starting;
        }
        Ok(snapshot)
    }

    /// То же по имени «Модуль.Метод» — для вложенного submit из worker.
    pub(crate) fn submit_by_name_shared(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let target = self
            .targets
            .resolve(method_name)
            .map_err(|text| HostError::new(HostErrorCode::InvalidCall, text))?;
        let snapshot = self.submit_shared(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )?;
        self.work_available.notify_one();
        // Helping-ожидание спит на terminal_watch без таймера: новое
        // задание в очереди — тоже его событие (появился кандидат на
        // доводку).
        self.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Доступность live-методов: закрытый и сломанный runtime отвечают
    /// типизированной ловимой ошибкой, а не тихим no-op; свойства уже
    /// материализованных снимков при этом остаются читаемыми.
    pub(super) fn live_guard(registry: &JobRegistry) -> Result<(), HostError> {
        match registry.state {
            RuntimeState::Closed | RuntimeState::Broken => Err(unavailable_error(registry.state)),
            _ => Ok(()),
        }
    }

    /// Задание известно либо ошибка `JobExpired`: live-методы вытесненного
    /// снимка не притворяются успешными и не скрывают потерю истории
    /// пустым результатом.
    pub(super) fn known_guard(registry: &JobRegistry, id: JobId) -> Result<(), HostError> {
        if registry.knows(id) {
            return Ok(());
        }
        Err(HostError::new(
            HostErrorCode::JobExpired,
            "задание неизвестно: вытеснено из истории либо никогда не существовало",
        ))
    }

    /// Отмена задания. `Queued` завершается сразу; `Running` получает
    /// взведённый флаг и завершится на границе кванта; повторная отмена и
    /// отмена terminal — успешный no-op (ИЗМЕРЕНО, `JOB.CANCEL.RACES`).
    /// Вытесненное из истории задание — ловимая `JobExpired`. Отмена — не
    /// ошибка BSL: снимок получает «Отменено» без `ИнформацияОбОшибке`.
    pub(crate) fn cancel(&self, id: JobId, end: Option<bsl_rt::BslDate>) -> Result<(), HostError> {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        Self::known_guard(&registry, id)?;
        let Some(record) = registry.record(id) else {
            // Запись в истории: terminal, no-op.
            return Ok(());
        };
        match record.snapshot.state {
            JobStateDto::Queued => {
                registry.finish(id, JobStateDto::Canceled, end, None);
                drop(registry);
                self.terminal_watch.notify_all();
            }
            JobStateDto::Running => {
                record
                    .cancel_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                // Точно спящий worker обязан проснуться и опросить
                // резидента со взведённым флагом: припаркованный на
                // host-операции poll вернёт Canceled, не дожидаясь
                // ответа транспорта.
                registry.wake_epoch += 1;
                drop(registry);
                self.work_available.notify_all();
                self.terminal_watch.notify_all();
            }
            _ => {}
        }
        Ok(())
    }

    /// Свежие снимки всех `ids`, удержанные под текущим локом реестра.
    /// Задание, известное на входе ожидания, но исчезнувшее к этому
    /// моменту, вытеснено из истории — ловимая `JobExpired`, устаревший
    /// снимок за свежий не выдаётся.
    pub(super) fn held_snapshots(
        registry: &JobRegistry,
        ids: &[JobId],
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        ids.iter()
            .map(|id| {
                registry.snapshot(*id).ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::JobExpired,
                        "задание вытеснено из истории во время ожидания",
                    )
                })
            })
            .collect()
    }

    /// Флаги семантики менеджерного ожидания по снимкам, удержанным под
    /// одним локом: (есть активные, есть изменившиеся, все terminal,
    /// есть аварийные).
    pub(super) fn first_change_flags(
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        held: &[Arc<JobSnapshotDto>],
    ) -> (bool, bool, bool, bool) {
        let mut any_active = false;
        let mut any_changed = false;
        let mut all_terminal = true;
        let mut any_failed = false;
        for ((_, initial), snapshot) in jobs.iter().zip(held) {
            let state = snapshot.state;
            if !state.is_terminal() {
                any_active = true;
                all_terminal = false;
            }
            if state != *initial {
                any_changed = true;
            }
            if state == JobStateDto::Failed {
                any_failed = true;
            }
        }
        (any_active, any_changed, all_terminal, any_failed)
    }

    /// Ожидание по семантике синтакс-помощника: активных нет — сразу;
    /// с таймаутом — до первого изменения статуса; без — до завершения
    /// всех либо первого аварийного. Возвращает свежие снимки всех
    /// `jobs`, удержанные под финальным локом, — размер результата равен
    /// размеру запроса, вытеснение во время ожидания — `JobExpired`.
    pub(crate) fn wait_first_change(
        &self,
        jobs: &[(JobId, bsl_rt::JobStateDto)],
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<JobSnapshotDto>>, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let ids: Vec<JobId> = jobs.iter().map(|(id, _)| *id).collect();
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        for id in &ids {
            Self::known_guard(&registry, *id)?;
        }
        loop {
            // Закрытие во время ожидания: ответ уже не придёт штатно —
            // ловимая ошибка вместо вечного сна на отсоединённом worker
            // (симметрично `wait_terminal_blocking`).
            Self::live_guard(&registry)?;
            let held = Self::held_snapshots(&registry, &ids)?;
            let (any_active, any_changed, all_terminal, any_failed) =
                Self::first_change_flags(jobs, &held);
            if !any_active {
                return Ok(held);
            }
            match deadline {
                Some(_) if any_changed => return Ok(held),
                None if all_terminal || any_failed => return Ok(held),
                _ => {}
            }
            match deadline {
                None => {
                    registry = self
                        .terminal_watch
                        .wait(registry)
                        .expect("реестр без отравления");
                }
                Some(deadline) => {
                    let now = std::time::Instant::now();
                    let Some(left) = deadline.checked_duration_since(now) else {
                        return Ok(held);
                    };
                    let (guard, _) = self
                        .terminal_watch
                        .wait_timeout(registry, left)
                        .expect("реестр без отравления");
                    registry = guard;
                }
            }
        }
    }

    /// Сообщения задания: живая запись и terminal-история отдают (и при
    /// `remove` атомарно забирают) свой FIFO — ИЗМЕРЕНО (`JOB.MESSAGES`).
    /// Вытесненное задание — ловимая `JobExpired`.
    pub(crate) fn take_messages(
        &self,
        id: JobId,
        remove: bool,
    ) -> Result<Vec<UserMessageDto>, HostError> {
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        registry.take_messages(id, remove).ok_or_else(|| {
            HostError::new(
                HostErrorCode::JobExpired,
                "задание неизвестно: вытеснено из истории либо никогда не существовало",
            )
        })
    }

    /// Блокирующее ожидание terminal-состояния всех `ids` — путь
    /// foreground-сеанса; worker вместо блокировки помогает пулу (см.
    /// `WorkerJobService`). Неизвестное на входе задание — `JobExpired`;
    /// задание, вытесненное УЖЕ ВО ВРЕМЯ ожидания, terminal, но его
    /// свежего снимка больше нет — тоже `JobExpired`, а не устаревший
    /// снимок. Снимки результата удерживаются под финальным локом.
    pub(crate) fn wait_terminal_blocking(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bsl_rt::JobWaitOutcome, HostError> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);
        let mut registry = self.registry.lock().expect("реестр без отравления");
        Self::live_guard(&registry)?;
        for id in ids {
            Self::known_guard(&registry, *id)?;
        }
        loop {
            if Self::all_terminal(&registry, ids) {
                return Ok(bsl_rt::JobWaitOutcome {
                    completed: true,
                    snapshots: Self::held_snapshots(&registry, ids)?,
                });
            }
            // Закрытие во время ожидания: ответ уже не придёт штатно —
            // ловимая ошибка вместо вечного сна на отсоединённом worker.
            Self::live_guard(&registry)?;
            match deadline {
                None => {
                    registry = self
                        .terminal_watch
                        .wait(registry)
                        .expect("реестр без отравления");
                }
                Some(deadline) => {
                    let now = std::time::Instant::now();
                    let Some(left) = deadline.checked_duration_since(now) else {
                        return Ok(bsl_rt::JobWaitOutcome {
                            completed: false,
                            snapshots: Self::held_snapshots(&registry, ids)?,
                        });
                    };
                    let (guard, result) = self
                        .terminal_watch
                        .wait_timeout(registry, left)
                        .expect("реестр без отравления");
                    registry = guard;
                    if result.timed_out() {
                        // Последняя проверка под локом — событие могло
                        // прийти на границе таймаута.
                        return Ok(bsl_rt::JobWaitOutcome {
                            completed: Self::all_terminal(&registry, ids),
                            snapshots: Self::held_snapshots(&registry, ids)?,
                        });
                    }
                }
            }
        }
    }
}

/// Нативный runtime фоновых заданий одного `Engine`. Клоны `Engine`
/// разделяют runtime; OS-потоки поднимаются лениво при первом успешном
/// admission — движок без заданий потоков не создаёт.
pub struct JobRuntime {
    pub(super) shared: Arc<JobRuntimeShared>,
    workers: usize,
}

/// UUID v4 из ключей ОС — общий генератор идентификаторов и токенов.
pub(crate) fn random_uuid() -> [u8; 16] {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        let state = std::collections::hash_map::RandomState::new();
        let value = state.build_hasher().finish().to_le_bytes();
        chunk.copy_from_slice(&value[..chunk.len()]);
    }
    bsl_rt::uuid::v4_from_bytes(bytes)
}

/// Стандартный источник идентификаторов: OS random + UUID v4.
pub(super) struct SystemJobIds;

impl JobIdSource for SystemJobIds {
    fn next_id(&self) -> JobId {
        JobId(random_uuid())
    }
}

/// Ошибка недоступного runtime по его состоянию — ловимая на стороне BSL.
fn unavailable_error(state: RuntimeState) -> HostError {
    match state {
        RuntimeState::Broken => HostError::new(
            HostErrorCode::RuntimeBroken,
            "фоновый runtime сломан и не принимает задания",
        ),
        _ => HostError::new(
            HostErrorCode::RuntimeClosed,
            "фоновый runtime закрыт и не принимает задания",
        ),
    }
}

impl JobRuntime {
    /// Создаёт холодный runtime: потоков нет до первого admission.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: BackgroundJobConfig,
        recipe: WorkerRecipe,
        targets: TargetTable,
        id_source: Arc<dyn JobIdSource>,
        temp_hub: Arc<bsl_rt::TempStorageHub>,
        profiles: Arc<[Arc<dyn BackgroundStateFactory>]>,
        message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
    ) -> Self {
        let workers = config.effective_workers();
        let staging_global = Arc::new(GlobalStagingBudget::new(config.max_live_staged_temp_bytes));
        Self {
            shared: Arc::new(JobRuntimeShared {
                registry: Mutex::new(JobRegistry::new(config.clone())),
                work_available: Condvar::new(),
                terminal_watch: Condvar::new(),
                recipe,
                id_source,
                time_source: Arc::new(SystemJobTime),
                targets,
                temp_hub,
                config,
                profiles,
                staging_global,
                message_display,
                threads: Mutex::new(Vec::new()),
                bootstrap_gate: Mutex::new(None),
            }),
            workers,
        }
    }

    /// Ставит тестовый шлюз бутстрапа — только до первого admission.
    #[cfg(test)]
    fn set_bootstrap_gate(&self, gate: Arc<BootstrapGate>) {
        *self
            .shared
            .bootstrap_gate
            .lock()
            .expect("шлюз без отравления") = Some(gate);
    }

    /// Запускает экспортный метод общего модуля в отдельном сеансе.
    /// Возвращает снимок `Queued`; сам запуск пула ленивый и submissions
    /// во время `Starting` не ждут создания всех потоков.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn submit(
        &self,
        method_name: &str,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let snapshot = self.shared.submit_shared(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )?;
        self.ensure_workers();
        self.shared.work_available.notify_one();
        // См. submit_by_name_shared: помощники ждут на terminal_watch.
        self.shared.terminal_watch.notify_all();
        Ok(snapshot)
    }

    /// Ленивый запуск пула: первый admission спавнит РОВНО ОДИН поток —
    /// первого worker, который перед собственным циклом бутстрапит
    /// соседей (`worker_bootstrap`). Возврат первого admission не ждёт
    /// создания всего пула; паника создания потока — `Broken` для всего
    /// runtime.
    fn ensure_workers(&self) {
        let mut threads = self
            .shared
            .threads
            .lock()
            .expect("список потоков без отравления");
        if !threads.is_empty() {
            return;
        }
        let shared = Arc::clone(&self.shared);
        let workers = self.workers;
        let builder = std::thread::Builder::new().name("bsl-job-worker-0".to_string());
        match builder.spawn(move || {
            worker_bootstrap(&shared, workers);
            worker_supervisor(&shared);
        }) {
            Ok(handle) => threads.push(handle),
            Err(_) => {
                let mut registry = self.shared.registry.lock().expect("реестр без отравления");
                registry.state = RuntimeState::Broken;
                fail_all_resident(&mut registry);
                self.shared.terminal_watch.notify_all();
            }
        }
    }

    /// Запускает цель по имени «Модуль.Метод»: разрешение цели по
    /// каталогу рецепта worker + admission. Ошибки цели и лимитов —
    /// ловимые на стороне BSL.
    ///
    /// # Errors
    ///
    /// [`HostError`] с причиной: `InvalidCall` для негодной цели и
    /// дублирующего ключа, `ResourceLimit` для явных лимитов,
    /// `RuntimeClosed`/`RuntimeBroken` для закрытого runtime.
    pub fn submit_by_name(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<JobSnapshotDto, HostError> {
        self.submit_by_name_with_caller(method_name, params, key, description, None, 0)
    }

    /// То же с token'ом временного хранилища вызывателя и host-профилем —
    /// путь сервисов.
    pub(crate) fn submit_by_name_with_caller(
        &self,
        method_name: &str,
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
        caller_token: Option<[u8; 16]>,
        profile_index: u32,
    ) -> Result<JobSnapshotDto, HostError> {
        let target = self
            .shared
            .targets
            .resolve(method_name)
            .map_err(|text| HostError::new(HostErrorCode::InvalidCall, text))?;
        self.submit(
            method_name,
            target,
            params,
            key,
            description,
            caller_token,
            profile_index,
        )
    }

    /// Снимок задания по идентификатору.
    pub fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshot(id)
    }

    /// Все снимки (живые и история) — фильтрация вне лока.
    pub fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        self.shared
            .registry
            .lock()
            .expect("реестр без отравления")
            .snapshots()
    }

    /// Ожидает terminal-состояния ВСЕХ перечисленных заданий (ИЗМЕРЕНО,
    /// `JOB.WAIT.MANY`). `None` — без предела. `Ok(true)` — дождались,
    /// `Ok(false)` — таймаут.
    ///
    /// # Errors
    ///
    /// `JobExpired` для неизвестного задания на входе и для задания,
    /// вытесненного из истории во время ожидания; `RuntimeClosed`/
    /// `RuntimeBroken` после закрытия runtime.
    pub fn wait_terminal(
        &self,
        ids: &[JobId],
        timeout: Option<Duration>,
    ) -> Result<bool, HostError> {
        self.shared
            .wait_terminal_blocking(ids, timeout)
            .map(|outcome| outcome.completed)
    }

    /// Отмена задания: `Queued` — сразу, `Running` — кооперативно на
    /// границе кванта; повторная отмена и terminal — no-op.
    ///
    /// # Errors
    ///
    /// Как у [`JobRuntime::wait_terminal`].
    pub fn cancel(&self, id: JobId) -> Result<(), HostError> {
        let end = self.shared.time_source.wall_now();
        self.shared.cancel(id, end)
    }

    /// Сообщения задания в порядке FIFO; `remove` атомарно забирает их —
    /// и у живой записи, и у terminal (ИЗМЕРЕНО, `JOB.MESSAGES`).
    ///
    /// # Errors
    ///
    /// Как у [`JobRuntime::wait_terminal`].
    pub fn take_messages(&self, id: JobId, remove: bool) -> Result<Vec<UserMessageDto>, HostError> {
        self.shared.take_messages(id, remove)
    }

    /// Явное завершение runtime: queued отменяются сразу, running —
    /// кооперативно на границах квантов; потоки соединяются до `deadline`,
    /// неответившие отсоединяются и попадают в отчёт. Новые submissions
    /// после закрытия получают ловимую ошибку.
    pub fn shutdown(&self, deadline: Duration) -> ShutdownReport {
        let end = self.shared.time_source.wall_now();
        {
            let mut registry = self.shared.registry.lock().expect("реестр без отравления");
            registry.state = RuntimeState::Closed;
            let jobs: Vec<(JobId, JobStateDto)> = registry
                .records()
                .map(|record| (record.snapshot.id, record.snapshot.state))
                .collect();
            for (id, state) in jobs {
                match state {
                    JobStateDto::Queued => {
                        registry.finish(id, JobStateDto::Canceled, end, None);
                    }
                    JobStateDto::Running => {
                        if let Some(record) = registry.record(id) {
                            record
                                .cancel_requested
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.shared.work_available.notify_all();
        self.shared.terminal_watch.notify_all();

        let handles: Vec<std::thread::JoinHandle<()>> = std::mem::take(
            &mut *self
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления"),
        );
        let deadline_at = std::time::Instant::now() + deadline;
        let mut detached_workers = 0usize;
        for handle in handles {
            loop {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                if std::time::Instant::now() >= deadline_at {
                    // Отсоединяем: поток физически может дорабатывать уже
                    // начатый внешний эффект, но реестр закрыт и его
                    // поздние публикации отвергаются состоянием Closed.
                    detached_workers += 1;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        ShutdownReport { detached_workers }
    }
}

/// Отчёт явного завершения: сколько workers не успели выйти до deadline
/// и были отсоединены.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReport {
    pub detached_workers: usize,
}

impl Drop for JobRuntime {
    /// Последний владелец сигнализирует завершение, но НЕ блокируется:
    /// потоки выйдут сами на ближайшей границе кванта.
    fn drop(&mut self) {
        let mut registry = self.shared.registry.lock().expect("реестр без отравления");
        if registry.state != RuntimeState::Closed {
            registry.state = RuntimeState::Closed;
            for record in registry.records() {
                record
                    .cancel_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        drop(registry);
        self.shared.work_available.notify_all();
        self.shared.terminal_watch.notify_all();
    }
}

/// Бутстрап пула на потоке ПЕРВОГО worker: соседи спавнятся отсюда, а не
/// из admission, поэтому первый submit возвращается после одного spawn,
/// а не N. Переход `Starting -> Running` делает бутстраппер после
/// последнего соседа; отказ spawn ломает runtime (`Broken`); закрытие во
/// время `Starting` останавливает спавн перед следующим соседом, и все
/// уже созданные handles остаются в разделяемом списке — shutdown их
/// соединяет. Порядок локов: сначала список потоков, затем реестр —
/// поэтому поздний spawn не может проскочить мимо взятого shutdown
/// списка.
fn worker_bootstrap(shared: &Arc<JobRuntimeShared>, workers: usize) {
    wait_bootstrap_gate(shared);
    for index in 1..workers {
        let mut threads = shared
            .threads
            .lock()
            .expect("список потоков без отравления");
        {
            let registry = shared.registry.lock().expect("реестр без отравления");
            if matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken) {
                return;
            }
        }
        let sibling = Arc::clone(shared);
        let builder = std::thread::Builder::new().name(format!("bsl-job-worker-{index}"));
        match builder.spawn(move || worker_supervisor(&sibling)) {
            Ok(handle) => threads.push(handle),
            Err(_) => {
                drop(threads);
                let mut registry = shared.registry.lock().expect("реестр без отравления");
                registry.state = RuntimeState::Broken;
                fail_all_resident(&mut registry);
                drop(registry);
                shared.terminal_watch.notify_all();
                shared.work_available.notify_all();
                return;
            }
        }
    }
    let mut registry = shared.registry.lock().expect("реестр без отравления");
    if registry.state == RuntimeState::Starting {
        registry.state = RuntimeState::Running;
    }
}

/// Удержание тестового шлюза бутстрапа. Без шлюза — no-op; удержанный
/// шлюз пережидается с оглядкой на состояние runtime: закрытие или
/// поломка снимают бутстраппер и с шлюза. Пятидесятимиллисекундный тик
/// существует только на тестовом пути с выставленным шлюзом.
fn wait_bootstrap_gate(shared: &Arc<JobRuntimeShared>) {
    let gate = shared
        .bootstrap_gate
        .lock()
        .expect("шлюз без отравления")
        .clone();
    let Some(gate) = gate else {
        return;
    };
    let (held, released) = &*gate;
    let mut held_guard = held.lock().expect("шлюз без отравления");
    while *held_guard {
        {
            let registry = shared.registry.lock().expect("реестр без отравления");
            if matches!(registry.state, RuntimeState::Closed | RuntimeState::Broken) {
                return;
            }
        }
        let (guard, _) = released
            .wait_timeout(held_guard, Duration::from_millis(50))
            .expect("шлюз без отравления");
        held_guard = guard;
    }
}

/// Надзор за worker: паника ВНЕ границы задания роняет резидентов этого
/// worker в `Failed` (их Drop-гарды, см. `RunningJob`) и создаёт замену —
/// цикл перезапускается. Три ПОСЛЕДОВАТЕЛЬНЫЕ паники запуска — worker ни
/// разу не довёл задание до terminal между ними — переводят runtime в
/// `Broken`: живые задания завершаются `Failed`, новые submissions
/// получают ловимую ошибку, автоматического recovery нет.
fn worker_supervisor(shared: &Arc<JobRuntimeShared>) {
    let mut consecutive_panics = 0u32;
    loop {
        let progress = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let progress_flag = Arc::clone(&progress);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker_main(shared, &progress_flag);
        }));
        match outcome {
            Ok(()) => return, // нормальный выход: runtime закрыт или сломан
            Err(_) => {
                // Резиденты живут в пуле ПОТОКА и разматыванием паники не
                // дропаются: забираем их здесь — Drop-гарды переводят их
                // в `Failed`, вечного `Running` не остаётся.
                drop(take_thread_residents());
                if progress.load(std::sync::atomic::Ordering::Relaxed) {
                    consecutive_panics = 1;
                } else {
                    consecutive_panics += 1;
                }
                if consecutive_panics >= 3 {
                    let mut registry = shared.registry.lock().expect("реестр без отравления");
                    registry.state = RuntimeState::Broken;
                    fail_all_resident(&mut registry);
                    drop(registry);
                    shared.terminal_watch.notify_all();
                    // Соседние worker'ы спят без таймера — поломку они
                    // обязаны увидеть по state, а не по случайному событию.
                    shared.work_available.notify_all();
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod pool_tests {
    use super::super::test_support::{engine, number, params};
    use super::super::*;
    use super::*;

    /// Критерий готовности плана: Engine без заданий не создаёт
    /// OS-потоков — пул поднимается лениво, первым admission.
    #[test]
    fn the_pool_spawns_no_threads_before_the_first_admission() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        assert!(
            runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .is_empty(),
            "до первого admission пул обязан быть пустым"
        );
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        assert!(
            !runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .is_empty(),
            "первый admission поднимает пул"
        );
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
    }

    /// Первый успешный admission возвращается ДО полного запуска пула:
    /// удержанный тестовый шлюз не даёт бутстрапу спавнить соседей и
    /// брать работу, а submit при этом уже вернул снимок `Queued` — ровно
    /// один поток, состояние `Starting`. После открытия шлюза пул
    /// достраивается до `workers` и доводит задание.
    #[test]
    fn the_first_admission_returns_before_the_pool_is_fully_started() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(4),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let gate: Arc<BootstrapGate> = Arc::new((Mutex::new(true), Condvar::new()));
        runtime.set_bootstrap_gate(Arc::clone(&gate));
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято при удержанном бутстрапе");
        // Submit уже вернулся, а пул стоит на шлюзе: ровно один поток
        // (бутстраппер), состояние Starting, задание ещё Queued.
        assert_eq!(
            runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .len(),
            1,
            "admission спавнит ровно одного бутстраппера"
        );
        {
            let registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            assert_eq!(registry.state, RuntimeState::Starting);
        }
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Queued,
            "до открытия шлюза задание не берётся в работу"
        );
        // Открытый шлюз достраивает пул и доводит задание.
        *gate.0.lock().expect("шлюз без отравления") = false;
        gate.1.notify_all();
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let spawned = runtime
                .shared
                .threads
                .lock()
                .expect("список потоков без отравления")
                .len();
            if spawned == 4 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "пул обязан достроиться до workers, потоков {spawned}"
            );
            std::thread::yield_now();
        }
        {
            let registry = runtime
                .shared
                .registry
                .lock()
                .expect("реестр без отравления");
            assert_eq!(registry.state, RuntimeState::Running);
        }
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Completed
        );
    }

    /// Shutdown во время `Starting` безопасен: бутстраппер снимается и со
    /// шлюза, очередь отменяется, ни один поток не отсоединяется, новые
    /// submissions получают ловимую ошибку закрытого runtime.
    #[test]
    fn a_shutdown_during_starting_is_safe() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(4),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let gate: Arc<BootstrapGate> = Arc::new((Mutex::new(true), Condvar::new()));
        runtime.set_bootstrap_gate(Arc::clone(&gate));
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        let report = runtime.shutdown(Duration::from_secs(10));
        assert_eq!(
            report.detached_workers, 0,
            "удержанный шлюзом бутстраппер обязан выйти по закрытию"
        );
        assert_eq!(
            runtime.snapshot(snapshot.id).expect("снимок").state,
            JobStateDto::Canceled,
            "queued-задание отменяется закрытием"
        );
        let error = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1), number(2)]),
                None,
                None,
                None,
                0,
            )
            .expect_err("закрытый runtime отвергает задания");
        assert_eq!(error.code, bsl_rt::HostErrorCode::RuntimeClosed);
    }

    #[test]
    fn a_job_runs_to_completion_in_a_worker() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(2), number(3)]),
                None,
                None,
                None,
                0,
            )
            .expect("задание принято");
        assert_eq!(snapshot.state, JobStateDto::Queued);
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
        let done = runtime
            .snapshot(snapshot.id)
            .expect("снимок после terminal");
        assert_eq!(
            done.state,
            JobStateDto::Completed,
            "ошибка: {:?}",
            done.error
        );
    }

    #[test]
    fn a_raising_job_finishes_as_failed_with_the_error_text() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Упасть")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit("Служебный.Упасть", target, params(&[]), None, None, None, 0)
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
            error.brief.contains("задание падает"),
            "не тот текст: {}",
            error.brief
        );
    }

    #[test]
    fn missing_required_arguments_fail_the_job() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let snapshot = runtime
            .submit(
                "Служебный.Сложить",
                target,
                params(&[number(1)]),
                None,
                None,
                None,
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
    }

    #[test]
    fn target_resolution_rejects_unknown_and_non_exported() {
        let engine = engine();
        let catalog = engine.catalog().unwrap();
        assert!(resolve_target(catalog, "Нет.Такого").is_err());
        assert!(resolve_target(catalog, "Служебный.НетМетода").is_err());
        assert!(resolve_target(catalog, "БезТочки").is_err());
        resolve_target(catalog, "Служебный.Сложить").expect("экспортная цель годится");
    }

    #[test]
    fn many_jobs_all_reach_terminal_states() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(2),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let target = resolve_target(engine.catalog().unwrap(), "Служебный.Сложить")
            .expect("цель разрешается");
        let ids: Vec<_> = (0..16)
            .map(|i| {
                runtime
                    .submit(
                        "Служебный.Сложить",
                        target,
                        params(&[number(i), number(i)]),
                        None,
                        None,
                        None,
                        0,
                    )
                    .expect("задание принято")
                    .id
            })
            .collect();
        assert!(
            runtime
                .wait_terminal(&ids, Some(Duration::from_secs(60)))
                .expect("ожидание без ошибок")
        );
        for id in ids {
            assert_eq!(
                runtime.snapshot(id).expect("снимок").state,
                JobStateDto::Completed
            );
        }
    }

    /// Переход `Queued -> Running` — первое изменение статуса по семантике
    /// менеджерного `ОжидатьЗавершенияВыполнения`: ожидание с таймаутом
    /// обязано проснуться на нём, а не досидеть до дедлайна.
    #[test]
    fn a_queued_to_running_transition_wakes_the_manager_wait() {
        let engine = engine();
        let runtime = runtime_for_engine(
            &engine,
            BackgroundJobConfig {
                workers: Some(1),
                ..BackgroundJobConfig::default()
            },
        )
        .expect("runtime собирается");
        let target =
            resolve_target(engine.catalog().unwrap(), "Служебный.Вечно").expect("цель разрешается");
        let snapshot = runtime
            .submit("Служебный.Вечно", target, params(&[]), None, None, None, 0)
            .expect("задание принято");
        // Задание не завершится само: единственное изменение статуса —
        // старт. Без пробуждения на нём ожидание досидело бы до дедлайна.
        let started = std::time::Instant::now();
        let held = runtime
            .shared
            .wait_first_change(
                &[(snapshot.id, JobStateDto::Queued)],
                Some(Duration::from_secs(20)),
            )
            .expect("ожидание без ошибок");
        let elapsed = started.elapsed();
        assert_eq!(held.len(), 1);
        assert_eq!(
            held[0].state,
            JobStateDto::Running,
            "ожидание обязано вернуть свежий снимок «Активно»"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "ожидание досидело до дедлайна ({elapsed:?}) — переход в Running не будит"
        );
        runtime.cancel(snapshot.id).expect("отмена");
        assert!(
            runtime
                .wait_terminal(&[snapshot.id], Some(Duration::from_secs(30)))
                .expect("ожидание без ошибок")
        );
    }

    #[test]
    fn waiting_for_an_unknown_job_is_a_job_expired_error() {
        let engine = engine();
        let runtime = runtime_for_engine(&engine, BackgroundJobConfig::default())
            .expect("runtime собирается");
        // Ложный идентификатор: live-метод не притворяется успешным —
        // ловимая ошибка с кодом JobExpired.
        let error = runtime
            .wait_terminal(&[JobId([9; 16])], Some(Duration::from_millis(10)))
            .expect_err("неизвестное задание — ошибка");
        assert_eq!(error.code, bsl_rt::HostErrorCode::JobExpired);
    }
}
