use std::{collections::HashMap, sync::Arc};

use bsl_config::{ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::MetadataCatalog;
use open_sdbl::metadata::{
    InfoBaseUser, UserRow, parse_extension_catalog, parse_extension_objects,
};
use open_sdbl_db::{ConfigResource, ConfigSelector};

#[path = "module_source_origin_tests.rs"]
mod module_source_origin;

use super::{
    BaseObject, FactoryConfigSelector, LoadError, Resources, STYLE_ITEM, SUBSYSTEM,
    append_subsystem, apply_extension_object, common_execution, load_base, load_catalog_base,
    metadata_kind, ordered_active_positions, principal_candidates,
    project_extension_metadata_catalog, reject_extension_session_module, retain_factory_resource,
    retain_factory_resources,
};

#[cfg(test)]
#[path = "document_registers_tests.rs"]
mod document_registers_tests;

#[path = "catalog_owners_tests.rs"]
mod catalog_owners_tests;

#[path = "common_attributes_tests.rs"]
mod common_attributes_tests;

#[path = "fill_checking_tests.rs"]
mod fill_checking_tests;

#[path = "catalog_hierarchy_tests.rs"]
mod catalog_hierarchy_tests;

#[path = "document_filling_acquisition_tests.rs"]
mod document_filling_acquisition_tests;

#[cfg(test)]
#[path = "document_numbering_tests.rs"]
mod document_numbering_tests;

#[path = "native_module_loading_tests.rs"]
mod native_module_loading_tests;

#[path = "protected_module_options_tests.rs"]
mod protected_module_options_tests;

#[test]
fn selected_acquisition_keeps_unlisted_bare_guid_descriptors() {
    let selected = std::collections::HashSet::new();
    assert!(retain_factory_resource(
        &selected,
        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"
    ));
    assert!(!retain_factory_resource(
        &selected,
        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb.0"
    ));
}

#[test]
fn extension_settings_preserve_boolean_and_named_profile_modes() {
    use bsl_orm::{ConfigurationExtensionScope, ExtensionSafeMode};
    use open_sdbl_db::ExtensionScope;

    for safe in [false, true] {
        assert_eq!(
            super::extension_settings(Some(safe), Some(""), Some(ExtensionScope::InfoBase)),
            Some((
                ExtensionSafeMode::Enabled(safe),
                ConfigurationExtensionScope::InfoBase
            ))
        );
    }
    assert_eq!(
        super::extension_settings(Some(true), Some("Профиль"), Some(ExtensionScope::InfoBase)),
        Some((
            ExtensionSafeMode::profile("Профиль"),
            ConfigurationExtensionScope::InfoBase
        ))
    );
}

#[test]
fn extension_settings_refuse_unknown_properties_instead_of_using_defaults() {
    use open_sdbl_db::ExtensionScope;

    assert!(super::extension_settings(None, Some(""), Some(ExtensionScope::InfoBase)).is_none());
    assert!(super::extension_settings(Some(true), None, Some(ExtensionScope::InfoBase)).is_none());
    assert!(super::extension_settings(Some(true), Some(""), None).is_none());
}

#[test]
fn extension_identity_rejects_malformed_storage_identifiers() {
    for identity in [
        "",
        "00",
        "0000000000000000000000000000000x",
        "я0000000000000000000000000000000",
    ] {
        assert!(super::extension_identity(identity).is_none());
    }
}

#[test]
fn extension_identity_matches_the_native_platform_uuid() {
    let native: open_sdbl::metadata::Guid = "222d62b8-bcb8-11f1-87bb-02d78b8d8e99".parse().unwrap();
    assert_eq!(
        super::extension_identity("87bb02d78b8d8e9911f1bcb8222d62b8"),
        Some(native.to_bytes())
    );
}

#[test]
fn native_language_descriptor_agrees_with_the_xml_language_code() {
    let descriptor = r#"{1,{0,{3,{1,0,af1b61ff-d1b9-4053-ad85-8259b6626adc},"Русский",{1,"ru","Русский"},"",0,0,00000000-0000-0000-0000-000000000000,0},"ru"},0}"#;
    let value = open_sdbl::metadata::parse_serialized(descriptor.as_bytes()).unwrap();
    assert_eq!(super::language_code(&value), Some("ru"));
    let empty = open_sdbl::metadata::parse_serialized(b"{1,{0,{},\"\"},0}").unwrap();
    assert_eq!(super::language_code(&empty), Some(""));
    for descriptor in ["{1,{0,{}},0}", "{1,{1,{},\"ru\"},0}", "{1,{0,{},{}},0}"] {
        let value = open_sdbl::metadata::parse_serialized(descriptor.as_bytes()).unwrap();
        assert_eq!(super::language_code(&value), None);
    }
}

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

#[test]
fn sequence_metadata_class_is_projected() {
    assert_eq!(
        metadata_kind("bc587f20-35d9-11d6-a3c7-0050bae0a776"),
        Some(bsl_orm::MetadataKind::Sequence)
    );
}

fn resource(name: &str, bytes: &[u8]) -> ConfigResource {
    ConfigResource {
        file_name: name.into(),
        compressed: Arc::from(bytes),
    }
}

#[test]
fn native_numeric_catalog_settings_reach_orm_without_string_defaults() {
    use bsl_orm::{CatalogCodeType, CatalogStandardAttributes, MetadataKind, MetadataObject};
    let id = "4bb0f7c3-62f3-4352-9bc8-e243dd18fe4a";
    let descriptor = open_sdbl::metadata::parse_config_descriptor(
        id,
        include_bytes!("../tests/fixtures/catalog-standard-attributes/versions.deflate"),
    )
    .unwrap()
    .unwrap();
    let object = MetadataObject::new(id, descriptor.name, MetadataKind::Catalog)
        .with_catalog_standard_attributes(super::catalog_standard_attributes(
            descriptor.catalog_standard_attributes.unwrap(),
        ))
        .with_catalog_default_presentation(super::catalog_default_presentation(
            descriptor.catalog_default_presentation.unwrap(),
        ));
    assert_eq!(object.name(), "ВерсииРасширений");
    assert_eq!(
        object.catalog_default_presentation(),
        Some(bsl_orm::CatalogDefaultPresentation::AsCode)
    );
    assert_eq!(
        object.catalog_standard_attributes(),
        Some(CatalogStandardAttributes {
            code_type: CatalogCodeType::Number,
            code_length: 9,
            description_length: 0,
        })
    );
}

#[test]
fn object_module_binding_uses_owner_and_kind_instead_of_display_name() {
    use bsl_orm::{MetadataKind, MetadataObject};
    let code = bsl_config::ConfigurationCodeSnapshot::new(
        "configuration",
        vec![
            ConfigurationModule::new(
                "manager",
                "Менеджер",
                "",
                ModuleKind::Manager,
                ModuleExecution::default().with_server(true),
            )
            .with_owner("ABC"),
            ConfigurationModule::new(
                "other",
                "ЧужойОбъект",
                "",
                ModuleKind::Object,
                ModuleExecution::default().with_server(true),
            )
            .with_owner("other"),
            ConfigurationModule::new(
                "object",
                "Справочник.Тест.МодульОбъекта",
                "",
                ModuleKind::Object,
                ModuleExecution::default().with_server(true),
            )
            .with_owner("ABC"),
        ],
    )
    .unwrap();
    let object = super::bind_object_module(
        MetadataObject::new("abc", "Тест", MetadataKind::Catalog),
        &code,
    );
    assert_eq!(
        object.object_module(),
        Some("Справочник.Тест.МодульОбъекта")
    );
    assert_eq!(object.manager_module(), None);
    assert_eq!(
        super::bind_object_module(
            MetadataObject::new("absent", "Тест", MetadataKind::Catalog),
            &code
        )
        .object_module(),
        None
    );
}

fn text_resource(name: &str, text: &str) -> ConfigResource {
    bytes_resource(name, text.as_bytes())
}

fn bytes_resource(name: &str, bytes: &[u8]) -> ConfigResource {
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
    let mut resources = vec![
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
        text_resource(
            "22222222-2222-2222-2222-222222222222.0",
            "Функция Ответ() Экспорт Возврат 42; КонецФункции",
        ),
        text_resource("22222222-2222-2222-2222-222222222222.9", "макет"),
    ];
    let mut selector = FactoryConfigSelector::default();
    assert_eq!(
        selector.root_resource_name(&resources[0]).unwrap(),
        "11111111-1111-1111-1111-111111111111"
    );
    selector.configure(&resources[1]).unwrap();
    let selected_names = resources
        .iter()
        .filter(|resource| selector.retain(&resource.file_name))
        .map(|resource| resource.file_name.clone())
        .collect::<Vec<_>>();
    assert!(selector.retain("22222222-2222-2222-2222-222222222222.0"));
    assert!(!selector.retain("22222222-2222-2222-2222-222222222222.9"));
    retain_factory_resources(&mut resources).expect("отбор ресурсов фабрики");
    assert_eq!(
        resources
            .iter()
            .map(|resource| resource.file_name.clone())
            .collect::<Vec<_>>(),
        selected_names
    );
    assert!(
        resources
            .iter()
            .any(|resource| { resource.file_name == "22222222-2222-2222-2222-222222222222.0" })
    );
    assert!(
        resources
            .iter()
            .all(|resource| { resource.file_name != "22222222-2222-2222-2222-222222222222.9" })
    );
    let code = load_base(&resources).expect("снимок кода");
    let catalog = load_catalog_base(&resources, &code, Vec::new()).expect("каталог метаданных");

    assert_eq!(catalog.identity(), code.identity());
    assert_eq!(catalog.objects().len(), 2);
    assert_eq!(
        catalog.main_role_ids().collect::<Vec<_>>(),
        ["55555555-5555-5555-5555-555555555555"]
    );
}

#[test]
fn nested_subsystem_descriptors_preserve_their_parent() {
    let parent = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let child = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let parent_record = format!(
        "{{1,{{22,{{3,{{1,0,{parent}}},\"Раздел\",{{1,\"ru\",\"Раздел\"}},\"\",0,0,00000000-0000-0000-0000-000000000000,0}},1,{{0,0}},1}},1,{{{SUBSYSTEM},1,{child}}}}}"
    );
    let child_record = format!(
        "{{1,{{22,{{3,{{1,0,{child}}},\"Подраздел\",{{1,\"ru\",\"Подраздел\"}},\"\",0,0,00000000-0000-0000-0000-000000000000,0}},1,{{0,0}},0}},1,{{{SUBSYSTEM},0}}}}"
    );
    let resources = [
        text_resource(parent, &parent_record),
        text_resource(child, &child_record),
    ];
    let resources = Resources::new(&resources).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut objects = Vec::new();
    append_subsystem(&resources, parent, None, &mut seen, &mut objects).unwrap();
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0].name(), "Раздел");
    assert_eq!(objects[0].subsystem_parent(), None);
    assert_eq!(objects[0].subsystem_command_interface(), Some(true));
    assert_eq!(objects[1].name(), "Подраздел");
    assert_eq!(objects[1].subsystem_parent(), Some(parent));
    assert_eq!(objects[1].subsystem_command_interface(), Some(false));
    MetadataCatalog::new("configuration", objects).unwrap();
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
fn principal_candidates_keep_full_name_separate_from_user_name() {
    let mut decoded_user = user(
        "11111111-1111-1111-1111-111111111111",
        "Краткое",
        &[],
        true,
        "",
    );
    decoded_user.data.full_name = "Полное имя".into();
    decoded_user.description = "Описание строки".into();
    let principal = principal_candidates(&[decoded_user]);
    assert_eq!(principal[0].principal().name(), "Краткое");
    assert_eq!(principal[0].principal().full_name(), "Полное имя");
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

const CATALOG_CLASS: &str = "cf4abea6-37b2-11d4-940f-008048da11f9";
const BASE_OBJECT: &str = "11111111-1111-1111-1111-111111111111";
const LOCAL_OBJECT: &str = "22222222-2222-2222-2222-222222222222";

fn extension_object(belonging: u8, extends: &str) -> open_sdbl::metadata::ExtensionObject {
    let source = format!(
        r#"{{3,{{1,0,{LOCAL_OBJECT}}},"Локальный",{{1,"ru","Локальный"}},"",{belonging},0,{extends},0}}"#
    );
    parse_extension_objects(source.as_bytes())
        .unwrap()
        .pop()
        .unwrap()
}

#[test]
fn active_extensions_are_selected_in_platform_order() {
    let extensions = [
        (0, "Позднее", Some(true), 20),
        (1, "Отключенное", Some(false), 1),
        (2, "Раннее", Some(true), 10),
    ];

    assert_eq!(
        ordered_active_positions(extensions.into_iter()).unwrap(),
        [2, 0]
    );
}

#[test]
fn unknown_extension_activity_fails_instead_of_looking_inactive() {
    let extensions = [(0, "Известное", Some(true), 1), (1, "Неизвестное", None, 2)];

    let error = ordered_active_positions(extensions.into_iter()).unwrap_err();

    assert!(matches!(
        error,
        LoadError::UnsupportedExtension {
            name,
            activity_known: false,
        } if name == "Неизвестное"
    ));
}

fn base_objects_for_test() -> HashMap<String, BaseObject> {
    HashMap::from([(
        BASE_OBJECT.into(),
        BaseObject {
            class: CATALOG_CLASS.into(),
            name: "Базовый".into(),
        },
    )])
}

#[test]
fn own_extension_object_adds_its_module_under_local_identity() {
    let resources = vec![text_resource(
        &format!("{LOCAL_OBJECT}.3"),
        "Функция Новая() Экспорт\nВозврат 1;\nКонецФункции",
    )];
    let resources = Resources::new(&resources).unwrap();
    let mut modules = Vec::new();

    apply_extension_object(
        &mut modules,
        &base_objects_for_test(),
        &resources,
        CATALOG_CLASS,
        &extension_object(0, "00000000-0000-0000-0000-000000000000"),
        "Тестовое",
    )
    .unwrap();

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].id(), format!("{LOCAL_OBJECT}.3"));
    assert_eq!(modules[0].name(), "Справочник.Локальный.МодульМенеджера");
    assert_eq!(modules[0].owner(), Some(LOCAL_OBJECT));
}

#[test]
fn adopted_extension_object_replaces_module_only_through_explicit_link() {
    let resources = vec![text_resource(
        &format!("{LOCAL_OBJECT}.3"),
        "Функция Измененная() Экспорт\nВозврат 2;\nКонецФункции",
    )];
    let resources = Resources::new(&resources).unwrap();
    let mut modules = vec![
        ConfigurationModule::new(
            format!("{BASE_OBJECT}.3"),
            "Справочник.Базовый.МодульМенеджера",
            "Возврат 1;",
            ModuleKind::Manager,
            ModuleExecution::default().with_server(true),
        )
        .with_owner(BASE_OBJECT),
    ];

    apply_extension_object(
        &mut modules,
        &base_objects_for_test(),
        &resources,
        CATALOG_CLASS,
        &extension_object(1, BASE_OBJECT),
        "Тестовое",
    )
    .unwrap();

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].id(), format!("{BASE_OBJECT}.3"));
    assert_eq!(modules[0].name(), "Справочник.Базовый.МодульМенеджера");
    assert!(modules[0].source().contains("Возврат 2"));
}

#[test]
fn later_extension_replaces_the_result_of_the_earlier_one() {
    let first_resources = vec![text_resource(&format!("{LOCAL_OBJECT}.3"), "Возврат 1;")];
    let second_resources = vec![text_resource(&format!("{LOCAL_OBJECT}.3"), "Возврат 2;")];
    let first = Resources::new(&first_resources).unwrap();
    let second = Resources::new(&second_resources).unwrap();
    let mut modules = Vec::new();
    let object = extension_object(1, BASE_OBJECT);

    apply_extension_object(
        &mut modules,
        &base_objects_for_test(),
        &first,
        CATALOG_CLASS,
        &object,
        "Первое",
    )
    .unwrap();
    apply_extension_object(
        &mut modules,
        &base_objects_for_test(),
        &second,
        CATALOG_CLASS,
        &object,
        "Второе",
    )
    .unwrap();

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].source(), "Возврат 2;");
}

#[test]
fn adopted_by_name_object_is_rejected_instead_of_being_guessed() {
    let resources = Resources::new(&[]).unwrap();
    let error = apply_extension_object(
        &mut Vec::new(),
        &base_objects_for_test(),
        &resources,
        CATALOG_CLASS,
        &extension_object(1, "00000000-0000-0000-0000-000000000000"),
        "ПоИмени",
    )
    .unwrap_err();

    assert!(matches!(
        error,
        LoadError::UnsupportedExtensionOperation { extension, .. }
            if extension == "ПоИмени"
    ));
}

#[test]
fn adopted_style_item_without_target_follows_the_measured_name_binding() {
    let resources = Resources::new(&[]).unwrap();
    let object = extension_object(1, "00000000-0000-0000-0000-000000000000");
    for identity in [BASE_OBJECT, LOCAL_OBJECT] {
        let mut base = HashMap::from([(
            identity.into(),
            BaseObject {
                class: STYLE_ITEM.into(),
                name: object.name.clone(),
            },
        )]);
        apply_extension_object(
            &mut Vec::new(),
            &base,
            &resources,
            STYLE_ITEM,
            &object,
            "ПоИмени",
        )
        .unwrap();
        base.get_mut(identity).unwrap().name = "Переименованный".into();
        assert!(matches!(
            apply_extension_object(
                &mut Vec::new(),
                &base,
                &resources,
                STYLE_ITEM,
                &object,
                "ПоИмени"
            ),
            Err(LoadError::UnsupportedExtensionOperation { .. })
        ));
        base.get_mut(identity).unwrap().name = object.name.to_lowercase();
        assert!(
            apply_extension_object(
                &mut Vec::new(),
                &base,
                &resources,
                STYLE_ITEM,
                &object,
                "ПоИмени"
            )
            .is_err()
        );
        base.get_mut(identity).unwrap().name = object.name.clone();
        base.get_mut(identity).unwrap().class = CATALOG_CLASS.into();
        assert!(
            apply_extension_object(
                &mut Vec::new(),
                &base,
                &resources,
                STYLE_ITEM,
                &object,
                "ПоИмени"
            )
            .is_err()
        );
    }
}

#[test]
fn an_explicit_style_target_never_falls_back_to_a_matching_name() {
    let resources = Resources::new(&[]).unwrap();
    let object = extension_object(1, BASE_OBJECT);
    let base = HashMap::from([(
        LOCAL_OBJECT.into(),
        BaseObject {
            class: STYLE_ITEM.into(),
            name: object.name.clone(),
        },
    )]);
    assert!(
        apply_extension_object(
            &mut Vec::new(),
            &base,
            &resources,
            STYLE_ITEM,
            &object,
            "ПоИдентификатору"
        )
        .is_err()
    );
}

#[test]
fn extension_session_module_is_rejected_instead_of_being_silently_ignored() {
    let catalog_source =
        "{2,{33333333-3333-3333-3333-333333333333},1,{0,{0,0,1,{$CLASS,1,$OBJECT}}}}"
            .replace("$CLASS", CATALOG_CLASS)
            .replace("$OBJECT", LOCAL_OBJECT);
    let catalog = parse_extension_catalog(catalog_source.as_bytes()).unwrap();
    let resources = vec![text_resource(
        &format!("{}.7", catalog.configuration),
        "Процедура УстановкаПараметровСеанса() КонецПроцедуры",
    )];
    let resources = Resources::new(&resources).unwrap();

    let error = reject_extension_session_module(&resources, &catalog, "Тестовое").unwrap_err();

    assert!(matches!(
        error,
        LoadError::UnsupportedExtensionOperation { extension, .. }
            if extension == "Тестовое"
    ));
}

#[test]
fn extension_catalog_object_without_its_header_is_not_projected() {
    let catalog_source =
        "{2,{33333333-3333-3333-3333-333333333333},1,{0,{0,0,1,{$CLASS,1,$OBJECT}}}}"
            .replace("$CLASS", CATALOG_CLASS)
            .replace("$OBJECT", LOCAL_OBJECT);
    let catalog = parse_extension_catalog(catalog_source.as_bytes()).unwrap();
    let code = bsl_config::ConfigurationCodeSnapshot::new(
        "44444444-4444-4444-4444-444444444444",
        Vec::new(),
    )
    .unwrap();

    let error =
        project_extension_metadata_catalog(&mut Vec::new(), &catalog, &[], &code, "Тестовое")
            .unwrap_err();

    assert!(matches!(
        error,
        LoadError::IncompleteExtensionStructure(extension)
            if extension == "Тестовое"
    ));
}

#[test]
fn own_extension_object_enters_the_effective_metadata_catalog() {
    let catalog_source =
        "{2,{33333333-3333-3333-3333-333333333333},1,{0,{0,0,1,{$CLASS,1,$OBJECT}}}}"
            .replace("$CLASS", CATALOG_CLASS)
            .replace("$OBJECT", LOCAL_OBJECT);
    let catalog = parse_extension_catalog(catalog_source.as_bytes()).unwrap();
    let object = extension_object(0, "00000000-0000-0000-0000-000000000000");
    let objects = [&object];
    let code = bsl_config::ConfigurationCodeSnapshot::new(
        "44444444-4444-4444-4444-444444444444",
        vec![
            ConfigurationModule::new(
                format!("{LOCAL_OBJECT}.3"),
                "Справочник.Локальный.МодульМенеджера",
                "Возврат 1;",
                ModuleKind::Manager,
                ModuleExecution::default().with_server(true),
            )
            .with_owner(LOCAL_OBJECT),
        ],
    )
    .unwrap();
    let mut projected = Vec::new();

    project_extension_metadata_catalog(&mut projected, &catalog, &objects, &code, "Тестовое")
        .unwrap();

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].id(), LOCAL_OBJECT);
    assert_eq!(projected[0].name(), "Локальный");
    assert_eq!(
        projected[0].manager_module(),
        Some("Справочник.Локальный.МодульМенеджера")
    );
}

#[test]
fn measured_legacy_classes_bind_by_exact_name_and_class() {
    let resources = Resources::new(&[]).unwrap();
    let object = extension_object(1, "00000000-0000-0000-0000-000000000000");
    for class in [
        crate::COMMON_PICTURE,
        crate::LANGUAGE,
        crate::ENUMERATION,
        CATALOG_CLASS,
        "c045099e-13b9-4fb6-9d50-fca00202971e",
        "2f1a5187-fb0e-4b05-9489-dc5dd6412348",
    ] {
        for id in [BASE_OBJECT, LOCAL_OBJECT] {
            let mut base = HashMap::from([(
                id.into(),
                BaseObject {
                    class: class.into(),
                    name: object.name.clone(),
                },
            )]);
            apply_extension_object(
                &mut Vec::new(),
                &base,
                &resources,
                class,
                &object,
                "Измеренное",
            )
            .unwrap();
            base.get_mut(id).unwrap().name = object.name.to_lowercase();
            assert!(
                apply_extension_object(
                    &mut Vec::new(),
                    &base,
                    &resources,
                    class,
                    &object,
                    "Измеренное"
                )
                .is_err()
            );
            base.get_mut(id).unwrap().name = "Переименованный".into();
            assert!(
                apply_extension_object(
                    &mut Vec::new(),
                    &base,
                    &resources,
                    class,
                    &object,
                    "Измеренное"
                )
                .is_err()
            );
        }
    }
}

#[test]
fn native_catalog_attributes_keep_own_order_and_exclude_tabular_fields() {
    let id = "92876c9e-52ff-480d-9878-5766f4f9d12c";
    let descriptors = open_sdbl::metadata::parse_config_descriptors(
        id,
        include_bytes!("../tests/fixtures/catalog-own-attributes/external-users.deflate"),
    )
    .unwrap();
    let owner = descriptors
        .iter()
        .find(|descriptor| descriptor.object_guid.as_str() == id)
        .unwrap();
    let rows = include_str!("../tests/fixtures/catalog-own-attributes/external-users.native.tsv")
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let own = rows
        .iter()
        .filter(|row| row[0] == "own")
        .collect::<Vec<_>>();
    assert_eq!(
        owner
            .catalog_attributes
            .as_ref()
            .unwrap()
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        own.iter().map(|row| row[1]).collect::<Vec<_>>()
    );
    let names = super::catalog_attribute_names(owner, &descriptors)
        .unwrap()
        .unwrap();
    assert_eq!(names, own.iter().map(|row| row[2]).collect::<Vec<_>>());
    for row in rows.iter().filter(|row| row[0] == "tabular") {
        assert!(!names.iter().any(|name| name == row[2]));
    }
    let object = bsl_orm::MetadataObject::new(id, &owner.name, bsl_orm::MetadataKind::Catalog)
        .with_catalog_attribute_names(names.clone())
        .with_catalog_hierarchical(owner.catalog_hierarchical.unwrap());
    assert_eq!(object.catalog_attribute_names(), Some(names.as_slice()));
    assert_eq!(owner.catalog_hierarchical, Some(false));
    assert_eq!(object.catalog_hierarchical(), Some(false));
    let mut unknown = owner.clone();
    unknown.catalog_attributes = None;
    assert!(
        super::catalog_attribute_names(&unknown, &descriptors)
            .unwrap()
            .is_none()
    );
    let mut empty = owner.clone();
    empty.catalog_attributes = Some(vec![]);
    assert_eq!(
        super::catalog_attribute_names(&empty, &descriptors).unwrap(),
        Some(vec![])
    );
    let mut incomplete = descriptors.clone();
    incomplete.retain(|descriptor| {
        descriptor.object_guid.as_str() != "d65c4485-7bac-4206-9a6c-7156f4bb876d"
    });
    assert!(super::catalog_attribute_names(owner, &incomplete).is_err());
}

#[test]
fn native_catalog_sections_exclude_nested_row_fields() {
    let id = "a2782760-dd4c-44d2-a785-814d6f610455";
    let descriptors = open_sdbl::metadata::parse_config_descriptors(
        id,
        include_bytes!("../tests/fixtures/catalog-tabular-sections/user-groups.deflate"),
    )
    .unwrap();
    let owner = descriptors
        .iter()
        .find(|descriptor| descriptor.object_guid.as_str() == id)
        .unwrap();
    let rows = include_str!("../tests/fixtures/catalog-tabular-sections/user-groups.native.tsv")
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let sections = rows
        .iter()
        .filter(|row| row[0] == "section")
        .collect::<Vec<_>>();
    assert_eq!(
        owner
            .catalog_tabular_sections
            .as_ref()
            .unwrap()
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        sections.iter().map(|row| row[1]).collect::<Vec<_>>()
    );
    let names = super::catalog_tabular_section_names(owner, &descriptors)
        .unwrap()
        .unwrap();
    assert_eq!(names, sections.iter().map(|row| row[2]).collect::<Vec<_>>());
    for field in rows.iter().filter(|row| row[0] == "field") {
        assert!(!names.iter().any(|name| name == field[2]));
    }
    let object = bsl_orm::MetadataObject::new(id, &owner.name, bsl_orm::MetadataKind::Catalog)
        .with_catalog_tabular_section_names(names.clone());
    assert_eq!(
        object.catalog_tabular_section_names(),
        Some(names.as_slice())
    );
    let mut unknown = owner.clone();
    unknown.catalog_tabular_sections = None;
    assert_eq!(
        super::catalog_tabular_section_names(&unknown, &descriptors).unwrap(),
        None
    );
    unknown.catalog_tabular_sections = Some(vec![]);
    assert_eq!(
        super::catalog_tabular_section_names(&unknown, &descriptors).unwrap(),
        Some(vec![])
    );
    let incomplete = descriptors
        .iter()
        .filter(|descriptor| {
            descriptor.object_guid.as_str() != "6581ac1f-11ef-42b8-9f3b-590bb7546e63"
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(super::catalog_tabular_section_names(owner, &incomplete).is_err());
}

#[test]
fn native_document_number_settings_project_all_measured_variants() {
    use bsl_orm::{DocumentNumberSettings, DocumentNumberType};
    for (id, bytes, number_type, number_length) in [
        (
            "4dbedad1-342b-54c1-9bcf-1d79091376ef",
            include_bytes!("../tests/fixtures/document-number-settings/ProbeString.deflate")
                .as_slice(),
            DocumentNumberType::String,
            9,
        ),
        (
            "c96786e5-f0b2-51f7-85a5-5d3bcbff2410",
            include_bytes!("../tests/fixtures/document-number-settings/ProbeNumeric.deflate")
                .as_slice(),
            DocumentNumberType::Number,
            9,
        ),
        (
            "6080fb5a-cf3e-599a-a005-37de07820830",
            include_bytes!("../tests/fixtures/document-number-settings/ProbeDisabled.deflate")
                .as_slice(),
            DocumentNumberType::String,
            0,
        ),
    ] {
        let descriptors = open_sdbl::metadata::parse_config_descriptors(id, bytes).unwrap();
        let owner = descriptors
            .iter()
            .find(|entry| entry.object_guid.as_str() == id)
            .unwrap();
        assert_eq!(
            super::document_number_settings(owner.document_number.unwrap()),
            Some(DocumentNumberSettings {
                number_type,
                number_length
            })
        );
    }
}

#[test]
fn acquired_basis_reference_guids_match_every_native_owner() {
    use open_sdbl::metadata::{DbNames, SchemaStorage, parse_config_descriptors, resolve_metadata};
    let mut descriptors = parse_config_descriptors(
        "571e1bd9-0f64-4a05-a276-dba4fe242ad0",
        include_bytes!("../tests/fixtures/document-owned-attributes/event.deflate"),
    )
    .unwrap();
    descriptors.extend(
        parse_config_descriptors(
            "51c403a3-e062-4f0d-b39c-11989c0ffe2b",
            include_bytes!("../tests/fixtures/document-owned-attributes/basis.deflate"),
        )
        .unwrap(),
    );
    let snapshot = resolve_metadata(
        DbNames::from_entries(vec![]),
        descriptors,
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let types = snapshot
        .descriptors()
        .iter()
        .find(|entry| entry.object_guid.as_str() == "e350fdab-d2d1-45e8-bd75-a2ee410245f1")
        .unwrap()
        .declared_types
        .as_ref()
        .unwrap();
    let expected: Vec<_> =
        include_str!("../../../docs/research/unf-event-basis-reference-identities.tsv")
            .lines()
            .skip(1)
            .map(|line| line.split('\t').nth(2).unwrap())
            .collect();
    assert_eq!(types.len(), 56);
    assert_eq!(expected.len(), 56);
    for (actual, expected) in types.iter().zip(expected) {
        assert_eq!(actual.reference.as_ref().unwrap().as_str(), expected);
    }
}

#[path = "global_separation_condition_tests.rs"]
mod global_separation_condition_tests;

#[cfg(test)]
#[path = "own_extension_policy_tests.rs"]
mod own_extension_policy;

#[cfg(test)]
#[path = "own_catalog_metadata_tests.rs"]
mod own_catalog_metadata;
