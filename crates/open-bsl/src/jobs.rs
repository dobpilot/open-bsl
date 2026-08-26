//! Нативный runtime фоновых заданий: конфигурация, реестр, admission.
//!
//! Архитектурные инварианты (план фоновых заданий, `docs/bsl-background-jobs.md`):
//! реестр хранит только владеющие `Send`-DTO и никогда не вызывает внешний
//! код под своим локом; мьютекс — синхронный `std::sync::Mutex` с короткими
//! секциями и не пересекает `await`; занятый пул означает FIFO, а не
//! ошибку; первый terminal transition выигрывает ровно один раз.

// Пул workers подключается следующим шагом этапа 3; до него часть
// каркаса (машина состояний runtime, очередь) используется только
// тестами admission.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use bsl_rt::{JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, SerializedValueGraph};

/// Конфигурация фонового runtime — одна публичная структура; scheduler
/// настраивается своей `SchedulerConfig` и здесь не дублируется.
///
/// Значения по умолчанию предварительные: production defaults, кроме
/// `max_history_jobs = 10_000`, выбираются после нагрузочных прогонов
/// плана.
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
        if self.max_error_bytes_per_job + self.max_message_bytes_per_job
            > self.max_single_job_record_bytes
        {
            return Err("бюджеты ошибки и сообщений не помещаются в запись истории".to_string());
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

/// Источник идентификаторов заданий: стандартный — OS random + UUID v4,
/// тестовый — детерминированная последовательность.
pub trait JobIdSource: Send + Sync {
    fn next_id(&self) -> JobId;
}

/// Состояние runtime. Занятость workers его не меняет: занятый пул — это
/// FIFO, а не ошибка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeState {
    Cold,
    Starting,
    Running,
    Broken,
    Closed,
}

/// Запись реестра: снимок + резервы admission. Только владеющие DTO.
pub(crate) struct JobRecord {
    pub snapshot: JobSnapshotDto,
    /// Байты payload, зарезервированные в live-бюджете до terminal.
    pub payload_bytes: usize,
    /// Цель вызова числами: модуль каталога и индекс чанка.
    pub target: (u32, u16),
}

/// Ключ уникальности: пара «цель + снимок ключа» резервируется до
/// terminal transition (`JOB.KEY.QUEUED` уточнит участие queued).
type KeyReservation = (u32, u16, JobKeyDto);

/// Реестр под одним синхронным мьютексом: записи, очередь, ключи,
/// бюджеты, история. История хранит `Arc`-снимки: листинг клонирует
/// указатели под локом и фильтрует вне его.
pub(crate) struct JobRegistry {
    pub state: RuntimeState,
    records: HashMap<JobId, JobRecord>,
    pub queue: VecDeque<JobId>,
    keys: Vec<(KeyReservation, JobId)>,
    live_payload_bytes: usize,
    inflight: usize,
    history: VecDeque<Arc<JobSnapshotDto>>,
    history_bytes: usize,
    config: BackgroundJobConfig,
}

/// Исход admission.
pub(crate) enum AdmissionError {
    /// Явный ресурсный лимит — ловимая ошибка BSL.
    ResourceLimit(String),
    /// Дублирующий ключ активного задания.
    DuplicateKey,
    /// Runtime закрыт или сломан.
    Unavailable(RuntimeState),
}

impl JobRegistry {
    pub fn new(config: BackgroundJobConfig) -> Self {
        Self {
            state: RuntimeState::Cold,
            records: HashMap::new(),
            queue: VecDeque::new(),
            keys: Vec::new(),
            live_payload_bytes: 0,
            inflight: 0,
            history: VecDeque::new(),
            history_bytes: 0,
            config,
        }
    }

    /// Admission: атомарно резервирует слот, байты payload и ключ до
    /// terminal transition; ставит задание в глобальную FIFO.
    pub fn admit(
        &mut self,
        id: JobId,
        method_name: String,
        target: (u32, u16),
        params: Arc<SerializedValueGraph>,
        key: Option<Arc<JobKeyDto>>,
        description: Option<String>,
    ) -> Result<JobSnapshotDto, AdmissionError> {
        match self.state {
            RuntimeState::Broken | RuntimeState::Closed => {
                return Err(AdmissionError::Unavailable(self.state));
            }
            _ => {}
        }
        if self.inflight >= self.config.max_inflight_jobs {
            return Err(AdmissionError::ResourceLimit(format!(
                "достигнут предел одновременных заданий ({})",
                self.config.max_inflight_jobs
            )));
        }
        let payload_bytes = params.byte_size();
        if payload_bytes > self.config.max_single_job_record_bytes {
            return Err(AdmissionError::ResourceLimit(
                "параметры задания больше предела одной записи".to_string(),
            ));
        }
        if self.live_payload_bytes + payload_bytes > self.config.max_live_payload_bytes {
            return Err(AdmissionError::ResourceLimit(
                "исчерпан live-бюджет параметров заданий".to_string(),
            ));
        }
        if let Some(key) = &key {
            let reservation = (target.0, target.1, (**key).clone());
            if self
                .keys
                .iter()
                .any(|(existing, _)| *existing == reservation)
            {
                return Err(AdmissionError::DuplicateKey);
            }
            self.keys.push((reservation, id));
        }
        self.inflight += 1;
        self.live_payload_bytes += payload_bytes;
        let snapshot = JobSnapshotDto {
            id,
            method_name,
            params,
            key,
            description,
            state: JobStateDto::Queued,
            begin: None,
            end: None,
            error: None,
        };
        self.records.insert(
            id,
            JobRecord {
                snapshot: snapshot.clone(),
                payload_bytes,
                target,
            },
        );
        self.queue.push_back(id);
        Ok(snapshot)
    }

    pub fn record(&self, id: JobId) -> Option<&JobRecord> {
        self.records.get(&id)
    }

    pub fn record_mut(&mut self, id: JobId) -> Option<&mut JobRecord> {
        self.records.get_mut(&id)
    }

    /// Terminal transition: ровно один раз — повторный вызов no-op с
    /// `false`. Освобождает admission-резервы и ключ, переносит payload из
    /// live-бюджета в history-бюджет без двойного учёта, вытесняет
    /// старейшие terminal-записи (амортизировано, на добавлении).
    pub fn finish(
        &mut self,
        id: JobId,
        state: JobStateDto,
        end: Option<bsl_rt::BslDate>,
        error: Option<Arc<JobErrorDto>>,
    ) -> bool {
        debug_assert!(state.is_terminal());
        let Some(mut record) = self.records.remove(&id) else {
            return false;
        };
        if record.snapshot.state.is_terminal() {
            self.records.insert(id, record);
            return false;
        }
        record.snapshot.state = state;
        record.snapshot.end = end;
        record.snapshot.error = error;
        self.inflight -= 1;
        self.live_payload_bytes -= record.payload_bytes;
        self.keys.retain(|(_, owner)| *owner != id);
        self.queue.retain(|queued| *queued != id);
        // Запись, которая не помещается в историю целиком, не хранится:
        // admission уже отверг явно превышающие, здесь — страховка.
        let record_bytes = record
            .payload_bytes
            .min(self.config.max_single_job_record_bytes);
        self.history_bytes += record_bytes;
        self.history.push_back(Arc::new(record.snapshot));
        while self.history.len() > self.config.max_history_jobs
            || self.history_bytes > self.config.max_history_bytes
        {
            let Some(evicted) = self.history.pop_front() else {
                break;
            };
            self.history_bytes -= evicted
                .params
                .byte_size()
                .min(self.config.max_single_job_record_bytes);
        }
        true
    }

    /// Снимок задания: живая запись либо история.
    pub fn snapshot(&self, id: JobId) -> Option<Arc<JobSnapshotDto>> {
        if let Some(record) = self.records.get(&id) {
            return Some(Arc::new(record.snapshot.clone()));
        }
        self.history
            .iter()
            .find(|snapshot| snapshot.id == id)
            .cloned()
    }

    /// Все снимки: живые + история. Возвращает `Arc`-указатели — фильтр
    /// работает вне лока.
    pub fn snapshots(&self) -> Vec<Arc<JobSnapshotDto>> {
        let mut all: Vec<Arc<JobSnapshotDto>> = self
            .records
            .values()
            .map(|record| Arc::new(record.snapshot.clone()))
            .collect();
        all.extend(self.history.iter().cloned());
        all
    }

    pub fn inflight(&self) -> usize {
        self.inflight
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::{GraphLimits, RuntimeShapes};

    fn graph(bytes_hint: usize) -> Arc<SerializedValueGraph> {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = bsl_rt::BslValue::Str(bsl_rt::BslString::from_str(&"ы".repeat(bytes_hint / 2)));
        Arc::new(
            SerializedValueGraph::capture(&[value], &rt, &GraphLimits::default()).expect("снимок"),
        )
    }

    fn id(byte: u8) -> JobId {
        JobId([byte; 16])
    }

    #[test]
    fn admission_reserves_and_finish_releases() {
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_inflight_jobs: 2,
            ..BackgroundJobConfig::default()
        });
        let params = graph(64);
        registry
            .admit(id(1), "М.Ф".into(), (0, 1), params.clone(), None, None)
            .map_err(|_| ())
            .expect("первое задание принято");
        registry
            .admit(id(2), "М.Ф".into(), (0, 1), params.clone(), None, None)
            .map_err(|_| ())
            .expect("второе задание принято");
        assert!(matches!(
            registry.admit(id(3), "М.Ф".into(), (0, 1), params.clone(), None, None),
            Err(AdmissionError::ResourceLimit(_))
        ));
        assert!(registry.finish(id(1), JobStateDto::Completed, None, None));
        assert!(
            !registry.finish(id(1), JobStateDto::Failed, None, None),
            "terminal transition ровно один раз"
        );
        registry
            .admit(id(3), "М.Ф".into(), (0, 1), params, None, None)
            .map_err(|_| ())
            .expect("слот освобождён");
        assert_eq!(registry.history_len(), 1);
        assert_eq!(
            registry
                .snapshot(id(1))
                .expect("история хранит снимок")
                .state,
            JobStateDto::Completed
        );
    }

    #[test]
    fn a_duplicate_key_is_rejected_until_terminal() {
        let mut registry = JobRegistry::new(BackgroundJobConfig::default());
        let params = graph(16);
        let key = {
            let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
            Arc::new(JobKeyDto {
                graph: SerializedValueGraph::capture(
                    &[bsl_rt::BslValue::Boolean(true)],
                    &rt,
                    &GraphLimits::default(),
                )
                .expect("снимок ключа"),
            })
        };
        registry
            .admit(
                id(1),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                Some(key.clone()),
                None,
            )
            .map_err(|_| ())
            .expect("первое принято");
        assert!(matches!(
            registry.admit(
                id(2),
                "М.Ф".into(),
                (0, 1),
                params.clone(),
                Some(key.clone()),
                None
            ),
            Err(AdmissionError::DuplicateKey)
        ));
        // Тот же ключ у ДРУГОЙ цели — не дубль.
        registry
            .admit(
                id(3),
                "М.Д".into(),
                (0, 2),
                params.clone(),
                Some(key.clone()),
                None,
            )
            .map_err(|_| ())
            .expect("другая цель принята");
        registry.finish(id(1), JobStateDto::Canceled, None, None);
        registry
            .admit(id(4), "М.Ф".into(), (0, 1), params, Some(key), None)
            .map_err(|_| ())
            .expect("ключ освобождён terminal transition");
    }

    #[test]
    fn history_eviction_is_amortized_on_insert() {
        let mut registry = JobRegistry::new(BackgroundJobConfig {
            max_history_jobs: 2,
            ..BackgroundJobConfig::default()
        });
        let params = graph(16);
        for i in 1..=4u8 {
            registry
                .admit(id(i), "М.Ф".into(), (0, 1), params.clone(), None, None)
                .map_err(|_| ())
                .expect("принято");
            registry.finish(id(i), JobStateDto::Completed, None, None);
        }
        assert_eq!(registry.history_len(), 2);
        assert!(registry.snapshot(id(1)).is_none(), "старейшие вытеснены");
        assert!(registry.snapshot(id(4)).is_some());
    }

    #[test]
    fn an_invalid_config_is_rejected() {
        let config = BackgroundJobConfig {
            workers: Some(0),
            ..BackgroundJobConfig::default()
        };
        assert!(config.validate().is_err());
        let config = BackgroundJobConfig {
            max_staged_temp_bytes_per_job: 2,
            max_live_staged_temp_bytes: 1,
            ..BackgroundJobConfig::default()
        };
        assert!(config.validate().is_err());
        assert!(BackgroundJobConfig::default().validate().is_ok());
    }
}
