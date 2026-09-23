use std::sync::Arc;

use open_sdbl::metadata::{InfoBaseUser, UserRow};
use open_sdbl_db::ConfigResource;

use super::{
    LoadError, apply_source_overlay, common_execution, load_base, load_catalog_base, metadata_kind,
    principal_candidates,
};

#[test]
fn enumeration_metadata_class_is_projected() {
    assert_eq!(
        metadata_kind("f6a80749-5ad7-400b-8519-39dc5dff2542"),
        Some(bsl_orm::MetadataKind::Enumeration)
    );
}

#[test]
fn common_picture_metadata_class_is_projected() {
    assert_eq!(
        metadata_kind("7dcd43d9-aca5-4926-b549-1842e6a4e8cf"),
        Some(bsl_orm::MetadataKind::CommonPicture)
    );
}

#[test]
fn language_metadata_class_is_projected() {
    assert_eq!(
        metadata_kind("9cd510ce-abfc-11d4-9434-004095e12fc7"),
        Some(bsl_orm::MetadataKind::Language)
    );
}

fn resource(name: &str, bytes: &[u8]) -> ConfigResource {
    ConfigResource {
        file_name: name.into(),
        compressed: Arc::from(bytes),
    }
}

fn text_resource(name: &str, text: &str) -> ConfigResource {
    let bytes = text.as_bytes();
    assert!(bytes.len() <= u16::MAX as usize);
    let length = bytes.len() as u16;
    let mut compressed = vec![1];
    compressed.extend_from_slice(&length.to_le_bytes());
    compressed.extend_from_slice(&(!length).to_le_bytes());
    compressed.extend_from_slice(bytes);
    resource(name, &compressed)
}

#[test]
fn catalog_is_projected_from_the_same_root_and_links_default_roles() {
    let root = r#"{2,{11111111-1111-1111-1111-111111111111},1,{9cd510cd-abfc-11d4-9434-004095e12fc7,{1,{68,{0,{3,{1,0,44444444-4444-4444-4444-444444444444},"Конфигурация"}},"",1,{0,0},{0,0},{0,0},{0,0},{0,0},00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,0,"Поставщик","1.2.3","",1,{0,0},1,{0,0},1,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,80324,{0,0},1,0,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,{0,0},00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,2,00000000-0000-0000-0000-000000000000,2,{1,55555555-5555-5555-5555-555555555555},{0,0},0,"",80327,1,0,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,00000000-0000-0000-0000-000000000000,1,00000000-0000-0000-0000-000000000000,{0,0},{0,0},{0,0},{0,0},00000000-0000-0000-0000-000000000000,"",0,1,{0,0},1},2,{0fe48980-252d-11d6-a3c7-0050bae0a776,1,22222222-2222-2222-2222-222222222222},{09736b02-9cac-4e3f-b4f7-d3e9576ab948,1,55555555-5555-5555-5555-555555555555}}}}"#;
    let resources = vec![
        text_resource("root", "{1,{11111111-1111-1111-1111-111111111111}}"),
        text_resource("11111111-1111-1111-1111-111111111111", root),
        text_resource(
            "22222222-2222-2222-2222-222222222222",
            r#"{1,{12,{2,{1,0,22222222-2222-2222-2222-222222222222},"Общий",{1,"ru","Общий"},"",0,0,33333333-3333-3333-3333-333333333333},1,0,1,0,0,1,0,0},0}"#,
        ),
        text_resource(
            "55555555-5555-5555-5555-555555555555",
            r#"{1,{12,{2,{1,0,55555555-5555-5555-5555-555555555555},"Основная",{1,"ru","Основная"},"",0,0,66666666-6666-6666-6666-666666666666},0}}"#,
        ),
    ];
    let code = load_base(&resources).expect("снимок кода");
    let catalog = load_catalog_base(&resources, &code).expect("каталог метаданных");

    assert_eq!(catalog.identity(), code.identity());
    assert_eq!(catalog.objects().len(), 2);
    assert_eq!(
        catalog.main_role_ids().collect::<Vec<_>>(),
        ["55555555-5555-5555-5555-555555555555"]
    );
}

fn user(
    id: &str,
    name: &str,
    roles: &[&str],
    standard_authentication: bool,
    os_name: &str,
) -> InfoBaseUser {
    let role_count = roles.len();
    let roles = roles
        .iter()
        .map(|role| format!(",{role}"))
        .collect::<String>();
    let record = format!(r#"{{{id},"{name}","Полное имя",{{{role_count}{roles}}}}}"#);
    let key = 0x5a;
    let mut encoded = vec![1, key];
    encoded.extend(record.as_bytes().iter().map(|byte| byte ^ key));
    InfoBaseUser::new(
        UserRow {
            name,
            description: "Полное имя",
            os_name,
            email: "",
            show_in_list: false,
            standard_authentication,
            administrative: true,
        },
        &encoded,
    )
    .unwrap()
}

#[test]
fn principal_candidates_preserve_identity_roles_and_authentication_decision() {
    let role = "22222222-2222-2222-2222-222222222222";
    let users = [
        user(
            "11111111-1111-1111-1111-111111111111",
            "Стандартный",
            &[role],
            true,
            "",
        ),
        user(
            "33333333-3333-3333-3333-333333333333",
            "ОперационнаяСистема",
            &[],
            false,
            " DOMAIN\\user ",
        ),
        user(
            "44444444-4444-4444-4444-444444444444",
            "Запрещённый",
            &[],
            false,
            " ",
        ),
    ];

    let candidates = principal_candidates(&users);

    assert_eq!(candidates[0].principal().name(), "Стандартный");
    assert_eq!(
        candidates[0].principal().role_ids().collect::<Vec<_>>(),
        [role]
    );
    assert!(candidates[0].can_authenticate());
    assert!(candidates[1].can_authenticate());
    assert!(!candidates[2].can_authenticate());
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
    assert!(!module.execution().global());
    assert!(module.execution().client());
    assert!(module.execution().external_connection());
    assert!(!module.execution().server());
    assert!(!module.execution().server_call());
    assert!(module.source().contains("Возврат 42"));
}

#[test]
fn common_module_flags_follow_the_serialized_descriptor_order() {
    let execution = common_execution(br#"{0,{12,{identity},0,1,0,1,1,1,0,1}}"#).unwrap();

    assert!(execution.client());
    assert!(execution.server());
    assert!(!execution.external_connection());
    assert!(execution.privileged());
    assert!(execution.global());
    assert!(execution.server_call());
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
