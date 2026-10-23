//! Приобретённые связи параметров разделения без зависимостей на источник данных.

use open_bsl::ColumnType;

/// Известные связи разделителя и приобретённые типы его значения.
/// `None` у списка типов сохраняет неизвестную форму метаданных.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonAttributeSeparationParameters {
    /// Идентификатор параметра значения; `None` — явно отсутствующая связь.
    pub value_parameter: Option<String>,
    /// Идентификатор параметра использования; `None` — явно отсутствующая связь.
    pub use_parameter: Option<String>,
    /// Полное объявление типов значения; неизвестное объявление — `None`.
    pub value_types: Option<Vec<ColumnType>>,
}
