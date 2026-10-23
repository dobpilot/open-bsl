use super::*;
use open_bsl::{
    ConfigurationArtifact, ConfigurationArtifacts, ConfigurationDescription, ConfigurationIndex,
    ConfigurationModuleDeclaration, ConfigurationProperty, ConfigurationSource, ModuleExecution,
    ModuleKind,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn leaf(name: &str, value: &str) -> ConfigurationProperty {
    ConfigurationProperty {
        name: name.into(),
        value: value.into(),
        attributes: Arc::from([]),
        children: Arc::from([]),
    }
}
fn branch(name: &str, children: Vec<ConfigurationProperty>) -> ConfigurationProperty {
    let mut node = leaf(name, "");
    node.children = children.into();
    node
}
struct Source {
    index: ConfigurationIndex,
    declarations: Vec<ConfigurationDescription>,
    descriptions: HashMap<String, Arc<ConfigurationProperty>>,
    reads: AtomicUsize,
}
impl ConfigurationSource for Source {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }
    fn artifacts(&self) -> Option<&dyn ConfigurationArtifacts> {
        Some(self)
    }
    fn module_source(&self, _: &str) -> Result<Arc<str>, String> {
        panic!("metadata access must not read BSL")
    }
}
impl ConfigurationArtifacts for Source {
    fn root_identity(&self) -> &str {
        "root"
    }
    fn descriptions(&self) -> &[ConfigurationDescription] {
        &self.declarations
    }
    fn description(&self, id: &str) -> Result<Option<Arc<ConfigurationProperty>>, String> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.descriptions.get(id).cloned())
    }
    fn artifact(&self, _: &str) -> Result<Option<Arc<ConfigurationArtifact>>, String> {
        panic!("metadata access must not acquire artifacts")
    }
}
fn source(field: &str, ty: &str) -> Arc<Source> {
    let description =
        |id: &str, owner: Option<&str>, kind: &str, name: &str| ConfigurationDescription {
            id: id.into(),
            owner: owner.map(Arc::from),
            kind: kind.into(),
            name: name.into(),
        };
    Arc::new(Source {
        index: ConfigurationIndex::new(
            "file-root",
            vec![
                ConfigurationModuleDeclaration::new(
                    "owner/Manager",
                    "Справочник.Проба.МодульМенеджера",
                    ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("owner"),
            ],
        )
        .unwrap(),
        declarations: vec![
            description("root", None, "Configuration", "ФайловаяКонфигурация"),
            description("owner", None, "Catalog", "Проба"),
            description("form", Some("owner"), "Form", "ФормаПроба"),
        ],
        descriptions: [
            (
                "root".into(),
                Arc::new(branch(
                    "description",
                    vec![
                        leaf("name", "ФайловаяКонфигурация"),
                        leaf("version", "local"),
                    ],
                )),
            ),
            (
                "owner".into(),
                Arc::new(branch(
                    "description",
                    vec![
                        leaf("name", "Проба"),
                        branch(
                            "attributes",
                            vec![branch(
                                "attribute",
                                vec![
                                    leaf("name", field),
                                    leaf("uuid", field),
                                    branch("type", vec![leaf("types", ty)]),
                                ],
                            )],
                        ),
                    ],
                )),
            ),
            (
                "form".into(),
                Arc::new(branch(
                    "description",
                    vec![
                        leaf("name", "ФормаПроба"),
                        branch(
                            "attributes",
                            vec![branch(
                                "attribute",
                                vec![
                                    leaf("name", "ЛокальныйРеквизит"),
                                    branch("type", vec![leaf("types", "String")]),
                                ],
                            )],
                        ),
                    ],
                )),
            ),
        ]
        .into_iter()
        .collect(),
        reads: AtomicUsize::new(0),
    })
}
fn catalog() -> MetadataCatalog {
    MetadataCatalog::new(
        "database-namespace",
        vec![
            MetadataObject::new("owner", "Проба", MetadataKind::Catalog)
                .with_catalog_attributes(vec![DocumentAttribute::new(
                    "Поле",
                    "Поле",
                    vec![open_bsl::ColumnType::plain(TypeRef::Native(
                        open_bsl::TypeId::Boolean,
                    ))],
                )])
                .with_manager_module("СтарыйМенеджер"),
        ],
    )
    .unwrap()
    .with_configuration_object_id("root")
    .unwrap()
}
#[test]
fn source_metadata_is_lazy_and_nonpersistent_form_attributes_do_not_require_db_fields() {
    let source = source("Поле", "Boolean");
    let catalog = Arc::new(catalog().with_file_source(source.clone()).unwrap());
    assert_eq!(source.reads.load(Ordering::Relaxed), 0);
    assert_eq!(
        catalog.object_by_id("owner").unwrap().manager_module(),
        Some("Справочник.Проба.МодульМенеджера")
    );
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, open_bsl::format_value);
    let forms = file_source::collection(&catalog, Some("owner"), "Form")
        .unwrap()
        .unwrap();
    assert_eq!(source.reads.load(Ordering::Relaxed), 0);
    let form = forms
        .object_ref()
        .unwrap()
        .get_property("фОрМаПрОбА", &mut context)
        .unwrap();
    let attributes = form
        .object_ref()
        .unwrap()
        .get_property("рЕкВиЗиТы", &mut context)
        .unwrap();
    let attribute = attributes
        .object_ref()
        .unwrap()
        .get_property("ЛокальныйРеквизит", &mut context)
        .unwrap();
    assert_eq!(
        attribute
            .object_ref()
            .unwrap()
            .get_property("Имя", &mut context)
            .unwrap(),
        Value::Str("ЛокальныйРеквизит".into())
    );
    let _ty = attribute
        .object_ref()
        .unwrap()
        .get_property("Тип", &mut context)
        .unwrap();
    assert_eq!(source.reads.load(Ordering::Relaxed), 1);
    catalog.materialized_object_by_id("owner").unwrap();
    catalog.materialized_object_by_id("owner").unwrap();
    assert_eq!(source.reads.load(Ordering::Relaxed), 2);
}
#[test]
fn persistent_field_and_type_mismatches_are_errors() {
    for source in [source("ДругоеПоле", "Boolean"), source("Поле", "String")] {
        let catalog = catalog().with_file_source(source).unwrap();
        assert!(catalog.materialized_object_by_id("owner").is_err());
    }
    let mut wrong = catalog();
    wrong.configuration_object_id = Some("other-root".into());
    assert!(wrong.with_file_source(source("Поле", "Boolean")).is_err());
}

#[test]
fn file_form_types_are_visible_to_bsl_and_root_enums_do_not_fall_back_to_db() {
    let mut source = source("Поле", "Boolean");
    let properties = Arc::get_mut(&mut source)
        .unwrap()
        .descriptions
        .get_mut("root")
        .unwrap();
    let mut children = properties.children.to_vec();
    children.extend([
        leaf("scriptVariant", "English"),
        leaf("compatibilityMode", "8.3.12"),
    ]);
    Arc::make_mut(properties).children = children.into();
    let catalog = Arc::new(catalog().with_file_source(source).unwrap());
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, open_bsl::format_value);
    let root = MetadataRoot {
        catalog: catalog.clone(),
    };
    assert_eq!(
        root.get_property("ВариантВстроенногоЯзыка", &mut context)
            .unwrap(),
        Value::Enum(EnumValue::ScriptVariantEnglish)
    );
    assert_eq!(
        root.get_property("CompatibilityMode", &mut context)
            .unwrap(),
        Value::Enum(EnumValue::CompatibilityModeVersion8_3_12)
    );
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Пользователь", vec![]),
        )))
        .build();
    assert_eq!(state.exec("Возврат Метаданные.Справочники.Проба.Формы.ФормаПроба.Реквизиты.ЛокальныйРеквизит.Тип.СодержитТип(Тип(\"Строка\"));").unwrap(), Value::Boolean(true));
}

#[test]
fn native_common_module_boolean_flags_keep_their_types_and_omitted_false_values() {
    let mut source = source("Поле", "Boolean");
    let data = Arc::get_mut(&mut source).unwrap();
    data.declarations.push(ConfigurationDescription {
        id: "common".into(),
        owner: None,
        kind: "CommonModule".into(),
        name: "Общий".into(),
    });
    data.descriptions.insert(
        "common".into(),
        Arc::new(branch(
            "description",
            vec![
                leaf("name", "Общий"),
                leaf("clientManagedApplication", "true"),
            ],
        )),
    );
    let catalog = Arc::new(catalog().with_file_source(source).unwrap());
    let collection = file_source::collection(&catalog, None, "CommonModule")
        .unwrap()
        .unwrap();
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, open_bsl::format_value);
    let module = collection
        .object_ref()
        .unwrap()
        .get_property("Общий", &mut context)
        .unwrap();
    assert_eq!(
        module
            .object_ref()
            .unwrap()
            .get_property("Сервер", &mut context)
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        module
            .object_ref()
            .unwrap()
            .get_property("КлиентУправляемоеПриложение", &mut context)
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn a_constant_type_disagreement_is_rejected_before_live_value_acquisition() {
    for (ty, compatible) in [("Number", true), ("String", false)] {
        let mut source = source("Поле", "Boolean");
        let data = Arc::get_mut(&mut source).unwrap();
        data.declarations.push(ConfigurationDescription {
            id: "constant".into(),
            owner: None,
            kind: "Constant".into(),
            name: "Значение".into(),
        });
        data.descriptions.insert(
            "constant".into(),
            Arc::new(branch(
                "description",
                vec![
                    leaf("name", "Значение"),
                    branch("type", vec![leaf("types", ty)]),
                ],
            )),
        );
        let catalog = MetadataCatalog::new(
            "database-namespace",
            vec![
                MetadataObject::new("owner", "Проба", MetadataKind::Catalog),
                MetadataObject::new("constant", "Значение", MetadataKind::Constant)
                    .with_declared_types(vec![open_bsl::ColumnType::plain(TypeRef::Native(
                        open_bsl::TypeId::Number,
                    ))]),
            ],
        )
        .unwrap()
        .with_configuration_object_id("root")
        .unwrap()
        .with_file_source(source)
        .unwrap();
        assert_eq!(
            catalog.materialized_object_by_id("constant").is_ok(),
            compatible
        );
    }
}

#[test]
fn local_role_descriptions_do_not_grant_database_principal_roles() {
    let mut source = source("Поле", "Boolean");
    let data = Arc::get_mut(&mut source).unwrap();
    data.declarations.push(ConfigurationDescription {
        id: "role".into(),
        owner: None,
        kind: "Role".into(),
        name: "РольПробы".into(),
    });
    data.descriptions.insert(
        "role".into(),
        Arc::new(branch(
            "description",
            vec![
                leaf("name", "РольПробы"),
                leaf("comment", "Локальная редакция роли не даёт прав"),
            ],
        )),
    );
    let catalog = Arc::new(
        MetadataCatalog::new(
            "database-namespace",
            vec![
                MetadataObject::new("owner", "Проба", MetadataKind::Catalog),
                MetadataObject::new("role", "РольПробы", MetadataKind::Role),
            ],
        )
        .unwrap()
        .with_configuration_object_id("root")
        .unwrap()
        .with_file_source(source)
        .unwrap(),
    );
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Пользователь", vec![]),
        )))
        .build();
    assert_eq!(
        state.exec("Возврат РольДоступна(\"РольПробы\");").unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Роли.РольПробы.Комментарий;")
            .unwrap(),
        Value::Str("Локальная редакция роли не даёт прав".into())
    );
}

#[test]
fn known_empty_form_collections_are_collections_rather_than_empty_strings() {
    let mut source = source("Поле", "Boolean");
    Arc::get_mut(&mut source).unwrap().descriptions.insert(
        "form".into(),
        Arc::new(branch(
            "description",
            vec![
                leaf("name", "ФормаПроба"),
                branch("attributes", vec![]),
                branch("commands", vec![]),
            ],
        )),
    );
    let catalog = Arc::new(catalog().with_file_source(source).unwrap());
    let collection = file_source::collection(&catalog, Some("owner"), "Form")
        .unwrap()
        .unwrap();
    let mut shapes = open_bsl::RuntimeShapes::seeded(vec![], vec![], None);
    let mut context = CallContext::minimal(&mut shapes, open_bsl::format_value);
    let form = collection
        .object_ref()
        .unwrap()
        .get_property("ФормаПроба", &mut context)
        .unwrap();
    for name in ["Реквизиты", "Команды"] {
        let values = form
            .object_ref()
            .unwrap()
            .get_property(name, &mut context)
            .unwrap();
        assert_eq!(values.object_ref().unwrap().collection_len().unwrap(), 0);
    }
}

#[test]
fn exchange_plan_fields_require_the_acquired_owner_membership_and_types() {
    let mut source = source("Поле", "Boolean");
    Arc::get_mut(&mut source).unwrap().declarations[1].kind = "ExchangePlan".into();
    let field = DocumentAttribute::new(
        "Поле",
        "Поле",
        vec![open_bsl::ColumnType::plain(TypeRef::Native(
            open_bsl::TypeId::Boolean,
        ))],
    );
    for (attributes, valid) in [
        (None, false),
        (Some(Vec::new()), false),
        (Some(vec![field]), true),
    ] {
        let catalog = MetadataCatalog::new(
            "database-namespace",
            vec![MetadataObject::new(
                "owner",
                "Проба",
                MetadataKind::ExchangePlan,
            )],
        )
        .unwrap()
        .with_configuration_object_id("root")
        .unwrap()
        .with_exchange_plan_attributes("owner", attributes)
        .unwrap()
        .with_file_source(source.clone())
        .unwrap();
        let result = file_source::validate_object(
            catalog.file_source.as_ref().unwrap(),
            catalog.object_by_id("owner").unwrap(),
        );
        assert_eq!(result.is_ok(), valid);
    }
}
