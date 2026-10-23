use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    DocumentAttribute, DocumentNumberSettings, DocumentNumberType, MetadataCatalog, MetadataKind,
    MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{
    BslObject, ColumnType, GraphLimits, RuntimeShapes, SerializedValueGraph, TypeRef, Value,
};
use std::sync::Arc;

fn factory() -> SessionFactory {
    let enumeration = MetadataObject::new("enum", "ЗамерПеречисления", MetadataKind::Enumeration)
        .with_reference_storage_number(54);
    let document = MetadataObject::new("doc", "ЗамерДокумента", MetadataKind::Document)
        .with_document_number_settings(DocumentNumberSettings {
            number_type: DocumentNumberType::String,
            number_length: 9,
        });
    let declarations = Arc::new(
        MetadataCatalog::new(
            "metadata-attributes",
            vec![enumeration.clone(), document.clone()],
        )
        .unwrap(),
    );
    let ty = declarations.value_types().next().unwrap().clone();
    let mut builder = open_bsl::Engine::builder();
    let mut value = Value::Undefined;
    for declaration in declarations.transferable_value_types() {
        if declaration.names()[0] == "ПеречислениеСсылка.ЗамерПеречисления"
        {
            value = declaration
                .materialize(b"7664d5d3-bca6-4114-8409-3272f9787060")
                .unwrap();
        }
        builder = builder.register_value_type(declaration);
    }
    let engine = builder.build().unwrap();
    let shapes = RuntimeShapes::seeded(vec![], vec![], Some(engine.registry()));
    let graph = SerializedValueGraph::capture(&[value], &shapes, &GraphLimits::default()).unwrap();
    let attributes = ["СсылкаПробы", "СоставнойПробы"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let mut types = vec![ColumnType::plain(TypeRef::Value(Arc::clone(&ty)))];
            if index == 1 {
                types.push(ColumnType {
                    id: TypeRef::Native(open_bsl::TypeId::String),
                    quals: vec!["12".into()],
                });
            }
            DocumentAttribute::new(name, name, types)
                .with_fill_from_filling_value(true)
                .with_fill_value(graph.clone())
        })
        .collect();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "metadata-attributes",
            vec![enumeration, document.with_document_attributes(attributes)],
        )
        .unwrap(),
    );
    let code = ConfigurationCodeSnapshot::new("metadata-attributes", vec![ConfigurationModule::new(
        "session", "МодульСеанса",
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция СоздатьПробу() Экспорт Возврат Документы.ЗамерДокумента.СоздатьДокумент(); КонецФункции",
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    SessionFactory::new(
        code,
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap()
}

fn replay(session: &mut Session, source: &str) -> String {
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let (declarations, body) = if source.starts_with("Функция") {
        let (declaration, body) = source.split_once("КонецФункции").unwrap();
        (format!("{declaration}КонецФункции"), body)
    } else {
        (String::new(), source.as_str())
    };
    let module = session.instantiate(&format!("{declarations}\nФункция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{body}\nВозврат РезультатыПробы; КонецФункции")).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("массив наблюдений")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив наблюдений")
    };
    result
        .borrow()
        .iter()
        .map(|value| {
            let Value::Str(text) = value else {
                panic!("строка наблюдения")
            };
            text.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn full_missing_reference_probe_matches_native_metadata_creation_and_filling() {
    let factory = factory();
    let script =
        include_str!("../../../tests/conformance/measure/1c/measure-dangling-enum-filling.bsl");
    for (language, oracle) in [
        (
            "en",
            include_str!(
                "../../../tests/conformance/measure/1c/measure-dangling-enum-filling.platform.txt"
            ),
        ),
        (
            "ru",
            include_str!(
                "../../../tests/conformance/measure/1c/measure-dangling-enum-storage-russian-ui.platform.txt"
            ),
        ),
    ] {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), 1).with_system_language(language),
            )
            .unwrap();
        assert_eq!(
            replay(&mut session, script),
            oracle.lines().take(7).collect::<Vec<_>>().join("\n")
        );
    }
}

#[test]
fn attribute_collection_matches_native_order_names_aliases_and_identity() {
    let factory = factory();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_system_language("en"),
        )
        .unwrap();
    let script = include_str!(
        "../../../tests/conformance/measure/1c/measure-document-attribute-metadata.bsl"
    );
    let actual = replay(&mut session, script).replace("=Да", "=Yes");
    assert_eq!(
        actual,
        include_str!(
            "../../../tests/conformance/measure/1c/measure-document-attribute-metadata.platform.txt"
        )
        .trim()
    );
}
