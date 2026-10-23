use super::*;

const DOCUMENT: &str = "f77edfd7-d0d4-532b-a649-399fb412f0a4";

fn native_document_snapshot() -> MetadataSnapshot {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/document-metadata-rights-0929");
    resolve_metadata(
        parse_db_names(&std::fs::read(root.join("dbnames.deflate")).unwrap()).unwrap(),
        parse_config_descriptors(
            DOCUMENT,
            &std::fs::read(root.join("document.deflate")).unwrap(),
        )
        .unwrap(),
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot
}

fn document_factory() -> SessionFactory {
    let catalog = MetadataCatalog::new(
        "document-right-matrix",
        vec![
            MetadataObject::new(DOCUMENT, "Проба", MetadataKind::Document)
                .with_document_number_settings(bsl_orm::DocumentNumberSettings {
                    number_type: bsl_orm::DocumentNumberType::String,
                    number_length: 9,
                })
                .with_document_attributes(vec![bsl_orm::DocumentAttribute::new(
                    "ae787631-1111-4444-8888-123456789001",
                    "Заполняемый",
                    vec![],
                )]),
        ],
    )
    .unwrap();
    let code = ConfigurationCodeSnapshot::new(
        "document-right-matrix",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn all_native_document_right_rows_replay_through_host_policy_and_denial_journal() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/document-metadata-rights-0929");
    let snapshot = native_document_snapshot();
    let factory = document_factory();
    let cases = std::fs::read_to_string(root.join("cases.tsv")).unwrap();
    assert_eq!(cases.lines().count(), 432);
    for (index, line) in cases.lines().enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7);
        let role = "00000000-0000-0000-0000-000000000001";
        let mut access = AccessStore::new(&snapshot);
        access.insert_rights(
            role.parse().unwrap(),
            parse_role_rights(&std::fs::read(root.join(fields[6])).unwrap()).unwrap(),
        );
        let journal = Rc::new(Journal::default());
        let mut session = factory
            .open(
                Principal::new("probe", "", vec![role.into()]),
                SessionResources::new(Box::new(()), index as u64 + 1)
                    .with_data_access(Rc::new(MetadataHost {
                        metadata: snapshot.clone(),
                        access,
                    }))
                    .with_event_log_writer(journal.clone()),
            )
            .unwrap();
        let target = if fields[1] == "attribute" {
            "Метаданные.Документы.Проба.Реквизиты.Заполняемый"
        } else {
            "Метаданные.Документы.Проба"
        };
        let reader = session
            .instantiate(&format!(
                "Функция Цель() Экспорт Возврат {target}; КонецФункции"
            ))
            .unwrap();
        let target = session.call(&reader, "Цель", vec![]).unwrap().0;
        let standard = match fields[1] {
            "owner" | "attribute" => "Неопределено",
            "number.ru" => "\"Номер\"",
            "number.en" => "\"Number\"",
            "date.ru" => "\"Дата\"",
            "date.en" => "\"Date\"",
            other => panic!("неизвестная цель {other}"),
        };
        let call = if fields[3] == "access" {
            format!(
                "Возврат AccessRight(\"{}\", Мета, Неопределено, {standard});",
                fields[2]
            )
        } else {
            format!(
                "VerifyAccessRights(\"{}\", Мета, {standard}); Возврат Истина;",
                fields[2]
            )
        };
        let privilege = if fields[4] == "1" {
            "УстановитьПривилегированныйРежим(Истина);"
        } else {
            ""
        };
        let module = session
            .instantiate(&format!(
                "Функция Проверить(Мета) Экспорт {privilege} {call} КонецФункции"
            ))
            .unwrap();
        let actual = session.call(&module, "Проверить", vec![target.clone()]);
        let label = format!("{} {} {} {}", fields[0], fields[1], fields[2], fields[3]);
        match fields[5] {
            "allowed" => assert!(
                matches!(actual, Ok((Value::Boolean(true), _))),
                "{label}: {actual:?}"
            ),
            "denied" if fields[3] == "access" => assert!(
                matches!(actual, Ok((Value::Boolean(false), _))),
                "{label}: {actual:?}"
            ),
            "denied" | "not_applicable" => {
                let expected = if fields[5] == "not_applicable" {
                    "неприменимо"
                } else {
                    "Нарушение прав доступа"
                };
                assert!(
                    matches!(
                        &actual,
                        Err(SessionError::OpenBsl(open_bsl::Error::Runtime(
                            RtError::DynamicError(message)
                        ))) if message.contains(expected)
                    ),
                    "{label}: {actual:?}"
                );
            }
            other => panic!("неизвестный нативный результат {other}"),
        }
        let entries = journal.entries.borrow();
        let denied = fields[3] == "verify" && fields[5] == "denied";
        assert_eq!(entries.len(), usize::from(denied), "{label}");
        if let Some(entry) = entries.first() {
            assert_eq!(entry.event_name(), "_$Access$_.AccessDenied");
            assert_eq!(
                entry.transaction_mode(),
                EventLogTransactionMode::Independent
            );
            assert!(std::ptr::eq(
                target.object_ref().unwrap(),
                entry.metadata().object_ref().unwrap()
            ));
        }
    }
}

#[test]
fn acquired_document_structure_is_checked_before_privilege_and_no_denial_is_logged() {
    let source = native_document_snapshot();
    let owner: Guid = DOCUMENT.parse().unwrap();
    let attribute: Guid = "ae787631-1111-4444-8888-123456789001".parse().unwrap();
    for damage in 0..4 {
        let mut descriptors = source.descriptors().to_vec();
        match damage {
            0 => {
                descriptors
                    .iter_mut()
                    .find(|item| item.object_guid == owner)
                    .unwrap()
                    .document_attributes = None
            }
            1 => {
                descriptors
                    .iter_mut()
                    .find(|item| item.object_guid == owner)
                    .unwrap()
                    .document_attributes = Some(vec![])
            }
            2 => descriptors.retain(|item| item.object_guid != attribute),
            3 => descriptors.push(
                descriptors
                    .iter()
                    .find(|item| item.object_guid == attribute)
                    .unwrap()
                    .clone(),
            ),
            _ => unreachable!(),
        }
        let snapshot = resolve_metadata(
            source.db_names().clone(),
            descriptors,
            source.schema().clone(),
            vec![],
        )
        .snapshot;
        for privileged in [false, true] {
            let journal = Rc::new(Journal::default());
            let mut session = document_factory()
                .open(
                    Principal::new("probe", "", vec![]),
                    SessionResources::new(Box::new(()), 1)
                        .with_data_access(Rc::new(MetadataHost {
                            metadata: snapshot.clone(),
                            access: AccessStore::new(&snapshot),
                        }))
                        .with_event_log_writer(journal.clone()),
                )
                .unwrap();
            let privilege = if privileged {
                "УстановитьПривилегированныйРежим(Истина);"
            } else {
                ""
            };
            for call in [
                "Возврат ПравоДоступа(\"Read\", Метаданные.Документы.Проба.Реквизиты.Заполняемый);",
                "ВыполнитьПроверкуПравДоступа(\"Read\", Метаданные.Документы.Проба.Реквизиты.Заполняемый); Возврат Истина;",
            ] {
                let module = session
                    .instantiate(&format!(
                        "Функция Проверить() Экспорт {privilege} {call} КонецФункции"
                    ))
                    .unwrap();
                let failure = session.call(&module, "Проверить", vec![]).unwrap_err();
                let message = failure.to_string();
                assert!(!message.contains("неприменимо"), "{damage}: {message}");
                assert!(
                    !message.contains("Нарушение прав доступа"),
                    "{damage}: {message}"
                );
                assert!(journal.entries.borrow().is_empty());
            }
        }
    }
}

#[test]
fn an_attribute_target_with_a_standard_path_is_not_silently_reinterpreted() {
    let snapshot = native_document_snapshot();
    for privileged in [false, true] {
        let journal = Rc::new(Journal::default());
        let mut session = document_factory()
            .open(
                Principal::new("probe", "", vec![]),
                SessionResources::new(Box::new(()), 1)
                    .with_data_access(Rc::new(MetadataHost {
                        metadata: snapshot.clone(),
                        access: AccessStore::new(&snapshot),
                    }))
                    .with_event_log_writer(journal.clone()),
            )
            .unwrap();
        let privilege = if privileged {
            "УстановитьПривилегированныйРежим(Истина);"
        } else {
            ""
        };
        for call in [
            "Возврат AccessRight(\"Read\", Метаданные.Документы.Проба.Реквизиты.Заполняемый, Неопределено, \"Дата\");",
            "VerifyAccessRights(\"Read\", Метаданные.Документы.Проба.Реквизиты.Заполняемый, \"Дата\"); Возврат Истина;",
        ] {
            let module = session
                .instantiate(&format!(
                    "Функция Проверить() Экспорт {privilege} {call} КонецФункции"
                ))
                .unwrap();
            let failure = session
                .call(&module, "Проверить", vec![])
                .unwrap_err()
                .to_string();
            assert!(
                failure.contains("стандартный путь для реквизита документа пока не поддерживается"),
                "{failure}"
            );
            assert!(journal.entries.borrow().is_empty());
        }
    }
}
