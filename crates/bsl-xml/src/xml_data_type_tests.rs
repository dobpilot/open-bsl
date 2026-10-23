use super::*;
use bsl_rt::{TypeId, TypeRef};

#[test]
fn canonical_inverse_types_match_the_native_control() {
    for (name, expected) in [
        ("string", TypeId::String),
        ("decimal", TypeId::Number),
        ("boolean", TypeId::Boolean),
        ("dateTime", TypeId::Date),
        ("base64Binary", TypeId::BinaryData),
    ] {
        let arguments = [text(name), text(super::super::xsd::XSD_NS)];
        let expected = BslValue::Type(TypeRef::Native(expected));
        assert_eq!(from_type(&arguments).unwrap(), expected);
        let object = construct(&arguments).unwrap();
        assert_eq!(from_type(&[object]).unwrap(), expected);
    }
}

#[test]
fn lexical_xdto_types_do_not_gain_an_inverse_native_mapping() {
    for name in [
        "integer",
        "double",
        "date",
        "time",
        "hexBinary",
        "QName",
        "token",
        "String",
        "unknown",
    ] {
        assert_eq!(
            from_type(&[text(name), text(super::super::xsd::XSD_NS)]).unwrap(),
            BslValue::Undefined
        );
    }
    assert_eq!(
        from_type(&[text("string"), text("urn:unknown")]).unwrap(),
        BslValue::Undefined
    );
}

#[test]
fn inverse_lookup_rejects_invalid_overloads() {
    for arguments in [
        vec![],
        vec![BslValue::Undefined],
        vec![text("string")],
        vec![BslValue::number_from_i64(1)],
        vec![text("string"), BslValue::number_from_i64(1)],
    ] {
        assert!(from_type(&arguments).is_err());
    }
}

fn text(value: &str) -> BslValue {
    BslValue::Str(value.into())
}
