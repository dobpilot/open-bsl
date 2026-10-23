use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

fn factory() -> SessionFactory {
    let module = "Документ.ЗамерДокумента.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("document-new-ref", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            include_str!("../../../../tests/conformance/measure/1c/document-new-reference-module.bsl"),
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_object_module(module)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_document_attributes(vec![]);
    let catalog = MetadataCatalog::new(
        "document-new-ref",
        vec![
            document,
            MetadataObject::new("other", "ЗамерЧужогоДокумента", MetadataKind::Document),
            MetadataObject::new("catalog", "ЗамерВременныхТаблиц", MetadataKind::Catalog),
        ],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn native_document_future_references_match_in_independent_sessions() {
    let factory = factory();
    for _ in 0..2 {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let script = include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-new-reference.bsl"
        )
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
        let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
        else {
            panic!("массив результатов");
        };
        let BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let output = result
            .borrow()
            .iter()
            .map(|value| format_value(value, None).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            output,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-new-reference.platform.txt"
            )
        );
    }
}
