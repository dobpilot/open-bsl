use super::*;
use std::path::PathBuf;

#[test]
fn edt_import_of_native_xml_preserves_modules_forms_and_templates() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/file-configuration-native-1022");
    let edt = EdtConfigurationSource::open(fixtures.join("edt-native")).unwrap();
    let xml = XmlConfigurationSource::open(fixtures.join("native-export")).unwrap();
    assert_eq!(edt.index().modules().len(), xml.index().modules().len());
    for module in xml.index().modules() {
        let other = edt
            .index()
            .modules()
            .iter()
            .find(|m| m.id() == module.id())
            .unwrap();
        assert_eq!(other.name(), module.name());
        assert_eq!(
            edt.module_source(other.id()).unwrap().replace("\r\n", "\n"),
            xml.module_source(module.id())
                .unwrap()
                .replace("\r\n", "\n")
        );
    }
    let edt = edt.artifacts().unwrap();
    let xml = xml.artifacts().unwrap();
    assert_eq!(edt.root_identity(), xml.root_identity());
    let identities = |source: &dyn bsl_config::ConfigurationArtifacts| {
        source
            .descriptions()
            .iter()
            .map(|d| (d.id.clone(), d.kind.clone(), d.name.clone()))
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(identities(edt), identities(xml));
    for id in [
        "94c69a28-4a6b-47a4-abcd-000000000001",
        "94c69a28-4a6b-47a4-abcd-000000000002",
        "94c69a28-4a6b-47a4-abcd-000000000003",
    ] {
        let left = edt.artifact(id).unwrap().unwrap();
        let right = xml.artifact(id).unwrap().unwrap();
        assert_eq!(left.representation, right.representation);
        assert_eq!(left.kind, right.kind);
        assert!(!left.bytes.is_empty());
    }
}

#[test]
fn native_buh_short_subsystems_and_missing_template_payload_remain_indexable() {
    // Фрагменты типовой конфигурации лежат в неотслеживаемом `private/`.
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../private/fixtures/file-configuration-native-1022/buh-edt-fragments");
    if !fixtures.is_dir() {
        eprintln!("пропуск: нет приватной фикстуры {}", fixtures.display());
        return;
    }
    let root =
        std::env::temp_dir().join(format!("open-bsl-edt-buh-fragments-{}", std::process::id()));
    fn copy_descriptions(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy_descriptions(&entry.path(), &target.join(entry.file_name()));
            } else if entry.path().extension().is_some_and(|e| e == "mdo") {
                std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
            }
        }
    }
    copy_descriptions(&fixtures, &root);
    std::fs::create_dir_all(root.join("Configuration")).unwrap();
    std::fs::write(root.join("Configuration/Configuration.mdo"), r#"<m:Configuration xmlns:m="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="e0666db2-45d6-49b4-a200-061c6ba7d569"><name>Контроль</name><subsystems>Subsystem.АдминистрированиеСервиса</subsystems><catalogs>Catalog.МашиночитаемыеДоверенности</catalogs></m:Configuration>"#).unwrap();
    let result = (|| {
        let source = EdtConfigurationSource::open(&root)?;
        let artifacts = source.artifacts().unwrap();
        assert!(
            artifacts
                .descriptions()
                .iter()
                .any(|d| d.name.as_ref() == "ОбластиДанных" && d.kind.as_ref() == "Subsystem")
        );
        let id = "e7d74349-6636-485f-888d-57086beb7574";
        assert!(artifacts.description(id)?.is_some());
        let error = artifacts.artifact(id).unwrap_err();
        assert!(error.contains(id) && error.contains("нет содержимого ресурса"));
        Ok::<_, String>(())
    })();
    std::fs::remove_dir_all(root).unwrap();
    result.unwrap();
}

#[test]
fn missing_native_common_module_body_is_an_acquisition_error() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/file-configuration-native-1022/demo-edt-fragments");
    let root =
        std::env::temp_dir().join(format!("open-bsl-edt-missing-body-{}", std::process::id()));
    let relative =
        "CommonModules/УчетСамочувствияСотрудниковКлиент/УчетСамочувствияСотрудниковКлиент.mdo";
    std::fs::create_dir_all(root.join(relative).parent().unwrap()).unwrap();
    std::fs::copy(fixture.join(relative), root.join(relative)).unwrap();
    std::fs::create_dir_all(root.join("Configuration")).unwrap();
    std::fs::write(root.join("Configuration/Configuration.mdo"), r#"<m:Configuration xmlns:m="http://g5.1c.ru/v8/dt/metadata/mdclass" uuid="a3f5e643-b9c4-4763-9c5d-91fc9be92710"><name>Контроль</name><commonModules>CommonModule.УчетСамочувствияСотрудниковКлиент</commonModules></m:Configuration>"#).unwrap();
    let result = (|| {
        let source = EdtConfigurationSource::open(&root)?;
        let module = source.index().modules().first().unwrap();
        let error = source.module_source(module.id()).unwrap_err();
        assert!(error.contains("нет BSL-текста модуля"));
        Ok::<_, String>(())
    })();
    std::fs::remove_dir_all(root).unwrap();
    result.unwrap();
}
