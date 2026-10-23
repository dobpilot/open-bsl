use super::*;

const DESCRIPTOR: &str = "11111111-1111-1111-1111-111111111111";
const COMMON_ID: &str = "22222222-2222-2222-2222-222222222222";
const SESSION_ID: &str = "44444444-4444-4444-4444-444444444444.7";

struct Reader {
    bootstrap: ConfigurationBootstrap,
    resources: HashMap<String, ConfigResource>,
    reads: Arc<Mutex<Vec<String>>>,
    failed_reads: HashSet<String>,
}

impl ResourceReader for Reader {
    fn metadata_inputs(&mut self) -> Result<open_sdbl_db::ConfigurationMetadataInputs, String> {
        self.reads.lock().unwrap().push("physical index".into());
        Ok(open_sdbl_db::ConfigurationMetadataInputs {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            db_names: open_sdbl::metadata::DbNames::from_entries(vec![]),
            schema: open_sdbl::metadata::SchemaStorage {
                tables: vec![],
                anomalies: vec![],
            },
            live_tables: vec![],
        })
    }
    fn extension_metadata(
        &mut self,
    ) -> Result<Vec<open_sdbl::metadata::ExtensionMetadata>, String> {
        self.reads
            .lock()
            .unwrap()
            .push("extension structures".into());
        Ok(vec![])
    }
    fn bootstrap(&mut self) -> Result<ConfigurationBootstrap, String> {
        Ok(self.bootstrap.clone())
    }
    fn names(&mut self) -> Result<Vec<String>, String> {
        Ok(self.resources.keys().cloned().collect())
    }
    fn base(&mut self, names: &[&str]) -> Result<Vec<ConfigResource>, String> {
        names
            .iter()
            .map(|name| {
                self.reads.lock().unwrap().push((*name).into());
                if self.failed_reads.contains(*name) {
                    return Err(format!("отказ чтения {name}"));
                }
                self.resources
                    .get(*name)
                    .cloned()
                    .ok_or_else(|| format!("лишнее чтение {name}"))
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
                    .ok_or_else(|| format!("лишнее чтение CAS {name}"))
            })
            .collect()
    }
}

fn text(name: &str, value: &str) -> ConfigResource {
    // Несжатый DEFLATE-блок тестовой обвязки, как у прежних тестов загрузчика.
    let length = u16::try_from(value.len()).unwrap();
    let mut compressed = vec![1];
    compressed.extend(length.to_le_bytes());
    compressed.extend((!length).to_le_bytes());
    compressed.extend(value.as_bytes());
    ConfigResource {
        file_name: name.into(),
        compressed: compressed.into(),
    }
}

fn reader() -> Reader {
    let root = text("root", &format!("{{2,{DESCRIPTOR},}}"));
    // Та же форма корня и общего модуля, что в существующем base_resources...
    let descriptor = text(
        DESCRIPTOR,
        r#"{2,{11111111-1111-1111-1111-111111111111},1,{9cd510cd-abfc-11d4-9434-004095e12fc7,{1,{68,{0,{3,{1,0,44444444-4444-4444-4444-444444444444},"Конфигурация"}}},1,{0fe48980-252d-11d6-a3c7-0050bae0a776,1,22222222-2222-2222-2222-222222222222}}}}"#,
    );
    let common = text(
        COMMON_ID,
        r#"{1,{12,{2,{1,0,22222222-2222-2222-2222-222222222222},"Общий",{1,"ru","Общий"},"",0,0,33333333-3333-3333-3333-333333333333},1,0,1,0,0,1,0,0},0}"#,
    );
    Reader {
        bootstrap: ConfigurationBootstrap {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            root,
            descriptor,
            extensions: vec![],
        },
        resources: [
            common,
            text(SESSION_ID, "Процедура Старт() КонецПроцедуры"),
            text(
                &format!("{COMMON_ID}.0"),
                "Функция Тест() Экспорт Возврат 17; КонецФункции",
            ),
        ]
        .into_iter()
        .map(|resource| (resource.file_name.clone(), resource))
        .collect(),
        reads: Arc::default(),
        failed_reads: HashSet::new(),
    }
}

#[test]
fn discovery_reads_headers_but_defers_all_module_resources() {
    let reader = reader();
    let reads = reader.reads.clone();
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    assert_eq!(source.index().modules().len(), 2);
    assert_eq!(*reads.lock().unwrap(), [COMMON_ID]);
    let body = source.module_source(SESSION_ID).unwrap();
    assert_eq!(body.as_ref(), "Процедура Старт() КонецПроцедуры");
    assert_eq!(*reads.lock().unwrap(), [COMMON_ID, SESSION_ID]);
    let common = source.module_source(&format!("{COMMON_ID}.0")).unwrap();
    assert!(common.contains("Возврат 17"));
    assert_eq!(
        reads.lock().unwrap().last().unwrap(),
        &format!("{COMMON_ID}.0")
    );
}

#[test]
fn absent_common_body_keeps_the_existing_empty_module_behavior_without_a_read() {
    let mut reader = reader();
    reader.resources.remove(&format!("{COMMON_ID}.0"));
    let reads = reader.reads.clone();
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    assert_eq!(
        source
            .module_source(&format!("{COMMON_ID}.0"))
            .unwrap()
            .as_ref(),
        ""
    );
    assert_eq!(*reads.lock().unwrap(), [COMMON_ID]);
}

#[test]
fn extension_discovery_does_not_read_module_bytes_and_keeps_the_adopted_target() {
    let mut reader = reader();
    let reads = reader.reads.clone();
    let configuration = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let local = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let target = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    let class = "cf4abea6-37b2-11d4-940f-008048da11f9";
    let key = |last: u8| {
        let mut info = [0; 24];
        info[23] = last;
        parse_extension_info(&info).unwrap().root_key
    };
    let root = format!(
        "{{2,{configuration},\"\"}}\n{{3,\"{configuration}\",AAAAAAAAAAAAAAAAAAAAAAAAAAE=,\"{local}\",AAAAAAAAAAAAAAAAAAAAAAAAAAI=,\"{local}.0\",AAAAAAAAAAAAAAAAAAAAAAAAAAM=}}"
    );
    let descriptor =
        format!("{{2,{{{configuration}}},1,{{{CONFIGURATION},{{1,0,1,{{{class},1,{local}}}}}}}}}");
    let object =
        format!("{{3,{{1,0,{local}}},\"Базовый\",{{1,\"ru\",\"Базовый\"}},\"\",1,0,{target},0}}");
    for (last, value) in [
        (0, root.as_str()),
        (1, descriptor.as_str()),
        (2, object.as_str()),
        (3, "Функция Тест() Экспорт Возврат 23; КонецФункции"),
    ] {
        let name = key(last).as_hex();
        reader.resources.insert(name.clone(), text(&name, value));
    }
    let info = parse_extension_info(&[0; 24]).unwrap();
    let mut reader: Box<dyn ResourceReader> = Box::new(reader);
    let mut discovery = Discovery {
        connection: reader.as_mut(),
        options: LoadOptions::default(),
        headers: vec![],
        extensions: vec![],
        modules: vec![],
        locations: HashMap::new(),
        objects: [(
            target.into(),
            BaseObject {
                class: class.into(),
                name: "Базовый".into(),
            },
        )]
        .into(),
    };
    discovery
        .extension(
            &open_sdbl_db::ExtensionCatalogRow {
                identity: vec![0; 16],
                order: 0,
                name: "Расширение".into(),
                info: vec![],
                scope: None,
                used_in_distributed_infobase: None,
            },
            &info,
            true,
        )
        .unwrap();
    assert_eq!(
        *reads.lock().unwrap(),
        [key(0).as_hex(), key(1).as_hex(), key(2).as_hex()]
    );
    assert_eq!(discovery.modules.len(), 1);
    let module = &discovery.modules[0];
    assert_eq!(module.id(), format!("{target}.0"));
    assert_eq!(module.owner(), Some(target));
    assert!(module.source_is_extension());
    let locations = discovery.locations;
    let index = ConfigurationIndex::new("test", discovery.modules).unwrap();
    let headers = discovery.headers;
    let extensions = discovery.extensions;
    let source = DatabaseConfigurationSource {
        index,
        locations,
        connection: Mutex::new(reader),
        options: LoadOptions::default(),
        roots: [text("root", ""), text(DESCRIPTOR, "")],
        headers,
        extensions,
        extension_count: 0,
        resource_names: HashSet::new(),
        owners: HashMap::new(),
        owner_modules: HashMap::new(),
        metadata: Mutex::new(None),
    };
    assert!(
        source
            .module_source(&format!("{target}.0"))
            .unwrap()
            .contains("Возврат 23")
    );
    assert_eq!(reads.lock().unwrap().last().unwrap(), &key(3).as_hex());
}

#[test]
fn protected_source_discovery_returns_declarations_separately_from_the_full_body() {
    let mut reader = reader();
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-call.Module.bin");
    let length = u16::try_from(bytes.len()).unwrap();
    let mut compressed = vec![1];
    compressed.extend(length.to_le_bytes());
    compressed.extend((!length).to_le_bytes());
    compressed.extend(bytes);
    let name = format!("{COMMON_ID}.0");
    reader.resources.insert(
        name.clone(),
        ConfigResource {
            file_name: name,
            compressed: compressed.into(),
        },
    );
    let source = DatabaseConfigurationSource::from_reader(
        Box::new(reader),
        LoadOptions {
            decompile_protected_modules: true,
        },
    )
    .unwrap();
    let bsl_config::ModuleDeclarationSource::Declarations(declarations) = source
        .module_declarations(&format!("{COMMON_ID}.0"))
        .unwrap()
    else {
        panic!("декларации не должны подменять исполняемый исходник");
    };
    let body = source.module_source(&format!("{COMMON_ID}.0")).unwrap();
    assert!(declarations.contains("Функция Проба(Аргумент) Экспорт"));
    assert_ne!(declarations, body);
    assert!(body.contains("Изменить(Аргумент)"));
}

#[test]
fn metadata_discovery_and_factory_cache_do_not_read_unused_details() {
    let mut reader = reader();
    let unused = "55555555-5555-5555-5555-555555555555";
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace("061d872a-5787-460e-95ac-ed74ea3a3e84", COMMON)
        .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", COMMON_ID)
        .replace(
            &format!("{COMMON},1,{COMMON_ID}"),
            &format!("{COMMON},2,{COMMON_ID},{unused}"),
        );
    reader.bootstrap.descriptor = text(DESCRIPTOR, &root);
    let value = inflate_raw_deflate(&reader.resources[COMMON_ID].compressed).unwrap();
    let value = String::from_utf8(value)
        .unwrap()
        .replace(COMMON_ID, unused)
        .replace("Общий", "Лишний");
    reader.resources.insert(unused.into(), text(unused, &value));
    let reads = reader.reads.clone();
    let source = Arc::new(
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap(),
    );
    let catalog = source.metadata_catalog().unwrap();
    assert_eq!(catalog.objects().len(), 2);
    assert_eq!(
        *reads.lock().unwrap(),
        [COMMON_ID, unused, "physical index", "extension structures"]
    );
    assert_eq!(
        catalog
            .materialized_object_by_id(COMMON_ID)
            .unwrap()
            .unwrap()
            .name(),
        "Общий"
    );
    let count = reads.lock().unwrap().len();
    assert_eq!(
        catalog
            .materialized_object_by_id(COMMON_ID)
            .unwrap()
            .unwrap()
            .name(),
        "Общий"
    );
    assert_eq!(count, 5);
    assert_eq!(reads.lock().unwrap().len(), count);
    assert_eq!(catalog.objects().len(), 2);
    assert!(catalog.object_by_id(unused).is_some());
    let first = source.metadata_snapshot(&[COMMON_ID]).unwrap();
    let second = source.metadata_snapshot(&[COMMON_ID]).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(reads.lock().unwrap().len(), count);
    assert_eq!(
        reads
            .lock()
            .unwrap()
            .iter()
            .filter(|name| name.as_str() == unused)
            .count(),
        1
    );
    catalog.prepare_objects(&[COMMON_ID, unused]).unwrap();
    let grouped = source.metadata_snapshot(&[COMMON_ID, unused]).unwrap();
    catalog.materialized_object_by_id(unused).unwrap().unwrap();
    let after = source.metadata_snapshot(&[COMMON_ID, unused]).unwrap();
    assert!(Arc::ptr_eq(&grouped, &after));
    assert_eq!(catalog.objects().len(), 2);
}

#[test]
fn query_discovery_reads_only_the_named_catalog_and_reuses_the_snapshot() {
    let mut reader = reader();
    let target = "0169b236-52cf-4e6a-966e-2e180783b83b";
    let unused = "55555555-5555-5555-5555-555555555555";
    let class = "cf4abea6-37b2-11d4-940f-008048da11f9";
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace("061d872a-5787-460e-95ac-ed74ea3a3e84", class)
        .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", target)
        .replace(
            &format!("{class},1,{target}"),
            &format!("{class},2,{target},{unused}"),
        );
    reader.bootstrap.descriptor = text(DESCRIPTOR, &root);
    reader.resources.remove(COMMON_ID);
    let native = include_bytes!("../tests/fixtures/catalog-owners/tax-registration.deflate");
    reader.resources.insert(
        target.into(),
        ConfigResource {
            file_name: target.into(),
            compressed: native.as_slice().into(),
        },
    );
    let other = String::from_utf8(inflate_raw_deflate(native).unwrap())
        .unwrap()
        .replace(target, unused)
        .replace("РегистрацииВНалоговомОргане", "Лишний");
    reader.resources.insert(unused.into(), text(unused, &other));
    let reads = reader.reads.clone();
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    assert_eq!(*reads.lock().unwrap(), [target, unused]);
    let query = "ВЫБРАТЬ К.Ссылка ИЗ Справочник.РегистрацииВНалоговомОргане КАК К";
    let first = source.query_metadata(query).unwrap();
    assert!(
        first
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.object_guid.as_str() == target)
    );
    let second = source.query_metadata(query).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    let projected = source.projected_catalog(&first).unwrap();
    assert_eq!(projected.objects().len(), 1);
    assert!(Arc::ptr_eq(
        &projected,
        &source.projected_catalog(&second).unwrap()
    ));
    assert_eq!(
        *reads.lock().unwrap(),
        [
            target,
            unused,
            "physical index",
            "extension structures",
            target
        ]
    );
    let restricted = source
        .query_metadata(
            "ВЫБРАТЬ К.Ссылка ИЗ Справочник.РегистрацииВНалоговомОргане КАК К \
             ГДЕ К.Ссылка В (ВЫБРАТЬ Д.Ссылка ИЗ Справочник.Лишний КАК Д)",
        )
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &restricted));
    let extended = source.projected_catalog(&restricted).unwrap();
    assert_eq!(extended.objects().len(), 2);
    assert!(!Arc::ptr_eq(&projected, &extended));
    assert!(Arc::ptr_eq(
        &extended,
        &source.projected_catalog(&restricted).unwrap()
    ));
    assert!(
        restricted
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.object_guid.as_str() == unused)
    );
    assert!(
        !first
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.object_guid.as_str() == unused)
    );
    assert_eq!(
        reads
            .lock()
            .unwrap()
            .iter()
            .filter(|name| name.as_str() == target)
            .count(),
        2
    );
}

#[test]
fn failed_predefined_read_cannot_publish_an_incomplete_snapshot_on_retry() {
    let mut reader = reader();
    let target = "0169b236-52cf-4e6a-966e-2e180783b83b";
    let class = "cf4abea6-37b2-11d4-940f-008048da11f9";
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace("061d872a-5787-460e-95ac-ed74ea3a3e84", class)
        .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", target);
    reader.bootstrap.descriptor = text(DESCRIPTOR, &root);
    reader.resources.remove(COMMON_ID);
    reader.resources.insert(
        target.into(),
        ConfigResource {
            file_name: target.into(),
            compressed: include_bytes!("../tests/fixtures/catalog-owners/tax-registration.deflate")
                .as_slice()
                .into(),
        },
    );
    let predefined = format!("{target}.1c");
    reader
        .resources
        .insert(predefined.clone(), text(&predefined, "{}"));
    reader.failed_reads.insert(predefined.clone());
    let reads = reader.reads.clone();
    let source =
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap();
    source.metadata_snapshot(&[]).unwrap();
    for _ in 0..2 {
        assert_eq!(
            source.metadata_snapshot(&[target]).unwrap_err(),
            format!("отказ чтения {predefined}")
        );
    }
    assert_eq!(
        reads
            .lock()
            .unwrap()
            .iter()
            .filter(|name| **name == predefined)
            .count(),
        2
    );
}

#[test]
fn bounded_details_restore_native_defaults_through_the_complete_root_registry() {
    let mut reader = reader();
    let event = "571e1bd9-0f64-4a05-a276-dba4fe242ad0";
    let enumeration = "19430845-e6b2-4a2b-944a-75142d4914c0";
    let unused = "55555555-5555-5555-5555-555555555555";
    let class = "061d872a-5787-460e-95ac-ed74ea3a3e84";
    let root =
        include_str!("../tests/fixtures/document-owned-attributes/isolated-enumeration-root.txt")
            .replace(
                &format!("{class},1,{event}"),
                &format!("{class},2,{event},{unused}"),
            );
    reader.bootstrap.descriptor = text(DESCRIPTOR, &root);
    reader.resources.remove(COMMON_ID);
    let event_bytes = include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate");
    for (id, bytes) in [
        (event, event_bytes.as_slice()),
        (
            enumeration,
            include_bytes!(
                "../tests/fixtures/document-owned-attributes/importance-enumeration.deflate"
            )
            .as_slice(),
        ),
    ] {
        reader.resources.insert(
            id.into(),
            ConfigResource {
                file_name: id.into(),
                compressed: bytes.into(),
            },
        );
    }
    let other = String::from_utf8(inflate_raw_deflate(event_bytes).unwrap())
        .unwrap()
        .replace(event, unused)
        .replace("Событие", "Лишний");
    reader.resources.insert(unused.into(), text(unused, &other));
    let reads = reader.reads.clone();
    let source = Arc::new(
        DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default()).unwrap(),
    );
    let root = Arc::new(source.metadata_catalog().unwrap());
    let object = root.materialized_object_by_id(event).unwrap().unwrap();
    let snapshot = source.metadata_snapshot(&[event]).unwrap();
    let details = source.projected_catalog(&snapshot).unwrap();
    assert_eq!(details.objects().len(), 2);
    assert!(details.object_by_id(unused).is_none());
    assert_eq!(root.objects().len(), 3);
    assert!(root.object_by_id(unused).is_some());
    assert_eq!(
        reads
            .lock()
            .unwrap()
            .iter()
            .filter(|name| name.as_str() == unused)
            .count(),
        1
    );
    let mut builder = open_bsl::Engine::builder().register_library(bsl_orm::library());
    for ty in root.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let actual = object
        .document_attributes()
        .unwrap()
        .iter()
        .find(|attribute| attribute.name() == "Важность")
        .unwrap()
        .fill_value()
        .unwrap()
        .materialize(&mut shapes)
        .unwrap();
    let expected = engine
        .registry()
        .value_types()
        .iter()
        .find(|ty| ty.names().contains(&"EnumRef.ВариантыВажности".to_owned()))
        .unwrap()
        .materialize(b"e2942d8d-433f-497b-8b7e-c999fbc88b44")
        .unwrap();
    assert_eq!(actual, vec![expected]);
}

#[path = "indexed_scheduled_job_tests.rs"]
mod scheduled_job_dependency_tests;

#[cfg(test)]
#[path = "indexed_defined_type_tests.rs"]
mod defined_type_tests;
