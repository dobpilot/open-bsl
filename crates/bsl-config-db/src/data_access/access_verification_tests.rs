use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{CompatibilityMode, MetadataCatalog, ScriptVariant};
use bsl_session::{
    EventLogEntry, EventLogLevel, EventLogTransactionMode, EventLogWriter, Session, SessionFactory,
    SessionKind, SessionResources,
};
use open_sdbl::metadata::{DbNames, SchemaStorage, parse_role_rights, resolve_metadata};
use std::sync::Arc;

#[path = "metadata_right_matrix_tests.rs"]
mod metadata_right_matrix;

const ROOT: &str = "26c30ece-33b5-498a-b409-a91a046ba66b";
const ALLOW: &str = "9d3bec1a-c27a-4d3b-94e1-45dd6c85cf81";
const DENY: &str = "114a9841-1e1b-487a-8340-d833179eb15f";
const VERIFY: &str = "ВыполнитьПроверкуПравДоступа(\"СохранениеДанныхПользователя\", Метаданные);";

struct Host(AccessStore);
impl std::fmt::Debug for Host {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Host")
    }
}
impl DataAccess for Host {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        panic!("чтение константы не запрашивалось")
    }
    fn verify_access_rights(
        &self,
        request: &bsl_orm::AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        access_verification::verify_access_rights(&self.0, request, principal, context)
    }
}

#[derive(Debug, Default)]
struct Journal {
    entries: RefCell<Vec<EventLogEntry>>,
    failing: bool,
    attempts: LocalCell<usize>,
}
impl EventLogWriter for Journal {
    fn write(&self, entry: EventLogEntry) -> RtResult<()> {
        self.attempts.set(self.attempts.get() + 1);
        if self.failing {
            return Err(error("недоступен журнал пробы"));
        }
        self.entries.borrow_mut().push(entry);
        Ok(())
    }
}

fn open(allowed: bool, writer: Option<Rc<Journal>>) -> Session {
    let metadata = resolve_metadata(
        DbNames::from_entries(vec![]),
        vec![],
        SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let mut access = AccessStore::new(&metadata);
    access.insert_rights(ALLOW.parse().unwrap(), parse_role_rights(include_bytes!("../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/RootAllow.rights.deflate")).unwrap());
    access.insert_rights(DENY.parse().unwrap(), parse_role_rights(include_bytes!("../../../../docs/research/fixtures/configuration-access-right-complete-native-1015/RootDeny.rights.deflate")).unwrap());
    let code = ConfigurationCodeSnapshot::new(
        "verification",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    let catalog = MetadataCatalog::new(
        "verification",
        vec![MetadataObject::new(
            "1316b080-3b2c-501b-ace3-a1234a3fedd1",
            "ВладелецПробы",
            MetadataKind::Catalog,
        )],
    )
    .unwrap()
    .with_configuration_object_id(ROOT)
    .unwrap()
    .with_configuration(
        "Probe",
        "1",
        ScriptVariant::English,
        CompatibilityMode::DontUse,
        [],
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
    let mut resources =
        SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(Host(access)));
    if let Some(writer) = writer {
        resources = resources.with_event_log_writer(writer);
    }
    factory
        .open(
            Principal::new(
                "probe",
                "probe",
                [if allowed {
                    ALLOW.to_owned()
                } else {
                    DENY.to_owned()
                }],
            ),
            resources,
        )
        .unwrap()
}

#[test]
fn a_denied_root_writes_the_native_independent_event_and_remains_catchable() {
    let journal = Rc::new(Journal::default());
    let mut session = open(false, Some(journal.clone()));
    let module = session.instantiate(&format!(
        "Функция ПроверитьОтказ() Экспорт Попытка {VERIFY} Исключение Возврат \"caught\"; КонецПопытки; Возврат \"missed\"; КонецФункции"
    )).unwrap();
    let result = session.call(&module, "ПроверитьОтказ", vec![]).unwrap().0;
    assert_eq!(result, Value::Str("caught".into()));
    let data = {
        let entries = journal.entries.borrow();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.event_name(), "_$Access$_.AccessDenied");
        assert_eq!(entry.level(), EventLogLevel::Information);
        assert_eq!(entry.metadata(), &Value::Undefined);
        assert_eq!(entry.comment(), "");
        assert_eq!(
            entry.transaction_mode(),
            EventLogTransactionMode::Independent
        );
        entry.data().clone()
    };
    let module = session
        .instantiate("Функция ПравоПробы(Данные) Экспорт Возврат Данные.Right; КонецФункции")
        .unwrap();
    assert_eq!(
        session.call(&module, "ПравоПробы", vec![data]).unwrap().0,
        Value::Str("SaveUserData".into())
    );
}

#[test]
fn grants_and_native_privileged_bypass_emit_no_denial() {
    for allowed in [true, false] {
        let journal = Rc::new(Journal::default());
        let mut session = open(allowed, Some(journal.clone()));
        let source = if allowed {
            VERIFY.to_owned()
        } else {
            format!("УстановитьПривилегированныйРежим(Истина); {VERIFY}")
        };
        session.execute(&source).unwrap();
        assert!(journal.entries.borrow().is_empty());
        assert_eq!(journal.attempts.get(), 0);
    }
}

#[test]
fn absence_or_failure_of_the_session_journal_never_reports_verified_success() {
    for allowed in [true, false] {
        assert!(
            open(allowed, None)
                .execute(VERIFY)
                .unwrap_err()
                .to_string()
                .contains("журнал регистрации не подключён")
        );
    }
    let journal = Rc::new(Journal {
        failing: true,
        ..Journal::default()
    });
    let mut session = open(false, Some(journal.clone()));
    assert!(
        session
            .execute(VERIFY)
            .unwrap_err()
            .to_string()
            .contains("недоступен журнал пробы")
    );
    assert_eq!(journal.attempts.get(), 1);
    assert!(journal.entries.borrow().is_empty());
}

#[test]
fn two_sessions_deliver_only_to_their_own_journals() {
    let first = Rc::new(Journal::default());
    let second = Rc::new(Journal::default());
    let mut one = open(false, Some(first.clone()));
    let mut two = open(false, Some(second.clone()));
    for _ in 0..2 {
        assert!(one.execute(VERIFY).is_err());
    }
    assert!(two.execute(VERIFY).is_err());
    assert_eq!(first.entries.borrow().len(), 2);
    assert_eq!(second.entries.borrow().len(), 1);
    assert!(open(false, None).execute(VERIFY).is_err());
    assert_eq!(first.entries.borrow().len(), 2);
    assert_eq!(second.entries.borrow().len(), 1);
}

#[test]
fn an_object_denial_keeps_the_original_validated_metadata_value() {
    let journal = Rc::new(Journal::default());
    let mut session = open(false, Some(journal.clone()));
    let reader = session.instantiate("Функция ПрочитатьМета() Экспорт Возврат Метаданные.Справочники.ВладелецПробы; КонецФункции").unwrap();
    let value = session.call(&reader, "ПрочитатьМета", vec![]).unwrap().0;
    let module = session
        .instantiate(
            "Процедура Проверить(Мета) Экспорт VerifyAccessRights(\"Read\", Мета); КонецПроцедуры",
        )
        .unwrap();
    assert!(
        session
            .call(&module, "Проверить", vec![value.clone()])
            .is_err()
    );
    let entries = journal.entries.borrow();
    assert_eq!(entries.len(), 1);
    assert!(std::ptr::eq(
        value.object_ref().unwrap(),
        entries[0].metadata().object_ref().unwrap()
    ));
}
