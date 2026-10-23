use super::*;
use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::MetadataCatalog;

#[derive(Debug)]
struct ContextAdapter;

impl ClientSettingsAdapter for ContextAdapter {
    fn load(&self, _: &Principal, _: &ClientSettingsLoadRequest) -> RtResult<Option<Value>> {
        panic!("должен вызываться обработчик с контекстом")
    }

    fn load_with_context(
        &self,
        principal: &Principal,
        request: &ClientSettingsLoadRequest,
        context: &mut CallContext<'_>,
    ) -> RtResult<Option<Value>> {
        let host = context.host_data()?.downcast_ref::<SessionData>().unwrap();
        assert_eq!(host.principal().name(), principal.name());
        assert_eq!(request.object_key(), "own-context");
        assert_eq!(request.storage(), ClientSettingsStorage::Common);
        Ok(Some(Value::Str(principal.name().into())))
    }

    fn home_page_forms(&self, _: &Principal) -> RtResult<HomePageFormsData> {
        unreachable!()
    }
    fn set_home_page_forms(&self, _: &Principal, _: &HomePageFormsData) -> RtResult<()> {
        unreachable!()
    }

    fn save(&self, _: &Principal, _: &ClientSettingsSaveRequest) -> RtResult<()> {
        unreachable!()
    }
    fn select(
        &self,
        _: &Principal,
        _: &ClientSettingsSelectRequest,
    ) -> RtResult<Vec<ClientSettingsEntry>> {
        unreachable!()
    }
}

#[test]
fn client_settings_context_belongs_to_each_current_session() {
    let code = ConfigurationCodeSnapshot::new(
        "settings-context",
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
        Arc::new(MetadataCatalog::new("settings-context", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    for name in ["Первый", "Второй"] {
        let mut session = factory
            .open(
                Principal::new(name, name, vec![]),
                SessionResources::new(Box::new(()), 1)
                    .with_client_settings_adapter(Rc::new(ContextAdapter)),
            )
            .unwrap();
        let module = session.instantiate("Функция Проверить() Экспорт Возврат ХранилищеОбщихНастроек.Загрузить(\"own-context\"); КонецФункции").unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::Str(name.into())
        );
    }
}
