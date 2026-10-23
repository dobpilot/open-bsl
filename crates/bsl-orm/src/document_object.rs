//! Новый объект документа и собственная память его модуля.

#[path = "document_check_filling.rs"]
mod checking;

use super::*;
use open_bsl::{BslDate, ColumnType, ConfigurationInstance, TypeId, adjust_to_types};

#[derive(Debug)]
struct DocumentObjectData {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    settings: DocumentNumberSettings,
    selected: Cell<bool>,
    number: RefCell<Value>,
    date: RefCell<Value>,
    deletion_mark: Cell<bool>,
    posted: Cell<bool>,
    new_reference: RefCell<Value>,
    attributes: RefCell<Vec<Value>>,
    sections: RefCell<Vec<Value>>,
    additional_properties: Value,
    exchange: Value,
    movements: Option<Value>,
    instance: ConfigurationInstance,
}

#[derive(Debug, Clone)]
struct DocumentObjectValue(Rc<DocumentObjectData>);

impl DocumentObjectValue {
    fn require_selected(&self) -> RtResult<()> {
        if self.0.selected.get() {
            Ok(())
        } else {
            Err(RtError::DynamicError("Документ не выбран".into()))
        }
    }
    fn metadata(&self) -> RtResult<&MetadataObject> {
        self.0
            .catalog
            .materialized_object_by_id(&self.0.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("метаданные документа отсутствуют в каталоге".into())
            })
    }
}

impl ObjectProtocol for DocumentObjectValue {
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
        if let Some(index) = self
            .metadata()?
            .document_tabular_sections()
            .into_iter()
            .flatten()
            .position(|section| folded_eq(section.name(), name))
        {
            return Ok(self.0.sections.borrow()[index].clone());
        }
        let attributes = self.metadata()?.document_attributes().ok_or_else(|| {
            RtError::DynamicError(
                "собственные реквизиты документа неизвестны в снимке конфигурации".into(),
            )
        })?;
        let index = attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), name));
        if let Some(index) = index {
            return Ok(self.0.attributes.borrow()[index].clone());
        }
        let module = self
            .metadata()?
            .object_module()
            .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
        context.read_configuration_instance_variable(module, &self.0.instance, name)
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
        let attributes = self.metadata()?.document_attributes().ok_or_else(|| {
            RtError::DynamicError(
                "собственные реквизиты документа неизвестны в снимке конфигурации".into(),
            )
        })?;
        let index = attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), name));
        let Some(index) = index else {
            let module = self
                .metadata()?
                .object_module()
                .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
            return context.write_configuration_instance_variable(
                module,
                &self.0.instance,
                name,
                value,
            );
        };
        self.require_selected()?;
        self.0.attributes.borrow_mut()[index] =
            adjust_attribute_value(value, &attributes[index], &self.0.catalog, context)?;
        Ok(())
    }
    fn has_method(&self, name: &str) -> bool {
        method_exists(METHODS, name)
            || self
                .metadata()
                .is_ok_and(|object| object.object_module().is_some())
    }
    fn call_method(
        &self,
        name: &str,
        arguments: &[Value],
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        // Платформенные декларации сохраняют ранее доступный экспорт модуля.
        let module_export = if (folded_eq(name, "Заполнить")
            || folded_eq(name, "Fill")
            || folded_eq(name, "Записать")
            || folded_eq(name, "Write")
            || folded_eq(name, "Скопировать")
            || folded_eq(name, "Copy")
            || folded_eq(name, "ПолучитьМакет")
            || folded_eq(name, "GetTemplate")
            || folded_eq(name, "ПроверитьЗаполнение")
            || folded_eq(name, "CheckFilling")
            || folded_eq(name, "Модифицированность")
            || folded_eq(name, "Modified")
            || folded_eq(name, "Заблокирован")
            || folded_eq(name, "IsLocked"))
            && let Some(module) = self.metadata()?.object_module()
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(METHODS, name) && !module_export {
            return call_method_from_table(METHODS, TYPE.name, self, name, arguments, context);
        }
        let module = self
            .metadata()?
            .object_module()
            .ok_or_else(|| RtError::UnknownMethod {
                method: name.into(),
                receiver: TYPE.name,
            })?;
        context
            .call_configuration_instance(
                module,
                &self.0.instance,
                name,
                arguments.to_vec(),
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
            RtError::DynamicError("метаданные документа отсутствуют в каталоге".into())
        })?;
    let settings = metadata.document_number_settings().ok_or_else(|| {
        RtError::DynamicError("настройки номера документа неизвестны в снимке конфигурации".into())
    })?;
    let ty = manager
        .catalog
        .document_object_types
        .iter()
        .find(|(id, _)| id.as_ref() == manager.object_id.as_ref())
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| {
            RtError::DynamicError("тип объекта документа отсутствует в каталоге".into())
        })?;
    let attributes = metadata
        .document_attributes()
        .into_iter()
        .flatten()
        .map(|attribute| {
            adjust_attribute_value(Value::Undefined, attribute, &manager.catalog, context)
        })
        .collect::<RtResult<Vec<_>>>()?;
    let create_sections = || {
        metadata
            .document_tabular_sections()
            .into_iter()
            .flatten()
            .map(|section| {
                document_tabular_section::create(&manager.catalog, &manager.object_id, section)
            })
            .collect::<RtResult<Vec<_>>>()
    };
    let sections = create_sections()?;
    let movements = metadata
        .document_registers()
        .map(|registers| document_movements::create(&manager.catalog, registers))
        .transpose()?;
    let object = DocumentObjectValue(Rc::new(DocumentObjectData {
        catalog: Arc::clone(&manager.catalog),
        object_id: Arc::clone(&manager.object_id),
        ty,
        settings,
        selected: Cell::new(false),
        number: RefCell::new(match settings.number_type {
            DocumentNumberType::String => Value::Str(
                " ".repeat(if metadata.document_number_fixed_length() {
                    settings.number_length as usize
                } else {
                    0
                })
                .as_str()
                .into(),
            ),
            DocumentNumberType::Number => Value::Number(BslNumber::from_i64(0)),
        }),
        date: RefCell::new(Value::Date(BslDate::empty())),
        deletion_mark: Cell::new(false),
        posted: Cell::new(false),
        new_reference: RefCell::new(configuration_reference_value(
            Arc::clone(&manager.catalog),
            &manager.object_id,
            [0; 16],
        )?),
        attributes: RefCell::new(attributes),
        sections: RefCell::new(sections),
        additional_properties: Value::new_structure(
            context.runtime_shapes().shapes.empty(),
            vec![],
        ),
        exchange: catalog_object::exchange_parameters_value(),
        movements,
        instance: ConfigurationInstance::new(),
    }));
    let value = Value::new_object(object.clone());
    if let Some(module) = metadata.object_module() {
        context.initialize_configuration_instance(module, &object.0.instance, value.clone())?;
        // Нативная проба сохраняет часть тела с её строками в переменной
        // модуля, но у выбранного документа появляется новая пустая часть.
        // Замер: `tests/conformance/measure/1c/measure-document-tabular-sections-properties.platform.txt`.
        *object.0.sections.borrow_mut() = create_sections()?;
    }
    // На платформе запись номера и даты и `ЭтоНовый` доступны после тела модуля.
    object.0.selected.set(true);
    Ok(value)
}

fn object(receiver: &dyn ObjectProtocol) -> RtResult<&DocumentObjectValue> {
    receiver
        .downcast_ref::<DocumentObjectValue>()
        .ok_or(RtError::NotAnObject)
}

fn copy(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let source = object(receiver)?;
    source.require_selected()?;
    let manager = ManagerValue {
        catalog: Arc::clone(&source.0.catalog),
        object_id: Arc::clone(&source.0.object_id),
    };
    // Создание выполняет тело модуля заново; его переменные и дополнительные
    // свойства не переносятся с исходного документа.
    let value = create(&manager, &[], context)?;
    let target = value
        .object_ref()
        .and_then(|value| value.downcast_ref::<DocumentObjectValue>())
        .ok_or(RtError::NotAnObject)?;
    *target.0.attributes.borrow_mut() = source.0.attributes.borrow().clone();
    for (source_section, target_section) in source
        .0
        .sections
        .borrow()
        .iter()
        .zip(target.0.sections.borrow().iter())
    {
        document_tabular_section::copy_rows(source_section, target_section, context)?;
    }
    if let Some(module) = source.metadata()?.object_module()
        && context
            .configuration_function_parameter_count(module, "ПриКопировании")?
            .is_some()
    {
        context.call_configuration_instance_handler(
            module,
            &target.0.instance,
            "ПриКопировании",
            vec![Value::new_object(source.clone())],
            value.clone(),
            open_bsl::ConfigurationInstanceEntryMode::Inherit,
        )?;
    }
    Ok(value)
}

pub(super) fn adjust_attribute_value(
    value: Value,
    attribute: &DocumentAttribute,
    catalog: &Arc<MetadataCatalog>,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if matches!(attribute.types(), [column] if column.id == TypeRef::Object(&VALUE_STORAGE_TYPE) && column.quals.is_empty())
    {
        // Измеренный одиночный реквизит всегда содержит хранилище.
        // Примитивы не становятся его содержимым при присваивании.
        return if value
            .object_ref()
            .is_some_and(|object| object.downcast_ref::<ValueStorageValue>().is_some())
        {
            Ok(value)
        } else {
            new_value_storage(context, &[Value::Undefined])
        };
    }
    if let Some((reference, target, kind)) = attribute.types().iter().find_map(|column| {
        catalog.reference_types.iter().find_map(|(id, ty)| {
            if TypeRef::Value(Arc::clone(ty)) != column.id {
                return None;
            }
            catalog
                .object_by_id(id)
                .map(|object| (column, id.as_ref(), object.kind()))
        })
    }) {
        // Определяемый тип основания события УНФ содержит 56 ссылок.
        // Размер множества не меняет правила: проверяется каждый его член.
        let mixed_references = attribute
            .types()
            .iter()
            .filter(|column| {
                catalog.reference_types.iter().any(|(id, ty)| {
                    TypeRef::Value(Arc::clone(ty)) == column.id
                        && catalog.object_by_id(id).is_some_and(|object| {
                            matches!(
                                object.kind(),
                                MetadataKind::Document | MetadataKind::Catalog
                            )
                        })
                })
            })
            .count()
            >= 2;
        let supported_mixed = mixed_references
            && attribute.types().iter().all(|column| {
                column.id == TypeRef::Native(TypeId::String)
                    || (column.quals.is_empty()
                        && catalog.reference_types.iter().any(|(id, ty)| {
                            TypeRef::Value(Arc::clone(ty)) == column.id
                                && catalog.object_by_id(id).is_some_and(|object| {
                                    matches!(
                                        object.kind(),
                                        MetadataKind::Document | MetadataKind::Catalog
                                    )
                                })
                        }))
            });
        let supported = supported_mixed
            || (reference.quals.is_empty()
                && (attribute.types().len() == 1
                    || (attribute.types().len() == 2
                        && attribute
                            .types()
                            .iter()
                            .any(|column| column.id == TypeRef::Native(TypeId::String)))));
        if !supported {
            return Err(RtError::DynamicError(format!(
                "тип собственного реквизита документа «{}» пока не поддерживается",
                attribute.name()
            )));
        }
        let value_type = value.type_of()?;
        if !matches!(
            &value,
            Value::Undefined | Value::Null | Value::Number(_) | Value::Boolean(_) | Value::Str(_)
        ) && !((kind != MetadataKind::Catalog || supported_mixed)
            && matches!(&value, Value::Enum(_)))
            && !attribute
                .types()
                .iter()
                .any(|column| value_type == Value::Type(column.id.clone()))
        {
            return Err(RtError::DynamicError(format!(
                "присваивание реквизита «{}» из этого типа пока не поддерживается",
                attribute.name()
            )));
        }
        // Измеренное системное перечисление в составном типе приводится
        // к строке тем же форматтером, что и обычная функция `Строка`.
        let value = if (kind != MetadataKind::Catalog || supported_mixed)
            && attribute.types().len() > 1
            && matches!(&value, Value::Enum(_))
        {
            Value::Str(open_bsl::format_value(&value, None)?.as_str().into())
        } else {
            value
        };
        let value = adjust_to_types(value, attribute.types());
        // У одиночной ссылки умолчание — пустая ссылка её собственного типа,
        // а у измеренного составного реквизита — `Неопределено`.
        return if attribute.types().len() == 1 && matches!(value, Value::Undefined) {
            if kind == MetadataKind::Enumeration {
                enumeration_reference_value(Arc::clone(catalog), target, "")
            } else {
                configuration_reference_value(Arc::clone(catalog), target, [0; 16])
            }
        } else {
            Ok(value)
        };
    }
    let id = match attribute.types() {
        [
            ColumnType {
                id: TypeRef::Native(TypeId::String),
                ..
            },
        ] => TypeId::String,
        [
            ColumnType {
                id: TypeRef::Native(TypeId::Boolean),
                quals,
            },
        ] if quals.is_empty() => TypeId::Boolean,
        [
            ColumnType {
                id: TypeRef::Native(TypeId::Date),
                quals,
            },
        ] if quals.is_empty() || quals.as_slice() == ["DT"] => TypeId::Date,
        _ => {
            return Err(RtError::DynamicError(format!(
                "тип собственного реквизита документа «{}» пока не поддерживается",
                attribute.name()
            )));
        }
    };
    if id != TypeId::String
        && !matches!(
            &value,
            Value::Undefined
                | Value::Null
                | Value::Boolean(_)
                | Value::Number(_)
                | Value::Date(_)
                | Value::Str(_)
        )
        && !matches!(&value, Value::Object(object) if matches!(object.as_ref(), BslObject::Array(_) | BslObject::Structure(_)))
    {
        return Err(RtError::DynamicError(format!(
            "присваивание реквизита «{}» из этого типа пока не поддерживается",
            attribute.name()
        )));
    }
    // Присваивание реквизиту принимает булев текст и цифровую строку даты,
    // хотя общее приведение ячейки таблицы значений их не преобразует.
    let value = match (id, value) {
        (TypeId::Boolean, value) => {
            Value::Boolean(open_bsl::convert_to_boolean(&value).unwrap_or(false))
        }
        (TypeId::Date, Value::Str(text)) => {
            Value::Date(BslDate::parse_digits(&text.to_string()).unwrap_or_else(BslDate::empty))
        }
        (_, value) => value,
    };
    Ok(adjust_to_types(value, attribute.types()))
}

fn is_new(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?.require_selected()?;
    Ok(Value::Boolean(true))
}

fn modified(_: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    // Сигнатура измерена на платформе; отслеживание изменений документа
    // пока не подключено к памяти объекта.
    Err(RtError::DynamicError(
        "Модифицированность: отслеживание изменений документа не подключено".into(),
    ))
}

fn is_locked(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    object(receiver)?.require_selected()?;
    // Здесь представлены только новые документы без собственной блокировки.
    // Ложь после выбора и ошибку в теле модуля подтверждает нативный контроль
    // (`docs/research/fixtures/document-lock-state-native-1015/platform.oracle`).
    Ok(Value::Boolean(false))
}

fn new_reference(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    Ok(object.0.new_reference.borrow().clone())
}

fn set_new_reference(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let value = &arguments[0];
    if !reference_matches_owner(
        value,
        &object.0.catalog,
        &object.0.object_id,
        MetadataKind::Document,
    ) {
        return Err(RtError::TypeError {
            expected: "ссылка того же документа",
            op: "УстановитьСсылкуНового",
        });
    }
    // Назначается ссылка будущей записи. Нативная проба сохраняет пустую
    // `Ссылка` самого объекта; отклонённое значение не меняет прежнее.
    // Замер: `tests/conformance/measure/1c/measure-document-new-reference.platform.txt`.
    *object.0.new_reference.borrow_mut() = value.clone();
    Ok(Value::Undefined)
}

fn reference(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    configuration_reference_value(Arc::clone(&object.0.catalog), &object.0.object_id, [0; 16])
}

fn number(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.number.borrow().clone())
}

fn movements(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?.0.movements.clone().ok_or_else(|| {
        RtError::DynamicError("регистры движений документа неизвестны в снимке конфигурации".into())
    })
}

fn date(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.date.borrow().clone())
}

fn point_in_time(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let Value::Date(date) = *object.0.date.borrow() else {
        return Err(RtError::DynamicError(
            "дата документа не является датой".into(),
        ));
    };
    // Значение фиксирует дату и текущую ссылку объекта на момент вызова.
    open_bsl::new_point_in_time(date, reference(receiver, context)?)
}

fn additional_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(object(receiver)?.0.additional_properties.clone())
}

fn exchange(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    // До выбора документа платформа возвращает `Неопределено`, а не объект обмена.
    if object.0.selected.get() {
        Ok(object.0.exchange.clone())
    } else {
        Ok(Value::Undefined)
    }
}

fn set_number(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    let settings = object.0.settings;
    let id = match settings.number_type {
        DocumentNumberType::String => TypeId::String,
        DocumentNumberType::Number => TypeId::Number,
    };
    let quals = if settings.number_length == 0 {
        vec![]
    } else {
        // Фиксированная длина номера применяется отдельно по метаданным ниже.
        vec![
            settings.number_length.to_string(),
            if id == TypeId::String { "1" } else { "0" }.into(),
        ]
    };
    let mut adjusted = adjust_to_types(
        value,
        &[ColumnType {
            id: TypeRef::Native(id),
            quals,
        }],
    );
    // Номер документа, как показала отдельная проба, не становится отрицательным.
    if let Value::Number(number) = &adjusted
        && number.cmp(&BslNumber::from_i64(0)) == std::cmp::Ordering::Less
    {
        adjusted = Value::Number(BslNumber::from_i64(0));
    }
    if object.metadata()?.document_number_fixed_length()
        && let Value::Str(text) = &adjusted
    {
        // Длина измеряется в UTF-16, как при штатном обрезании номера.
        let padding = (settings.number_length as usize).saturating_sub(text.len_utf16());
        adjusted = Value::Str(text.concat(&" ".repeat(padding).as_str().into()));
    }
    *object.0.number.borrow_mut() = adjusted;
    Ok(())
}

fn set_date(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    *object.0.date.borrow_mut() = match value {
        Value::Date(_) => value,
        Value::Str(text) => {
            Value::Date(BslDate::parse_digits(&text.to_string()).unwrap_or_else(BslDate::empty))
        }
        Value::Undefined | Value::Null | Value::Number(_) | Value::Boolean(_) => {
            Value::Date(BslDate::empty())
        }
        Value::Object(ref object) if matches!(object.as_ref(), BslObject::Array(_)) => {
            Value::Date(BslDate::empty())
        }
        _ => {
            return Err(RtError::DynamicError(
                "присваивание даты документа из этого типа пока не поддерживается".into(),
            ));
        }
    };
    Ok(())
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
    object.require_selected()?;
    object
        .0
        .deletion_mark
        .set(open_bsl::convert_to_boolean(&value)?);
    Ok(())
}

fn posted(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(object(receiver)?.0.posted.get()))
}
fn set_posted(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    object.0.posted.set(open_bsl::convert_to_boolean(&value)?);
    Ok(())
}

fn set_new_number(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let mut prefix = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| Value::Str("".into()));
    let mut standard_processing = true;
    if let Some(module) = object.metadata()?.object_module()
        && let Some(count) =
            context.configuration_function_parameter_count(module, "ПриУстановкеНовогоНомера")?
    {
        let arguments = match count {
            1 => vec![Value::Boolean(true)],
            2 => vec![Value::Boolean(true), prefix.clone()],
            _ => {
                return Err(RtError::DynamicError(
                    "число параметров ПриУстановкеНовогоНомера пока не поддерживается".into(),
                ));
            }
        };
        let (_, params) = context.call_configuration_instance_handler(
            module,
            &object.0.instance,
            "ПриУстановкеНовогоНомера",
            arguments,
            Value::new_object(object.clone()),
            open_bsl::ConfigurationInstanceEntryMode::Inherit,
        )?;
        standard_processing = open_bsl::convert_to_boolean(&params[0])?;
        if count == 2 {
            prefix = params[1].clone();
        }
    }
    if !standard_processing {
        return Ok(Value::Undefined);
    }
    let Value::Str(prefix) = prefix else {
        return Err(RtError::DynamicError(
            "выдача номера с нестроковым префиксом пока не поддерживается".into(),
        ));
    };
    let Value::Date(date) = *object.0.date.borrow() else {
        return Err(RtError::DynamicError("дата документа недоступна".into()));
    };
    // Резервирование принадлежит адаптеру этого сеанса, а не экземпляру
    // документа: дата и изменённый обработчиком префикс определяют его область.
    let (principal, adapter) = current_data_access(context)?;
    let value = adapter.allocate_document_number(
        object.metadata()?,
        &principal,
        date,
        &prefix.to_string(),
        context,
    )?;
    set_number(receiver, value, context)?;
    Ok(Value::Undefined)
}

fn fill(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let mut standard_processing = true;
    let mut filling_data = arguments[0].clone();
    if let Some(module) = object.metadata()?.object_module()
        && let Some(count) =
            context.configuration_function_parameter_count(module, "ОбработкаЗаполнения")?
    {
        let mut params = vec![arguments[0].clone()];
        match count {
            2 => {}
            3 => params.push(Value::Undefined),
            _ => {
                return Err(RtError::DynamicError(
                    "число параметров ОбработкаЗаполнения не поддерживается".into(),
                ));
            }
        }
        params.push(Value::Boolean(true));
        let (_, params) = context.call_configuration_instance_handler(
            module,
            &object.0.instance,
            "ОбработкаЗаполнения",
            params,
            Value::new_object(object.clone()),
            open_bsl::ConfigurationInstanceEntryMode::Inherit,
        )?;
        // После обработчика платформа заполняет реквизиты из возвращённого параметра.
        filling_data = params[0].clone();
        standard_processing = open_bsl::convert_to_boolean(&params[count - 1])?;
    }
    if !standard_processing {
        return Ok(Value::Undefined);
    }
    let attributes = object.metadata()?.document_attributes().ok_or_else(|| {
        RtError::DynamicError(
            "настройки заполнения реквизитов документа неизвестны в снимке конфигурации".into(),
        )
    })?;
    if !attributes.is_empty()
        && !matches!(&filling_data, Value::Undefined)
        && !matches!(&filling_data, Value::Object(value) if matches!(value.as_ref(), BslObject::Structure(_)))
    {
        return Err(RtError::DynamicError(
            "стандартное заполнение собственных реквизитов из этого типа пока не поддерживается"
                .into(),
        ));
    }
    attribute_filling::fill(
        attributes.iter().enumerate(),
        &object.0.attributes,
        &object.0.catalog,
        &filling_data,
        context,
        |_| Ok(()),
    )?;
    Ok(Value::Undefined)
}

pub(super) static TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ДокументОбъект",
    type_display: "Документ объект",
    type_names: &["DocumentObject"],
};
fn write(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?;
    Err(RtError::DynamicError(
        "запись объекта документа в информационную базу пока не подключена".into(),
    ))
}

pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["ПолучитьМакет", "GetTemplate"],
        Arity::exact(1),
        document_get_template,
    ),
    MethodDescriptor::new(&["Скопировать", "Copy"], Arity::exact(0), copy),
    MethodDescriptor::new(
        &["ПроверитьЗаполнение", "CheckFilling"],
        Arity::exact(0),
        checking::check_filling,
    ),
    MethodDescriptor::new(
        &["МоментВремени", "PointInTime"],
        Arity::exact(0),
        point_in_time,
    ),
    MethodDescriptor::new(&["Записать", "Write"], Arity::range(0, 2), write),
    MethodDescriptor::new(
        &["УстановитьНовыйНомер", "SetNewNumber"],
        Arity::range(0, 1),
        set_new_number,
    ),
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
    MethodDescriptor::new(&["Заполнить", "Fill"], Arity::exact(1), fill),
    MethodDescriptor::new(&["ЭтоНовый", "IsNew"], Arity::exact(0), is_new),
    MethodDescriptor::new(
        &["Модифицированность", "Modified"],
        Arity::exact(0),
        modified,
    ),
    MethodDescriptor::new(&["Заблокирован", "IsLocked"], Arity::exact(0), is_locked),
];
pub(super) static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Движения", "RegisterRecords"],
        get: movements,
        set: None,
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
        get: deletion_mark,
        set: Some(set_deletion_mark),
    },
    PropertyDescriptor {
        names: &["Проведен", "Posted"],
        get: posted,
        set: Some(set_posted),
    },
    PropertyDescriptor {
        names: &["Ссылка", "Ref"],
        get: reference,
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
];

/// Возвращает владельца для общего чтения файлового макета.
pub(super) fn file_owner(receiver: &dyn ObjectProtocol) -> Option<(&Arc<MetadataCatalog>, &str)> {
    let object = receiver.downcast_ref::<DocumentObjectValue>()?;
    Some((&object.0.catalog, object.0.object_id.as_ref()))
}
