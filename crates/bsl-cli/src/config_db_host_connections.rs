//! Авторизация полного снимка реальных сеансов текущего CLI-хоста.

use bsl_session::InfoBaseConnection;
use open_sdbl::metadata::{Guid, Right};
use open_sdbl_db::AccessStore;

pub(super) fn authorized_snapshot(
    access: &AccessStore,
    roles: &[Guid],
    root: Option<&str>,
    connections: impl IntoIterator<Item = InfoBaseConnection>,
) -> Result<Option<Vec<InfoBaseConnection>>, String> {
    let root: Guid = root
        .ok_or("идентификатор корневого объекта конфигурации не приобретён")?
        .parse()
        .map_err(|error| format!("неверный идентификатор корня конфигурации: {error}"))?;
    if !access.missing_rights(roles).is_empty() || !access.unreadable_rights(roles).is_empty() {
        return Ok(None);
    }
    let allowed = access
        .rights_of(roles)
        .iter()
        .any(|rights| rights.grants(&root, &Right::ActiveUsers));
    Ok(allowed.then(|| connections.into_iter().collect()))
}

pub(super) fn computer_name() -> Result<String, String> {
    // На Linux читаем имя самого хоста, а не необязательную переменную
    // оболочки. На остальных поддерживаемых хостах его передаёт окружение.
    #[cfg(target_os = "linux")]
    let name = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map_err(|error| format!("не удалось получить имя компьютера: {error}"))?;
    #[cfg(not(target_os = "linux"))]
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map_err(|error| format!("не удалось получить имя компьютера: {error}"))?;
    let name = name.trim();
    if name.is_empty() {
        return Err("имя компьютера хоста пусто".into());
    }
    Ok(name.to_owned())
}

#[cfg(test)]
#[path = "config_db_host_connections_tests.rs"]
mod tests;
