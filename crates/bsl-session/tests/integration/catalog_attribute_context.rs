use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogDefaultPresentation, CatalogStandardAttributes, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let object_module = "Справочник.Проба.МодульОбъекта";
    let catalog = MetadataCatalog::new(
        "attributes",
        vec![
            MetadataObject::new("catalog", "Проба", MetadataKind::Catalog)
                .with_object_module(object_module)
                .with_catalog_attribute_names(["ОбъектАвторизации"])
                .with_catalog_standard_attributes(CatalogStandardAttributes {
                    code_type: CatalogCodeType::String,
                    code_length: 9,
                    description_length: 25,
                })
                .with_catalog_default_presentation(CatalogDefaultPresentation::AsDescription),
        ],
    )
    .unwrap();
    let modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция СоздатьПробу() Экспорт Возврат Справочники.Проба.СоздатьЭлемент(); КонецФункции",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "object",
            object_module,
            source,
            ModuleKind::Object,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("catalog"),
    ];
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("attributes", modules).unwrap(),
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
}

fn session(source: &str) -> Session {
    factory(source)
        .unwrap()
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap()
}

fn call(session: &mut Session, source: &str) -> Result<Value, bsl_session::SessionError> {
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт {source} КонецФункции"
    ))?;
    session
        .call(&module, "Проверить", vec![])
        .map(|(value, _)| value)
}

#[test]
fn acquired_attribute_names_link_original_reads_and_writes_without_fabricating_values() {
    let mut session = session(
        r#"
        Функция Читать() Экспорт Возврат ОбъектАвторизации; КонецФункции
        Процедура Записать() Экспорт ОбъектАвторизации = 17; КонецПроцедуры
        Функция ЧитатьДинамически() Экспорт Возврат Вычислить("объектавторизации"); КонецФункции
        Процедура ЗаписатьДинамически() Экспорт Выполнить("ОбъектАвторизации = 17;"); КонецПроцедуры
    "#,
    );
    for method in [
        "Читать",
        "Записать",
        "ЧитатьДинамически",
        "ЗаписатьДинамически",
    ] {
        let error = call(
            &mut session,
            &format!(
                "Объект = Справочники.Проба.СоздатьЭлемент(); Объект.{method}(); Возврат Истина;"
            ),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("ОбъектАвторизации"), "{error}");
        assert!(
            error.contains("тип и значение пока не реализованы"),
            "{error}"
        );
    }
    assert!(session.instantiate("Возврат ОбъектАвторизации;").is_err());
}

#[test]
fn acquired_names_keep_local_parameter_and_module_variable_priority() {
    let mut local = session(
        r#"
        Функция Проверить(ОбъектАвторизации) Экспорт
            Возврат ОбъектАвторизации + Вычислить("ОбъектАвторизации");
        КонецФункции
        Функция Локальная() Экспорт
            Перем ОбъектАвторизации;
            ОбъектАвторизации = 11;
            Возврат ОбъектАвторизации;
        КонецФункции
    "#,
    );
    assert_eq!(call(&mut local, "Объект = Справочники.Проба.СоздатьЭлемент(); Возврат Объект.Проверить(7) + Объект.Локальная();").unwrap(), Value::number_from_i64(25));
    let mut module = session(
        "Перем ОбъектАвторизации; ОбъектАвторизации = 19; Функция Читать() Экспорт Возврат ОбъектАвторизации; КонецФункции",
    );
    assert_eq!(
        call(
            &mut module,
            "Возврат Справочники.Проба.СоздатьЭлемент().Читать();"
        )
        .unwrap(),
        Value::number_from_i64(19)
    );
}

#[test]
fn unknown_attribute_stays_undefined_in_an_object_module() {
    let error = factory("Функция Читать() Экспорт Возврат НесуществующийРеквизит; КонецФункции")
        .err()
        .expect("неизвестный реквизит должен остаться ошибкой")
        .to_string();
    assert!(error.contains("НесуществующийРеквизит"), "{error}");
    assert!(error.contains("читается до присваивания"), "{error}");
}
