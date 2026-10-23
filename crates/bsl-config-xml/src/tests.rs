use super::*;
use bsl_config::{ConfigurationArtifacts, ConfigurationSource};
use std::path::PathBuf;

fn native() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/research/fixtures/configuration-default-interface-native-1016/empty/native-xml",
    )
}

#[test]
fn native_index_and_module_text_come_from_the_export() {
    let source = XmlConfigurationSource::open(native()).unwrap();
    assert_eq!(
        source.root_identity(),
        "26c30ece-33b5-498a-b409-a91a046ba66b"
    );
    let module = source
        .index()
        .modules()
        .iter()
        .find(|m| m.name() == "ОчередьЗаданийСлужебный")
        .unwrap();
    assert!(module.execution().server());
    assert_eq!(
        source.module_source(module.id()).unwrap().as_ref(),
        std::fs::read_to_string(
            native().join("CommonModules/ОчередьЗаданийСлужебный/Ext/Module.bsl")
        )
        .unwrap()
        .trim_start_matches('\u{feff}')
    );
    assert!(
        source
            .descriptions()
            .iter()
            .any(|d| d.kind.as_ref() == "Document")
    );
}

#[test]
fn unknown_resources_are_not_fabricated() {
    let source = XmlConfigurationSource::open(native()).unwrap();
    assert!(source.module_source("unknown").is_err());
    assert!(source.description("unknown").unwrap().is_none());
    assert!(source.artifact("unknown").unwrap().is_none());
}

#[test]
fn native_templates_and_nested_form_attributes_are_not_discarded() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/research/fixtures/file-configuration-native-1022/native-export");
    let source = XmlConfigurationSource::open(path).unwrap();
    let text = source
        .descriptions()
        .iter()
        .find(|d| d.name.as_ref() == "ТекстФайла")
        .unwrap();
    let artifact = source.artifact(&text.id).unwrap().unwrap();
    assert_eq!(artifact.representation.as_ref(), "TextDocument");
    assert!(
        resource::decode(&artifact.bytes)
            .unwrap()
            .contains("Контроль & <тег>")
    );
    assert!(Arc::ptr_eq(
        &artifact,
        &source.artifact(&text.id).unwrap().unwrap()
    ));
    let form = source
        .descriptions()
        .iter()
        .find(|d| d.kind.as_ref() == "Form")
        .unwrap();
    let description = source.description(&form.id).unwrap().unwrap();
    assert!(description.child("attributes").is_some());
    assert!(
        source
            .index()
            .modules()
            .iter()
            .any(|m| m.kind() == ModuleKind::Form)
    );
}

#[test]
fn xml_attribute_properties_are_normalized_like_edt_properties() {
    let node = tree::parse(r#"<Attribute uuid="field"><Properties><Name>Реквизит</Name><Type><Type>xs:string</Type><StringQualifiers><Length>10</Length></StringQualifiers></Type></Properties></Attribute>"#).unwrap();
    let property = node.property();
    assert_eq!(property.child("name").unwrap().value.as_ref(), "Реквизит");
    assert_eq!(property.child("uuid").unwrap().value.as_ref(), "field");
    assert!(
        property
            .child("type")
            .unwrap()
            .child("stringQualifiers")
            .is_some()
    );
}

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "open-bsl-source-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn resources_check_generation_and_cache_the_original_immutable_text() {
    let temporary = Temp::new();
    let path = temporary.0.join("Модуль.bsl");
    std::fs::write(&path, "Возврат 1;").unwrap();
    let cached = Resource::new(&temporary.0, &path).unwrap();
    let unread = Resource::new(&temporary.0, &path).unwrap();
    let text = cached.module_text().unwrap();
    std::fs::write(&path, "Возврат 200;").unwrap();
    assert!(
        unread
            .module_text()
            .unwrap_err()
            .contains("источник изменён")
    );
    assert!(Arc::ptr_eq(&text, &cached.module_text().unwrap()));
    assert_eq!(text.as_ref(), "Возврат 1;");
}
#[test]
fn supported_text_encodings_preserve_unicode_and_reject_invalid_sequences() {
    for little in [true, false] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for unit in "Привет😀".encode_utf16() {
            bytes.extend(if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        assert_eq!(resource::decode(&bytes).unwrap().as_ref(), "Привет😀");
    }
    assert_eq!(
        resource::decode(b"\xef\xbb\xbftext").unwrap().as_ref(),
        "text"
    );
    assert!(resource::decode(&[0xff, 0xfe, 0x00]).is_err());
    assert!(resource::decode(&[0xff, 0xfe, 0x00, 0xd8]).is_err());
    assert!(resource::decode(&[0xff]).is_err());
}
#[test]
fn form_payload_is_not_parsed_during_indexing_and_binary_code_is_not_synthesized() {
    let temporary = Temp::new();
    let ns = "http://v8.1c.ru/8.3/MDClasses";
    std::fs::write(temporary.0.join("Configuration.xml"), format!(r#"<MetaDataObject xmlns="{ns}" version="2.20"><Configuration uuid="11111111-1111-1111-1111-111111111111"><Properties><Name>Проба</Name></Properties><ChildObjects><CommonForm>Форма</CommonForm></ChildObjects></Configuration></MetaDataObject>"#)).unwrap();
    std::fs::create_dir_all(temporary.0.join("CommonForms/Форма/Ext")).unwrap();
    std::fs::write(temporary.0.join("CommonForms/Форма.xml"), format!(r#"<MetaDataObject xmlns="{ns}" version="2.20"><CommonForm uuid="22222222-2222-2222-2222-222222222222"><Properties><Name>Форма</Name></Properties></CommonForm></MetaDataObject>"#)).unwrap();
    std::fs::write(
        temporary.0.join("CommonForms/Форма/Ext/Form.xml"),
        "<malformed",
    )
    .unwrap();
    let source = XmlConfigurationSource::open(&temporary.0).unwrap();
    assert!(
        source
            .description("22222222-2222-2222-2222-222222222222")
            .is_err()
    );
    std::fs::create_dir_all(temporary.0.join("Ext")).unwrap();
    std::fs::write(temporary.0.join("Ext/SessionModule.bin"), [1, 2, 3]).unwrap();
    let source = XmlConfigurationSource::open(&temporary.0).unwrap();
    let module = source
        .index()
        .modules()
        .iter()
        .find(|m| m.kind() == ModuleKind::Session)
        .unwrap();
    assert!(
        source
            .module_source(module.id())
            .unwrap_err()
            .contains("нет BSL-текста")
    );
    let image = source.module_image(module.id()).unwrap().unwrap();
    assert_eq!(image.as_ref(), [1, 2, 3]);
    assert!(Arc::ptr_eq(
        &image,
        &source.module_image(module.id()).unwrap().unwrap()
    ));
}

#[test]
fn native_form_main_attributes_and_bindings_survive_neutral_projection() {
    // Форма типовой конфигурации лежит в неотслеживаемом `private/`.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../private/fixtures/file-configuration-native-1022/buh-form/Form.xml");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("пропуск: нет приватной фикстуры {}", path.display());
        return;
    };
    let node = tree::parse(&resource::decode(&bytes).unwrap()).unwrap();
    let properties = node.description();
    let attributes = properties.child("attributes").unwrap();
    let main = attributes
        .children
        .iter()
        .find(|p| {
            p.child("name")
                .is_some_and(|n| n.value.as_ref() == "Объект")
        })
        .unwrap();
    assert_eq!(main.child("main").unwrap().value.as_ref(), "true");
    assert!(main.child("type").is_some());
    let language = attributes
        .children
        .iter()
        .find(|p| p.child("name").is_some_and(|n| n.value.as_ref() == "Язык"))
        .unwrap();
    assert_eq!(
        language
            .child("type")
            .unwrap()
            .child("stringQualifiers")
            .unwrap()
            .child("allowedLength")
            .unwrap()
            .value
            .as_ref(),
        "Variable"
    );
    assert!(properties.child("events").is_some());
    assert!(properties.child("childItems").is_some());
}

#[test]
fn edt_qualifier_names_match_native_xml_qualifier_names() {
    let edt = tree::parse("<numberQualifiers><precision>15</precision><scale>2</scale><nonNegative>true</nonNegative></numberQualifiers>").unwrap().property();
    let xml = tree::parse("<NumberQualifiers><Digits>15</Digits><FractionDigits>2</FractionDigits><AllowedSign>Nonnegative</AllowedSign></NumberQualifiers>").unwrap().property();
    assert_eq!(edt, xml);
    let edt =
        tree::parse("<stringQualifiers><length>10</length><fixed>true</fixed></stringQualifiers>")
            .unwrap()
            .property();
    let xml = tree::parse("<StringQualifiers><Length>10</Length><AllowedLength>Fixed</AllowedLength></StringQualifiers>").unwrap().property();
    assert_eq!(edt, xml);
}

#[test]
fn native_uuid_spelling_is_canonicalized_without_changing_source_names() {
    let node = tree::parse(r#"<Attribute uuid="ABCDEFAB-1234-5678-9012-ABCDEFABCDEF"><Properties><Name>ИмяРеквизита</Name></Properties></Attribute>"#).unwrap().property();
    assert_eq!(
        node.child("uuid").unwrap().value.as_ref(),
        "abcdefab-1234-5678-9012-abcdefabcdef"
    );
    assert_eq!(node.child("name").unwrap().value.as_ref(), "ИмяРеквизита");
}

#[test]
fn native_image_in_a_bsl_path_is_exposed_as_bytes_without_text_synthesis() {
    let temporary = Temp::new();
    std::fs::copy(
        native().join("Configuration.xml"),
        temporary.0.join("Configuration.xml"),
    )
    .unwrap();
    // Для этого контроля индекс исходной конфигурации не нужен: ресурс
    // проверяется отдельно, с тем же ограниченным чтением модуля.
    let path = temporary.0.join("Module.bsl");
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
    );
    std::fs::write(&path, bytes).unwrap();
    let root = temporary.0.canonicalize().unwrap();
    let resource = resource::Resource::new(&root, &path).unwrap();
    let image = resource.module_image().unwrap().unwrap();
    assert_eq!(image.as_ref(), bytes);
    assert!(Arc::ptr_eq(
        &image,
        &resource.module_image().unwrap().unwrap()
    ));
    assert!(
        resource
            .module_text()
            .unwrap_err()
            .contains("нет BSL-текста")
    );
}
