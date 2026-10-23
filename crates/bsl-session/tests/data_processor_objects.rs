use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentAttribute, DocumentTabularSection, MetadataCatalog, MetadataKind, MetadataObject,
    Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, Value, format_value};
use std::sync::Arc;

fn factory(fields: bool) -> SessionFactory {
    factory_extra(fields, "")
}

fn factory_extra(fields: bool, extra: &str) -> SessionFactory {
    let mut object = MetadataObject::new("processor", "Замеры", MetadataKind::DataProcessor)
        .with_object_module("Обработка.Замеры.МодульОбъекта")
        .with_manager_module("Обработка.Замеры.МодульМенеджера");
    let string = ColumnType {
        id: TypeRef::Native(TypeId::String),
        quals: vec!["12".into()],
    };
    let boolean = ColumnType::plain(TypeRef::Native(TypeId::Boolean));
    object = object
        .with_data_processor_attributes(if fields {
            vec![
                DocumentAttribute::new("text", "ТекстПробы", vec![string.clone()]),
                DocumentAttribute::new("flag", "ФлагПробы", vec![boolean.clone()]),
            ]
        } else {
            vec![]
        })
        .with_data_processor_tabular_sections(if fields {
            vec![
                DocumentTabularSection::new("rows", "СтрокиПробы").with_attributes(vec![
                    DocumentAttribute::new("row-text", "ТекстСтроки", vec![string]),
                    DocumentAttribute::new("row-flag", "ФлагСтроки", vec![boolean]),
                ]),
            ]
        } else {
            vec![]
        });
    let body = if fields {
        include_str!("../../../docs/research/fixtures/data-processor-object/FieldsObjectModule.bsl")
    } else {
        include_str!("../../../docs/research/fixtures/data-processor-object/ObjectModule.bsl")
    };
    let body = body.replace("СчетчикПробы = 7;", &format!("{extra}\nСчетчикПробы = 7;"));
    let modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Обработки.Замеры.Создать(); КонецФункции",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "object",
            "Обработка.Замеры.МодульОбъекта",
            body,
            ModuleKind::Object,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("processor"),
        ConfigurationModule::new(
            "manager",
            "Обработка.Замеры.МодульМенеджера",
            include_str!("../../../docs/research/fixtures/data-processor-object/ManagerModule.bsl"),
            ModuleKind::Manager,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("processor"),
    ];
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("processor", modules).unwrap(),
        Arc::new(MetadataCatalog::new("processor", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

#[test]
fn own_processor_modules_and_fields_replay_native_controls() {
    for (fields, source, receipt, count) in [
        (
            false,
            include_str!("../../../docs/research/fixtures/data-processor-object/measure.bsl"),
            include_str!(
                "../../../docs/research/fixtures/data-processor-object/measure.platform.txt"
            ),
            10,
        ),
        (
            true,
            include_str!("../../../docs/research/fixtures/data-processor-object/fields.bsl"),
            include_str!(
                "../../../docs/research/fixtures/data-processor-object/fields.platform.txt"
            ),
            15,
        ),
        (
            true,
            include_str!("../../../docs/research/fixtures/data-processor-object/synonym.bsl"),
            include_str!(
                "../../../docs/research/fixtures/data-processor-object/synonym.platform.txt"
            ),
            6,
        ),
    ] {
        let factory = factory(fields);
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        // Сохраняются все нативные вычисления; меняется только приёмник сообщений.
        let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
        let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы; КонецФункции")).unwrap();
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
        assert_eq!(lines.len(), count, "{}", lines.join("\n"));
        assert_eq!(lines.join("\n"), receipt.trim_end());
    }
}

#[test]
fn exported_processor_variable_rejects_another_session_and_unknown_names() {
    let factory = factory(false);
    let principal = || Principal::new("user", "Тест", vec![]);
    let mut first = factory
        .open(principal(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let mut second = factory
        .open(principal(), SessionResources::new(Box::new(()), 2))
        .unwrap();
    let create = first
        .instantiate(
            "Функция СоздатьПробу() Экспорт Возврат Обработки.Замеры.Создать(); КонецФункции",
        )
        .unwrap();
    let value = first.call(&create, "СоздатьПробу", vec![]).unwrap().0;
    let source = "Функция Прочитать(ОбъектПробы) Экспорт Возврат ОбъектПробы.сЧеТчИкПробы; КонецФункции Процедура Изменить(ОбъектПробы) Экспорт ОбъектПробы.СчетчикПробы = 99; КонецПроцедуры Функция Неизвестное(ОбъектПробы) Экспорт Возврат ОбъектПробы.НетТакогоПоля; КонецФункции";
    let own = first.instantiate(source).unwrap();
    let foreign = second.instantiate(source).unwrap();
    for function in ["Прочитать", "Изменить"] {
        let error = second
            .call(&foreign, function, vec![value.clone()])
            .unwrap_err();
        assert!(error.to_string().contains("другому сеансу"), "{error}");
    }
    assert_eq!(
        first
            .call(&own, "Прочитать", vec![value.clone()])
            .unwrap()
            .0,
        Value::number_from_i64(7)
    );
    assert!(
        first
            .call(&own, "Неизвестное", vec![value])
            .unwrap_err()
            .to_string()
            .contains("НетТакогоПоля")
    );
}

#[test]
fn exported_variable_keeps_dynamic_structure_names_through_the_existing_context() {
    let factory = factory_extra(
        false,
        "Функция СоздатьСтруктуру(Имя) Экспорт СчетчикПробы = Новый Структура; СчетчикПробы.Вставить(Имя, 17); КонецФункции Функция ПрочитатьПоле(Имя) Экспорт Возврат СчетчикПробы[Имя]; КонецФункции Функция ЧерезОбъект() Экспорт Возврат ЭтотОбъект.СчетчикПробы; КонецФункции",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate("Функция Проверить() Экспорт ОбъектПробы = Обработки.Замеры.Создать(); ИмяПробы = \"Динамическое\" + \"Имя\"; ОбъектПробы.СоздатьСтруктуру(ИмяПробы); Первая = ОбъектПробы.СчетчикПробы; Если Первая[ИмяПробы] <> 17 Или ОбъектПробы.ЧерезОбъект()[ИмяПробы] <> 17 Тогда Возврат Ложь; КонецЕсли; Вторая = Новый Структура; Вторая.Вставить(ИмяПробы, 42); ОбъектПробы.СчетчикПробы = Вторая; Возврат ОбъектПробы.ПрочитатьПоле(ИмяПробы) = 42; КонецФункции").unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Boolean(true)
    );
}

#[test]
fn existing_same_named_processor_manager_export_keeps_its_arity_and_result() {
    let metadata = MetadataObject::new("processor", "Проба", MetadataKind::DataProcessor)
        .with_manager_module("Обработка.Проба.МодульМенеджера");
    let modules = vec![
        ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проверить() Экспорт Возврат Обработки.Проба.Create(20,22); КонецФункции",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        ),
        ConfigurationModule::new(
            "manager",
            "Обработка.Проба.МодульМенеджера",
            "Функция Create(Первый,Второй) Экспорт Возврат Первый + Второй; КонецФункции",
            ModuleKind::Manager,
            ModuleExecution::new().with_server(true),
        )
        .with_owner("processor"),
    ];
    let factory = SessionFactory::new(
        ConfigurationCodeSnapshot::new("processor-export", modules).unwrap(),
        Arc::new(MetadataCatalog::new("processor-export", vec![metadata]).unwrap()),
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
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат Обработки.Проба.Create(20,22); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(42)
    );
}
