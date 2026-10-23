//! Новый объект плана видов характеристик и его собственный модуль.

use super::*;
use open_bsl::ConfigurationInstance;

#[derive(Debug, Clone)]
struct CharacteristicObject(Rc<Data>);

#[derive(Debug)]
struct Data {
    catalog: Arc<MetadataCatalog>,
    id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    settings: CharacteristicSettings,
    selected: Cell<bool>,
    deletion_mark: Cell<bool>,
    code: RefCell<Value>,
    description: RefCell<Value>,
    value_type: RefCell<Value>,
    own_values: RefCell<Vec<Option<Value>>>,
    exchange: Value,
    additional_properties: Value,
    instance: ConfigurationInstance,
}

impl CharacteristicObject {
    fn attribute(&self, name: &str) -> RtResult<Option<(usize, &DocumentAttribute)>> {
        let metadata = self
            .0
            .catalog
            .materialized_object_by_id(&self.0.id)?
            .ok_or_else(|| {
                RtError::DynamicError("метаданные объекта отсутствуют в каталоге".into())
            })?;
        let attributes = metadata.characteristic_attributes().ok_or_else(|| {
            RtError::DynamicError("состав реквизитов плана видов характеристик неизвестен".into())
        })?;
        Ok(attributes
            .iter()
            .enumerate()
            .find(|(_, attribute)| folded_eq(attribute.name(), name)))
    }

    fn adjust_attribute(
        &self,
        attribute: &DocumentAttribute,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        if !matches!(attribute.types(), [column] if column.id == TypeRef::Native(open_bsl::TypeId::String))
        {
            return Err(RtError::DynamicError(format!(
                "тип реквизита плана видов характеристик «{}» пока не поддерживается",
                attribute.name()
            )));
        }
        document_object::adjust_attribute_value(value, attribute, &self.0.catalog, context)
    }

    fn require_selected(&self) -> RtResult<()> {
        if self.0.selected.get() {
            Ok(())
        } else {
            Err(RtError::DynamicError("не выбран элемент".into()))
        }
    }
}

impl ObjectProtocol for CharacteristicObject {
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
        if let Some((index, attribute)) = self.attribute(name)? {
            if let Some(value) = &self.0.own_values.borrow()[index] {
                return Ok(value.clone());
            }
            let value = self.adjust_attribute(attribute, Value::Undefined, context)?;
            self.0.own_values.borrow_mut()[index] = Some(value.clone());
            return Ok(value);
        }
        Err(RtError::UnknownProperty(name.into()))
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
        if let Some((index, attribute)) = self.attribute(name)? {
            self.require_selected()?;
            self.0.own_values.borrow_mut()[index] =
                Some(self.adjust_attribute(attribute, value, context)?);
            return Ok(());
        }
        Err(RtError::UnknownProperty(name.into()))
    }
    fn has_method(&self, name: &str) -> bool {
        method_exists(METHODS, name)
    }
    fn call_method(
        &self,
        name: &str,
        args: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        if method_exists(METHODS, name) {
            return call_method_from_table(METHODS, TYPE.name, self, name, args, context);
        }
        let module = self
            .0
            .catalog
            .materialized_object_by_id(&self.0.id)?
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
        .ok_or_else(|| {
            RtError::DynamicError("метаданные менеджера отсутствуют в каталоге".into())
        })?;
    let settings = metadata.characteristic_settings().cloned().ok_or_else(|| {
        RtError::DynamicError(
            "настройки плана видов характеристик неизвестны в снимке конфигурации".into(),
        )
    })?;
    let ty = manager
        .catalog
        .characteristic_object_types
        .iter()
        .find(|(id, _)| id == &manager.object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип объекта отсутствует в каталоге".into()))?;
    let object = CharacteristicObject(Rc::new(Data {
        catalog: Arc::clone(&manager.catalog),
        id: Arc::clone(&manager.object_id),
        ty,
        settings,
        selected: Cell::new(false),
        deletion_mark: Cell::new(false),
        code: RefCell::new(Value::Str("".into())),
        description: RefCell::new(Value::Str("".into())),
        value_type: RefCell::new(type_description(&[])),
        own_values: RefCell::new(vec![
            None;
            metadata
                .characteristic_attributes()
                .map_or(0, <[_]>::len)
        ]),
        exchange: catalog_object::exchange_parameters_value(),
        // Структура доступна уже в теле модуля и сохраняется после выбора
        // элемента; каждый экземпляр получает отдельное изменяемое значение.
        additional_properties: Value::new_structure(
            context.runtime_shapes().shapes.empty(),
            vec![],
        ),
        instance: ConfigurationInstance::new(),
    }));
    let value = Value::new_object(object.clone());
    if let Some(module) = metadata.object_module() {
        context.initialize_configuration_instance(module, &object.0.instance, value.clone())?;
    }
    // На платформе состав типов устанавливается после тела модуля, одновременно
    // с выбором нового элемента. В теле описание типов ещё пустое.
    *object.0.value_type.borrow_mut() = type_description(&object.0.settings.value_types);
    object.0.selected.set(true);
    Ok(value)
}

fn type_description(types: &[open_bsl::ColumnType]) -> Value {
    Value::new_qualified_type_description(types.to_vec())
}
fn object(receiver: &dyn ObjectProtocol) -> RtResult<&CharacteristicObject> {
    receiver.downcast_ref().ok_or(RtError::NotAnObject)
}
fn code(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.code.borrow().clone())
}
fn description(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.description.borrow().clone())
}
fn deletion_mark(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(object(receiver)?.0.deletion_mark.get()))
}
fn set_deletion_mark(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    // Нативная проба разрешает чтение в теле модуля, но запись — только
    // после выбора нового элемента. Свойство не записывает объект в базу.
    // Замер: `docs/research/fixtures/characteristic-deletion-mark/measure.platform.txt`.
    object.require_selected()?;
    object
        .0
        .deletion_mark
        .set(open_bsl::convert_to_boolean(&value)?);
    Ok(())
}
fn value_type(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.value_type.borrow().clone())
}
fn exchange(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.exchange.clone())
}
fn additional_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(object(receiver)?.0.additional_properties.clone())
}
fn folder(_: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(false))
}
fn predefined(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?;
    // Здесь представлены только новые элементы. На 1С флаг ложен уже
    // в теле модуля, до выбора элемента; загрузка предопределённых не подключена.
    Ok(Value::Boolean(false))
}
fn reference(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    configuration_reference_value(Arc::clone(&object.0.catalog), &object.0.id, [0; 16])
}
fn assign_string(receiver: &dyn ObjectProtocol, value: Value, description: bool) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    let (length, slot) = if description {
        (object.0.settings.description_length, &object.0.description)
    } else {
        (object.0.settings.code_length, &object.0.code)
    };
    let Value::Str(value) = value else {
        return Err(RtError::DynamicError(
            "преобразование нестрокового реквизита плана видов характеристик пока не подключено"
                .into(),
        ));
    };
    let value = value.to_string();
    *slot.borrow_mut() = Value::Str(open_bsl::BslString::from_utf8_string(
        value.chars().take(length as usize).collect(),
    ));
    Ok(())
}
fn set_code(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    assign_string(receiver, value, false)
}
fn set_description(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    assign_string(receiver, value, true)
}
fn set_value_type(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    let value = match value {
        Value::Undefined => type_description(&object.0.settings.value_types),
        Value::Object(ref value_object)
            if matches!(
                value_object.as_ref(),
                BslObject::TypeDescription(_) | BslObject::QualifiedTypeDescription(_)
            ) =>
        {
            value
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "ОписаниеТипов",
                op: "ТипЗначения",
            });
        }
    };
    *object.0.value_type.borrow_mut() = value;
    Ok(())
}
fn is_new(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?.require_selected()?;
    Ok(Value::Boolean(true))
}

pub(super) static TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектПланаВидовХарактеристикКонфигурации");
pub(super) static MANAGER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["СоздатьЭлемент", "CreateItem"], Arity::exact(0), create),
    MethodDescriptor::new(
        &["ПустаяСсылка", "EmptyRef"],
        Arity::exact(0),
        super::document_empty_ref,
    ),
    MethodDescriptor::new(
        &["ПолучитьСсылку", "GetRef"],
        Arity::range(0, 1),
        super::configuration_get_ref,
    ),
];
pub(super) static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ЭтоНовый", "IsNew"],
    Arity::exact(0),
    is_new,
)];
pub(super) static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ПометкаУдаления", "DeletionMark"],
        get: deletion_mark,
        set: Some(set_deletion_mark),
    },
    PropertyDescriptor {
        names: &["Предопределенный", "Predefined"],
        get: predefined,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДополнительныеСвойства", "AdditionalProperties"],
        get: additional_properties,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбменДанными", "DataExchange"],
        get: exchange,
        set: None,
    },
    PropertyDescriptor {
        names: &["Код", "Code"],
        get: code,
        set: Some(set_code),
    },
    PropertyDescriptor {
        names: &["Наименование", "Description"],
        get: description,
        set: Some(set_description),
    },
    PropertyDescriptor {
        names: &["ТипЗначения", "ValueType"],
        get: value_type,
        set: Some(set_value_type),
    },
    PropertyDescriptor {
        names: &["ЭтоГруппа", "IsFolder"],
        get: folder,
        set: None,
    },
    PropertyDescriptor {
        names: &["Ссылка", "Ref"],
        get: reference,
        set: None,
    },
];
