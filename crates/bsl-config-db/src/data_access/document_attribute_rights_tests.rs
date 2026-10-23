use super::*;

#[test]
fn a_document_read_grant_does_not_answer_an_attribute_read_check() {
    const DOCUMENT: &str = "f77edfd7-d0d4-532b-a649-399fb412f0a4";
    const ROLE: &str = "d0c7f3bf-7e25-5068-9d5a-35fa41923ea2";
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    for privileged in [false, true] {
        let mut access = AccessStore::new(&snapshot);
        access.insert_rights(ROLE.parse().unwrap(), parse_role_rights(include_bytes!(
            "../../../../docs/research/fixtures/document-right-profiles-1019/owner-all-children/ControlReadOn.rights.deflate"
        )).unwrap());
        let catalog = Arc::new(
            MetadataCatalog::new(
                "document-rights",
                vec![
                    MetadataObject::new(DOCUMENT, "Проба", MetadataKind::Document)
                        .with_document_attributes(vec![bsl_orm::DocumentAttribute::new(
                            "ae787631-1111-4444-8888-123456789001",
                            "Заполняемый",
                            vec![],
                        )]),
                ],
            )
            .unwrap(),
        );
        let data = SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![ROLE.to_owned()]),
        )
        .with_data_access(Rc::new(Host { access }));
        let engine = open_bsl::Engine::builder()
            .register_library(bsl_orm::library())
            .register_library(bsl_session::library())
            .build()
            .unwrap();
        let mut state = engine.state_builder().host_data(Rc::new(data)).build();
        let prefix = if privileged {
            "УстановитьПривилегированныйРежим(Истина);"
        } else {
            ""
        };
        assert_eq!(
            state
                .exec(&format!(
                    "{prefix} Возврат ПравоДоступа(\"Read\", Метаданные.Документы.Проба);"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
        let failure = state.exec(&format!(
            "{prefix} Возврат ПравоДоступа(\"Read\", Метаданные.Документы.Проба.Реквизиты.Заполняемый);"
        )).unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("проверка прав реквизита документа не поддерживается"),
            "{failure}"
        );
    }
}
