use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionError, SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;
use std::sync::Arc;

fn factory(source: &str) -> Result<SessionFactory, SessionError> {
    let module = "Обработка.Проверка.МодульОбъекта";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "processor-object-template",
            vec![
                ConfigurationModule::new(
                    "session",
                    "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Обработки.Проверка.Создать(); КонецФункции",
                    ModuleKind::Session,
                    ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "object",
                    module,
                    source,
                    ModuleKind::Object,
                    ModuleExecution::new().with_server(true),
                )
                .with_owner("processor"),
            ],
        )
        .unwrap(),
        Arc::new(
            MetadataCatalog::new(
                "processor-object-template",
                vec![
                    MetadataObject::new("processor", "Проверка", MetadataKind::DataProcessor)
                        .with_object_module(module)
                        .with_data_processor_attributes(vec![])
                        .with_data_processor_tabular_sections(vec![]),
                ],
            )
            .unwrap(),
        ),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
}

#[test]
fn processor_object_template_calls_bind_but_do_not_fabricate_resources() {
    let factory = factory("Функция Макет() Экспорт Возврат ПолучитьМакет(\"Пример\"); КонецФункции Функция English() Экспорт Возврат gEtTeMpLaTe(\"Пример\"); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let probe = session.instantiate("Функция Проверить() Экспорт Возврат ТипЗнч(Обработки.Проверка.Создать()) = Тип(\"DataProcessorObject.Проверка\"); КонецФункции").unwrap();
    assert_eq!(
        session.call(&probe, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
    for name in ["ПолучитьМакет", "gEtTeMpLaTe", "Макет", "English"] {
        let arguments = if name == "Макет" || name == "English" {
            ""
        } else {
            "\"Пример\""
        };
        let probe = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Обработки.Проверка.Создать().{name}({arguments}); КонецФункции")).unwrap();
        let error = session
            .call(&probe, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("ПолучитьМакет") && error.contains("пока не поддерживается"),
            "{error}"
        );
    }
    assert!(
        session
            .instantiate(
                "Функция Проверить() Экспорт Возврат ПолучитьМакет(\"Пример\"); КонецФункции"
            )
            .is_err()
    );
}

#[test]
fn processor_object_template_own_calls_reject_wrong_arity() {
    for name in ["ПолучитьМакет", "GetTemplate"] {
        for args in ["", "1, 2"] {
            let factory = factory(&format!(
                "Процедура Проверить() Экспорт {name}({args}); КонецПроцедуры"
            ))
            .unwrap();
            let mut session = factory
                .open(
                    Principal::new("user", "Тест", vec![]),
                    SessionResources::new(Box::new(()), 1),
                )
                .unwrap();
            let probe = session.instantiate("Процедура Вызвать() Экспорт Обработки.Проверка.Создать().Проверить(); КонецПроцедуры").unwrap();
            let error = session
                .call(&probe, "Вызвать", vec![])
                .unwrap_err()
                .to_string();
            assert!(error.contains("аргумент"), "{error}");
        }
    }
}

#[test]
fn processor_template_declaration_preserves_existing_object_exports() {
    for name in ["ПолучитьМакет", "GetTemplate"] {
        let factory = factory(&format!(
            "Функция {name}(А, Б, В) Экспорт Возврат А + Б + В; КонецФункции"
        ))
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let probe = session.instantiate(&format!("Функция Проверить() Экспорт Возврат Обработки.Проверка.Создать().{name}(1, 2, 3); КонецФункции")).unwrap();
        assert_eq!(
            session.call(&probe, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(6)
        );
    }
}
