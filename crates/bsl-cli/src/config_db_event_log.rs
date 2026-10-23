//! Приём независимых событий конфигурационного сеанса в stderr CLI.

use bsl_session::{EventLogEntry, EventLogTransactionMode, EventLogWriter};
use open_bsl::{RtError, RtResult, format_value};
use std::io::Write;

/// Приёмник событий только для сконфигурированного сеанса CLI.
#[derive(Debug)]
pub(crate) struct CliEventLogWriter;

impl EventLogWriter for CliEventLogWriter {
    fn write(&self, entry: EventLogEntry) -> RtResult<()> {
        write_event(&mut std::io::stderr().lock(), entry)
    }
}

fn write_event(output: &mut impl Write, entry: EventLogEntry) -> RtResult<()> {
    if entry.transaction_mode() != EventLogTransactionMode::Independent {
        return Err(RtError::DynamicError(
            "транзакционная запись журнала регистрации не поддерживается CLI".into(),
        ));
    }
    let metadata = format_value(entry.metadata(), None)
        .map_err(|error| RtError::DynamicError(error.to_string()))?;
    let data = format_value(entry.data(), None)
        .map_err(|error| RtError::DynamicError(error.to_string()))?;
    writeln!(
        output,
        "Журнал регистрации [{:?}] {}: {}\nМетаданные: {}\nДанные: {}",
        entry.level(),
        entry.event_name(),
        entry.comment(),
        metadata,
        data,
    )
    .map_err(|error| {
        RtError::DynamicError(format!("не удалось записать событие журнала CLI: {error}"))
    })
}

#[cfg(test)]
#[path = "config_db_event_log_tests.rs"]
mod tests;
