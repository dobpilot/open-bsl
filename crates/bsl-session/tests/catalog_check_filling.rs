use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributeFillChecking, CatalogStandardAttributes,
    DocumentAttribute, FillChecking, MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};

fn session(body: &str) -> Session {
    session_with_settings(body, true)
}

fn session_with_settings(body: &str, known: bool) -> Session {
    session_with_attributes(body, known, vec![])
}

fn session_with_attributes(body: &str, known: bool, attributes: Vec<DocumentAttribute>) -> Session {
    session_with_fields(body, known, attributes, vec![])
}

fn session_with_fields(
    body: &str,
    known: bool,
    attributes: Vec<DocumentAttribute>,
    common: Vec<DocumentAttribute>,
) -> Session {
    let object_module = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let mut object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_catalog_attribute_names(
            attributes
                .iter()
                .map(|attribute| attribute.name().to_owned()),
        )
        .with_catalog_attributes(attributes)
        .with_catalog_tabular_section_names(Vec::<String>::new())
        .with_catalog_common_attributes(common)
        .with_object_module(object_module);
    if known {
        object = object.with_catalog_standard_fill_checking(vec![
            CatalogStandardAttributeFillChecking {
                name: "Code".into(),
                fill_checking: FillChecking::DontCheck,
            },
            CatalogStandardAttributeFillChecking {
                name: "Description".into(),
                fill_checking: FillChecking::ShowError,
            },
        ]);
    }
    let code = ConfigurationCodeSnapshot::new(
        "filling",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                object_module,
                body,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("catalog"),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(
            MetadataCatalog::new(
                "filling",
                vec![
                    object,
                    MetadataObject::new("language-en", "Английский", MetadataKind::Language)
                        .with_string_property("КодЯзыка", "en"),
                ],
            )
            .unwrap()
            .with_configuration(
                "Проба",
                "1.0",
                bsl_orm::ScriptVariant::English,
                bsl_orm::CompatibilityMode::DontUse,
                Vec::<String>::new(),
            )
            .unwrap()
            .with_primary_language("language-en")
            .unwrap(),
        ),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
    .open(
        Principal::new("user", "Тест", vec![]),
        SessionResources::new(Box::new(()), 1).with_system_language("en"),
    )
    .unwrap()
}

fn replay(body: &str, source: &str) -> String {
    let mut session = session(body);
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let (helpers, source) = if let Some(end) = source.find("КонецФункции") {
        source.split_at(end + "КонецФункции".len())
    } else {
        ("", source.as_str())
    };
    let module = session
        .instantiate(&format!(
            "{helpers}\n Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source}\n Возврат РезультатыПробы; КонецФункции"
        ))
        .unwrap();
    let Value::Object(value) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(value) = value.as_ref() else {
        panic!("массив результатов");
    };
    value
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn catalog_checks_match_native_initialization_fields_alias_and_arity() {
    assert_eq!(
        replay(
            include_str!("../../../tests/conformance/measure/1c/catalog-check-filling-module.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-catalog-check-filling.bsl"),
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-catalog-check-filling.platform.txt"
        )
        .trim(),
    );
}

#[test]
fn catalog_check_handler_matches_native_array_identity_and_paths() {
    assert_eq!(
        replay(
            include_str!("../../../tests/conformance/measure/1c/catalog-check-filling-handler-module.bsl"),
            include_str!("../../../tests/conformance/measure/1c/measure-catalog-check-filling-handler.bsl"),
        ),
        include_str!("../../../tests/conformance/measure/1c/measure-catalog-check-filling-handler.platform.txt")
            .trim(),
    );
}

#[test]
fn unknown_filling_settings_do_not_become_successful_validation() {
    let mut session = session_with_settings("", false);
    let module = session
        .instantiate("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().ПроверитьЗаполнение(); КонецФункции")
        .unwrap();
    let error = session.call(&module, "Проверить", vec![]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("настройки проверки стандартных реквизитов неизвестны")
    );
}

#[test]
fn existing_object_export_keeps_its_arguments_and_priority() {
    let mut session =
        session("Функция CheckFilling(Значение) Экспорт Возврат Значение; КонецФункции");
    let module = session
        .instantiate("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().CheckFilling(77); КонецФункции")
        .unwrap();
    assert_eq!(
        format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
        "77"
    );
}

#[test]
fn native_own_string_value_is_typed_and_belongs_to_its_object() {
    let attribute = DocumentAttribute::new(
        "own",
        "ПроверяемаяСтрока",
        vec![open_bsl::ColumnType {
            id: open_bsl::TypeRef::Native(open_bsl::TypeId::String),
            quals: vec!["30".into(), "1".into()],
        }],
    )
    .with_fill_checking(FillChecking::ShowError);
    let mut session = session_with_attributes("", true, vec![attribute]);
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            Первый = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Второй = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Первый.Наименование = "filled";
            Первый.ПроверяемаяСтрока = "test";
            Возврат Первый.ПроверитьЗаполнение() И Первый.ПроверяемаяСтрока = "test"
                И Второй.ПроверяемаяСтрока = "";
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn own_string_initialization_read_and_write_match_native_object_body() {
    let attribute = DocumentAttribute::new(
        "own",
        "ПроверяемаяСтрока",
        vec![open_bsl::ColumnType {
            id: open_bsl::TypeRef::Native(open_bsl::TypeId::String),
            quals: vec!["30".into(), "1".into()],
        }],
    )
    .with_fill_checking(FillChecking::ShowError);
    let mut session = session_with_attributes(
        include_str!("../../../tests/conformance/measure/1c/catalog-check-filling-own-module.bsl"),
        true,
        vec![attribute],
    );
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент().ПрочитатьИнициализациюРеквизита(); КонецФункции").unwrap();
    let expected = include_str!("../../../tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym-ru.platform.txt")
        .lines().find_map(|line| line.strip_prefix("catalog.own.body=")).unwrap();
    assert_eq!(
        format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
        expected
    );
}

#[test]
fn applied_common_check_follows_native_own_field_order() {
    let attribute = |id: &str, name: &str, length: &str| {
        DocumentAttribute::new(
            id,
            name,
            vec![open_bsl::ColumnType {
                id: open_bsl::TypeRef::Native(open_bsl::TypeId::String),
                quals: vec![length.into(), "1".into()],
            }],
        )
        .with_fill_checking(FillChecking::ShowError)
    };
    let mut session = session_with_fields(
        include_str!(
            "../../../tests/conformance/measure/1c/catalog-check-filling-handler-module.bsl"
        ),
        true,
        vec![attribute("own", "ПроверяемаяСтрока", "30")],
        vec![attribute("common", "НаименованиеЯзык1", "150")],
    );
    let module = session.instantiate(r#"
        Функция Проверить() Экспорт
            Проба = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Проба.Наименование = "filled";
            Итог = ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE") + "|" + Проба.ПрочитатьНаблюдение();
            Проба.ПроверяемаяСтрока = "own";
            Итог = Итог + "|" + ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE");
            Проба.НаименованиеЯзык1 = "common";
            Возврат Итог + "|" + ?(Проба.ПроверитьЗаполнение(), "TRUE", "FALSE");
        КонецФункции
    "#).unwrap();
    let native = include_str!(
        "../../../tests/conformance/measure/1c/measure-catalog-check-filling-common.platform.txt"
    );
    let expected = [
        "catalog.common.empty=",
        "catalog.common.input=",
        "catalog.common.only_own=",
        "catalog.common.filled=",
    ]
    .map(|prefix| {
        native
            .lines()
            .find_map(|line| line.strip_prefix(prefix))
            .unwrap()
    })
    .join("|");
    assert_eq!(
        format_value(&session.call(&module, "Проверить", vec![]).unwrap().0, None).unwrap(),
        expected
    );
}
