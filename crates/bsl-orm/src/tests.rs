use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Debug)]
struct DataAccessStub;

impl DataAccess for DataAccessStub {
    fn constant_value(
        &self,
        metadata: &MetadataObject,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        assert_eq!(metadata.id(), "constant-answer");
        assert_eq!(principal.id(), "user");
        Ok(Value::number_from_i64(42))
    }
}

#[derive(Debug, Default)]
struct QueryAccessStub {
    calls: RefCell<Vec<QueryCall>>,
}

type QueryCall = (String, Vec<(String, Value)>);

impl DataAccess for QueryAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        assert_eq!(principal.id(), "user");
        self.calls.borrow_mut().push((
            request.text().to_string(),
            request
                .parameters()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect(),
        ));
        Ok(QueryResultData::new(
            vec!["Значение".into()],
            vec![vec![Value::number_from_i64(42)]],
        )
        .unwrap())
    }
}

#[derive(Debug, Default)]
struct FunctionalOptionAccessStub {
    calls: RefCell<Vec<FunctionalOptionCall>>,
}

type FunctionalOptionCall = (String, Vec<(String, Value)>, String);

impl DataAccess for FunctionalOptionAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn functional_option(
        &self,
        request: &FunctionalOptionRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        self.calls.borrow_mut().push((
            request.name().to_string(),
            request
                .parameters()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect(),
            principal.id().to_string(),
        ));
        Ok(Value::Str(request.name().into()))
    }
}

#[derive(Debug, Default)]
struct TempTablesAccessStub {
    next: Cell<u64>,
    created: RefCell<Vec<TempTablesManagerHandle>>,
    closed: RefCell<Vec<TempTablesManagerHandle>>,
    queries: RefCell<Vec<Option<TempTablesManagerHandle>>>,
}

impl DataAccess for TempTablesAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn create_temp_tables_manager(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<TempTablesManagerHandle> {
        assert_eq!(principal.id(), "user");
        let handle = TempTablesManagerHandle::new(self.next.get() + 1);
        self.next.set(handle.value());
        self.created.borrow_mut().push(handle);
        Ok(handle)
    }

    fn close_temp_tables_manager(
        &self,
        manager: TempTablesManagerHandle,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        assert_eq!(principal.id(), "user");
        self.closed.borrow_mut().push(manager);
        Ok(())
    }

    fn execute_query(
        &self,
        request: &QueryRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.queries
            .borrow_mut()
            .push(request.temp_tables_manager());
        QueryResultData::new(Vec::new(), Vec::new()).map_err(|error| {
            RtError::DynamicError(format!("не удалось создать результат пробы: {error}"))
        })
    }
}

#[derive(Debug, Default)]
struct RecordingDataLocks {
    requests: RefCell<Vec<Vec<String>>>,
    principals: RefCell<Vec<String>>,
    transactions: RefCell<Vec<&'static str>>,
    transaction_active: Cell<bool>,
    exclusive: Cell<bool>,
    exclusive_changes: RefCell<Vec<bool>>,
    integration_processing: RefCell<Vec<bool>>,
    integration_services: RefCell<Vec<String>>,
    predefined_data_update: Cell<Option<PredefinedDataUpdateMode>>,
    predefined_data_updates: RefCell<Vec<PredefinedDataUpdateMode>>,
    predefined_initializations: Cell<usize>,
}

impl DataAccess for RecordingDataLocks {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "константы в этой пробе не читаются".into(),
        ))
    }

    fn lock_data(
        &self,
        requests: &[DataLockRequest],
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.requests.borrow_mut().push(
            requests
                .iter()
                .map(|request| request.lock_space().to_string())
                .collect(),
        );
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        Ok(())
    }

    fn begin_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.transactions.borrow_mut().push("begin");
        self.transaction_active.set(true);
        Ok(())
    }

    fn commit_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.transactions.borrow_mut().push("commit");
        self.transaction_active.set(false);
        Ok(())
    }

    fn rollback_transaction(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.transactions.borrow_mut().push("rollback");
        self.transaction_active.set(false);
        Ok(())
    }

    fn transaction_active(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<bool> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.transactions.borrow_mut().push("active");
        Ok(self.transaction_active.get())
    }

    fn exclusive_mode(&self, principal: &Principal, _: &mut CallContext<'_>) -> RtResult<bool> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        Ok(self.exclusive.get())
    }

    fn set_exclusive_mode(
        &self,
        enabled: bool,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.exclusive.set(enabled);
        self.exclusive_changes.borrow_mut().push(enabled);
        Ok(())
    }

    fn set_integration_services_processing(
        &self,
        enabled: bool,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.integration_processing.borrow_mut().push(enabled);
        Ok(())
    }

    fn integration_service(
        &self,
        name: &str,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.integration_services.borrow_mut().push(name.into());
        Ok(Value::Str(open_bsl::BslString::from_utf8_string(format!(
            "service:{name}"
        ))))
    }

    fn info_base_predefined_data_update(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<PredefinedDataUpdateMode> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        Ok(self
            .predefined_data_update
            .get()
            .unwrap_or(PredefinedDataUpdateMode::Auto))
    }

    fn set_info_base_predefined_data_update(
        &self,
        mode: PredefinedDataUpdateMode,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.predefined_data_update.set(Some(mode));
        self.predefined_data_updates.borrow_mut().push(mode);
        Ok(())
    }

    fn initialize_predefined_data(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.predefined_initializations
            .set(self.predefined_initializations.get() + 1);
        Ok(())
    }
}

#[derive(Debug)]
struct DeniedDataAccess;

impl DataAccess for DeniedDataAccess {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "права роли для чтения константы недоступны".into(),
        ))
    }

    fn execute_query(
        &self,
        _: &QueryRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        Err(RtError::DynamicError(
            "ограничение доступа запроса не раскрыто".into(),
        ))
    }
}

#[derive(Debug, Default)]
struct RecordingRights {
    calls: RefCell<Vec<RecordedAccessRightCall>>,
}

type RecordedAccessRightCall = (String, String, String, Option<String>, String);

impl DataAccess for RecordingRights {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError(
            "константы в этой пробе не читаются".into(),
        ))
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        let subject = match request.subject() {
            AccessRightSubject::CurrentPrincipal => "current".into(),
            AccessRightSubject::Principal(principal) => {
                format!("principal:{}", principal.id())
            }
            AccessRightSubject::Role(id) => format!("role:{id}"),
        };
        self.calls.borrow_mut().push((
            request.right().into(),
            request.metadata().id().into(),
            subject,
            request.standard_attribute().map(str::to_owned),
            principal.id().into(),
        ));
        Ok(true)
    }
}

const APPLIED_EXTENSION_ID: [u8; 16] = [0x11; 16];
const DISABLED_EXTENSION_ID: [u8; 16] = [0x22; 16];

fn extensions() -> ConfigurationExtensionsSnapshot {
    ConfigurationExtensionsSnapshot::new(
        vec![
            ConfigurationExtension::new(
                APPLIED_EXTENSION_ID,
                "Исправление",
                "1.2",
                "hash-one",
                ExtensionSafeMode::Enabled(true),
                false,
                true,
                ConfigurationExtensionScope::InfoBase,
            )
            .unwrap(),
            ConfigurationExtension::new(
                DISABLED_EXTENSION_ID,
                "Адаптация",
                "2.0",
                "hash-two",
                ExtensionSafeMode::profile("ПрофильРасширения"),
                true,
                false,
                ConfigurationExtensionScope::DataSeparation,
            )
            .unwrap(),
        ],
        [APPLIED_EXTENSION_ID],
        [DISABLED_EXTENSION_ID],
        vec![ExtensionApplicationIssue::new(
            DISABLED_EXTENSION_ID,
            "конфликт метода",
            ExtensionApplicationIssueSeverity::Critical,
        )],
    )
    .unwrap()
}

fn catalog() -> Arc<MetadataCatalog> {
    Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("role-user", "Пользователь", MetadataKind::Role),
                MetadataObject::new("language-ru", "Русский", MetadataKind::Language),
                MetadataObject::new("language-en", "English", MetadataKind::Language),
                MetadataObject::new("role-admin", "Администратор", MetadataKind::Role),
                MetadataObject::new("enum-statuses", "Статусы", MetadataKind::Enumeration),
                MetadataObject::new("exchange-plan-main", "Основной", MetadataKind::ExchangePlan),
                MetadataObject::new(
                    "picture-information",
                    "ИнформацияКонфигурации",
                    MetadataKind::CommonPicture,
                ),
                MetadataObject::new(
                    "picture-warning",
                    "ПредупреждениеКонфигурации",
                    MetadataKind::CommonPicture,
                ),
                MetadataObject::new("catalog-versions", "Версии", MetadataKind::Catalog)
                    .with_manager_module("ВерсииМенеджер"),
                MetadataObject::new("constant-answer", "Ответ", MetadataKind::Constant)
                    .with_manager_module("ОтветМенеджер"),
                MetadataObject::new(
                    "scheduled-job-cleanup",
                    "ОчисткаДанных",
                    MetadataKind::ScheduledJob,
                )
                .with_string_property("ИмяМетода", "Обслуживание.ОчиститьДанные"),
            ],
        )
        .unwrap()
        .with_configuration(
            "Демонстрационная",
            "1.2.3",
            ScriptVariant::Russian,
            CompatibilityMode::Version8_3_24,
            ["role-user".into()],
        )
        .unwrap()
        .with_primary_language("language-ru")
        .unwrap()
        .with_enumeration_values(vec![ConfigurationEnumValue::new(
            "enum-statuses",
            "enum-status-active",
            "Активен",
        )])
        .unwrap()
        .with_configuration_extensions(extensions())
        .with_configuration_changed(false),
    )
}

#[test]
fn predefined_value_resolves_configuration_and_system_enumerations() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Значение = ПредопределенноеЗначение("Перечисление.статусы.активен");
                Англ = PredefinedValue("Enum.СТАТУСЫ.АКТИВЕН");
                Возврат Строка(Значение) + ":" + Строка(Значение = Англ)
                    + ":" + Строка(ПредопределенноеЗначение("ОтправкаЭлементаДанных.Авто"));
                "#,
            )
            .unwrap(),
        Value::Str("Активен:Да:Auto".into())
    );
    assert!(
        state
            .exec("Возврат ПредопределенноеЗначение(\"Перечисление.Статусы.Нет\");")
            .unwrap_err()
            .to_string()
            .contains("не найдено")
    );
}

fn engine() -> open_bsl::Engine {
    open_bsl::Engine::builder()
        .register_library(library())
        .common_module(
            "ВерсииМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Версии" Тогда
                    ВызватьИсключение "неверный менеджер";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .common_module(
            "ОтветМенеджер",
            r#"
            Функция Получить() Экспорт
                Возврат 999;
            КонецФункции
            "#,
        )
        .build()
        .unwrap()
}

fn state(engine: &open_bsl::Engine) -> open_bsl::State {
    state_with_data_access(engine, None)
}

fn state_with_data_access(
    engine: &open_bsl::Engine,
    data_access: Option<Rc<dyn DataAccess>>,
) -> open_bsl::State {
    let mut session = SessionData::new(
        catalog(),
        Principal::new("user", "Тест", vec!["role-user".into()]),
    );
    if let Some(data_access) = data_access {
        session = session.with_data_access(data_access);
    }
    engine.state_builder().host_data(Rc::new(session)).build()
}

#[test]
fn current_info_base_user_is_frozen_per_state() {
    let engine = engine();
    let make_state = |name: &str| {
        engine
            .state_builder()
            .host_data(Rc::new(SessionData::new(
                catalog(),
                Principal::new("user", name, vec!["role-user".into()]),
            )))
            .build()
    };
    let mut first = make_state("Первый");
    let mut second = make_state("Второй");

    assert_eq!(
        first
            .exec("Возврат ПользователиИнформационнойБазы.ТекущийПользователь().Имя;")
            .unwrap(),
        Value::Str("Первый".into())
    );
    assert_eq!(
        second
            .exec("Возврат InfoBaseUsers.CurrentUser().Name;")
            .unwrap(),
        Value::Str("Второй".into())
    );
    assert!(
        first
            .exec("ПользователиИнформационнойБазы.ТекущийПользователь().Имя = \"Подмена\";")
            .unwrap_err()
            .to_string()
            .contains("Имя")
    );
    assert_eq!(
        first.exec("Возврат ИмяПользователя();").unwrap(),
        Value::Str("Первый".into())
    );
    assert_eq!(
        second.exec("Возврат UserName();").unwrap(),
        Value::Str("Второй".into())
    );
}

#[test]
fn enumeration_manager_is_resolved_from_the_session_catalog() {
    let engine = engine();
    let mut state = state(&engine);

    assert!(
        state
            .exec("Возврат Перечисления.сТаТуСы;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат Enums.СТАТУСЫ;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат Перечисления.Отсутствует;")
            .unwrap_err()
            .to_string()
            .contains("Отсутствует")
    );
}

#[test]
fn exchange_plan_manager_and_metadata_use_the_session_catalog() {
    let engine = engine();
    let mut state = state(&engine);

    assert!(
        state
            .exec("Возврат ПланыОбмена.оСнОвНоЙ;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат ExchangePlans.ОСНОВНОЙ;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат Метаданные.ПланыОбмена.Основной;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат Metadata.ExchangePlans.ОСНОВНОЙ;")
            .unwrap()
            .object_ref()
            .is_some()
    );
    assert!(
        state
            .exec("Возврат ПланыОбмена.Отсутствует;")
            .unwrap_err()
            .to_string()
            .contains("Отсутствует")
    );
}

#[test]
fn data_lock_delegates_named_spaces_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                Блокировка = Новый БлокировкаДанных;
                Элемент = Блокировка.Добавить("Catalog.Test");
                Блокировка.Заблокировать();
                Другая = New DataLock;
                ДругойЭлемент = Другая.Add();
                Возврат Строка(ТипЗнч(Блокировка)) + ":"
                    + Строка(ТипЗнч(Элемент)) + ":"
                    + Строка(ТипЗнч(Другая) = ТипЗнч(Блокировка)) + ":"
                    + Строка(ТипЗнч(ДругойЭлемент) = ТипЗнч(Элемент));
                "#,
            )
            .unwrap(),
        Value::Str("Data lock:Data lock item:Да:Да".into())
    );
    assert_eq!(
        *access.requests.borrow(),
        vec![vec!["Catalog.Test".to_string()]]
    );
    assert_eq!(*access.principals.borrow(), vec!["user".to_string()]);

    let mut missing = state_with_data_access(&engine, None);
    assert!(
        missing
            .exec("Блокировка = Новый БлокировкаДанных; Блокировка.Заблокировать();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
}

#[test]
fn transaction_procedures_delegate_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    state
        .exec(
            r#"
Если ТранзакцияАктивна() Тогда ВызватьИсключение "initial"; КонецЕсли;
НачатьТранзакцию();
Если Не TransactionActive() Тогда ВызватьИсключение "active"; КонецЕсли;
ЗафиксироватьТранзакцию();
BeginTransaction();
RollbackTransaction();
Если TransactionActive() Тогда ВызватьИсключение "finished"; КонецЕсли;
"#,
        )
        .unwrap();
    assert_eq!(
        *access.transactions.borrow(),
        vec![
            "active", "begin", "active", "commit", "begin", "rollback", "active"
        ]
    );
    assert_eq!(access.principals.borrow().as_slice(), ["user"; 7]);

    let mut missing = state_with_data_access(&engine, None);
    assert!(
        missing
            .exec("НачатьТранзакцию();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
    assert!(
        missing
            .exec("ТранзакцияАктивна();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
}

#[test]
fn exclusive_mode_delegates_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                До = МонопольныйРежим();
                УстановитьМонопольныйРежим(Истина);
                ВоВремя = ExclusiveMode();
                SetExclusiveMode(False);
                Возврат Строка(До) + ":" + Строка(ВоВремя) + ":"
                    + Строка(МонопольныйРежим());
                "#,
            )
            .unwrap(),
        Value::Str("Нет:Да:Нет".into())
    );
    assert_eq!(*access.exclusive_changes.borrow(), [true, false]);
    assert_eq!(access.principals.borrow().as_slice(), ["user"; 5]);

    let mut missing = state_with_data_access(&engine, None);
    assert!(
        missing
            .exec("Возврат МонопольныйРежим();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
}

#[test]
fn integration_services_delegate_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                СервисыИнтеграции.ВыполнитьОбработку();
                IntegrationServices.StopProcessing();
                Возврат СервисыИнтеграции.Тестовый;
                "#,
            )
            .unwrap(),
        Value::Str("service:Тестовый".into())
    );
    assert_eq!(*access.integration_processing.borrow(), [true, false]);
    assert_eq!(
        access.integration_services.borrow().as_slice(),
        ["Тестовый"]
    );
    assert_eq!(access.principals.borrow().as_slice(), ["user"; 3]);

    let mut missing = state_with_data_access(&engine, None);
    assert!(
        missing
            .exec("СервисыИнтеграции.ВыполнитьОбработку();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
}

#[test]
fn predefined_data_update_delegates_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                До = ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы();
                УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы(
                    ОбновлениеПредопределенныхДанных.ОбновлятьАвтоматически);
                После = GetInfoBasePredefinedData();
                SetInfoBasePredefinedDataUpdate(PredefinedDataUpdate.DontAutoUpdate);
                Возврат Строка(До) + ":" + Строка(После) + ":"
                    + Строка(ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы());
                "#,
            )
            .unwrap(),
        Value::Str("Auto:Auto update:Auto update off".into())
    );
    assert_eq!(
        *access.predefined_data_updates.borrow(),
        [
            PredefinedDataUpdateMode::AutoUpdate,
            PredefinedDataUpdateMode::DontAutoUpdate
        ]
    );
    assert_eq!(access.principals.borrow().as_slice(), ["user"; 5]);

    assert!(
        state
            .exec("УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы();")
            .unwrap_err()
            .to_string()
            .contains("аргумент")
    );
}

#[test]
fn predefined_data_initialization_delegates_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    state
        .exec("ИнициализироватьПредопределенныеДанные(); InitializePredefinedData();")
        .unwrap();
    assert_eq!(access.predefined_initializations.get(), 2);
    assert_eq!(access.principals.borrow().as_slice(), ["user"; 2]);

    let mut missing = state_with_data_access(&engine, None);
    assert!(
        missing
            .exec("ИнициализироватьПредопределенныеДанные();")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных")
    );
}

#[test]
fn picture_library_returns_measured_opaque_picture_values() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Картинка1 = БиблиотекаКартинок.информацияконфигурации;
                Картинка1Англ = PictureLib.ИНФОРМАЦИЯКОНФИГУРАЦИИ;
                Картинка2 = БиблиотекаКартинок.ПредупреждениеКонфигурации;
                Возврат Строка(ТипЗнч(Картинка1)) + ":" + Строка(Картинка1)
                    + ":" + Строка(ЗначениеЗаполнено(Картинка1))
                    + ":" + Строка(Картинка1 = Картинка1Англ)
                    + ":" + Строка(Картинка1 = Картинка2);
                "#,
            )
            .unwrap(),
        Value::Str("Picture:Picture:Да:Да:Нет".into())
    );
    assert!(
        state
            .exec("Возврат БиблиотекаКартинок.Отсутствует;")
            .unwrap_err()
            .to_string()
            .contains("Отсутствует")
    );
}

#[test]
fn configuration_changed_reads_the_frozen_catalog_state() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state.exec("Возврат КонфигурацияИзменена();").unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state.exec("Возврат ConfigurationChanged();").unwrap(),
        Value::Boolean(false)
    );

    let catalog = Arc::new(MetadataCatalog::new("configuration", vec![]).unwrap());
    let session = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut missing = engine.state_builder().host_data(Rc::new(session)).build();
    assert!(
        missing
            .exec("Возврат КонфигурацияИзменена();")
            .unwrap_err()
            .to_string()
            .contains("не предоставлен")
    );
}

#[test]
fn query_uses_the_session_adapter_and_exposes_a_measured_linear_result() {
    let engine = engine();
    let access = Rc::new(QueryAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                ЗапросПробы = Новый Запрос;
                Если Строка(ТипЗнч(ЗапросПробы)) <> "Query"
                    ИЛИ ЗапросПробы.Текст <> "" Тогда
                    Возврат Ложь;
                КонецЕсли;
                ЗапросПробы.Текст = "ВЫБРАТЬ &Значение КАК Значение";
                ЗапросПробы.УстановитьПараметр("Значение", 1);
                ЗапросПробы.УстановитьПараметр("зНаЧеНиЕ", 42);
                Результат = ЗапросПробы.Выполнить();
                Если Строка(ТипЗнч(Результат)) <> "Query result"
                    ИЛИ Результат.Пустой() Тогда
                    Возврат Ложь;
                КонецЕсли;
                Выборка = Результат.Выбрать();
                Если Строка(ТипЗнч(Выборка)) <> "Selected data from query result"
                    ИЛИ Выборка.Значение <> Неопределено
                    ИЛИ НЕ Выборка.Следующий()
                    ИЛИ Выборка.Значение <> 42
                    ИЛИ Выборка.Получить(0) <> 42
                    ИЛИ Выборка[0] <> 42
                    ИЛИ Выборка.Следующий()
                    ИЛИ Выборка.Значение <> 42 Тогда
                    Возврат Ложь;
                КонецЕсли;
                Таблица = Результат.Выгрузить();
                Возврат Строка(ТипЗнч(Таблица)) = "ТаблицаЗначений"
                    И Таблица.Количество() = 1
                    И Таблица[0].Значение = 42;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );

    assert_eq!(
        access.calls.borrow().as_slice(),
        [(
            "ВЫБРАТЬ &Значение КАК Значение".into(),
            vec![("Значение".into(), Value::number_from_i64(42))],
        )]
    );
}

#[test]
fn query_fails_closed_without_support_and_rejects_non_linear_traversal() {
    let engine = engine();
    let mut without_access = state(&engine);
    let error = without_access
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("доступ к данным конфигурации не подключён")
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    let error = unsupported
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить();")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("выполнение запросов не поддерживается адаптером данных")
    );

    let mut state = state_with_data_access(&engine, Some(Rc::new(QueryAccessStub::default())));
    let error = state
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить().Выбрать(1);")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("поддерживается только прямой обход по умолчанию")
    );
}

#[test]
fn functional_option_uses_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(FunctionalOptionAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                Параметры = Новый Структура;
                Параметры.Вставить("Первый", 1);
                Параметры.Вставить("Второй", "два");
                Русский = ПолучитьФункциональнуюОпцию("Опция", Параметры);
                English = GetFunctionalOption("Option");
                Возврат Русский + ":" + English;
                "#,
            )
            .unwrap(),
        Value::Str("Опция:Option".into())
    );
    assert_eq!(
        access.calls.borrow().as_slice(),
        [
            (
                "Опция".into(),
                vec![
                    ("Первый".into(), Value::number_from_i64(1)),
                    ("Второй".into(), Value::Str("два".into())),
                ],
                "user".into(),
            ),
            ("Option".into(), Vec::new(), "user".into()),
        ]
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("Возврат ПолучитьФункциональнуюОпцию(\"Опция\");")
            .unwrap_err()
            .to_string()
            .contains("функциональные опции не поддерживаются")
    );
}

#[test]
fn temporary_tables_manager_stays_on_its_session_adapter() {
    let engine = engine();
    let access = Rc::new(TempTablesAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                Менеджер = Новый МенеджерВременныхТаблиц;
                Первый = Новый Запрос("ВЫБРАТЬ 1");
                Второй = New Query("ВЫБРАТЬ 2");
                Если Первый.МенеджерВременныхТаблиц <> Неопределено Тогда
                    Возврат Ложь;
                КонецЕсли;
                Первый.МенеджерВременныхТаблиц = Менеджер;
                Второй.TempTablesManager = Менеджер;
                Первый.Выполнить();
                Второй.Execute();
                Менеджер.Close();
                Менеджер.Закрыть();
                Первый.Выполнить();
                Возврат Истина;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
    let handle = TempTablesManagerHandle::new(1);
    assert_eq!(access.created.borrow().as_slice(), [handle]);
    assert_eq!(
        access.queries.borrow().as_slice(),
        [Some(handle), Some(handle), Some(handle)]
    );
    assert_eq!(access.closed.borrow().as_slice(), [handle, handle]);

    let other_access = Rc::new(TempTablesAccessStub::default());
    let mut other_state = state_with_data_access(&engine, Some(other_access));
    let foreign_manager = state
        .exec("Возврат Новый МенеджерВременныхТаблиц;")
        .unwrap();
    let query = other_state.exec("Возврат Новый Запрос;").unwrap();
    let module = engine
        .compile(
            "Процедура Назначить(ЗапросПробы, Менеджер) Экспорт ЗапросПробы.МенеджерВременныхТаблиц = Менеджер; КонецПроцедуры",
        )
        .unwrap();
    let instance = other_state.instantiate(&module).unwrap();
    assert!(
        instance
            .call(&mut other_state, "Назначить", vec![query, foreign_manager],)
            .unwrap_err()
            .to_string()
            .contains("другому сеансу")
    );
}

#[test]
fn denied_or_incomplete_data_access_is_not_replaced_with_a_value() {
    let engine = engine();
    let mut state = state_with_data_access(&engine, Some(Rc::new(DeniedDataAccess)));

    let query_error = state
        .exec("Возврат Новый Запрос(\"ВЫБРАТЬ 1\").Выполнить();")
        .unwrap_err();
    assert!(
        query_error
            .to_string()
            .contains("ограничение доступа запроса не раскрыто")
    );

    let constant_error = state
        .exec("Возврат Константы.Ответ.Получить();")
        .unwrap_err();
    assert!(
        constant_error
            .to_string()
            .contains("права роли для чтения константы недоступны")
    );
}

#[test]
fn query_result_data_rejects_ambiguous_or_misaligned_rows() {
    assert!(matches!(
        QueryResultData::new(vec!["Поле".into(), "пОлЕ".into()], Vec::<Vec<Value>>::new()),
        Err(QueryResultDataError::DuplicateColumnName { name }) if name == "пОлЕ"
    ));
    assert!(matches!(
        QueryResultData::new(vec!["Поле".into()], vec![Vec::new()]),
        Err(QueryResultDataError::RowWidth {
            row: 0,
            expected: 1,
            actual: 0,
        })
    ));
}

#[test]
fn value_storage_snapshots_and_detaches_mutable_values() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Сжатие = Новый СжатиеДанных;
                Исходный = Новый Массив;
                Исходный.Добавить(1);
                Хранилище = Новый ХранилищеЗначения(Исходный, Сжатие);
                Исходный[0] = 2;
                Первое = Хранилище.Получить();
                Первое[0] = 3;
                Второе = Хранилище.Получить();

                Общий = Новый Массив;
                Общий.Добавить(4);
                САлиасом = Новый Массив;
                САлиасом.Добавить(Общий);
                САлиасом.Добавить(Общий);
                Извлеченный = Новый ХранилищеЗначения(
                    САлиасом, Новый Deflation(6)).Get();
                Извлеченный[0][0] = 5;

                Возврат Строка(ТипЗнч(Сжатие)) = "Compress Data"
                    И Строка(ТипЗнч(Хранилище)) = "Value storage"
                    И Второе[0] = 1
                    И Извлеченный[1][0] = 4;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn value_storage_rejects_invalid_compression_and_nonportable_values() {
    let engine = engine();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый СжатиеДанных(-2);",
        "Возврат Новый СжатиеДанных(10);",
        "Возврат Новый СжатиеДанных(1.5);",
        "Возврат Новый ХранилищеЗначения(1, 1);",
        "Возврат Новый ХранилищеЗначения(Новый Запрос);",
    ] {
        assert!(state.exec(source).is_err(), "должен быть отказ: {source}");
    }
}

#[test]
fn role_available_accepts_a_name_and_a_metadata_role() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec("Возврат РольДоступна(\"пОлЬзОвАтЕлЬ\");")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат IsInRole(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert!(
        state
            .exec("Возврат РольДоступна(\"НетТакойРоли\");")
            .unwrap_err()
            .to_string()
            .contains("роль конфигурации не найдена")
    );
}

#[test]
fn access_right_preserves_current_user_role_and_standard_attribute() {
    let engine = engine();
    let access = Rc::new(RecordingRights::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec("Возврат ПравоДоступа(\"Чтение\", Метаданные.Справочники.Версии);")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec(
                "Возврат AccessRight(\"Read\", Metadata.Catalogs.Версии, Metadata.Roles.Администратор, \"Код\");"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec(
                "Возврат ПравоДоступа(\"Использование\", Метаданные.Справочники.Версии, ПользователиИнформационнойБазы.ТекущийПользователь());"
            )
            .unwrap(),
        Value::Boolean(true)
    );

    assert_eq!(
        *access.calls.borrow(),
        vec![
            (
                "Чтение".into(),
                "catalog-versions".into(),
                "current".into(),
                None,
                "user".into(),
            ),
            (
                "Read".into(),
                "catalog-versions".into(),
                "role:role-admin".into(),
                Some("Код".into()),
                "user".into(),
            ),
            (
                "Использование".into(),
                "catalog-versions".into(),
                "principal:user".into(),
                None,
                "user".into(),
            ),
        ]
    );
}

#[test]
fn access_right_rejects_missing_adapter_and_invalid_arguments() {
    let engine = engine();
    let mut state = state(&engine);
    let error = state
        .exec("Возврат ПравоДоступа(\"Чтение\", Метаданные.Справочники.Версии);")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("адаптер данных текущего сеанса не подключён"),
        "{error}"
    );

    let access = Rc::new(RecordingRights::default());
    let mut state = state_with_data_access(&engine, Some(access));
    for source in [
        "Возврат ПравоДоступа(\"\", Метаданные.Справочники.Версии);",
        "Возврат ПравоДоступа(\"Чтение\", 1);",
        "Возврат ПравоДоступа(\"Чтение\", Метаданные.Справочники.Версии, 1);",
        "Возврат ПравоДоступа(\"Чтение\", Метаданные.Справочники.Версии, , \"\");",
    ] {
        assert!(state.exec(source).is_err(), "должен быть отказ: {source}");
    }
}

#[test]
fn configuration_metadata_exposes_name_version_language_and_main_roles() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec("Возврат Метаданные.Имя + \" \" + Метаданные.Версия;")
            .unwrap(),
        Value::Str("Демонстрационная 1.2.3".into())
    );
    assert_eq!(
        state
            .exec(
                "Возврат Метаданные.ВариантВстроенногоЯзыка = ВариантВстроенногоЯзыка.Русский И Метаданные.ВариантВстроенногоЯзыка = Метаданные.СвойстваОбъектов.ВариантВстроенногоЯзыка[\"Russian\"] ;"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.ОсновныеРоли.Содержит(Метаданные.Роли.Пользователь);")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.DefaultRoles.Contains(Метаданные.Roles.Администратор);")
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        state
            .exec(
                r#"
                Имена = "";
                Для Каждого Задание Из Метаданные.РегламентныеЗадания Цикл
                    Имена = Имена + Задание.Имя + ":" + Задание.ИмяМетода;
                КонецЦикла;
                Возврат Имена;
                "#,
            )
            .unwrap(),
        Value::Str("ОчисткаДанных:Обслуживание.ОчиститьДанные".into())
    );
    assert_eq!(
        state
            .exec("Возврат Метаданные.Роли[0] = Метаданные.Роли.Пользователь;")
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn current_language_uses_the_principal_then_the_configuration_fallback() {
    let engine = engine();
    let catalog = catalog();
    let mut fallback = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::clone(&catalog),
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    assert_eq!(
        fallback
            .exec("Возврат ТекущийЯзык() = Метаданные.Языки.Русский И CurrentLanguage().Имя = \"Русский\";")
            .unwrap(),
        Value::Boolean(true)
    );

    let principal =
        Principal::new("user", "Тест", Vec::new()).with_interface_language_id("language-en");
    let mut selected = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(Arc::clone(&catalog), principal)))
        .build();
    assert_eq!(
        selected.exec("Возврат Строка(ТекущийЯзык());").unwrap(),
        Value::Str("English".into())
    );
    assert_eq!(
        fallback.exec("Возврат Строка(CurrentLanguage());").unwrap(),
        Value::Str("Русский".into())
    );

    let invalid =
        Principal::new("user", "Тест", Vec::new()).with_interface_language_id("language-missing");
    let mut invalid_state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(catalog, invalid)))
        .build();
    assert!(invalid_state.exec("Возврат ТекущийЯзык();").is_err());
}

#[test]
fn configuration_metadata_exposes_the_measured_compatibility_mode() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Режим = Метаданные.РежимСовместимости;
                Варианты = Метаданные.СвойстваОбъектов.РежимСовместимости;
                Возврат Строка(ТипЗнч(Режим)) = "Compatibility mode"
                    И Строка(Режим) = "Version8_3_24"
                    И Строка(ТипЗнч(Варианты)) = "Metadata object enumerated property"
                    И Строка(Варианты) = "MetadataObjectsPropertyEnumed"
                    И Режим = Варианты["версия8_3_24"]
                    И Режим = Метаданные.ObjectProperties.CompatibilityMode["VERSION8_3_24"];
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );

    assert!(
        engine
            .compile("Возврат РежимСовместимости;")
            .err()
            .expect("голое имя режима совместимости не должно разрешаться")
            .to_string()
            .contains("переменная «РежимСовместимости» читается до присваивания")
    );
    for type_name in ["РежимСовместимости", "CompatibilityMode"] {
        assert!(
            state
                .exec(&format!("Возврат Тип(\"{type_name}\");"))
                .unwrap_err()
                .to_string()
                .contains(&format!("тип «{type_name}» не определён"))
        );
    }
}

#[test]
fn missing_configuration_properties_and_invalid_main_roles_fail_explicitly() {
    let missing = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![MetadataObject::new(
                "role-user",
                "Пользователь",
                MetadataKind::Role,
            )],
        )
        .unwrap(),
    );
    let engine = engine();
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            missing,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    assert!(
        state
            .exec("Возврат Метаданные.Имя;")
            .unwrap_err()
            .to_string()
            .contains("свойства конфигурации отсутствуют")
    );

    let error = MetadataCatalog::new(
        "configuration",
        vec![MetadataObject::new(
            "role-user",
            "Пользователь",
            MetadataKind::Role,
        )],
    )
    .unwrap()
    .with_configuration(
        "Конфигурация",
        "1",
        ScriptVariant::Russian,
        CompatibilityMode::DontUse,
        ["missing".into()],
    )
    .unwrap_err();
    assert_eq!(error, CatalogError::UnknownMainRole("missing".into()));
}

#[test]
fn manager_export_runs_in_the_session_module_instance() {
    let engine = engine();
    let entry = engine
        .compile(
            r#"
            Функция Проверка() Экспорт
                Возврат Справочники.Версии.Проверить(10);
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = state(&engine);
    let instance = state.instantiate(&entry).unwrap();
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(11)
    );
    assert_eq!(
        instance.call(&mut state, "Проверка", vec![]).unwrap().0,
        Value::number_from_i64(12)
    );
}

#[test]
fn platform_manager_method_wins_over_same_named_module_export() {
    let engine = engine();
    let mut state = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert_eq!(
        state.exec("Возврат Константы.Ответ.Получить();").unwrap(),
        Value::number_from_i64(42)
    );
}

#[test]
fn platform_manager_method_does_not_fall_back_when_data_access_is_missing() {
    let engine = engine();
    let mut state = state(&engine);
    let error = state
        .exec("Возврат Константы.Ответ.Получить();")
        .unwrap_err()
        .to_string();
    assert!(error.contains("доступ к данным конфигурации не подключён"));
    assert!(!error.contains("999"));
}

#[test]
fn unknown_manager_member_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);
    assert!(
        state
            .exec("Возврат Константы.НетТакойКонстанты;")
            .unwrap_err()
            .to_string()
            .contains("НетТакойКонстанты")
    );
    assert!(
        state
            .exec("Возврат Справочники.Версии.НетТакогоМетода();")
            .unwrap_err()
            .to_string()
            .contains("НетТакогоМетода")
    );
}

#[test]
fn configuration_extensions_filter_each_complete_source() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Активные = РасширенияКонфигурации.Получить(
                    Новый Структура("Имя", "иСпРаВлЕнИе"),
                    ИсточникРасширенийКонфигурации.СеансАктивные);
                Если Активные.Количество() <> 1 Тогда
                    Возврат "wrong applied count";
                КонецЕсли;
                Расширение = Активные[0];
                ПоИдентификатору = ConfigurationExtensions.Get(
                    Новый Структура("UUID", Расширение.UUID),
                    ConfigurationExtensionsSource.Database);
                Возврат Расширение.Name + ":" + Расширение.Version + ":"
                    + Расширение.HashSum + ":" + Строка(Расширение.SafeMode) + ":"
                    + Строка(Расширение.UsedInDistributedInfoBase) + ":"
                    + Строка(Расширение.Active) + ":" + Строка(Расширение.Scope)
                    + ":" + Строка(ПоИдентификатору.Количество());
                "#,
            )
            .unwrap(),
        Value::Str("Исправление:1.2:hash-one:Да:Нет:Да:Infobase:1".into())
    );

    assert_eq!(
        state
            .exec(
                "Возврат РасширенияКонфигурации.Получить(, ИсточникРасширенийКонфигурации.СеансОтключенные).Количество();"
            )
            .unwrap(),
        Value::number_from_i64(1)
    );
}

#[test]
fn configuration_extension_issues_refer_to_the_same_extension_snapshot() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Проблема = РасширенияКонфигурации
                    .ПолучитьИнформациюОПроблемахПримененияВСеансе()[0];
                Отключенное = РасширенияКонфигурации.Получить(
                    , ИсточникРасширенийКонфигурации.СеансОтключенные)[0];
                Возврат Проблема.Расширение = Отключенное
                    И Проблема.Описание = "конфликт метода"
                    И Проблема.Важность = ВажностьПроблемыПримененияРасширенияКонфигурации.Критичная;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn complete_empty_and_missing_extension_snapshots_are_distinct() {
    let empty_catalog = Arc::new(
        MetadataCatalog::new("configuration", Vec::new())
            .unwrap()
            .with_configuration_extensions(
                ConfigurationExtensionsSnapshot::new(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )
                .unwrap(),
            ),
    );
    let missing_catalog = Arc::new(MetadataCatalog::new("configuration", Vec::new()).unwrap());
    let engine = engine();
    let make_state = |catalog| {
        engine
            .state_builder()
            .host_data(Rc::new(SessionData::new(
                catalog,
                Principal::new("user", "Тест", Vec::new()),
            )))
            .build()
    };

    let mut empty_state = make_state(empty_catalog);
    assert_eq!(
        empty_state
            .exec("Возврат РасширенияКонфигурации.Получить().Количество();")
            .unwrap(),
        Value::number_from_i64(0)
    );
    assert_eq!(
        empty_state
            .exec(
                "Возврат РасширенияКонфигурации.ПолучитьИнформациюОПроблемахПримененияВСеансе().Количество();"
            )
            .unwrap(),
        Value::number_from_i64(0)
    );

    let mut missing_state = make_state(missing_catalog);
    assert!(
        missing_state
            .exec("Возврат РасширенияКонфигурации.Получить();")
            .unwrap_err()
            .to_string()
            .contains("не загружены для текущего сеанса")
    );
}

#[test]
fn extension_snapshot_rejects_dangling_references() {
    let extension = ConfigurationExtension::new(
        APPLIED_EXTENSION_ID,
        "Исправление",
        "1",
        "hash",
        ExtensionSafeMode::Enabled(false),
        false,
        true,
        ConfigurationExtensionScope::InfoBase,
    )
    .unwrap();
    assert_eq!(
        ConfigurationExtensionsSnapshot::new(
            vec![extension.clone()],
            [DISABLED_EXTENSION_ID],
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err(),
        CatalogError::UnknownExtensionIdentity
    );
    assert_eq!(
        ConfigurationExtensionsSnapshot::new(
            vec![extension],
            Vec::new(),
            Vec::new(),
            vec![ExtensionApplicationIssue::new(
                DISABLED_EXTENSION_ID,
                "нет расширения",
                ExtensionApplicationIssueSeverity::Low,
            )],
        )
        .unwrap_err(),
        CatalogError::UnknownIssueExtension
    );
}
