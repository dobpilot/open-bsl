use super::*;
use bsl_orm::{AccessRightRequest, CatalogCodeType, CatalogStandardAttributes};
use bsl_session::SessionError;
use open_sdbl::metadata::{parse_config_descriptors, parse_db_names};
use std::path::{Path, PathBuf};

fn native_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/metadata-right-host-integration-1018/native")
}

struct MetadataHost {
    metadata: MetadataSnapshot,
    access: AccessStore,
}

impl DataAccess for MetadataHost {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("матрица прав не читает константы")
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        configuration_rights::metadata_access_right(
            &self.access,
            &self.metadata,
            request,
            principal,
            context,
            false,
        )
    }

    fn verify_access_rights(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        let granted = configuration_rights::metadata_access_right(
            &self.access,
            &self.metadata,
            request,
            principal,
            context,
            true,
        )?;
        access_verification::record_verification(request, context, granted)
    }
}

fn factory() -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "metadata-right-matrix",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "metadata-right-matrix",
        vec![
            MetadataObject::new(
                "1316b080-3b2c-501b-ace3-a1234a3fedd1",
                "ВладелецПробы",
                MetadataKind::Catalog,
            )
            .with_catalog_standard_attributes(CatalogStandardAttributes {
                code_type: CatalogCodeType::String,
                code_length: 9,
                description_length: 25,
            }),
        ],
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
fn all_native_right_rows_replay_through_bsl_adapter_policy_and_session_journal() {
    let root = native_root();
    let snapshot = resolve_metadata(
        parse_db_names(&std::fs::read(root.join("dbnames.deflate")).unwrap()).unwrap(),
        parse_config_descriptors(
            "1316b080-3b2c-501b-ace3-a1234a3fedd1",
            &std::fs::read(root.join("catalog.deflate")).unwrap(),
        )
        .unwrap(),
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let factory = factory();
    let cases = std::fs::read_to_string(root.join("cases.tsv")).unwrap();
    assert_eq!(cases.lines().count(), 492);
    for (index, line) in cases.lines().enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7);
        let mut access = AccessStore::new(&snapshot);
        let mut roles = Vec::new();
        for (number, path) in fields[6]
            .split(',')
            .filter(|path| !path.is_empty())
            .enumerate()
        {
            let id = format!("00000000-0000-0000-0000-{:012}", number + 1);
            access.insert_rights(
                id.parse().unwrap(),
                parse_role_rights(&std::fs::read(root.join(path)).unwrap()).unwrap(),
            );
            roles.push(id);
        }
        let host = Rc::new(MetadataHost {
            metadata: snapshot.clone(),
            access,
        });
        let journal = Rc::new(Journal::default());
        let mut session = factory
            .open(
                Principal::new("probe", "", roles),
                SessionResources::new(Box::new(()), index as u64 + 1)
                    .with_data_access(host)
                    .with_event_log_writer(journal.clone()),
            )
            .unwrap();
        let standard = match fields[3] {
            "undefined" => "Неопределено".to_owned(),
            "empty" => "\"\"".to_owned(),
            name => format!("\"{name}\""),
        };
        let call = if fields[1] == "access" {
            format!(
                "Возврат AccessRight(\"{}\", Метаданные.Справочники.ВладелецПробы, Неопределено, {standard});",
                fields[2]
            )
        } else {
            format!(
                "VerifyAccessRights(\"{}\", Метаданные.Справочники.ВладелецПробы, {standard}); Возврат Истина;",
                fields[2]
            )
        };
        let privilege = if fields[4] == "true" {
            "УстановитьПривилегированныйРежим(Истина);"
        } else {
            ""
        };
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт {privilege} {call} КонецФункции"
            ))
            .unwrap();
        let actual = session.call(&module, "Проверить", vec![]);
        let label = format!("{} {} {} {}", fields[0], fields[1], fields[2], fields[3]);
        match fields[5] {
            "allowed" => assert!(
                matches!(actual, Ok((Value::Boolean(true), _))),
                "{label}: {actual:?}"
            ),
            "denied" if fields[1] == "access" => assert!(
                matches!(actual, Ok((Value::Boolean(false), _))),
                "{label}: {actual:?}"
            ),
            "denied" | "not_applicable" | "unknown_right" => assert!(
                matches!(
                    actual,
                    Err(SessionError::OpenBsl(open_bsl::Error::Runtime(
                        RtError::DynamicError(_)
                    )))
                ),
                "{label}: {actual:?}"
            ),
            "invalid_standard_path" => assert!(
                matches!(
                    actual,
                    Err(SessionError::OpenBsl(open_bsl::Error::Runtime(
                        RtError::DynamicError(_) | RtError::TypeError { .. }
                    )))
                ),
                "{label}: {actual:?}"
            ),
            value => panic!("неизвестная категория нативного результата {value}"),
        }
        let denied_verification = fields[1] == "verify" && fields[5] == "denied";
        assert_eq!(
            journal.entries.borrow().len(),
            usize::from(denied_verification),
            "{label}"
        );
        session.execute("КонтрольПробы = 1;").unwrap();
    }
}

#[path = "metadata_explicit_subject_tests.rs"]
mod explicit_subjects;

#[path = "document_right_matrix_tests.rs"]
mod document_rights;
