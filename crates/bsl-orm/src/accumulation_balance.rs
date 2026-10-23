//! Остатки через существующий адаптер запросов текущего сеанса.

use super::*;
use open_bsl::TypeId;

pub(super) const METHOD: MethodDescriptor =
    MethodDescriptor::new(&["Остатки", "Balance"], Arity::range(0, 4), balance);

fn failure(message: &str) -> RtError {
    RtError::DynamicError(format!("РегистрНакопления.Остатки: {message}"))
}

fn selected<'a>(
    fields: &'a [DocumentAttribute],
    value: &Value,
) -> RtResult<Vec<&'a DocumentAttribute>> {
    let text = match value {
        Value::Undefined => return Ok(fields.iter().collect()),
        Value::Str(text) => text.to_string(),
        _ => return Err(failure("список полей должен быть строкой")),
    };
    if text.trim().is_empty() {
        return Ok(fields.iter().collect());
    }
    text.split(',')
        .map(|name| {
            fields
                .iter()
                .find(|field| folded_eq(field.name(), name.trim()))
                .ok_or_else(|| failure("неизвестное поле в списке"))
        })
        .collect()
}

fn balance(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| failure("менеджер отсутствует в каталоге"))?;
    match object.accumulation_register_kind() {
        Some(AccumulationRegisterKind::Balance) => {}
        Some(AccumulationRegisterKind::Turnovers) => {
            return Err(failure("регистр имеет вид Обороты"));
        }
        None => return Err(failure("вид регистра неизвестен")),
    }
    let dimensions = object
        .accumulation_register_dimensions()
        .ok_or_else(|| failure("полный состав измерений неизвестен"))?;
    let resources = object
        .accumulation_register_resources()
        .ok_or_else(|| failure("полный состав ресурсов неизвестен"))?;
    let argument = |index| arguments.get(index).unwrap_or(&Value::Undefined);
    let mut parameters = Vec::new();
    let moment = match argument(0) {
        Value::Undefined => "",
        value
            if matches!(value, Value::Date(_))
                || query_boundary_parts(value).is_some()
                || matches!(value, Value::Object(object) if matches!(object.as_ref(), BslObject::Extension(component) if component.downcast_ref::<open_bsl::PointInTimeValue>().is_some())) =>
        {
            // Типизированная граница остаётся значением параметра: SQL,
            // права и разделение данных обрабатывает прежний адаптер сеанса.
            parameters.push((Rc::from("МоментОстатков"), value.clone()));
            "&МоментОстатков"
        }
        _ => return Err(failure("этот тип временной границы пока не поддерживается")),
    };
    let mut conditions = Vec::new();
    match argument(1) {
        Value::Undefined => {}
        Value::Object(object) => {
            let BslObject::Structure(storage) = object.as_ref() else {
                return Err(failure("отбор должен быть структурой"));
            };
            let storage = storage.borrow();
            for index in 0..storage.len() {
                let (name, value) = storage
                    .entry_at(index)
                    .ok_or_else(|| failure("отбор повреждён"))?;
                let name = context
                    .runtime_shapes()
                    .names
                    .name(name)
                    .ok_or_else(|| failure("имя поля отбора неизвестно"))?
                    .to_string();
                let field = dimensions
                    .iter()
                    .find(|field| folded_eq(field.name(), &name))
                    .ok_or_else(|| failure("неизвестное измерение отбора"))?;
                let parameter = format!("ОтборОстатков{index}");
                conditions.push(format!("{} = &{parameter}", field.name()));
                parameters.push((Rc::from(parameter), value));
            }
        }
        _ => return Err(failure("отбор должен быть структурой")),
    }
    let selected_dimensions = selected(dimensions, argument(2))?;
    let selected_resources = selected(resources, argument(3))?;
    let mut schema = Vec::new();
    let mut projections = Vec::new();
    for (field, resource) in selected_dimensions
        .into_iter()
        .map(|field| (field, false))
        .chain(selected_resources.into_iter().map(|field| (field, true)))
    {
        let mut types = field.types().to_vec();
        if types.is_empty() {
            return Err(failure("тип выбранного поля неизвестен"));
        }
        if resource {
            // Квалификаторы результата измерены на ресурсе 15,2 любого знака.
            // Другие описания требуют контроля, а не вывода по аналогии.
            if types.len() != 1 {
                return Err(failure("составной тип ресурса пока не поддерживается"));
            }
            for ty in &mut types {
                if ty.id.native() != Some(TypeId::Number) || ty.quals != ["15", "2", "0"] {
                    return Err(failure("описание типа ресурса пока не поддерживается"));
                }
                ty.quals[0] = "38".into();
            }
        }
        let suffix = if resource { "Остаток" } else { "" };
        projections.push(format!(
            "Остатки.{}{suffix} КАК ПолеОстатков{}",
            field.name(),
            schema.len()
        ));
        schema.push((field.name().to_string(), types));
    }
    if projections.is_empty() {
        return Err(failure("нет выбранных полей"));
    }
    // SDK сам сворачивает неиспользуемые измерения виртуальной таблицы.
    // Уникальные псевдонимы запроса не меняют повторные имена результата метода.
    let request = QueryRequest {
        text: format!(
            "ВЫБРАТЬ {} ИЗ РегистрНакопления.{}.Остатки({moment}, {}) КАК Остатки",
            projections.join(", "),
            object.name(),
            conditions.join(" И ")
        )
        .into(),
        parameters: parameters.into(),
        temp_tables_manager: None,
    };
    let (principal, data_access) = current_data_access(context)?;
    let result = data_access.execute_query(&request, &principal, context)?;
    if result.columns.len() != schema.len() {
        return Err(failure("ширина результата запроса не совпадает со схемой"));
    }
    let table = Value::new_table_with_typed_columns(schema);
    let Value::Object(object) = &table else {
        unreachable!()
    };
    let BslObject::ValueTable(data) = object.as_ref() else {
        unreachable!()
    };
    let mut data = data.borrow_mut();
    for values in result.rows.iter() {
        let row = data.add_row()?;
        for (index, value) in values.iter().enumerate() {
            data.set_cell(row, index, value.clone())
                .ok_or_else(|| failure("ячейка результата недоступна"))?;
        }
    }
    drop(data);
    Ok(table)
}

#[cfg(test)]
#[path = "accumulation_balance_tests.rs"]
mod tests;
