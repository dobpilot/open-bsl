//! Контекст получателя запрашивается вместе с телом выбранного модуля.

#[cfg(test)]
#[path = "source_context_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "source_context_lookup_tests.rs"]
mod lookup_tests;

use super::*;

#[derive(Debug)]
pub(super) struct SessionModuleContext {
    index: open_bsl::ConfigurationIndex,
    catalog: Arc<MetadataCatalog>,
    module_positions: std::collections::HashMap<String, Vec<usize>>,
}

impl SessionModuleContext {
    pub(super) fn new(index: open_bsl::ConfigurationIndex, catalog: Arc<MetadataCatalog>) -> Self {
        let mut module_positions = std::collections::HashMap::<String, Vec<usize>>::new();
        for (position, module) in index.modules().iter().enumerate() {
            module_positions
                .entry(module.name().to_uppercase())
                .or_default()
                .push(position);
        }
        Self {
            index,
            catalog,
            module_positions,
        }
    }
}

impl open_bsl::ModuleContextSource for SessionModuleContext {
    fn prepare_contexts(&self, modules: &[&str]) -> Result<(), String> {
        // Владельцы подготавливаются в порядке индекса, как при прежнем полном
        // обходе. Поиск группы не сравнивает каждое имя с каждым модулем каталога.
        let mut positions = modules
            .iter()
            .filter_map(|name| self.module_positions.get(&name.to_uppercase()))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();
        let mut seen = std::collections::HashSet::new();
        let owners = positions
            .into_iter()
            .filter_map(|position| self.index.modules()[position].owner())
            .filter(|owner| seen.insert(*owner))
            .collect::<Vec<_>>();
        if !owners.is_empty() {
            self.catalog.prepare_objects(&owners)?;
        }
        Ok(())
    }

    fn receiver_properties(&self, module: &str) -> Result<Vec<String>, String> {
        let Some(declaration) = self
            .module_positions
            .get(&module.to_uppercase())
            .and_then(|positions| positions.first())
            .map(|position| &self.index.modules()[*position])
        else {
            return Err(format!("модуль «{module}» отсутствует в индексе"));
        };
        let Some(owner) = declaration.owner() else {
            return Ok(Vec::new());
        };
        let object = self
            .catalog
            .materialized_object_by_id(owner)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("владелец модуля «{module}» отсутствует в каталоге"))?;
        if declaration.kind() == ModuleKind::Manager {
            return Ok(object.catalog_predefined_names().to_vec());
        }
        let static_properties = match object.kind() {
            MetadataKind::Catalog => bsl_orm::catalog_object_receiver_properties(),
            MetadataKind::Document => bsl_orm::document_object_receiver_properties(),
            MetadataKind::BusinessProcess => bsl_orm::business_process_object_receiver_properties(),
            MetadataKind::ChartOfCharacteristicTypes => {
                bsl_orm::characteristic_object_receiver_properties()
            }
            MetadataKind::Report => bsl_orm::report_object_receiver_properties(),
            _ => &[],
        };
        let mut names = static_properties
            .iter()
            .flat_map(|property| property.names.iter().copied())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let attributes = match object.kind() {
            MetadataKind::Catalog => object.catalog_attribute_names(),
            _ => None,
        };
        names.extend(attributes.into_iter().flatten().cloned());
        names.extend(
            object
                .catalog_common_attributes()
                .into_iter()
                .flatten()
                .map(|attribute| attribute.name().to_owned()),
        );
        names.extend(
            object
                .catalog_tabular_section_names()
                .into_iter()
                .flatten()
                .cloned(),
        );
        for attributes in [
            object.document_attributes(),
            object.business_process_attributes(),
            object.characteristic_attributes(),
            object.data_processor_attributes(),
            object.report_attributes(),
        ] {
            names.extend(
                attributes
                    .into_iter()
                    .flatten()
                    .map(|attribute| attribute.name().to_owned()),
            );
        }
        for sections in [
            object.document_tabular_sections(),
            object.business_process_tabular_sections(),
            object.data_processor_tabular_sections(),
            object.report_tabular_sections(),
        ] {
            names.extend(
                sections
                    .into_iter()
                    .flatten()
                    .map(|section| section.name().to_owned()),
            );
        }
        Ok(names)
    }
}
