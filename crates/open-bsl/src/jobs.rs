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
use prepare::{TargetTable, WorkerRecipe};
mod runtime;
mod worker;

use worker::{DriveMode, drive_local, take_thread_residents, worker_main};

use runtime::JobRuntimeShared;
pub(crate) use runtime::random_uuid;
pub use runtime::{
    BackgroundJobConfig, BackgroundStateFactory, HostProfileId, JobIdSource, JobRuntime,
    JobTimeSource, ShutdownReport,
};

use registry::JobRegistry;
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::{HostError, JobId, JobSnapshotDto, SerializedValueGraph, UserMessageDto};

// --- Пул workers -------------------------------------------------------

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

#[cfg(test)]
mod test_support {
    use super::*;
    use bsl_rt::{GraphLimits, RuntimeShapes};

    pub(super) fn engine() -> crate::Engine {
        crate::Engine::builder()
            .common_module(
                "Служебный",
                "Перем Счётчик Экспорт;\n\
                 Функция Сложить(Знач а, Знач б) Экспорт\n\
                     Возврат а + б;\n\
                 КонецФункции\n\
                 Процедура Упасть() Экспорт\n\
                     ВызватьИсключение \"задание падает\";\n\
                 КонецПроцедуры\n\
                 Процедура Вечно() Экспорт\n\
                     Пока Истина Цикл\n\
                     КонецЦикла;\n\
                 КонецПроцедуры\n\
                 Счётчик = 0;",
            )
            .build()
            .expect("движок с каталогом")
    }

    pub(super) fn params(values: &[bsl_rt::BslValue]) -> Arc<SerializedValueGraph> {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        Arc::new(
            SerializedValueGraph::capture(values, &rt, &GraphLimits::default())
                .expect("снимок параметров"),
        )
    }

    pub(super) fn number(value: i64) -> bsl_rt::BslValue {
        bsl_rt::BslValue::number_from_i64(value)
    }
}
