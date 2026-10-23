//! Планы чтения измеренных параметризованных источников функциональных опций.

use bsl_orm::{FunctionalOptionLocation, FunctionalOptionSettings, MetadataCatalog, MetadataKind};
use open_bsl::{RtError, RtResult, Value, folded_eq};
use open_sdbl::metadata::{
    CatalogAttributeUse, ConfigDescriptor, ConfigFieldPurpose, Guid,
    InformationRegisterPeriodicity, InformationRegisterWriteMode, MetadataSnapshot, ObjectId,
};
use open_sdbl::query::QueryParameter;

pub(super) struct ReadPlan {
    pub query: String,
    pub parameters: Vec<QueryParameter>,
    pub owner: ObjectId,
    complete: bool,
}

impl ReadPlan {
    pub(super) fn value(&self, rows: &[std::rc::Rc<[Value]>]) -> Value {
        if !self.complete {
            return Value::Boolean(!rows.is_empty());
        }
        // SDK уже проверил ширину единственной проекции. Отсутствие строки
        // даёт булево `Ложь` даже для числовой опции, как в нативном контроле.
        // Замеры: `tests/conformance/measure/1c/measure-functional-option-register.platform.txt`,
        // `tests/conformance/measure/1c/measure-functional-option-catalog.platform.txt`.
        rows.first()
            .and_then(|row| row.first())
            .cloned()
            .unwrap_or(Value::Boolean(false))
    }
}

fn error(message: &str) -> RtError {
    RtError::DynamicError(format!("ПолучитьФункциональнуюОпцию: {message}"))
}

fn descriptor<'a>(metadata: &'a MetadataSnapshot, id: &str) -> RtResult<&'a ConfigDescriptor> {
    metadata
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.object_guid.as_str() == id)
        .ok_or_else(|| error("описатель источника отсутствует в снимке"))
}

pub(super) fn prepare(
    metadata: &MetadataSnapshot,
    catalog: &MetadataCatalog,
    settings: &FunctionalOptionSettings,
    supplied: &[(&str, &Value)],
) -> RtResult<ReadPlan> {
    let (owner_id, field_id, kind, source) = match settings.location.as_ref() {
        Some(FunctionalOptionLocation::CatalogAttribute { owner, attribute }) => (
            owner.as_ref(),
            attribute.as_ref(),
            MetadataKind::Catalog,
            "Справочник",
        ),
        Some(FunctionalOptionLocation::InformationRegisterResource { owner, resource }) => (
            owner.as_ref(),
            resource.as_ref(),
            MetadataKind::InformationRegister,
            "РегистрСведений",
        ),
        _ => return Err(error("ожидался параметризованный источник")),
    };
    let owner = catalog
        .object_by_id(owner_id)
        .filter(|object| object.kind() == kind)
        .ok_or_else(|| error("владелец источника отсутствует в каталоге"))?;
    let owner_descriptor = descriptor(metadata, owner_id)?;
    if owner_descriptor.name != owner.name() {
        return Err(error("имя владельца не совпадает со снимком данных"));
    }
    let field = descriptor(metadata, field_id)?;
    if field.resource_guid.as_str() != owner_id {
        return Err(error("поле источника не принадлежит владельцу"));
    }
    let boolean = match field.declared_types.as_deref() {
        Some([ty]) if ty.tag == "B" => true,
        Some([ty]) if ty.tag == "N" => false,
        _ => return Err(error("тип источника пока не поддерживается")),
    };
    let bindings = settings
        .parameters
        .as_deref()
        .ok_or_else(|| error("полный состав параметров опции неизвестен"))?;
    let (dimensions, applicability) = if kind == MetadataKind::Catalog {
        if !owner_descriptor
            .catalog_attributes
            .as_ref()
            .is_some_and(|ids| ids.iter().any(|id| id.as_str() == field_id))
        {
            return Err(error("собственный реквизит отсутствует в полном составе"));
        }
        let use_mode = field
            .catalog_attribute_use
            .ok_or_else(|| error("использование реквизита неизвестно"))?;
        let applicability = match use_mode {
            CatalogAttributeUse::ForFolderAndItem => None,
            CatalogAttributeUse::ForItem | CatalogAttributeUse::ForFolder => {
                match owner_descriptor.catalog_hierarchical {
                    Some(true) => Some(if use_mode == CatalogAttributeUse::ForItem {
                        "НЕ О.ЭтоГруппа"
                    } else {
                        "О.ЭтоГруппа"
                    }),
                    Some(false) if use_mode == CatalogAttributeUse::ForItem => None,
                    _ => {
                        return Err(error(
                            "применимость реквизита к группам неизвестна или не поддерживается",
                        ));
                    }
                }
            }
        };
        (vec![owner_id], applicability)
    } else {
        if owner_descriptor.information_register_write_mode
            != Some(InformationRegisterWriteMode::Independent)
            || owner_descriptor.information_register_periodicity
                != Some(InformationRegisterPeriodicity::Nonperiodical)
        {
            return Err(error(
                "поддержан только независимый непериодический регистр",
            ));
        }
        if field.field_purpose != Some(ConfigFieldPurpose::InformationRegisterResource)
            || !owner_descriptor
                .information_register_resources
                .as_ref()
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == field_id))
        {
            return Err(error("ресурс отсутствует в полном составе регистра"));
        }
        let dimensions = owner_descriptor
            .information_register_dimensions
            .as_ref()
            .ok_or_else(|| error("полный состав измерений неизвестен"))?;
        if dimensions.is_empty() {
            return Err(error("источник без измерений пока не поддерживается"));
        }
        (dimensions.iter().map(|id| id.as_str()).collect(), None)
    };
    let mut parameters = Vec::new();
    let mut conditions = Vec::new();
    let mut selected = Vec::new();
    let mut targets = Vec::new();
    for binding in bindings {
        let target = binding.target_id.as_ref();
        if !dimensions.contains(&target) || targets.contains(&target) {
            return Err(error(
                "привязки параметров к источнику неизвестны или неоднозначны",
            ));
        }
        targets.push(target);
        let Some((_, value)) = supplied
            .iter()
            .find(|(name, _)| folded_eq(name, &binding.name))
        else {
            continue;
        };
        let field_name = if kind == MetadataKind::Catalog {
            let (reference_owner, _) = bsl_orm::catalog_reference_parts(value)
                .ok_or_else(|| error("параметр справочника должен быть его ссылкой"))?;
            if reference_owner.id() != owner_id {
                return Err(error("параметр содержит ссылку другого справочника"));
            }
            "Ссылка"
        } else {
            let dimension = descriptor(metadata, target)?;
            if dimension.resource_guid.as_str() != owner_id
                || dimension.field_purpose != Some(ConfigFieldPurpose::InformationRegisterDimension)
            {
                return Err(error("параметр не связан с собственным измерением"));
            }
            dimension.name.as_str()
        };
        let parameter = format!("ПараметрОпции{}", parameters.len());
        conditions.push(format!("О.{field_name} = &{parameter}"));
        parameters.push(QueryParameter::new(
            parameter,
            super::data_access::parameter_value(value)?,
        ));
        selected.push(target);
    }
    let complete = dimensions
        .iter()
        .all(|dimension| selected.contains(dimension));
    if !complete && !boolean {
        return Err(error(
            "для небулевой опции должны быть заданы все параметры источника",
        ));
    }
    let projection = if complete {
        match applicability {
            Some(condition) => format!(
                "ВЫБОР КОГДА {condition} ТОГДА О.{} ИНАЧЕ NULL КОНЕЦ",
                field.name
            ),
            None => format!("О.{}", field.name),
        }
    } else {
        // Булево «или» проверяется одним совпавшим истинным значением в БД,
        // без переноса всех строк регистра или справочника в память сеанса.
        conditions.push(format!("О.{} = ИСТИНА", field.name));
        if let Some(condition) = applicability {
            conditions.push(condition.to_owned());
        }
        "ИСТИНА".to_owned()
    };
    let mut query = format!(
        "ВЫБРАТЬ ПЕРВЫЕ 1 {projection} КАК Значение ИЗ {source}.{} КАК О",
        owner.name()
    );
    if !conditions.is_empty() {
        query.push_str(" ГДЕ ");
        query.push_str(&conditions.join(" И "));
    }
    let guid: Guid = owner_id
        .parse()
        .map_err(|_| error("идентификатор источника некорректен"))?;
    Ok(ReadPlan {
        query,
        parameters,
        owner: ObjectId::from_bytes(guid.to_bytes()),
        complete,
    })
}

#[cfg(test)]
#[path = "functional_option_reads_tests.rs"]
mod tests;
