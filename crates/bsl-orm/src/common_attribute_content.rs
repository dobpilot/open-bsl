//! Полный приобретённый состав общего реквизита и его неизменяемые элементы.

use super::*;

/// Известное условие разделения общего реквизита или элемента его состава.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommonAttributeCondition {
    /// Условие явно отсутствует.
    Undefined,
    /// Идентификатор константы условия.
    Constant(String),
}

/// Один элемент полного состава, включая автоматическое использование.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonAttributeContentEntry {
    /// Идентификатор связанного объекта метаданных.
    pub metadata_id: String,
    /// Одно из значений `CommonAttributeUse`: Auto, Use или DontUse.
    pub usage: EnumValue,
    /// Известное условие; `None` сохраняет неизвестные данные.
    pub conditional_separation: Option<CommonAttributeCondition>,
}

impl MetadataObject {
    /// Возвращает полный приобретённый состав; `None` — неизвестный состав.
    #[must_use]
    pub fn common_attribute_content(&self) -> Option<&[CommonAttributeContentEntry]> {
        self.common_attribute_content.as_deref()
    }
}

impl MetadataCatalog {
    /// Присоединяет проверенный полный состав общего реквизита в заданном порядке.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного общего реквизита или члена состава,
    /// неверного значения использования либо условия, не указывающего на константу.
    pub fn with_common_attribute_content(
        mut self,
        owner: &str,
        entries: Vec<CommonAttributeContentEntry>,
    ) -> Result<Self, CatalogError> {
        let invalid = || CatalogError::InvalidCommonAttributeContent(owner.into());
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::CommonAttribute
            })
            .ok_or_else(invalid)?;
        for entry in &entries {
            if self.object_by_id(&entry.metadata_id).is_none()
                || !matches!(
                    entry.usage,
                    EnumValue::CommonAttributeUseAuto
                        | EnumValue::CommonAttributeUseUse
                        | EnumValue::CommonAttributeUseDontUse
                )
            {
                return Err(invalid());
            }
            if let Some(CommonAttributeCondition::Constant(id)) = &entry.conditional_separation
                && !self
                    .object_by_id(id)
                    .is_some_and(|object| object.kind() == MetadataKind::Constant)
            {
                return Err(invalid());
            }
        }
        Arc::make_mut(&mut self.objects)[index].common_attribute_content = Some(entries.into());
        Ok(self)
    }
}

#[derive(Debug)]
struct Collection {
    catalog: Arc<MetadataCatalog>,
    entries: Arc<[CommonAttributeContentEntry]>,
}

impl Collection {
    fn item(&self, index: usize) -> Value {
        Value::new_object(Item {
            catalog: Arc::clone(&self.catalog),
            entries: Arc::clone(&self.entries),
            index,
        })
    }
}

impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &COLLECTION_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.entries.len())
    }

    fn collection_item(&self, index: &Value) -> RtResult<Value> {
        // Индекс нужен только протоколу обхода: BSL-индексация состава недоступна.
        let Value::Number(number) = index else {
            return Err(RtError::BadIndex);
        };
        let index = number
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        if index >= self.entries.len() {
            return Err(RtError::IndexOutOfBounds {
                index: index as i64,
                len: self.entries.len(),
            });
        }
        Ok(self.item(index))
    }
}

#[derive(Debug)]
struct Item {
    catalog: Arc<MetadataCatalog>,
    entries: Arc<[CommonAttributeContentEntry]>,
    index: usize,
}

impl ObjectProtocol for Item {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ITEM_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ITEM_PROPERTIES
    }

    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((
            Arc::as_ptr(&self.entries) as *const CommonAttributeContentEntry as usize,
            self.index,
        ))
    }
}

pub(super) fn get(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let metadata = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let entries = metadata
        .catalog
        .materialized_object_by_id(&metadata.object_id)?
        .and_then(|object| object.common_attribute_content.clone())
        .ok_or_else(|| RtError::DynamicError("полный состав общего реквизита неизвестен".into()))?;
    Ok(Value::new_object(Collection {
        catalog: Arc::clone(&metadata.catalog),
        entries,
    }))
}

fn count(receiver: &dyn ObjectProtocol, _: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let collection = receiver
        .downcast_ref::<Collection>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(collection.entries.len() as i64))
}

fn find(receiver: &dyn ObjectProtocol, args: &[Value], _: &mut CallContext<'_>) -> RtResult<Value> {
    let collection = receiver
        .downcast_ref::<Collection>()
        .ok_or(RtError::NotAnObject)?;
    let [argument] = args else {
        return Err(RtError::MethodNotApplicable {
            method: "Найти",
            receiver: COLLECTION_TYPE.name,
        });
    };
    let index = match argument {
        Value::Str(name) => collection.entries.iter().position(|entry| {
            collection
                .catalog
                .object_by_id(&entry.metadata_id)
                .is_some_and(|object| folded_eq(object.name(), &name.to_string()))
        }),
        argument => {
            let metadata = argument
                .object_ref()
                .and_then(|value| value.downcast_ref::<MetadataValue>())
                .ok_or(RtError::MethodNotApplicable {
                    method: "Найти",
                    receiver: COLLECTION_TYPE.name,
                })?;
            collection.entries.iter().position(|entry| {
                Arc::ptr_eq(&metadata.catalog, &collection.catalog)
                    && entry.metadata_id == metadata.object_id.as_ref()
            })
        }
    };
    Ok(index.map_or(Value::Undefined, |index| collection.item(index)))
}

fn contains(
    receiver: &dyn ObjectProtocol,
    args: &[Value],
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let collection = receiver
        .downcast_ref::<Collection>()
        .ok_or(RtError::NotAnObject)?;
    let [argument] = args else {
        return Err(RtError::MethodNotApplicable {
            method: "Содержит",
            receiver: COLLECTION_TYPE.name,
        });
    };
    if matches!(argument, Value::Str(_)) {
        return Ok(Value::Boolean(false));
    }
    let metadata = argument
        .object_ref()
        .and_then(|value| value.downcast_ref::<MetadataValue>())
        .ok_or(RtError::MethodNotApplicable {
            method: "Содержит",
            receiver: COLLECTION_TYPE.name,
        })?;
    Ok(Value::Boolean(
        Arc::ptr_eq(&metadata.catalog, &collection.catalog)
            && collection
                .entries
                .iter()
                .any(|entry| entry.metadata_id == metadata.object_id.as_ref()),
    ))
}

fn item_metadata(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let item = receiver
        .downcast_ref::<Item>()
        .ok_or(RtError::NotAnObject)?;
    let object = item
        .catalog
        .materialized_object_by_id(&item.entries[item.index].metadata_id)?
        .ok_or(RtError::NotAnObject)?;
    Ok(metadata_object_value(&item.catalog, object))
}

fn item_use(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let item = receiver
        .downcast_ref::<Item>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::Enum(item.entries[item.index].usage))
}

fn item_condition(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let item = receiver
        .downcast_ref::<Item>()
        .ok_or(RtError::NotAnObject)?;
    match &item.entries[item.index].conditional_separation {
        Some(CommonAttributeCondition::Undefined) => Ok(Value::Undefined),
        Some(CommonAttributeCondition::Constant(id)) => {
            let object = item
                .catalog
                .materialized_object_by_id(id)?
                .ok_or(RtError::NotAnObject)?;
            Ok(metadata_object_value(&item.catalog, object))
        }
        None => Err(RtError::DynamicError(
            "условие разделения элемента состава неизвестно".into(),
        )),
    }
}

pub(super) static COLLECTION_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "СоставОбщегоРеквизита",
    type_display: "Состав общего реквизита",
    type_names: &["CommonAttributeContent"],
};
pub(super) static ITEM_TYPE: TypeDescriptor = TypeDescriptor {
    package: "bsl-orm",
    name: "ЭлементСоставаОбщегоРеквизита",
    type_display: "Элемент состава общего реквизита",
    type_names: &["CommonAttributeContentItem"],
};
pub(super) static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), count),
    MethodDescriptor::new(&["Найти", "Find"], Arity::exact(1), find),
    MethodDescriptor::new(&["Содержит", "Contains"], Arity::exact(1), contains),
];
pub(super) static ITEM_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Метаданные", "Metadata"],
        get: item_metadata,
        set: None,
    },
    PropertyDescriptor {
        names: &["Использование", "Use"],
        get: item_use,
        set: None,
    },
    PropertyDescriptor {
        names: &["УсловноеРазделение", "ConditionalSeparation"],
        get: item_condition,
        set: None,
    },
];

impl MetadataObject {
    /// Возвращает приобретённое глобальное условие; `None` сохраняет неизвестные данные.
    #[must_use]
    pub fn common_attribute_condition(&self) -> Option<&CommonAttributeCondition> {
        self.common_attribute_condition.as_ref()
    }
}

impl MetadataCatalog {
    /// Присоединяет приобретённое глобальное условие разделения общего реквизита.
    ///
    /// # Errors
    /// Возвращает ошибку для неизвестного владельца или ссылки, не указывающей на константу.
    pub fn with_common_attribute_condition(
        mut self,
        owner: &str,
        condition: CommonAttributeCondition,
    ) -> Result<Self, CatalogError> {
        let invalid = || CatalogError::InvalidCommonAttributeSeparation(owner.into());
        let index = self
            .objects
            .iter()
            .position(|object| {
                object.id() == owner && object.kind() == MetadataKind::CommonAttribute
            })
            .ok_or_else(invalid)?;
        if let CommonAttributeCondition::Constant(id) = &condition
            && !self
                .object_by_id(id)
                .is_some_and(|object| object.kind() == MetadataKind::Constant)
        {
            return Err(invalid());
        }
        Arc::make_mut(&mut self.objects)[index].common_attribute_condition = Some(condition);
        Ok(self)
    }
}

pub(super) fn get_condition(
    receiver: &dyn ObjectProtocol,
    _: &mut CallContext<'_>,
) -> RtResult<Value> {
    let metadata = receiver
        .downcast_ref::<MetadataValue>()
        .ok_or(RtError::NotAnObject)?;
    let condition = metadata
        .catalog
        .materialized_object_by_id(&metadata.object_id)?
        .and_then(MetadataObject::common_attribute_condition)
        .ok_or_else(|| RtError::DynamicError("глобальное условие разделения неизвестно".into()))?;
    match condition {
        CommonAttributeCondition::Undefined => Ok(Value::Undefined),
        CommonAttributeCondition::Constant(id) => {
            let object = metadata
                .catalog
                .materialized_object_by_id(id)?
                .ok_or(RtError::NotAnObject)?;
            Ok(metadata_object_value(&metadata.catalog, object))
        }
    }
}
