use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Files {
    index: ConfigurationIndex,
    descriptions: Vec<ConfigurationDescription>,
    text: Arc<str>,
    reads: AtomicUsize,
}
impl ConfigurationSource for Files {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        Some(self)
    }
    fn module_source(&self, _: &str) -> Result<Arc<str>, String> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::clone(&self.text))
    }
}
impl ConfigurationArtifacts for Files {
    fn root_identity(&self) -> &str {
        self.index.identity()
    }
    fn descriptions(&self) -> &[ConfigurationDescription] {
        &self.descriptions
    }
    fn description(&self, id: &str) -> Result<Option<Arc<ConfigurationProperty>>, String> {
        Ok(self
            .descriptions
            .iter()
            .find(|d| d.id.as_ref() == id)
            .map(|d| {
                Arc::new(ConfigurationProperty {
                    name: "description".into(),
                    value: "".into(),
                    attributes: Arc::from([]),
                    children: vec![ConfigurationProperty {
                        name: "name".into(),
                        value: d.name.clone(),
                        attributes: Arc::from([]),
                        children: Arc::from([]),
                    }]
                    .into(),
                })
            }))
    }
    fn artifact(&self, _: &str) -> Result<Option<Arc<ConfigurationArtifact>>, String> {
        Ok(None)
    }
}
fn files(root: &str, owner: &str, text: &str) -> Arc<Files> {
    Arc::new(Files {
        index: ConfigurationIndex::new(
            root,
            vec![ConfigurationModuleDeclaration::new(
                format!("{owner}/Common"),
                "Общий",
                ModuleKind::Common,
                ModuleExecution::new().with_server(true),
            )],
        )
        .unwrap(),
        descriptions: vec![
            ConfigurationDescription {
                id: root.into(),
                owner: None,
                kind: "Configuration".into(),
                name: "Конфигурация".into(),
            },
            ConfigurationDescription {
                id: owner.into(),
                owner: None,
                kind: "CommonModule".into(),
                name: "Общий".into(),
            },
        ],
        text: text.into(),
        reads: AtomicUsize::new(0),
    })
}
#[test]
fn file_extensions_replace_one_effective_body_without_eager_body_reads() {
    let base = files("root", "base", "ИсходныйТекст");
    let first = files("extension-root-1", "local-1", "ПервыйТекст");
    let second = files("extension-root-2", "local-2", "ВторойТекст");
    let mut bundle = FileBundle::new(base.clone(), LoadOptions::default()).unwrap();
    let info = parse_extension_info(&[0; 24]).unwrap();
    bundle
        .append(
            first.clone(),
            &[("local-1".into(), "base".into())].into(),
            &info,
        )
        .unwrap();
    bundle
        .append(
            second.clone(),
            &[("local-2".into(), "base".into())].into(),
            &info,
        )
        .unwrap();
    assert_eq!(bundle.index().modules().len(), 1);
    assert_eq!(base.reads.load(Ordering::Relaxed), 0);
    assert_eq!(first.reads.load(Ordering::Relaxed), 0);
    assert_eq!(second.reads.load(Ordering::Relaxed), 0);
    assert_eq!(
        bundle.module_source("base/Common").unwrap().as_ref(),
        "ВторойТекст"
    );
    assert_eq!(second.reads.load(Ordering::Relaxed), 1);
    assert!(bundle.index().modules()[0].source_is_extension());
    assert_eq!(
        bundle
            .descriptions()
            .iter()
            .filter(|d| d.kind.as_ref() == "CommonModule")
            .count(),
        1
    );
}
#[test]
fn unmapped_extension_owner_and_structural_merge_fail_explicitly() {
    let mut bundle = FileBundle::new(
        files("root", "base", "ИсходныйТекст"),
        LoadOptions::default(),
    )
    .unwrap();
    assert!(
        bundle
            .append(
                files("extension", "unknown", "Текст"),
                &HashMap::new(),
                &parse_extension_info(&[0; 24]).unwrap()
            )
            .is_err()
    );
    let base = ConfigurationProperty {
        name: "description".into(),
        value: "".into(),
        attributes: Arc::from([]),
        children: Arc::from([]),
    };
    let mut overlay = base.clone();
    let mut fields = base.clone();
    fields.name = "attributes".into();
    fields.children = vec![base.clone()].into();
    overlay.children = vec![fields].into();
    assert!(merge(&base, &overlay).is_err());
}

struct HeadersOnly {
    bootstrap: ConfigurationBootstrap,
    resources: HashMap<String, ConfigResource>,
    reads: Arc<Mutex<Vec<String>>>,
}
impl ResourceReader for HeadersOnly {
    fn bootstrap(&mut self) -> Result<ConfigurationBootstrap, String> {
        Ok(self.bootstrap.clone())
    }
    fn names(&mut self) -> Result<Vec<String>, String> {
        Ok(self
            .resources
            .keys()
            .cloned()
            .chain([
                "44444444-4444-4444-4444-444444444444.7".into(),
                "22222222-2222-2222-2222-222222222222.0".into(),
            ])
            .collect())
    }
    fn base(&mut self, names: &[&str]) -> Result<Vec<ConfigResource>, String> {
        names
            .iter()
            .map(|name| {
                self.reads.lock().unwrap().push((*name).into());
                self.resources
                    .get(*name)
                    .cloned()
                    .ok_or_else(|| format!("prohibited body read: {name}"))
            })
            .collect()
    }
    fn extension(&mut self, keys: &[ContentKey]) -> Result<Vec<ConfigResource>, String> {
        keys.iter()
            .map(|key| {
                let name = key.as_hex();
                self.reads.lock().unwrap().push(name.clone());
                self.resources
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("prohibited extension body read: {name}"))
            })
            .collect()
    }
}
fn descriptor(name: &str, value: &str) -> ConfigResource {
    let n = u16::try_from(value.len()).unwrap();
    let mut bytes = vec![1];
    bytes.extend(n.to_le_bytes());
    bytes.extend((!n).to_le_bytes());
    bytes.extend(value.as_bytes());
    ConfigResource {
        file_name: name.into(),
        compressed: bytes.into(),
    }
}
#[test]
fn orm_acquisition_does_not_request_database_bsl_bodies() {
    let root = descriptor("root", "{2,11111111-1111-1111-1111-111111111111,}");
    let root_descriptor = descriptor(
        "11111111-1111-1111-1111-111111111111",
        r#"{2,{11111111-1111-1111-1111-111111111111},1,{9cd510cd-abfc-11d4-9434-004095e12fc7,{1,{68,{0,{3,{1,0,44444444-4444-4444-4444-444444444444},"Конфигурация"}}},1,{0fe48980-252d-11d6-a3c7-0050bae0a776,1,22222222-2222-2222-2222-222222222222}}}}"#,
    );
    let common = descriptor(
        "22222222-2222-2222-2222-222222222222",
        r#"{1,{12,{2,{1,0,22222222-2222-2222-2222-222222222222},"Общий",{1,"ru","Общий"},"",0,0,33333333-3333-3333-3333-333333333333},1,0,1,0,0,1,0,0},0}"#,
    );
    let reads = Arc::new(Mutex::new(Vec::new()));
    let reader = HeadersOnly {
        bootstrap: ConfigurationBootstrap {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            root,
            descriptor: root_descriptor,
            extensions: vec![],
        },
        resources: [(common.file_name.clone(), common)].into(),
        reads: reads.clone(),
    };
    let source = DatabaseConfigurationSource::from_reader_mode(
        Box::new(reader),
        LoadOptions::default(),
        false,
    )
    .unwrap();
    assert_eq!(source.index().modules().len(), 2);
    assert_eq!(
        *reads.lock().unwrap(),
        ["22222222-2222-2222-2222-222222222222"]
    );
    let selected = source
        .bind_file_sources(files("root", "base", "ИзФайла"), vec![])
        .unwrap();
    assert_eq!(
        selected.module_source("base/Common").unwrap().as_ref(),
        "ИзФайла"
    );
    assert_eq!(
        *reads.lock().unwrap(),
        ["22222222-2222-2222-2222-222222222222"]
    );
    assert!(
        source
            .bind_file_sources(
                files("root", "base", "ИзФайла"),
                vec![("unknown".into(), files("extension", "local", "Расширение"))]
            )
            .is_err()
    );
}

const EXTENSION_ROOT: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const EXTENSION_ID: &str = "222d62b8-bcb8-11f1-87bb-02d78b8d8e99";
fn binding_source(active: bool) -> DatabaseConfigurationSource {
    let mut info = vec![0x43, 0xc2, 0x9a, 0x14];
    info.extend([0; 20]);
    info.extend([0x81, if active { 0x82 } else { 0x81 }, 0x82, 0x20]);
    assert_eq!(parse_extension_info(&info).unwrap().active, Some(active));
    let key0 = parse_extension_info(&[0; 24]).unwrap().root_key.as_hex();
    let mut key1_info = [0; 24];
    key1_info[23] = 1;
    let key1 = parse_extension_info(&key1_info).unwrap().root_key.as_hex();
    let root = descriptor(
        &key0,
        &format!(
            "{{2,{EXTENSION_ROOT},\"\"}}\n{{1,\"{EXTENSION_ROOT}\",AAAAAAAAAAAAAAAAAAAAAAAAAAE=}}"
        ),
    );
    let config = descriptor(
        &key1,
        &format!("{{2,{{{EXTENSION_ROOT}}},1,{{{CONFIGURATION},{{1,0,0}}}}}}"),
    );
    let row = open_sdbl_db::ExtensionCatalogRow {
        identity: open_bsl::uuid::to_1c_bytes(open_bsl::uuid::parse(EXTENSION_ID).unwrap())
            .to_vec(),
        order: 0,
        name: "Расширение".into(),
        info,
        scope: None,
        used_in_distributed_infobase: None,
    };
    let reader = HeadersOnly {
        bootstrap: ConfigurationBootstrap {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            root: descriptor("root", ""),
            descriptor: descriptor("descriptor", ""),
            extensions: vec![row],
        },
        resources: [(key0, root), (key1, config)].into(),
        reads: Arc::default(),
    };
    DatabaseConfigurationSource {
        index: ConfigurationIndex::new("root", vec![]).unwrap(),
        connection: Mutex::new(Box::new(reader)),
        locations: HashMap::new(),
        options: LoadOptions::default(),
        roots: [descriptor("root", ""), descriptor("descriptor", "")],
        headers: vec![],
        extensions: vec![],
        resource_names: HashSet::new(),
        owners: HashMap::new(),
        owner_modules: HashMap::new(),
        extension_count: 1,
        metadata: Mutex::new(None),
    }
}
fn empty_extension(root: &str) -> Arc<Files> {
    let mut source = files(root, "unused", "Текст не должен читаться");
    let data = Arc::get_mut(&mut source).unwrap();
    data.index = ConfigurationIndex::new(root, vec![]).unwrap();
    data.descriptions
        .retain(|d| d.kind.as_ref() == "Configuration");
    source
}
#[test]
fn database_extension_binding_checks_missing_mismatched_and_canonical_identities() {
    let source = binding_source(true);
    let error = source
        .bind_file_sources(files("root", "base", "ИзФайла"), vec![])
        .err()
        .unwrap();
    assert!(error.contains(EXTENSION_ID));
    let error = source
        .bind_file_sources(
            files("root", "base", "ИзФайла"),
            vec![(EXTENSION_ID.into(), empty_extension("other-root"))],
        )
        .err()
        .unwrap();
    assert!(error.contains("UUID файлового расширения"), "{error}");
    let source = binding_source(true)
        .bind_file_sources(
            files("root", "base", "ИзФайла"),
            vec![(EXTENSION_ID.into(), empty_extension(EXTENSION_ROOT))],
        )
        .unwrap();
    assert_eq!(
        source.module_source("base/Common").unwrap().as_ref(),
        "ИзФайла"
    );
}
#[test]
fn inactive_extension_content_is_not_applied_and_unknown_bindings_fail() {
    let extension = empty_extension("inactive-root-is-not-applied");
    let source = binding_source(false)
        .bind_file_sources(
            files("root", "base", "ИзФайла"),
            vec![(EXTENSION_ID.into(), extension.clone())],
        )
        .unwrap();
    assert_eq!(source.index().modules().len(), 1);
    assert_eq!(extension.reads.load(Ordering::Relaxed), 0);
    assert!(
        binding_source(false)
            .bind_file_sources(
                files("root", "base", "ИзФайла"),
                vec![("unknown".into(), extension)]
            )
            .is_err()
    );
}

#[test]
fn an_adopted_common_module_without_a_local_body_updates_flags_and_keeps_the_selected_base_body() {
    let base = files("root", "base", "Основное файловое тело");
    let mut extension = files("extension", "local", "Это тело не выбрано");
    Arc::get_mut(&mut extension).unwrap().index =
        ConfigurationIndex::new("extension", vec![]).unwrap();
    let mut bundle = FileBundle::new(base.clone(), LoadOptions::default()).unwrap();
    bundle
        .append(
            extension.clone(),
            &[("local".into(), "base".into())].into(),
            &parse_extension_info(&[0; 24]).unwrap(),
        )
        .unwrap();
    assert!(!bundle.index().modules()[0].execution().server());
    assert!(!bundle.index().modules()[0].source_is_extension());
    assert_eq!(
        bundle.module_source("base/Common").unwrap().as_ref(),
        "Основное файловое тело"
    );
    assert_eq!(base.reads.load(Ordering::Relaxed), 1);
    assert_eq!(extension.reads.load(Ordering::Relaxed), 0);
}

#[test]
fn extension_composition_preserves_the_verified_storage_namespace() {
    let source = Arc::new(
        bsl_config::BoundConfigurationSource::new(
            files("native-root", "base", "Базовое тело"),
            "native-root",
            "database-namespace",
        )
        .unwrap(),
    );
    let mut bundle = FileBundle::new(source, LoadOptions::default()).unwrap();
    bundle
        .append(
            files("extension", "local", "Тело расширения"),
            &[("local".into(), "base".into())].into(),
            &parse_extension_info(&[0; 24]).unwrap(),
        )
        .unwrap();
    assert_eq!(bundle.root_identity(), "native-root");
    assert_eq!(bundle.index().identity(), "database-namespace");
    assert_eq!(
        bundle.module_source("base/Common").unwrap().as_ref(),
        "Тело расширения"
    );
}

struct BinaryFiles {
    files: Arc<Files>,
    bytes: Arc<[u8]>,
    image_reads: AtomicUsize,
}
impl ConfigurationSource for BinaryFiles {
    fn index(&self) -> &ConfigurationIndex {
        self.files.index()
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        Some(self.files.as_ref())
    }
    fn module_source(&self, _: &str) -> Result<Arc<str>, String> {
        panic!("binary file must not fall back to another source")
    }
    fn module_image(&self, _: &str) -> Result<Option<Arc<[u8]>>, String> {
        self.image_reads.fetch_add(1, Ordering::Relaxed);
        Ok(Some(Arc::clone(&self.bytes)))
    }
}
#[test]
fn selected_file_image_reuses_native_policy_and_caches_its_result() {
    let image = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
    );
    let raw = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.image"
    );
    for recover in [false, true] {
        let source = Arc::new(BinaryFiles {
            files: files("root", "owner", ""),
            bytes: Arc::from(image.as_slice()),
            image_reads: AtomicUsize::new(0),
        });
        let bundle = FileBundle::new(
            source.clone(),
            LoadOptions {
                decompile_protected_modules: recover,
            },
        )
        .unwrap();
        let expected = if recover {
            native_image::decompile(raw).unwrap()
        } else {
            native_image::exported_interface(raw).unwrap()
        };
        let first = bundle.module_source("owner/Common").unwrap();
        assert_eq!(first.as_ref(), expected);
        assert!(Arc::ptr_eq(
            &first,
            &bundle.module_source("owner/Common").unwrap()
        ));
        assert_eq!(source.image_reads.load(Ordering::Relaxed), 1);
        assert!(bundle.description("owner").unwrap().is_some());
    }
}
#[test]
fn malformed_selected_file_image_never_becomes_a_successful_empty_module() {
    let source = Arc::new(BinaryFiles {
        files: files("root", "owner", ""),
        bytes: Arc::from([0xff, 0xff, 0xff, 0x7f]),
        image_reads: AtomicUsize::new(0),
    });
    let bundle = FileBundle::new(source, LoadOptions::default()).unwrap();
    assert!(
        bundle
            .module_source("owner/Common")
            .unwrap_err()
            .contains("owner/Common")
    );
}

#[test]
fn explicit_file_recovery_keeps_declarations_separate_from_instruction_bodies() {
    let image = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
    );
    let raw = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.image"
    );
    let source = Arc::new(BinaryFiles {
        files: files("root", "owner", ""),
        bytes: Arc::from(image.as_slice()),
        image_reads: AtomicUsize::new(0),
    });
    let bundle = FileBundle::new(
        source,
        LoadOptions {
            decompile_protected_modules: true,
        },
    )
    .unwrap();
    let bsl_config::ModuleDeclarationSource::Declarations(declarations) =
        bundle.module_declarations("owner/Common").unwrap()
    else {
        panic!("expected declarations without recovered instructions")
    };
    assert_eq!(
        declarations.as_ref(),
        native_image::declarations(raw).unwrap()
    );
    assert_ne!(declarations, bundle.module_source("owner/Common").unwrap());
}
