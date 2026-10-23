//! Передача отказа проверки права в журнал вызывающего сеанса.

use super::*;
use bsl_orm::{AccessRightRequest, AccessRightSubject};
use bsl_session::{EventLogEntry, EventLogLevel, EventLogTransactionMode};
use open_sdbl::metadata::Right;

#[cfg(test)]
pub(super) fn verify_access_rights(
    access: &AccessStore,
    request: &AccessRightRequest,
    principal: &Principal,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    if !matches!(request.subject(), AccessRightSubject::CurrentPrincipal) {
        return Err(error("проверка относится только к текущему пользователю"));
    }
    let granted = configuration_rights::access_right(access, request, principal, context)?;
    record_verification(request, context, granted)
}

pub(super) fn record_verification(
    request: &AccessRightRequest,
    context: &mut CallContext<'_>,
    granted: bool,
) -> RtResult<()> {
    if !matches!(request.subject(), AccessRightSubject::CurrentPrincipal) {
        return Err(error("проверка относится только к текущему пользователю"));
    }
    let writer = {
        let runtime = context
            .host_data()?
            .downcast_ref::<SessionData>()
            .and_then(|session| session.extension::<SessionRuntime>())
            .ok_or_else(|| error("runtime сеанса не передан для проверки прав"))?;
        Rc::clone(
            runtime
                .event_log_writer()
                .ok_or_else(|| error("журнал регистрации не подключён для текущего сеанса"))?,
        )
    };
    if granted {
        return Ok(());
    }
    let right = Right::parse(request.right())
        .expect("право принято проверкой приобретённых прав")
        .name();
    let runtime = context.runtime_shapes();
    let name = runtime.names.intern("Right");
    let data = Value::new_dictionary_structure_spelled(
        vec![(name, "Right".into(), Value::Str(right.as_str().into()))],
        &runtime.names,
    );
    let metadata = if request.metadata().kind() == MetadataKind::Configuration {
        Value::Undefined
    } else {
        request.metadata_value().clone()
    };
    // Нативный отказ сохраняется после отмены транзакции со статусом
    // `No transaction`. Журнал берётся из текущего контекста при каждом
    // вызове, поэтому другой сеанс не наследует ни запись, ни получателя.
    writer.write(EventLogEntry::new(
        "_$Access$_.AccessDenied",
        EventLogLevel::Information,
        metadata,
        data,
        "",
        EventLogTransactionMode::Independent,
    ))?;
    Err(error("Нарушение прав доступа"))
}
