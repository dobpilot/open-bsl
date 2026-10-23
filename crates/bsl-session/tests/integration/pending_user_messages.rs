use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    BusinessProcessNumberSettings, CatalogStandardAttributeFillChecking, DocumentAttribute,
    DocumentNumberType, FillChecking, MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{ColumnType, TypeId, TypeRef, Value};

fn factory(kind: SessionKind) -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "messages",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "messages",
        vec![
            MetadataObject::new(
                "process",
                "ЗамерБизнесПроцесса",
                MetadataKind::BusinessProcess,
            ),
            MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
                .with_string_property("КодЯзыка", "ru"),
        ],
    )
    .unwrap()
    .with_business_process_metadata(
        "process",
        Some(BusinessProcessNumberSettings {
            number_type: DocumentNumberType::String,
            length: 11,
            variable_length: true,
        }),
        Some(vec![
            DocumentAttribute::new(
                "text",
                "ТекстПробы",
                vec![ColumnType {
                    id: TypeRef::Native(TypeId::String),
                    quals: vec!["12".into()],
                }],
            )
            .with_fill_from_filling_value(false)
            .with_fill_checking(FillChecking::ShowError)
            .with_synonym("ru", "Текст пробы"),
        ]),
        Some(vec![]),
    )
    .unwrap()
    .with_business_process_standard_fill_checking(
        "process",
        [
            "Started",
            "HeadTask",
            "Completed",
            "Ref",
            "DeletionMark",
            "Date",
            "Number",
        ]
        .into_iter()
        .map(|name| CatalogStandardAttributeFillChecking {
            name: name.into(),
            fill_checking: FillChecking::DontCheck,
        })
        .collect(),
    )
    .unwrap();
    let catalog = catalog
        .with_configuration(
            "Проба",
            "1.0",
            bsl_orm::ScriptVariant::Russian,
            bsl_orm::CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap()
        .with_primary_language("language-ru")
        .unwrap();
    SessionFactory::new(code, Arc::new(catalog), "МодульСеанса", kind, "8.3.27.2342").unwrap()
}

fn principal() -> Principal {
    Principal::new("user", "Тест", vec![])
}

#[test]
fn original_business_check_filling_message_receipt_runs_in_a_real_session() {
    let factory = factory(SessionKind::ThinClient);
    let mut session = factory
        .open(
            principal(),
            SessionResources::new(Box::new(()), 1).with_system_language("en"),
        )
        .unwrap();
    let original = include_str!(
        "../../../../docs/research/fixtures/business-process-object/check-filling-message.bsl"
    );
    let module = session
        .instantiate(&format!(
            "Перем ВыводПробы;\nПроцедура ЗаписатьКонтроль(Текст) ВыводПробы = ВыводПробы + Текст + Символ(10); КонецПроцедуры\nФункция Проверить() Экспорт ВыводПробы = \"\";\n{}\nВозврат ВыводПробы; КонецФункции",
            original.replace("Сообщить(", "ЗаписатьКонтроль(")
        ))
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::Str(include_str!("../../../../docs/research/fixtures/business-process-object/check-filling-message.platform.txt").into())
    );
}

#[test]
fn pending_messages_are_not_inherited_by_another_or_background_session() {
    let foreground = factory(SessionKind::ThinClient);
    let mut parent = foreground
        .open(principal(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let emit = parent.instantiate("Процедура Отправить() Экспорт Сообщение = Новый СообщениеПользователю; Сообщение.Текст = \"pending\"; Сообщение.Message(); КонецПроцедуры").unwrap();
    parent.call(&emit, "Отправить", vec![]).unwrap();
    let read = "Функция Количество() Экспорт Возврат ПолучитьСообщенияПользователю().Количество(); КонецФункции";
    let module = parent.instantiate(read).unwrap();
    assert_eq!(
        parent.call(&module, "Количество", vec![]).unwrap().0,
        Value::number_from_i64(1)
    );
    let mut other = foreground
        .open(principal(), SessionResources::new(Box::new(()), 2))
        .unwrap();
    let module = other.instantiate(read).unwrap();
    assert_eq!(
        other.call(&module, "Количество", vec![]).unwrap().0,
        Value::number_from_i64(0)
    );
    let background = factory(SessionKind::BackgroundJob);
    let mut child = background
        .open_background(
            principal(),
            SessionResources::new(Box::new(()), 3),
            "Обработка.Проба",
            &parent,
        )
        .unwrap();
    let module = child.instantiate(read).unwrap();
    assert_eq!(
        child.call(&module, "Количество", vec![]).unwrap().0,
        Value::number_from_i64(0)
    );
}
