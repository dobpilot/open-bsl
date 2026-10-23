use super::*;

fn state(hash: &str) -> open_bsl::State {
    let extension = ConfigurationExtension::new(
        [1; 16],
        "ProbeHash1013",
        "",
        hash,
        ExtensionSafeMode::Enabled(true),
        false,
        true,
        ConfigurationExtensionScope::InfoBase,
    )
    .unwrap();
    let snapshot =
        ConfigurationExtensionsSnapshot::new(vec![extension], [[1; 16]], Vec::new(), Vec::new())
            .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new("hash", Vec::new())
            .unwrap()
            .with_configuration_extensions(snapshot),
    );
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build()
}

#[test]
fn extension_hash_matches_the_native_binary_type_and_encoding() {
    let mut state = state("55K0byUrioB3aHqMgHSxrIug2Ls=");
    let source = include_str!("../../../docs/research/fixtures/extension-hash-type/measure.bsl")
        .replace("Сообщить(", "РезультатПробы.Добавить(")
        .replace(
            "КонецПроцедуры",
            "КонецПроцедуры\nРезультатПробы = Новый Массив;",
        );
    let Value::Object(object) = state
        .exec(&format!(
            "Перем РезультатПробы;\n{source}\nВозврат РезультатПробы;"
        ))
        .unwrap()
    else {
        panic!("ожидался массив");
    };
    let BslObject::Array(array) = object.as_ref() else {
        panic!("ожидался массив");
    };
    let actual = array
        .borrow()
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(
        actual,
        include_str!("../../../docs/research/fixtures/extension-hash-type/measure.platform.txt")
    );
    let result = state
        .exec("Возврат РасширенияКонфигурации.Получить()[0].hAsHsUm;")
        .unwrap();
    assert_eq!(
        result.binary_data_bytes().unwrap(),
        open_bsl::decode_base64("55K0byUrioB3aHqMgHSxrIug2Ls=").unwrap()
    );
}

#[test]
fn an_invalid_host_hash_fails_instead_of_inventing_binary_data() {
    let error = state("not base64!")
        .exec("Возврат РасширенияКонфигурации.Получить()[0].ХешСумма;")
        .unwrap_err();
    assert!(error.to_string().contains("некорректный Base64"), "{error}");
}
