use super::*;
use open_bsl::ModuleContextSource;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct RecordedOwners(Mutex<Vec<Vec<String>>>);

impl bsl_orm::MetadataObjectSource for RecordedOwners {
    fn prepare_objects(&self, ids: &[&str]) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .push(ids.iter().map(|id| (*id).into()).collect());
        Ok(())
    }

    fn load_object(&self, id: &str) -> Result<bsl_orm::MetadataObject, String> {
        Ok(bsl_orm::MetadataObject::new(id, id, MetadataKind::Catalog))
    }
}

#[test]
fn context_lookup_preserves_index_order_deduplication_and_unicode_case() {
    let execution = open_bsl::ModuleExecution::new().with_server(true);
    let index = open_bsl::ConfigurationIndex::new(
        "lookup",
        [
            ("Первый", "one"),
            ("Второй", "two"),
            ("i", "one"),
            ("Лишний", "unused"),
        ]
        .into_iter()
        .map(|(name, owner)| {
            open_bsl::ConfigurationModuleDeclaration::new(
                name,
                name,
                ModuleKind::Manager,
                execution,
            )
            .with_owner(owner)
        })
        .collect(),
    )
    .unwrap();
    let details = Arc::new(RecordedOwners::default());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "lookup",
            ["one", "two", "unused"]
                .map(|id| bsl_orm::MetadataObject::new(id, id, MetadataKind::Catalog))
                .to_vec(),
        )
        .unwrap()
        .with_object_source(details.clone()),
    );
    let context = SessionModuleContext::new(index, catalog);
    context
        .prepare_contexts(&["ı", "ВТОРОЙ", "первый", "Второй", "Нет"])
        .unwrap();
    assert_eq!(*details.0.lock().unwrap(), [vec!["one", "two"]]);
    context.receiver_properties("ПЕРВЫЙ").unwrap();
    context.receiver_properties("ı").unwrap();
    assert_eq!(
        context.receiver_properties("Нет").unwrap_err(),
        "модуль «Нет» отсутствует в индексе"
    );
}
