use super::{resource, text_resource};
use bsl_config::ConfigurationCodeSnapshot;
use bsl_orm::CatalogHierarchyType;

#[test]
fn native_catalog_hierarchy_kind_and_flag_survive_projection_independently() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/catalog-hierarchy");
    for row in include_str!("../tests/fixtures/catalog-hierarchy/native.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let id = fields[1];
        let root = include_str!("../tests/fixtures/document-owned-attributes/isolated-root.txt")
            .replace("571e1bd9-0f64-4a05-a276-dba4fe242ad0", id)
            .replace(
                "061d872a-5787-460e-95ac-ed74ea3a3e84",
                "cf4abea6-37b2-11d4-940f-008048da11f9",
            );
        let bytes = std::fs::read(directory.join(format!("{}-{id}.deflate", fields[0]))).unwrap();
        let resources = vec![
            text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
            text_resource("11111111-1111-1111-1111-111111111111", &root),
            resource(id, &bytes),
        ];
        let code = ConfigurationCodeSnapshot::new("isolated-root", vec![]).unwrap();
        let catalog = crate::load_catalog_base(&resources, &code, vec![]).unwrap();
        let object = catalog.object_by_id(id).unwrap();
        assert_eq!(object.name(), fields[2]);
        assert_eq!(object.catalog_hierarchical(), Some(fields[4] == "true"));
        assert_eq!(
            object.catalog_hierarchy_type(),
            Some(match fields[3] {
                "HierarchyFoldersAndItems" => CatalogHierarchyType::FoldersAndItems,
                "HierarchyOfItems" => CatalogHierarchyType::Items,
                _ => panic!("неизвестный нативный вид"),
            })
        );
    }
}
