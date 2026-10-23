use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    BusinessProcessNumberSettings, DocumentAttribute, DocumentNumberType, MetadataCatalog,
    MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{ColumnType, TypeId, TypeRef, Value};
use std::sync::Arc;

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "БизнесПроцесс.Проба.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("write-control", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат БизнесПроцессы.Проба.СоздатьБизнесПроцесс(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", module, source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("process"),
    ]).unwrap();
    let catalog = MetadataCatalog::new(
        "write-control",
        vec![
            MetadataObject::new("process", "Проба", MetadataKind::BusinessProcess)
                .with_object_module(module),
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
        Some(vec![DocumentAttribute::new(
            "text",
            "ТекстПробы",
            vec![ColumnType {
                id: TypeRef::Native(TypeId::String),
                quals: vec!["12".into()],
            }],
        )]),
        Some(vec![]),
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(catalog),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
}

#[test]
fn declared_write_accepts_own_context_but_cannot_mutate_an_unsaved_object() {
    let factory = factory("Процедура Неиспользуемая() Записать(); КонецПроцедуры").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"
Перем ОбъектПробы;
Процедура Создать() Экспорт ОбъектПробы = БизнесПроцессы.Проба.СоздатьБизнесПроцесс(); ОбъектПробы.Номер = "контроль"; ОбъектПробы.ТекстПробы = "начало"; КонецПроцедуры
Процедура ЗаписатьРУ() Экспорт ОбъектПробы.Записать(); КонецПроцедуры
Процедура ЗаписатьEN() Экспорт ОбъектПробы.Write(); КонецПроцедуры
Функция Состояние() Экспорт Возврат ?(ОбъектПробы.ЭтоНовый(), "new", "saved") + "|" + ?(ОбъектПробы.Ссылка.Пустая(), "empty", "ref") + "|" + ОбъектПробы.Номер + "|" + ОбъектПробы.ТекстПробы; КонецФункции
"#).unwrap();
    session.call(&module, "Создать", vec![]).unwrap();
    let state = session.call(&module, "Состояние", vec![]).unwrap().0;
    assert_eq!(state, Value::Str("new|empty|контроль|начало".into()));
    for method in ["ЗаписатьРУ", "ЗаписатьEN"] {
        let error = session.call(&module, method, vec![]).unwrap_err();
        assert!(
            error.to_string().contains(
                "запись объекта бизнес-процесса в информационную базу пока не подключена"
            ),
            "{error}"
        );
        assert_eq!(session.call(&module, "Состояние", vec![]).unwrap().0, state);
    }
}

#[test]
fn previously_callable_standard_method_exports_keep_their_behavior() {
    for name in [
        "Записать",
        "Write",
        "Прочитать",
        "Read",
        "Модифицированность",
        "Modified",
        "ПолучитьСсылкуНового",
        "GetNewObjectRef",
        "УстановитьСсылкуНового",
        "SetNewObjectRef",
    ] {
        let factory = factory(&format!(
            "Функция {name}(Значение) Экспорт Возврат Значение; КонецФункции"
        ))
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate(&format!("Функция Проверить() Экспорт Возврат БизнесПроцессы.Проба.СоздатьБизнесПроцесс().{}(17); КонецФункции", name.to_lowercase())).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(17)
        );
    }
}

#[test]
fn own_context_rejects_an_argument_to_zero_arity_standard_methods() {
    for name in [
        "Записать",
        "Write",
        "Прочитать",
        "Read",
        "Модифицированность",
        "Modified",
        "ПолучитьСсылкуНового",
        "GetNewObjectRef",
    ] {
        let factory = factory(&format!(
            "Процедура Неверная() Экспорт {name}(1); КонецПроцедуры"
        ))
        .unwrap();
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session.instantiate("Процедура Проверить() Экспорт БизнесПроцессы.Проба.СоздатьБизнесПроцесс().Неверная(); КонецПроцедуры").unwrap();
        let error = session.call(&module, "Проверить", vec![]).unwrap_err();
        assert!(error.to_string().contains("передано: 1"), "{error}");
    }
}
