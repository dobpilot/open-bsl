use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;

#[test]
fn native_event_register_association_survives_the_configuration_loader() {
    // Корень синтетически ограничен одним документом; его описатель и GUID
    // регистра взяты из Config и нативной XML УНФ соответственно.
    let resources = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource(
            "11111111-1111-1111-1111-111111111111",
            include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt"),
        ),
        resource(
            "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
            include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
        ),
    ];
    let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let event = catalog
        .object_by_id("571e1bd9-0f64-4a05-a276-dba4fe242ad0")
        .unwrap();
    assert_eq!(event.name(), "Событие");
    assert_eq!(
        event.document_registers().unwrap(),
        ["c34d5785-6f04-45c7-9555-088544ddc489"]
    );
}

#[test]
fn native_customer_order_preserves_all_fifty_one_registers_in_order() {
    // Ожидания извлечены из штатной XML УНФ и UUID самих регистров.
    // Проверяем проекцию полного списка через адаптер, а не только SDK.
    let id = "9133d1ee-54e6-4556-8f42-a94010c0b021";
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", id);
    let resources = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource("11111111-1111-1111-1111-111111111111", &root),
        resource(
            id,
            include_bytes!("../tests/fixtures/document-owned-attributes/customer-order.deflate"),
        ),
    ];
    let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let order = catalog.object_by_id(id).unwrap();
    assert_eq!(order.name(), "ЗаказПокупателя");
    let expected = include_str!(
        "../tests/fixtures/document-owned-attributes/customer-order-registers.native.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(expected.len(), 51);
    assert_eq!(order.document_registers().unwrap(), expected);
}
