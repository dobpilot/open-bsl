use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

#[test]
fn object_initialization_and_export_callbacks_replay_native_handler_scopes() {
    let object_module = "Обработка.Замеры.МодульОбъекта";
    let factory = SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "error-description-object",
            vec![
                ConfigurationModule::new(
                    "session",
                    "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Обработки.Замеры.Создать(); КонецФункции",
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "object",
                    object_module,
                    include_str!("../../../docs/research/fixtures/error-description-context/ObjectModule.bsl"),
                    ModuleKind::Object,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("processor"),
            ],
        )
        .unwrap(),
        Arc::new(
            MetadataCatalog::new(
                "error-description-object",
                vec![MetadataObject::new("processor", "Замеры", MetadataKind::DataProcessor)
                    .with_object_module(object_module)
                    .with_data_processor_attributes(vec![])
                    .with_data_processor_tabular_sections(vec![])],
            )
            .unwrap(),
        ),
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
    let source =
        include_str!("../../../docs/research/fixtures/error-description-context/instance.bsl")
            .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы; КонецФункции"
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов")
    };
    let lines = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 9, "{}", lines.join("\n"));
    assert_eq!(
        lines.join("\n"),
        include_str!(
            "../../../docs/research/fixtures/error-description-context/instance.platform.txt"
        )
        .trim_end()
    );
}
