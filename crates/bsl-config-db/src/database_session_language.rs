//! Фиксация приобретённого языка пользователя перед открытием сеанса.

use bsl_orm::{MetadataCatalog, MetadataKind, Principal};
use open_sdbl::metadata::Guid;

pub(super) fn resolve(
    principal: Principal,
    selected: Option<&Guid>,
    catalog: &MetadataCatalog,
) -> Result<Principal, String> {
    let id = if principal.id().is_empty()
        || selected.is_some_and(|id| id.as_str() == "00000000-0000-0000-0000-000000000000")
    {
        catalog
            .primary_language_id()
            .ok_or("основной язык конфигурации не приобретён")?
    } else {
        selected
            .ok_or("выбранный язык пользователя не удалось прочитать")?
            .as_str()
    };
    catalog
        .object_by_id(id)
        .filter(|object| object.kind() == MetadataKind::Language)
        .ok_or("выбранный язык отсутствует в метаданных языков конфигурации")?;
    Ok(principal.with_interface_language_id(id))
}

#[cfg(test)]
#[path = "database_session_language_tests.rs"]
mod tests;
