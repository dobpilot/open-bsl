use std::sync::Arc;

use open_sdbl_db::ConfigResource;

use super::{LoadError, apply_source_overlay, load_base};

fn resource(name: &str, bytes: &[u8]) -> ConfigResource {
    ConfigResource {
        file_name: name.into(),
        compressed: Arc::from(bytes),
    }
}

#[test]
fn base_resources_become_a_snapshot_with_common_module_flags() {
    let resources = vec![
        resource(
            "root",
            &[171, 54, 210, 49, 132, 2, 93, 44, 4, 12, 232, 212, 2, 0],
        ),
        resource(
            "11111111-1111-1111-1111-111111111111",
            &[
                109, 202, 201, 13, 192, 32, 12, 68, 209, 134, 108, 105, 112, 204, 86, 14, 49, 208,
                132, 69, 239, 1, 41, 185, 229, 29, 230, 50, 223, 133, 60, 188, 248, 103, 62, 139,
                2, 121, 181, 30, 3, 172, 115, 187, 167, 237, 187, 43, 87, 189, 148, 1, 69, 141, 35,
                200, 180, 76, 158, 200, 65, 56, 61, 230, 208, 82, 11, 88, 162, 244, 211, 39, 110,
                151, 229, 221, 71, 220, 109, 160, 229, 156, 118, 39, 47, 254, 153, 207, 58, 30,
            ],
        ),
        resource(
            "22222222-2222-2222-2222-222222222222",
            &[
                171, 54, 212, 169, 54, 52, 210, 169, 6, 34, 67, 29, 3, 29, 35, 40, 208, 197, 66,
                192, 64, 173, 142, 210, 133, 121, 23, 54, 94, 236, 188, 176, 227, 194, 78, 37, 144,
                62, 165, 162, 82, 37, 20, 65, 160, 18, 37, 160, 105, 6, 58, 198, 80, 160, 139, 133,
                128, 129, 90, 29, 144, 205, 134, 96, 245, 96, 178, 86, 199, 160, 22, 0,
            ],
        ),
        resource(
            "22222222-2222-2222-2222-222222222222.0",
            &[
                77, 204, 33, 10, 128, 48, 0, 70, 225, 190, 83, 44, 106, 21, 155, 151, 155, 19, 86,
                5, 163, 193, 100, 21, 100, 32, 14, 97, 243, 10, 239, 191, 145, 139, 214, 143, 199,
                99, 215, 68, 230, 81, 32, 105, 182, 108, 242, 68, 46, 249, 166, 181, 28, 213, 71,
                94, 138, 156, 188, 97, 161, 112, 19, 229, 56, 229, 109, 223, 13, 134, 181, 82, 174,
                121, 224, 247, 33, 125,
            ],
        ),
    ];
    let snapshot = load_base(&resources).expect("синтетическая конфигурация");
    assert_eq!(snapshot.identity(), "11111111-1111-1111-1111-111111111111");
    assert_eq!(snapshot.modules().len(), 1);
    let module = &snapshot.modules()[0];
    assert_eq!(module.name(), "Общий");
    assert!(module.execution().global());
    assert!(module.execution().server());
    assert!(module.execution().server_call());
    assert!(module.source().contains("Возврат 42"));
}

#[test]
fn duplicate_resource_is_rejected_before_decoding() {
    let resources = vec![resource("root", &[]), resource("ROOT", &[])];
    assert!(matches!(
        load_base(&resources),
        Err(LoadError::DuplicateResource(_))
    ));
}

#[test]
fn active_extension_replaces_an_existing_source_resource() {
    let mut base = vec![resource("module.0", &[1, 2, 3])];
    let replacement = resource(
        "MODULE.0",
        &[
            77, 140, 49, 10, 128, 48, 16, 4, 251, 188, 226, 74, 125, 66, 240, 115, 49, 130, 150,
            130, 141, 96, 109, 43, 136, 16, 12, 66, 226, 23, 230, 126, 228, 149, 86, 203, 238, 44,
            195, 174, 3, 133, 71, 71, 178, 206, 194, 106, 229, 212, 137, 100, 153, 73, 77, 43, 28,
            70, 123, 94, 170, 6, 141, 78, 88, 168, 220, 92, 26, 236, 24, 197, 251, 206, 177, 217,
            84, 72, 38, 249, 233, 200, 31,
        ],
    );
    apply_source_overlay(&mut base, "Тестовое", &[replacement]).unwrap();
    let decoded = open_sdbl::metadata::inflate_raw_deflate(base[0].compressed.as_ref()).unwrap();
    assert!(String::from_utf8(decoded).unwrap().contains("Возврат 99"));
}

#[test]
fn structural_extension_is_rejected_with_its_name() {
    let mut base = vec![resource("module.0", &[1, 2, 3])];
    let error = apply_source_overlay(
        &mut base,
        "Структурное",
        &[resource("new-module.0", &[4, 5, 6])],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        LoadError::UnsupportedExtension { name, .. } if name == "Структурное"
    ));
}
