//! Новый объект справочника и принадлежащая ему память модуля объекта.

use super::*;
use open_bsl::{ColumnType, ConfigurationInstance, TypeId, adjust_to_types};

#[derive(Debug)]
struct CatalogObjectData {
    catalog: Arc<MetadataCatalog>,
    object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    settings: CatalogStandardAttributes,
    selected: Cell<bool>,
    is_folder: Cell<bool>,
    deletion_mark: Cell<bool>,
    predefined_name: RefCell<Value>,
    code: RefCell<Value>,
    description: RefCell<Value>,
    parent: RefCell<Value>,
    owner: RefCell<Option<Value>>,
    common_values: RefCell<Vec<Option<Value>>>,
    own_values: RefCell<Vec<Option<Value>>>,
    new_reference: RefCell<Value>,
    additional_properties: Value,
    exchange: Value,
    instance: ConfigurationInstance,
}

#[derive(Debug, Clone)]
struct CatalogObjectValue(Rc<CatalogObjectData>);

impl CatalogObjectValue {
    fn own_attribute_index(&self, name: &str) -> RtResult<Option<usize>> {
        Ok(self
            .metadata()?
            .catalog_attributes()
            .into_iter()
            .flatten()
            .position(|attribute| folded_eq(attribute.name(), name)))
    }

    fn adjust_own_attribute(
        &self,
        index: usize,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let attribute = &self.metadata()?.catalog_attributes().ok_or_else(|| {
            RtError::DynamicError("типы собственных реквизитов справочника неизвестны".into())
        })?[index];
        if !matches!(attribute.types(), [column] if matches!(column.id,
            TypeRef::Native(TypeId::String | TypeId::Number | TypeId::Boolean | TypeId::Date)))
        {
            return Err(RtError::DynamicError(format!(
                "тип собственного реквизита «{}» пока не поддерживается",
                attribute.name()
            )));
        }
        if matches!(attribute.types(), [column] if column.id == TypeRef::Native(TypeId::Number)) {
            // Помощник документа пока не принимает числовые реквизиты.
            // Измеренные ноль и число используют существующее приведение типов.
            return Ok(adjust_to_types(value, attribute.types()));
        }
        document_object::adjust_attribute_value(value, attribute, &self.0.catalog, context)
    }

    fn common_attribute_index(&self, name: &str) -> RtResult<Option<usize>> {
        Ok(self
            .metadata()?
            .catalog_common_attributes()
            .into_iter()
            .flatten()
            .position(|attribute| folded_eq(attribute.name(), name)))
    }

    fn adjust_common_attribute(
        &self,
        index: usize,
        value: Value,
        context: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        let attribute = &self
            .metadata()?
            .catalog_common_attributes()
            .ok_or_else(|| {
                RtError::DynamicError(
                    "общие реквизиты справочника неизвестны в снимке конфигурации".into(),
                )
            })?[index];
        if !matches!(attribute.types(), [column] if column.id == TypeRef::Native(TypeId::String)) {
            return Err(RtError::DynamicError(format!(
                "тип общего реквизита «{}» пока не поддерживается",
                attribute.name()
            )));
        }
        // Нативная строковая проба совпала с конверсией существующего реквизита.
        // Один помощник сохраняет приобретённые квалификаторы обоих случаев.
        document_object::adjust_attribute_value(value, attribute, &self.0.catalog, context)
    }

    fn unavailable_attribute(&self, name: &str) -> RtResult<()> {
        if let Some(attribute) = self
            .metadata()?
            .catalog_attribute_names()
            .into_iter()
            .flatten()
            .find(|attribute| folded_eq(attribute, name))
        {
            return Err(RtError::DynamicError(format!(
                "реквизит справочника «{attribute}»: тип и значение пока не реализованы"
            )));
        }
        if let Some(section) = self
            .metadata()?
            .catalog_tabular_section_names()
            .into_iter()
            .flatten()
            .find(|section| folded_eq(section, name))
        {
            return Err(RtError::DynamicError(format!(
                "табличная часть справочника «{section}»: тип и строки пока не реализованы"
            )));
        }
        Ok(())
    }

    fn require_selected(&self) -> RtResult<()> {
        if self.0.selected.get() {
            Ok(())
        } else {
            Err(RtError::DynamicError(
                "Элемент справочника не выбран".into(),
            ))
        }
    }

    fn metadata(&self) -> RtResult<&MetadataObject> {
        self.0
            .catalog
            .materialized_object_by_id(&self.0.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("метаданные объекта отсутствуют в каталоге".into())
            })
    }
}

impl ObjectProtocol for CatalogObjectValue {
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
        if let Some(index) = self.own_attribute_index(name)? {
            if let Some(value) = &self.0.own_values.borrow()[index] {
                return Ok(value.clone());
            }
            let value = self.adjust_own_attribute(index, Value::Undefined, context)?;
            self.0.own_values.borrow_mut()[index] = Some(value.clone());
            return Ok(value);
        }
        if let Some(index) = self.common_attribute_index(name)? {
            if let Some(value) = &self.0.common_values.borrow()[index] {
                return Ok(value.clone());
            }
            let value = self.adjust_common_attribute(index, Value::Undefined, context)?;
            self.0.common_values.borrow_mut()[index] = Some(value.clone());
            return Ok(value);
        }
        self.unavailable_attribute(name)?;
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
            return open_bsl::set_property_from_table(
                PROPERTIES, TYPE.name, self, name, value, context,
            );
        }
        if let Some(index) = self.own_attribute_index(name)? {
            self.require_selected()?;
            let value = self.adjust_own_attribute(index, value, context)?;
            self.0.own_values.borrow_mut()[index] = Some(value);
            return Ok(());
        }
        if let Some(index) = self.common_attribute_index(name)? {
            self.require_selected()?;
            let value = self.adjust_common_attribute(index, value, context)?;
            self.0.common_values.borrow_mut()[index] = Some(value);
            return Ok(());
        }
        self.unavailable_attribute(name)?;
        Err(RtError::UnknownProperty(name.into()))
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
        // Новые имена записи, проверки, блокировки, уровня и выдачи кода
        // сохраняют экспорт модуля.
        // Остальные платформенные методы используют прежний порядок.
        let write_export = if (folded_eq(name, "Записать")
            || folded_eq(name, "Write")
            || folded_eq(name, "ПроверитьЗаполнение")
            || folded_eq(name, "CheckFilling")
            || folded_eq(name, "Заблокировать")
            || folded_eq(name, "Lock")
            || folded_eq(name, "Уровень")
            || folded_eq(name, "Level")
            || folded_eq(name, "УстановитьНовыйКод")
            || folded_eq(name, "SetNewCode"))
            && let Some(module) = self.metadata()?.object_module()
        {
            context.configuration_has_export(module, name)?
        } else {
            false
        };
        if method_exists(METHODS, name) && !write_export {
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

    fn presentation(&self) -> RtResult<String> {
        let value = match self.metadata()?.catalog_default_presentation() {
            Some(CatalogDefaultPresentation::AsCode) => self.0.code.borrow(),
            Some(CatalogDefaultPresentation::AsDescription) => self.0.description.borrow(),
            None => {
                return Err(RtError::DynamicError(
                    "основное представление справочника неизвестно в снимке конфигурации".into(),
                ));
            }
        };
        open_bsl::format_value(&value, None)
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
    let settings = metadata.catalog_standard_attributes().ok_or_else(|| {
        RtError::DynamicError(
            "стандартные реквизиты справочника неизвестны в снимке конфигурации".into(),
        )
    })?;
    let ty = manager
        .catalog
        .catalog_object_types
        .iter()
        .find(|(id, _)| id.as_ref() == manager.object_id.as_ref())
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип объекта отсутствует в каталоге".into()))?;
    let object = CatalogObjectValue(Rc::new(CatalogObjectData {
        catalog: Arc::clone(&manager.catalog),
        object_id: Arc::clone(&manager.object_id),
        ty,
        settings,
        selected: Cell::new(false),
        is_folder: Cell::new(false),
        deletion_mark: Cell::new(false),
        predefined_name: RefCell::new(Value::Str("".into())),
        code: RefCell::new(match settings.code_type {
            CatalogCodeType::Number => Value::Number(BslNumber::from_i64(0)),
            CatalogCodeType::String => Value::Str("".into()),
        }),
        description: RefCell::new(Value::Str("".into())),
        parent: RefCell::new(catalog_reference_value(
            Arc::clone(&manager.catalog),
            &manager.object_id,
            [0; 16],
        )?),
        owner: RefCell::new(None),
        common_values: RefCell::new(vec![
            None;
            metadata
                .catalog_common_attributes()
                .map_or(0, <[_]>::len)
        ]),
        own_values: RefCell::new(vec![
            None;
            metadata.catalog_attributes().map_or(0, <[_]>::len)
        ]),
        new_reference: RefCell::new(catalog_reference_value(
            Arc::clone(&manager.catalog),
            &manager.object_id,
            [0; 16],
        )?),
        additional_properties: Value::new_structure(
            context.runtime_shapes().shapes.empty(),
            vec![],
        ),
        exchange: exchange_parameters_value(),
        instance: ConfigurationInstance::new(),
    }));
    let value = Value::new_object(object.clone());
    if let Some(module) = metadata.object_module() {
        // Тело модуля выполняется при создании, а не при первом вызове экспорта.
        // Получатель и переменные принадлежат этому экземпляру объекта.
        context.initialize_configuration_instance(module, &object.0.instance, value.clone())?;
    }
    // На 1С тело модуля уже видит стандартные реквизиты для чтения, но запись
    // и `ЭтоНовый` становятся доступны только после выбора нового элемента.
    object.0.selected.set(true);
    Ok(value)
}

pub(super) fn create_folder(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
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
    let hierarchical = metadata.catalog_hierarchical().ok_or_else(|| {
        RtError::DynamicError(
            "признак иерархии справочника неизвестен в снимке конфигурации".into(),
        )
    })?;
    match metadata.catalog_hierarchy_type() {
        Some(CatalogHierarchyType::FoldersAndItems) => {}
        Some(CatalogHierarchyType::Items) => {
            return Err(RtError::DynamicError(
                "справочник не может содержать группы".into(),
            ));
        }
        None => {
            return Err(RtError::DynamicError(
                "вид иерархии справочника неизвестен в снимке конфигурации".into(),
            ));
        }
    }
    let value = create(receiver, arguments, context)?;
    let object = value
        .object_ref()
        .and_then(|value| value.downcast_ref::<CatalogObjectValue>())
        .ok_or(RtError::NotAnObject)?;
    // На платформе тело группы ещё видит `ЭтоГруппа=false`. Признак меняется
    // после той же инициализации, которую проходит обычный новый элемент.
    object.0.is_folder.set(hierarchical);
    Ok(value)
}

fn object(receiver: &dyn ObjectProtocol) -> RtResult<&CatalogObjectValue> {
    receiver
        .downcast_ref::<CatalogObjectValue>()
        .ok_or(RtError::NotAnObject)
}

fn is_new(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?.require_selected()?;
    Ok(Value::Boolean(true))
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
    let [value] = arguments else {
        unreachable!("таблица методов проверяет арность");
    };
    let compatible = reference_matches_owner(
        value,
        &object.0.catalog,
        &object.0.object_id,
        MetadataKind::Catalog,
    );
    if !compatible {
        return Err(RtError::TypeError {
            expected: "ссылка того же справочника",
            op: "УстановитьСсылкуНового",
        });
    }
    // Значение назначается будущей записи. `Ссылка` нового объекта остаётся
    // пустой; уникальность проверяется платформой при записи, не здесь.
    *object.0.new_reference.borrow_mut() = value.clone();
    Ok(Value::Undefined)
}

fn metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    Ok(metadata_object_value(&object.0.catalog, object.metadata()?))
}

fn reference(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    catalog_reference_value(Arc::clone(&object.0.catalog), &object.0.object_id, [0; 16])
}

fn is_folder(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    // Стандартный признак нового элемента читается и в теле модуля,
    // до выбора объекта. Отдельная проверка `selected` здесь не нужна.
    Ok(Value::Boolean(object(receiver)?.0.is_folder.get()))
}

fn owner_ids(object: &CatalogObjectValue) -> RtResult<&[String]> {
    let owners = object.metadata()?.catalog_owners().ok_or_else(|| {
        RtError::DynamicError("владельцы справочника неизвестны в снимке конфигурации".into())
    })?;
    for id in owners {
        if !object
            .0
            .catalog
            .materialized_object_by_id(id)?
            .is_some_and(|metadata| metadata.kind() == MetadataKind::Catalog)
        {
            return Err(RtError::DynamicError(format!(
                "справочник-владелец «{id}» отсутствует в снимке конфигурации"
            )));
        }
    }
    Ok(owners)
}

fn initial_owner(object: &CatalogObjectValue, owners: &[String]) -> RtResult<Value> {
    match owners {
        [id] => catalog_reference_value(Arc::clone(&object.0.catalog), id, [0; 16]),
        _ => Ok(Value::Undefined),
    }
}

fn owner(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let object = object(receiver)?;
    let owners = owner_ids(object)?;
    if let Some(value) = &*object.0.owner.borrow() {
        return Ok(value.clone());
    }
    // Неизвестный список владельцев не мешает операциям с другими реквизитами.
    // Начальное значение вычисляется лишь при первом обращении к `Владелец`.
    let value = initial_owner(object, owners)?;
    *object.0.owner.borrow_mut() = Some(value.clone());
    Ok(value)
}

fn set_owner(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    if object.0.is_folder.get() {
        return Err(RtError::DynamicError(
            "назначение владельца группы пока не поддерживается".into(),
        ));
    }
    let owners = owner_ids(object)?;
    if owners.is_empty() {
        return Err(RtError::PropertyReadOnly {
            property: "Владелец".into(),
            receiver: TYPE.name,
        });
    }
    // Измеренная конверсия сохраняет ссылку допустимого владельца. Иначе
    // получается пустая ссылка единственного типа либо `Неопределено`.
    let value = if owners
        .iter()
        .any(|id| reference_matches_owner(&value, &object.0.catalog, id, MetadataKind::Catalog))
    {
        value
    } else {
        initial_owner(object, owners)?
    };
    *object.0.owner.borrow_mut() = Some(value);
    Ok(())
}

fn predefined(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    // Признак определяется существующим именем, отдельного состояния нет.
    let object = object(receiver)?;
    Ok(Value::Boolean(
        matches!(&*object.0.predefined_name.borrow(), Value::Str(name) if name.len_utf16() != 0),
    ))
}

fn predefined_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.predefined_name.borrow().clone())
}

fn set_predefined_name(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    let name = match value {
        Value::Str(name) => name,
        Value::Undefined | Value::Null => "".into(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Имя предопределённых данных",
                op: "ИмяПредопределенныхДанных",
            });
        }
    };
    let text = name.to_string();
    if !text.is_empty()
        && !object
            .metadata()?
            .catalog_predefined_names()
            .iter()
            .any(|known| folded_eq(known, &text))
    {
        return Err(RtError::DynamicError(
            "недопустимое имя предопределённых данных".into(),
        ));
    }
    // Проверка не заменяет пользовательское написание каноническим именем.
    // Присваивание меняет только память объекта, не строку информационной базы.
    *object.0.predefined_name.borrow_mut() = Value::Str(name);
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
    // Используем преобразование `Булево`, но измеренный отказ здесь даёт ложь.
    // Меняется память объекта; запись в информационную базу здесь не выполняется.
    let value = open_bsl::convert_to_boolean(&value).unwrap_or(false);
    object.0.deletion_mark.set(value);
    Ok(())
}

fn code(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.code.borrow().clone())
}

fn set_code(receiver: &dyn ObjectProtocol, value: Value, _: &mut CallContext<'_>) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    let settings = object.0.settings;
    if settings.code_length == 0 {
        return Err(RtError::DynamicError(
            "присваивание кода нулевой длины пока не поддерживается".into(),
        ));
    }
    let id = match settings.code_type {
        CatalogCodeType::Number => TypeId::Number,
        CatalogCodeType::String => TypeId::String,
    };
    let mut value = adjust_to_types(
        value,
        &[ColumnType {
            id: TypeRef::Native(id),
            // Нативный контроль кода сохраняет короткую строку без пробелов
            // (`tests/conformance/measure/1c/measure-catalog-assign-string.platform.txt`).
            // Второй квалификатор числа задаёт дробность, строки — политику длины.
            quals: vec![
                settings.code_length.to_string(),
                if id == TypeId::String { "1" } else { "0" }.into(),
            ],
        }],
    );
    // Числовой код справочника не принимает отрицательные значения.
    // Это измеренное ограничение кода, а не правило числовых колонок таблицы.
    if let Value::Number(number) = &value
        && number.cmp(&BslNumber::from_i64(0)) == std::cmp::Ordering::Less
    {
        value = Value::Number(BslNumber::from_i64(0));
    }
    *object.0.code.borrow_mut() = value;
    Ok(())
}

fn description(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.description.borrow().clone())
}

fn parent(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.parent.borrow().clone())
}

fn set_parent(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    match object.metadata()?.catalog_hierarchical() {
        Some(true) => {}
        Some(false) => {
            return Err(RtError::PropertyReadOnly {
                property: "Родитель".into(),
                receiver: TYPE.name,
            });
        }
        None => {
            return Err(RtError::DynamicError(
                "признак иерархии справочника неизвестен в снимке конфигурации".into(),
            ));
        }
    }
    let target_type = object.0.parent.borrow().type_of()?;
    // Нативный родитель сохраняет лишь ссылку своего типа; измеренные
    // несовместимые значения приводятся к собственной пустой ссылке.
    let value = if value.type_of()? == target_type
        && value.object_ref().is_some_and(|reference| {
            reference
                .downcast_ref::<ConfigurationReferenceValue>()
                .is_some()
        }) {
        value
    } else {
        catalog_reference_value(Arc::clone(&object.0.catalog), &object.0.object_id, [0; 16])?
    };
    *object.0.parent.borrow_mut() = value;
    Ok(())
}

fn set_description(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let object = object(receiver)?;
    object.require_selected()?;
    if object.0.settings.description_length == 0 {
        return Err(RtError::PropertyReadOnly {
            property: "Наименование".into(),
            receiver: TYPE.name,
        });
    }
    *object.0.description.borrow_mut() = adjust_to_types(
        value,
        &[ColumnType {
            id: TypeRef::Native(TypeId::String),
            quals: vec![object.0.settings.description_length.to_string()],
        }],
    );
    Ok(())
}

fn additional_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    Ok(object(receiver)?.0.additional_properties.clone())
}

fn exchange(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(object(receiver)?.0.exchange.clone())
}

#[derive(Debug)]
struct ExchangeParameters {
    loading: Cell<bool>,
    recipients: Value,
}

pub(super) fn exchange_parameters_value() -> Value {
    // Каждому объекту принадлежит отдельный набор параметров обмена.
    Value::new_object(ExchangeParameters {
        loading: Cell::new(false),
        recipients: Value::new_object(Recipients {
            auto_fill: Cell::new(true),
        }),
    })
}

impl ObjectProtocol for ExchangeParameters {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EXCHANGE_TYPE
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EXCHANGE_PROPERTIES
    }
}

#[derive(Debug)]
struct Recipients {
    auto_fill: Cell<bool>,
}

impl ObjectProtocol for Recipients {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &RECIPIENTS_TYPE
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        RECIPIENTS_PROPERTIES
    }
}

fn loading(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let settings = receiver
        .downcast_ref::<ExchangeParameters>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(settings.loading.get()))
}

fn set_loading(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let settings = receiver
        .downcast_ref::<ExchangeParameters>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Boolean(value) = value else {
        return Err(RtError::DynamicError(
            "присваивание Загрузка поддерживает пока только Булево".into(),
        ));
    };
    settings.loading.set(value);
    Ok(())
}

fn recipients(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(receiver
        .downcast_ref::<ExchangeParameters>()
        .ok_or(RtError::NotAnObject)?
        .recipients
        .clone())
}

fn auto_fill(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    Ok(Value::Boolean(
        receiver
            .downcast_ref::<Recipients>()
            .ok_or(RtError::NotAnObject)?
            .auto_fill
            .get(),
    ))
}

fn set_auto_fill(
    receiver: &dyn ObjectProtocol,
    value: Value,
    _: &mut CallContext<'_>,
) -> RtResult<()> {
    let recipients = receiver
        .downcast_ref::<Recipients>()
        .ok_or(RtError::NotAnObject)?;
    let Value::Boolean(value) = value else {
        return Err(RtError::DynamicError(
            "присваивание АвтоЗаполнение поддерживает пока только Булево".into(),
        ));
    };
    recipients.auto_fill.set(value);
    Ok(())
}

pub(super) static TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектСправочникаКонфигурации");
fn write(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    object(receiver)?;
    Err(RtError::DynamicError(
        "запись объекта справочника в информационную базу пока не подключена".into(),
    ))
}

fn check_filling(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let metadata = object.metadata()?;
    let settings = metadata.catalog_standard_fill_checking().ok_or_else(|| {
        RtError::DynamicError("настройки проверки стандартных реквизитов неизвестны".into())
    })?;
    let own = metadata.catalog_attribute_names().ok_or_else(|| {
        RtError::DynamicError("состав собственных реквизитов для проверки неизвестен".into())
    })?;
    let sections = metadata.catalog_tabular_section_names().ok_or_else(|| {
        RtError::DynamicError("состав табличных частей для проверки неизвестен".into())
    })?;
    if !sections.is_empty() {
        return Err(RtError::DynamicError(
            "проверка табличных частей справочника пока не реализована".into(),
        ));
    }
    let attributes = metadata.catalog_attributes();
    if !own.is_empty() && attributes.is_none() {
        return Err(RtError::DynamicError(
            "настройки собственных реквизитов справочника неизвестны".into(),
        ));
    }
    let common = metadata.catalog_common_attributes().ok_or_else(|| {
        RtError::DynamicError("состав общих реквизитов для проверки неизвестен".into())
    })?;
    let mut paths = Vec::new();
    for setting in settings {
        if setting.fill_checking == FillChecking::ShowError {
            paths.push(Value::Str(setting.name.as_str().into()));
        }
    }
    // Платформа передаёт собственные поля перед применимыми общими.
    for attribute in attributes.into_iter().flatten() {
        match attribute.fill_checking() {
            Some(FillChecking::ShowError) => {
                paths.push(Value::Str(attribute.name().into()));
            }
            Some(FillChecking::DontCheck) => {}
            None => {
                return Err(RtError::DynamicError(format!(
                    "настройка проверки собственного реквизита «{}» неизвестна",
                    attribute.name()
                )));
            }
        }
    }
    for attribute in common {
        match attribute.fill_checking() {
            Some(FillChecking::ShowError) => paths.push(Value::Str(attribute.name().into())),
            Some(FillChecking::DontCheck) => {}
            None => {
                return Err(RtError::DynamicError(format!(
                    "настройка проверки общего реквизита «{}» неизвестна",
                    attribute.name()
                )));
            }
        }
    }
    let paths = Value::Object(Rc::new(BslObject::Array(RefCell::new(paths))));
    let mut refused = false;
    if let Some(params) = attribute_filling::call_check_filling_handler(
        context,
        metadata.object_module(),
        &object.0.instance,
        Value::new_object(object.clone()),
        &paths,
        "число параметров ОбработкаПроверкиЗаполнения не поддерживается",
    )? {
        refused = attribute_filling::handler_refused(&params[0]);
        // Нативная проба закрепляет исходный массив: его мутация влияет на
        // проверку, а присваивание нового массива параметру — нет.
        // Замер: `tests/conformance/measure/1c/measure-catalog-check-filling-handler.platform.txt`.
    }
    let Value::Object(paths) = paths else {
        unreachable!("массив создан выше");
    };
    let BslObject::Array(paths) = paths.as_ref() else {
        unreachable!("массив создан выше");
    };
    for path in paths.borrow().iter() {
        let Value::Str(path) = path else {
            return Err(RtError::DynamicError(
                "проверка нестрокового пути реквизита пока не поддерживается".into(),
            ));
        };
        let path = path.to_string();
        let value = if folded_eq(&path, "Code") {
            Some(object.0.code.borrow().clone())
        } else if folded_eq(&path, "Description") {
            Some(object.0.description.borrow().clone())
        } else if object.own_attribute_index(&path)?.is_some()
            || common
                .iter()
                .any(|attribute| folded_eq(attribute.name(), &path))
        {
            Some(object.get_property(&path, context)?)
        } else if settings
            .iter()
            .any(|setting| folded_eq(&setting.name, &path))
        {
            return Err(RtError::DynamicError(format!(
                "проверка стандартного реквизита «{path}» пока не поддерживается"
            )));
        } else {
            // Измеренные неизвестные пути и русские имена стандартных полей
            // не добавляют отказа. Английские пути принимаются в любом регистре.
            None
        };
        if let Some(value) = value
            && !value.is_filled()?
        {
            refused = true;
            filling_message(&path, metadata, context)?;
        }
    }
    Ok(Value::Boolean(!refused))
}

pub(super) fn filling_message(
    path: &str,
    metadata: &MetadataObject,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let (field, russian) = if folded_eq(path, "Code") {
        ("Code", "Код")
    } else if folded_eq(path, "Description") {
        ("Description", "Наименование")
    } else if let Some(attribute) = metadata
        .catalog_attributes()
        .into_iter()
        .flatten()
        .chain(metadata.catalog_common_attributes().into_iter().flatten())
        .chain(metadata.business_process_attributes().into_iter().flatten())
        .find(|attribute| folded_eq(attribute.name(), path))
    {
        (attribute.name(), attribute.name())
    } else {
        return Err(RtError::DynamicError(
            "сообщение проверки этого реквизита пока не измерено".into(),
        ));
    };
    let language = session(context)?.system_language().ok_or_else(|| {
        RtError::DynamicError("язык сообщения проверки не предоставлен сеансом".into())
    })?;
    let russian_system = language.starts_with("ru");
    let english_system = language.starts_with("en");
    if !russian_system && !english_system {
        return Err(RtError::DynamicError(
            "текст проверки заполнения для этого языка пока не измерен".into(),
        ));
    }
    let caption = if let Some(attribute) = metadata
        .catalog_attributes()
        .into_iter()
        .flatten()
        .chain(metadata.catalog_common_attributes().into_iter().flatten())
        .chain(metadata.business_process_attributes().into_iter().flatten())
        .find(|attribute| attribute.name() == field)
    {
        // Текущий язык конфигурации выбирает синоним, язык системы — шаблон.
        // Их независимость подтверждена пробой `en` при `/Lru`.
        let current = current_language(context, &[])?;
        let code = current
            .object_ref()
            .ok_or(RtError::NotAnObject)?
            .get_property("КодЯзыка", context)?;
        let Value::Str(code) = code else {
            return Err(RtError::DynamicError(
                "код текущего языка не является строкой".into(),
            ));
        };
        attribute.presentation(&code.to_string())
    } else if russian_system {
        russian
    } else {
        field
    };
    let text = if russian_system {
        format!("Поле \"{caption}\" не заполнено")
    } else {
        format!("\"{caption}\" is a required field.")
    };
    if let Some(sink) = context.message_sink().map(Rc::clone) {
        let mut message = open_bsl::UserMessageDto::from_text(text);
        message.field = field.into();
        if let Some(left) = sink.message_bytes_left()
            && message.byte_size() > left
        {
            return Err(open_bsl::HostError::new(
                open_bsl::HostErrorCode::ResourceLimit,
                "исчерпан бюджет сообщений задания",
            )
            .raise());
        }
        sink.enqueue(&message).map_err(open_bsl::HostError::raise)?;
        if sink.also_write_to_stdout() {
            writeln!(context.stdout()?, "{}", message.text)
                .map_err(|error| RtError::IoError(error.to_string()))?;
        }
    } else {
        writeln!(context.stdout()?, "{text}")
            .map_err(|error| RtError::IoError(error.to_string()))?;
    }
    Ok(())
}

fn set_new_code(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let metadata = object.metadata()?;
    let mut prefix = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| Value::Str("".into()));
    if let Some(module) = metadata.object_module() {
        let mut handler = None;
        for name in ["ПриУстановкеНовогоКода", "OnSetNewCode"] {
            if let Some(count) = context.configuration_function_parameter_count(module, name)? {
                handler = Some((name, count));
                break;
            }
        }
        if let Some((handler, count)) = handler {
            let arguments = match (count, object.0.settings.code_type) {
                (1, CatalogCodeType::Number) => vec![Value::Boolean(true)],
                (2, CatalogCodeType::String) => vec![Value::Boolean(true), prefix.clone()],
                _ => {
                    return Err(RtError::DynamicError(
                        "форма параметров ПриУстановкеНовогоКода пока не поддерживается".into(),
                    ));
                }
            };
            let entry_mode =
                if let Some(safe_mode) = metadata.own_extension_object_module_safe_mode() {
                    open_bsl::ConfigurationInstanceEntryMode::OwnExtensionSafeMode(safe_mode)
                } else if metadata.object_module_is_extension() {
                    // НЕ ИЗМЕРЕНО(CATALOG.NEW_CODE_EXTENSION_SAFE_MODE)
                    // Справка исключает обработчик расширения из подавления режима;
                    // отдельный нативный контроль расширения ещё не завершён.
                    open_bsl::ConfigurationInstanceEntryMode::Inherit
                } else {
                    open_bsl::ConfigurationInstanceEntryMode::WithoutSafeMode
                };
            let (_, params) = context.call_configuration_instance_handler(
                module,
                &object.0.instance,
                handler,
                arguments,
                Value::new_object(object.clone()),
                entry_mode,
            )?;
            if !open_bsl::convert_to_boolean(&params[0])? {
                return Ok(Value::Undefined);
            }
            if count == 2 {
                prefix = params[1].clone();
            }
        }
    }
    let Value::Str(prefix) = prefix else {
        return Err(RtError::DynamicError(
            "выдача кода с нестроковым префиксом пока не поддерживается".into(),
        ));
    };
    // Настройки серии, права и соединение принадлежат адаптеру этого сеанса.
    // Объект не хранит собственный счётчик и не выдаёт код без адаптера.
    let (principal, adapter) = current_data_access(context)?;
    let value =
        adapter.allocate_catalog_code(metadata, &principal, &prefix.to_string(), context)?;
    let valid_type = matches!(
        (&value, object.0.settings.code_type),
        (Value::Str(_), CatalogCodeType::String) | (Value::Number(_), CatalogCodeType::Number)
    );
    if !valid_type {
        return Err(RtError::DynamicError(
            "адаптер выдачи кода вернул значение другого типа".into(),
        ));
    }
    set_code(receiver, value, context)?;
    Ok(Value::Undefined)
}

fn level(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let metadata = object.metadata()?;
    match metadata.catalog_hierarchical() {
        Some(false) => return Ok(Value::number_from_i64(0)),
        Some(true) => {}
        None => {
            return Err(RtError::DynamicError(
                "признак иерархии справочника неизвестен в снимке конфигурации".into(),
            ));
        }
    }
    // Родитель берётся из памяти объекта: изменение ещё не обязано быть записано.
    let mut parent = object.0.parent.borrow().clone();
    let mut depth = 0;
    let mut visited = HashSet::new();
    loop {
        if !reference_matches_owner(
            &parent,
            &object.0.catalog,
            &object.0.object_id,
            MetadataKind::Catalog,
        ) {
            return Err(RtError::DynamicError(
                "адаптер уровня вернул несовместимую ссылку родителя".into(),
            ));
        }
        let (_, uuid) = catalog_reference_parts(&parent).ok_or(RtError::NotAnObject)?;
        if uuid == [0; 16] {
            return Ok(Value::number_from_i64(depth));
        }
        if !visited.insert(uuid) {
            return Err(RtError::DynamicError(
                "адаптер уровня вернул циклическую иерархию".into(),
            ));
        }
        depth += 1;
        let request = QueryRequest {
            text: format!(
                "ВЫБРАТЬ ПЕРВЫЕ 1 Источник.Родитель КАК Родитель ИЗ Справочник.{} КАК Источник ГДЕ Источник.Ссылка = &Ссылка",
                metadata.name(),
            )
            .into(),
            parameters: vec![(Rc::from("Ссылка"), parent)].into(),
            temp_tables_manager: None,
        };
        let (principal, adapter) = current_data_access(context)?;
        let result = adapter.execute_query(&request, &principal, context)?;
        let row = match result.rows() {
            [] => {
                // Несуществующий родитель учитывается как последний уровень.
                // НЕ ИЗМЕРЕНО(CATALOG.LEVEL_HIDDEN_PARENT)
                // Отсутствие строки из-за RLS пока трактуется так же; требуется
                // отдельный нативный контроль с ограниченным пользователем.
                return Ok(Value::number_from_i64(depth));
            }
            [row] => row,
            _ => {
                return Err(RtError::DynamicError(
                    "адаптер уровня вернул несколько строк родителя".into(),
                ));
            }
        };
        let [next] = row.as_ref() else {
            return Err(RtError::DynamicError(
                "адаптер уровня вернул неверную строку родителя".into(),
            ));
        };
        parent = next.clone();
    }
}

fn lock(_: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    // Локальный флаг не обеспечивает блокировку объекта информационной базы.
    // До подключения механизма хоста операция явно отклоняется.
    Err(RtError::DynamicError(
        "блокировка объекта справочника пока не подключена".into(),
    ))
}

pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьНовыйКод", "SetNewCode"],
        Arity::range(0, 1),
        set_new_code,
    ),
    MethodDescriptor::new(&["Уровень", "Level"], Arity::exact(0), level),
    MethodDescriptor::new(
        &["ПроверитьЗаполнение", "CheckFilling"],
        Arity::exact(0),
        check_filling,
    ),
    MethodDescriptor::new(&["Записать", "Write"], Arity::exact(0), write),
    MethodDescriptor::new(&["Заблокировать", "Lock"], Arity::exact(0), lock).as_procedure(),
    MethodDescriptor::new(&["ЭтоНовый", "IsNew"], Arity::exact(0), is_new),
    MethodDescriptor::new(&["Метаданные", "Metadata"], Arity::exact(0), metadata),
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
];
pub(super) static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Предопределенный", "Predefined"],
        get: predefined,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяПредопределенныхДанных", "PredefinedDataName"],
        get: predefined_name,
        set: Some(set_predefined_name),
    },
    PropertyDescriptor {
        names: &["ПометкаУдаления", "DeletionMark"],
        get: deletion_mark,
        set: Some(set_deletion_mark),
    },
    PropertyDescriptor {
        names: &["ЭтоГруппа", "IsFolder"],
        get: is_folder,
        set: None,
    },
    PropertyDescriptor {
        names: &["Родитель", "Parent"],
        get: parent,
        set: Some(set_parent),
    },
    PropertyDescriptor {
        names: &["Владелец", "Owner"],
        get: owner,
        set: Some(set_owner),
    },
    PropertyDescriptor {
        names: &["Ссылка", "Ref"],
        get: reference,
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

pub(super) static EXCHANGE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПараметрыОбменаДанными",
    type_display: "Параметры обмена данными",
    type_names: &["DataExchangeParameters"],
};
pub(super) static RECIPIENTS_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "НаборУзлов",
    type_display: "Коллекция узлов",
    type_names: &["NodeSet"],
};
pub(super) static EXCHANGE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Загрузка", "Load"],
        get: loading,
        set: Some(set_loading),
    },
    PropertyDescriptor {
        names: &["Получатели", "Recipients"],
        get: recipients,
        set: None,
    },
];
pub(super) static RECIPIENTS_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["АвтоЗаполнение", "AutoFill"],
    get: auto_fill,
    set: Some(set_auto_fill),
}];

/// Возвращает владельца для общего чтения файлового макета.
pub(super) fn file_owner(receiver: &dyn ObjectProtocol) -> Option<(&Arc<MetadataCatalog>, &str)> {
    let object = receiver.downcast_ref::<CatalogObjectValue>()?;
    Some((&object.0.catalog, object.0.object_id.as_ref()))
}
