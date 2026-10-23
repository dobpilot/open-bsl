use super::*;

#[derive(Debug, PartialEq, Eq)]
struct CallRecord {
    id: String,
    name: String,
    kind: MetadataKind,
    subject: &'static str,
    privileged: bool,
}

#[derive(Debug, Default)]
struct Recording {
    calls: RefCell<Vec<CallRecord>>,
}

impl DataAccess for Recording {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Err(RtError::DynamicError("неожиданное чтение константы".into()))
    }

    fn access_right(
        &self,
        request: &AccessRightRequest,
        _: &Principal,
        context: &mut CallContext<'_>,
    ) -> RtResult<bool> {
        let subject = match request.subject() {
            AccessRightSubject::CurrentPrincipal => "current",
            AccessRightSubject::Role(_) => "role",
            AccessRightSubject::Principal(_) => "user",
        };
        let privileged = context.privileged_mode()?;
        self.calls.borrow_mut().push(CallRecord {
            id: request.metadata().id().into(),
            name: request.metadata().name().into(),
            kind: request.metadata().kind(),
            subject,
            privileged,
        });
        Ok(subject == "current" && privileged)
    }
}

fn set_probe_mode(context: &mut CallContext<'_>, _: &[Value]) -> RtResult<Value> {
    context.set_privileged_mode(true)?;
    Ok(Value::Undefined)
}

static HOST: LibraryDescriptor =
    LibraryDescriptor::new("root-right-probe", "1").with_functions(&[FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["УстановитьРежимПробы"],
        arity: Arity::exact(0),
        kind: FunctionKind::Procedure,
        call: set_probe_mode,
    }]);

fn state(access: Rc<Recording>) -> open_bsl::State {
    let catalog = Arc::new(
        MetadataCatalog::new(
            "catalog-namespace",
            vec![MetadataObject::new(
                "role-deny",
                "RootDeny",
                MetadataKind::Role,
            )],
        )
        .unwrap()
        .with_configuration_object_id("26c30ece-33b5-498a-b409-a91a046ba66b")
        .unwrap()
        .with_configuration(
            "Configuration",
            "1",
            ScriptVariant::English,
            CompatibilityMode::Version8_3_24,
            [],
        )
        .unwrap(),
    );
    let data = SessionData::new(
        catalog,
        Principal::new("user", "Проба", vec!["role-deny".into()]),
    )
    .with_data_access(access);
    open_bsl::Engine::builder()
        .register_library(library())
        .register_library(HOST)
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(data))
        .build()
}

#[test]
fn configuration_root_requests_preserve_acquired_identity_and_name() {
    let access = Rc::new(Recording::default());
    assert_eq!(
        state(access.clone())
            .exec(r#"Возврат ПравоДоступа("СохранениеДанныхПользователя", Метаданные);"#,)
            .unwrap(),
        Value::Boolean(false)
    );
    assert_eq!(
        access.calls.borrow().as_slice(),
        &[CallRecord {
            id: "26c30ece-33b5-498a-b409-a91a046ba66b".into(),
            name: "Configuration".into(),
            kind: MetadataKind::Configuration,
            subject: "current",
            privileged: false,
        }]
    );
}

#[test]
fn privileged_root_checks_delegate_explicit_subjects_and_validation() {
    let access = Rc::new(Recording::default());
    let mut state = state(access.clone());
    for (expression, expected) in [
        (r#"AccessRight("SaveUserData", Metadata)"#, true),
        (
            r#"AccessRight("SaveUserData", Metadata, Metadata.Roles.RootDeny)"#,
            false,
        ),
        (
            r#"AccessRight("SaveUserData", Metadata, InfoBaseUsers.CurrentUser())"#,
            false,
        ),
    ] {
        assert_eq!(
            state
                .exec(&format!("УстановитьРежимПробы(); Возврат {expression};"))
                .unwrap(),
            Value::Boolean(expected)
        );
    }
    let calls = access.calls.borrow();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().all(|call| call.privileged));
    assert_eq!(
        calls.iter().map(|call| call.subject).collect::<Vec<_>>(),
        ["current", "role", "user"]
    );
}

#[test]
fn missing_root_object_identity_does_not_fall_back_to_the_catalog_namespace() {
    let catalog = Arc::new(
        MetadataCatalog::new("catalog-namespace", vec![])
            .unwrap()
            .with_configuration(
                "Configuration",
                "1",
                ScriptVariant::English,
                CompatibilityMode::Version8_3_24,
                [],
            )
            .unwrap(),
    );
    let access = Rc::new(Recording::default());
    let data = SessionData::new(catalog, Principal::new("user", "Проба", vec![]))
        .with_data_access(access.clone());
    let mut state = open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(data))
        .build();
    let error = state
        .exec(r#"Возврат ПравоДоступа("Администрирование", Метаданные);"#)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("идентификатор корневого объекта конфигурации не приобретён")
    );
    assert!(access.calls.borrow().is_empty());
}

#[test]
fn the_host_cannot_declare_an_empty_root_object_identity() {
    assert!(matches!(
        MetadataCatalog::new("catalog-namespace", vec![])
            .unwrap()
            .with_configuration_object_id(""),
        Err(CatalogError::EmptyConfigurationIdentity)
    ));
}
