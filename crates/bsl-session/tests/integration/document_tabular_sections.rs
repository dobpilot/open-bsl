use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DocumentTabularSection, MetadataCatalog, MetadataKind, MetadataObject};
use bsl_session::{SessionFactory, SessionKind};
use std::sync::Arc;

fn section_factory(body: Option<&str>) -> SessionFactory {
    use bsl_orm::{DocumentAttribute, DocumentNumberSettings, DocumentNumberType};
    use open_bsl::{ColumnType, TypeId, TypeRef};
    let section = DocumentTabularSection::new("section", "Запасы").with_attributes(vec![
        DocumentAttribute::new(
            "bool",
            "ТипНоменклатурыЗапас",
            vec![ColumnType::plain(TypeRef::Native(TypeId::Boolean))],
        ),
        DocumentAttribute::new(
            "number",
            "НомерВариантаКП",
            vec![ColumnType {
                id: TypeRef::Native(TypeId::Number),
                quals: vec!["2".into(), "0".into(), "0".into()],
            }],
        ),
    ]);
    let mut object = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        })
        .with_document_attributes(vec![])
        .with_document_tabular_sections(vec![section]);
    let mut modules = vec![ConfigurationModule::new(
        "session",
        "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session,
        ModuleExecution::new().with_server(true),
    )];
    if let Some(body) = body {
        let module = "Документ.ЗамерДокумента.МодульОбъекта";
        object = object.with_object_module(module);
        modules.push(
            ConfigurationModule::new(
                "object",
                module,
                body,
                ModuleKind::Object,
                ModuleExecution::new().with_server(true),
            )
            .with_owner("doc"),
        );
    }
    SessionFactory::new(
        ConfigurationCodeSnapshot::new("section-replay", modules).unwrap(),
        Arc::new(MetadataCatalog::new("section-replay", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn replay(factory: &SessionFactory, script: &str) -> String {
    use bsl_orm::Principal;
    use bsl_session::SessionResources;
    use open_bsl::{BslObject, Value, format_value};
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let script = script.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!("Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{script}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let result = session.call(&module, "Проверить", vec![]).unwrap().0;
    let Value::Object(result) = result else {
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
fn native_section_unloading_and_row_operations_match_in_two_sessions() {
    let factory = section_factory(None);
    for _ in 0..2 {
        assert_eq!(
            replay(
                &factory,
                include_str!(
                    "../../../../tests/conformance/measure/1c/measure-document-tabular-sections.bsl"
                )
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-tabular-sections.platform.txt"
            )
        );
        assert_eq!(
            replay(
                &factory,
                include_str!(
                    "../../../../tests/conformance/measure/1c/measure-document-tabular-sections-english.bsl"
                )
            ),
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-tabular-sections-english.platform.txt"
            )
        );
    }
}

#[test]
fn native_initialization_sections_read_only_numbers_and_identity_match() {
    let factory = section_factory(Some(include_str!(
        "../../../../tests/conformance/measure/1c/document-tabular-sections-module.bsl"
    )));
    assert_eq!(
        replay(
            &factory,
            include_str!(
                "../../../../tests/conformance/measure/1c/measure-document-tabular-sections-properties.bsl"
            )
        ),
        include_str!(
            "../../../../tests/conformance/measure/1c/measure-document-tabular-sections-properties.platform.txt"
        )
    );
}

#[test]
fn acquired_section_name_is_an_object_module_property() {
    let object_module = "Документ.ЗаказПокупателя.МодульОбъекта";
    let code = ConfigurationCodeSnapshot::new("section-context", vec![
        ConfigurationModule::new("session", "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба() Экспорт Возврат Документы.ЗаказПокупателя.СоздатьДокумент(); КонецФункции",
            ModuleKind::Session, ModuleExecution::new().with_server(true)),
        ConfigurationModule::new("object", object_module,
            "Функция ВыгрузитьЗапасы() Экспорт Возврат Запасы.Выгрузить(Новый Структура(\"ТипНоменклатурыЗапас, НомерВариантаКП\", Истина, 2)); КонецФункции",
            ModuleKind::Object, ModuleExecution::new().with_server(true)).with_owner("doc"),
    ]).unwrap();
    let object = MetadataObject::new("doc", "ЗаказПокупателя", MetadataKind::Document)
        .with_object_module(object_module)
        .with_document_tabular_sections(vec![DocumentTabularSection::new(
            "3827a494-db18-4d85-98ff-9376e84082d1",
            "Запасы",
        )]);
    // Здесь проверяется компиляция достигнутого исходного выражения.
    // Исполнение выгрузки доказывается отдельно нативным воспроизведением.
    SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("section-context", vec![object]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
}
