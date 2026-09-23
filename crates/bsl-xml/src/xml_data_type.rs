//! Значение `ТипДанныхXML` и отображение примитивных значений BSL в XSD.

use std::hash::{Hash, Hasher};

use bsl_rt::{
    BslString, BslValue, CallContext, ObjectMembersDescriptor, ObjectProtocol, ObjectRef,
    PropertyDescriptor, RtError, RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static XML_DATA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТипДанныхXML",
    type_display: "XML data type",
    type_names: &["XMLDataType"],
};

#[derive(Debug)]
struct XmlDataTypeObject {
    name: BslString,
    namespace: BslString,
}

impl ObjectProtocol for XmlDataTypeObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &XML_DATA_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        PROPERTIES
    }

    fn get_property(&self, name: &str, context: &mut CallContext<'_>) -> RtResult<BslValue> {
        bsl_rt::get_property_from_table(PROPERTIES, "ТипДанныхXML", self, name, context)
    }

    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        other
            .downcast_ref::<Self>()
            .map(|other| self.name == other.name && self.namespace == other.namespace)
    }

    fn value_hash_key(&self) -> Option<u64> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.namespace.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn display(&self) -> String {
        "XMLDataType".into()
    }
}

fn get_name(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    let value = receiver_of::<XmlDataTypeObject>(receiver, "ИмяТипа")?;
    Ok(BslValue::Str(value.name.clone()))
}

fn get_namespace(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let value = receiver_of::<XmlDataTypeObject>(receiver, "URIПространстваИмен")?;
    Ok(BslValue::Str(value.namespace.clone()))
}

static PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ИмяТипа", "TypeName"],
        get: get_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["URIПространстваИмен", "NamespaceURI"],
        get: get_namespace,
        set: None,
    },
];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&XML_DATA_TYPE).with_properties(PROPERTIES)];

fn new_value(name: &str, namespace: &str) -> BslValue {
    BslValue::new_object(XmlDataTypeObject {
        name: name.into(),
        namespace: namespace.into(),
    })
}

pub(crate) fn construct(arguments: &[BslValue]) -> RtResult<BslValue> {
    let [BslValue::Str(name), BslValue::Str(namespace)] = arguments else {
        return Err(RtError::TypeError {
            expected: "две строки",
            op: "Новый ТипДанныхXML",
        });
    };
    Ok(new_value(&name.to_string(), &namespace.to_string()))
}

pub(crate) fn type_of(value: &BslValue) -> BslValue {
    super::xdto::builtin_name_for_value(value).map_or(BslValue::Undefined, |name| {
        new_value(name, super::xsd::XSD_NS)
    })
}
