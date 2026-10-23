use super::super::bind_object_module;
use super::*;
use bsl_config::ConfigurationCodeSnapshot;
use bsl_orm::{MetadataKind, MetadataObject};

#[test]
fn final_object_source_origin_survives_extension_replacement_and_orm_binding() {
    for (belonging, target) in [(0, LOCAL_OBJECT), (1, BASE_OBJECT)] {
        let raw = vec![text_resource(
            &format!("{LOCAL_OBJECT}.0"),
            "Процедура ПриУстановкеНовогоКода(СтандартнаяОбработка, Префикс) КонецПроцедуры",
        )];
        let resources = Resources::new(&raw).unwrap();
        let mut modules = vec![
            ConfigurationModule::new(
                format!("{BASE_OBJECT}.0"),
                "Справочник.Базовый.МодульОбъекта",
                "",
                ModuleKind::Object,
                ModuleExecution::default().with_server(true),
            )
            .with_owner(BASE_OBJECT),
        ];
        assert!(!modules[0].source_is_extension());
        apply_extension_object(
            &mut modules,
            &base_objects_for_test(),
            &resources,
            CATALOG_CLASS,
            &extension_object(belonging, BASE_OBJECT),
            "Тестовое",
        )
        .unwrap();
        let code = ConfigurationCodeSnapshot::new("origin", modules).unwrap();
        let module = code
            .modules()
            .iter()
            .find(|m| m.owner() == Some(target))
            .unwrap();
        assert!(module.source_is_extension());
        let object = bind_object_module(
            MetadataObject::new(target, "Проба", MetadataKind::Catalog),
            &code,
        );
        assert!(object.object_module_is_extension());
        assert_eq!(object.object_module(), Some(module.name()));
        let reset = object.with_object_module("Основной");
        assert!(!reset.object_module_is_extension());
    }
}

#[test]
fn absent_extension_source_keeps_the_base_origin() {
    let raw = vec![];
    let resources = Resources::new(&raw).unwrap();
    let base = ConfigurationModule::new(
        format!("{BASE_OBJECT}.0"),
        "Справочник.Базовый.МодульОбъекта",
        "Возврат;",
        ModuleKind::Object,
        ModuleExecution::default().with_server(true),
    )
    .with_owner(BASE_OBJECT);
    let mut modules = vec![base.clone()];
    apply_extension_object(
        &mut modules,
        &base_objects_for_test(),
        &resources,
        CATALOG_CLASS,
        &extension_object(1, BASE_OBJECT),
        "Тестовое",
    )
    .unwrap();
    assert_eq!(modules, vec![base]);
    let code = ConfigurationCodeSnapshot::new("origin", modules).unwrap();
    let object = bind_object_module(
        MetadataObject::new(BASE_OBJECT, "Базовый", MetadataKind::Catalog),
        &code,
    );
    assert!(!object.object_module_is_extension());
}
