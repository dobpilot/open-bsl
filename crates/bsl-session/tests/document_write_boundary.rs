use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentAttribute, DocumentNumberSettings, DocumentNumberType, DocumentTabularSection,
    MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{ColumnType, TypeId, TypeRef, Value};
use std::sync::Arc;

fn factory(source: &str) -> SessionFactory {
    let object_module = "Документ.Проба.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("write-boundary", vec![
        ConfigurationModule::new("session", "МодульСеанса", "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Связать() Экспорт Возврат Документы.Проба.СоздатьДокумент(); КонецФункции", ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", object_module, source, ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    let catalog = MetadataCatalog::new(
        "write-boundary",
        vec![
            MetadataObject::new("doc", "Проба", MetadataKind::Document)
                .with_object_module(object_module)
                .with_document_attributes(vec![DocumentAttribute::new(
                    "text",
                    "ТекстПробы",
                    vec![ColumnType::plain(TypeRef::Native(TypeId::String))],
                )])
                .with_document_tabular_sections(vec![
                    DocumentTabularSection::new("section", "СтрокиПробы").with_attributes(vec![
                        DocumentAttribute::new(
                            "flag",
                            "ФлагПробы",
                            vec![ColumnType::plain(TypeRef::Native(TypeId::Boolean))],
                        ),
                    ]),
                ])
                .with_document_number_settings(DocumentNumberSettings {
                    number_type: DocumentNumberType::String,
                    number_length: 9,
                }),
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
fn document_write_aliases_and_arguments_preserve_the_unsaved_object() {
    let factory = factory(
        "Процедура Неиспользуемая() Записать(); КонецПроцедуры Процедура ПередЗаписью(Отказ, РежимЗаписи, РежимПроведения) ТекстПробы = \"обработчик\"; КонецПроцедуры",
    );
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let module = session.instantiate(r#"
Перем ОбъектПробы;
Процедура Создать() Экспорт ОбъектПробы = Документы.Проба.СоздатьДокумент(); ОбъектПробы.Номер = "контроль"; ОбъектПробы.ТекстПробы = "начало"; ОбъектПробы.СтрокиПробы.Добавить().ФлагПробы = Истина; КонецПроцедуры
Функция Состояние() Экспорт Возврат ?(ОбъектПробы.ЭтоНовый(), "new", "saved") + "|" + ?(ОбъектПробы.Ссылка.Пустая(), "empty", "ref") + "|" + ОбъектПробы.Номер + "|" + ОбъектПробы.ТекстПробы + "|" + Строка(ОбъектПробы.СтрокиПробы.Количество()) + "|" + ?(ОбъектПробы.СтрокиПробы[0].ФлагПробы, "true", "false"); КонецФункции
Процедура Проверить(Имя, Аргументы) Экспорт Выполнить("ОбъектПробы." + Имя + "(" + Аргументы + ");"); КонецПроцедуры
"#).unwrap();
    session.call(&module, "Создать", vec![]).unwrap();
    let state = session.call(&module, "Состояние", vec![]).unwrap().0;
    assert_eq!(state, Value::Str("new|empty|контроль|начало|1|true".into()));
    for name in ["Записать", "Write"] {
        for arguments in ["", "Неопределено", "Неопределено, Неопределено"]
        {
            let error = session
                .call(
                    &module,
                    "Проверить",
                    vec![Value::Str(name.into()), Value::Str(arguments.into())],
                )
                .unwrap_err();
            assert!(error.to_string().contains("пока не подключена"), "{error}");
            assert_eq!(session.call(&module, "Состояние", vec![]).unwrap().0, state);
        }
        let error = session
            .call(
                &module,
                "Проверить",
                vec![
                    Value::Str(name.into()),
                    Value::Str("Неопределено, Неопределено, Неопределено".into()),
                ],
            )
            .unwrap_err();
        assert!(!error.to_string().contains("пока не подключена"), "{error}");
        assert_eq!(session.call(&module, "Состояние", vec![]).unwrap().0, state);
    }
}

#[test]
fn document_write_declaration_does_not_intercept_existing_exports() {
    for name in ["Записать", "Write"] {
        let factory = factory(&format!(
            "Функция {name}() Экспорт Возврат \"export\"; КонецФункции"
        ));
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат Документы.Проба.СоздатьДокумент().{name}(); КонецФункции"
            ))
            .unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Str("export".into())
        );
    }
}
