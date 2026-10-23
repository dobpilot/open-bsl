use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CatalogCodeType, CatalogStandardAttributes, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

const OBJECT: &str = "Справочник.Контроль.МодульОбъекта";
const ENTRY: &str = "Функция Проверить() Экспорт ОбъектПробы = Справочники.Контроль.СоздатьЭлемент(); ОбъектПробы.УстановитьНовыйКод(); Возврат ОбъектПробы.Код; КонецФункции";
const EVENT: &str = "Процедура ПриУстановкеНовогоКода(СтандартнаяОбработка, Префикс) СтандартнаяОбработка = Ложь; Код = \"entry\"; КонецПроцедуры";

fn inputs(object_source: &str, dormant: bool) -> (ConfigurationCodeSnapshot, Arc<MetadataCatalog>) {
    let mut modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "object",
            OBJECT,
            object_source,
            ModuleKind::Object,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("catalog"),
    ];
    if dormant {
        modules.push(
            ConfigurationModule::new(
                "dormant",
                "ЗакрытыйНеДостигнутый",
                "Функция НеВызывать() Возврат НеизвестнаяПеременная; КонецФункции",
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("unused"),
        );
    }
    let object = MetadataObject::new("catalog", "Контроль", MetadataKind::Catalog)
        .with_object_module(OBJECT)
        .with_catalog_standard_attributes(CatalogStandardAttributes {
            code_type: CatalogCodeType::String,
            code_length: 9,
            description_length: 25,
        });
    (
        ConfigurationCodeSnapshot::new("entry-roots", modules).unwrap(),
        Arc::new(MetadataCatalog::new("entry-roots", vec![object]).unwrap()),
    )
}

fn factory(source: &str, entry: &str, dormant: bool) -> SessionFactory {
    let (code, catalog) = inputs(source, dormant);
    SessionFactory::new_with_entry_sources(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
        [entry],
    )
    .unwrap()
}

#[test]
fn declared_entry_reaches_its_object_without_a_session_module_reference() {
    let mut session = factory(EVENT, ENTRY, true)
        .open(
            Principal::new("probe", "Тест", []),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(ENTRY).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str("entry".into())
    );
}

#[test]
fn undeclared_entry_keeps_the_existing_frozen_graph_boundary() {
    let (code, catalog) = inputs(EVENT, false);
    let factory = SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("probe", "Тест", []),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(ENTRY).unwrap();
    assert!(
        session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string()
            .contains(OBJECT)
    );
}

#[test]
fn shadowed_component_roots_do_not_admit_an_invalid_private_body() {
    for entry in [
        "Перем Справочники; Функция Проверить() Экспорт Если Ложь Тогда Справочники.Контроль.СоздатьЭлемент(); КонецЕсли; Возврат 1; КонецФункции",
        "Функция Проверить() Экспорт Перем Справочники; Если Ложь Тогда Справочники.Контроль.СоздатьЭлемент(); КонецЕсли; Возврат 1; КонецФункции",
    ] {
        let mut session = factory(
            "Функция НеВызывать() Возврат НеизвестнаяПеременная; КонецФункции",
            entry,
            false,
        )
        .open(
            Principal::new("probe", "Тест", []),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
        let module = session.instantiate(entry).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(1)
        );
    }
}

#[test]
fn declaring_a_source_neither_executes_it_nor_initializes_the_reached_object() {
    let entry = "ВызватьИсключение \"entry execution\"; Справочники.Контроль.СоздатьЭлемент();";
    let mut session = factory("ВызватьИсключение \"object initialization\";", entry, false)
        .open(
            Principal::new("probe", "Тест", []),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let error = session.execute(entry).unwrap_err().to_string();
    assert!(error.contains("entry execution"), "{error}");
    assert!(!error.contains("object initialization"), "{error}");
}
