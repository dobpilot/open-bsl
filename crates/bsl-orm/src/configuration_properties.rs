//! Свойства конфигурации в целом: вариант встроенного языка, режим
//! совместимости и режим обновления предопределённых данных.

use super::*;

/// Основной язык программирования конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptVariant {
    /// Русский вариант.
    Russian,
    /// Английский вариант.
    English,
}

impl ScriptVariant {
    pub(crate) fn value(self) -> Value {
        Value::Enum(match self {
            Self::Russian => EnumValue::ScriptVariantRussian,
            Self::English => EnumValue::ScriptVariantEnglish,
        })
    }
}

/// Режим обновления предопределённых данных информационной базы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredefinedDataUpdateMode {
    /// Режим определяется настройками объектов и узла распределённой базы.
    Auto,
    /// Предопределённые данные обновляются автоматически.
    AutoUpdate,
    /// Автоматическое обновление отключено.
    DontAutoUpdate,
}

impl PredefinedDataUpdateMode {
    pub(crate) fn value(self) -> Value {
        Value::Enum(match self {
            Self::Auto => EnumValue::PredefinedDataUpdateAuto,
            Self::AutoUpdate => EnumValue::PredefinedDataUpdateAutoUpdate,
            Self::DontAutoUpdate => EnumValue::PredefinedDataUpdateDontAutoUpdate,
        })
    }

    pub(crate) fn from_value(value: &Value) -> RtResult<Self> {
        match value {
            Value::Enum(EnumValue::PredefinedDataUpdateAuto) => Ok(Self::Auto),
            Value::Enum(EnumValue::PredefinedDataUpdateAutoUpdate) => Ok(Self::AutoUpdate),
            Value::Enum(EnumValue::PredefinedDataUpdateDontAutoUpdate) => Ok(Self::DontAutoUpdate),
            _ => Err(RtError::TypeError {
                expected: "ОбновлениеПредопределенныхДанных",
                op: "УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
            }),
        }
    }
}

/// Режим совместимости конфигурации с версией платформы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityMode {
    /// Совместимость с 8.1.
    Version8_1,
    /// Совместимость с 8.2.13.
    Version8_2_13,
    /// Совместимость с 8.2.16.
    Version8_2_16,
    /// Совместимость с 8.3.1.
    Version8_3_1,
    /// Совместимость с 8.3.2.
    Version8_3_2,
    /// Совместимость с 8.3.3.
    Version8_3_3,
    /// Совместимость с 8.3.4.
    Version8_3_4,
    /// Совместимость с 8.3.5.
    Version8_3_5,
    /// Совместимость с 8.3.6.
    Version8_3_6,
    /// Совместимость с 8.3.7.
    Version8_3_7,
    /// Совместимость с 8.3.8.
    Version8_3_8,
    /// Совместимость с 8.3.9.
    Version8_3_9,
    /// Совместимость с 8.3.10.
    Version8_3_10,
    /// Совместимость с 8.3.11.
    Version8_3_11,
    /// Совместимость с 8.3.12.
    Version8_3_12,
    /// Совместимость с 8.3.13.
    Version8_3_13,
    /// Совместимость с 8.3.14.
    Version8_3_14,
    /// Совместимость с 8.3.15.
    Version8_3_15,
    /// Совместимость с 8.3.16.
    Version8_3_16,
    /// Совместимость с 8.3.17.
    Version8_3_17,
    /// Совместимость с 8.3.18.
    Version8_3_18,
    /// Совместимость с 8.3.19.
    Version8_3_19,
    /// Совместимость с 8.3.20.
    Version8_3_20,
    /// Совместимость с 8.3.21.
    Version8_3_21,
    /// Совместимость с 8.3.22.
    Version8_3_22,
    /// Совместимость с 8.3.23.
    Version8_3_23,
    /// Совместимость с 8.3.24.
    Version8_3_24,
    /// Совместимость с 8.3.25.
    Version8_3_25,
    /// Совместимость с 8.3.26.
    Version8_3_26,
    /// Ограничение совместимости не используется.
    DontUse,
}

impl CompatibilityMode {
    pub(crate) fn value(self) -> Value {
        Value::Enum(match self {
            Self::Version8_1 => EnumValue::CompatibilityModeVersion8_1,
            Self::Version8_2_13 => EnumValue::CompatibilityModeVersion8_2_13,
            Self::Version8_2_16 => EnumValue::CompatibilityModeVersion8_2_16,
            Self::Version8_3_1 => EnumValue::CompatibilityModeVersion8_3_1,
            Self::Version8_3_2 => EnumValue::CompatibilityModeVersion8_3_2,
            Self::Version8_3_3 => EnumValue::CompatibilityModeVersion8_3_3,
            Self::Version8_3_4 => EnumValue::CompatibilityModeVersion8_3_4,
            Self::Version8_3_5 => EnumValue::CompatibilityModeVersion8_3_5,
            Self::Version8_3_6 => EnumValue::CompatibilityModeVersion8_3_6,
            Self::Version8_3_7 => EnumValue::CompatibilityModeVersion8_3_7,
            Self::Version8_3_8 => EnumValue::CompatibilityModeVersion8_3_8,
            Self::Version8_3_9 => EnumValue::CompatibilityModeVersion8_3_9,
            Self::Version8_3_10 => EnumValue::CompatibilityModeVersion8_3_10,
            Self::Version8_3_11 => EnumValue::CompatibilityModeVersion8_3_11,
            Self::Version8_3_12 => EnumValue::CompatibilityModeVersion8_3_12,
            Self::Version8_3_13 => EnumValue::CompatibilityModeVersion8_3_13,
            Self::Version8_3_14 => EnumValue::CompatibilityModeVersion8_3_14,
            Self::Version8_3_15 => EnumValue::CompatibilityModeVersion8_3_15,
            Self::Version8_3_16 => EnumValue::CompatibilityModeVersion8_3_16,
            Self::Version8_3_17 => EnumValue::CompatibilityModeVersion8_3_17,
            Self::Version8_3_18 => EnumValue::CompatibilityModeVersion8_3_18,
            Self::Version8_3_19 => EnumValue::CompatibilityModeVersion8_3_19,
            Self::Version8_3_20 => EnumValue::CompatibilityModeVersion8_3_20,
            Self::Version8_3_21 => EnumValue::CompatibilityModeVersion8_3_21,
            Self::Version8_3_22 => EnumValue::CompatibilityModeVersion8_3_22,
            Self::Version8_3_23 => EnumValue::CompatibilityModeVersion8_3_23,
            Self::Version8_3_24 => EnumValue::CompatibilityModeVersion8_3_24,
            Self::Version8_3_25 => EnumValue::CompatibilityModeVersion8_3_25,
            Self::Version8_3_26 => EnumValue::CompatibilityModeVersion8_3_26,
            Self::DontUse => EnumValue::CompatibilityModeDontUse,
        })
    }
}
