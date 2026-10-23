//! Ошибки построения и проверки каталога метаданных.

use super::*;

/// Ошибка целостности каталога метаданных.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// Идентичность конфигурации пуста.
    EmptyConfigurationIdentity,
    /// Идентификатор объекта пуст.
    EmptyObjectIdentity,
    /// Имя объекта пусто.
    EmptyObjectName {
        /// Идентификатор объекта.
        object: String,
    },
    /// Идентификатор объекта повторяется.
    DuplicateObjectIdentity {
        /// Повторный идентификатор.
        object: String,
    },
    /// Имя объекта одного вида повторяется.
    DuplicateObjectName {
        /// Повторное имя.
        name: String,
    },
    /// Родитель вложенной подсистемы отсутствует или имеет другой вид.
    UnknownSubsystemParent(String),
    /// Имя связанного модуля менеджера пусто.
    EmptyManagerModule {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя связанного модуля объекта пусто.
    EmptyObjectModule {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя дополнительного свойства пусто.
    EmptyPropertyName {
        /// Идентификатор объекта.
        object: String,
    },
    /// Имя дополнительного свойства повторяется.
    DuplicatePropertyName {
        /// Идентификатор объекта.
        object: String,
        /// Повторное имя.
        property: String,
    },
    /// Имя конфигурации пусто.
    EmptyConfigurationName,
    /// Основная роль отсутствует или ссылается не на роль.
    UnknownMainRole(String),
    /// Основная роль повторяется.
    DuplicateMainRole(String),
    /// Основной язык отсутствует или ссылается не на язык.
    UnknownPrimaryLanguage(String),
    /// Имя расширения пусто.
    EmptyExtensionName,
    /// Идентификатор расширения повторяется.
    DuplicateExtensionIdentity,
    /// Имя расширения повторяется.
    DuplicateExtensionName {
        /// Повторное имя.
        name: String,
    },
    /// Идентификатор или имя значения перечисления пусы.
    InvalidEnumerationValue,
    /// Владелец значения отсутствует или не является перечислением.
    UnknownEnumerationOwner(String),
    /// Владелец реквизитов отсутствует или не является документом.
    UnknownDocumentOwner(String),
    /// Владелец общих реквизитов отсутствует или не является справочником.
    UnknownCatalogOwner(String),
    /// Связи общего реквизита указывают на отсутствующие параметры или неверного владельца.
    InvalidCommonAttributeSeparation(String),
    /// Полный состав общего реквизита содержит неизвестные или неверные связи.
    InvalidCommonAttributeContent(String),
    /// Владелец настроек отсутствует или не является планом видов характеристик.
    UnknownCharacteristicOwner(String),
    /// Владелец настроек отсутствует или не является регистром накопления.
    UnknownAccumulationRegisterOwner(String),
    /// Владелец настроек отсутствует или не является регистром сведений.
    UnknownInformationRegisterOwner(String),
    /// Владелец настроек отсутствует или не является бизнес-процессом.
    UnknownBusinessProcessOwner(String),
    /// Владелец коллекций отсутствует или не является обработкой.
    UnknownDataProcessorOwner(String),
    /// Владелец реквизитов или табличных частей отчёта отсутствует или имеет другой вид.
    UnknownReportOwner(String),
    /// Владелец настроек отсутствует или не является функциональной опцией.
    UnknownFunctionalOptionOwner(String),
    /// Настроенная задача отсутствует или относится к другому виду метаданных.
    InvalidBusinessProcessTask(String),
    /// Некорректный владелец, тип или полный состав точек маршрута.
    InvalidRoutePoints(String),
    /// Имя значения повторяется у одного перечисления.
    DuplicateEnumerationValue {
        /// Имя перечисления.
        owner: String,
        /// Повторившееся имя значения.
        name: String,
    },
    /// Набор ссылается на неизвестное расширение.
    UnknownExtensionIdentity,
    /// Расширение одновременно активно и отключено в сеансе.
    ExtensionBothAppliedAndDisabled,
    /// Проблема ссылается на неизвестное расширение.
    UnknownIssueExtension,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfigurationIdentity => {
                formatter.write_str("идентичность конфигурации пуста")
            }
            Self::EmptyObjectIdentity => formatter.write_str("идентификатор объекта пуст"),
            Self::EmptyObjectName { object } => {
                write!(formatter, "имя объекта `{object}` пусто")
            }
            Self::DuplicateObjectIdentity { object } => {
                write!(formatter, "идентификатор объекта `{object}` повторяется")
            }
            Self::DuplicateObjectName { name } => {
                write!(formatter, "имя объекта `{name}` повторяется")
            }
            Self::UnknownSubsystemParent(parent) => {
                write!(
                    formatter,
                    "родитель подсистемы `{parent}` отсутствует в метаданных"
                )
            }
            Self::EmptyManagerModule { object } => {
                write!(formatter, "имя модуля менеджера объекта `{object}` пусто")
            }
            Self::EmptyObjectModule { object } => {
                write!(formatter, "имя модуля объекта `{object}` пусто")
            }
            Self::EmptyPropertyName { object } => {
                write!(formatter, "имя свойства объекта `{object}` пусто")
            }
            Self::DuplicatePropertyName { object, property } => write!(
                formatter,
                "свойство `{property}` объекта `{object}` повторяется"
            ),
            Self::EmptyConfigurationName => formatter.write_str("имя конфигурации пусто"),
            Self::UnknownMainRole(role) => {
                write!(formatter, "основная роль `{role}` отсутствует в метаданных")
            }
            Self::DuplicateMainRole(role) => {
                write!(formatter, "основная роль `{role}` повторяется")
            }
            Self::UnknownPrimaryLanguage(language) => {
                write!(
                    formatter,
                    "основной язык `{language}` отсутствует в метаданных"
                )
            }
            Self::EmptyExtensionName => formatter.write_str("имя расширения пусто"),
            Self::DuplicateExtensionIdentity => {
                formatter.write_str("идентификатор расширения повторяется")
            }
            Self::DuplicateExtensionName { name } => {
                write!(formatter, "имя расширения `{name}` повторяется")
            }
            Self::InvalidEnumerationValue => {
                formatter.write_str("идентификатор или имя значения перечисления пусы")
            }
            Self::UnknownDocumentOwner(owner) => write!(
                formatter,
                "владелец реквизитов `{owner}` отсутствует или не является документом"
            ),
            Self::InvalidCommonAttributeSeparation(owner) => write!(
                formatter,
                "некорректные связи параметров разделения `{owner}`"
            ),
            Self::InvalidCommonAttributeContent(owner) => write!(
                formatter,
                "некорректный полный состав общего реквизита `{owner}`"
            ),
            Self::UnknownCatalogOwner(owner) => write!(
                formatter,
                "владелец общих реквизитов `{owner}` отсутствует или не является справочником"
            ),
            Self::InvalidRoutePoints(owner) => {
                write!(formatter, "некорректные точки маршрута `{owner}`")
            }
            Self::InvalidBusinessProcessTask(task) => {
                write!(formatter, "некорректная задача бизнес-процесса `{task}`")
            }
            Self::UnknownReportOwner(owner) => write!(
                formatter,
                "владелец метаданных отчёта «{owner}» отсутствует или имеет другой вид"
            ),
            Self::UnknownDataProcessorOwner(owner) => write!(
                formatter,
                "владелец коллекций `{owner}` отсутствует или не является обработкой"
            ),
            Self::UnknownFunctionalOptionOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является функциональной опцией"
            ),
            Self::UnknownBusinessProcessOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является бизнес-процессом"
            ),
            Self::UnknownInformationRegisterOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является регистром сведений"
            ),
            Self::UnknownAccumulationRegisterOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является регистром накопления"
            ),
            Self::UnknownCharacteristicOwner(owner) => write!(
                formatter,
                "владелец настроек `{owner}` отсутствует или не является планом видов характеристик"
            ),
            Self::UnknownEnumerationOwner(owner) => write!(
                formatter,
                "владелец значения перечисления `{owner}` отсутствует или имеет другой вид"
            ),
            Self::DuplicateEnumerationValue { owner, name } => write!(
                formatter,
                "значение `{name}` перечисления `{owner}` повторяется"
            ),
            Self::UnknownExtensionIdentity => {
                formatter.write_str("набор сеанса ссылается на неизвестное расширение")
            }
            Self::ExtensionBothAppliedAndDisabled => {
                formatter.write_str("расширение одновременно активно и отключено в текущем сеансе")
            }
            Self::UnknownIssueExtension => {
                formatter.write_str("проблема ссылается на неизвестное расширение")
            }
        }
    }
}

impl Error for CatalogError {}
