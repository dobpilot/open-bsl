//! Несохранённый объект бизнес-процесса и собственное состояние его модуля.
use super::*;
use open_bsl::{BslDate, ColumnType, ConfigurationInstance, TypeId};

#[derive(Debug)]
struct Data {
    catalog: Arc<MetadataCatalog>,
    owner: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    ready: Cell<bool>,
    values: RefCell<[Value; 7]>,
    attributes: RefCell<Vec<Value>>,
    sections: RefCell<Vec<Value>>,
    head_task: RefCell<Option<Value>>,
    // После чтения прежние части сохраняют свой флаг и не меняют новый объект.
    modified: RefCell<Rc<Cell<bool>>>,
    exchange: Value,
    new_reference: RefCell<Value>,
    instance: ConfigurationInstance,
}

#[derive(Debug, Clone)]
struct Object(Rc<Data>);

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "БизнесПроцессОбъект",
    type_display: "БизнесПроцессОбъект",
    type_names: &["BusinessProcessObject"],
};

impl ObjectProtocol for Object {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.0.ty))
    }
    fn configuration_instance(&self) -> Option<&ConfigurationInstance> {
        Some(&self.0.instance)
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if PROPERTIES.iter().any(|property| {
            property
                .names
                .iter()
                .any(|candidate| folded_eq(candidate, name))
        }) {
            return get_property_from_table(PROPERTIES, TYPE.name, self, name, context);
        }
        let metadata = self
            .0
            .catalog
            .materialized_object_by_id(&self.0.owner)?
            .ok_or(RtError::NotAnObject)?;
        if let Some(index) = metadata
            .business_process_tabular_sections()
            .into_iter()
            .flatten()
            .position(|section| folded_eq(section.name(), name))
        {
            return Ok(self.0.sections.borrow()[index].clone());
        }
        let index = metadata
            .business_process_attributes()
            .into_iter()
            .flatten()
            .position(|attribute| folded_eq(attribute.name(), name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        Ok(self.0.attributes.borrow()[index].clone())
    }
    fn set_property(
        &self,
        name: &str,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        if PROPERTIES.iter().any(|property| {
            property
                .names
                .iter()
                .any(|candidate| folded_eq(candidate, name))
        }) {
            return set_property_from_table(PROPERTIES, TYPE.name, self, name, value, context);
        }
        let metadata = self
            .0
            .catalog
            .materialized_object_by_id(&self.0.owner)?
            .ok_or(RtError::NotAnObject)?;
        if metadata
            .business_process_tabular_sections()
            .into_iter()
            .flatten()
            .any(|section| folded_eq(section.name(), name))
        {
            return Err(RtError::PropertyReadOnly {
                property: name.into(),
                receiver: TYPE.name,
            });
        }
        let attributes = metadata
            .business_process_attributes()
            .ok_or(RtError::NotAnObject)?;
        let index = attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), name))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        // Собственные поля читаются в теле модуля, но присваивание в этой
        // фазе платформа отклоняет, как подтвердил отдельный контроль.
        if !self.0.ready.get() {
            return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
        }
        self.0.attributes.borrow_mut()[index] = document_object::adjust_attribute_value(
            value,
            &attributes[index],
            &self.0.catalog,
            context,
        )?;
        self.0.modified.borrow().set(true);
        Ok(())
    }
    fn has_method(&self, name: &str) -> bool {
        method_exists(METHODS, name)
            || self
                .0
                .catalog
                .object_by_id(&self.0.owner)
                .is_some_and(|object| object.object_module().is_some())
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        // Новые стандартные методы сохраняют уже доступный экспорт
        // с тем же именем, как существующий путь объекта справочника.
        let standard_export = if [
            "Записать",
            "Write",
            "Прочитать",
            "Read",
            "Модифицированность",
            "Modified",
            "ПолучитьСсылкуНового",
            "GetNewObjectRef",
            "УстановитьСсылкуНового",
            "SetNewObjectRef",
        ]
        .iter()
        .any(|candidate| folded_eq(name, candidate))
            && let Some(module) = self
                .0
                .catalog
                .materialized_object_by_id(&self.0.owner)?
                .and_then(MetadataObject::object_module)
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(METHODS, name) && !standard_export {
            return call_method_from_table(METHODS, TYPE.name, self, name, args, context);
        }
        let module = self
            .0
            .catalog
            .materialized_object_by_id(&self.0.owner)?
            .and_then(MetadataObject::object_module)
            .ok_or_else(|| RtError::UnknownMethod {
                method: name.into(),
                receiver: TYPE.name,
            })?;
        context
            .call_configuration_instance(
                module,
                &self.0.instance,
                name,
                args.to_vec(),
                Value::new_object(self.clone()),
            )
            .map(|(value, _)| value)
    }
}

pub(super) fn create(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let metadata = manager
        .catalog
        .materialized_object_by_id(&manager.object_id)?
        .ok_or_else(|| RtError::DynamicError("метаданные бизнес-процесса отсутствуют".into()))?;
    let number = metadata.business_process_number_settings().ok_or_else(|| {
        RtError::DynamicError("настройки номера бизнес-процесса неизвестны".into())
    })?;
    let attributes = metadata.business_process_attributes().ok_or_else(|| {
        RtError::DynamicError("собственные реквизиты бизнес-процесса неизвестны".into())
    })?;
    let sections = metadata
        .business_process_tabular_sections()
        .ok_or_else(|| {
            RtError::DynamicError("табличные части бизнес-процесса неизвестны".into())
        })?;
    // Начальные значения измерены для строк переменной длины и булевых
    // полей (см. `variable_length_string`). Другие настройки и собственные
    // поля не подменяются поведением документа по аналогии.
    if number.number_type != DocumentNumberType::String
        || number.length != 11
        || !number.variable_length
        || attributes.iter().any(|attribute| !variable_length_string(attribute.types()) && !matches!(attribute.types(),
            [ColumnType { id: TypeRef::Native(TypeId::Boolean), quals }] if quals.is_empty()
        ))
        || sections.iter().any(|section| section.attributes().is_none_or(|attributes| attributes.iter().any(|attribute| !variable_length_string(attribute.types()) && !matches!(attribute.types(), [ColumnType { id: TypeRef::Native(TypeId::Boolean), quals }] if quals.is_empty()))))
    {
        return Err(RtError::DynamicError(
            "начальные значения этой формы объекта бизнес-процесса пока не поддерживаются".into(),
        ));
    }
    let ty = manager
        .catalog
        .business_process_object_types
        .iter()
        .find(|(owner, _)| owner == &manager.object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип объекта бизнес-процесса отсутствует".into()))?;
    let modified = Rc::new(Cell::new(false));
    let (initial_attributes, initial_sections) =
        initial_data(metadata, &manager.catalog, context, &modified)?;
    let reference =
        configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])?;
    let object = Object(Rc::new(Data {
        catalog: Arc::clone(&manager.catalog),
        owner: Arc::clone(&manager.object_id),
        ty,
        ready: Cell::new(false),
        values: RefCell::new([
            Value::Str("".into()),
            Value::Date(BslDate::empty()),
            Value::Boolean(false),
            Value::Boolean(false),
            Value::Boolean(false),
            reference.clone(),
            Value::new_structure(context.runtime_shapes().shapes.empty(), vec![]),
        ]),
        attributes: RefCell::new(initial_attributes),
        sections: RefCell::new(initial_sections),
        head_task: RefCell::new(empty_head_task(metadata, &manager.catalog)?),
        modified: RefCell::new(modified),
        exchange: catalog_object::exchange_parameters_value(),
        new_reference: RefCell::new(reference),
        instance: ConfigurationInstance::new(),
    }));
    let value = Value::new_object(object.clone());
    if let Some(module) = metadata.object_module() {
        context.initialize_configuration_instance(module, &object.0.instance, value.clone())?;
    }
    object.0.ready.set(true);
    Ok(value)
}

/// Строка переменной длины: квалификаторы `[длина, 1]` из нативного Config
/// или сокращённые хостовые `[длина]`. Её начальное значение и проверка
/// заполнения от длины не зависят. Строка фиксированной длины (`[длина, 0]`)
/// дополняется пробелами и сюда не входит: для неё остаётся явная ошибка.
fn variable_length_string(types: &[ColumnType]) -> bool {
    matches!(types, [ColumnType { id: TypeRef::Native(TypeId::String), quals }]
        if matches!(quals.as_slice(), [length] | [length, _] if length.parse::<u32>().is_ok())
            && quals.get(1).is_none_or(|variable| variable == "1"))
}
fn initial_data(
    metadata: &MetadataObject,
    catalog: &Arc<MetadataCatalog>,
    context: &mut CallContext<'_>,
    modified: &Rc<Cell<bool>>,
) -> RtResult<(Vec<Value>, Vec<Value>)> {
    let attributes = metadata.business_process_attributes().ok_or_else(|| {
        RtError::DynamicError("собственные реквизиты бизнес-процесса неизвестны".into())
    })?;
    let sections = metadata
        .business_process_tabular_sections()
        .ok_or_else(|| {
            RtError::DynamicError("табличные части бизнес-процесса неизвестны".into())
        })?;
    Ok((
        attributes
            .iter()
            .map(|attribute| {
                document_object::adjust_attribute_value(
                    Value::Undefined,
                    attribute,
                    catalog,
                    context,
                )
            })
            .collect::<RtResult<Vec<_>>>()?,
        sections
            .iter()
            .map(|section| {
                document_tabular_section::create_tracked(
                    catalog,
                    metadata.id(),
                    section,
                    Rc::clone(modified),
                )
            })
            .collect::<RtResult<Vec<_>>>()?,
    ))
}

fn read(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError(
            "чтение бизнес-процесса при инициализации модуля пока не поддерживается".into(),
        ));
    }
    let metadata = object
        .0
        .catalog
        .materialized_object_by_id(&object.0.owner)?
        .ok_or(RtError::NotAnObject)?;
    let modified = Rc::new(Cell::new(false));
    let (attributes, sections) = initial_data(metadata, &object.0.catalog, context, &modified)?;
    let head_task = empty_head_task(metadata, &object.0.catalog)?;
    // Нативный контроль нового объекта сохраняет состояние модуля и
    // дополнительные свойства. Старые части и строки остаются доступны
    // через прежние ссылки, поэтому части заменяются, а не очищаются.
    // Замеры: `docs/research/fixtures/business-process-object/read-new-module.platform.txt`,
    // `docs/research/fixtures/business-process-object/read-new-section.platform.txt`.
    object.0.values.borrow_mut()[..5].clone_from_slice(&[
        Value::Str("".into()),
        Value::Date(BslDate::empty()),
        Value::Boolean(false),
        Value::Boolean(false),
        Value::Boolean(false),
    ]);
    *object.0.attributes.borrow_mut() = attributes;
    *object.0.sections.borrow_mut() = sections;
    *object.0.head_task.borrow_mut() = head_task;
    *object.0.modified.borrow_mut() = modified;
    Ok(Value::Undefined)
}

fn empty_head_task(
    metadata: &MetadataObject,
    catalog: &Arc<MetadataCatalog>,
) -> RtResult<Option<Value>> {
    metadata
        .business_process_task()
        .map(|task| configuration_reference_value(Arc::clone(catalog), task, [0; 16]))
        .transpose()
}

fn head_task(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?
        .0
        .head_task
        .borrow()
        .clone()
        .ok_or_else(|| {
            RtError::DynamicError("настроенная задача бизнес-процесса неизвестна".into())
        })
}

fn set_head_task(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    // Нативный контроль допускает чтение при инициализации модуля,
    // но отклоняет присваивание до выбора объекта.
    // Замер: `docs/research/fixtures/business-process-object/head-task-module.platform.txt`.
    if !object.0.ready.get() {
        return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
    }
    let metadata = object
        .0
        .catalog
        .materialized_object_by_id(&object.0.owner)?
        .ok_or(RtError::NotAnObject)?;
    let task = metadata.business_process_task().ok_or_else(|| {
        RtError::DynamicError("настроенная задача бизнес-процесса неизвестна".into())
    })?;
    let value = if reference_matches_owner(&value, &object.0.catalog, task, MetadataKind::Task) {
        value
    } else if matches!(value, Value::Undefined | Value::Str(_)) {
        configuration_reference_value(Arc::clone(&object.0.catalog), task, [0; 16])?
    } else {
        return Err(RtError::DynamicError(
            "присваивание ведущей задачи из этого типа пока не поддерживается".into(),
        ));
    };
    *object.0.head_task.borrow_mut() = Some(value);
    object.0.modified.borrow().set(true);
    Ok(())
}

fn object(receiver: &dyn ObjectProtocol) -> RtResult<&Object> {
    receiver
        .downcast_ref::<Object>()
        .ok_or(RtError::NotAnObject)
}

macro_rules! property {
    ($get:ident, $set:ident, $index:expr, $pattern:pat) => {
        fn $get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
            Ok(object(receiver)?.0.values.borrow()[$index].clone())
        }
        fn $set(
            receiver: &dyn ObjectProtocol,
            value: Value,
            _: &mut CallContext<'_>,
        ) -> RtResult<()> {
            let object = object(receiver)?;
            if !object.0.ready.get() {
                return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
            }
            if !matches!(value, $pattern) {
                return Err(RtError::DynamicError(
                    "это присвоение стандартного свойства бизнес-процесса пока не поддерживается"
                        .into(),
                ));
            }
            object.0.values.borrow_mut()[$index] = value;
            object.0.modified.borrow().set(true);
            Ok(())
        }
    };
}
property!(number, set_number, 0, Value::Str(_));
property!(date, set_date, 1, Value::Date(_));
property!(deletion, set_deletion, 2, Value::Boolean(_));
property!(started, set_started, 3, Value::Boolean(_));
property!(completed, set_completed, 4, Value::Boolean(_));
fn reference(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.values.borrow()[5].clone())
}
fn additional(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.values.borrow()[6].clone())
}
fn exchange(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    // Нативный контроль отклоняет доступ к параметрам обмена в теле модуля.
    // После выбора один набор сохраняется вместе с его ссылками даже при Read.
    // Замеры: `docs/research/fixtures/business-process-object/data-exchange-phase.platform.txt`,
    // `docs/research/fixtures/business-process-object/data-exchange.platform.txt`.
    if !object.0.ready.get() {
        return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
    }
    Ok(object.0.exchange.clone())
}

fn is_new(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?;
    Ok(Value::Boolean(true))
}

fn fill(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError(
            "заполнение в теле модуля бизнес-процесса пока не поддерживается".into(),
        ));
    }
    let storage = match &arguments[0] {
        Value::Undefined => None,
        Value::Object(data) => match data.as_ref() {
            BslObject::Structure(storage) => Some(storage),
            _ => {
                return Err(RtError::DynamicError(
                    "это заполнение объекта бизнес-процесса пока не поддерживается".into(),
                ));
            }
        },
        _ => {
            return Err(RtError::DynamicError(
                "это заполнение объекта бизнес-процесса пока не поддерживается".into(),
            ));
        }
    };
    let empty_structure = storage.is_none_or(|storage| storage.borrow().is_empty());
    let module = object
        .0
        .catalog
        .materialized_object_by_id(&object.0.owner)?
        .and_then(MetadataObject::object_module);
    let handler_count = match module {
        Some(module) => {
            context.configuration_function_parameter_count(module, "ОбработкаЗаполнения")?
        }
        None => None,
    };
    if storage.is_none()
        && (handler_count.is_some()
            || !object.0.sections.borrow().is_empty()
            || !object
                .0
                .catalog
                .materialized_object_by_id(&object.0.owner)?
                .and_then(MetadataObject::business_process_attributes)
                .is_some_and(|attributes| {
                    attributes
                        .iter()
                        .any(|attribute| attribute.fill_from_filling_value() == Some(true))
                }))
    {
        return Err(RtError::DynamicError(
            "это заполнение объекта бизнес-процесса пока не поддерживается".into(),
        ));
    }
    let named_section_table = storage.is_some_and(|storage| storage.borrow().len() == 1) && object.0.catalog.materialized_object_by_id(&object.0.owner)?.and_then(MetadataObject::business_process_tabular_sections).into_iter().flatten().any(|section| {
        let field = context.runtime_shapes().names.intern(section.name());
        matches!(arguments[0].get_field(field), Ok(Value::Object(value)) if matches!(value.as_ref(), BslObject::ValueTable(_)))
    });
    if !object.0.sections.borrow().is_empty()
        && !empty_structure
        && (handler_count.is_some() || !named_section_table)
    {
        return Err(RtError::DynamicError(
            "заполнение табличной части бизнес-процесса из этих данных пока не поддерживается"
                .into(),
        ));
    }
    let mut filling_data = arguments[0].clone();
    if let (Some(module), Some(count)) = (module, handler_count) {
        if count != 3 {
            return Err(RtError::DynamicError(
                "эта сигнатура обработчика заполнения бизнес-процесса пока не поддерживается"
                    .into(),
            ));
        }
        // Платформа передаёт пустую структуру как Неопределено. Непустая
        // сохраняет общую память, но замена параметра не заменяет аргумент.
        let handler_data = if empty_structure {
            Value::Undefined
        } else {
            arguments[0].clone()
        };
        let (_, parameters) = context.call_configuration_instance_handler(
            module,
            &object.0.instance,
            "ОбработкаЗаполнения",
            vec![handler_data, Value::Undefined, Value::Boolean(true)],
            Value::new_object(object.clone()),
            open_bsl::ConfigurationInstanceEntryMode::Inherit,
        )?;
        if !matches!(&parameters[1], Value::Undefined) {
            return Err(RtError::DynamicError(
                "изменённый текст заполнения бизнес-процесса пока не поддерживается".into(),
            ));
        }
        if !open_bsl::convert_to_boolean(&parameters[2])? {
            return Ok(Value::Undefined);
        }
        if !object.0.sections.borrow().is_empty()
            && !matches!(&parameters[0], Value::Undefined)
            && !matches!(&parameters[0], Value::Object(value) if matches!(value.as_ref(), BslObject::Structure(storage) if storage.borrow().is_empty()))
        {
            return Err(RtError::DynamicError("стандартное заполнение табличной части из этих данных после обработчика пока не поддерживается".into()));
        }
        if !matches!(&parameters[0], Value::Undefined)
            && !matches!(&parameters[0], Value::Object(value) if matches!(value.as_ref(), BslObject::Structure(_)))
        {
            return Err(RtError::DynamicError(
                "стандартное заполнение из этих данных после обработчика пока не поддерживается"
                    .into(),
            ));
        }
        // `fill-enabled-handler` различает замену параметра и изменение
        // общей структуры: стандартная обработка использует возвращённые данные.
        filling_data = parameters[0].clone();
    }
    let attributes = object
        .0
        .catalog
        .materialized_object_by_id(&object.0.owner)?
        .and_then(MetadataObject::business_process_attributes)
        .ok_or(RtError::NotAnObject)?;
    if attributes
        .iter()
        .any(|attribute| attribute.fill_from_filling_value().is_none())
    {
        return Err(RtError::DynamicError("эта настройка заполнения собственного реквизита бизнес-процесса пока не поддерживается".into()));
    }
    // Нативный `fill-enabled` сохраняет заполненные значения и использует
    // то же приведение строк и Булево, что заполнение реквизитов документа.
    object.0.modified.borrow().set(true);
    attribute_filling::fill(
        attributes
            .iter()
            .enumerate()
            .filter(|(_, attribute)| attribute.fill_from_filling_value() == Some(true)),
        &object.0.attributes,
        &object.0.catalog,
        &filling_data,
        context,
        |value| {
            if matches!(value, Value::Undefined) {
                Ok(())
            } else {
                Err(RtError::DynamicError("это значение заполнения собственного реквизита бизнес-процесса пока не поддерживается".into()))
            }
        },
    )?;
    // Отключённые собственные реквизиты сохраняют прежнее состояние.
    // Стандартные свойства не заполняются из структуры, включая значения
    // неверного типа и имена свойств только для чтения: это измерено отдельно.
    // Пустая структура и измеренный элемент-таблица без обработчика
    // сохраняют строки: содержимое таблицы не копируется в часть.
    Ok(Value::Undefined)
}

fn check_filling(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError(
            "проверка заполнения в теле модуля бизнес-процесса пока не поддерживается".into(),
        ));
    }
    let metadata = object
        .0
        .catalog
        .materialized_object_by_id(&object.0.owner)?
        .ok_or(RtError::NotAnObject)?;
    let settings = metadata
        .business_process_standard_fill_checking()
        .ok_or_else(|| {
            RtError::DynamicError(
                "настройки проверки стандартных реквизитов бизнес-процесса неизвестны".into(),
            )
        })?;
    if settings
        .iter()
        .any(|setting| setting.fill_checking != FillChecking::DontCheck)
        || !object.0.sections.borrow().is_empty()
    {
        return Err(RtError::DynamicError(
            "эта политика проверки бизнес-процесса пока не поддерживается".into(),
        ));
    }
    let attributes = metadata.business_process_attributes().ok_or_else(|| {
        RtError::DynamicError("собственные реквизиты бизнес-процесса неизвестны".into())
    })?;
    let mut paths = Vec::new();
    for attribute in attributes {
        match attribute.fill_checking() {
            Some(FillChecking::DontCheck) => {}
            Some(FillChecking::ShowError) => {
                if !variable_length_string(attribute.types()) {
                    return Err(RtError::DynamicError("проверка этого типа собственного реквизита бизнес-процесса пока не поддерживается".into()));
                }
                paths.push(Value::Str(attribute.name().into()));
            }
            None => {
                return Err(RtError::DynamicError(
                    "настройка проверки собственного реквизита бизнес-процесса неизвестна".into(),
                ));
            }
        }
    }
    let paths = Value::Object(Rc::new(BslObject::Array(RefCell::new(paths))));
    let mut refused = false;
    if let Some(parameters) = attribute_filling::call_check_filling_handler(
        context,
        metadata.object_module(),
        &object.0.instance,
        Value::new_object(object.clone()),
        &paths,
        "эта сигнатура обработчика проверки бизнес-процесса пока не поддерживается",
    )? {
        // Приведение небулева `Отказ` измерено на справочнике и документе
        // (`attribute_filling::handler_refused`); база бизнес-процесса для
        // той же пробы собрана вручную и раннером не воспроизводится, поэтому
        // здесь остаётся явная ошибка вместо переноса правила.
        let Value::Boolean(handler_refused) = parameters[0] else {
            return Err(RtError::DynamicError(
                "небулев отказ обработчика проверки бизнес-процесса пока не поддерживается".into(),
            ));
        };
        // Явный отказ сохраняется даже после очистки списка проверяемых полей.
        refused = handler_refused;
        // Присваивание нового массива параметру не меняет исходный список проверки.
        // Изменения самого исходного массива остаются видимыми стандартной обработке.
        if !matches!(&parameters[1], Value::Object(array) if matches!(array.as_ref(), BslObject::Array(_)))
        {
            return Err(RtError::DynamicError(
                "не-массив в параметре проверки бизнес-процесса пока не поддерживается".into(),
            ));
        }
    }
    let Value::Object(array) = paths else {
        return Err(RtError::NotAnObject);
    };
    let BslObject::Array(array) = array.as_ref() else {
        return Err(RtError::NotAnObject);
    };
    for path in array.borrow().iter() {
        let Value::Str(path) = path else {
            return Err(RtError::DynamicError(
                "этот путь проверки бизнес-процесса пока не поддерживается".into(),
            ));
        };
        let path = path.to_string();
        let index = attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), &path))
            .ok_or_else(|| {
                RtError::DynamicError(
                    "этот путь проверки бизнес-процесса пока не поддерживается".into(),
                )
            })?;
        if !object.0.attributes.borrow()[index].is_filled()? {
            refused = true;
            catalog_object::filling_message(attributes[index].name(), metadata, context)?;
        }
    }
    Ok(Value::Boolean(!refused))
}

fn modified(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
    }
    Ok(Value::Boolean(object.0.modified.borrow().get()))
}

fn new_reference(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
    }
    Ok(object.0.new_reference.borrow().clone())
}

fn set_new_reference(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    if !object.0.ready.get() {
        return Err(RtError::DynamicError("Бизнес-процесс не выбран".into()));
    }
    let value = &arguments[0];
    if !reference_matches_owner(
        value,
        &object.0.catalog,
        &object.0.owner,
        MetadataKind::BusinessProcess,
    ) {
        return Err(RtError::TypeError {
            expected: "ссылка того же бизнес-процесса",
            op: "УстановитьСсылкуНового",
        });
    }
    // Нативные контроли сохраняют эту ссылку после Read, отдельно от
    // пустой ссылки самого объекта и его флага модифицированности.
    // Замеры: `docs/research/fixtures/business-process-object/new-reference.platform.txt`,
    // `docs/research/fixtures/business-process-object/new-reference-module.platform.txt`.
    *object.0.new_reference.borrow_mut() = value.clone();
    Ok(Value::Undefined)
}

fn write(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?;
    Err(RtError::DynamicError(
        "запись объекта бизнес-процесса в информационную базу пока не подключена".into(),
    ))
}

pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьСсылкуНового", "GetNewObjectRef"],
        Arity::exact(0),
        new_reference,
    ),
    MethodDescriptor::new(
        &["УстановитьСсылкуНового", "SetNewObjectRef"],
        Arity::exact(1),
        set_new_reference,
    ),
    MethodDescriptor::new(
        &["Модифицированность", "Modified"],
        Arity::exact(0),
        modified,
    ),
    MethodDescriptor::new(&["Прочитать", "Read"], Arity::exact(0), read),
    MethodDescriptor::new(&["Записать", "Write"], Arity::exact(0), write),
    MethodDescriptor::new(
        &["ПроверитьЗаполнение", "CheckFilling"],
        Arity::exact(0),
        check_filling,
    ),
    MethodDescriptor::new(&["Заполнить", "Fill"], Arity::exact(1), fill),
    MethodDescriptor::new(&["ЭтоНовый", "IsNew"], Arity::exact(0), is_new),
];
pub(super) static MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["СоздатьБизнесПроцесс", "CreateBusinessProcess"],
        Arity::exact(0),
        create,
    ),
    CONFIGURATION_REFERENCE_MANAGER_METHODS[0],
    CONFIGURATION_REFERENCE_MANAGER_METHODS[1],
];
pub(super) static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ОбменДанными", "DataExchange"],
        get: exchange,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВедущаяЗадача", "HeadTask"],
        get: head_task,
        set: Some(set_head_task),
    },
    PropertyDescriptor {
        names: &["Номер", "Number"],
        get: number,
        set: Some(set_number),
    },
    PropertyDescriptor {
        names: &["Дата", "Date"],
        get: date,
        set: Some(set_date),
    },
    PropertyDescriptor {
        names: &["ПометкаУдаления", "DeletionMark"],
        get: deletion,
        set: Some(set_deletion),
    },
    PropertyDescriptor {
        names: &["Стартован", "Started"],
        get: started,
        set: Some(set_started),
    },
    PropertyDescriptor {
        names: &["Завершен", "Completed"],
        get: completed,
        set: Some(set_completed),
    },
    PropertyDescriptor {
        names: &["Ссылка", "Ref"],
        get: reference,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДополнительныеСвойства", "AdditionalProperties"],
        get: additional,
        set: None,
    },
];
