//! Значения глобального свойства `Метаданные`: корень, коллекции, объекты
//! метаданных и их свойства.

use super::*;

#[derive(Debug)]
pub(crate) struct MetadataRoot {
    pub(crate) catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
pub(crate) struct MetadataCollection {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) kind: MetadataKind,
    pub(crate) subsystem_parent: Option<Arc<str>>,
}

impl MetadataCollection {
    fn positions(&self) -> &[usize] {
        if self.kind == MetadataKind::CommonAttribute {
            return &self.catalog.common_attribute_positions;
        }
        let parent = if self.kind == MetadataKind::Subsystem {
            self.subsystem_parent.clone()
        } else {
            None
        };
        self.catalog
            .collection_positions
            .get(&(std::mem::discriminant(&self.kind), parent))
            .map_or(&[], Vec::as_slice)
    }

    fn object(&self, position: usize) -> &MetadataObject {
        let object = &self.catalog.objects[position];
        if self.kind == MetadataKind::CommonAttribute {
            // Корневой состав задан отдельно; уже приобретённые подробности
            // общих реквизитов остаются видны, как при поиске по идентификатору.
            self.catalog.object_by_id(&object.id).unwrap_or(object)
        } else {
            object
        }
    }

    pub(crate) fn members(&self) -> impl Iterator<Item = &MetadataObject> {
        self.positions()
            .iter()
            .map(|&position| self.object(position))
    }

    pub(crate) fn find(&self, name: &str) -> Option<&MetadataObject> {
        self.members().find(|object| folded_eq(&object.name, name))
    }
}

#[derive(Debug)]
pub(crate) struct MetadataValue {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) object_id: Arc<str>,
}

#[derive(Debug)]
struct MainRoles {
    catalog: Arc<MetadataCatalog>,
}

#[derive(Debug)]
struct MetadataObjectProperties;

#[derive(Debug)]
struct MetadataPropertyEnumeration(EnumKind);

pub(crate) static METADATA_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ОбъектМетаданныхКонфигурация");

pub(crate) static METADATA_COLLECTION_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияОбъектовМетаданных");

pub(crate) static METADATA_VALUE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ОбъектМетаданныхКонфигурации",
    type_display: "Metadata object",
    type_names: &["ОбъектМетаданных", "MetadataObject"],
};

pub(crate) static MAIN_ROLES_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "КоллекцияЗначенийСвойстваОбъектаМетаданных");

pub(crate) static METADATA_OBJECT_PROPERTIES_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "ПеречислимыеСвойстваОбъектовМетаданных");

pub(crate) static COMPATIBILITY_MODE_PROPERTY_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "Metadata object enumerated property",
    type_display: "Metadata object enumerated property",
    type_names: &[],
};

impl ObjectProtocol for MetadataRoot {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_TYPE
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if let Some(value) = file_source::root_enum(&self.catalog, name)? {
            return Ok(value);
        }
        if (folded_eq(name, "Имя")
            || folded_eq(name, "Name")
            || folded_eq(name, "Версия")
            || folded_eq(name, "Version"))
            && let Some(id) = self.catalog.configuration_object_id()
            && let Some(value) = file_source::property(&self.catalog, id, name, context)?
        {
            return Ok(value);
        }
        for (ru, en, kind) in [
            ("ОбщиеФормы", "CommonForms", "CommonForm"),
            ("ОбщиеМакеты", "CommonTemplates", "CommonTemplate"),
        ] {
            if (folded_eq(name, ru) || folded_eq(name, en))
                && let Some(value) = file_source::collection(&self.catalog, None, kind)?
            {
                return Ok(value);
            }
        }
        if let Some(kind) = collection_kind(name) {
            if kind == MetadataKind::CommonModule
                && let Some(value) = file_source::collection(&self.catalog, None, "CommonModule")?
            {
                return Ok(value);
            }
            if kind == MetadataKind::Subsystem {
                return metadata_root_subsystems(self, context);
            }
            if kind == MetadataKind::CommonAttribute {
                return metadata_root_common_attributes(self, context);
            }
            return Ok(Value::new_object(MetadataCollection {
                catalog: Arc::clone(&self.catalog),
                kind,
                subsystem_parent: None,
            }));
        }
        get_property_from_table(
            METADATA_ROOT_PROPERTIES,
            METADATA_TYPE.name,
            self,
            name,
            context,
        )
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        metadata_find_by_type::METHODS
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_ROOT_PROPERTIES
    }
}

impl ObjectProtocol for MetadataCollection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_COLLECTION_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        self.find(name)
            .map(|object| metadata_object_value(&self.catalog, object))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_COLLECTION_METHODS
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        let positions = self.positions();
        let &position = positions.get(index).ok_or(RtError::IndexOutOfBounds {
            index: index as i64,
            len: positions.len(),
        })?;
        Ok(metadata_object_value(&self.catalog, self.object(position)))
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.positions().len())
    }
}

impl ObjectProtocol for MetadataValue {
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_VALUE_METHODS
    }

    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        if self
            .catalog
            .object_by_id(&self.object_id)
            .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            METADATA_VALUE_PROPERTIES
        } else {
            // Первые четыре свойства относятся только к общему реквизиту.
            &METADATA_VALUE_PROPERTIES[4..]
        }
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<Value> {
        if let Some(value) = file_source::property(&self.catalog, &self.object_id, name, context)? {
            return Ok(value);
        }

        if (folded_eq(name, "АвтоИспользование")
            || folded_eq(name, "AutoUse")
            || folded_eq(name, "РазделениеДанных")
            || folded_eq(name, "DataSeparation")
            || folded_eq(name, "Состав")
            || folded_eq(name, "Content")
            || folded_eq(name, "УсловноеРазделение")
            || folded_eq(name, "ConditionalSeparation"))
            && self
                .catalog
                .materialized_object_by_id(&self.object_id)?
                .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            return get_property_from_table(
                METADATA_VALUE_PROPERTIES,
                METADATA_VALUE_TYPE.name,
                self,
                name,
                context,
            );
        }
        if folded_eq(name, "Имя")
            || folded_eq(name, "Name")
            || folded_eq(name, "Подсистемы")
            || folded_eq(name, "Subsystems")
            || folded_eq(name, "ВключатьВКомандныйИнтерфейс")
            || folded_eq(name, "IncludeInCommandInterface")
            || folded_eq(name, "КодЯзыка")
            || folded_eq(name, "LanguageCode")
            || folded_eq(name, "Реквизиты")
            || folded_eq(name, "Attributes")
        {
            return get_property_from_table(
                METADATA_VALUE_PROPERTIES,
                METADATA_VALUE_TYPE.name,
                self,
                name,
                context,
            );
        }
        let object = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .ok_or_else(|| {
                RtError::DynamicError("объект метаданных отсутствует в каталоге".into())
            })?;
        object
            .string_properties
            .iter()
            .find(|(property, _)| folded_eq(property, name))
            .map(|(_, value)| Value::Str(value.as_ref().into()))
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        let index = self
            .catalog
            .objects
            .iter()
            .position(|object| object.id == self.object_id)?;
        Some((Arc::as_ptr(&self.catalog) as usize, index))
    }

    fn display(&self) -> String {
        self.catalog
            .object_by_id(&self.object_id)
            .filter(|object| object.kind == MetadataKind::Language)
            .map_or_else(
                || METADATA_VALUE_TYPE.name.into(),
                |object| object.name().into(),
            )
    }
}

impl ObjectProtocol for MainRoles {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MAIN_ROLES_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        MAIN_ROLES_METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self
            .catalog
            .configuration
            .as_ref()
            .map_or(0, |description| description.main_role_ids.len()))
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Number(index) = index else {
            return Err(RtError::BadIndex);
        };
        let index = index
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        let description = self.catalog.configuration.as_ref().ok_or_else(|| {
            RtError::DynamicError("свойства конфигурации отсутствуют в каталоге метаданных".into())
        })?;
        let id = description
            .main_role_ids
            .get(index)
            .ok_or(RtError::IndexOutOfBounds {
                index: index as i64,
                len: description.main_role_ids.len(),
            })?;
        let object = self.catalog.materialized_object_by_id(id)?.ok_or_else(|| {
            RtError::DynamicError("основная роль отсутствует в каталоге метаданных".into())
        })?;
        Ok(metadata_object_value(&self.catalog, object))
    }
}

impl ObjectProtocol for MetadataObjectProperties {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_OBJECT_PROPERTIES_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METADATA_OBJECT_PROPERTIES
    }
}

impl ObjectProtocol for MetadataPropertyEnumeration {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COMPATIBILITY_MODE_PROPERTY_TYPE
    }

    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        // Точка и индекс дают одинаковые члены, включая измеренный режим совместимости.
        lookup_member(self.0, name)
            .map(Value::Enum)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }

    fn get_index(&self, index: &Value) -> RtResult<Value> {
        let Value::Str(member) = index else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ПеречислимоеСвойствоОбъектаМетаданных[]",
            });
        };
        lookup_member(self.0, &member.to_string())
            .map(Value::Enum)
            .ok_or_else(|| RtError::UnknownProperty(member.to_string()))
    }

    fn display(&self) -> String {
        "MetadataObjectsPropertyEnumed".into()
    }
}

fn metadata_collection_find(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let [Value::Str(wanted)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Найти",
        });
    };
    let wanted = wanted.to_string();
    if let Some(collection) = receiver.downcast_ref::<file_source::Collection>() {
        return Ok(collection.find(&wanted).unwrap_or(Value::Undefined));
    }
    if let Some(collection) = receiver.downcast_ref::<metadata_attributes::Collection>() {
        return Ok(collection.find(&wanted).unwrap_or(Value::Undefined));
    }
    let collection = receiver
        .downcast_ref::<MetadataCollection>()
        .ok_or(RtError::NotAnObject)?;
    Ok(collection
        .find(&wanted)
        .map(|object| metadata_object_value(&collection.catalog, object))
        .unwrap_or(Value::Undefined))
}

pub(crate) fn metadata_collection_count(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let count = i64::try_from(receiver.collection_len()?)
        .map_err(|_| RtError::DynamicError("слишком много объектов метаданных".into()))?;
    Ok(Value::number_from_i64(count))
}

fn metadata_value_common_attribute_data_separation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let enabled = object.common_attribute_data_separation.ok_or_else(|| {
        RtError::DynamicError("режим разделения общего реквизита неизвестен".into())
    })?;
    Ok(Value::Enum(if enabled {
        EnumValue::CommonAttributeDataSeparationSeparate
    } else {
        EnumValue::CommonAttributeDataSeparationDontUse
    }))
}

fn metadata_value_common_attribute_auto_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let enabled = object.common_attribute_auto_use.ok_or_else(|| {
        RtError::DynamicError("автоиспользование общего реквизита неизвестно".into())
    })?;
    Ok(Value::Enum(if enabled {
        EnumValue::CommonAttributeAutoUseUse
    } else {
        EnumValue::CommonAttributeAutoUseDontUse
    }))
}

fn metadata_value_full_name(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    if receiver.downcast_ref::<file_source::View>().is_some() {
        return Err(RtError::DynamicError(
            "полное имя этого вида метаданных пока не поддерживается".into(),
        ));
    }
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    let variant = value
        .catalog
        .configuration
        .as_ref()
        .ok_or_else(|| {
            RtError::DynamicError("вариант встроенного языка конфигурации неизвестен".into())
        })?
        .script_variant;
    // Нативные контроли обоих вариантов языка закрепляют префикс,
    // независимо от языка обращения к объекту и методу.
    // Замеры: `tests/conformance/measure/1c/measure-metadata-full-name*.platform.txt`,
    // `tests/conformance/measure/1c/measure-metadata-option-full-name*.platform.txt`.
    let prefix = match (object.kind, variant) {
        (MetadataKind::Catalog, ScriptVariant::Russian) => "Справочник",
        (MetadataKind::Catalog, ScriptVariant::English) => "Catalog",
        (MetadataKind::CommonAttribute, ScriptVariant::Russian) => "ОбщийРеквизит",
        (MetadataKind::CommonAttribute, ScriptVariant::English) => "CommonAttribute",
        (MetadataKind::Constant, ScriptVariant::Russian) => "Константа",
        (MetadataKind::Constant, ScriptVariant::English) => "Constant",
        (MetadataKind::FunctionalOption, ScriptVariant::Russian) => "ФункциональнаяОпция",
        (MetadataKind::FunctionalOption, ScriptVariant::English) => "FunctionalOption",
        _ => {
            return Err(RtError::DynamicError(
                "полное имя этого вида метаданных пока не поддерживается".into(),
            ));
        }
    };
    Ok(Value::Str(
        format!("{prefix}.{}", object.name()).as_str().into(),
    ))
}

pub(crate) fn metadata_value_name(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    if let Some(attribute) = receiver.downcast_ref::<metadata_attributes::Attribute>() {
        return Ok(Value::Str(attribute.name().into()));
    }
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    Ok(Value::Str(object.name().into()))
}

fn metadata_value_attributes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind() != MetadataKind::Document {
        return Err(RtError::UnknownProperty("Реквизиты".into()));
    }
    let attributes = object.document_attributes.as_ref().ok_or_else(|| {
        RtError::DynamicError("коллекция реквизитов документа не приобретена".into())
    })?;
    Ok(metadata_attributes::collection(
        Arc::clone(&value.catalog),
        Arc::clone(&value.object_id),
        Arc::clone(attributes),
    ))
}

fn metadata_collection_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    // `Получить` приводит аргумент к числу; строковый индекс `[...]`
    // отдельно ищет имя реквизита. Различие подтверждено замером платформы.
    let number = collection_index_number(&arguments[0])?.trunc_to_scale(0);
    receiver.get_index(&Value::Number(number))
}

fn metadata_value_subsystems(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Subsystem {
        return Err(RtError::UnknownProperty("Подсистемы".into()));
    }
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&value.catalog),
        kind: MetadataKind::Subsystem,
        subsystem_parent: Some(Arc::clone(&object.id)),
    }))
}

fn metadata_value_subsystem_command_interface(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Subsystem {
        return Err(RtError::UnknownProperty(
            "ВключатьВКомандныйИнтерфейс".into(),
        ));
    }
    object
        .subsystem_command_interface
        .map(Value::Boolean)
        .ok_or_else(|| RtError::DynamicError("признак подсистемы отсутствует в метаданных".into()))
}

fn metadata_value_language_code(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or_else(|| RtError::DynamicError("объект метаданных отсутствует в каталоге".into()))?;
    if object.kind != MetadataKind::Language {
        return Err(RtError::UnknownProperty("КодЯзыка".into()));
    }
    object
        .string_properties
        .iter()
        .find(|(name, _)| folded_eq(name, "КодЯзыка"))
        .map(|(_, code)| Value::Str(code.as_ref().into()))
        .ok_or_else(|| RtError::DynamicError("код языка отсутствует в метаданных".into()))
}

pub(crate) fn configuration_description(
    root: &MetadataRoot,
) -> RtResult<&ConfigurationDescription> {
    root.catalog.configuration.as_ref().ok_or_else(|| {
        RtError::DynamicError("свойства конфигурации отсутствуют в каталоге метаданных".into())
    })
}

fn metadata_root_common_attributes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    let ids = root.catalog.common_attribute_ids().ok_or_else(|| {
        RtError::DynamicError("состав общих реквизитов конфигурации неизвестен".into())
    })?;
    // Публикуем только полный состав корня, а не применимые поля справочников.
    for id in ids {
        if !root
            .catalog
            .materialized_object_by_id(id)?
            .is_some_and(|object| object.kind == MetadataKind::CommonAttribute)
        {
            return Err(RtError::DynamicError(
                "неполный состав общих реквизитов конфигурации".into(),
            ));
        }
    }
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&root.catalog),
        kind: MetadataKind::CommonAttribute,
        subsystem_parent: None,
    }))
}

fn metadata_root_name(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        configuration_description(root)?.name.as_ref().into(),
    ))
}

fn metadata_root_default_interface(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    // Нативные пустой и назначенный интерфейсы различаются UUID настройки.
    // Неизвестные данные и неподдержанный объект не превращаем в отсутствие.
    match root.catalog.default_interface_id {
        Some(id) if id == [0; 16] => Ok(Value::Undefined),
        Some(_) => Err(RtError::DynamicError(
            "метаданные назначенного основного интерфейса не поддерживаются".into(),
        )),
        None => Err(RtError::DynamicError(
            "основной интерфейс конфигурации не приобретён".into(),
        )),
    }
}

fn metadata_root_primary_language(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    let language = root
        .catalog
        .primary_language_id()
        .and_then(|id| root.catalog.object_by_id(id))
        .ok_or_else(|| {
            RtError::DynamicError("основной язык отсутствует в каталоге метаданных".into())
        })?;
    Ok(metadata_object_value(&root.catalog, language))
}

fn metadata_root_subsystems(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataCollection {
        catalog: Arc::clone(&root.catalog),
        kind: MetadataKind::Subsystem,
        subsystem_parent: None,
    }))
}

fn metadata_root_version(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Str(
        configuration_description(root)?.version.as_ref().into(),
    ))
}

fn metadata_root_script_variant(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(configuration_description(root)?.script_variant.value())
}

fn metadata_root_compatibility_mode(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(configuration_description(root)?.compatibility_mode.value())
}

fn metadata_root_main_roles(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let root = receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    configuration_description(root)?;
    Ok(Value::new_object(MainRoles {
        catalog: Arc::clone(&root.catalog),
    }))
}

fn metadata_root_object_properties(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataRoot>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataObjectProperties))
}

fn metadata_object_script_variants(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::EnumType(EnumKind::ScriptVariant))
}

fn metadata_object_common_attribute_data_separation(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeDataSeparation,
    )))
}

fn metadata_object_common_attribute_auto_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeAutoUse,
    )))
}

fn metadata_object_common_attribute_use(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CommonAttributeUse,
    )))
}

fn metadata_object_extension_purpose(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::ConfigurationExtensionPurpose,
    )))
}

fn metadata_object_compatibility_modes(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    receiver
        .downcast_ref::<MetadataObjectProperties>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::new_object(MetadataPropertyEnumeration(
        EnumKind::CompatibilityMode,
    )))
}

fn main_roles_contains(
    receiver: &dyn ObjectProtocol,
    arguments: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let roles = receiver
        .downcast_ref::<MainRoles>()
        .ok_or(RtError::NotAnObject)?;
    let [argument] = arguments else {
        return Err(RtError::MethodNotApplicable {
            method: "Содержит",
            receiver: MAIN_ROLES_TYPE.name,
        });
    };
    let Some(role) = argument
        .object_ref()
        .and_then(|value| value.downcast_ref::<MetadataValue>())
        .filter(|value| Arc::ptr_eq(&value.catalog, &roles.catalog))
        .and_then(|value| roles.catalog.object_by_id(&value.object_id))
        .filter(|object| object.kind == MetadataKind::Role)
    else {
        return Ok(Value::Boolean(false));
    };
    let contains = roles
        .catalog
        .configuration
        .as_ref()
        .is_some_and(|description| {
            description
                .main_role_ids
                .iter()
                .any(|id| id.as_ref() == role.id())
        });
    Ok(Value::Boolean(contains))
}

pub(crate) static METADATA_COLLECTION_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Получить", "Get"],
        Arity::exact(1),
        metadata_collection_get,
    ),
    MethodDescriptor::new(
        &["Найти", "Find"],
        Arity::exact(1),
        metadata_collection_find,
    ),
    MethodDescriptor::new(
        &["Количество", "Count"],
        Arity::exact(0),
        metadata_collection_count,
    ),
];

pub(crate) static METADATA_ROOT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ОбщиеФормы", "CommonForms"],
        get: file_source::common_forms,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбщиеМакеты", "CommonTemplates"],
        get: file_source::common_templates,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновнойИнтерфейс", "DefaultInterface"],
        get: metadata_root_default_interface,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОбщиеРеквизиты", "CommonAttributes"],
        get: metadata_root_common_attributes,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновнойЯзык", "DefaultLanguage"],
        get: metadata_root_primary_language,
        set: None,
    },
    PropertyDescriptor {
        names: &["Подсистемы", "Subsystems"],
        get: metadata_root_subsystems,
        set: None,
    },
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: metadata_root_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Версия", "Version"],
        get: metadata_root_version,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВариантВстроенногоЯзыка", "ScriptVariant"],
        get: metadata_root_script_variant,
        set: None,
    },
    PropertyDescriptor {
        names: &["РежимСовместимости", "CompatibilityMode"],
        get: metadata_root_compatibility_mode,
        set: None,
    },
    PropertyDescriptor {
        names: &["ОсновныеРоли", "DefaultRoles"],
        get: metadata_root_main_roles,
        set: None,
    },
    PropertyDescriptor {
        names: &["СвойстваОбъектов", "ObjectProperties"],
        get: metadata_root_object_properties,
        set: None,
    },
];

pub(crate) static MAIN_ROLES_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Содержит", "Contains"],
    Arity::exact(1),
    main_roles_contains,
)];

pub(crate) static METADATA_OBJECT_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &[
            "НазначениеРасширенияКонфигурации",
            "ConfigurationExtensionPurpose",
        ],
        get: metadata_object_extension_purpose,
        set: None,
    },
    PropertyDescriptor {
        names: &[
            "РазделениеДанныхОбщегоРеквизита",
            "CommonAttributeDataSeparation",
        ],
        get: metadata_object_common_attribute_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["АвтоИспользованиеОбщегоРеквизита", "CommonAttributeAutoUse"],
        get: metadata_object_common_attribute_auto_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИспользованиеОбщегоРеквизита", "CommonAttributeUse"],
        get: metadata_object_common_attribute_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВариантВстроенногоЯзыка", "ScriptVariant"],
        get: metadata_object_script_variants,
        set: None,
    },
    PropertyDescriptor {
        names: &["РежимСовместимости", "CompatibilityMode"],
        get: metadata_object_compatibility_modes,
        set: None,
    },
];

pub(crate) static METADATA_VALUE_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолноеИмя", "FullName"],
    Arity::exact(0),
    metadata_value_full_name,
)];

pub(crate) static METADATA_VALUE_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["РазделениеДанных", "DataSeparation"],
        get: metadata_value_common_attribute_data_separation,
        set: None,
    },
    PropertyDescriptor {
        names: &["АвтоИспользование", "AutoUse"],
        get: metadata_value_common_attribute_auto_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["УсловноеРазделение", "ConditionalSeparation"],
        get: common_attribute_content::get_condition,
        set: None,
    },
    PropertyDescriptor {
        names: &["Состав", "Content"],
        get: common_attribute_content::get,
        set: None,
    },
    PropertyDescriptor {
        names: &["Реквизиты", "Attributes"],
        get: metadata_value_attributes,
        set: None,
    },
    metadata_attributes::PROPERTIES[1],
    metadata_attributes::PROPERTIES[2],
    PropertyDescriptor {
        names: &["КодЯзыка", "LanguageCode"],
        get: metadata_value_language_code,
        set: None,
    },
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: metadata_value_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["Подсистемы", "Subsystems"],
        get: metadata_value_subsystems,
        set: None,
    },
    PropertyDescriptor {
        names: &["ВключатьВКомандныйИнтерфейс", "IncludeInCommandInterface"],
        get: metadata_value_subsystem_command_interface,
        set: None,
    },
    PropertyDescriptor {
        names: &["Формы", "Forms"],
        get: file_source::forms,
        set: None,
    },
    PropertyDescriptor {
        names: &["Макеты", "Templates"],
        get: file_source::templates,
        set: None,
    },
];

pub(crate) fn metadata_collections() -> &'static [(&'static str, &'static str, MetadataKind)] {
    &[
        ("Подсистемы", "Subsystems", MetadataKind::Subsystem),
        (
            "ОбщиеРеквизиты",
            "CommonAttributes",
            MetadataKind::CommonAttribute,
        ),
        ("Перечисления", "Enums", MetadataKind::Enumeration),
        ("Справочники", "Catalogs", MetadataKind::Catalog),
        ("Документы", "Documents", MetadataKind::Document),
        (
            "ЖурналыДокументов",
            "DocumentJournals",
            MetadataKind::DocumentJournal,
        ),
        ("Отчеты", "Reports", MetadataKind::Report),
        ("Обработки", "DataProcessors", MetadataKind::DataProcessor),
        (
            "РегистрыСведений",
            "InformationRegisters",
            MetadataKind::InformationRegister,
        ),
        (
            "РегистрыНакопления",
            "AccumulationRegisters",
            MetadataKind::AccumulationRegister,
        ),
        (
            "РегистрыБухгалтерии",
            "AccountingRegisters",
            MetadataKind::AccountingRegister,
        ),
        (
            "РегистрыРасчета",
            "CalculationRegisters",
            MetadataKind::CalculationRegister,
        ),
        (
            "ПланыСчетов",
            "ChartsOfAccounts",
            MetadataKind::ChartOfAccounts,
        ),
        (
            "ПланыВидовХарактеристик",
            "ChartsOfCharacteristicTypes",
            MetadataKind::ChartOfCharacteristicTypes,
        ),
        (
            "ПланыВидовРасчета",
            "ChartsOfCalculationTypes",
            MetadataKind::ChartOfCalculationTypes,
        ),
        (
            "БизнесПроцессы",
            "BusinessProcesses",
            MetadataKind::BusinessProcess,
        ),
        ("Задачи", "Tasks", MetadataKind::Task),
        (
            "ХранилищаНастроек",
            "SettingsStorages",
            MetadataKind::SettingsStorage,
        ),
        ("Последовательности", "Sequences", MetadataKind::Sequence),
        ("Константы", "Constants", MetadataKind::Constant),
        ("ПланыОбмена", "ExchangePlans", MetadataKind::ExchangePlan),
        (
            "ОбщиеКартинки",
            "CommonPictures",
            MetadataKind::CommonPicture,
        ),
        (
            "ФункциональныеОпции",
            "FunctionalOptions",
            MetadataKind::FunctionalOption,
        ),
        ("ЭлементыСтиля", "StyleItems", MetadataKind::StyleItem),
        ("Языки", "Languages", MetadataKind::Language),
        ("Роли", "Roles", MetadataKind::Role),
        (
            "ПараметрыСеанса",
            "SessionParameters",
            MetadataKind::SessionParameter,
        ),
        ("ОбщиеМодули", "CommonModules", MetadataKind::CommonModule),
        (
            "РегламентныеЗадания",
            "ScheduledJobs",
            MetadataKind::ScheduledJob,
        ),
    ]
}

pub(crate) fn collection_kind(name: &str) -> Option<MetadataKind> {
    metadata_collections()
        .iter()
        .find(|(ru, en, _)| folded_eq(name, ru) || folded_eq(name, en))
        .map(|(_, _, kind)| *kind)
}

pub(crate) fn metadata_object_value(
    catalog: &Arc<MetadataCatalog>,
    object: &MetadataObject,
) -> Value {
    Value::new_object(MetadataValue {
        catalog: Arc::clone(catalog),
        object_id: Arc::clone(&object.id),
    })
}

pub(crate) fn metadata(ctx: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(MetadataRoot {
        catalog: Arc::clone(session(ctx)?.catalog()),
    }))
}
