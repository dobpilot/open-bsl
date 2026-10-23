//! Неизменяемые представления приобретённых реквизитов документа.

use super::*;

#[derive(Debug)]
pub(super) struct Collection {
    catalog: Arc<MetadataCatalog>,
    owner_id: Arc<str>,
    attributes: Arc<[DocumentAttribute]>,
}

impl Collection {
    pub(super) fn find(&self, name: &str) -> Option<Value> {
        self.attributes
            .iter()
            .position(|attribute| folded_eq(attribute.name(), name))
            .map(|index| {
                Value::new_object(Attribute {
                    attributes: Arc::clone(&self.attributes),
                    catalog: Arc::clone(&self.catalog),
                    owner_id: Arc::clone(&self.owner_id),
                    index,
                })
            })
    }
}

pub(super) fn collection(
    catalog: Arc<MetadataCatalog>,
    owner_id: Arc<str>,
    attributes: Arc<[DocumentAttribute]>,
) -> Value {
    Value::new_object(Collection {
        catalog,
        owner_id,
        attributes,
    })
}

impl ObjectProtocol for Collection {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_COLLECTION_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METADATA_COLLECTION_METHODS
    }
    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.attributes.len())
    }
    fn get_property(&self, name: &str, _: &mut CallContext<'_>) -> RtResult<Value> {
        self.find(name)
            .ok_or_else(|| RtError::UnknownProperty(name.into()))
    }
    fn get_index(&self, index: &Value) -> RtResult<Value> {
        if let Value::Str(name) = index {
            return self
                .find(&name.to_string())
                .ok_or_else(|| RtError::UnknownProperty(name.to_string()));
        }
        let Value::Number(number) = index else {
            return Err(RtError::BadIndex);
        };
        let index = number
            .trunc_to_scale(0)
            .to_i64_exact()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or(RtError::BadIndex)?;
        if index >= self.attributes.len() {
            return Err(RtError::IndexOutOfBounds {
                index: index as i64,
                len: self.attributes.len(),
            });
        }
        Ok(Value::new_object(Attribute {
            attributes: Arc::clone(&self.attributes),
            catalog: Arc::clone(&self.catalog),
            owner_id: Arc::clone(&self.owner_id),
            index,
        }))
    }
}

#[derive(Debug)]
pub(super) struct Attribute {
    pub(super) catalog: Arc<MetadataCatalog>,
    pub(super) owner_id: Arc<str>,
    attributes: Arc<[DocumentAttribute]>,
    index: usize,
}

impl Attribute {
    pub(super) fn descriptor(&self) -> &DocumentAttribute {
        &self.attributes[self.index]
    }

    pub(super) fn name(&self) -> &str {
        self.attributes[self.index].name()
    }
}

impl ObjectProtocol for Attribute {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METADATA_VALUE_TYPE
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((
            Arc::as_ptr(&self.attributes) as *const DocumentAttribute as usize,
            self.index,
        ))
    }
}

fn fill_flag(receiver: &dyn ObjectProtocol, _: &mut CallContext<'_>) -> RtResult<Value> {
    let attribute = receiver
        .downcast_ref::<Attribute>()
        .ok_or(RtError::NotAnObject)?;
    attribute.attributes[attribute.index]
        .fill_from_filling_value()
        .map(Value::Boolean)
        .ok_or_else(|| {
            RtError::DynamicError("настройка заполнения реквизита не приобретена".into())
        })
}

fn fill_value(receiver: &dyn ObjectProtocol, context: &mut CallContext<'_>) -> RtResult<Value> {
    let attribute = receiver
        .downcast_ref::<Attribute>()
        .ok_or(RtError::NotAnObject)?;
    let graph = attribute.attributes[attribute.index]
        .fill_value()
        .ok_or_else(|| {
            RtError::DynamicError("значение заполнения реквизита не приобретено".into())
        })?;
    let mut values = graph.materialize(context.runtime_shapes())?;
    if values.len() != 1 {
        return Err(RtError::DynamicError(
            "снимок значения заполнения должен иметь один корень".into(),
        ));
    }
    Ok(values.pop().expect("число корней проверено"))
}

pub(super) static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Имя", "Name"],
        get: metadata_value_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["ЗаполнятьИзДанныхЗаполнения", "FillFromFillingValue"],
        get: fill_flag,
        set: None,
    },
    PropertyDescriptor {
        names: &["ЗначениеЗаполнения", "FillValue"],
        get: fill_value,
        set: None,
    },
];

#[cfg(test)]
#[path = "metadata_attributes_tests.rs"]
mod tests;
