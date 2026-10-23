//! Приобретённые источники и привязки параметров функциональных опций.

use std::sync::Arc;

use super::{CatalogError, MetadataCatalog, MetadataKind, MetadataObject};

/// Известное место хранения значения функциональной опции.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionalOptionLocation {
    /// Константа с устойчивым идентификатором конфигурации.
    Constant(Arc<str>),
    /// Собственный реквизит справочника.
    CatalogAttribute {
        /// Идентификатор справочника.
        owner: Arc<str>,
        /// Идентификатор собственного реквизита.
        attribute: Arc<str>,
    },
    /// Ресурс регистра сведений.
    InformationRegisterResource {
        /// Идентификатор регистра сведений.
        owner: Arc<str>,
        /// Идентификатор ресурса.
        resource: Arc<str>,
    },
    /// Приобретённое отсутствие источника, отличное от неизвестной формы.
    Absent,
}

/// Привязка параметра к источнику конкретной функциональной опции.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionalOptionParameter {
    /// Каноническое имя параметра функциональных опций.
    pub name: Arc<str>,
    /// Идентификатор справочника либо измерения регистра сведений.
    pub target_id: Arc<str>,
}

/// Неизменяемые сведения для получения значения функциональной опции.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionalOptionSettings {
    /// Источник значения; `None` означает неизвестные сведения.
    pub location: Option<FunctionalOptionLocation>,
    /// Приобретённый флаг привилегированного получения; неизвестность
    /// не разрешает обход проверки прав.
    pub privileged_get_mode: Option<bool>,
    /// Полный список привязок к этому источнику; `None` означает
    /// неизвестный состав, а `Some([])` — проверенное отсутствие параметров.
    pub parameters: Option<Vec<FunctionalOptionParameter>>,
}

impl MetadataObject {
    /// Сохраняет приобретённые настройки функциональной опции.
    /// Описание содержит только метаданные и не хранит состояние сеанса.
    #[must_use]
    pub fn with_functional_option_settings(mut self, settings: FunctionalOptionSettings) -> Self {
        self.functional_option_settings = Some(Arc::new(settings));
        self
    }

    /// Возвращает приобретённые настройки; `None` означает, что описание
    /// функциональной опции не было передано загрузчиком.
    #[must_use]
    pub fn functional_option_settings(&self) -> Option<&FunctionalOptionSettings> {
        self.functional_option_settings.as_deref()
    }
}

impl MetadataCatalog {
    /// Присоединяет приобретённые настройки к существующей функциональной опции.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или объекта другого вида.
    pub fn with_functional_option_settings(
        mut self,
        owner: &str,
        settings: FunctionalOptionSettings,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::FunctionalOption
            })
            .ok_or_else(|| CatalogError::UnknownFunctionalOptionOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].functional_option_settings =
            Some(Arc::new(settings));
        Ok(self)
    }
}
