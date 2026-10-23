//! Поиск стандартных полей справочника через запрос текущего сеанса.

use std::rc::Rc;
use std::sync::Arc;

use super::{
    CatalogCodeType, ConfigurationReferenceValue, ManagerValue, QueryRequest,
    catalog_reference_value, current_data_access,
};
use open_bsl::{CallContext, ObjectProtocol, RtError, RtResult, Value};

fn unsupported() -> RtError {
    RtError::DynamicError(
        "этот режим поиска стандартного поля справочника пока не поддерживается".into(),
    )
}

fn flag(args: &[Value]) -> RtResult<bool> {
    match args.get(1) {
        None | Some(Value::Undefined) => Ok(false),
        Some(Value::Boolean(value)) => Ok(*value),
        _ => Err(unsupported()),
    }
}

pub(super) fn code(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if flag(args)? {
        return Err(unsupported());
    }
    find(receiver, args, context, false, true)
}

pub(super) fn description(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    find(receiver, args, context, true, flag(args)?)
}

fn find(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
    description: bool,
    exact: bool,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or(RtError::NotAnObject)?;
    let settings = object.catalog_standard_attributes().ok_or_else(|| {
        RtError::DynamicError("стандартные реквизиты справочника не приобретены".into())
    })?;
    if args
        .iter()
        .skip(2)
        .any(|value| !matches!(value, Value::Undefined))
    {
        return Err(unsupported());
    }
    if (description && settings.description_length == 0)
        || (!description
            && (settings.code_length == 0 || settings.code_type != CatalogCodeType::String))
    {
        return Err(unsupported());
    }
    let Value::Str(search) = &args[0] else {
        return Err(unsupported());
    };
    let search = search.shared_utf8();
    let field = if description {
        "Наименование"
    } else {
        "Код"
    };
    let predicate = if exact {
        format!("СОКРП(Источник.{field}) = СОКРП(&Значение)")
    } else {
        format!("ЛЕВ(Источник.{field}, ДЛИНАСТРОКИ(&Значение)) = &Значение")
    };
    let request = QueryRequest {
        text: format!("ВЫБРАТЬ Источник.Ссылка КАК Ссылка, Источник.{field} КАК ЗначениеПоиска ИЗ Справочник.{} КАК Источник ГДЕ {predicate}", object.name()).into(),
        parameters: vec![(Rc::from("Значение"), args[0].clone())].into(),
        temp_tables_manager: None,
    };
    let (principal, adapter) = current_data_access(context)?;
    let result = adapter.execute_query(&request, &principal, context)?;
    for row in result.rows() {
        let [reference, Value::Str(candidate)] = row.as_ref() else {
            return Err(RtError::DynamicError(
                "адаптер поиска вернул неверную строку результата".into(),
            ));
        };
        let candidate = candidate.shared_utf8();
        let matches = if exact {
            candidate.trim_end_matches(' ') == search.trim_end_matches(' ')
        } else {
            candidate.starts_with(search.as_ref())
        };
        if !matches {
            continue;
        }
        // Сравнение в СУБД может допускать строки другого регистра. Поэтому
        // ограничивать результат первой строкой до этой проверки нельзя.
        let compatible = reference
            .object_ref()
            .and_then(|value| value.downcast_ref::<ConfigurationReferenceValue>())
            .is_some_and(|reference| {
                reference.object_id == manager.object_id
                    && reference.catalog.identity() == manager.catalog.identity()
            });
        if !compatible {
            return Err(RtError::DynamicError(
                "адаптер поиска вернул несовместимую ссылку".into(),
            ));
        }
        return Ok(reference.clone());
    }
    catalog_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])
}
