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

    /// Байты владеющего текста ошибки вместе с cause-цепочкой — вклад в
    /// размер записи истории.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.brief.len()
            + self.full.len()
            + self.module.as_deref().map_or(0, str::len)
            + self.method.as_deref().map_or(0, str::len)
            + self.cause.as_deref().map_or(0, JobErrorDto::byte_size)
    }

    /// Укладывает диагностику в бюджет `max_error_bytes_per_job`: сначала
    /// отбрасывается cause-цепочка, затем усечению подлежит полный текст,
    /// последним — краткий. Oversized diagnostic не обходит бюджет памяти:
    /// хвост заменяется признаком `DiagnosticResourceLimit`, а
    /// `diagnostic_truncated` остаётся видимым Rust-встраиванию. Бюджет
    /// строгий для ЛЮБОГО `max_bytes`, включая нулевой и меньший самого
    /// признака: `byte_size()` результата никогда не превышает
    /// `max_bytes`, а признак появляется, только когда сам помещается.
    #[must_use]
    pub fn bounded(mut self, max_bytes: usize) -> Self {
        if self.byte_size() <= max_bytes {
            return self;
        }
        self.diagnostic_truncated = true;
        self.cause = None;
        let clip = |text: &str, budget: usize| -> String {
            if text.len() <= budget {
                return text.to_string();
            }
            // Бюджет строже информативности: когда признак усечения сам
            // не помещается, остаётся жёсткая обрезка по границе символа.
            let keep = if budget < TRUNCATION_SUFFIX.len() {
                budget
            } else {
                budget - TRUNCATION_SUFFIX.len()
            };
            let mut cut = keep;
            while cut > 0 && !text.is_char_boundary(cut) {
                cut -= 1;
            }
            if budget < TRUNCATION_SUFFIX.len() {
                text[..cut].to_string()
            } else {
                format!("{}{TRUNCATION_SUFFIX}", &text[..cut])
            }
        };
        // Краткому тексту отдаётся не больше половины бюджета, полному —
        // остаток: обе части остаются информативными.
        let brief_budget = (max_bytes / 2).max(TRUNCATION_SUFFIX.len()).min(max_bytes);
        self.brief = clip(&self.brief, brief_budget);
        let mut rest = max_bytes - self.brief.len();
        // Имена модуля и метода либо помещаются в остаток целиком, либо
        // отбрасываются: усечённый идентификатор бесполезен, а текст
        // ошибки важнее координат.
        for field in [&mut self.module, &mut self.method] {
            match field {
                Some(text) if text.len() <= rest => rest -= text.len(),
                Some(_) => *field = None,
                None => {}
            }
        }
        self.full = clip(&self.full, rest);
        self
    }
}

/// Признак усечения диагностики бюджетом: подменяет хвост текста, когда
/// сам помещается в отведённый бюджет.
const TRUNCATION_SUFFIX: &str = "… [DiagnosticResourceLimit]";

/// Владеющее сообщение пользователю — `Send`-снимок всех полей
/// `СообщениеПользователю`. Значения `КлючДанных` и
/// `ИдентификаторНазначения` произвольны, поэтому пересекают границу
/// сеансов сериализованными графами и материализуются у читающего.
#[derive(Debug, Clone)]
pub struct UserMessageDto {
    pub text: String,
    pub field: String,
    pub data_path: String,
    pub data_key: Option<SerializedValueGraph>,
    pub target_id: Option<SerializedValueGraph>,
}

impl UserMessageDto {
    /// Сообщение из одного текста — путь глобального `Сообщить`.
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            field: String::new(),
            data_path: String::new(),
            data_key: None,
            target_id: None,
        }
    }

    /// Байты сообщения — вклад в `max_message_bytes_per_job` и в размер
    /// записи истории.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.text.len()
            + self.field.len()
            + self.data_path.len()
            + self
                .data_key
                .as_ref()
                .map_or(0, SerializedValueGraph::byte_size)
            + self
                .target_id
                .as_ref()
                .map_or(0, SerializedValueGraph::byte_size)
    }
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<JobKeyDto>();
    assert_send_sync::<JobErrorDto>();
    assert_send_sync::<JobSnapshotDto>();
    assert_send_sync::<UserMessageDto>();
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Полностью заполненная ошибка с cause-цепочкой — худший случай для
    /// бюджета: усечению подлежат все составляющие.
    fn full_error() -> JobErrorDto {
        JobErrorDto {
            brief: "краткое описание ошибки".repeat(4),
            full: "полная диагностика с подробностями".repeat(8),
            module: Some("ОченьДлинноеИмяОбщегоМодуля".to_string()),
            method: Some("ИмяМетодаЦели".to_string()),
            line: Some(42),
            diagnostic_truncated: false,
            cause: Some(Box::new(JobErrorDto::from_text(
                "вложенная причина".repeat(6),
            ))),
        }
    }

    /// Бюджет строгий для любого допустимого лимита: `byte_size()`
    /// результата не превышает `max_bytes` даже там, где не помещается
    /// сам признак `DiagnosticResourceLimit`.
    #[test]
    fn bounded_is_strict_for_any_budget() {
        for max_bytes in [
            0,
            1,
            TRUNCATION_SUFFIX.len() - 1,
            TRUNCATION_SUFFIX.len(),
            256,
        ] {
            let bounded = full_error().bounded(max_bytes);
            assert!(
                bounded.byte_size() <= max_bytes,
                "бюджет {max_bytes} нарушен: {} байт ({bounded:?})",
                bounded.byte_size()
            );
            assert!(bounded.diagnostic_truncated, "усечение обязано быть видно");
            assert!(bounded.cause.is_none(), "cause отбрасывается первым");
        }
    }

    /// Нулевой бюджет опустошает каждое текстовое поле, а не паникует и
    /// не оставляет «минимальный» признак сверх бюджета.
    #[test]
    fn a_zero_budget_empties_every_text_field() {
        let bounded = full_error().bounded(0);
        assert_eq!(bounded.byte_size(), 0);
        assert_eq!(bounded.brief, "");
        assert_eq!(bounded.full, "");
        assert_eq!(bounded.module, None);
        assert_eq!(bounded.method, None);
    }

    /// В обычном бюджете признак усечения присутствует, краткий текст
    /// сохраняет информативное начало, а имена модуля и метода целы.
    #[test]
    fn a_normal_budget_keeps_the_suffix_and_coordinates() {
        let budget = 512;
        let bounded = full_error().bounded(budget);
        assert!(bounded.byte_size() <= budget);
        assert!(
            bounded.brief.starts_with("краткое описание"),
            "начало краткого текста обязано сохраниться: {}",
            bounded.brief
        );
        assert!(
            bounded.full.ends_with(TRUNCATION_SUFFIX),
            "нет признака усечения: {}",
            bounded.full
        );
        assert_eq!(
            bounded.module.as_deref(),
            Some("ОченьДлинноеИмяОбщегоМодуля"),
            "в просторном бюджете координаты сохраняются"
        );
    }

    /// Ошибка, уже помещающаяся в бюджет, не меняется вовсе.
    #[test]
    fn a_fitting_error_is_returned_untouched() {
        let error = JobErrorDto::from_text("короткий текст");
        let size = error.byte_size();
        let bounded = error.clone().bounded(size);
        assert_eq!(bounded, error);
        assert!(!bounded.diagnostic_truncated);
    }

    /// Многобайтовые символы режутся по границе символа и в ветке без
    /// признака: бюджет в середине кодовой точки не рвёт UTF-8.
    #[test]
    fn truncation_respects_char_boundaries() {
        for max_bytes in 0..TRUNCATION_SUFFIX.len() + 8 {
            let bounded = full_error().bounded(max_bytes);
            assert!(bounded.byte_size() <= max_bytes, "бюджет {max_bytes}");
            // Валидность UTF-8 гарантирована типом String; здесь важно,
            // что усечение не паникует ни на одной границе.
        }
    }
}
