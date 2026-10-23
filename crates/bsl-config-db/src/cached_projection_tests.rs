use super::*;
use std::{borrow::Cow, sync::Arc};

#[test]
fn projection_borrows_original_cached_descriptors_and_preserves_the_fallback() {
    let id = "4bb0f7c3-62f3-4352-9bc8-e243dd18fe4a";
    let resource = ConfigResource {
        file_name: id.into(),
        compressed: Arc::from(
            include_bytes!("../tests/fixtures/catalog-standard-attributes/versions.deflate")
                .as_slice(),
        ),
    };
    let mut cache = open_sdbl_db::ConfigurationMetadataCache::new(open_sdbl_db::Limits::default());
    cache.admit(&resource).unwrap();
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let cached = projected_descriptors(id, &resources, Some(&cache)).unwrap();
    let plain = projected_descriptors(id, &resources, None).unwrap();
    assert!(matches!(cached, Cow::Borrowed(_)));
    assert!(matches!(plain, Cow::Owned(_)));
    assert!(std::ptr::eq(
        cached.as_ref(),
        cache.parsed_resource(id).unwrap().descriptors.as_slice()
    ));
    assert_eq!(format!("{cached:?}"), format!("{plain:?}"));
    let empty = open_sdbl_db::ConfigurationMetadataCache::new(open_sdbl_db::Limits::default());
    let fallback = projected_descriptors(id, &resources, Some(&empty)).unwrap();
    assert!(matches!(fallback, Cow::Owned(_)));
    assert_eq!(format!("{fallback:?}"), format!("{plain:?}"));
}

#[test]
fn projection_without_a_cached_resource_still_rejects_invalid_bytes() {
    let id = "4bb0f7c3-62f3-4352-9bc8-e243dd18fe4a";
    let resource = ConfigResource {
        file_name: id.into(),
        compressed: Arc::from([0xff_u8]),
    };
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let empty = open_sdbl_db::ConfigurationMetadataCache::new(open_sdbl_db::Limits::default());
    let plain = projected_descriptors(id, &resources, None).unwrap_err();
    let cached = projected_descriptors(id, &resources, Some(&empty)).unwrap_err();
    assert_eq!(plain.to_string(), cached.to_string());
}
