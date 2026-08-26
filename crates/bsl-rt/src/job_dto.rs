//! Владеющие DTO границы фоновых заданий.
//!
//! Через межпоточную границу `JobRegistry` проходят только владеющие
//! `Send`-значения: ни `BslValue`, ни `Rc`, ни `RtError` в этих типах не
//! встречаются (см. план фоновых заданий, «Архитектурные инварианты»).

use crate::SerializedValueGraph;

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
};
