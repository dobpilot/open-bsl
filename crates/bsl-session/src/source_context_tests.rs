use super::*;
use open_bsl::ModuleContextSource;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct GroupedDetails {
    groups: Mutex<Vec<Vec<String>>>,
    loads: Mutex<Vec<String>>,
}

impl bsl_orm::MetadataObjectSource for GroupedDetails {
    fn prepare_objects(&self, ids: &[&str]) -> Result<(), String> {
        self.groups
            .lock()
            .unwrap()
            .push(ids.iter().map(|id| (*id).into()).collect());
        Ok(())
    }

    fn load_object(&self, id: &str) -> Result<bsl_orm::MetadataObject, String> {
        assert!(
            self.groups
                .lock()
                .unwrap()
                .iter()
                .flatten()
                .any(|known| known == id)
        );
        self.loads.lock().unwrap().push(id.into());
        Ok(bsl_orm::MetadataObject::new(id, id, MetadataKind::Catalog))
    }
}

#[test]
fn a_context_frontier_acquires_only_its_metadata_owners_as_one_group() {
    let execution = open_bsl::ModuleExecution::new().with_server(true);
    let index = open_bsl::ConfigurationIndex::new(
        "group",
        ["one", "two", "unused"]
            .map(|id| {
                open_bsl::ConfigurationModuleDeclaration::new(
                    id,
                    id,
                    ModuleKind::Manager,
                    execution,
                )
                .with_owner(id)
            })
            .to_vec(),
    )
    .unwrap();
    let details = Arc::new(GroupedDetails::default());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "group",
            ["one", "two", "unused"]
                .map(|id| bsl_orm::MetadataObject::new(id, id, MetadataKind::Catalog))
                .to_vec(),
        )
        .unwrap()
        .with_object_source(details.clone()),
    );
    let contexts = SessionModuleContext::new(index, catalog.clone());
    contexts.prepare_contexts(&["two", "one"]).unwrap();
    assert_eq!(*details.groups.lock().unwrap(), [vec!["one", "two"]]);
    contexts.receiver_properties("one").unwrap();
    contexts.receiver_properties("two").unwrap();
    assert_eq!(*details.loads.lock().unwrap(), ["one", "two"]);
    assert_eq!(catalog.objects().len(), 3);
}
