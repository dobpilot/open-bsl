use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

#[test]
fn document_point_is_a_snapshot_through_qualified_and_own_module_calls() {
    let metadata = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_attributes(vec![])
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_object_module("Документ.ЗамерДокумента.МодульОбъекта");
    let code = ConfigurationCodeSnapshot::new(
        "point",
        vec![
            ConfigurationModule::new(
                "session",
                "МодульСеанса",
                "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
                ModuleKind::Session,
                ModuleExecution::new().with_server(true),
            ),
            ConfigurationModule::new(
                "object",
                "Документ.ЗамерДокумента.МодульОбъекта",
                "Функция ПолучитьМомент() Экспорт Возврат МоментВремени(); КонецФункции",
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("doc"),
        ],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("point", vec![metadata]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт ДокументПробы = Документы.ЗамерДокумента.СоздатьДокумент(); ДокументПробы.Дата = Дата(2024,1,2,12,34,56); МоментПробы = ДокументПробы.МоментВремени(); Равны = МоментПробы = ДокументПробы.ПолучитьМомент() И МоментПробы = ДокументПробы.PointInTime(); ДокументПробы.Дата = Дата(2025,1,1); Возврат Равны И МоментПробы.Дата = Дата(2024,1,2,12,34,56) И МоментПробы.Ссылка = Документы.ЗамерДокумента.ПустаяСсылка() И МоментПробы.Сравнить(Новый МоментВремени(МоментПробы.Дата)) = 1; КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}
