//! Прямое чтение регистра через адаптер данных текущего сеанса.

use super::*;
use open_bsl::{TypeId, adjust_to_types};

/// Измеренная периодичность независимого регистра сведений.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationRegisterPeriodicity {
    /// Период отсутствует в ключе записи.
    Nonperiodic,
    /// Записи имеют дневную периодичность; прямое чтение ищет точную дату.
    Day,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Settings {
    periodicity: Option<InformationRegisterPeriodicity>,
    dimensions: Option<Arc<[DocumentAttribute]>>,
    resources: Option<Arc<[DocumentAttribute]>>,
}

impl Settings {
    pub(super) fn file_fields(&self, group: &str) -> Option<&[DocumentAttribute]> {
        match group {
            "dimensions" => self.dimensions.as_deref(),
            "resources" => self.resources.as_deref(),
            _ => None,
        }
    }
}

impl MetadataCatalog {
    /// Добавляет явные настройки независимого регистра сведений.
    /// Коллекции должны быть полными; `None` сохраняет неизвестность,
    /// а `Some([])` обозначает проверенную пустую коллекцию.
    ///
    /// # Errors
    /// Возвращает ошибку для отсутствующего владельца или объекта другого вида.
    pub fn with_information_register_metadata(
        mut self,
        owner: &str,
        periodicity: Option<InformationRegisterPeriodicity>,
        dimensions: Option<Vec<DocumentAttribute>>,
        resources: Option<Vec<DocumentAttribute>>,
    ) -> Result<Self, CatalogError> {
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::InformationRegister
            })
            .ok_or_else(|| CatalogError::UnknownInformationRegisterOwner(owner.into()))?;
        Arc::make_mut(&mut self.objects)[index].information_register = Some(Settings {
            periodicity,
            dimensions: dimensions.map(Into::into),
            resources: resources.map(Into::into),
        });
        Ok(self)
    }
}

pub(super) const METHOD: MethodDescriptor =
    MethodDescriptor::new(&["Получить", "Get"], Arity::range(0, 2), get);

fn failure(message: &str) -> RtError {
    RtError::DynamicError(format!("РегистрСведений.Получить: {message}"))
}

fn get(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| failure("менеджер отсутствует в каталоге"))?;
    let settings = object
        .information_register
        .as_ref()
        .ok_or_else(|| failure("периодичность регистра неизвестна"))?;
    let periodicity = settings
        .periodicity
        .ok_or_else(|| failure("периодичность регистра неизвестна"))?;
    let dimensions = settings
        .dimensions
        .as_deref()
        .ok_or_else(|| failure("полный состав измерений неизвестен"))?;
    let resources = settings
        .resources
        .as_deref()
        .ok_or_else(|| failure("полный состав ресурсов неизвестен"))?;
    // Пока реализована только форма, для которой сохранён нативный контроль.
    // Другие типы не превращаются в пустой успешный результат.
    let supported = |field: &DocumentAttribute, digits: &str| {
        matches!(field.types(),
        [column] if column.id.native() == Some(TypeId::Number) && column.quals == [digits, "0", "0"])
    };
    if dimensions.iter().any(|field| !supported(field, "5"))
        || resources.iter().any(|field| !supported(field, "15"))
        || resources.is_empty()
    {
        return Err(failure("описания типов полей пока не поддерживаются"));
    }
    let mut parameters = Vec::new();
    let mut conditions = Vec::new();
    let filter_index = if periodicity == InformationRegisterPeriodicity::Day {
        let period = args.first().ok_or_else(|| failure("не указан период"))?;
        if !matches!(period, Value::Date(_)) {
            return Err(failure("период должен быть датой"));
        }
        // Запись округляет период, но измеренный Get сравнивает точную дату.
        parameters.push((Rc::from("ПериодЧтения"), period.clone()));
        conditions.push("Данные.Период = &ПериодЧтения".to_string());
        1
    } else {
        if args.len() > 1 {
            return Err(failure(
                "для непериодического регистра допустим один аргумент",
            ));
        }
        0
    };
    let filter = args
        .get(filter_index)
        .ok_or_else(|| failure("не указан отбор"))?;
    let Value::Object(filter) = filter else {
        return Err(failure("отбор должен быть структурой"));
    };
    let BslObject::Structure(filter) = filter.as_ref() else {
        return Err(failure("отбор должен быть структурой"));
    };
    let filter = filter.borrow();
    if filter.len() != dimensions.len() {
        return Err(failure("отбор должен содержать все измерения и только их"));
    }
    for field in dimensions {
        let field_id = context.runtime_shapes().names.intern(field.name());
        let value = (0..filter.len())
            .filter_map(|index| filter.entry_at(index))
            .find(|(name, _)| *name == field_id)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| failure("отбор должен содержать все измерения и только их"))?;
        let parameter = format!("ИзмерениеЧтения{}", parameters.len());
        conditions.push(format!("Данные.{} = &{parameter}", field.name()));
        parameters.push((Rc::from(parameter), adjust_to_types(value, field.types())));
    }
    drop(filter);
    let projections = resources
        .iter()
        .enumerate()
        .map(|(index, field)| format!("Данные.{} КАК РесурсЧтения{index}", field.name()))
        .collect::<Vec<_>>();
    let predicate = if conditions.is_empty() {
        String::new()
    } else {
        format!(" ГДЕ {}", conditions.join(" И "))
    };
    let request = QueryRequest {
        text: format!(
            "ВЫБРАТЬ {} ИЗ РегистрСведений.{} КАК Данные{predicate}",
            projections.join(", "),
            object.name()
        )
        .into(),
        parameters: parameters.into(),
        temp_tables_manager: None,
    };
    let (principal, access) = current_data_access(context)?;
    let result = access.execute_query(&request, &principal, context)?;
    if result.columns.len() != resources.len() || result.rows.len() > 1 {
        return Err(failure("результат запроса не соответствует ключу регистра"));
    }
    let result_value = Value::new_structure(context.runtime_shapes().shapes.empty(), vec![]);
    for (index, field) in resources.iter().enumerate() {
        let value = result
            .rows
            .first()
            .map_or_else(|| Value::number_from_i64(0), |row| row[index].clone());
        let shapes = context.runtime_shapes();
        let name = shapes.names.intern(field.name());
        result_value.structure_insert_spelled(
            name,
            field.name(),
            value,
            &mut shapes.shapes,
            &shapes.names,
        )?;
    }
    Ok(result_value)
}
