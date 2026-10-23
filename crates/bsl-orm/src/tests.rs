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

#[derive(Debug)]
struct NoQueryResultAccessStub;

impl DataAccess for NoQueryResultAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn execute_query_result(
        &self,
        _: &QueryRequest,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Option<QueryResultData>> {
        Ok(None)
    }
}

#[derive(Debug, Default)]
struct PasswordPolicyAccessStub {
    calls: RefCell<Vec<(Option<String>, String)>>,
}

impl DataAccess for PasswordPolicyAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn user_password_policy(
        &self,
        name: Option<&str>,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Option<UserPasswordPolicySnapshot>> {
        self.calls
            .borrow_mut()
            .push((name.map(str::to_owned), principal.id().to_owned()));
        Ok((name == Some("Строгая")).then(|| UserPasswordPolicySnapshot::new(12)))
    }

    fn user_password_minimum_length(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<u32> {
        self.calls
            .borrow_mut()
            .push((Some("<минимум>".into()), principal.id().to_owned()));
        Ok(9)
    }
}

#[derive(Debug)]
struct AuthenticationLockAccessStub {
    settings: AuthenticationLockSettingsSnapshot,
    reads: RefCell<Vec<String>>,
    writes: RefCell<Vec<(AuthenticationLockSettingsSnapshot, String)>>,
}

#[derive(Debug)]
struct PasswordStrengthAccessStub {
    enabled: bool,
    reads: RefCell<Vec<String>>,
}

#[derive(Debug)]
struct PasswordNumericAccessStub {
    values: [i64; 4],
    reads: RefCell<Vec<(UserPasswordNumericSetting, String)>>,
}

impl PasswordNumericAccessStub {
    fn new(values: [i64; 4]) -> Self {
        Self {
            values,
            reads: RefCell::new(Vec::new()),
        }
    }
}

impl DataAccess for PasswordNumericAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn user_password_numeric_setting(
        &self,
        setting: UserPasswordNumericSetting,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<BslNumber> {
        self.reads
            .borrow_mut()
            .push((setting, principal.id().to_owned()));
        let index = match setting {
            UserPasswordNumericSetting::MaxEffectivePeriod => 0,
            UserPasswordNumericSetting::MinEffectivePeriod => 1,
            UserPasswordNumericSetting::ReuseLimit => 2,
            UserPasswordNumericSetting::ExpirationNotificationPeriod => 3,
        };
        Ok(BslNumber::from_i64(self.values[index]))
    }
}

#[derive(Debug, Default)]
struct PasswordSettingsWriteStub {
    writes: RefCell<Vec<(UserPasswordSetting, Value, String)>>,
}

impl DataAccess for PasswordSettingsWriteStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn set_user_password_setting(
        &self,
        setting: UserPasswordSetting,
        value: &Value,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.writes
            .borrow_mut()
            .push((setting, value.clone(), principal.id().to_owned()));
        Ok(())
    }
}

impl PasswordStrengthAccessStub {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            reads: RefCell::new(Vec::new()),
        }
    }
}

impl DataAccess for PasswordStrengthAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn user_password_strength_check(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        self.reads.borrow_mut().push(principal.id().to_owned());
        Ok(self.enabled)
    }
}

impl AuthenticationLockAccessStub {
    fn new(lock_duration: i64, max_attempts: i64, codes: &str) -> Self {
        Self {
            settings: AuthenticationLockSettingsSnapshot::new(
                BslNumber::from_i64(lock_duration),
                BslNumber::from_i64(max_attempts),
                codes,
            ),
            reads: RefCell::new(Vec::new()),
            writes: RefCell::new(Vec::new()),
        }
    }
}

impl DataAccess for AuthenticationLockAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn authentication_lock_settings(
        &self,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<AuthenticationLockSettingsSnapshot> {
        self.reads.borrow_mut().push(principal.id().to_owned());
        Ok(self.settings.clone())
    }

    fn set_authentication_lock_settings(
        &self,
        settings: &AuthenticationLockSettingsSnapshot,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.writes
            .borrow_mut()
            .push((settings.clone(), principal.id().to_owned()));
        Ok(())
    }
}

#[derive(Debug, Default)]
struct QueryAccessStub {
    calls: RefCell<Vec<QueryCall>>,
}

#[derive(Debug, Default)]
struct FindByRefAccessStub {
    calls: RefCell<Vec<FindByRefCall>>,
}

type FindByRefCall = (usize, bool, bool, bool, String);

impl DataAccess for FindByRefAccessStub {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("константы не поддерживаются".into()))
    }

    fn find_by_ref(
        &self,
        request: &FindByRefRequest,
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<QueryResultData> {
        self.calls.borrow_mut().push((
            request.references().collection_len()?,
            request.search_area().is_some(),
            request.include_objects().is_some(),
            request.exclude_objects().is_some(),
            principal.id().into(),
        ));
        QueryResultData::new(
            vec![
                "Ref".into(),
                "Data".into(),
                "Metadata".into(),
                "DataSeparation".into(),
            ],
            Vec::new(),
        )
        .map_err(|error| RtError::DynamicError(error.to_string()))
    }
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
    navigation_shapes: RefCell<Vec<Vec<bool>>>,
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

    fn navigation_link(
        &self,
        arguments: &[Value],
        principal: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<String> {
        self.principals
            .borrow_mut()
            .push(principal.id().to_string());
        self.navigation_shapes.borrow_mut().push(
            arguments
                .iter()
                .map(|argument| matches!(argument, Value::Undefined))
                .collect(),
        );
        Ok("e1cib/data/test".into())
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
    deny_verification: Cell<bool>,
    denial_events: Cell<usize>,
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

    fn verify_access_rights(
        &self,
        request: &AccessRightRequest,
        principal: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        self.access_right(request, principal, context)?;
        if self.deny_verification.get() {
            self.denial_events.set(self.denial_events.get() + 1);
            return Err(RtError::DynamicError("доступ запрещён".into()));
        }
        Ok(())
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
                "55K0byUrioB3aHqMgHSxrIug2Ls=",
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
                MetadataObject::new("subsystem-sales", "Продажи", MetadataKind::Subsystem)
                    .with_subsystem_command_interface(true),
                MetadataObject::new("subsystem-purchases", "Закупки", MetadataKind::Subsystem)
                    .with_subsystem_command_interface(false),
                MetadataObject::new("subsystem-sales-prices", "Цены", MetadataKind::Subsystem)
                    .with_subsystem_parent("subsystem-sales")
                    .with_subsystem_command_interface(true),
                MetadataObject::new(
                    "subsystem-purchases-prices",
                    "Цены",
                    MetadataKind::Subsystem,
                )
                .with_subsystem_parent("subsystem-purchases")
                .with_subsystem_command_interface(false),
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
                MetadataObject::new("document-order", "Заказ", MetadataKind::Document)
                    .with_manager_module("ЗаказМенеджер"),
                MetadataObject::new(
                    "document-journal-orders",
                    "Заказы",
                    MetadataKind::DocumentJournal,
                )
                .with_manager_module("ЗаказыЖурналМенеджер"),
                MetadataObject::new("report-sales", "Продажи", MetadataKind::Report)
                    .with_manager_module("ПродажиОтчетМенеджер"),
                MetadataObject::new(
                    "data-processor-import",
                    "Импорт",
                    MetadataKind::DataProcessor,
                )
                .with_manager_module("ИмпортОбработкаМенеджер"),
                MetadataObject::new(
                    "accumulation-register-stock",
                    "Остатки",
                    MetadataKind::AccumulationRegister,
                ),
                MetadataObject::new(
                    "accounting-register-main",
                    "Основной",
                    MetadataKind::AccountingRegister,
                ),
                MetadataObject::new(
                    "calculation-register-payroll",
                    "Начисления",
                    MetadataKind::CalculationRegister,
                ),
                MetadataObject::new(
                    "chart-of-accounts-main",
                    "Основной",
                    MetadataKind::ChartOfAccounts,
                ),
                MetadataObject::new(
                    "chart-of-characteristic-types-properties",
                    "Свойства",
                    MetadataKind::ChartOfCharacteristicTypes,
                ),
                MetadataObject::new(
                    "chart-of-calculation-types-main",
                    "Основной",
                    MetadataKind::ChartOfCalculationTypes,
                ),
                MetadataObject::new(
                    "business-process-approval",
                    "Согласование",
                    MetadataKind::BusinessProcess,
                ),
                MetadataObject::new("task-performer", "Исполнитель", MetadataKind::Task),
                MetadataObject::new(
                    "settings-storage-user",
                    "Пользовательские",
                    MetadataKind::SettingsStorage,
                ),
                MetadataObject::new("sequence-documents", "Документы", MetadataKind::Sequence),
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
                    + ":" + Строка(ПредопределенноеЗначение("ОтправкаЭлементаДанных.Авто"))
                    + ":" + ПолучитьПолноеИмяПредопределенногоЗначения(Значение)
                    + ":" + GetPredefinedValueFullName(
                        ПредопределенноеЗначение("ОтправкаЭлементаДанных.Авто"));
                "#,
            )
            .unwrap(),
        Value::Str(
            "Активен:Да:Авто:Перечисление.Статусы.Активен:ОтправкаЭлементаДанных.Авто".into()
        )
    );
    assert!(
        state
            .exec("Возврат ПредопределенноеЗначение(\"Перечисление.Статусы.Нет\");")
            .unwrap_err()
            .to_string()
            .contains("не найдено")
    );
    assert!(
        state
            .exec("Возврат ПолучитьПолноеИмяПредопределенногоЗначения(1);")
            .unwrap_err()
            .to_string()
            .contains("недоступно")
    );
}

#[test]
fn navigation_link_preserves_positions_and_uses_the_session_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingDataLocks::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state.exec("Возврат GetURL(\"object\", , 3);").unwrap(),
        Value::Str("e1cib/data/test".into())
    );
    assert_eq!(
        *access.navigation_shapes.borrow(),
        vec![vec![false, true, false]]
    );
    assert_eq!(
        access.principals.borrow().last().map(String::as_str),
        Some("user")
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("Возврат ПолучитьНавигационнуюСсылку(\"object\");")
            .unwrap_err()
            .to_string()
            .contains("не поддерживаются")
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
        .common_module(
            "ЗаказМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Заказ" Тогда
                    ВызватьИсключение "неверный менеджер документа";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .common_module(
            "ЗаказыЖурналМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Заказы" Тогда
                    ВызватьИсключение "неверный менеджер журнала документов";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .common_module(
            "ПродажиОтчетМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Продажи" Тогда
                    ВызватьИсключение "неверный менеджер отчета";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .common_module(
            "ИмпортОбработкаМенеджер",
            r#"
            Перем Вызовов;
            Вызовов = 0;
            Функция Проверить(Добавка) Экспорт
                Если ЭтотОбъект.Имя <> "Импорт" Тогда
                    ВызватьИсключение "неверный менеджер обработки";
                КонецЕсли;
                Вызовов = Вызовов + 1;
                Возврат Добавка + Вызовов;
            КонецФункции
            "#,
        )
        .build()
        .unwrap()
}

fn state(engine: &open_bsl::Engine) -> open_bsl::State {
    state_with_data_access(engine, None)
}

#[test]
fn common_template_resolves_but_fails_without_template_resources() {
    let engine = engine();
    assert!(engine.compile("ПолучитьОбщийМакет();").is_err());
    engine
        .compile("Возврат GetCommonTemplate(\"Макет\");")
        .unwrap();
    let result = state(&engine)
        .exec(
            r#"
Попытка
    Макет = ПолучитьОбщийМакет("Макет");
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
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
fn user_full_name_reads_the_frozen_principal_field() {
    let engine = engine();
    let principal = Principal::new("user", "Краткое", Vec::new()).with_full_name("Полное имя");
    let mut named = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(catalog(), principal)))
        .build();
    assert_eq!(
        named.exec("Возврат ПолноеИмяПользователя();").unwrap(),
        Value::Str("Полное имя".into())
    );
    assert_eq!(
        named.exec("Возврат UserFullName();").unwrap(),
        Value::Str("Полное имя".into())
    );
    assert_eq!(
        named.exec("Возврат ИмяПользователя();").unwrap(),
        Value::Str("Краткое".into())
    );

    let mut empty = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog(),
            Principal::new("", "", Vec::new()),
        )))
        .build();
    assert_eq!(
        empty.exec("Возврат ПолноеИмяПользователя();").unwrap(),
        Value::Str("".into())
    );
}

#[test]
fn standard_odata_content_name_resolves_without_fabricating_a_list() {
    let engine = engine();
    let mut state = state(&engine);
    for name in [
        "ПолучитьСоставСтандартногоИнтерфейсаOData",
        "GetStandardODataInterfaceContent",
    ] {
        let source = format!("Возврат {name}();");
        assert!(engine.compile(&source).is_ok());
        assert!(
            state
                .exec(&source)
                .unwrap_err()
                .to_string()
                .contains("OData")
        );
    }
}

#[test]
fn choice_data_name_and_arity_resolve_without_fabricating_choices() {
    let engine = engine();
    let mut state = state(&engine);
    for name in ["ПолучитьДанныеВыбора", "GetChoiceData"] {
        let source = format!("Возврат {name}(Тип(\"Строка\"), Новый Структура);");
        assert!(engine.compile(&source).is_ok());
        assert!(
            state
                .exec(&source)
                .unwrap_err()
                .to_string()
                .contains("ПолучитьДанныеВыбора")
        );
        assert!(
            engine
                .compile(&format!("Возврат {name}(Тип(\"Строка\"));"))
                .is_err()
        );
        assert!(
            engine
                .compile(&format!(
                    "Возврат {name}(Тип(\"Строка\"), Новый Структура, 1);"
                ))
                .is_err()
        );
    }
}

#[test]
fn standard_odata_content_setter_does_not_report_an_unapplied_write() {
    let engine = engine();
    let mut state = state(&engine);
    for name in [
        "УстановитьСоставСтандартногоИнтерфейсаOData",
        "SetStandardODataInterfaceContent",
    ] {
        let source = format!("{name}(Новый Массив);");
        assert!(engine.compile(&source).is_ok());
        assert!(
            state
                .exec(&source)
                .unwrap_err()
                .to_string()
                .contains("OData")
        );
    }
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
fn user_password_policy_lookup_uses_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(PasswordPolicyAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                "Возврат ПолитикиПаролейПользователей.НайтиПоИмени(\"Строгая\").МинимальнаяДлинаПаролей;",
            )
            .unwrap(),
        Value::number_from_i64(12)
    );
    assert_eq!(
        state
            .exec("Возврат UserPasswordPolicies.FindByName(\"missing\");")
            .unwrap(),
        Value::Undefined
    );
    assert_eq!(
        state
            .exec("Возврат ПолучитьМинимальнуюДлинуПаролейПользователей();")
            .unwrap(),
        Value::number_from_i64(9)
    );
    assert_eq!(
        state.exec("Возврат GetUserPasswordMinLength();").unwrap(),
        Value::number_from_i64(9)
    );
    assert_eq!(
        state
            .exec("Возврат ПолитикиПаролейПользователей.НайтиПоИмени();")
            .unwrap(),
        Value::Undefined
    );
    assert_eq!(
        &*access.calls.borrow(),
        &[
            (Some("Строгая".into()), "user".into()),
            (Some("missing".into()), "user".into()),
            (Some("<минимум>".into()), "user".into()),
            (Some("<минимум>".into()), "user".into()),
            (None, "user".into()),
        ]
    );

    let mutation = state
        .exec("ПолитикиПаролейПользователей.НайтиПоИмени(\"Строгая\").МинимальнаяДлинаПаролей = 8;")
        .unwrap_err();
    assert!(mutation.to_string().contains("МинимальнаяДлинаПаролей"));

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    let error = unsupported
        .exec("Возврат ПолитикиПаролейПользователей.НайтиПоИмени(\"Строгая\");")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("политики паролей пользователей не поддерживаются")
    );
    let minimum_error = unsupported
        .exec("Возврат ПолучитьМинимальнуюДлинуПаролейПользователей();")
        .unwrap_err();
    assert!(
        minimum_error
            .to_string()
            .contains("минимальная длина паролей пользователей не поддерживается")
    );
}

#[test]
fn authentication_lock_settings_use_complete_session_bound_snapshots() {
    let engine = engine();
    let first_access = Rc::new(AuthenticationLockAccessStub::new(60, 5, "a;b"));
    let second_access = Rc::new(AuthenticationLockAccessStub::new(120, 9, "c"));
    let mut first = state_with_data_access(&engine, Some(first_access.clone()));
    let mut second = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog(), Principal::new("second-user", "Второй", vec![]))
                .with_data_access(second_access.clone()),
        ))
        .build();

    assert_eq!(
        first
            .exec(
                r#"
                Настройки = БлокировкаАутентификации.ПолучитьНастройки();
                Настройки.ДлительностьБлокировки = "90";
                Настройки.МаксимальноеКоличествоНеуспешныхПопыток = 7;
                БлокировкаАутентификации.УстановитьНастройки(Настройки);
                Возврат Настройки.КодыДополненияИмениПользователя;
                "#,
            )
            .unwrap(),
        Value::Str("a;b".into())
    );
    assert_eq!(first_access.reads.borrow().as_slice(), ["user"]);
    let writes = first_access.writes.borrow();
    assert_eq!(writes.len(), 1);
    let (written, principal) = &writes[0];
    assert_eq!(written.lock_duration(), &BslNumber::from_i64(90));
    assert_eq!(
        written.max_unsuccessful_attempts_count(),
        &BslNumber::from_i64(7)
    );
    assert_eq!(written.user_name_addition_codes(), "a;b");
    assert_eq!(principal, "user");
    drop(writes);

    assert_eq!(
        second
            .exec(
                r#"
                Settings = AuthenticationLock.GetSettings();
                Settings.UserNameAdditionCodes = 1;
                Return Settings.LockDuration + Settings.MaxUnsuccessfulAttemptsCount;
                "#,
            )
            .unwrap(),
        Value::number_from_i64(129)
    );
    assert_eq!(second_access.reads.borrow().as_slice(), ["second-user"]);
    assert!(second_access.writes.borrow().is_empty());

    for source in [
        "БлокировкаАутентификации.УстановитьНастройки();",
        "БлокировкаАутентификации.УстановитьНастройки(1);",
    ] {
        assert!(
            first
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("УстановитьНастройки")
        );
    }

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("Возврат БлокировкаАутентификации.ПолучитьНастройки();")
            .unwrap_err()
            .to_string()
            .contains("настройки блокировки аутентификации не поддерживаются")
    );
}

#[test]
fn password_strength_check_uses_the_current_session_adapter() {
    let engine = engine();
    let enabled_access = Rc::new(PasswordStrengthAccessStub::new(true));
    let disabled_access = Rc::new(PasswordStrengthAccessStub::new(false));
    let mut enabled = state_with_data_access(&engine, Some(enabled_access.clone()));
    let mut disabled = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog(), Principal::new("disabled-user", "Второй", vec![]))
                .with_data_access(disabled_access.clone()),
        ))
        .build();

    assert_eq!(
        enabled
            .exec("Возврат ПолучитьПроверкуСложностиПаролейПользователей();")
            .unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(
        disabled
            .exec("Return GetUserPasswordStrengthCheck();")
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(enabled_access.reads.borrow().as_slice(), ["user"]);
    assert_eq!(disabled_access.reads.borrow().as_slice(), ["disabled-user"]);

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("Возврат ПолучитьПроверкуСложностиПаролейПользователей();")
            .unwrap_err()
            .to_string()
            .contains("проверка сложности паролей пользователей не поддерживается")
    );
}

#[test]
fn numeric_password_settings_are_isolated_between_session_principals() {
    let engine = engine();
    let first_access = Rc::new(PasswordNumericAccessStub::new([1, 2, 3, 4]));
    let second_access = Rc::new(PasswordNumericAccessStub::new([11, 12, 13, 14]));
    let mut first = state_with_data_access(&engine, Some(first_access.clone()));
    let mut second = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog(), Principal::new("second", "Второй", vec![]))
                .with_data_access(second_access.clone()),
        ))
        .build();
    for (expression, expected_first, expected_second) in [
        ("GetUserPasswordMaxEffectivePeriod()", 1, 11),
        ("GetUserPasswordMinEffectivePeriod()", 2, 12),
        ("GetUserPasswordReuseLimit()", 3, 13),
        ("GetUserPasswordExpirationNotificationPeriod()", 4, 14),
    ] {
        let source = format!("Возврат {expression};");
        assert_eq!(
            first.exec(&source).unwrap(),
            Value::number_from_i64(expected_first)
        );
        assert_eq!(
            second.exec(&source).unwrap(),
            Value::number_from_i64(expected_second)
        );
    }
    assert_eq!(first_access.reads.borrow().len(), 4);
    assert_eq!(second_access.reads.borrow().len(), 4);
    assert!(
        first_access
            .reads
            .borrow()
            .iter()
            .all(|(_, id)| id == "user")
    );
    assert!(
        second_access
            .reads
            .borrow()
            .iter()
            .all(|(_, id)| id == "second")
    );
}

#[test]
fn numeric_password_settings_use_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(PasswordNumericAccessStub::new([10, 20, 30, 40]));
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    assert_eq!(
        state
            .exec(
                r#"
                Возврат ПолучитьМаксимальныйСрокДействияПаролейПользователей()
                    + GetUserPasswordMinEffectivePeriod()
                    + ПолучитьОграничениеПовторенияПаролейПользователейСредиПоследних()
                    + GetUserPasswordExpirationNotificationPeriod();
                "#,
            )
            .unwrap(),
        Value::number_from_i64(100)
    );
    assert_eq!(
        access.reads.borrow().as_slice(),
        [
            (
                UserPasswordNumericSetting::MaxEffectivePeriod,
                "user".into()
            ),
            (
                UserPasswordNumericSetting::MinEffectivePeriod,
                "user".into()
            ),
            (UserPasswordNumericSetting::ReuseLimit, "user".into()),
            (
                UserPasswordNumericSetting::ExpirationNotificationPeriod,
                "user".into(),
            ),
        ]
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("Возврат ПолучитьМаксимальныйСрокДействияПаролейПользователей();")
            .unwrap_err()
            .to_string()
            .contains("числовые настройки паролей пользователей не поддерживаются")
    );
}

#[test]
fn password_setting_writes_are_isolated_between_session_principals() {
    let engine = engine();
    let first_access = Rc::new(PasswordSettingsWriteStub::default());
    let second_access = Rc::new(PasswordSettingsWriteStub::default());
    let mut first = state_with_data_access(&engine, Some(first_access.clone()));
    let mut second = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog(), Principal::new("second", "Второй", vec![]))
                .with_data_access(second_access.clone()),
        ))
        .build();
    for name in [
        "SetUserPasswordStrengthCheck",
        "SetUserPasswordMinLength",
        "SetUserPasswordMaxEffectivePeriod",
        "SetUserPasswordMinEffectivePeriod",
        "SetUserPasswordReuseLimit",
        "SetUserPasswordExpirationNotificationPeriod",
    ] {
        first.exec(&format!("{name}(\"first\");")).unwrap();
        second.exec(&format!("{name}(\"second\");")).unwrap();
    }
    let first_writes = first_access.writes.borrow();
    let second_writes = second_access.writes.borrow();
    assert_eq!(first_writes.len(), 6);
    assert_eq!(second_writes.len(), 6);
    for ((first_kind, first_value, first_id), (second_kind, second_value, second_id)) in
        first_writes.iter().zip(second_writes.iter())
    {
        assert_eq!(first_kind, second_kind);
        assert_eq!(first_value, &Value::Str("first".into()));
        assert_eq!(second_value, &Value::Str("second".into()));
        assert_eq!(first_id, "user");
        assert_eq!(second_id, "second");
    }
}

#[test]
fn password_setting_writes_pass_values_to_the_current_session_adapter() {
    let engine = engine();
    let access = Rc::new(PasswordSettingsWriteStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));

    state
        .exec(
            r#"
            УстановитьПроверкуСложностиПаролейПользователей(Истина);
            SetUserPasswordMinLength("12");
            УстановитьМаксимальныйСрокДействияПаролейПользователей(10);
            SetUserPasswordMinEffectivePeriod(20);
            УстановитьОграничениеПовторенияПаролейПользователейСредиПоследних(30);
            SetUserPasswordExpirationNotificationPeriod(40);
            "#,
        )
        .unwrap();

    assert_eq!(
        access.writes.borrow().as_slice(),
        [
            (
                UserPasswordSetting::StrengthCheck,
                Value::Boolean(true),
                "user".into(),
            ),
            (
                UserPasswordSetting::MinimumLength,
                Value::Str("12".into()),
                "user".into(),
            ),
            (
                UserPasswordSetting::Numeric(UserPasswordNumericSetting::MaxEffectivePeriod),
                Value::number_from_i64(10),
                "user".into(),
            ),
            (
                UserPasswordSetting::Numeric(UserPasswordNumericSetting::MinEffectivePeriod),
                Value::number_from_i64(20),
                "user".into(),
            ),
            (
                UserPasswordSetting::Numeric(UserPasswordNumericSetting::ReuseLimit),
                Value::number_from_i64(30),
                "user".into(),
            ),
            (
                UserPasswordSetting::Numeric(
                    UserPasswordNumericSetting::ExpirationNotificationPeriod,
                ),
                Value::number_from_i64(40),
                "user".into(),
            ),
        ]
    );

    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("УстановитьПроверкуСложностиПаролейПользователей(Истина);")
            .unwrap_err()
            .to_string()
            .contains("изменение настроек паролей пользователей не поддерживается")
    );
}

#[test]
fn random_password_generation_fails_without_password_grade_entropy() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат (Новый ГенераторСлучайныхПаролей).СлучайныйПароль(12);",
        "Return (New RandomPasswordGenerator).RandomPassword(12, False);",
    ] {
        let error = state.exec(source).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("криптографический источник генерации паролей")
        );
    }
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
        Value::Str("Блокировка данных:Элемент блокировки данных:Да:Да".into())
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
fn data_locks_stay_with_each_session_adapter_and_principal() {
    let engine = engine();
    let first_access = Rc::new(RecordingDataLocks::default());
    let second_access = Rc::new(RecordingDataLocks::default());
    let mut first = state_with_data_access(&engine, Some(first_access.clone()));
    let mut second = engine
        .state_builder()
        .host_data(Rc::new(
            SessionData::new(catalog(), Principal::new("second", "Второй", vec![]))
                .with_data_access(second_access.clone()),
        ))
        .build();

    first
        .exec(
            r#"
            Блокировка = Новый БлокировкаДанных;
            Блокировка.Добавить("Catalog.First");
            Блокировка.Добавить("Document.First");
            Блокировка.Заблокировать();
            "#,
        )
        .unwrap();
    assert!(second_access.requests.borrow().is_empty());
    second
        .exec(
            r#"
            Lock = New DataLock;
            Lock.Add("Catalog.Second");
            Lock.Lock();
            "#,
        )
        .unwrap();

    assert_eq!(
        *first_access.requests.borrow(),
        vec![vec![
            "Catalog.First".to_string(),
            "Document.First".to_string()
        ]]
    );
    assert_eq!(
        *second_access.requests.borrow(),
        vec![vec!["Catalog.Second".to_string()]]
    );
    assert_eq!(*first_access.principals.borrow(), vec!["user".to_string()]);
    assert_eq!(
        *second_access.principals.borrow(),
        vec!["second".to_string()]
    );
}

#[test]
fn query_schema_constructor_compiles_both_names_and_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in ["Возврат Новый СхемаЗапроса;", "Возврат New QuerySchema;"]
    {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель схемы запроса не реализована")
        );
    }
    assert!(
        state
            .exec("Возврат Новый СхемаЗапроса(1);")
            .unwrap_err()
            .to_string()
            .contains("принимает аргументов: 0")
    );
}

#[test]
fn dormant_query_schema_method_does_not_stop_unrelated_execution() {
    let engine = engine();
    let mut state = state(&engine);
    let result = state.exec(
        "Процедура НеВызывается()\nСхема = Новый СхемаЗапроса;\nСхема.УстановитьТекстЗапроса(\"ВЫБРАТЬ 1\");\nКонецПроцедуры\nВозврат 1;",
    );
    assert_eq!(result.unwrap(), Value::number_from_i64(1));
}

#[test]
fn query_schema_expression_constructor_requires_one_argument_and_fails_explicitly() {
    let engine = engine();
    engine
        .compile(
            "Процедура НеВызывается()\nВыражение = Новый ВыражениеСхемыЗапроса(\"&Параметр\");\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый ВыражениеСхемыЗапроса(\"&Параметр\");",
        "Возврат New QuerySchemaExpression(\"&Параметр\");",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель выражения схемы запроса не реализована")
        );
    }
    assert!(
        engine
            .compile("Возврат Новый ВыражениеСхемыЗапроса;")
            .is_err()
    );
    assert!(
        engine
            .compile("Возврат New QuerySchemaExpression(1, 2);")
            .is_err()
    );
}

#[test]
fn object_deletion_constructor_requires_one_argument_and_cannot_fake_deletion() {
    let engine = engine();
    engine
        .compile(
            "Процедура НеВызывается(Ссылка)\nДанные = Новый УдалениеОбъекта(Ссылка);\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый УдалениеОбъекта(Неопределено);",
        "Возврат New ObjectDeletion(Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("удаление объекта через текущий сеанс пока не поддерживается")
        );
    }
    assert!(engine.compile("Возврат Новый УдалениеОбъекта;").is_err());
    assert!(engine.compile("Возврат New ObjectDeletion(1, 2);").is_err());
}

#[test]
fn configuration_metadata_object_constructor_does_not_reuse_current_metadata() {
    let engine = engine();
    engine
        .compile(
            "Процедура НеВызывается(Данные)\nМетаданныеФайла = Новый ОбъектМетаданныхКонфигурация(Данные);\nКонецПроцедуры",
        )
        .unwrap();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый ОбъектМетаданныхКонфигурация(Неопределено);",
        "Возврат New ConfigurationMetadataObject(Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("разбор файла конфигурации из двоичных данных пока не поддерживается")
        );
    }
    assert!(
        engine
            .compile("Возврат Новый ОбъектМетаданныхКонфигурация;")
            .is_err()
    );
    assert!(
        engine
            .compile("Возврат New ConfigurationMetadataObject(1, 2);")
            .is_err()
    );
}

#[test]
fn data_composition_settings_composer_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый КомпоновщикНастроекКомпоновкиДанных;",
        "Возврат New DataCompositionSettingsComposer;",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель настроек компоновки данных не реализована")
        );
    }
    assert!(
        state
            .exec("Возврат Новый КомпоновщикНастроекКомпоновкиДанных(1);")
            .unwrap_err()
            .to_string()
            .contains("принимает аргументов: 0")
    );
}

#[test]
fn data_composition_available_settings_source_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ИсточникДоступныхНастроекКомпоновкиДанных(Неопределено);",
        "Возврат New DataCompositionAvailableSettingsSource(Undefined, Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель источника доступных настроек не реализована")
        );
    }
    for source in [
        "Возврат Новый ИсточникДоступныхНастроекКомпоновкиДанных;",
        "Возврат New DataCompositionAvailableSettingsSource(1, 2, 3);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("принимает аргументов:")
        );
    }
}

#[test]
fn data_composition_details_process_constructor_has_measured_arity() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ОбработкаРасшифровкиКомпоновкиДанных(Неопределено, Неопределено);",
        "Возврат New DataCompositionDetailsProcess(Undefined, Undefined);",
    ] {
        assert!(
            state.exec(source).unwrap_err().to_string().contains(
                "объектная модель обработки расшифровки компоновки данных не реализована"
            )
        );
    }
    for source in [
        "Возврат Новый ОбработкаРасшифровкиКомпоновкиДанных;",
        "Возврат Новый ОбработкаРасшифровкиКомпоновкиДанных(Неопределено);",
        "Возврат Новый ОбработкаРасшифровкиКомпоновкиДанных(Неопределено, Неопределено, Неопределено);",
    ] {
        assert!(engine.compile(source).is_err());
    }
}

#[test]
fn configuration_update_description_constructor_has_measured_arity() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ОписаниеОбновленияКонфигурации(Неопределено);",
        "Возврат New ConfigurationUpdateDescription(Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("разбор описания обновления конфигурации из CFU не реализован")
        );
    }
    for source in [
        "Возврат Новый ОписаниеОбновленияКонфигурации;",
        "Возврат Новый ОписаниеОбновленияКонфигурации(Неопределено, Неопределено);",
    ] {
        assert!(engine.compile(source).is_err());
    }
}

#[test]
fn standard_period_constructor_resolves_documented_arities_and_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый СтандартныйПериод;",
        "Возврат Новый СтандартныйПериод(Неопределено);",
        "Возврат New StandardPeriod(Undefined, Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель стандартного периода не реализована")
        );
    }
    assert!(
        state
            .exec("Возврат Новый СтандартныйПериод(1, 2, 3);")
            .unwrap_err()
            .to_string()
            .contains("принимает аргументов:")
    );
}

#[test]
fn data_composition_schema_and_spreadsheet_output_constructors_fail_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for (source, message) in [
        (
            "Возврат Новый СхемаКомпоновкиДанных;",
            "объектная модель схемы компоновки данных не реализована",
        ),
        (
            "Возврат New DataCompositionSchema;",
            "объектная модель схемы компоновки данных не реализована",
        ),
        (
            "Возврат Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент;",
            "вывод результата компоновки данных в табличный документ не реализован",
        ),
        (
            "Возврат New DataCompositionResultSpreadsheetDocumentOutputProcessor;",
            "вывод результата компоновки данных в табличный документ не реализован",
        ),
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    }
    for source in [
        "Возврат Новый СхемаКомпоновкиДанных(1);",
        "Возврат Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент(1);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("принимает аргументов: 0")
        );
    }
}

#[test]
fn query_boundary_constructor_resolves_documented_arity_and_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state.exec("Возврат Новый Граница(1).Значение;").unwrap(),
        Value::number_from_i64(1)
    );
    assert!(state.exec("Возврат New Boundary(1, Undefined);").is_err());
    for source in ["Возврат Новый Граница();", "Возврат New Boundary(1, 2, 3);"]
    {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("принимает аргументов:")
        );
    }
}

#[test]
fn data_composition_user_settings_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ПользовательскиеНастройкиКомпоновкиДанных;",
        "Возврат New DataCompositionUserSettings;",
    ] {
        assert!(state.exec(source).unwrap_err().to_string().contains(
            "объектная модель пользовательских настроек компоновки данных не реализована"
        ));
    }
    assert!(
        state
            .exec("Возврат Новый ПользовательскиеНастройкиКомпоновкиДанных(1);")
            .unwrap_err()
            .to_string()
            .contains("принимает аргументов: 0")
    );
}

#[test]
fn report_builder_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ПостроительОтчета;",
        "Возврат Новый ПостроительОтчета(\"ВЫБРАТЬ 1\");",
        "Возврат New ReportBuilder();",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель построителя отчёта не реализована")
        );
    }
    assert!(
        state
            .exec("Возврат Новый ПостроительОтчета(1, 2);")
            .unwrap_err()
            .to_string()
            .contains("принимает аргументов:")
    );
}

#[test]
fn query_builder_constructor_resolves_without_substituting_a_query() {
    let engine = engine();
    let mut state = state(&engine);
    for source in [
        "Возврат Новый ПостроительЗапроса;",
        "Возврат Новый ПостроительЗапроса(\"ВЫБРАТЬ 1\");",
        "Возврат New QueryBuilder();",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("построителя запроса")
        );
    }
    assert!(
        engine
            .compile("Возврат Новый ПостроительЗапроса(1, 2);")
            .is_err()
    );
}

#[test]
fn data_source_description_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ОписаниеИсточникаДанных(Неопределено);",
        "Возврат New DataSourceDescription(Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("объектная модель описания источника данных не реализована")
        );
    }
    for source in [
        "Возврат Новый ОписаниеИсточникаДанных;",
        "Возврат New DataSourceDescription(1, 2);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("принимает аргументов:")
        );
    }
}

#[test]
fn configuration_description_constructor_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "Возврат Новый ОписаниеКонфигурации(Неопределено);",
        "Возврат New ConfigurationDescription(Undefined);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("разбор описания конфигурации из файла cf/cfe не реализован")
        );
    }
    for source in [
        "Возврат Новый ОписаниеКонфигурации;",
        "Возврат New ConfigurationDescription(1, 2);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("принимает аргументов:")
        );
    }
}

#[test]
fn edit_data_lock_has_measured_arity_and_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    for source in [
        "ЗаблокироватьДанныеДляРедактирования();",
        "LockDataForEdit(1, 2, 3, 4);",
    ] {
        assert!(
            engine
                .compile(source)
                .err()
                .expect("недопустимая арность")
                .to_string()
                .contains("принимает аргументов:")
        );
    }
    for source in [
        "Попытка ЗаблокироватьДанныеДляРедактирования(Неопределено); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
        "Try LockDataForEdit(Undefined, Undefined, Undefined); Except Return True; EndTry; Return False;",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::Boolean(true));
    }
}

#[test]
fn edit_data_unlock_has_measured_arity_and_fails_explicitly() {
    let engine = engine();
    let mut state = state(&engine);

    assert!(
        engine
            .compile("UnlockDataForEdit(1, 2, 3);")
            .err()
            .expect("недопустимая арность")
            .to_string()
            .contains("принимает аргументов:")
    );
    for source in [
        "Попытка РазблокироватьДанныеДляРедактирования(); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
        "Попытка РазблокироватьДанныеДляРедактирования(Неопределено); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;",
        "Try UnlockDataForEdit(Undefined, Undefined); Except Return True; EndTry; Return False;",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::Boolean(true));
    }
}

#[test]
fn data_composition_template_composer_has_the_measured_reached_surface() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Русский = Новый КомпоновщикМакетаКомпоновкиДанных;
                English = New DataCompositionTemplateComposer();
                Возврат Строка(ТипЗнч(Русский)) + "|" + Строка(ТипЗнч(English));
                "#,
            )
            .unwrap(),
        Value::Str(
            "Компоновщик макета компоновки данных|Компоновщик макета компоновки данных".into()
        )
    );
    for source in [
        "Возврат (Новый КомпоновщикМакетаКомпоновкиДанных).Выполнить(1, 2);",
        "Возврат (New DataCompositionTemplateComposer).Execute(1, 2, 3, 4, 5, 6, 7);",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("движок компоновки макета данных не реализован")
        );
    }
    let error = state
        .exec("Возврат (Новый КомпоновщикМакетаКомпоновкиДанных).Выполнить(1);")
        .unwrap_err()
        .to_string();
    assert!(error.contains("не применим"), "{error}");
}

#[test]
fn data_composition_processor_has_the_measured_complete_surface() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Русский = Новый ПроцессорКомпоновкиДанных;
                English = New DataCompositionProcessor();
                Возврат Строка(ТипЗнч(Русский)) + "|" + Строка(ТипЗнч(English));
                "#,
            )
            .unwrap(),
        Value::Str("Процессор компоновки данных|Процессор компоновки данных".into())
    );
    for source in [
        "Процессор = Новый ПроцессорКомпоновкиДанных; Процессор.Инициализировать(1);",
        "Процессор = New DataCompositionProcessor; Процессор.Initialize(1, 2, 3, 4, 5, 6);",
        "Процессор = Новый ПроцессорКомпоновкиДанных; Процессор.Сбросить();",
        "Процессор = New DataCompositionProcessor; Возврат Процессор.Next();",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("движок компоновки данных не реализован")
        );
    }
}

#[test]
fn data_composition_value_output_processor_has_the_measured_surface() {
    let engine = engine();
    let mut state = state(&engine);

    assert_eq!(
        state
            .exec(
                r#"
                Вывод = Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений;
                Начало = Строка(ТипЗнч(Вывод)) + "|" + Строка(Вывод.ОтображатьПроцентВывода);
                Вывод.ShowOutputPercent = Истина;
                English = New DataCompositionResultValueCollectionOutputProcessor;
                Возврат Начало + "|" + Строка(Вывод.ShowOutputPercent)
                    + "|" + Строка(ТипЗнч(English));
                "#,
            )
            .unwrap(),
        Value::Str(
            "Процессор вывода результата компоновки данных в коллекцию значений|Нет|Да|Процессор вывода результата компоновки данных в коллекцию значений".into()
        )
    );
    for source in [
        "Вывод = Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений; Вывод.УстановитьОбъект();",
        "Вывод = New DataCompositionResultValueCollectionOutputProcessor; Возврат Вывод.Output(1, 2);",
        "Вывод = Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений; Вывод.ВывестиЭлемент(1);",
        "Вывод = New DataCompositionResultValueCollectionOutputProcessor; Вывод.BeginOutput();",
        "Вывод = Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений; Возврат Вывод.ЗакончитьВывод();",
    ] {
        assert!(
            state
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("движок вывода результата компоновки данных не реализован")
        );
    }
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
        Value::Str("Авто:Обновлять автоматически:Не обновлять автоматически".into())
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
        Value::Str("Картинка:Picture:Да:Да:Нет".into())
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
                Если Строка(ТипЗнч(ЗапросПробы)) <> "Запрос"
                    ИЛИ ЗапросПробы.Текст <> "" Тогда
                    Возврат Ложь;
                КонецЕсли;
                ЗапросПробы.Текст = "ВЫБРАТЬ &Значение КАК Значение";
                ЗапросПробы.УстановитьПараметр("Значение", 1);
                ЗапросПробы.УстановитьПараметр("зНаЧеНиЕ", 42);
                Результат = ЗапросПробы.Выполнить();
                Если Строка(ТипЗнч(Результат)) <> "Результат запроса"
                    ИЛИ Результат.Пустой() Тогда
                    Возврат Ложь;
                КонецЕсли;
                Выборка = Результат.Выбрать();
                Если Строка(ТипЗнч(Выборка)) <> "Выборка из результата запроса"
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
                Возврат Строка(ТипЗнч(Таблица)) = "Таблица значений"
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
            vec![("зНаЧеНиЕ".into(), Value::number_from_i64(42))],
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
fn query_without_result_returns_undefined_from_the_session_adapter() {
    let engine = engine();
    let mut state = state_with_data_access(&engine, Some(Rc::new(NoQueryResultAccessStub)));
    assert_eq!(
        state
            .exec("Возврат Новый Запрос(\"УНИЧТОЖИТЬ ВТ\").Выполнить();")
            .unwrap(),
        Value::Undefined
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

                Возврат Строка(ТипЗнч(Сжатие)) = "Сжатие данных"
                    И Строка(ТипЗнч(Хранилище)) = "Хранилище значения"
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
fn verify_access_rights_delegates_to_the_current_sessions_verifying_adapter() {
    let engine = engine();
    let access = Rc::new(RecordingRights::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));
    assert_eq!(
        state
            .exec(
                "ВыполнитьПроверкуПравДоступа(\"Чтение\", Метаданные.Справочники.Версии, \"Код\");"
            )
            .unwrap(),
        Value::Undefined
    );
    assert_eq!(
        *access.calls.borrow(),
        vec![(
            "Чтение".into(),
            "catalog-versions".into(),
            "current".into(),
            Some("Код".into()),
            "user".into()
        )]
    );

    access.deny_verification.set(true);
    let error = state
        .exec("VerifyAccessRights(\"Read\", Metadata.Catalogs.Версии);")
        .unwrap_err();
    assert!(error.to_string().contains("доступ запрещён"), "{error}");
    assert_eq!(access.denial_events.get(), 1);
}

#[test]
fn verify_access_rights_rejects_absent_or_non_verifying_adapters() {
    let engine = engine();
    let mut missing = state(&engine);
    assert!(
        missing
            .exec("ВыполнитьПроверкуПравДоступа(\"Чтение\", Метаданные.Справочники.Версии);")
            .unwrap_err()
            .to_string()
            .contains("адаптер данных текущего сеанса не подключён")
    );
    let mut unsupported = state_with_data_access(&engine, Some(Rc::new(DataAccessStub)));
    assert!(
        unsupported
            .exec("ВыполнитьПроверкуПравДоступа(\"Чтение\", Метаданные.Справочники.Версии);")
            .unwrap_err()
            .to_string()
            .contains("не поддерживается адаптером данных")
    );
}

#[test]
fn external_data_processors_global_compiles_but_requires_a_loader() {
    let engine = engine();
    let mut state = state(&engine);
    for source in [
        "Возврат ВнешниеОбработки;",
        "Возврат ExternalDataProcessors;",
    ] {
        let error = state.exec(source).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("внешние обработки не подключены"),
            "{error}"
        );
    }
    for source in ["Возврат ВнешниеОтчеты;", "Возврат ExternalReports;"]
    {
        let error = state.exec(source).unwrap_err();
        assert!(
            error.to_string().contains("внешние отчёты не подключены"),
            "{error}"
        );
    }
}

#[test]
fn data_history_global_compiles_and_requires_a_session_service() {
    let engine = engine();
    engine
        .compile(
            "Процедура ОчиститьИсторию()\n\
             ИсторияДанных.УдалитьВерсии(Неопределено);\n\
             ИсторияДанных.ПолучитьНастройки(Неопределено);\n\
             ИсторияДанных.УстановитьНастройки(Неопределено, Неопределено);\n\
             КонецПроцедуры",
        )
        .unwrap();
    let mut state = state(&engine);
    for source in ["Возврат ИсторияДанных;", "Возврат DataHistory;"] {
        let error = state.exec(source).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("история данных не подключена к текущему сеансу"),
            "{error}"
        );
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
fn language_code_preserves_acquired_strings_and_is_read_only() {
    let engine = engine();
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("language-ru", "Русский", MetadataKind::Language)
                    .with_string_property("КодЯзыка", "ru"),
                MetadataObject::new("language-empty", "Пустой", MetadataKind::Language)
                    .with_string_property("КодЯзыка", ""),
                MetadataObject::new("language-unknown", "Неизвестный", MetadataKind::Language),
                MetadataObject::new("role", "Роль", MetadataKind::Role),
            ],
        )
        .unwrap(),
    );
    let mut state = engine
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", Vec::new()),
        )))
        .build();
    assert_eq!(state.exec("Возврат Метаданные.Языки.Русский.КодЯзыка = \"ru\" И Метаданные.Языки.Русский.languagecode = \"ru\" И Метаданные.Языки.Пустой.КодЯзыка = \"\";").unwrap(), Value::Boolean(true));
    assert!(
        state
            .exec("Метаданные.Языки.Русский.КодЯзыка = \"en\";")
            .is_err()
    );
    assert!(
        state
            .exec("Возврат Метаданные.Языки.Неизвестный.КодЯзыка;")
            .is_err()
    );
    assert!(
        state
            .exec("Возврат Метаданные.Роли.Роль.КодЯзыка;")
            .is_err()
    );
}

#[test]
fn find_by_ref_delegates_arrays_and_materializes_the_result_table() {
    let engine = engine();
    let access = Rc::new(FindByRefAccessStub::default());
    let mut session_state = state_with_data_access(&engine, Some(access.clone()));
    assert_eq!(
        session_state
            .exec(
                r#"
                Ссылки = Новый Массив;
                Ссылки.Добавить(1);
                Область = Новый Массив;
                Результат = FindByRef(Ссылки, Область, , Новый Массив);
                Возврат Результат.Количество() + Результат.Колонки.Количество();
                "#,
            )
            .unwrap(),
        Value::number_from_i64(4)
    );
    assert_eq!(
        access.calls.borrow().as_slice(),
        &[(1, true, false, true, "user".into())]
    );

    let mut unsupported = state(&engine);
    assert!(
        unsupported
            .exec("Возврат НайтиПоСсылкам(Новый Массив);")
            .is_err()
    );
    assert!(
        session_state
            .exec("Возврат НайтиПоСсылкам(Неопределено);")
            .is_err()
    );
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
                Возврат Строка(ТипЗнч(Режим)) = "Режим совместимости"
                    И Строка(Режим) = "Версия8_3_24"
                    И Строка(ТипЗнч(Варианты)) = "Перечислимое свойство объектов метаданных"
                    И Строка(Варианты) = "ПеречислимоеСвойствоОбъектовМетаданных"
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
fn document_root_and_metadata_use_the_existing_manager_path() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Первый = Документы.Заказ.Проверить(20);
                Второй = Documents.зАкАз.Проверить(20);
                Если Метаданные.Документы.Заказ.Имя <> "Заказ"
                    Или Metadata.Documents.зАкАз.Name <> "Заказ" Тогда
                    ВызватьИсключение "metadata";
                КонецЕсли;
                Возврат Первый + Второй;
                "#,
            )
            .unwrap(),
        Value::number_from_i64(43)
    );
}

#[test]
fn document_journal_root_and_metadata_use_the_existing_manager_path() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Первый = ЖурналыДокументов.Заказы.Проверить(30);
                Второй = DocumentJournals.зАкАзЫ.Проверить(30);
                Если Метаданные.ЖурналыДокументов.Заказы.Имя <> "Заказы"
                    Или Metadata.DocumentJournals.зАкАзЫ.Name <> "Заказы" Тогда
                    ВызватьИсключение "metadata";
                КонецЕсли;
                Возврат Первый + Второй;
                "#,
            )
            .unwrap(),
        Value::number_from_i64(63)
    );
}

#[test]
fn report_root_and_metadata_use_the_existing_manager_path() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Первый = Отчеты.Продажи.Проверить(40);
                Второй = Reports.пРоДаЖи.Проверить(40);
                Если Метаданные.Отчеты.Продажи.Имя <> "Продажи"
                    Или Metadata.Reports.пРоДаЖи.Name <> "Продажи" Тогда
                    ВызватьИсключение "metadata";
                КонецЕсли;
                Возврат Первый + Второй;
                "#,
            )
            .unwrap(),
        Value::number_from_i64(83)
    );
}

#[test]
fn data_processor_root_and_metadata_use_the_existing_manager_path() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Первый = Обработки.Импорт.Проверить(50);
                Второй = DataProcessors.иМпОрТ.Проверить(50);
                Если Метаданные.Обработки.Импорт.Имя <> "Импорт"
                    Или Metadata.DataProcessors.иМпОрТ.Name <> "Импорт" Тогда
                    ВызватьИсключение "metadata";
                КонецЕсли;
                Возврат Первый + Второй;
                "#,
            )
            .unwrap(),
        Value::number_from_i64(103)
    );
}

#[test]
fn remaining_acquired_manager_roots_and_metadata_collections_are_projected() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Если РегистрыНакопления.Остатки.Имя <> "Остатки"
                    Или AccumulationRegisters.оСтАтКи.Name <> "Остатки"
                    Или Метаданные.РегистрыНакопления.Остатки.Имя <> "Остатки"
                    Или РегистрыБухгалтерии.Основной.Имя <> "Основной"
                    Или Metadata.AccountingRegisters.Основной.Name <> "Основной"
                    Или РегистрыРасчета.Начисления.Имя <> "Начисления"
                    Или Metadata.CalculationRegisters.Начисления.Name <> "Начисления"
                    Или ПланыСчетов.Основной.Имя <> "Основной"
                    Или Metadata.ChartsOfAccounts.Основной.Name <> "Основной"
                    Или ПланыВидовХарактеристик.Свойства.Имя <> "Свойства"
                    Или Metadata.ChartsOfCharacteristicTypes.Свойства.Name <> "Свойства"
                    Или ПланыВидовРасчета.Основной.Имя <> "Основной"
                    Или Metadata.ChartsOfCalculationTypes.Основной.Name <> "Основной"
                    Или БизнесПроцессы.Согласование.Имя <> "Согласование"
                    Или Metadata.BusinessProcesses.Согласование.Name <> "Согласование"
                    Или Задачи.Исполнитель.Имя <> "Исполнитель"
                    Или Metadata.Tasks.Исполнитель.Name <> "Исполнитель"
                    Или ХранилищаНастроек.Пользовательские.Имя <> "Пользовательские"
                    Или Metadata.SettingsStorages.Пользовательские.Name <> "Пользовательские" Тогда
                    Возврат Ложь;
                КонецЕсли;
                Возврат Истина;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn sequence_root_and_metadata_collection_are_projected() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Возврат Последовательности.Документы.Имя = "Документы"
                    И Sequences.дОкУмЕнТы.Name = "Документы"
                    И Метаданные.Последовательности.Документы.Имя = "Документы"
                    И Metadata.Sequences.дОкУмЕнТы.Name = "Документы";
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn subsystem_metadata_uses_acquired_parent_child_relationships() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                r#"
                Возврат Метаданные.Подсистемы.Продажи.Подсистемы.Цены.Имя = "Цены"
                    И Metadata.Subsystems.зАкУпКи.Subsystems.цЕнЫ.Name = "Цены"
                    И Метаданные.Подсистемы.Продажи.ВключатьВКомандныйИнтерфейс
                    И НЕ Metadata.Subsystems.Закупки.IncludeInCommandInterface
                    И Метаданные.Подсистемы.Найти("Нет") = Неопределено
                    И Метаданные.Подсистемы.Продажи.Подсистемы.Найти("Нет") = Неопределено;
                "#,
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(state.exec("Возврат Метаданные.Подсистемы.Цены;").is_err());
}

#[test]
fn metadata_collection_count_uses_its_current_subsystem_scope() {
    let engine = engine();
    let mut state = state(&engine);
    assert_eq!(
        state
            .exec(
                "Возврат Метаданные.Подсистемы.Количество() = 2 \
                 И Metadata.Subsystems.Продажи.Subsystems.Count() = 1 \
                 И Метаданные.Подсистемы.Закупки.Подсистемы.Количество() = 1;"
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        state
            .exec("Возврат Метаданные.Подсистемы.Количество(1);")
            .is_err()
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
fn erase_infobase_data_is_rejected_by_the_read_only_host() {
    let engine = engine();
    let mut state = state(&engine);
    for source in ["УдалитьДанныеИнформационнойБазы();", "EraseInfoBaseData();"]
    {
        let error = state.exec(source).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("недоступно в хосте только для чтения"),
            "{error}"
        );
    }

    let error = state.exec("EraseInfoBaseData(Неопределено);").unwrap_err();
    assert!(error.to_string().contains("аргумент"), "{error}");
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
                    + Base64Строка(Расширение.HashSum) + ":" + Строка(Расширение.SafeMode) + ":"
                    + Строка(Расширение.UsedInDistributedInfoBase) + ":"
                    + Строка(Расширение.Active) + ":" + Строка(Расширение.Scope)
                    + ":" + Строка(ПоИдентификатору.Количество());
                "#,
            )
            .unwrap(),
        Value::Str(
            "Исправление:1.2:55K0byUrioB3aHqMgHSxrIug2Ls=:Да:Нет:Да:Информационная база:1".into()
        )
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

#[test]
fn enumeration_types_and_materializers_preserve_owner_and_member_identity() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "enum-snapshot",
            vec![
                MetadataObject::new("first", "Первое", MetadataKind::Enumeration),
                MetadataObject::new("second", "Второе", MetadataKind::Enumeration),
            ],
        )
        .unwrap()
        .with_enumeration_values(vec![
            ConfigurationEnumValue::new("first", "shared-member", "Значение"),
            ConfigurationEnumValue::new("second", "shared-member", "Значение"),
        ])
        .unwrap(),
    );
    let types: Vec<_> = catalog.transferable_value_types().collect();
    assert_eq!(types.len(), 2);
    let first =
        enumeration_reference_value(Arc::clone(&catalog), "first", "shared-member").unwrap();
    let restored = types[0].materialize(b"shared-member").unwrap();
    let other = types[1].materialize(b"shared-member").unwrap();
    assert_eq!(first, restored);
    assert_ne!(first, other);
    assert_ne!(types[0].identity(), types[1].identity());
    assert_eq!(
        first.object_ref().unwrap().value_type(),
        TypeRef::Value(Arc::clone(&types[0]))
    );
    assert_eq!(
        first.object_ref().unwrap().immutable_snapshot(),
        Some(b"shared-member".as_slice())
    );
    assert!(types[0].materialize(b"missing").is_err());
    assert!(types[0].materialize(&[255]).is_err());
    let empty = types[0].materialize(b"").unwrap();
    assert!(!empty.object_ref().unwrap().is_filled().unwrap());
    assert_eq!(empty.object_ref().unwrap().display(), "");
    assert_ne!(first, empty);
}

#[test]
fn concrete_enumeration_type_and_value_storage_work_through_the_facade() {
    let catalog = catalog();
    let mut builder = open_bsl::Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let host = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(host)).build();
    assert_eq!(
        state
            .exec(
                r#"
        ЗначениеПробы = ПредопределенноеЗначение("Перечисление.Статусы.Активен");
        КопияПробы = Новый ХранилищеЗначения(ЗначениеПробы);
        Возврат ТипЗнч(ЗначениеПробы) = Тип("ПеречислениеСсылка.Статусы")
            И ТипЗнч(ЗначениеПробы) = Тип("EnumRef.Статусы")
            И КопияПробы.Получить() = ЗначениеПробы;
    "#
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn enumeration_presentation_uses_the_acquired_primary_language_synonym() {
    fn configured(language: &str) -> Arc<MetadataCatalog> {
        Arc::new(
            MetadataCatalog::new(
                "enum-language",
                vec![
                    MetadataObject::new("enum", "ЗамерПеречисления", MetadataKind::Enumeration),
                    MetadataObject::new("language", "Язык", MetadataKind::Language)
                        .with_string_property("КодЯзыка", language),
                ],
            )
            .unwrap()
            .with_configuration(
                "Замер",
                "1",
                ScriptVariant::Russian,
                CompatibilityMode::Version8_3_24,
                [],
            )
            .unwrap()
            .with_primary_language("language")
            .unwrap()
            .with_enumeration_values(vec![
                ConfigurationEnumValue::new("enum", "member", "Обычная")
                    .with_synonym("ru", "Второе значение"),
            ])
            .unwrap(),
        )
    }
    let ru = enumeration_reference_value(configured("ru"), "enum", "member").unwrap();
    let en = enumeration_reference_value(configured("en"), "enum", "member").unwrap();
    assert_eq!(ru.object_ref().unwrap().display(), "Второе значение");
    assert_eq!(en.object_ref().unwrap().display(), "Обычная");
    assert_eq!(
        ru.object_ref().unwrap().immutable_snapshot(),
        Some(b"member".as_slice())
    );
}

#[test]
fn enumeration_manager_replays_native_boundary_and_alias_probe() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "enum-manager",
            vec![MetadataObject::new(
                "enum",
                "ЗамерПеречисления",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_configuration(
            "Замер",
            "1",
            ScriptVariant::English,
            CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap()
        .with_enumeration_values(vec![
            ConfigurationEnumValue::new("enum", "low", "Низкая"),
            ConfigurationEnumValue::new("enum", "normal", "Обычная"),
            ConfigurationEnumValue::new("enum", "high", "Высокая"),
        ])
        .unwrap(),
    );
    let mut builder = open_bsl::Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let host = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(host)).build();
    let source = include_str!(
        "../../../tests/conformance/measure/1c/measure-configuration-enum-manager.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let source = format!(
        "РезультатыПробы = Новый Массив;\n{source}\nВозврат СтрСоединить(РезультатыПробы, Символы.ПС);"
    );
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-configuration-enum-manager.platform.txt"
    )
    .replace("[Yes]", "[Да]")
    .replace("[No]", "[Нет]");
    assert_eq!(
        state.exec(&source).unwrap(),
        Value::Str(expected.trim().into())
    );
    assert_eq!(state.exec(r#"Возврат ПолучитьПолноеИмяПредопределенногоЗначения(Перечисления.ЗамерПеречисления.Обычная);"#).unwrap(), Value::Str("Enum.ЗамерПеречисления.Обычная".into()));
}

#[test]
fn missing_enumeration_uuid_retains_identity_without_a_named_member() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "missing-enum",
            vec![MetadataObject::new(
                "enum",
                "ЗамерПеречисления",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap(),
    );
    let identity = "7664d5d3-bca6-4114-8409-3272f9787060";
    let reference = enumeration_reference_value(Arc::clone(&catalog), "enum", identity).unwrap();
    let uppercase =
        enumeration_reference_value(Arc::clone(&catalog), "enum", &identity.to_uppercase())
            .unwrap();
    assert_eq!(reference, uppercase);
    assert!(reference.object_ref().unwrap().is_filled().unwrap());
    assert!(reference.object_ref().unwrap().presentation().is_err());
    let ty = catalog.transferable_value_types().next().unwrap();
    assert_eq!(reference, ty.materialize(identity.as_bytes()).unwrap());
    assert_eq!(
        reference.object_ref().unwrap().immutable_snapshot(),
        Some(identity.as_bytes())
    );
    assert!(ty.materialize(b"missing").is_err());
    assert!(ty.materialize(&[255]).is_err());
    assert!(enumeration_reference_value(Arc::clone(&catalog), "unknown", identity).is_err());
    let empty = enumeration_reference_value(catalog, "enum", "").unwrap();
    assert_ne!(reference, empty);
    assert!(!empty.object_ref().unwrap().is_filled().unwrap());
}

#[test]
fn missing_enumeration_presentation_matches_native_identity_and_session_language() {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "missing-enum",
            vec![
                MetadataObject::new("enum", "ЗамерПеречисления", MetadataKind::Enumeration)
                    .with_reference_storage_number(54),
            ],
        )
        .unwrap(),
    );
    let reference = enumeration_reference_value(
        Arc::clone(&catalog),
        "enum",
        "7664d5d3-bca6-4114-8409-3272f9787060",
    )
    .unwrap();
    let mut builder = open_bsl::Engine::builder().register_library(library());
    for ty in catalog.transferable_value_types() {
        builder = builder.register_value_type(ty);
    }
    let engine = builder.build().unwrap();
    let module = engine
        .compile(
            r#"
        Функция Представить(СсылкаПробы) Экспорт
            КопияПробы = Новый ХранилищеЗначения(СсылкаПробы);
            Если КопияПробы.Получить() <> СсылкаПробы Или Не ЗначениеЗаполнено(СсылкаПробы) Тогда
                ВызватьИсключение "Потеряна идентичность";
            КонецЕсли;
            Возврат Строка(КопияПробы.Получить());
        КонецФункции
    "#,
        )
        .unwrap();
    for language in ["ru", "en", "ru"] {
        let host = SessionData::new(Arc::clone(&catalog), Principal::new("user", "Тест", vec![]))
            .with_system_language(language);
        let mut state = engine.state_builder().host_data(Rc::new(host)).build();
        let instance = state.instantiate(&module).unwrap();
        let label = if language == "ru" {
            "Объект не найден"
        } else {
            "Object not found"
        };
        assert_eq!(
            instance
                .call(&mut state, "Представить", vec![reference.clone()])
                .unwrap()
                .0,
            Value::Str(
                format!("<{label}> (54:84093272f97870604114bca67664d5d3)")
                    .as_str()
                    .into()
            )
        );
    }
    let host = SessionData::new(catalog, Principal::new("user", "Тест", vec![]));
    let mut state = engine.state_builder().host_data(Rc::new(host)).build();
    let instance = state.instantiate(&module).unwrap();
    assert!(
        instance
            .call(&mut state, "Представить", vec![reference])
            .is_err()
    );
}

#[test]
fn second_factor_template_global_compiles_both_aliases_but_requires_the_service() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    for name in [
        "ШаблоныНастроекВторогоФактораАутентификации",
        "SecondAuthenticationFactorSettingsTemplates",
        "secondauthenticationfactorsettingstemplates",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nШаблон = {name}.НайтиПоИмени(\"тест\");\nШаблон = {name}.СоздатьШаблон();\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        let error = engine
            .new_state()
            .exec(&format!("Возврат {name};"))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("не подключены к текущему сеансу")
        );
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nШаблоны = {name};\nИсключение\nВозврат Истина;\nКонецПопытки\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
}

#[test]
fn second_factor_template_global_keeps_module_and_local_variable_priority() {
    let engine = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    assert_eq!(
        engine.new_state().exec(
            "Перем ШаблоныНастроекВторогоФактораАутентификации;\nШаблоныНастроекВторогоФактораАутентификации = 42;\nВозврат ШаблоныНастроекВторогоФактораАутентификации;"
        ).unwrap(),
        Value::number_from_i64(42)
    );
    assert_eq!(
        engine.new_state().exec(
            "Функция Проба(SecondAuthenticationFactorSettingsTemplates)\nВозврат SecondAuthenticationFactorSettingsTemplates;\nКонецФункции\nВозврат Проба(23);"
        ).unwrap(),
        Value::number_from_i64(23)
    );
}

#[path = "delete_objects_tests.rs"]
mod delete_objects;

#[test]
fn functional_option_arguments_follow_the_native_acceptance_control() {
    let engine = engine();
    let access = Rc::new(FunctionalOptionAccessStub::default());
    let mut state = state_with_data_access(&engine, Some(access.clone()));
    let receipt = include_str!(
        "../../../tests/conformance/measure/1c/measure-functional-option-arguments.platform.txt"
    );
    let cases = [
        ("OMITTED", "\"ОпцияПробы\""),
        ("UNDEFINED", "\"ОпцияПробы\", Неопределено"),
        ("NUMBER", "\"ОпцияПробы\", 1"),
        ("STRING", "\"ОпцияПробы\", \"строка\""),
        ("ARRAY", "\"ОпцияПробы\", Новый Массив"),
        ("STRUCTURE", "\"ОпцияПробы\", Новый Структура"),
        (
            "FIXED_STRUCTURE",
            "\"ОпцияПробы\", Новый ФиксированнаяСтруктура(Новый Структура)",
        ),
        ("NAME_NUMBER", "1"),
        ("NAME_UNDEFINED", "Неопределено"),
    ];
    for (id, arguments) in cases {
        let native = receipt
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once('\t')?;
                (key == format!("FUNCTIONAL_OPTION.ARGUMENT_{id}")).then_some(value)
            })
            .expect("нативный контроль содержит каждый случай");
        let result = state.exec(&format!(
            "Возврат ПолучитьФункциональнуюОпцию({arguments});"
        ));
        assert_eq!(result.is_ok(), native != "ERROR", "{id}: {result:?}");
    }
    let calls = access.calls.borrow();
    assert_eq!(calls.len(), 3);
    for (name, parameters, principal) in calls.iter() {
        assert_eq!(name, "ОпцияПробы");
        assert!(parameters.is_empty());
        assert_eq!(principal, "user");
    }
}

#[test]
fn empty_enumeration_full_name_uses_the_acquired_script_variant() {
    let engine = engine();
    for (variant, expected) in [
        (ScriptVariant::Russian, "Перечисление.Статусы.ПустаяСсылка"),
        (ScriptVariant::English, "Enum.Статусы.EmptyRef"),
    ] {
        let catalog = MetadataCatalog::new(
            "configuration",
            vec![MetadataObject::new(
                "enum-statuses",
                "Статусы",
                MetadataKind::Enumeration,
            )],
        )
        .unwrap()
        .with_configuration(
            "Конфигурация",
            "1",
            variant,
            CompatibilityMode::DontUse,
            Vec::<String>::new(),
        )
        .unwrap();
        let session = SessionData::new(
            Arc::new(catalog),
            Principal::new("user", "Тест", Vec::new()),
        );
        let mut state = engine.state_builder().host_data(Rc::new(session)).build();
        for name in [
            "ПолучитьПолноеИмяПредопределенногоЗначения",
            "GetPredefinedValueFullName",
        ] {
            assert_eq!(
                state
                    .exec(&format!(
                        "Возврат {name}(Перечисления.Статусы.ПустаяСсылка());"
                    ))
                    .unwrap(),
                Value::Str(expected.into())
            );
        }
    }
}

#[test]
fn access_verification_denials_stay_with_the_invoking_sessions_adapter() {
    let engine = engine();
    let denied_access = Rc::new(RecordingRights::default());
    denied_access.deny_verification.set(true);
    let allowed_access = Rc::new(RecordingRights::default());
    let mut denied = state_with_data_access(&engine, Some(denied_access.clone()));
    let mut allowed = state_with_data_access(&engine, Some(allowed_access.clone()));
    let source = "VerifyAccessRights(\"Read\", Metadata.Catalogs.Версии);";
    for _ in 0..2 {
        assert!(
            denied
                .exec(source)
                .unwrap_err()
                .to_string()
                .contains("доступ запрещён")
        );
        assert_eq!(allowed.exec(source).unwrap(), Value::Undefined);
    }
    assert_eq!(denied_access.denial_events.get(), 2);
    assert_eq!(allowed_access.denial_events.get(), 0);
    assert_eq!(denied_access.calls.borrow().len(), 2);
    assert_eq!(allowed_access.calls.borrow().len(), 2);
    for state in [&mut denied, &mut allowed] {
        assert!(state.exec("VerifyAccessRights(\"Read\", 1);").is_err());
    }
    assert_eq!(denied_access.calls.borrow().len(), 2);
    assert_eq!(allowed_access.calls.borrow().len(), 2);
    let foreign = denied
        .exec("Возврат Метаданные.Справочники.Версии;")
        .unwrap();
    let module = engine
        .compile(
            "Процедура Проверить(Мета) Экспорт VerifyAccessRights(\"Read\", Мета); КонецПроцедуры",
        )
        .unwrap();
    let instance = allowed.instantiate(&module).unwrap();
    assert!(
        instance
            .call(&mut allowed, "Проверить", vec![foreign])
            .is_err()
    );
    assert_eq!(allowed_access.calls.borrow().len(), 2);
}
