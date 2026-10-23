use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;

#[test]
fn native_tax_registration_owners_survive_configuration_projection() {
    // Корень ограничен одним справочником; сжатый описатель взят без изменений
    // из Config buh. UUID владельца независимо подтверждён нативной XML.
    let id = "0169b236-52cf-4e6a-966e-2e180783b83b";
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
            include_bytes!("../tests/fixtures/catalog-owners/tax-registration.deflate"),
        ),
    ];
    let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
    let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
    let object = catalog.object_by_id(id).unwrap();
    assert_eq!(object.name(), "РегистрацииВНалоговомОргане");
    assert_eq!(
        object.catalog_owners().unwrap(),
        ["fd0c3124-91f5-4c1e-bbc0-f2163e61ff2a"]
    );
}
