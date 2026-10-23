//! Проекция приобретённых источников опций без повторного разбора Config.

use std::{collections::HashMap, sync::Arc};

use bsl_orm::{
    FunctionalOptionLocation, FunctionalOptionParameter, FunctionalOptionSettings, MetadataCatalog,
    MetadataKind,
};
use open_bsl::folded_eq;
use open_sdbl::metadata::{
    ConfigDescriptor, ConfigFieldPurpose, FunctionalOptionLocation as SourceLocation,
    RuntimeMetadataFacts,
};

#[cfg(test)]
use open_sdbl::metadata::MetadataSnapshot;

use super::{LoadError, malformed};

type Descriptors<'a> = HashMap<&'a str, &'a ConfigDescriptor>;

#[cfg(test)]
pub(super) fn project(
    catalog: MetadataCatalog,
    metadata: &open_sdbl::metadata::MetadataSnapshot,
) -> Result<MetadataCatalog, LoadError> {
    project_runtime(catalog, metadata.runtime_facts())
}

pub(super) fn project_runtime(
    mut catalog: MetadataCatalog,
    metadata: &RuntimeMetadataFacts,
) -> Result<MetadataCatalog, LoadError> {
    let descriptors = metadata
        .descriptors()
        .iter()
        .map(|descriptor| (descriptor.object_guid.as_str(), descriptor))
        .collect::<Descriptors<'_>>();
    let complete_parameters = project_parameters(&catalog, &descriptors);
    let options = catalog
        .objects()
        .iter()
        .filter(|object| object.kind() == MetadataKind::FunctionalOption)
        .map(|object| object.id().to_owned())
        .collect::<Vec<_>>();
    for id in options {
        let Some(descriptor) = descriptors.get(id.as_str()) else {
            continue;
        };
        let location = descriptor
            .functional_option_location
            .as_ref()
            .map(|location| project_location(location, &catalog, &descriptors))
            .transpose()?;
        let parameters = match &location {
            Some(FunctionalOptionLocation::Constant(_) | FunctionalOptionLocation::Absent) => {
                Some(Vec::new())
            }
            Some(location) => complete_parameters.as_ref().map(|parameters| {
                parameters
                    .iter()
                    .filter(|parameter| match location {
                        FunctionalOptionLocation::CatalogAttribute { owner, .. } => {
                            parameter.target_id.as_ref() == owner.as_ref()
                        }
                        FunctionalOptionLocation::InformationRegisterResource { owner, .. } => {
                            descriptors.get(parameter.target_id.as_ref()).is_some_and(
                                |descriptor| descriptor.resource_guid.as_str() == owner.as_ref(),
                            )
                        }
                        _ => false,
                    })
                    .cloned()
                    .collect()
            }),
            None => None,
        };
        catalog = catalog
            .with_functional_option_settings(
                &id,
                FunctionalOptionSettings {
                    location,
                    privileged_get_mode: descriptor.functional_option_privileged_get_mode,
                    parameters,
                },
            )
            .map_err(LoadError::Catalog)?;
    }
    Ok(catalog)
}

fn require_owner(catalog: &MetadataCatalog, id: &str, kind: MetadataKind) -> Result<(), LoadError> {
    if catalog
        .object_by_id(id)
        .is_some_and(|owner| owner.kind() == kind)
    {
        Ok(())
    } else {
        Err(malformed(
            id,
            "источник функциональной опции имеет неверного владельца",
        ))
    }
}

fn project_location(
    source: &SourceLocation,
    catalog: &MetadataCatalog,
    descriptors: &Descriptors<'_>,
) -> Result<FunctionalOptionLocation, LoadError> {
    match source {
        SourceLocation::Constant(id) => {
            require_owner(catalog, id.as_str(), MetadataKind::Constant)?;
            Ok(FunctionalOptionLocation::Constant(Arc::from(id.as_str())))
        }
        SourceLocation::CatalogAttribute { owner, attribute } => {
            require_owner(catalog, owner.as_str(), MetadataKind::Catalog)?;
            let valid = descriptors
                .get(owner.as_str())
                .and_then(|descriptor| descriptor.catalog_attributes.as_ref())
                .is_some_and(|attributes| attributes.contains(attribute));
            let owned = descriptors
                .get(attribute.as_str())
                .is_some_and(|descriptor| descriptor.resource_guid == *owner);
            if !valid || !owned {
                return Err(malformed(
                    attribute.as_str(),
                    "источник опции не является собственным реквизитом справочника",
                ));
            }
            Ok(FunctionalOptionLocation::CatalogAttribute {
                owner: Arc::from(owner.as_str()),
                attribute: Arc::from(attribute.as_str()),
            })
        }
        SourceLocation::InformationRegisterResource { owner, resource } => {
            require_owner(catalog, owner.as_str(), MetadataKind::InformationRegister)?;
            let valid = descriptors
                .get(resource.as_str())
                .is_some_and(|descriptor| {
                    descriptor.resource_guid == *owner
                        && descriptor.field_purpose
                            == Some(ConfigFieldPurpose::InformationRegisterResource)
                });
            if !valid {
                return Err(malformed(
                    resource.as_str(),
                    "источник опции не является ресурсом своего регистра",
                ));
            }
            Ok(FunctionalOptionLocation::InformationRegisterResource {
                owner: Arc::from(owner.as_str()),
                resource: Arc::from(resource.as_str()),
            })
        }
        SourceLocation::Absent => Ok(FunctionalOptionLocation::Absent),
    }
}

fn project_parameters(
    catalog: &MetadataCatalog,
    descriptors: &Descriptors<'_>,
) -> Option<Vec<FunctionalOptionParameter>> {
    let ids = descriptors
        .get(catalog.identity())?
        .configuration_functional_option_parameters
        .as_ref()?;
    let mut result = Vec::new();
    let mut names = Vec::<&str>::new();
    for id in ids {
        let parameter = descriptors.get(id.as_str())?;
        if parameter.name.is_empty() || names.iter().any(|name| folded_eq(name, &parameter.name)) {
            return None;
        }
        names.push(&parameter.name);
        for target in parameter.functional_option_parameter_use.as_ref()? {
            // Проверяем весь приобретённый список: неизвестная привязка
            // может относиться к нашему источнику и не должна исчезнуть.
            let target_catalog = catalog
                .object_by_id(target.as_str())
                .is_some_and(|object| object.kind() == MetadataKind::Catalog);
            let target_dimension = descriptors.get(target.as_str()).filter(|descriptor| {
                descriptor.field_purpose == Some(ConfigFieldPurpose::InformationRegisterDimension)
                    && catalog
                        .object_by_id(descriptor.resource_guid.as_str())
                        .is_some_and(|owner| owner.kind() == MetadataKind::InformationRegister)
            });
            if !target_catalog && target_dimension.is_none() {
                return None;
            }
            result.push(FunctionalOptionParameter {
                name: Arc::from(parameter.name.as_str()),
                target_id: Arc::from(target.as_str()),
            });
        }
    }
    Some(result)
}

#[cfg(test)]
#[path = "functional_options_tests.rs"]
pub(crate) mod tests;
