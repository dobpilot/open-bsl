use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::BslDate;
use std::sync::Arc;

fn factory(kind: SessionKind, read_during_startup: bool) -> SessionFactory {
    let body = if read_during_startup {
        "ПараметрыСеанса.Замер = ПолучитьТекущийСеансИнформационнойБазы().НачалоСеанса;"
    } else {
        ""
    };
    let code = ConfigurationCodeSnapshot::new(
        "started",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            format!("Процедура УстановкаПараметровСеанса(Имена) Экспорт {body} КонецПроцедуры"),
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "started",
        vec![MetadataObject::new(
            "sample",
            "Замер",
            MetadataKind::SessionParameter,
        )],
    )
    .unwrap();
    SessionFactory::new(code, Arc::new(catalog), "МодульСеанса", kind, "8.3.27.2342").unwrap()
}

fn date(day: u32) -> BslDate {
    BslDate::from_civil(2001, 1, day, 3, 4, 5).unwrap()
}
fn principal() -> Principal {
    Principal::new("user", "Тест", vec![])
}

#[test]
fn native_controls_and_startup_observe_the_supplied_date() {
    let mut session = factory(SessionKind::ThinClient, true)
        .open(
            principal(),
            SessionResources::new(Box::new(()), 1).with_session_started(date(2)),
        )
        .unwrap();
    session.execute("Если ПараметрыСеанса.Замер <> Дата(2001, 1, 2, 3, 4, 5) Тогда ВызватьИсключение \"startup date\"; КонецЕсли;").unwrap();
    let source = include_str!("../../../../docs/research/fixtures/session-started/measure.bsl")
        .replace("Сообщить(", "РезультатПробы.Добавить(")
        .replace(
            "КонецПроцедуры",
            "КонецПроцедуры\nРезультатПробы = Новый Массив;",
        );
    let expected =
        include_str!("../../../../docs/research/fixtures/session-started/measure.platform.txt");
    let mut assertions = format!(
        "Если РезультатПробы.Количество() <> {} Тогда ВызватьИсключение \"line count\"; КонецЕсли;",
        expected.lines().count()
    );
    for (index, line) in expected.lines().enumerate() {
        let escaped = line.replace('"', "\"\"");
        assertions.push_str(&format!("\nЕсли РезультатПробы[{index}] <> \"{escaped}\" Тогда ВызватьИсключение \"native control {index}\"; КонецЕсли;"));
    }
    session
        .execute(&format!("Перем РезультатПробы;\n{source}\n{assertions}"))
        .unwrap();
}

#[test]
fn missing_start_time_is_explicit_on_read_and_at_initialization() {
    let mut session = factory(SessionKind::ThinClient, false)
        .open(principal(), SessionResources::new(Box::new(()), 1))
        .unwrap();
    let error = session
        .execute("ЗамерПробы = ПолучитьТекущийСеансИнформационнойБазы().SessionStarted;")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("начало сеанса информационной базы не передано хостом")
    );
    let error = factory(SessionKind::ThinClient, true)
        .open(principal(), SessionResources::new(Box::new(()), 1))
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("начало сеанса информационной базы не передано хостом")
    );
}

#[test]
fn separate_foreground_and_background_sessions_keep_their_own_dates() {
    let mut parent = factory(SessionKind::ThinClient, true)
        .open(
            principal(),
            SessionResources::new(Box::new("parent"), 1).with_session_started(date(2)),
        )
        .unwrap();
    let mut other = factory(SessionKind::ThinClient, true)
        .open(
            principal(),
            SessionResources::new(Box::new("other"), 3).with_session_started(date(4)),
        )
        .unwrap();
    let mut child = factory(SessionKind::BackgroundJob, true)
        .open_background(
            principal(),
            SessionResources::new(Box::new("child"), 2).with_session_started(date(3)),
            "Модуль.Метод",
            &parent,
        )
        .unwrap();
    for (session, day) in [(&mut parent, 2), (&mut child, 3), (&mut other, 4)] {
        session.execute(&format!("Если ПолучитьТекущийСеансИнформационнойБазы().НачалоСеанса <> Дата(2001, 1, {day}, 3, 4, 5) Тогда ВызватьИсключение \"session isolation\"; КонецЕсли;")).unwrap();
    }
}
