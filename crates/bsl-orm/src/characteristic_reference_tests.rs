use super::*;

#[test]
fn characteristic_manager_matches_the_native_reference_control() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "reference",
            vec![MetadataObject::new(
                "characteristic",
                "ЗамерХарактеристик",
                MetadataKind::ChartOfCharacteristicTypes,
            )],
        )
        .unwrap(),
    );
    let mut builder = open_bsl::Engine::builder().register_library(library());
    for ty in catalog.value_types() {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("test", "Тест", Vec::new()),
        )))
        .build();
    let source = include_str!(
        "../../../docs/research/fixtures/characteristic-manager-reference/measure.bsl"
    )
    .replace("Сообщить(", "РезультатПробы.Добавить(");
    let value = state
        .exec(&format!(
            "РезультатПробы = Новый Массив;\n{source}\nВозврат РезультатПробы;"
        ))
        .unwrap();
    let Value::Object(object) = value else {
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
            "../../../docs/research/fixtures/characteristic-manager-reference/measure.platform.txt"
        )
    );
}
