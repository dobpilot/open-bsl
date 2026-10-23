//! Имя метода регламентного задания из явной ссылки на общий модуль.

use super::*;

pub(super) fn project(
    object: MetadataObject,
    resources: &Resources<'_>,
) -> Result<MetadataObject, LoadError> {
    let value = parse_serialized(&resources.decoded(object.id())?)
        .map_err(|error| malformed(object.id(), error))?;
    let Some((module_id, procedure)) = method_reference(object.id(), &value)? else {
        return Ok(object);
    };
    let module = parse_serialized(&resources.decoded(module_id)?)
        .map_err(|error| malformed(module_id, error))?;
    let module_fields = list_path(&module, &[1])
        .filter(|fields| fields.first().and_then(scalar) == Some("12"))
        .ok_or_else(|| malformed(module_id, "ссылка задания ведёт не на общий модуль"))?;
    let header = module_fields
        .get(1)
        .and_then(list)
        .ok_or_else(|| malformed(module_id, "нет заголовка общего модуля"))?;
    let identity = header
        .get(1)
        .and_then(list)
        .and_then(|identity| identity.get(2))
        .and_then(scalar);
    if !identity.is_some_and(|id| id.eq_ignore_ascii_case(module_id)) {
        return Err(malformed(
            module_id,
            "не совпадает GUID общего модуля задания",
        ));
    }
    let Some(Value::String(name)) = header.get(2) else {
        return Err(malformed(module_id, "нет имени общего модуля задания"));
    };
    // Нативный runtime возвращает квалифицированное имя без XML-префикса.
    let method = format!("{name}.{procedure}");
    Ok(object
        .with_string_property("ИмяМетода", method.clone())
        .with_string_property("MethodName", method))
}

// Та же измеренная ссылка нужна адресному загрузчику до публикации описателя.
// Зависимость относится к заголовку общего модуля, а не к его BSL-телу.
pub(super) fn referenced_module(resource: &ConfigResource) -> Result<Option<String>, LoadError> {
    let resources = Resources::new(std::slice::from_ref(resource))?;
    let value = parse_serialized(&resources.decoded(&resource.file_name)?)
        .map_err(|error| malformed(&resource.file_name, error))?;
    Ok(method_reference(&resource.file_name, &value)?
        .map(|(module, _)| module.to_ascii_lowercase()))
}

fn method_reference<'a>(
    id: &str,
    value: &'a Value,
) -> Result<Option<(&'a str, &'a str)>, LoadError> {
    let Some(fields) = list_path(value, &[1]) else {
        return Ok(None);
    };
    // Форма сверена с 185 парами CF/XML BUH; неизвестные формы не угадываем.
    if fields.len() != 10 || fields.first().and_then(scalar) != Some("2") {
        return Ok(None);
    }
    let Some(module_id) = fields.get(6).and_then(scalar).filter(|id| guid(id)) else {
        return Err(malformed(id, "нет ссылки на общий модуль задания"));
    };
    let Some(Value::String(procedure)) = fields.get(7) else {
        return Err(malformed(id, "нет имени процедуры задания"));
    };
    Ok(Some((module_id, procedure)))
}

#[cfg(test)]
#[path = "scheduled_job_metadata_tests.rs"]
mod tests;
