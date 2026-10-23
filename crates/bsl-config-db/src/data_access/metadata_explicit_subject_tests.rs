use super::*;
use open_sdbl::metadata::{InfoBaseUser, UserRow};

const SAVE: &str = "b1df1c9b-a628-4963-8ed3-fb0f314688b9";
const OWNER: &str = "1316b080-3b2c-501b-ace3-a1234a3fedd1";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/metadata-explicit-subjects-native-1018")
}

fn user(id: &str, name: &str, role: &str) -> InfoBaseUser {
    // Идентификаторы пользователей модельные; состав и права ролей взяты из CF.
    let record = format!("{{{id},\"{name}\",\"\",{{1,{role}}}}}");
    let mut encoded = vec![1, 0x5a];
    encoded.extend(record.bytes().map(|byte| byte ^ 0x5a));
    InfoBaseUser::new(
        UserRow {
            name,
            description: "",
            os_name: "",
            email: "",
            show_in_list: false,
            standard_authentication: true,
            administrative: false,
        },
        &encoded,
    )
    .unwrap()
}

fn open(profile: &str, missing_role: Option<&str>, root_known: bool) -> Session {
    let root = fixtures();
    let metadata = resolve_metadata(
        parse_db_names(&std::fs::read(root.join("dbnames.deflate")).unwrap()).unwrap(),
        parse_config_descriptors(OWNER, &std::fs::read(root.join("catalog.deflate")).unwrap())
            .unwrap(),
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let mut access = AccessStore::new(&metadata);
    for (name, id) in [
        ("RootAllow", ALLOW),
        ("RootDeny", DENY),
        ("RootSaveOnly", SAVE),
    ] {
        if missing_role != Some(id) {
            access.insert_rights(
                id.parse().unwrap(),
                parse_role_rights(
                    &std::fs::read(root.join(format!("{name}.rights.deflate"))).unwrap(),
                )
                .unwrap(),
            );
        }
    }
    let users = [
        (
            "11111111-1111-1111-1111-111111111111",
            "RootProbeAdmin",
            ALLOW,
        ),
        (
            "22222222-2222-2222-2222-222222222222",
            "RootProbeDeny",
            DENY,
        ),
    ];
    access.set_users(
        users
            .iter()
            .map(|(id, name, role)| user(id, name, role))
            .collect(),
    );
    let (id, name, role) = users[usize::from(profile == "deny")];
    let code = ConfigurationCodeSnapshot::new(
        "explicit-subjects",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let mut catalog = MetadataCatalog::new(
        "explicit-subjects",
        vec![
            MetadataObject::new(OWNER, "ВладелецПробы", MetadataKind::Catalog)
                .with_catalog_standard_attributes(CatalogStandardAttributes {
                    code_type: CatalogCodeType::String,
                    code_length: 9,
                    description_length: 25,
                }),
            MetadataObject::new(ALLOW, "RootAllow", MetadataKind::Role),
            MetadataObject::new(DENY, "RootDeny", MetadataKind::Role),
            MetadataObject::new(SAVE, "RootSaveOnly", MetadataKind::Role),
        ],
    )
    .unwrap()
    .with_configuration(
        "Configuration",
        "1",
        ScriptVariant::English,
        CompatibilityMode::DontUse,
        [],
    )
    .unwrap();
    if root_known {
        catalog = catalog.with_configuration_object_id(ROOT).unwrap();
    }
    let factory = SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    factory
        .open(
            Principal::new(id, name, vec![role.to_owned()]),
            SessionResources::new(Box::new(()), 1)
                .with_data_access(Rc::new(MetadataHost { metadata, access })),
        )
        .unwrap()
}

fn evaluate(session: &mut Session, body: &str) -> Result<Value, SessionError> {
    let module =
        session.instantiate(&format!("Функция Проверить() Экспорт {body} КонецФункции"))?;
    session
        .call(&module, "Проверить", vec![])
        .map(|result| result.0)
}

#[test]
fn explicit_catalog_subjects_replay_all_120_native_rows() {
    let mut count = 0;
    for profile in ["admin", "deny"] {
        let mut session = open(profile, None, true);
        let native = std::fs::read_to_string(fixtures().join(format!("{profile}.oracle"))).unwrap();
        for line in native.lines().filter(|line| line.starts_with("priv=")) {
            let parts = line.splitn(4, ';').collect::<Vec<_>>();
            let privilege = if parts[0] == "priv=1" {
                "УстановитьПривилегированныйРежим(Истина);"
            } else {
                ""
            };
            let subject = match parts[1] {
                "subject=0" => "Неопределено",
                "subject=1" => "Метаданные.Роли.RootAllow",
                "subject=2" => "Метаданные.Роли.RootDeny",
                "subject=3" => "Метаданные.Роли.RootSaveOnly",
                "subject=4" => "ПользователиИнформационнойБазы.ТекущийПользователь()",
                value => panic!("неизвестный субъект {value}"),
            };
            let standard = match parts[2] {
                "standard=0" => "Неопределено",
                "standard=1" => "\"Code\"",
                "standard=2" => "\"Description\"",
                value => panic!("неизвестный стандартный реквизит {value}"),
            };
            let (right, expected) = parts[3]
                .strip_prefix("right=")
                .unwrap()
                .split_once('=')
                .unwrap();
            let body = format!(
                "{privilege} Возврат AccessRight(\"{right}\", Метаданные.Справочники.ВладелецПробы, {subject}, {standard});"
            );
            let actual = evaluate(&mut session, &body);
            match expected {
                "YES" => assert_eq!(actual.unwrap(), Value::Boolean(true), "{profile}: {line}"),
                "NO" => assert_eq!(actual.unwrap(), Value::Boolean(false), "{profile}: {line}"),
                value => {
                    let message = actual.unwrap_err().to_string();
                    let expected_kind = if value.contains("Cannot apply") {
                        "неприменимо"
                    } else {
                        assert!(value.contains("administrative rights"), "{value}");
                        "административное"
                    };
                    assert!(
                        message.contains(expected_kind),
                        "{profile}: {line}: {message}"
                    );
                }
            }
            session.execute("КонтрольПробы = 1;").unwrap();
            count += 1;
        }
    }
    assert_eq!(count, 120);
}

#[test]
fn explicit_subject_cannot_borrow_privilege_or_a_missing_configuration_root() {
    let mut missing = open("admin", Some(SAVE), true);
    let error = evaluate(&mut missing, "УстановитьПривилегированныйРежим(Истина); Возврат AccessRight(\"View\", Метаданные.Справочники.ВладелецПробы, Метаданные.Роли.RootSaveOnly);").unwrap_err();
    assert!(
        error.to_string().contains("complete role rights") && error.to_string().contains(SAVE),
        "{error}"
    );
    let mut no_root = open("admin", None, false);
    let error = evaluate(&mut no_root, "Возврат AccessRight(\"Read\", Метаданные.Справочники.ВладелецПробы, ПользователиИнформационнойБазы.ТекущийПользователь());").unwrap_err();
    assert!(error.to_string().contains("корня конфигурации"), "{error}");
}
