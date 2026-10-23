use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

fn factory(registers: Option<Vec<String>>) -> SessionFactory {
    factory_with_source(
        registers,
        include_str!("../../../tests/conformance/measure/1c/document-movements-module.bsl"),
    )
}

fn factory_with_source(registers: Option<Vec<String>>, source: &str) -> SessionFactory {
    let module = "Документ.ЗамерДокумента.МодульОбъекта";
    let mut document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_object_module(module)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_document_attributes(vec![]);
    if let Some(registers) = registers {
        document = document.with_document_registers(registers);
    }
    let code = ConfigurationCodeSnapshot::new("movements-test", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module,
            source,
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    SessionFactory::new(
        code,
        Arc::new(
            MetadataCatalog::new(
                "movements-test",
                vec![
                    document,
                    MetadataObject::new(
                        "register",
                        "РасписаниеЗагрузкиРесурсов",
                        MetadataKind::InformationRegister,
                    ),
                    MetadataObject::new(
                        "unsupported",
                        "Накопление",
                        MetadataKind::AccumulationRegister,
                    ),
                ],
            )
            .unwrap(),
        ),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn native_body_record_set_flags_and_identity_are_retained() {
    let factory = factory_with_source(
        Some(vec!["register".into()]),
        include_str!("../../../tests/conformance/measure/1c/document-movements-flags-module.bsl"),
    );
    assert_eq!(
        replay(
            &factory,
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-movements-flags.bsl"
            )
        ),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-document-movements-flags.platform.txt"
        )
    );
}

fn replay(factory: &SessionFactory, script: &str) -> String {
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let script = script.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    let lines = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>();
    lines.join("\n") + "\n"
}

#[test]
fn native_empty_document_movements_match_in_independent_sessions() {
    let factory = factory(Some(vec![]));
    for _ in 0..2 {
        assert_eq!(
            replay(
                &factory,
                include_str!(
                    "../../../tests/conformance/measure/1c/measure-document-movements.bsl"
                )
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-movements.platform.txt"
            )
        );
    }
}

#[test]
fn native_owned_information_record_sets_match_in_independent_sessions() {
    let factory = factory(Some(vec!["register".into()]));
    for _ in 0..2 {
        assert_eq!(
            replay(
                &factory,
                include_str!(
                    "../../../tests/conformance/measure/1c/measure-document-movements-register.bsl"
                )
            ),
            include_str!(
                "../../../tests/conformance/measure/1c/measure-document-movements-register.platform.txt"
            )
        );
    }
}

#[test]
fn unknown_document_associations_are_not_a_fabricated_empty_collection() {
    let factory = factory(None);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент().Движения; КонецФункции").unwrap();
    let error = session
        .call(&module, "Проверить", vec![])
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("регистры движений документа неизвестны"),
        "{error}"
    );
}

#[test]
fn invalid_or_unsupported_associations_do_not_return_shortened_collections() {
    for (registers, message) in [
        (vec!["register", "missing"], "отсутствует в каталоге"),
        (vec!["register", "unsupported"], "пока не поддерживается"),
        (vec!["register", "register"], "повторный регистр"),
    ] {
        let factory = factory(Some(registers.into_iter().map(str::to_owned).collect()));
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate("Функция Проверить() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции").unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(error.contains(message), "{error}");
    }
}
