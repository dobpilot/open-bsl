use super::*;

fn module(id: &str, name: &str) -> ConfigurationModule {
    ConfigurationModule::new(
        id,
        name,
        "Процедура Тест() Экспорт\nКонецПроцедуры",
        ModuleKind::Common,
        ModuleExecution::new()
            .with_server(true)
            .with_external_connection(true),
    )
}

#[test]
fn snapshot_keeps_one_immutable_logical_graph() {
    let snapshot = ConfigurationCodeSnapshot::new(
        "configuration-guid",
        vec![
            module("common", "ОбщийМодуль"),
            module("session", "МодульСеанса")
                .with_imports(vec![ModuleImport::new("ОбщийМодуль", "common")])
                .with_owner("configuration-guid"),
        ],
    )
    .unwrap();

    assert_eq!(snapshot.identity(), "configuration-guid");
    assert_eq!(snapshot.modules().len(), 2);
    assert_eq!(snapshot.module("common").unwrap().name(), "ОбщийМодуль");
    assert_eq!(
        snapshot.module("session").unwrap().imports()[0].module(),
        "common"
    );
    assert_eq!(
        snapshot.module("session").unwrap().owner(),
        Some("configuration-guid")
    );
    assert!(snapshot.module("common").unwrap().execution().server());

    let cloned = snapshot.clone();
    assert!(std::ptr::eq(
        snapshot.modules().as_ptr(),
        cloned.modules().as_ptr()
    ));
}

#[test]
fn snapshot_rejects_duplicate_module_identity() {
    let error = ConfigurationCodeSnapshot::new(
        "configuration-guid",
        vec![module("same", "Первый"), module("same", "Второй")],
    )
    .unwrap_err();

    assert_eq!(
        error,
        SnapshotError::DuplicateModuleIdentity {
            module: "same".into()
        }
    );
}

#[test]
fn snapshot_rejects_import_outside_graph() {
    let error = ConfigurationCodeSnapshot::new(
        "configuration-guid",
        vec![
            module("session", "МодульСеанса")
                .with_imports(vec![ModuleImport::new("Нет", "missing")]),
        ],
    )
    .unwrap_err();

    assert_eq!(
        error,
        SnapshotError::UnknownImportedModule {
            module: "session".into(),
            imported: "missing".into()
        }
    );
}
