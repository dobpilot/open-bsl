//! Проекция приобретённых реквизитов объектов без повторного декодирования Config.

use super::{LoadError, malformed, scalar};
use bsl_orm::{DocumentAttribute, DocumentTabularSection, MetadataCatalog, MetadataKind};
use open_bsl::{
    ColumnType, Engine, EnumValue, GraphLimits, NumberFormat, RuntimeShapes, SerializedValueGraph,
    TypeId, TypeRef, ValueTypeDescriptor,
};
#[cfg(test)]
use open_sdbl::metadata::MetadataSnapshot;
use open_sdbl::metadata::{
    CommonAttributeConditionalSeparation, CommonAttributeContentUse, CommonAttributeUse,
    ConfigDeclaredType, ConfigDescriptor, ConfigFillingValue, RuntimeMetadataFacts,
};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[cfg(test)]
fn head_task_target<'a>(metadata: &'a MetadataSnapshot, owner: &str) -> Option<&'a str> {
    head_task_runtime_target(metadata.runtime_facts(), owner)
}

fn head_task_runtime_target<'a>(
    metadata: &'a RuntimeMetadataFacts,
    owner: &str,
) -> Option<&'a str> {
    let process = metadata.objects().iter().find(|object| {
        object.guid.as_str() == owner
            && object.kind == Some(open_sdbl::metadata::MetadataKind::BusinessProcess)
    })?;
    let physical = process.physical_table.as_deref()?;
    let mut tables = metadata
        .schema()
        .tables
        .iter()
        .filter(|table| table.physical_name().eq_ignore_ascii_case(physical));
    let table = tables.next()?;
    if tables.next().is_some() {
        return None;
    }
    let mut columns = table
        .columns
        .iter()
        .filter(|column| column.name.eq_ignore_ascii_case("HeadTask"));
    let column = columns.next()?;
    if columns.next().is_some() {
        return None;
    }
    let [ty] = column.types.as_slice() else {
        return None;
    };
    if ty.tag != "R" {
        return None;
    }
    let target = ty.reference_target.as_deref()?;
    let mut tasks = metadata.objects().iter().filter(|object| {
        object.kind == Some(open_sdbl::metadata::MetadataKind::Task)
            && object.physical_table.as_deref().is_some_and(|name| {
                name.strip_prefix('_')
                    .unwrap_or(name)
                    .eq_ignore_ascii_case(target)
            })
    });
    let task = tasks.next()?;
    if tasks.next().is_some() {
        return None;
    }
    Some(task.guid.as_str())
}

pub(super) type ReferenceFamilies =
    Arc<[(open_sdbl::metadata::MetadataKind, Option<Vec<ColumnType>>)]>;

struct References<'a> {
    by_guid: HashMap<&'a str, (MetadataKind, Arc<ValueTypeDescriptor>)>,
    families: ReferenceFamilies,
}

impl<'a> From<HashMap<&'a str, (MetadataKind, Arc<ValueTypeDescriptor>)>> for References<'a> {
    fn from(by_guid: HashMap<&'a str, (MetadataKind, Arc<ValueTypeDescriptor>)>) -> Self {
        Self {
            by_guid,
            families: Vec::new().into(),
        }
    }
}

fn reference_kind(kind: MetadataKind) -> Option<(open_sdbl::metadata::MetadataKind, &'static str)> {
    use open_sdbl::metadata::MetadataKind as DbKind;
    Some(match kind {
        MetadataKind::Catalog => (DbKind::Catalog, "CatalogRef"),
        MetadataKind::Document => (DbKind::Document, "DocumentRef"),
        MetadataKind::Enumeration => (DbKind::Enumeration, "EnumRef"),
        MetadataKind::ChartOfCharacteristicTypes => (
            DbKind::ChartOfCharacteristicTypes,
            "ChartOfCharacteristicTypesRef",
        ),
        MetadataKind::ChartOfCalculationTypes => (
            DbKind::ChartOfCalculationTypes,
            "ChartOfCalculationTypesRef",
        ),
        MetadataKind::ChartOfAccounts => (DbKind::ChartOfAccounts, "ChartOfAccountsRef"),
        MetadataKind::ExchangePlan => (DbKind::ExchangePlan, "ExchangePlanRef"),
        MetadataKind::BusinessProcess => (DbKind::BusinessProcess, "BusinessProcessRef"),
        MetadataKind::Task => (DbKind::Task, "TaskRef"),
        _ => return None,
    })
}

impl<'a> References<'a> {
    fn new() -> Self {
        HashMap::new().into()
    }
    fn for_catalog(
        catalog: &MetadataCatalog,
        types: &HashMap<&str, Arc<ValueTypeDescriptor>>,
    ) -> Self {
        let mut families: Vec<(open_sdbl::metadata::MetadataKind, Option<Vec<ColumnType>>)> =
            Vec::new();
        // Индекс владельцев полон уже при создании каталога. Достигнутые
        // описатели lazy-снимка не доказывают полноту семейства ссылок.
        for object in catalog.objects() {
            let Some((kind, prefix)) = reference_kind(object.kind()) else {
                continue;
            };
            let position =
                if let Some(index) = families.iter().position(|(known, _)| *known == kind) {
                    index
                } else {
                    families.push((kind, Some(Vec::new())));
                    families.len() - 1
                };
            let Some(ty) = types.get(format!("{prefix}.{}", object.name()).as_str()) else {
                families[position].1 = None;
                continue;
            };
            if let Some(members) = &mut families[position].1 {
                members.push(ColumnType::plain(TypeRef::Value(Arc::clone(ty))));
            }
        }
        let mut references = Self::new();
        references.families = families.into();
        references
    }
    fn insert(&mut self, guid: &'a str, value: (MetadataKind, Arc<ValueTypeDescriptor>)) {
        self.by_guid.insert(guid, value);
    }
    fn get(&self, guid: &str) -> Option<&(MetadataKind, Arc<ValueTypeDescriptor>)> {
        self.by_guid.get(guid)
    }
    fn columns(&self, types: &[ConfigDeclaredType]) -> Option<Vec<ColumnType>> {
        let mut columns = Vec::new();
        let mut seen = HashSet::new();
        for ty in types {
            let members = if let Some(family) = ty.reference_type_set() {
                let mut members = Vec::new();
                for (kind, types) in self.families.iter() {
                    if family.contains_kind(*kind) {
                        members.extend(types.as_ref()?.iter().cloned());
                    }
                }
                members
            } else {
                vec![column(ty, self)?]
            };
            for member in members {
                if seen.insert((member.id.clone(), member.quals.clone())) {
                    columns.push(member);
                }
            }
        }
        Some(columns)
    }
}

pub(super) fn reference_families(catalog: &MetadataCatalog) -> ReferenceFamilies {
    let types = catalog
        .value_types()
        .flat_map(|ty| {
            ty.names()
                .iter()
                .map(move |name| (name.as_str(), Arc::clone(ty)))
        })
        .collect();
    References::for_catalog(catalog, &types).families
}

#[cfg(test)]
pub(super) fn project(
    catalog: MetadataCatalog,
    metadata: &MetadataSnapshot,
) -> Result<MetadataCatalog, LoadError> {
    project_runtime(catalog, metadata.runtime_facts(), false)
}

#[cfg(test)]
pub(super) fn project_selected(
    catalog: MetadataCatalog,
    metadata: &MetadataSnapshot,
) -> Result<MetadataCatalog, LoadError> {
    project_runtime(catalog, metadata.runtime_facts(), true)
}

#[cfg(test)]
pub(super) fn project_runtime(
    catalog: MetadataCatalog,
    metadata: &RuntimeMetadataFacts,
    selective: bool,
) -> Result<MetadataCatalog, LoadError> {
    project_runtime_with_references(catalog, metadata, selective, None)
}

pub(super) fn project_runtime_with_references(
    catalog: MetadataCatalog,
    metadata: &RuntimeMetadataFacts,
    selective: bool,
    reference_families: Option<&ReferenceFamilies>,
) -> Result<MetadataCatalog, LoadError> {
    let catalog = Arc::new(catalog);
    let (
        attributes,
        common_attributes,
        catalog_attributes,
        characteristics,
        registers,
        information_registers,
        separation_parameters,
        content,
        declared_objects,
    ) = {
        let descriptors: HashMap<_, _> = metadata
            .descriptors()
            .iter()
            .map(|descriptor| (descriptor.object_guid.as_str(), descriptor))
            .collect();
        let types: HashMap<_, _> = catalog
            .value_types()
            .chain(catalog.data_processor_object_types())
            .chain(catalog.report_object_types())
            .flat_map(|ty| {
                ty.names()
                    .iter()
                    .map(move |name| (name.as_str(), Arc::clone(ty)))
            })
            .collect();
        let mut references = if let Some(families) = reference_families {
            let mut references = References::new();
            references.families = Arc::clone(families);
            references
        } else {
            References::for_catalog(&catalog, &types)
        };
        for descriptor in metadata.descriptors() {
            let Some(guid) = &descriptor.object_reference_type else {
                continue;
            };
            let Some(owner) = catalog.object_by_id(descriptor.object_guid.as_str()) else {
                continue;
            };
            let Some((_, prefix)) = reference_kind(owner.kind()) else {
                continue;
            };
            if let Some(ty) = types.get(format!("{prefix}.{}", owner.name()).as_str()) {
                references.insert(guid.as_str(), (owner.kind(), Arc::clone(ty)));
            }
        }
        // Типы объектов отчётов и обработок известны каталогу, но не имеют декодера состояния.
        // GUID берём из SDK, а не из имени ресурса или имени самой обработки.
        for descriptor in metadata.descriptors() {
            let Some(owner) = catalog.object_by_id(descriptor.object_guid.as_str()) else {
                continue;
            };
            let (guid, prefix) = match owner.kind() {
                MetadataKind::DataProcessor => (
                    &descriptor.data_processor_object_type,
                    "DataProcessorObject",
                ),
                MetadataKind::Report => (&descriptor.report_object_type, "ReportObject"),
                _ => continue,
            };
            let Some(guid) = guid else {
                continue;
            };
            if let Some(ty) = types.get(format!("{prefix}.{}", owner.name()).as_str()) {
                references.insert(guid.as_str(), (owner.kind(), Arc::clone(ty)));
            }
        }
        let declared_objects = descriptors
            .values()
            .filter_map(|descriptor| {
                let owner = catalog.object_by_id(descriptor.object_guid.as_str())?;
                if !matches!(
                    owner.kind(),
                    MetadataKind::Constant
                        | MetadataKind::SessionParameter
                        | MetadataKind::CommonAttribute
                ) {
                    return None;
                }
                let types = references.columns(descriptor.declared_types.as_ref()?)?;
                Some((owner.id().to_owned(), types))
            })
            .collect::<Vec<_>>();
        let mut builder = Engine::builder().register_library(bsl_orm::library());
        // Реестр нужен здесь только для сериализации значений заполнения.
        // Объявления типов колонок уже содержатся в `references` и не требуют
        // повторной регистрации всех типов обнаруженного каталога.
        let filling_types = descriptors
            .values()
            .filter_map(|descriptor| {
                let ConfigFillingValue::Reference { type_id, .. } =
                    descriptor.filling_value.as_ref()?
                else {
                    return None;
                };
                references
                    .get(type_id.as_str())
                    .map(|(_, ty)| ty.identity())
            })
            .collect::<HashSet<_>>();
        for ty in catalog.transferable_value_types() {
            if !selective || filling_types.contains(ty.identity()) {
                builder = builder.register_value_type(ty);
            }
        }
        let engine = builder
            .build()
            .map_err(|error| malformed("document attributes", error))?;
        let shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
        let mut result = Vec::new();
        for owner in catalog.objects().iter().filter(|owner| {
            matches!(
                owner.kind(),
                MetadataKind::Document
                    | MetadataKind::BusinessProcess
                    | MetadataKind::DataProcessor
                    | MetadataKind::Report
                    | MetadataKind::ExchangePlan
            )
        }) {
            let Some(descriptor) = descriptors.get(owner.id()) else {
                continue;
            };
            let attribute_ids = match owner.kind() {
                MetadataKind::BusinessProcess => &descriptor.business_process_attributes,
                MetadataKind::DataProcessor => &descriptor.data_processor_attributes,
                MetadataKind::Report => &descriptor.report_attributes,
                MetadataKind::ExchangePlan => &descriptor.exchange_plan_attributes,
                _ => &descriptor.document_attributes,
            };
            let section_ids = match owner.kind() {
                MetadataKind::BusinessProcess => &descriptor.business_process_tabular_sections,
                MetadataKind::DataProcessor => &descriptor.data_processor_tabular_sections,
                MetadataKind::Report => &descriptor.report_tabular_sections,
                MetadataKind::ExchangePlan => &None,
                _ => &descriptor.document_tabular_sections,
            };
            let attributes = attribute_ids
                .as_ref()
                .map(|ids| {
                    ids.iter()
                        .map(|id| {
                            let descriptor = descriptors.get(id.as_str()).ok_or_else(|| {
                                malformed(id.as_str(), "нет описателя реквизита документа")
                            })?;
                            attribute(descriptor, &references, &engine, &shapes)
                        })
                        .collect::<Result<Vec<_>, LoadError>>()
                })
                .transpose()?;
            let sections = section_ids
                .as_ref()
                .map(|ids| {
                    ids.iter()
                        .map(|id| {
                            let section = descriptors.get(id.as_str()).ok_or_else(|| {
                                malformed(id.as_str(), "нет описателя табличной части документа")
                            })?;
                            let mut projected =
                                DocumentTabularSection::new(id.as_str(), section.name.as_str());
                            // Членство и порядок принадлежат самой части.
                            // Неизвестная коллекция не подменяется пустой.
                            if let Some(fields) = &section.tabular_section_attributes {
                                let attributes = fields
                                    .iter()
                                    .map(|field| {
                                        let descriptor =
                                            descriptors.get(field.as_str()).ok_or_else(|| {
                                                malformed(
                                                    field.as_str(),
                                                    "нет описателя реквизита табличной части",
                                                )
                                            })?;
                                        attribute(descriptor, &references, &engine, &shapes)
                                    })
                                    .collect::<Result<Vec<_>, LoadError>>()?;
                                projected = projected.with_attributes(attributes);
                            }
                            Ok(projected)
                        })
                        .collect::<Result<Vec<_>, LoadError>>()
                })
                .transpose()?;
            result.push((
                owner.id().to_owned(),
                owner.kind(),
                *descriptor,
                attributes,
                sections,
            ));
        }
        let common_attributes = catalog
            .common_attribute_ids()
            .map(|ids| {
                // Список берётся из корня, а не фильтра известного usage:
                // неизвестный описатель не должен молча исчезнуть из состава.
                let common = ids
                    .iter()
                    .map(|id| {
                        let descriptor = descriptors
                            .get(id.as_str())
                            .ok_or_else(|| malformed(id, "нет описателя общего реквизита"))?;
                        let usage =
                            descriptor.common_attribute_usage.as_ref().ok_or_else(|| {
                                malformed(id, "неизвестно использование общего реквизита")
                            })?;
                        Ok((*descriptor, usage))
                    })
                    .collect::<Result<Vec<_>, LoadError>>()?;
                catalog
                    .objects()
                    .iter()
                    .filter(|owner| {
                        owner.kind() == MetadataKind::Catalog
                            && (!selective || descriptors.contains_key(owner.id()))
                    })
                    .map(|owner| {
                        let attributes = common
                            .iter()
                            .filter(|(_, usage)| {
                                usage
                                    .content
                                    .iter()
                                    .find(|entry| entry.object.as_str() == owner.id())
                                    .map_or(usage.auto_use == CommonAttributeUse::Use, |entry| {
                                        match entry.usage {
                                            CommonAttributeContentUse::Auto => {
                                                usage.auto_use == CommonAttributeUse::Use
                                            }
                                            CommonAttributeContentUse::Use => true,
                                            CommonAttributeContentUse::DontUse => false,
                                        }
                                    })
                            })
                            .map(|(descriptor, _)| {
                                attribute(descriptor, &references, &engine, &shapes)
                            })
                            .collect::<Result<Vec<_>, LoadError>>()?;
                        Ok((owner.id().to_owned(), attributes))
                    })
                    .collect::<Result<Vec<_>, LoadError>>()
            })
            .transpose()?;
        let catalog_attributes = catalog
            .objects()
            .iter()
            .filter(|owner| {
                owner.kind() == MetadataKind::Catalog
                    && (!selective || descriptors.contains_key(owner.id()))
            })
            .map(|owner| {
                let attributes = descriptors
                    .get(owner.id())
                    .and_then(|descriptor| descriptor.catalog_attributes.as_ref())
                    .map(|ids| {
                        ids.iter()
                            .map(|id| {
                                let descriptor = descriptors.get(id.as_str()).ok_or_else(|| {
                                    malformed(
                                        id.as_str(),
                                        "нет описателя собственного реквизита справочника",
                                    )
                                })?;
                                attribute(descriptor, &references, &engine, &shapes)
                            })
                            .collect::<Result<Vec<_>, LoadError>>()
                    })
                    .transpose()?;
                Ok((owner.id().to_owned(), attributes))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        let characteristics = catalog
            .objects()
            .iter()
            .filter(|owner| {
                owner.kind() == MetadataKind::ChartOfCharacteristicTypes
                    && (!selective || descriptors.contains_key(owner.id()))
            })
            .map(|owner| {
                let descriptor = descriptors.get(owner.id());
                let settings = descriptor
                    .and_then(|descriptor| descriptor.characteristic_settings.as_ref())
                    .map(|settings| {
                        let value_types =
                            references.columns(&settings.value_types).ok_or_else(|| {
                                malformed(owner.id(), "неизвестен тип значения характеристики")
                            })?;
                        Ok::<_, LoadError>(bsl_orm::CharacteristicSettings {
                            code_length: settings.code_length,
                            description_length: settings.description_length,
                            value_types,
                        })
                    })
                    .transpose()?;
                let attributes = descriptor
                    .and_then(|descriptor| descriptor.characteristic_attributes.as_ref())
                    .map(|ids| {
                        ids.iter()
                            .map(|id| {
                                let descriptor = descriptors.get(id.as_str()).ok_or_else(|| {
                                    malformed(id.as_str(), "нет описателя реквизита характеристики")
                                })?;
                                attribute(descriptor, &references, &engine, &shapes)
                            })
                            .collect::<Result<Vec<_>, LoadError>>()
                    })
                    .transpose()?;
                Ok((owner.id().to_owned(), settings, attributes))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        let project_fields = |ids: &Option<Vec<open_sdbl::metadata::Guid>>, missing: &str| {
            ids.as_ref()
                .map(|ids| {
                    ids.iter()
                        .map(|id| {
                            let descriptor = descriptors
                                .get(id.as_str())
                                .ok_or_else(|| malformed(id.as_str(), missing))?;
                            attribute(descriptor, &references, &engine, &shapes)
                        })
                        .collect::<Result<Vec<_>, LoadError>>()
                })
                .transpose()
        };
        let registers = catalog
            .objects()
            .iter()
            .filter(|owner| owner.kind() == MetadataKind::AccumulationRegister)
            .filter_map(|owner| {
                descriptors
                    .get(owner.id())
                    .map(|descriptor| (owner, *descriptor))
            })
            .map(|(owner, descriptor)| {
                Ok((
                    owner.id().to_owned(),
                    descriptor.accumulation_register_kind,
                    project_fields(
                        &descriptor.accumulation_register_dimensions,
                        "нет описателя поля регистра накопления",
                    )?,
                    project_fields(
                        &descriptor.accumulation_register_resources,
                        "нет описателя поля регистра накопления",
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        let information_registers = catalog
            .objects()
            .iter()
            .filter(|owner| owner.kind() == MetadataKind::InformationRegister)
            .filter_map(|owner| {
                descriptors
                    .get(owner.id())
                    .map(|descriptor| (owner, *descriptor))
            })
            .filter(|(_, descriptor)| {
                descriptor.information_register_write_mode
                    == Some(open_sdbl::metadata::InformationRegisterWriteMode::Independent)
            })
            .map(|(owner, descriptor)| {
                let periodicity =
                    descriptor
                        .information_register_periodicity
                        .map(|period| match period {
                            open_sdbl::metadata::InformationRegisterPeriodicity::Nonperiodical => {
                                bsl_orm::InformationRegisterPeriodicity::Nonperiodic
                            }
                            open_sdbl::metadata::InformationRegisterPeriodicity::Day => {
                                bsl_orm::InformationRegisterPeriodicity::Day
                            }
                        });
                Ok((
                    owner.id().to_owned(),
                    periodicity,
                    project_fields(
                        &descriptor.information_register_dimensions,
                        "нет описателя поля регистра сведений",
                    )?,
                    project_fields(
                        &descriptor.information_register_resources,
                        "нет описателя поля регистра сведений",
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, LoadError>>()?;
        let content = metadata
            .descriptors()
            .iter()
            .filter_map(|descriptor| {
                let owner = catalog.object_by_id(descriptor.object_guid.as_str())?;
                if owner.kind() != MetadataKind::CommonAttribute {
                    return None;
                }
                // Полноту доказывает SDK; непредставимый участник оставляет весь состав неизвестным.
                let entries = descriptor
                    .common_attribute_content
                    .as_ref()?
                    .iter()
                    .map(|entry| {
                        catalog.object_by_id(entry.object.as_str())?;
                        let conditional_separation = match &entry.conditional_separation {
                            Some(CommonAttributeConditionalSeparation::Undefined) => {
                                Some(bsl_orm::CommonAttributeCondition::Undefined)
                            }
                            Some(CommonAttributeConditionalSeparation::Constant(id)) => catalog
                                .object_by_id(id.as_str())
                                .filter(|object| object.kind() == MetadataKind::Constant)
                                .map(|_| {
                                    bsl_orm::CommonAttributeCondition::Constant(id.to_string())
                                }),
                            None => None,
                        };
                        Some(bsl_orm::CommonAttributeContentEntry {
                            metadata_id: entry.object.to_string(),
                            usage: match entry.usage {
                                CommonAttributeContentUse::Auto => {
                                    EnumValue::CommonAttributeUseAuto
                                }
                                CommonAttributeContentUse::Use => EnumValue::CommonAttributeUseUse,
                                CommonAttributeContentUse::DontUse => {
                                    EnumValue::CommonAttributeUseDontUse
                                }
                            },
                            conditional_separation,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some((owner.id().to_owned(), entries))
            })
            .collect::<Vec<_>>();
        let separation_parameters = metadata
            .separators()
            .filter_map(|field| {
                let descriptor = descriptors.get(field.guid.as_str())?;
                let settings = descriptor.separation.as_ref()?;
                if !catalog
                    .object_by_id(field.guid.as_str())
                    .is_some_and(|object| object.kind() == MetadataKind::CommonAttribute)
                {
                    return None;
                }
                let value_types = descriptor
                    .declared_types
                    .as_ref()
                    .and_then(|types| references.columns(types));
                Some((
                    field.guid.to_string(),
                    bsl_orm::CommonAttributeSeparationParameters {
                        value_parameter: settings.value_parameter.as_ref().map(ToString::to_string),
                        use_parameter: settings.use_parameter.as_ref().map(ToString::to_string),
                        value_types,
                    },
                ))
            })
            .collect::<Vec<_>>();
        (
            result,
            common_attributes,
            catalog_attributes,
            characteristics,
            registers,
            information_registers,
            separation_parameters,
            content,
            declared_objects,
        )
    };
    // Материализаторы временного движка отпущены; графы содержат только данные.
    let mut catalog = Arc::try_unwrap(catalog).map_err(|_| {
        malformed(
            "document attributes",
            "временный каталог остался связан с материализатором",
        )
    })?;
    for (owner, types) in declared_objects {
        catalog = catalog
            .with_object_declared_types(&owner, types)
            .map_err(|error| malformed(&owner, error))?;
    }
    for (owner, kind, descriptor, attributes, sections) in attributes {
        if kind == MetadataKind::ExchangePlan {
            catalog = catalog
                .with_exchange_plan_attributes(&owner, attributes)
                .map_err(|error| malformed(&owner, error))?;
            continue;
        }
        if kind == MetadataKind::Report {
            catalog = catalog
                .with_report_metadata(&owner, attributes, sections)
                .map_err(LoadError::Catalog)?;
            continue;
        }
        if kind == MetadataKind::DataProcessor {
            catalog = catalog
                .with_data_processor_metadata(&owner, attributes, sections)
                .map_err(LoadError::Catalog)?;
            continue;
        }
        if kind == MetadataKind::BusinessProcess {
            let number = descriptor.business_process_number.map(|settings| {
                bsl_orm::BusinessProcessNumberSettings {
                    number_type: match settings.number_type {
                        open_sdbl::metadata::CatalogCodeType::String => {
                            bsl_orm::DocumentNumberType::String
                        }
                        open_sdbl::metadata::CatalogCodeType::Number => {
                            bsl_orm::DocumentNumberType::Number
                        }
                    },
                    length: settings.length,
                    variable_length: settings.variable_length,
                }
            });
            catalog = catalog
                .with_business_process_metadata(&owner, number, attributes, sections)
                .map_err(LoadError::Catalog)?;
            if let Some(policy) = &descriptor.business_process_standard_fill_checking {
                catalog = catalog
                    .with_business_process_standard_fill_checking(
                        &owner,
                        policy
                            .iter()
                            .map(|setting| bsl_orm::CatalogStandardAttributeFillChecking {
                                name: setting.name.clone(),
                                fill_checking: super::fill_checking(setting.fill_checking),
                            })
                            .collect(),
                    )
                    .map_err(LoadError::Catalog)?;
            }
            if let Some(task) = head_task_runtime_target(metadata, &owner)
                && catalog
                    .object_by_id(task)
                    .is_some_and(|object| object.kind() == MetadataKind::Task)
            {
                catalog = catalog
                    .with_business_process_task(&owner, task)
                    .map_err(LoadError::Catalog)?;
            }
            continue;
        }
        if let Some(attributes) = attributes {
            catalog = catalog
                .with_document_attributes(&owner, attributes)
                .map_err(LoadError::Catalog)?;
        }
        if let Some(sections) = sections {
            catalog = catalog
                .with_document_tabular_sections(&owner, sections)
                .map_err(LoadError::Catalog)?;
        }
    }
    // Неизвестная ссылка или тип остаются неизвестными. Только полное
    // приобретённое описание отличает Boolean-константу от отсутствия условия.
    let conditions = catalog
        .objects()
        .iter()
        .filter(|object| object.kind() == MetadataKind::CommonAttribute)
        .filter_map(|object| {
            let descriptor = metadata
                .descriptors()
                .iter()
                .find(|descriptor| descriptor.object_guid.as_str() == object.id())?;
            let settings = descriptor.separation.as_ref()?;
            let condition = match &settings.conditional_separation {
                None => bsl_orm::CommonAttributeCondition::Undefined,
                Some(id) => {
                    if !catalog
                        .object_by_id(id.as_str())
                        .is_some_and(|target| target.kind() == MetadataKind::Constant)
                        || !metadata.descriptors().iter().any(|target| {
                            target.object_guid == *id
                                && target
                                    .declared_types
                                    .as_ref()
                                    .is_some_and(|types| types.len() == 1 && types[0].tag == "B")
                        })
                    {
                        return None;
                    }
                    bsl_orm::CommonAttributeCondition::Constant(id.to_string())
                }
            };
            Some((object.id().to_owned(), condition))
        })
        .collect::<Vec<_>>();
    for (owner, condition) in conditions {
        catalog = catalog
            .with_common_attribute_condition(&owner, condition)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, entries) in content {
        catalog = catalog
            .with_common_attribute_content(&owner, entries)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, settings) in separation_parameters {
        catalog = catalog
            .with_common_attribute_separation_parameters(&owner, settings)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, attributes) in common_attributes.into_iter().flatten() {
        catalog = catalog
            .with_catalog_common_attributes(&owner, attributes)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, attributes) in catalog_attributes {
        if let Some(attributes) = attributes {
            catalog = catalog
                .with_catalog_attributes(&owner, attributes)
                .map_err(LoadError::Catalog)?;
        }
    }
    for (owner, settings, attributes) in characteristics {
        catalog = catalog
            .with_characteristic_metadata(&owner, settings, attributes)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, kind, dimensions, resources) in registers {
        let kind = kind.map(|kind| match kind {
            open_sdbl::metadata::AccumulationRegisterKind::Balance => {
                bsl_orm::AccumulationRegisterKind::Balance
            }
            open_sdbl::metadata::AccumulationRegisterKind::Turnovers => {
                bsl_orm::AccumulationRegisterKind::Turnovers
            }
        });
        catalog = catalog
            .with_accumulation_register_metadata(&owner, kind, dimensions, resources)
            .map_err(LoadError::Catalog)?;
    }
    for (owner, periodicity, dimensions, resources) in information_registers {
        catalog = catalog
            .with_information_register_metadata(&owner, periodicity, dimensions, resources)
            .map_err(LoadError::Catalog)?;
    }
    Ok(catalog)
}

fn attribute(
    descriptor: &ConfigDescriptor,
    references: &References<'_>,
    engine: &Engine,
    shapes: &RuntimeShapes,
) -> Result<DocumentAttribute, LoadError> {
    let types = descriptor
        .declared_types
        .as_ref()
        .and_then(|types| references.columns(types))
        .unwrap_or_default();
    let mut attribute =
        DocumentAttribute::new(descriptor.object_guid.as_str(), &descriptor.name, types);
    for synonym in &descriptor.synonyms {
        attribute = attribute.with_synonym(&synonym.language, &synonym.text);
    }
    if let Some(setting) = descriptor.fill_checking {
        attribute = attribute.with_fill_checking(super::fill_checking(setting));
    }
    if let Some(enabled) = descriptor.fill_from_filling_value {
        attribute = attribute.with_fill_from_filling_value(enabled);
    }
    if let Some(value) = descriptor
        .filling_value
        .as_ref()
        .map(|value| filling(value, references, engine))
        .transpose()?
        .flatten()
    {
        let graph = SerializedValueGraph::capture(&[value], shapes, &GraphLimits::default())
            .map_err(|error| malformed(descriptor.object_guid.as_str(), error))?;
        attribute = attribute.with_fill_value(graph);
    }
    Ok(attribute)
}

// Тип хранилища задаётся приобретённым описанием. Содержимое двоичной
// ячейки само по себе не объявляет её хранилищем.
pub(super) fn is_value_storage_declared_type(ty: &ConfigDeclaredType) -> bool {
    ty.tag == "#"
        && ty
            .reference
            .as_ref()
            .is_some_and(|reference| reference.as_str() == "e199ca70-93cf-46ce-a54b-6edc88c3a296")
}

fn column(ty: &ConfigDeclaredType, references: &References<'_>) -> Option<ColumnType> {
    let id = match ty.tag.as_str() {
        "S" => TypeRef::Native(TypeId::String),
        "N" => TypeRef::Native(TypeId::Number),
        "D" => TypeRef::Native(TypeId::Date),
        "B" => TypeRef::Native(TypeId::Boolean),
        "#" => {
            let reference = ty.reference.as_ref()?;
            if is_value_storage_declared_type(ty) {
                TypeRef::Object(
                    *bsl_orm::library()
                        .types()
                        .iter()
                        .find(|ty| ty.name == "ХранилищеЗначения")?,
                )
            } else if reference.as_str() == "fc01b5df-97fe-449b-83d4-218a090e681e"
                && ty.qualifiers.is_empty()
            {
                // Парная нативная выгрузка `native-uuid` связывает этот
                // идентификатор Config с `v8:UUID` в XML и EDT.
                TypeRef::Native(TypeId::Uuid)
            } else {
                TypeRef::Value(Arc::clone(&references.get(reference.as_str())?.1))
            }
        }
        _ => return None,
    };
    let quals = ty
        .qualifiers
        .iter()
        .map(|value| scalar(value).map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    Some(ColumnType { id, quals })
}

fn filling(
    value: &ConfigFillingValue,
    references: &References<'_>,
    engine: &Engine,
) -> Result<Option<open_bsl::Value>, LoadError> {
    use open_bsl::Value;
    Ok(Some(match value {
        ConfigFillingValue::Undefined => Value::Undefined,
        ConfigFillingValue::String(text) => Value::Str(text.as_str().into()),
        ConfigFillingValue::Boolean(value) => Value::Boolean(*value),
        ConfigFillingValue::Number(text) => Value::Number(
            open_bsl::parse_number(text, &NumberFormat::default())
                .map_err(|error| malformed("filling value", error))?,
        ),
        ConfigFillingValue::Reference { type_id, value_id } => {
            let Some((kind, declaration)) = references.get(type_id.as_str()) else {
                return Ok(None);
            };
            // Объявление типа не даёт права декодировать изменяемый объект как ссылку.
            if *kind == MetadataKind::DataProcessor {
                return Ok(None);
            }
            let materializer = engine
                .registry()
                .value_types()
                .iter()
                .find(|ty| ty.identity() == declaration.identity())
                .ok_or_else(|| malformed(type_id.as_str(), "нет декодера ссылочного типа"))?;
            let bytes = value_id.to_bytes();
            let payload = if *kind == MetadataKind::Enumeration {
                if value_id.is_nil() {
                    &[][..]
                } else {
                    value_id.as_str().as_bytes()
                }
            } else {
                bytes.as_slice()
            };
            materializer
                .materialize(payload)
                .map_err(|error| malformed(type_id.as_str(), error))?
        }
    }))
}

#[cfg(test)]
#[path = "document_attributes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "runtime_object_projection_tests.rs"]
mod runtime_object_tests;

#[cfg(test)]
#[path = "information_register_projection_tests.rs"]
mod information_register_tests;

#[cfg(test)]
#[path = "storage_declared_type_tests.rs"]
mod storage_declared_type_tests;

#[cfg(test)]
#[path = "selective_projection_tests.rs"]
mod selective_projection_tests;

#[cfg(test)]
#[path = "document_attribute_reference_sets_tests.rs"]
mod reference_sets_tests;
