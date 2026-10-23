use super::*;
use open_bsl::Engine;

#[test]
fn native_metadata_type_names_preserve_identity_and_distinguish_the_root() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "metadata-type-names",
            vec![MetadataObject::new("doc", "Проба", MetadataKind::Document)],
        )
        .unwrap(),
    );
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build();
    for name in [
        "ОбъектМетаданных",
        "MetadataObject",
        "ОбъектМетаданныхКонфигурации",
    ] {
        assert_eq!(
            state
                .exec(&format!(
                    "Возврат ТипЗнч(Метаданные.Документы.Проба) = Тип(\"{name}\");"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert_eq!(
        state
            .exec("Возврат ТипЗнч(Метаданные) = Тип(\"ОбъектМетаданных\");")
            .unwrap(),
        Value::Boolean(false)
    );
}
