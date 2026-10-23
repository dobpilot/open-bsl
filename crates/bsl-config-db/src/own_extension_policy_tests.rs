use super::*;
use bsl_config::ConfigurationCodeSnapshot;
use bsl_orm::{MetadataKind, MetadataObject};

#[test]
fn own_source_policy_reaches_binding_and_replacement_clears_it() {
    for safe in [false, true] {
        let mut modules = Vec::new();
        let raw = vec![text_resource(&format!("{LOCAL_OBJECT}.0"), "")];
        let resources = Resources::new(&raw).unwrap();
        apply_extension_object(
            &mut modules,
            &base_objects_for_test(),
            &resources,
            CATALOG_CLASS,
            &extension_object(0, BASE_OBJECT),
            "Собственное",
        )
        .unwrap();
        crate::stamp_own_extension_policy(&mut modules, LOCAL_OBJECT, safe);
        let code = ConfigurationCodeSnapshot::new("own-policy", modules.clone()).unwrap();
        let object = crate::bind_object_module(
            MetadataObject::new(LOCAL_OBJECT, "Проба", MetadataKind::Catalog),
            &code,
        );
        assert_eq!(object.own_extension_object_module_safe_mode(), Some(safe));
        assert!(object.object_module_is_extension());
        assert_eq!(
            object
                .clone()
                .with_object_module("База")
                .own_extension_object_module_safe_mode(),
            None
        );
        assert_eq!(
            object
                .with_extension_object_module("Заимствованный")
                .own_extension_object_module_safe_mode(),
            None
        );

        // Новое расширение заимствует тот же объект без своего текста.
        let bases = HashMap::from([(
            LOCAL_OBJECT.into(),
            BaseObject {
                class: CATALOG_CLASS.into(),
                name: "Проба".into(),
            },
        )]);
        apply_extension_object(
            &mut modules,
            &bases,
            &Resources::new(&[]).unwrap(),
            CATALOG_CLASS,
            &extension_object(1, LOCAL_OBJECT),
            "БезЗамены",
        )
        .unwrap();
        assert_eq!(modules[0].own_extension_safe_mode(), Some(safe));

        apply_extension_object(
            &mut modules,
            &bases,
            &resources,
            CATALOG_CLASS,
            &extension_object(1, LOCAL_OBJECT),
            "Замена",
        )
        .unwrap();
        assert!(modules[0].source_is_extension());
        assert_eq!(modules[0].own_extension_safe_mode(), None);
        let code = ConfigurationCodeSnapshot::new("own-policy", modules).unwrap();
        let object = crate::bind_object_module(
            MetadataObject::new(LOCAL_OBJECT, "Проба", MetadataKind::Catalog),
            &code,
        );
        assert_eq!(object.own_extension_object_module_safe_mode(), None);
    }
}

#[test]
fn stamping_policy_does_not_change_another_owner_or_a_base_source() {
    let base = ConfigurationModule::new(
        "base",
        "База",
        "",
        ModuleKind::Object,
        ModuleExecution::default(),
    )
    .with_owner(LOCAL_OBJECT);
    let other = ConfigurationModule::new(
        "other",
        "Другой",
        "",
        ModuleKind::Object,
        ModuleExecution::default(),
    )
    .with_owner(BASE_OBJECT)
    .with_extension_source(true);
    let mut modules = vec![base.clone(), other.clone()];
    crate::stamp_own_extension_policy(&mut modules, LOCAL_OBJECT, true);
    assert_eq!(modules, [base, other]);
    let own = modules[1].clone().with_own_extension_source(false);
    assert_eq!(
        own.clone()
            .with_extension_source(true)
            .own_extension_safe_mode(),
        None
    );
    assert_eq!(
        own.with_extension_source(false).own_extension_safe_mode(),
        None
    );
}
