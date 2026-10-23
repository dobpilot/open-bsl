use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, MetadataObject, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{CallContext, RtResult, Value};
use std::rc::Rc;

#[derive(Debug)]
struct ReadOnly;

impl DataAccess for ReadOnly {
    fn constant_value(
        &self,
        _: &MetadataObject,
        _: &Principal,
        _: &mut CallContext<'_>,
    ) -> RtResult<Value> {
        Ok(Value::number_from_i64(17))
    }
}

fn factory(source: &str) -> Result<SessionFactory, bsl_session::SessionError> {
    let module = "Константа.Проба.МодульМенеджера";
    SessionFactory::new(
        ConfigurationCodeSnapshot::new(
            "constant-set",
            vec![
                ConfigurationModule::new(
                    "session", "МодульСеанса",
                    "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры\nФункция Проба() Экспорт Возврат Константы.Проба.Проверить(); КонецФункции",
                    ModuleKind::Session, ModuleExecution::new().with_server(true),
                ),
                ConfigurationModule::new(
                    "manager", module, source, ModuleKind::Manager,
                    ModuleExecution::new().with_server(true),
                ).with_owner("register"),
            ],
        ).unwrap(),
        Arc::new(MetadataCatalog::new("constant-set", vec![
            MetadataObject::new("register", "Проба", MetadataKind::Constant)
                .with_manager_module(module),
        ]).unwrap()),
        "МодульСеанса", SessionKind::ThinClient, "8.3.27.2342",
    )
}

#[test]
fn bare_set_uses_the_manager_receiver_and_preserves_same_named_variables() {
    let factory = factory("Перем Установить; Установить = 19; Функция Проверить() Экспорт Возврат Установить; КонецФункции\nФункция Читать() Экспорт Возврат Установить(Новый Структура(\"Измерение\",7)); КонецФункции\nФункция ЧитатьАнглийский() Экспорт Возврат sEt(Новый Структура(\"Измерение\",7)); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1).with_data_access(Rc::new(ReadOnly)),
        )
        .unwrap();
    let module = session
        .instantiate(
            "Функция Проверить() Экспорт Возврат Константы.Проба.Проверить(); КонецФункции",
        )
        .unwrap();
    assert_eq!(
        session.call(&module, "Проверить", vec![]).unwrap().0,
        Value::number_from_i64(19)
    );
    for name in ["Читать", "ЧитатьАнглийский"] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат Константы.Проба.{name}(); КонецФункции"
            ))
            .unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("запись константы не поддерживается"),
            "{error}"
        );
    }
}

#[test]
fn bare_set_has_a_finite_platform_arity() {
    let factory =
        factory("Функция Проверить() Экспорт Возврат Установить(1,2,3); КонецФункции").unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    assert!(
        session
            .instantiate("Возврат Константы.Проба.Проверить();")
            .is_err()
    );
}
