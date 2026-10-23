use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{BslObject, Value, format_value};
use std::sync::Arc;

#[test]
fn subsystem_scope_replays_the_native_nested_configuration() {
    let objects = vec![
        MetadataObject::new("parent", "Родитель", MetadataKind::Subsystem)
            .with_subsystem_command_interface(true),
        MetadataObject::new("other", "Другой", MetadataKind::Subsystem)
            .with_subsystem_command_interface(false),
        MetadataObject::new("child", "Дочерняя", MetadataKind::Subsystem)
            .with_subsystem_parent("parent")
            .with_subsystem_command_interface(false),
    ];
    let code = ConfigurationCodeSnapshot::new(
        "subsystem-scope",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("subsystem-scope", objects).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    // Меняется только приёмник вывода; выражения и обработка ошибки — нативные.
    let source = include_str!("../../../docs/research/fixtures/subsystem-scope-1017/measure.bsl")
        .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let module = session
        .instantiate(&format!(
            "Функция Проверить() Экспорт РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы; КонецФункции"
        ))
        .unwrap();
    let Value::Object(result) = session.call(&module, "Проверить", vec![]).unwrap().0
    else {
        panic!("ожидался массив результатов")
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("ожидался массив результатов")
    };
    let actual = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        include_str!("../../../docs/research/fixtures/subsystem-scope-1017/measure.platform.txt")
            .trim()
    );
}
