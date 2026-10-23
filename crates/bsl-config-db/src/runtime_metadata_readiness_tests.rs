use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

const ONE: &str = "22222222-2222-2222-2222-222222222222";
const TWO: &str = "55555555-5555-5555-5555-555555555555";
const DESCRIPTOR: &str = "11111111-1111-1111-1111-111111111111";

struct Reader {
    resources: HashMap<String, ConfigResource>,
    reject_second: Arc<AtomicBool>,
}

impl ResourceReader for Reader {
    fn bootstrap(&mut self) -> Result<ConfigurationBootstrap, String> {
        Ok(ConfigurationBootstrap {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            root: self.resources["root"].clone(),
            descriptor: self.resources[DESCRIPTOR].clone(),
            extensions: vec![],
        })
    }
    fn names(&mut self) -> Result<Vec<String>, String> {
        Ok(self.resources.keys().cloned().collect())
    }
    fn base(&mut self, names: &[&str]) -> Result<Vec<ConfigResource>, String> {
        names
            .iter()
            .map(|name| {
                if *name == TWO && self.reject_second.load(Ordering::Relaxed) {
                    return Err("отказ чтения второго объекта".into());
                }
                self.resources
                    .get(*name)
                    .cloned()
                    .ok_or_else(|| format!("нет ресурса {name}"))
            })
            .collect()
    }
    fn extension(&mut self, _: &[ContentKey]) -> Result<Vec<ConfigResource>, String> {
        Err("нет расширений в фикстуре".into())
    }
    fn metadata_inputs(&mut self) -> Result<ConfigurationMetadataInputs, String> {
        Ok(ConfigurationMetadataInputs {
            layout: open_sdbl::metadata::StorageLayout::MODERN,
            db_names: open_sdbl::metadata::DbNames::from_entries(vec![]),
            schema: open_sdbl::metadata::SchemaStorage {
                tables: vec![],
                anomalies: vec![],
            },
            live_tables: vec![],
        })
    }
    fn extension_metadata(&mut self) -> Result<Vec<ExtensionMetadata>, String> {
        Ok(vec![])
    }
}

fn source() -> (Arc<DatabaseConfigurationSource>, Arc<AtomicBool>) {
    let reject_second = Arc::new(AtomicBool::new(false));
    let resources = [
        (
            "root",
            include_bytes!("../tests/fixtures/runtime-metadata-readiness/root.deflate").as_slice(),
        ),
        (
            DESCRIPTOR,
            include_bytes!("../tests/fixtures/runtime-metadata-readiness/descriptor.deflate")
                .as_slice(),
        ),
        (
            ONE,
            include_bytes!("../tests/fixtures/runtime-metadata-readiness/one.deflate").as_slice(),
        ),
        (
            TWO,
            include_bytes!("../tests/fixtures/runtime-metadata-readiness/two.deflate").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        (
            name.to_string(),
            ConfigResource {
                file_name: name.into(),
                compressed: bytes.into(),
            },
        )
    })
    .collect();
    let reader = Reader {
        resources,
        reject_second: Arc::clone(&reject_second),
    };
    (
        Arc::new(
            DatabaseConfigurationSource::from_reader(Box::new(reader), LoadOptions::default())
                .unwrap(),
        ),
        reject_second,
    )
}

#[test]
fn runtime_growth_does_not_publish_a_query_snapshot_and_reuses_its_facts() {
    let (source, _) = source();
    source.metadata_catalog().unwrap();
    bsl_orm::MetadataObjectSource::load_object(source.as_ref(), ONE).unwrap();
    assert!(
        source
            .metadata
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .snapshot
            .is_none()
    );
    let first = source.runtime_metadata(&[ONE]).unwrap();
    let repeated = source.runtime_metadata(&[ONE]).unwrap();
    assert!(Arc::ptr_eq(&first, &repeated));
    assert!(Arc::ptr_eq(
        &source.projected_runtime_catalog(&first).unwrap(),
        &source.projected_runtime_catalog(&repeated).unwrap()
    ));
    bsl_orm::MetadataObjectSource::prepare_objects(source.as_ref(), &[ONE, TWO]).unwrap();
    let state = source.metadata.lock().unwrap();
    let state = state.as_ref().unwrap();
    assert!(state.snapshot.is_none());
    assert!(!Arc::ptr_eq(state.runtime.as_ref().unwrap(), &first));
    assert!(
        state
            .runtime
            .as_ref()
            .unwrap()
            .descriptors()
            .iter()
            .any(|entry| entry.object_guid.as_str() == TWO)
    );
}

#[test]
fn explicit_snapshot_finalizes_current_facts_and_retains_the_old_identity() {
    let (source, _) = source();
    let first = source.metadata_snapshot(&[ONE]).unwrap();
    let fingerprint = first.fingerprint();
    bsl_orm::MetadataObjectSource::load_object(source.as_ref(), TWO).unwrap();
    assert!(
        source
            .metadata
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .snapshot
            .is_none()
    );
    let second = source.metadata_snapshot(&[]).unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(first.fingerprint(), fingerprint);
    assert!(
        !first
            .descriptors()
            .iter()
            .any(|entry| entry.object_guid.as_str() == TWO)
    );
    assert!(
        second
            .descriptors()
            .iter()
            .any(|entry| entry.object_guid.as_str() == TWO)
    );
    assert!(Arc::ptr_eq(
        &second,
        &source.metadata_snapshot(&[]).unwrap()
    ));
    let state = source.metadata.lock().unwrap();
    let state = state.as_ref().unwrap();
    let full = state
        .decoded
        .resolve(&state.inputs, state.extensions.clone())
        .unwrap();
    assert_eq!(second.fingerprint(), full.snapshot.fingerprint());
    assert_eq!(second.descriptors(), full.snapshot.descriptors());
}

#[test]
fn failed_runtime_growth_does_not_publish_incomplete_facts_and_can_retry() {
    let (source, reject) = source();
    let initial = source.runtime_metadata(&[ONE]).unwrap();
    reject.store(true, Ordering::Relaxed);
    assert!(source.runtime_metadata(&[TWO]).is_err());
    let retained = source
        .metadata
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .runtime
        .clone()
        .unwrap();
    assert!(Arc::ptr_eq(&initial, &retained));
    assert!(
        !retained
            .descriptors()
            .iter()
            .any(|entry| entry.object_guid.as_str() == TWO)
    );
    reject.store(false, Ordering::Relaxed);
    let complete = source.runtime_metadata(&[TWO]).unwrap();
    assert!(
        complete
            .descriptors()
            .iter()
            .any(|entry| entry.object_guid.as_str() == TWO)
    );
    assert!(
        source
            .metadata
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .snapshot
            .is_none()
    );
}
