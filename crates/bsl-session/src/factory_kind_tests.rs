use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataKind, MetadataObject};

fn factory() -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "factory-kinds",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Перем СчетчикМодуля;
        Процедура УстановкаПараметровСеанса(Имена)
            Если СчетчикМодуля = Неопределено Тогда СчетчикМодуля = 0; КонецЕсли;
            СчетчикМодуля = СчетчикМодуля + 1;
            ПараметрыСеанса.НомерИнициализации = СчетчикМодуля;
        КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "factory-kinds",
        vec![
            MetadataObject::new(
                "count",
                "НомерИнициализации",
                MetadataKind::SessionParameter,
            ),
            MetadataObject::new("inherited", "Наследуемый", MetadataKind::SessionParameter),
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
    .with_connection_string("Srvr=\"fixture\";Ref=\"factory\";")
    .unwrap()
}

fn reported_kind(session: &mut Session) -> Value {
    let module = session
        .instantiate(
            "Функция Вид() Экспорт Возврат КлиентскоеПриложение.ТипПриложения(); КонецФункции",
        )
        .unwrap();
    session.call(&module, "Вид", vec![]).unwrap().0
}

#[test]
fn a_reused_background_factory_exposes_the_russian_current_job_method_name() {
    let factory = factory();
    let principal = Principal::new("user", "", vec![]);
    let parent = factory
        .open(principal.clone(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let mut child = factory
        .with_kind(SessionKind::BackgroundJob)
        .open_background(
            principal,
            SessionResources::new(Box::new(()), 2),
            "ДлительныеОперации.СообщитьПрогресс",
            &parent,
        )
        .unwrap();
    let module = child
        .instantiate(
            "Функция Контроль() Экспорт
                ФоновоеЗадание = ПолучитьТекущийСеансИнформационнойБазы().ПолучитьФоновоеЗадание();
                Возврат НРег(ФоновоеЗадание.ИмяМетода);
            КонецФункции",
        )
        .unwrap();
    assert_eq!(
        child.call(&module, "Контроль", vec![]).unwrap().0,
        Value::Str("длительныеоперации.сообщитьпрогресс".into())
    );
}

#[test]
fn factory_kind_selection_reuses_the_engine_and_isolates_session_state() {
    let factory = factory();
    let web = factory.clone().with_kind(SessionKind::WebClient);
    let background = factory.clone().with_kind(SessionKind::BackgroundJob);
    assert!(std::ptr::eq(
        factory.engine.registry(),
        web.engine.registry()
    ));
    assert!(std::ptr::eq(
        factory.engine.registry(),
        background.engine.registry()
    ));
    assert!(Arc::ptr_eq(&factory.catalog, &background.catalog));
    assert!(Arc::ptr_eq(
        factory.connection_string.as_ref().unwrap(),
        background.connection_string.as_ref().unwrap()
    ));
    assert_eq!(factory.kind, SessionKind::ThinClient);
    let principal = Principal::new("user", "", vec![]);
    let mut parent = factory
        .open(principal.clone(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let mut other = web
        .open(principal.clone(), SessionResources::new(Box::new(()), 2))
        .unwrap();
    parent
        .parameters()
        .set("Наследуемый", Value::number_from_i64(17))
        .unwrap();
    let mut child = background
        .open_background(
            principal.clone(),
            SessionResources::new(Box::new(()), 3),
            "Модуль.Контроль",
            &parent,
        )
        .unwrap();
    for session in [&parent, &other, &child] {
        assert_eq!(
            session.parameters().peek("НомерИнициализации").unwrap(),
            Some(Value::number_from_i64(1))
        );
    }
    assert_eq!(
        reported_kind(&mut parent),
        Value::Enum(EnumValue::ClientApplicationTypeThinClient)
    );
    assert_eq!(
        reported_kind(&mut other),
        Value::Enum(EnumValue::ClientApplicationTypeWebClient)
    );
    assert_eq!(reported_kind(&mut child), Value::Undefined);
    assert!(!Rc::ptr_eq(parent.parameters(), child.parameters()));
    assert_eq!(
        child.parameters().peek("Наследуемый").unwrap(),
        Some(Value::number_from_i64(17))
    );
    child
        .parameters()
        .set("Наследуемый", Value::number_from_i64(27))
        .unwrap();
    assert_eq!(
        parent.parameters().peek("Наследуемый").unwrap(),
        Some(Value::number_from_i64(17))
    );
    assert!(matches!(
        factory.open_background(
            principal,
            SessionResources::new(Box::new(()), 4),
            "Модуль.Контроль",
            &parent
        ),
        Err(SessionError::NotBackgroundFactory)
    ));
}
