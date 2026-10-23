use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;
use bsl_orm::FillChecking;

#[test]
fn native_countries_standard_checks_survive_projection_without_probe_defaults() {
    // Описатель сохранён без изменений из Config buh; собственная XML
    // независимо подтверждает проверку обоих стандартных реквизитов.
    let id = "5b2a2ccb-a948-4628-b145-1ce5696fdfe0";
    let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
        .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", id)
        .replace(
            "061d872a-5787-460e-95ac-ed74ea3a3e84",
            "cf4abea6-37b2-11d4-940f-008048da11f9",
        );
    let resources = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource("11111111-1111-1111-1111-111111111111", &root),
        resource(
            id,
            include_bytes!("../tests/fixtures/fill-checking/countries.deflate"),
        ),
    ];
    let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let object = catalog.object_by_id(id).unwrap();
    assert_eq!(object.name(), "СтраныМира");
    let settings = object.catalog_standard_fill_checking().unwrap();
    for name in ["Code", "Description"] {
        assert_eq!(
            settings
                .iter()
                .find(|setting| setting.name == name)
                .unwrap()
                .fill_checking,
            FillChecking::ShowError,
        );
    }
}
