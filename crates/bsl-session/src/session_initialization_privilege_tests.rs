use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataKind, MetadataObject};

fn factory(fail_selective: bool) -> SessionFactory {
    let selective = if fail_selective {
        "ВызватьИсключение \"selective-entry-error\";"
    } else {
        "ПараметрыСеанса.LazyPrivilege = ПривилегированныйРежим();"
    };
    let source = format!(
        "Процедура УстановкаПараметровСеанса(Имена)
         Если Имена = Неопределено Тогда
         ПараметрыСеанса.InitialPrivilege = ПривилегированныйРежим();
         Иначе {selective} КонецЕсли;
         КонецПроцедуры"
    );
    factory_from_source(&source)
}

fn factory_from_source(source: &str) -> SessionFactory {
    let code = ConfigurationCodeSnapshot::new(
        "session-entry-privilege",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            source,
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "session-entry-privilege",
        vec![
            MetadataObject::new(
                "initial",
                "InitialPrivilege",
                MetadataKind::SessionParameter,
            ),
            MetadataObject::new("lazy", "LazyPrivilege", MetadataKind::SessionParameter),
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

fn open(factory: &SessionFactory, id: u64) -> Session {
    factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), id),
        )
        .unwrap()
}

fn evaluate(session: &mut Session, expression: &str) -> Value {
    let module = session
        .instantiate(&format!(
            "Функция Контроль() Экспорт Возврат {expression}; КонецФункции"
        ))
        .unwrap();
    session.call(&module, "Контроль", vec![]).unwrap().0
}

#[test]
fn automatic_initial_and_selective_entries_match_the_native_modes() {
    let factory = factory(false);
    let mut session = open(&factory, 1);
    assert_eq!(
        session.parameters().peek("InitialPrivilege").unwrap(),
        Some(Value::Boolean(true))
    );
    assert_eq!(
        evaluate(&mut session, "ПривилегированныйРежим()"),
        Value::Boolean(false)
    );
    assert_eq!(session.parameters().peek("LazyPrivilege").unwrap(), None);
    assert_eq!(
        evaluate(&mut session, "ПараметрыСеанса.LazyPrivilege"),
        Value::Boolean(true)
    );
    assert_eq!(
        evaluate(&mut session, "ПривилегированныйРежим()"),
        Value::Boolean(false)
    );
    let mut other = open(&factory, 2);
    assert_eq!(other.parameters().peek("LazyPrivilege").unwrap(), None);
    assert_eq!(
        evaluate(&mut other, "ПривилегированныйРежим()"),
        Value::Boolean(false)
    );
}

#[test]
fn an_exception_in_selective_initialization_does_not_elevate_the_caller() {
    let mut session = open(&factory(true), 1);
    let module = session
        .instantiate(
            "Функция Контроль() Экспорт
         Попытка ЗначениеПробы = ПараметрыСеанса.LazyPrivilege;
         Исключение Возврат ПривилегированныйРежим(); КонецПопытки;
         ВызватьИсключение \"missing-error\"; КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Контроль", vec![]).unwrap().0,
        Value::Boolean(false)
    );
    assert_eq!(session.parameters().peek("LazyPrivilege").unwrap(), None);
}

#[test]
fn a_generic_handler_call_keeps_its_ordinary_default() {
    let mut session = open(&factory(false), 1);
    session
        .state
        .call_configuration_handler(
            "МодульСеанса",
            "УстановкаПараметровСеанса",
            vec![Value::Undefined],
            open_bsl::ConfigurationHandlerEntryMode::Inherit,
        )
        .unwrap();
    assert_eq!(
        session.parameters().peek("InitialPrivilege").unwrap(),
        Some(Value::Boolean(false))
    );
}

#[test]
fn automatic_privilege_is_not_a_decrementable_enable_in_the_handler_frame() {
    let factory = factory_from_source(
        "Процедура УстановкаПараметровСеанса(Имена)
            ОшибкаСнятия = Ложь;
            Попытка УстановитьПривилегированныйРежим(Ложь);
            Исключение ОшибкаСнятия = Истина; КонецПопытки;
            УстановитьПривилегированныйРежим(Истина);
            УстановитьПривилегированныйРежим(Ложь);
            РезультатПробы = ОшибкаСнятия И ПривилегированныйРежим();
            Если Имена = Неопределено Тогда
                ПараметрыСеанса.InitialPrivilege = РезультатПробы;
            Иначе ПараметрыСеанса.LazyPrivilege = РезультатПробы; КонецЕсли;
         КонецПроцедуры",
    );
    let mut session = open(&factory, 1);
    assert_eq!(
        session.parameters().peek("InitialPrivilege").unwrap(),
        Some(Value::Boolean(true))
    );
    assert_eq!(
        evaluate(&mut session, "ПараметрыСеанса.LazyPrivilege"),
        Value::Boolean(true)
    );
    assert_eq!(
        evaluate(&mut session, "ПривилегированныйРежим()"),
        Value::Boolean(false)
    );
}
