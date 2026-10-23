//! Ссылки на объекты конфигурации и значения конфигурационных перечислений.

use super::*;

#[derive(Debug)]
pub(crate) struct ConfigurationReferenceValue {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    pub(crate) uuid: [u8; 16],
}

#[derive(Debug)]
pub(crate) struct EnumerationReferenceValue {
    pub(crate) catalog: Arc<MetadataCatalog>,
    pub(crate) object_id: Arc<str>,
    ty: Arc<ValueTypeDescriptor>,
    pub(crate) value_id: Arc<str>,
}

pub(crate) static ENUMERATION_REFERENCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ПеречислениеСсылка",
    type_display: "Enumeration reference",
    type_names: &["EnumerationRef"],
};

impl ObjectProtocol for ConfigurationReferenceValue {
    fn reference_order_key(&self) -> Option<open_bsl::ReferenceOrderKey> {
        let metadata = self.catalog.object_by_id(&self.object_id)?;
        // Межвидовой порядок измерен пока только для ссылок документов.
        // Для других классов ключ не публикуется по аналогии.
        (metadata.kind() == MetadataKind::Document).then_some(open_bsl::ReferenceOrderKey {
            storage_number: metadata.reference_storage_number(),
            uuid: self.uuid,
        })
    }
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &CONFIGURATION_REFERENCE_TYPE
    }
    fn immutable_snapshot(&self) -> Option<&[u8]> {
        Some(&self.uuid)
    }

    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        CONFIGURATION_REFERENCE_METHODS
    }
    fn is_filled(&self) -> RtResult<bool> {
        Ok(self.uuid != [0; 16])
    }
    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some_and(|other| {
            TypeRef::Value(Arc::clone(&self.ty)) == TypeRef::Value(Arc::clone(&other.ty))
                && self.uuid == other.uuid
        }))
    }
    fn value_hash_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.ty.package().hash(&mut hash);
        self.ty.identity().hash(&mut hash);
        self.uuid.hash(&mut hash);
        Some(hash.finish())
    }
    fn display(&self) -> String {
        format!(
            "{}:{}",
            self.ty.identity(),
            Value::Object(Rc::new(BslObject::Uuid(self.uuid)))
        )
    }
    fn presentation(&self) -> RtResult<String> {
        if self.uuid == [0; 16] {
            return Ok(String::new());
        }
        Err(RtError::DynamicError(
            "получение представления заполненной ссылки не подключено к адаптеру данных".into(),
        ))
    }
}

/// Создаёт ссылку справочника по метаданным и UUID в каноническом порядке байтов.
/// Не читает и не записывает строку информационной базы.
///
/// # Errors
/// Возвращает ошибку, если объект или его тип ссылки отсутствует в каталоге.
pub fn catalog_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    uuid: [u8; 16],
) -> RtResult<Value> {
    if !catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind() == MetadataKind::Catalog)
    {
        return Err(RtError::DynamicError(
            "тип ссылки отсутствует в каталоге".into(),
        ));
    }
    configuration_reference_value(catalog, object_id, uuid)
}

// Общая память ссылки не содержит соединения или данных строки. Вид и
// собственный тип задаёт приобретённое описание, а не значение UUID.
pub(crate) fn reference_matches_owner(
    value: &Value,
    catalog: &MetadataCatalog,
    owner: &str,
    kind: MetadataKind,
) -> bool {
    value
        .object_ref()
        .and_then(|value| value.downcast_ref::<ConfigurationReferenceValue>())
        .is_some_and(|reference| {
            reference.object_id.as_ref() == owner
                && reference.catalog.identity() == catalog.identity()
                && reference
                    .catalog
                    .object_by_id(&reference.object_id)
                    .is_some_and(|metadata| metadata.kind() == kind)
        })
}

/// Создаёт ссылку объявленного объекта конфигурации по каноническому UUID.
/// Соединение и строка информационной базы не сохраняются в значении.
///
/// # Errors
/// Возвращает ошибку, если в каталоге отсутствует объявленный тип ссылки владельца.
pub fn configuration_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    uuid: [u8; 16],
) -> RtResult<Value> {
    if catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind() == MetadataKind::Enumeration)
    {
        // Константа и менеджер должны возвращать один и тот же тип значения.
        let identity = if uuid == [0; 16] {
            String::new()
        } else {
            open_bsl::uuid::format(&uuid)
        };
        return enumeration_reference_value(catalog, object_id, &identity);
    }
    let ty = catalog
        .reference_types
        .iter()
        .find(|(id, _)| id.as_ref() == object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип ссылки отсутствует в каталоге".into()))?;
    Ok(Value::new_object(ConfigurationReferenceValue {
        object_id: object_id.into(),
        catalog,
        ty,
        uuid,
    }))
}

/// Возвращает метаданные справочника и канонический UUID ссылки.
/// Для значений других видов возвращает `None` без преобразования в строку.
pub fn catalog_reference_parts(value: &Value) -> Option<(&MetadataObject, [u8; 16])> {
    let (object, uuid) = configuration_reference_parts(value)?;
    (object.kind() == MetadataKind::Catalog).then_some((object, uuid))
}

/// Возвращает метаданные и канонический UUID типизированной ссылки конфигурации.
/// Для значений других типов возвращает `None`, не обращаясь к базе данных.
pub fn configuration_reference_parts(value: &Value) -> Option<(&MetadataObject, [u8; 16])> {
    let reference = value
        .object_ref()?
        .downcast_ref::<ConfigurationReferenceValue>()?;
    let object = reference.catalog.object_by_id(&reference.object_id)?;
    Some((object, reference.uuid))
}

/// Идентификатор метаданных справочника для объявленного типа ссылки.
/// Для типов других компонентов и других видов значений возвращает `None`.
pub fn catalog_reference_type_id(ty: &TypeRef) -> Option<&str> {
    let TypeRef::Value(description) = ty else {
        return None;
    };
    if description.package() != "bsl-orm" {
        return None;
    }
    description
        .identity()
        .rsplit_once("/catalog-ref/")
        .map(|(_, id)| id)
}

fn catalog_reference(manager: &ManagerValue, uuid: [u8; 16]) -> RtResult<Value> {
    catalog_reference_value(Arc::clone(&manager.catalog), &manager.object_id, uuid)
}

pub(crate) fn catalog_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    catalog_reference(manager, [0; 16])
}

pub(crate) fn document_empty_ref(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, [0; 16])
}

pub(crate) fn configuration_get_ref(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let manager = receiver
        .downcast_ref::<ManagerValue>()
        .ok_or(RtError::NotAnObject)?;
    let argument = args.first().unwrap_or(&Value::Undefined);
    let generated;
    let value = if matches!(argument, Value::Undefined) {
        let uuid = session(context)?
            .reference_uuids
            .borrow_mut()
            .next(context.unix_millis()?, context.random()?)?;
        generated = Value::Object(Rc::new(BslObject::Uuid(uuid)));
        &generated
    } else {
        argument
    };
    let Value::Object(object) = value else {
        return Err(RtError::TypeError {
            expected: "УникальныйИдентификатор",
            op: "ПолучитьСсылку",
        });
    };
    let BslObject::Uuid(uuid) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "УникальныйИдентификатор",
            op: "ПолучитьСсылку",
        });
    };
    configuration_reference_value(Arc::clone(&manager.catalog), &manager.object_id, *uuid)
}

fn configuration_reference_uuid(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Object(Rc::new(BslObject::Uuid(reference.uuid))))
}

fn configuration_reference_metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    let object = reference
        .catalog
        .materialized_object_by_id(&reference.object_id)?
        .ok_or_else(|| RtError::DynamicError("метаданные ссылки отсутствуют в каталоге".into()))?;
    Ok(metadata_object_value(&reference.catalog, object))
}

fn configuration_reference_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let reference = receiver
        .downcast_ref::<ConfigurationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(reference.uuid == [0; 16]))
}

pub(crate) static CONFIGURATION_REFERENCE_TYPE: TypeDescriptor =
    TypeDescriptor::new("bsl-orm", "СсылкаКонфигурации");

pub(crate) static CONFIGURATION_REFERENCE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустая", "IsEmpty"],
        Arity::exact(0),
        configuration_reference_is_empty,
    ),
    MethodDescriptor::new(
        &["УникальныйИдентификатор", "UUID"],
        Arity::exact(0),
        configuration_reference_uuid,
    ),
    MethodDescriptor::new(
        &["Метаданные", "Metadata"],
        Arity::exact(0),
        configuration_reference_metadata,
    ),
];

// Декодер связывает только неизменяемую идентичность с каталогом получателя.
// Пустой идентификатор обозначает пустую ссылку конкретного перечисления.
pub(crate) fn enumeration_reference_value(
    catalog: Arc<MetadataCatalog>,
    object_id: &str,
    value_id: &str,
) -> RtResult<Value> {
    if !catalog
        .materialized_object_by_id(object_id)?
        .is_some_and(|object| object.kind == MetadataKind::Enumeration)
    {
        return Err(RtError::DynamicError(
            "значение перечисления отсутствует в каталоге".into(),
        ));
    }
    // Список значений ограничивает поиск по имени, но не идентичность ссылки:
    // платформа сохраняет UUID удалённого значения в реквизитах метаданных.
    // Объявленные компонентом идентификаторы сохраняют прежний формат.
    let value_id: Arc<str> = if value_id.is_empty()
        || catalog.enumeration_values.iter().any(|value| {
            value.owner_id.as_ref() == object_id && value.value_id.as_ref() == value_id
        }) {
        value_id.into()
    } else {
        open_bsl::uuid::format(&open_bsl::uuid::parse(value_id)?).into()
    };
    let ty = catalog
        .reference_types
        .iter()
        .find(|(id, _)| id.as_ref() == object_id)
        .map(|(_, ty)| Arc::clone(ty))
        .ok_or_else(|| RtError::DynamicError("тип перечисления отсутствует в каталоге".into()))?;
    Ok(Value::new_object(EnumerationReferenceValue {
        catalog,
        object_id: object_id.into(),
        ty,
        value_id,
    }))
}

fn enumeration_reference_is_empty(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<EnumerationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Boolean(value.value_id.is_empty()))
}

fn enumeration_reference_metadata(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let value = receiver
        .downcast_ref::<EnumerationReferenceValue>()
        .ok_or(RtError::NotAnObject)?;
    let owner = value
        .catalog
        .materialized_object_by_id(&value.object_id)?
        .ok_or(RtError::NotAnObject)?;
    Ok(metadata_object_value(&value.catalog, owner))
}

pub(crate) static ENUMERATION_REFERENCE_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Пустая", "IsEmpty"],
        Arity::exact(0),
        enumeration_reference_is_empty,
    ),
    MethodDescriptor::new(
        &["Метаданные", "Metadata"],
        Arity::exact(0),
        enumeration_reference_metadata,
    ),
];

// НЕ ИЗМЕРЕНО(PREDEFINED.CONFIG_ENUM.REFERENCE_SURFACE): базовые операции
// измерены отдельными пробами; полный контракт переноса и языковых
// вариантов ещё не закрыт в реестре замеров.
impl ObjectProtocol for EnumerationReferenceValue {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ENUMERATION_REFERENCE_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ENUMERATION_REFERENCE_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.value_id.is_empty())
    }

    fn value_type(&self) -> TypeRef {
        TypeRef::Value(Arc::clone(&self.ty))
    }

    fn immutable_snapshot(&self) -> Option<&[u8]> {
        Some(self.value_id.as_bytes())
    }

    fn value_eq(&self, other: &open_bsl::ObjectRef) -> Option<bool> {
        Some(other.downcast_ref::<Self>().is_some_and(|other| {
            TypeRef::Value(Arc::clone(&self.ty)) == TypeRef::Value(Arc::clone(&other.ty))
                && self.value_id == other.value_id
        }))
    }

    fn value_hash_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.ty.package().hash(&mut hash);
        self.ty.identity().hash(&mut hash);
        self.value_id.hash(&mut hash);
        Some(hash.finish())
    }

    fn presentation(&self) -> RtResult<String> {
        if self.value_id.is_empty()
            || self
                .catalog
                .enumeration_values
                .iter()
                .any(|value| value.owner_id == self.object_id && value.value_id == self.value_id)
        {
            return Ok(self.display());
        }
        Err(RtError::DynamicError(
            "представление отсутствующего значения перечисления требует данных сеанса".into(),
        ))
    }

    fn presentation_with_context(&self, context: &CallContext<'_>) -> RtResult<String> {
        if let Ok(text) = self.presentation() {
            return Ok(text);
        }
        let number = self
            .catalog
            .materialized_object_by_id(&self.object_id)?
            .and_then(MetadataObject::reference_storage_number)
            .ok_or_else(|| {
                RtError::DynamicError(
                    "нет приобретённого номера DBNames для представления перечисления".into(),
                )
            })?;
        let label = match session(context)?.system_language() {
            Some(language) if folded_eq(language, "ru") => "Объект не найден",
            Some(language) if folded_eq(language, "en") => "Object not found",
            _ => {
                return Err(RtError::DynamicError(
                    "язык представления отсутствующего значения перечисления не подключён".into(),
                ));
            }
        };
        let physical = open_bsl::uuid::to_1c_bytes(open_bsl::uuid::parse(&self.value_id)?);
        use std::fmt::Write;
        let mut text = format!("<{label}> ({number}:");
        for byte in physical {
            write!(text, "{byte:02x}").expect("запись в строку не завершается ошибкой");
        }
        text.push(')');
        Ok(text)
    }

    fn display(&self) -> String {
        self.catalog
            .enumeration_values
            .iter()
            .find(|value| value.owner_id == self.object_id && value.value_id == self.value_id)
            .map_or_else(String::new, |value| {
                let language = self
                    .catalog
                    .primary_language_id()
                    .and_then(|id| self.catalog.object_by_id(id))
                    .and_then(|object| {
                        object
                            .string_properties
                            .iter()
                            .find(|(name, _)| name.as_ref() == "КодЯзыка")
                    })
                    .map(|(_, code)| code.as_ref());
                language
                    .and_then(|language| {
                        value
                            .synonyms
                            .iter()
                            .find(|(code, _)| code.as_ref() == language)
                    })
                    .map_or_else(|| value.name.to_string(), |(_, text)| text.to_string())
            })
    }
}
