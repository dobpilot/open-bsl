use open_bsl::*;
use std::{path::PathBuf, rc::Rc, sync::Arc};

fn template(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    let artifact = context
        .host_data()?
        .downcast_ref::<Arc<ConfigurationArtifact>>()
        .unwrap()
        .clone();
    configuration_template_value(&artifact, context)
}
static FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
    code: FunctionCode::new(1),
    names: &["МакетФайла"],
    arity: Arity::exact(0),
    kind: FunctionKind::Function,
    call: template,
}];
fn native() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/file-configuration-native-1022/native-export")
}
#[test]
fn file_readers_and_native_artifacts_are_available_through_the_facade() {
    let source = Arc::new(XmlConfigurationSource::open(native()).unwrap());
    let artifacts = source.artifacts().unwrap();
    let root = artifacts.root_identity().to_string();
    let text = artifacts
        .descriptions()
        .iter()
        .find(|d| d.name.as_ref() == "ТекстФайла")
        .unwrap();
    let artifact = artifacts.artifact(&text.id).unwrap().unwrap();
    let bound =
        BoundConfigurationSource::new(source.clone(), &root, "database-storage-namespace").unwrap();
    assert!(source.module_image("missing").unwrap().is_none());
    assert!(bound.module_image("missing").unwrap().is_none());
    assert_eq!(bound.index().identity(), "database-storage-namespace");
    assert_eq!(bound.artifacts().unwrap().root_identity(), root);
    assert!(BoundConfigurationSource::new(source.clone(), "another-root", "namespace").is_err());
    let engine = Engine::builder()
        .register_library(
            LibraryDescriptor::new("file-template-test", "1").with_functions(FUNCTIONS),
        )
        .build()
        .unwrap();
    let mut first = engine
        .state_builder()
        .host_data(Rc::new(artifact.clone()))
        .build();
    assert_eq!(first.exec("ДокументМакета = МакетФайла(); ДокументМакета.УстановитьТекст(\"Изменён\"); Возврат МакетФайла().ПолучитьТекст();").unwrap(), Value::Str("Контроль & <тег>\nВторая строка".into()));
    let mut second = engine.state_builder().host_data(Rc::new(artifact)).build();
    assert_eq!(
        second
            .exec("Возврат МакетФайла().ПолучитьТекст();")
            .unwrap(),
        Value::Str("Контроль & <тег>\nВторая строка".into())
    );
    let sheet = artifacts
        .descriptions()
        .iter()
        .find(|d| d.name.as_ref() == "ТаблицаФайла")
        .unwrap();
    let artifact = artifacts.artifact(&sheet.id).unwrap().unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(artifact)).build();
    assert_eq!(
        state
            .exec("Возврат ТипЗнч(МакетФайла()) = Тип(\"ТабличныйДокумент\");")
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn facade_edt_reader_preserves_xml_looking_text_templates_as_text() {
    let root = std::env::temp_dir().join(format!("open-bsl-facade-edt-{}", std::process::id()));
    let source_root = root.join("src");
    std::fs::create_dir_all(source_root.join("Configuration")).unwrap();
    let owner = source_root.join("Catalogs/ХранилищаКонфигураций");
    std::fs::create_dir_all(owner.join("Templates/ПустоеРасширение")).unwrap();
    std::fs::write(source_root.join("Configuration/Configuration.mdo"), r#"<md:Configuration xmlns:md="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a8dd2e89-c043-4f61-ab58-dafb034cd4c1"><name>Контроль</name><catalogs>Catalog.ХранилищаКонфигураций</catalogs></md:Configuration>"#).unwrap();
    // Сокращённое описание сохраняет нативные UUID и вид исходного макета.
    std::fs::write(owner.join("ХранилищаКонфигураций.mdo"), r#"<md:Catalog xmlns:md="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="58570c20-db1f-43f1-af51-47fbbb1a4807"><name>ХранилищаКонфигураций</name><templates uuid="b6f898af-182b-43d9-aac4-567daaf55704"><name>ПустоеРасширение</name><templateType>TextDocument</templateType></templates></md:Catalog>"#).unwrap();
    let original = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/fixtures/file-configuration-native-1022/edt-reference/GitConverter/src/Catalogs/ХранилищаКонфигураций/Templates/ПустоеРасширение/Template.txt");
    std::fs::copy(
        original,
        owner.join("Templates/ПустоеРасширение/Template.txt"),
    )
    .unwrap();
    let source = EdtConfigurationSource::open(&root).unwrap();
    let artifact = source
        .artifacts()
        .unwrap()
        .artifact("b6f898af-182b-43d9-aac4-567daaf55704")
        .unwrap()
        .unwrap();
    assert_eq!(
        artifact.description.owner.as_deref(),
        Some("58570c20-db1f-43f1-af51-47fbbb1a4807")
    );
    assert!(
        decode_configuration_text(&artifact.bytes)
            .unwrap()
            .starts_with("<?xml")
    );
    let engine = Engine::builder()
        .register_library(
            LibraryDescriptor::new("edt-template-test", "1").with_functions(FUNCTIONS),
        )
        .build()
        .unwrap();
    let mut state = engine.state_builder().host_data(Rc::new(artifact)).build();
    assert_eq!(
        state
            .exec("Возврат Лев(МакетФайла().ПолучитьТекст(), 5);")
            .unwrap(),
        Value::Str("<?xml".into())
    );
    std::fs::remove_dir_all(root).unwrap();
}
