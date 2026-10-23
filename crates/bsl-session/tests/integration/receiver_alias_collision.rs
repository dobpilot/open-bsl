use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributes, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn receiver_property_beats_module_alias_while_parameters_and_other_modules_keep_priority() {
    let owner = "Справочник.ЗамерВременныхТаблиц.МодульОбъекта";
    let server = ModuleExecution::new().with_server(true);
    let modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент(); КонецФункции Функция ПроверитьМодуль() Экспорт Возврат ДополнительныеСвойства.Количество(); КонецФункции",
            ModuleKind::Session,
            server,
        ),
        ConfigurationModule::new(
            "common",
            "ДополнительныеСвойства",
            include_str!(
                "../../../../tests/conformance/measure/1c/receiver-alias-collision-common.bsl"
            ),
            ModuleKind::Common,
            server,
        ),
        ConfigurationModule::new(
            "object",
            owner,
            include_str!(
                "../../../../tests/conformance/measure/1c/receiver-alias-collision-object.bsl"
            ),
            ModuleKind::Object,
            server,
        )
        .with_owner("catalog"),
    ];
    let object = MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        })
        .with_object_module(owner);
    let factory = SessionFactory::new(
        ConfigurationCodeSnapshot::new("collision", modules).unwrap(),
        Arc::new(MetadataCatalog::new("collision", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("test", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session
        .instantiate(
            r#"
        Функция Проверить() Экспорт
            ПробаОбъект = Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент();
            Возврат ПробаОбъект.ПроверитьГолоеИмя() = 0
                И ПробаОбъект.ПроверитьПолучателя() = 0
                И ПробаОбъект.ПроверитьПараметр(Новый Структура("Маркер", 42)) = 1
                И МодульСеанса.ПроверитьМодуль() = 999;
        КонецФункции
    "#,
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
