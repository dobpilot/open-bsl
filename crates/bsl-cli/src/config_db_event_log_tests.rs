use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, Principal};
use bsl_session::{EventLogEntry, EventLogWriter, SessionFactory, SessionKind, SessionResources};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Debug, Default)]
struct Capture(RefCell<Vec<EventLogEntry>>);
impl EventLogWriter for Capture {
    fn write(&self, entry: EventLogEntry) -> open_bsl::RtResult<()> {
        self.0.borrow_mut().push(entry);
        Ok(())
    }
}

fn event(mode: &str) -> EventLogEntry {
    let code = ConfigurationCodeSnapshot::new("cli-log", vec![ConfigurationModule::new(
        "session", "МодульСеанса",
        format!("Процедура УстановкаПараметровСеанса(Имена) ЗаписьЖурналаРегистрации(\"Контроль\", УровеньЖурналаРегистрации.Ошибка, Метаданные, 17, \"Комментарий\", РежимТранзакцииЗаписиЖурналаРегистрации.{mode}); КонецПроцедуры"),
        ModuleKind::Session, ModuleExecution::new().with_server(true),
    )]).unwrap();
    let factory = SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("cli-log", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let capture = Rc::new(Capture::default());
    factory
        .open(
            Principal::new("user", "", vec![]),
            SessionResources::new(Box::new(()), 1).with_event_log_writer(capture.clone()),
        )
        .unwrap();
    capture.0.borrow_mut().pop().unwrap()
}

#[test]
fn independent_bsl_event_preserves_fields_in_the_host_destination() {
    let mut bytes = vec![];
    let entry = event("Независимая");
    let metadata = open_bsl::format_value(entry.metadata(), None).unwrap();
    assert!(!metadata.is_empty());
    write_event(&mut bytes, entry).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("Контроль") && text.contains("Комментарий"),
        "{text}"
    );
    assert!(
        text.contains("Error") && text.contains("Данные: 17"),
        "{text}"
    );
    assert!(text.contains(&format!("Метаданные: {metadata}")), "{text}");
}

#[test]
fn transactional_delivery_and_broken_destination_fail_explicitly() {
    let mut bytes = vec![];
    assert!(write_event(&mut bytes, event("Транзакционная")).is_err());
    assert!(bytes.is_empty());
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("destination unavailable"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(
        write_event(&mut Broken, event("Независимая"))
            .unwrap_err()
            .to_string()
            .contains("destination unavailable")
    );
}
