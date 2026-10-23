//! Приобретённые настройки объектов бизнес-процесса без состояния сеанса.
use super::*;

/// Настройки номера бизнес-процесса, явно переданные хостом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusinessProcessNumberSettings {
    /// Строковый или числовой тип номера; используется существующее описание.
    pub number_type: DocumentNumberType,
    /// Настроенная длина номера.
    pub length: u32,
    /// Разрешена переменная длина номера.
    pub variable_length: bool,
}

impl MetadataCatalog {
    /// Присоединяет явно приобретённую связь бизнес-процесса с задачей.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего бизнес-процесса, задачи или неверного вида метаданных.
    pub fn with_business_process_task(
        mut self,
        owner: &str,
        task: &str,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::BusinessProcess
            })
            .ok_or_else(|| CatalogError::UnknownBusinessProcessOwner(owner.into()))?;
        if !self
            .object_by_id(task)
            .is_some_and(|object| object.kind() == MetadataKind::Task)
        {
            return Err(CatalogError::InvalidBusinessProcessTask(task.into()));
        }
        Arc::make_mut(&mut self.objects)[index].business_process_task = Some(task.into());
        Ok(self)
    }

    /// Присоединяет полную политику проверки стандартных реквизитов бизнес-процесса.
    /// Пустой список означает приобретённую пустую политику. Используется
    /// существующее описание именованной настройки стандартного реквизита.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_business_process_standard_fill_checking(
        mut self,
        owner: &str,
        settings: Vec<CatalogStandardAttributeFillChecking>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::BusinessProcess
            })
            .ok_or_else(|| CatalogError::UnknownBusinessProcessOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].business_process_standard_fill_checking =
            Some(settings.into());
        Ok(self)
    }

    /// Присоединяет настройки и полные упорядоченные коллекции бизнес-процесса.
    /// `None` означает неизвестность, пустой список — приобретённую пустоту.
    /// Описания реквизитов и частей общие с другими объектами конфигурации.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или другого вида объекта.
    pub fn with_business_process_metadata(
        mut self,
        owner: &str,
        number: Option<BusinessProcessNumberSettings>,
        attributes: Option<Vec<DocumentAttribute>>,
        sections: Option<Vec<DocumentTabularSection>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::BusinessProcess
            })
            .ok_or_else(|| CatalogError::UnknownBusinessProcessOwner(owner.into()))?;
        let object = &mut Arc::make_mut(&mut self.objects)[index];
        object.business_process_number = number;
        object.business_process_attributes = attributes.map(Into::into);
        object.business_process_sections = sections.map(Into::into);
        document_tabular_section::update_types(
            &self.identity,
            &self.objects,
            index,
            &mut self.business_section_types,
        );
        Ok(self)
    }
}

impl MetadataObject {
    /// Возвращает идентификатор настроенной задачи; `None` означает неизвестную связь.
    #[must_use]
    pub fn business_process_task(&self) -> Option<&str> {
        self.business_process_task.as_deref()
    }

    /// Возвращает полную политику стандартных реквизитов в порядке конфигурации.
    /// `None` означает неизвестность; пустой список — приобретённую пустоту.
    #[must_use]
    pub fn business_process_standard_fill_checking(
        &self,
    ) -> Option<&[CatalogStandardAttributeFillChecking]> {
        self.business_process_standard_fill_checking.as_deref()
    }

    /// Возвращает настройки номера; `None` означает неизвестность.
    #[must_use]
    pub const fn business_process_number_settings(&self) -> Option<BusinessProcessNumberSettings> {
        self.business_process_number
    }
    /// Возвращает полные собственные реквизиты в порядке конфигурации.
    #[must_use]
    pub fn business_process_attributes(&self) -> Option<&[DocumentAttribute]> {
        self.business_process_attributes.as_deref()
    }
    /// Возвращает полные собственные табличные части в порядке конфигурации.
    #[must_use]
    pub fn business_process_tabular_sections(&self) -> Option<&[DocumentTabularSection]> {
        self.business_process_sections.as_deref()
    }
}

#[cfg(test)]
#[path = "business_process_metadata_tests.rs"]
mod tests;
