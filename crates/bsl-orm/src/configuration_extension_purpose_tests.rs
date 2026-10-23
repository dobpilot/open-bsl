use super::*;

#[test]
fn scoped_extension_purpose_matches_the_native_namespace_control() {
    let catalog = Arc::new(MetadataCatalog::new("purpose", Vec::new()).unwrap());
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    let source =
        include_str!("../../../docs/research/fixtures/configuration-extension-purpose/measure.bsl")
            .replace("Сообщить(", "РезультатПробы.Добавить(");
    let Value::Object(object) = state
        .exec(&format!(
            "РезультатПробы = Новый Массив;\n{source}\nВозврат РезультатПробы;"
        ))
        .unwrap()
    else {
        panic!("ожидался массив")
    };
    let BslObject::Array(array) = object.as_ref() else {
        panic!("ожидался массив")
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
        include_str!(
            "../../../docs/research/fixtures/configuration-extension-purpose/measure.platform.txt"
        )
    );
}

fn extension_state(purpose: Option<ConfigurationExtensionPurpose>) -> open_bsl::State {
    let mut extension = ConfigurationExtension::new(
        [1; 16],
        "EFИмяНеОпределяетНазначение",
        "1.0",
        "hash",
        ExtensionSafeMode::Enabled(true),
        false,
        true,
        ConfigurationExtensionScope::InfoBase,
    )
    .unwrap();
    if let Some(purpose) = purpose {
        extension = extension.with_purpose(purpose);
    }
    let snapshot =
        ConfigurationExtensionsSnapshot::new(vec![extension], [[1; 16]], Vec::new(), Vec::new())
            .unwrap();
    let catalog = Arc::new(
        MetadataCatalog::new("purpose", Vec::new())
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
fn supplied_extension_purposes_preserve_identity_and_reject_assignment() {
    for (purpose, member) in [
        (ConfigurationExtensionPurpose::Patch, "Исправление"),
        (ConfigurationExtensionPurpose::Customization, "Адаптация"),
        (ConfigurationExtensionPurpose::AddOn, "Дополнение"),
    ] {
        let mut state = extension_state(Some(purpose));
        for property in ["Назначение", "pUrPoSe"] {
            let source = format!(
                r#"
                РасширениеПробы = РасширенияКонфигурации.Получить()[0];
                ОжидаемоеПробы = Метаданные.СвойстваОбъектов.НазначениеРасширенияКонфигурации.{member};
                БылоПробы = РасширениеПробы.{property};
                ОтказПробы = Ложь;
                Попытка РасширениеПробы.{property} = ОжидаемоеПробы; Исключение ОтказПробы = Истина; КонецПопытки;
                Возврат БылоПробы = ОжидаемоеПробы И РасширениеПробы.{property} = БылоПробы И ОтказПробы;
            "#
            );
            assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
        }
    }
}

#[test]
fn unknown_extension_purpose_is_not_derived_from_an_ef_name_or_other_properties() {
    let mut state = extension_state(None);
    for property in ["Назначение", "Purpose"] {
        let error = state
            .exec(&format!(
                "Возврат РасширенияКонфигурации.Получить()[0].{property};"
            ))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("назначение расширения не приобретено"),
            "{error}"
        );
    }
}
