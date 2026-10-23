//! Приобретённые настройки регистра накопления без обращения к базе данных.

use super::*;

/// Вид регистра накопления, известный из метаданных конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccumulationRegisterKind {
    /// Регистр хранит движения для расчёта остатков.
    Balance,
    /// Регистр хранит движения для расчёта оборотов.
    Turnovers,
}

impl MetadataCatalog {
    /// Присоединяет приобретённый вид и полный упорядоченный состав полей.
    /// `None` сохраняет неизвестность; пустая коллекция означает известную пустоту.
    /// Для полей используется то же описание, что для реквизитов каталога:
    /// оно хранит идентификатор, имя и типы без состояния живого сеанса.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или объекта другого вида.
    pub fn with_accumulation_register_metadata(
        mut self,
        owner: &str,
        kind: Option<AccumulationRegisterKind>,
        dimensions: Option<Vec<DocumentAttribute>>,
        resources: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::AccumulationRegister
            })
            .ok_or_else(|| CatalogError::UnknownAccumulationRegisterOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.accumulation_register_kind = kind;
        object.accumulation_dimensions = dimensions.map(Into::into);
        object.accumulation_resources = resources.map(Into::into);
        Ok(self)
    }
}

impl MetadataObject {
    /// Возвращает приобретённый вид регистра; `None` означает неизвестность.
    #[must_use]
    pub const fn accumulation_register_kind(&self) -> Option<AccumulationRegisterKind> {
        self.accumulation_register_kind
    }

    /// Возвращает полный состав измерений в порядке конфигурации.
    /// `None` отличается от известной пустой коллекции.
    #[must_use]
    pub fn accumulation_register_dimensions(&self) -> Option<&[DocumentAttribute]> {
        self.accumulation_dimensions.as_deref()
    }

    /// Возвращает полный состав ресурсов в порядке конфигурации.
    /// `None` отличается от известной пустой коллекции.
    #[must_use]
    pub fn accumulation_register_resources(&self) -> Option<&[DocumentAttribute]> {
        self.accumulation_resources.as_deref()
    }
}

#[cfg(test)]
#[path = "accumulation_metadata_tests.rs"]
mod tests;
