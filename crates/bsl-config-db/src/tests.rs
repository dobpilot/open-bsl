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
                109, 141, 77, 10, 195, 32, 16, 133, 239, 146, 245, 12, 140, 198, 159, 120, 28, 163,
                230, 18, 34, 20, 90, 186, 237, 190, 167, 40, 116, 147, 77, 123, 134, 241, 70, 53,
                197, 236, 252, 22, 15, 134, 55, 188, 47, 75, 200, 162, 131, 131, 56, 41, 32, 32,
                187, 16, 181, 160, 16, 209, 175, 91, 104, 117, 84, 232, 212, 172, 144, 72, 145,
                211, 73, 200, 45, 216, 54, 7, 217, 44, 144, 9, 242, 124, 28, 4, 170, 131, 131, 56,
                41, 48, 241, 147, 191, 252, 169, 55, 222, 249, 93, 175, 245, 194, 175, 122, 231,
                189, 62, 166, 82, 254, 118, 218, 146, 90, 220, 66, 40, 181, 140, 135, 221, 160,
                159, 131, 109, 118, 77, 171, 79, 228, 173, 53, 237, 79, 118, 112, 16, 39, 109, 177,
                252, 0,
            ],
        ),
        resource(
            "44444444-4444-4444-4444-444444444444.7",
            &[
                187, 48, 255, 98, 195, 133, 125, 23, 219, 46, 108, 189, 176, 229, 98, 51, 144, 189,
                65, 225, 2, 72, 104, 199, 133, 185, 23, 54, 92, 108, 191, 176, 225, 194, 110, 160,
                212, 66, 32, 222, 112, 97, 239, 197, 198, 11, 27, 52, 52, 185, 46, 204, 186, 176,
                239, 194, 222, 11, 91, 129, 186, 208, 116, 95, 236, 230, 2, 0,
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
    assert_eq!(snapshot.identity(), "44444444-4444-4444-4444-444444444444");
    assert_eq!(snapshot.modules().len(), 2);
    let session = &snapshot.modules()[0];
    assert_eq!(session.name(), "МодульСеанса");
    assert!(session.source().contains("ПриНачалеСеанса"));
    let module = &snapshot.modules()[1];
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
