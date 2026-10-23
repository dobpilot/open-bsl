use super::*;
use open_sdbl::metadata::{DbNames, SchemaStorage, resolve_metadata};

#[test]
fn enumeration_cells_reuse_the_declared_member_and_empty_carrier() {
    let owner = "c143f002-c8de-45ef-8272-ca10ec452db4";
    let member = "40ae3f99-8ef0-4819-8dff-e6701aab4cf4";
    let catalog = std::sync::Arc::new(
        bsl_orm::MetadataCatalog::new(
            "enum-constant",
            vec![bsl_orm::MetadataObject::new(
                owner,
                "EnumLegacyProbe",
                bsl_orm::MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_enumeration_values(vec![bsl_orm::ConfigurationEnumValue::new(
            owner, member, "First",
        )])
        .unwrap(),
    );
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let kind = ColumnKind::Reference {
        targets: vec![open_sdbl::metadata::ObjectId::from(
            &owner.parse::<open_sdbl::metadata::Guid>().unwrap(),
        )],
        runtime_typed: false,
    };
    let ty = catalog.transferable_value_types().next().unwrap();
    for identity in [member, "00000000-0000-0000-0000-000000000000"] {
        let id: open_sdbl::metadata::Guid = identity.parse().unwrap();
        let actual = reference_from_cell(
            Cell::Bytes(id.to_1c_bytes().to_vec()),
            &kind,
            &metadata,
            catalog.clone(),
        )
        .unwrap();
        let expected = ty
            .materialize(if identity == member {
                member.as_bytes()
            } else {
                b""
            })
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.object_ref().unwrap().value_type(),
            expected.object_ref().unwrap().value_type()
        );
    }
}
