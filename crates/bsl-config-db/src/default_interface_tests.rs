use super::*;

fn native_properties(raw: &[u8]) -> Vec<Value> {
    let value = parse_serialized(&inflate_raw_deflate(raw).unwrap()).unwrap();
    configuration_properties(&value).unwrap().to_vec()
}

#[test]
fn native_root_pair_projects_the_exact_default_interface_uuid() {
    for (raw, expected) in [
        (include_bytes!("../../../docs/research/fixtures/configuration-default-interface-native-1016/empty/root-resource.deflate").as_slice(), [0;16]),
        (include_bytes!("../../../docs/research/fixtures/configuration-default-interface-native-1016/present/root-resource.deflate").as_slice(), open_bsl::uuid::parse("e9008419-cc3a-4a87-8b7b-50eb53b985bf").unwrap()),
    ] {
        let properties = native_properties(raw);
        assert_eq!(configuration_default_interface_id(&properties), Some(expected));
    }
}

#[test]
fn malformed_default_interface_profiles_remain_unknown() {
    let source = native_properties(include_bytes!(
        "../../../docs/research/fixtures/configuration-default-interface-native-1016/empty/root-resource.deflate"
    ));
    for case in 0..7 {
        let mut properties = source.clone();
        match case {
            0 => {
                properties.pop();
            }
            1 => properties[0] = Value::Atom("67".into()),
            2 => properties[0] = Value::String("68".into()),
            3 => properties[11] = Value::String("00000000-0000-0000-0000-000000000000".into()),
            4 => properties[11] = Value::Atom("не UUID".into()),
            5 => properties[11] = Value::List(vec![source[11].clone()]),
            6 => properties[11] = Value::Null,
            _ => unreachable!(),
        }
        assert_eq!(
            configuration_default_interface_id(&properties),
            None,
            "случай {case}"
        );
    }
    let mut properties = source;
    properties[9] = Value::Atom("e9008419-cc3a-4a87-8b7b-50eb53b985bf".into());
    assert_eq!(
        configuration_default_interface_id(&properties),
        Some([0; 16])
    );
}
