//! Выборка справочника через адаптер данных текущего сеанса.

use super::*;

#[derive(Debug)]
struct CatalogSelection {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    data: QueryResultData,
    next: Cell<usize>,
    current: Cell<Option<usize>>,
}

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СправочникВыборка",
    type_display: "Catalog selection",
    type_names: &["CatalogSelection"],
};
pub(super) static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Следующий", "Next"],
    Arity::exact(0),
    next,
)];

impl ObjectProtocol for CatalogSelection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        let name = match name {
            name if folded_eq(name, "Ref") => "Ссылка",
            name if folded_eq(name, "Code") => "Код",
            name if folded_eq(name, "Description") => "Наименование",
            name if folded_eq(name, "DeletionMark") => "ПометкаУдаления",
            name => name,
        };
        let index = self
            .data
            .columns()
            .iter()
            .position(|column| folded_eq(column, name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        if let Some(current) = self.current.get() {
            return Ok(self.data.rows()[current][index].clone());
        }
        // В нативной выборке ссылка до первого шага пустая, остальные поля
        // неопределены. Последняя строка сохраняется после отказа следующего шага.
        if folded_eq(name, "Ссылка") {
            return catalog_reference_value(Arc::clone(&self.catalog), &self.object_id, [0; 16]);
        }
        Ok(Value::Undefined)
    }
}

fn next(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let selection = receiver
        .downcast_ref::<CatalogSelection>()
        .ok_or(RtError::NotAnObject)?;
    let next = selection.next.get();
    if next < selection.data.rows().len() {
        selection.current.set(Some(next));
        selection.next.set(next + 1);
        Ok(Value::Boolean(true))
    } else {
        // Измеренный конец обхода возвращает ложь один раз; следующий вызов
        // начинает новый обход, не очищая доступную последнюю строку.
        selection.next.set(0);
        Ok(Value::Boolean(false))
    }
}

pub(super) fn select(
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
        .ok_or(RtError::NotAnObject)?;
    let mut parameters = Vec::new();
    let mut predicates = Vec::new();
    if args
        .first()
        .is_some_and(|value| !matches!(value, Value::Undefined))
        && object.catalog_hierarchical() != Some(false)
    {
        if object.catalog_hierarchical() != Some(true) {
            return Err(RtError::DynamicError(
                "иерархичность справочника не приобретена".into(),
            ));
        }
        let parent = &args[0];
        let valid = parent
            .object_ref()
            .and_then(|value| value.downcast_ref::<ConfigurationReferenceValue>())
            .is_some_and(|reference| {
                reference.object_id == manager.object_id
                    && reference.catalog.identity() == manager.catalog.identity()
            });
        if !valid {
            return Err(RtError::TypeError {
                expected: "Ссылка того же справочника",
                op: "Выбрать",
            });
        }
        predicates.push("Источник.Родитель = &Родитель".into());
        parameters.push((Rc::from("Родитель"), parent.clone()));
    }
    if args
        .get(1)
        .is_some_and(|value| !matches!(value, Value::Undefined))
    {
        return Err(RtError::DynamicError(
            "отбор выборки по владельцу пока не подключён".into(),
        ));
    }
    if let Some(value) = args
        .get(2)
        .filter(|value| !matches!(value, Value::Undefined))
    {
        let Value::Object(value) = value else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Выбрать",
            });
        };
        let BslObject::Structure(value) = value.as_ref() else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Выбрать",
            });
        };
        let value = value.borrow();
        if value.len() > 1 {
            return Err(RtError::DynamicError(
                "отбор выборки должен содержать одно поле".into(),
            ));
        }
        if let Some((name, value)) = value.entry_at(0) {
            let name = context
                .runtime_shapes()
                .names
                .name(name)
                .ok_or(RtError::NotAnObject)?;
            let name = standard_order_field(name)?;
            predicates.push(format!("Источник.{name} = &Отбор"));
            parameters.push((Rc::from("Отбор"), value.clone()));
        }
    }
    let default_order = match object.catalog_default_presentation() {
        Some(CatalogDefaultPresentation::AsCode) => "Код",
        Some(CatalogDefaultPresentation::AsDescription) => "Наименование",
        None => {
            return Err(RtError::DynamicError(
                "основное представление справочника не приобретено".into(),
            ));
        }
    };
    let mut order = default_order.to_owned();
    if let Some(value) = args
        .get(3)
        .filter(|value| !matches!(value, Value::Undefined))
    {
        let Value::Str(value) = value else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Выбрать",
            });
        };
        let text = value.to_string();
        let mut words = text.split_whitespace();
        if let Some(field) = words.next() {
            order = standard_order_field(field)?.to_owned();
            // Неизвестное слово направления измерено как порядок по возрастанию.
            if let Some(direction) = words.next()
                && (folded_eq(direction, "Убыв") || folded_eq(direction, "Desc"))
            {
                order.push_str(" УБЫВ");
            }
            if words.next().is_some() {
                return Err(RtError::DynamicError(
                    "неподдерживаемый порядок выборки".into(),
                ));
            }
        }
    }
    let condition = if predicates.is_empty() {
        String::new()
    } else {
        format!(" ГДЕ {}", predicates.join(" И "))
    };
    let request = QueryRequest {
        text: format!("ВЫБРАТЬ Источник.Ссылка КАК Ссылка, Источник.Код КАК Код, Источник.Наименование КАК Наименование, Источник.ПометкаУдаления КАК ПометкаУдаления ИЗ Справочник.{} КАК Источник{condition} УПОРЯДОЧИТЬ ПО Источник.{order}", object.name()).into(),
        parameters: parameters.into(), temp_tables_manager: None,
    };
    let (principal, adapter) = current_data_access(context)?;
    let data = adapter.execute_query(&request, &principal, context)?;
    let ty = manager
        .catalog
        .catalog_selection_types
        .iter()
        .find(|(id, _)| id == &manager.object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(CatalogSelection {
        catalog: Arc::clone(&manager.catalog),
        object_id: Arc::clone(&manager.object_id),
        ty,
        data,
        next: Cell::new(0),
        current: Cell::new(None),
    }))
}

fn standard_order_field(name: &str) -> RtResult<&'static str> {
    if folded_eq(name, "Код") || folded_eq(name, "Code") {
        Ok("Код")
    } else if folded_eq(name, "Наименование") || folded_eq(name, "Description") {
        Ok("Наименование")
    } else {
        Err(RtError::DynamicError(
            "индексированный реквизит выборки не подключён".into(),
        ))
    }
}
