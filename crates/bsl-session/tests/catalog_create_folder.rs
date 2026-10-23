use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogHierarchyType, CatalogStandardAttributes,
    MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session(hierarchical: Option<bool>, kind: Option<CatalogHierarchyType>) -> Session {
    session_with_manager(hierarchical, kind, "")
}

fn session_with_manager(
    hierarchical: Option<bool>,
    kind: Option<CatalogHierarchyType>,
    manager_source: &str,
) -> Session {
    let module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_object_module(module)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription);
    if let Some(flag) = hierarchical {
        object = object.with_catalog_hierarchical(flag);
    }
    if let Some(kind) = kind {
        object = object.with_catalog_hierarchy_type(kind);
    }
    let manager = "Справочник.ЗамерВременныхТаблиц.МодульМенеджера";
    if !manager_source.is_empty() {
        object = object.with_manager_module(manager);
    }
    let catalog = MetadataCatalog::new("create-folder", vec![object]).unwrap();
    let mut modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "object",
            module,
            include_str!("../../../tests/conformance/measure/1c/catalog-folder-flag-module.bsl"),
            ModuleKind::Object,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("catalog"),
    ];
    if !manager_source.is_empty() {
        modules.push(
            ConfigurationModule::new(
                "manager",
                manager,
                manager_source,
                ModuleKind::Manager,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog"),
        );
    }
    let code = ConfigurationCodeSnapshot::new("create-folder", modules).unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1),
    )
    .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source}\n Возврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn folder_creation_matches_native_body_and_returned_flags() {
    for (flag, source, expected) in [
        (
            true,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-catalog-create-folder-hierarchical.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-catalog-create-folder-hierarchical.platform.txt"
            ),
        ),
        (
            false,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-catalog-create-folder-flat.bsl"
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-catalog-create-folder-flat.platform.txt"
            ),
        ),
    ] {
        assert_eq!(
            replay(
                &mut session(Some(flag), Some(CatalogHierarchyType::FoldersAndItems)),
                source
            ),
            expected.trim()
        );
    }
}

#[test]
fn item_only_kind_rejects_folders_with_either_flag() {
    for flag in [false, true] {
        let mut session = session(Some(flag), Some(CatalogHierarchyType::Items));
        let module = session.instantiate("Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьГруппу(); КонецФункции").unwrap();
        let error = session.call(&module, "Проба", vec![]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("справочник не может содержать группы"),
            "{error}"
        );
        assert_eq!(
            replay(
                &mut session,
                "Сообщить(?(Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().ЭтоГруппа, \"folder\", \"item\"));"
            ),
            "item"
        );
    }
}

#[test]
fn unknown_folder_settings_do_not_change_item_creation() {
    for (flag, kind) in [
        (None, Some(CatalogHierarchyType::FoldersAndItems)),
        (Some(true), None),
    ] {
        let mut session = session(flag, kind);
        let module = session.instantiate("Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.CreateFolder(); КонецФункции").unwrap();
        let error = session.call(&module, "Проба", vec![]).unwrap_err();
        assert!(error.to_string().contains("неизвестен в снимке"), "{error}");
        assert_eq!(
            replay(
                &mut session,
                "Сообщить(?(Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().ЭтоГруппа, \"folder\", \"item\"));"
            ),
            "item"
        );
    }
}

#[test]
fn english_folder_alias_preserves_distinct_new_object_flags() {
    let mut session = session(Some(true), Some(CatalogHierarchyType::FoldersAndItems));
    assert_eq!(
        replay(
            &mut session,
            "Группа = Справочники.ЗамерВременныхТаблиц.createfolder(); Элемент = Справочники.ЗамерВременныхТаблиц.CreateItem(); Сообщить(?(Группа.IsFolder, \"folder\", \"item\")); Сообщить(?(Элемент.IsFolder, \"folder\", \"item\"));"
        ),
        "folder\nitem"
    );
    let result = session.instantiate("Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.CreateFolder(1); КонецФункции");
    if let Ok(module) = result {
        assert!(session.call(&module, "Проба", vec![]).is_err());
    }
}

#[test]
fn own_manager_context_resolves_bare_folder_factory() {
    let mut session = session_with_manager(
        Some(true),
        Some(CatalogHierarchyType::FoldersAndItems),
        "Функция ПробаГруппы() Экспорт Возврат СоздатьГруппу().ЭтоГруппа; КонецФункции",
    );
    assert_eq!(
        replay(
            &mut session,
            "Сообщить(?(Справочники.ЗамерВременныхТаблиц.ПробаГруппы(), \"folder\", \"item\"));"
        ),
        "folder"
    );
}

#[test]
fn new_factory_does_not_replace_an_existing_manager_export_or_its_arity() {
    let mut session = session_with_manager(
        Some(true),
        Some(CatalogHierarchyType::FoldersAndItems),
        "Функция CreateFolder(Значение) Экспорт Возврат Значение; КонецФункции",
    );
    assert_eq!(
        replay(
            &mut session,
            "Сообщить(Справочники.ЗамерВременныхТаблиц.CreateFolder(77));"
        ),
        "77"
    );
}
