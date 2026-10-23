use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    BusinessProcessNumberSettings, DocumentAttribute, DocumentNumberType, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{
    ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeId, TypeRef, Value,
};
use std::sync::Arc;

#[test]
fn empty_returned_handler_data_replays_the_complete_native_control() {
    let factory = factory(
        enabled_attributes(),
        Some(include_str!(
            "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler-empty-object.bsl"
        )),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!(
        "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler-empty.bsl"
    );
    let expected = include_str!(
        "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler-empty.platform.txt"
    );
    // Нативное тело сохраняется; заменяется только приёмник вывода.
    let source = format!(
        r#"Перем ВыводПробы; Процедура ЗаписатьКонтроль(Текст) ВыводПробы = ВыводПробы + Текст + Символ(10); КонецПроцедуры Функция Проверить() Экспорт ВыводПробы = ""; {} Возврат ВыводПробы; КонецФункции"#,
        source.replace("Сообщить(", "ЗаписатьКонтроль(")
    );
    let module = session.instantiate(&source).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(expected.into())
    );
}

#[test]
fn enabled_attributes_replay_the_complete_native_control() {
    let attributes = enabled_attributes();
    let factory = factory(attributes, None);
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source =
        include_str!("../../../../docs/research/fixtures/business-process-object/fill-enabled.bsl");
    let expected = include_str!(
        "../../../../docs/research/fixtures/business-process-object/fill-enabled.platform.txt"
    );
    // Заменяется только приёмник вывода; нативный контроль не переписывается.
    let source = format!(
        r#"Перем ВыводПробы; Процедура ЗаписатьКонтроль(Текст) ВыводПробы = ВыводПробы + Текст + Символ(10); КонецПроцедуры Функция Проверить() Экспорт ВыводПробы = ""; {} Возврат ВыводПробы; КонецФункции"#,
        source.replace("Сообщить(", "ЗаписатьКонтроль(")
    );
    let module = session.instantiate(&source).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(expected.into())
    );
}

fn factory(attributes: Vec<DocumentAttribute>, object_source: Option<&str>) -> SessionFactory {
    factory_with_static_binding(attributes, object_source, true)
}

fn factory_with_static_binding(
    attributes: Vec<DocumentAttribute>,
    object_source: Option<&str>,
    bind_object: bool,
) -> SessionFactory {
    let object_module = "БизнесПроцесс.ЗамерБизнесПроцесса.МодульОбъекта";
    let mut object = MetadataObject::new(
        "process",
        "ЗамерБизнесПроцесса",
        MetadataKind::BusinessProcess,
    );
    if object_source.is_some() {
        object = object.with_object_module(object_module);
    }
    let catalog = MetadataCatalog::new("enabled-business", vec![object])
        .unwrap()
        .with_business_process_metadata(
            "process",
            Some(BusinessProcessNumberSettings {
                number_type: DocumentNumberType::String,
                length: 11,
                variable_length: true,
            }),
            Some(attributes),
            Some(vec![]),
        )
        .unwrap();
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        if bind_object {
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция ОбъектДляСвязывания() Экспорт Возврат БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс(); КонецФункции"
        } else {
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры"
        },
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    if let Some(source) = object_source {
        modules.push(
            ConfigurationModule::new(
                "object",
                object_module,
                source,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("process"),
        );
    }
    let code = ConfigurationCodeSnapshot::new("enabled-business", modules).unwrap();
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
fn separately_compiled_entry_cannot_load_an_object_outside_the_static_roots() {
    // Закрепляет нынешнее ограничение хоста, чтобы расширение корней не
    // затерялось в успешных контролях со статической ссылкой на объект.
    let factory = factory_with_static_binding(
        enabled_attributes(),
        Some("Процедура ОбработкаЗаполнения(Данные, Текст, СтандартнаяОбработка) КонецПроцедуры"),
        false,
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for expression in [
        "БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс()",
        "Вычислить(\"БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс()\")",
    ] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат {expression}; КонецФункции"
            ))
            .unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(
            error.to_string().contains(
                "Модуль конфигурации «БизнесПроцесс.ЗамерБизнесПроцесса.МодульОбъекта» не найден"
            ),
            "{error}"
        );
    }
}

#[test]
fn filling_rejects_unknown_policy_defaults_and_unmeasured_defaults() {
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let attribute = DocumentAttribute::new(
        "text",
        "ТекстПробы",
        vec![ColumnType {
            id: TypeRef::Native(TypeId::String),
            quals: vec!["12".into()],
        }],
    );
    let mut cases = vec![
        attribute.clone(),
        attribute.clone().with_fill_from_filling_value(true),
    ];
    for roots in [
        vec![],
        vec![Value::Undefined, Value::Boolean(true)],
        vec![Value::Str("не измерено".into())],
    ] {
        let graph =
            SerializedValueGraph::capture(&roots, &shapes, &GraphLimits::default()).unwrap();
        cases.push(
            attribute
                .clone()
                .with_fill_from_filling_value(true)
                .with_fill_value(graph),
        );
    }
    for attribute in cases {
        let factory = factory(vec![attribute], None);
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate("Функция Проверить() Экспорт Объект = БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс(); Объект.Заполнить(Новый Структура); Возврат 1; КонецФункции").unwrap();
        assert!(session.call(&module, "Проверить", vec![]).is_err());
    }
}

fn enabled_attributes() -> Vec<DocumentAttribute> {
    let shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let default =
        SerializedValueGraph::capture(&[Value::Undefined], &shapes, &GraphLimits::default())
            .unwrap();
    vec![
        DocumentAttribute::new(
            "text",
            "ТекстПробы",
            vec![ColumnType {
                id: TypeRef::Native(TypeId::String),
                quals: vec!["12".into()],
            }],
        )
        .with_fill_from_filling_value(true)
        .with_fill_value(default.clone()),
        DocumentAttribute::new(
            "flag",
            "ФлагПробы",
            vec![ColumnType::plain(TypeRef::Native(TypeId::Boolean))],
        )
        .with_fill_from_filling_value(true)
        .with_fill_value(default),
    ]
}

#[test]
fn enabled_handler_replays_returned_data_and_caller_identity() {
    let factory = factory(
        enabled_attributes(),
        Some(include_str!(
            "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler-object.bsl"
        )),
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let source = include_str!(
        "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler.bsl"
    );
    let expected = include_str!(
        "../../../../docs/research/fixtures/business-process-object/fill-enabled-handler.platform.txt"
    );
    let source = format!(
        r#"Перем ВыводПробы; Процедура ЗаписатьКонтроль(Текст) ВыводПробы = ВыводПробы + Текст + Символ(10); КонецПроцедуры Функция Проверить() Экспорт ВыводПробы = ""; {} Возврат ВыводПробы; КонецФункции"#,
        source.replace("Сообщить(", "ЗаписатьКонтроль(")
    );
    let module = session.instantiate(&source).unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(expected.into())
    );
}
