//! Владеющие DTO границы фоновых заданий.
//!
//! Через межпоточную границу `JobRegistry` проходят только владеющие
//! `Send`-значения: ни `BslValue`, ни `Rc`, ни `RtError` в этих типах не
//! встречаются (см. план фоновых заданий, «Архитектурные инварианты»).

use std::sync::Arc;

use crate::{BslDate, SerializedValueGraph};

/// Идентификатор фонового задания — UUID, выданный `JobIdSource`
/// runtime. Значение непрозрачно и стабильно на всё время жизни записи.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub [u8; 16]);

impl JobId {
    /// Каноническая печать UUID — нижний регистр, 8-4-4-4-12.
    #[must_use]
    pub fn to_uuid_string(&self) -> String {
        crate::uuid::format(&self.0)
    }
}

/// Состояние задания в снимке. Внутренняя машина переходов закрыта в
/// runtime; BSL-отображение (`Активно` для Queued/Running) подтверждается
/// замером `JOB.STATE.SNAPSHOT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStateDto {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
}

impl JobStateDto {
    /// Terminal-состояние: запись больше не изменится.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

/// Неизменяемый снимок задания: то, что видит BSL-объект
/// `ФоновоеЗадание`. Старые снимки не обновляются — поиск и ожидание
/// создают новый. Тяжёлые поля разделяются через `Arc`, поэтому список
/// из тысяч снимков не копирует графы параметров.
#[derive(Debug, Clone)]
pub struct JobSnapshotDto {
    pub id: JobId,
    /// Полное имя цели: `Модуль.Метод`.
    pub method_name: String,
    pub params: Arc<SerializedValueGraph>,
    pub key: Option<Arc<JobKeyDto>>,
    pub description: Option<String>,
    pub state: JobStateDto,
    /// Wall-часы `JobTimeSource`; `Начало` пишется атомарно при
    /// `Queued -> Running`, `Конец` — при terminal transition.
    pub begin: Option<BslDate>,
    pub end: Option<BslDate>,
    pub error: Option<Arc<JobErrorDto>>,
    /// Сообщения пользователю задания в порядке FIFO. У живой записи
    /// снимок несёт срез на момент создания; полная модель сообщения —
    /// за замером `JOB.MESSAGES`.
    pub messages: Vec<String>,
}

/// Ключ фонового задания: снимок значения ключа. Равенство — полная
/// структурная идентичность снимка; канонизация типов и семантика циклов
/// вводятся после замеров `JOB.KEY.EQUALITY`/`JOB.KEY.QUEUED`. Хеш для
/// индекса registry строит process-local keyed hasher поверх снимка —
/// digest не сериализуется и BSL-контрактом не является.
#[derive(Debug, Clone, PartialEq)]
pub struct JobKeyDto {
    pub graph: SerializedValueGraph,
}

/// Ошибка фонового задания: владеющий текст и координаты, рекурсивная
/// причина — тем же типом. В вызывающем сеансе из DTO строится новая
/// `ИнформацияОбОшибке`; идентичность исходного объекта не сохраняется.
#[derive(Debug, Clone, PartialEq)]
pub struct JobErrorDto {
    /// Краткий текст — то, что вернула бы `ОписаниеОшибки()`.
    pub brief: String,
    /// Полный диагностический текст. Ограничен бюджетом
    /// `max_error_bytes_per_job`; при усечении хост ставит
    /// `diagnostic_truncated` и подменяет хвост признаком
    /// `DiagnosticResourceLimit`.
    pub full: String,
    /// Имя модуля и метода, если известны.
    pub module: Option<String>,
    pub method: Option<String>,
    /// Строка исходника, если известна.
    pub line: Option<u32>,
    /// Диагностика была усечена бюджетом.
    pub diagnostic_truncated: bool,
    /// Вложенная причина.
    pub cause: Option<Box<JobErrorDto>>,
}

impl JobErrorDto {
    /// DTO из одного текста — самый частый случай на границе VM.
    #[must_use]
    pub fn from_text(brief: impl Into<String>) -> Self {
        let brief = brief.into();
        Self {
            full: brief.clone(),
            brief,
            module: None,
            method: None,
            line: None,
            diagnostic_truncated: false,
            cause: None,
        }
    }
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<JobKeyDto>();
    assert_send_sync::<JobErrorDto>();
    assert_send_sync::<JobSnapshotDto>();
};
