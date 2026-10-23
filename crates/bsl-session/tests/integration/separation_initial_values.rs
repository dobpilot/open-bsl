use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{
    CommonAttributeSeparationParameters, MetadataCatalog, MetadataKind, MetadataObject, Principal,
};
use bsl_session::{Session, SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, ColumnType, TypeId, TypeRef, Value, format_value};
use std::sync::Arc;

fn observation(session: &mut Session, source: &str) -> String {
    // Меняется только приёмник вывода нативной пробы.
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session.instantiate(&format!(
        "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции"
    )).unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("ожидался массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив результатов")
    };
    result
        .borrow()
        .iter()
        .map(|v| format_value(v, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn bound_values_match_native_inside_the_original_startup_handler() {
    assert_eq!(
        include_str!(
            "../../../../docs/research/fixtures/separation-initial-1017/string/input-xml/Ext/SessionModule.bsl"
        ),
        include_str!(
            "../../../../docs/research/fixtures/separation-initial-1017/number/input-xml/Ext/SessionModule.bsl"
        )
    );
    for (kind, source, expected) in [
        (
            TypeId::String,
            include_str!(
                "../../../../docs/research/fixtures/separation-initial-1017/measure-string.bsl"
            ),
            include_str!(
                "../../../../docs/research/fixtures/separation-initial-1017/measure-string.platform.txt"
            ),
        ),
        (
            TypeId::Number,
            include_str!(
                "../../../../docs/research/fixtures/separation-initial-1017/measure-number.bsl"
            ),
            include_str!(
                "../../../../docs/research/fixtures/separation-initial-1017/measure-number.platform.txt"
            ),
        ),
    ] {
        let code = ConfigurationCodeSnapshot::new(
            "separation-initial",
            vec![ConfigurationModule::new(
                "session", "МодульСеанса",
                include_str!("../../../../docs/research/fixtures/separation-initial-1017/string/input-xml/Ext/SessionModule.bsl"),
                ModuleKind::Session, ModuleExecution::new().with_server(true),
            )],
        ).unwrap();
        let catalog = MetadataCatalog::new(
            "separation-initial",
            vec![
                MetadataObject::new(
                    "attribute",
                    "НаименованиеЯзык1",
                    MetadataKind::CommonAttribute,
                )
                .with_common_attribute_data_separation(true),
                MetadataObject::new(
                    "use",
                    "ИспользованиеРазделителяПробы",
                    MetadataKind::SessionParameter,
                ),
                MetadataObject::new(
                    "value",
                    "ЗначениеРазделителяПробы",
                    MetadataKind::SessionParameter,
                ),
                MetadataObject::new(
                    "ordinary",
                    "ОбычныйПараметрПробы",
                    MetadataKind::SessionParameter,
                ),
                MetadataObject::new("trace", "НаблюдениеСтарта", MetadataKind::SessionParameter),
            ],
        )
        .unwrap()
        .with_common_attribute_separation_parameters(
            "attribute",
            CommonAttributeSeparationParameters {
                value_parameter: Some("value".into()),
                use_parameter: Some("use".into()),
                value_types: Some(vec![ColumnType {
                    id: TypeRef::Native(kind),
                    quals: vec![],
                }]),
            },
        )
        .unwrap();
        let factory = SessionFactory::new(
            code,
            Arc::new(catalog),
            "МодульСеанса",
            SessionKind::ThinClient,
            "8.3.27.2342",
        )
        .unwrap();
        let mut first = factory
            .open(
                Principal::new("user", "", vec![]),
                SessionResources::new(Box::new(()), 1),
            )
            .unwrap();
        assert_eq!(observation(&mut first, source), expected.trim());
        first
            .parameters()
            .set("ИспользованиеРазделителяПробы", Value::Boolean(true))
            .unwrap();
        first
            .parameters()
            .set(
                "ЗначениеРазделителяПробы",
                if kind == TypeId::Number {
                    Value::number_from_i64(17)
                } else {
                    Value::Str("parent".into())
                },
            )
            .unwrap();
        let mut second = factory
            .open(
                Principal::new("user", "", vec![]),
                SessionResources::new(Box::new(()), 2),
            )
            .unwrap();
        assert_eq!(observation(&mut second, source), expected.trim());
    }
}
