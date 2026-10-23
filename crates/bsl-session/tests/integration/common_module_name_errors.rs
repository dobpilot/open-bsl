use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::Value;

#[test]
fn session_starts_with_unused_faulty_child_and_fails_on_actual_access() {
    let modules = [
        (
            "МодульСеанса",
            ModuleKind::Session,
            "Процедура УстановкаПараметровСеанса(Имена)
                Ответ = Родитель.Исправный();
             КонецПроцедуры",
        ),
        (
            "Родитель",
            ModuleKind::Common,
            "Функция Исправный() Экспорт Возврат 42; КонецФункции
             Функция Дочерний() Экспорт Возврат Дочерний.Исправный(); КонецФункции",
        ),
        (
            "Дочерний",
            ModuleKind::Common,
            "Функция Исправный() Экспорт Возврат 7; КонецФункции
             Функция Ошибка() Экспорт Возврат СовсемНеизвестноеИмя; КонецФункции",
        ),
    ]
    .into_iter()
    .map(|(name, kind, source)| {
        ConfigurationModule::new(
            name,
            name,
            source,
            kind,
            ModuleExecution::new().with_server(true),
        )
    })
    .collect();
    let factory = SessionFactory::new(
        ConfigurationCodeSnapshot::new("deferred-common", modules).unwrap(),
        Arc::new(MetadataCatalog::new("deferred-common", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .unwrap();
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    let good = session
        .instantiate("Функция Проба() Экспорт Возврат Родитель.Исправный(); КонецФункции")
        .unwrap();
    assert_eq!(
        session.call(&good, "Проба", vec![]).unwrap().0,
        Value::number_from_i64(42)
    );
    let bad = session.instantiate("Функция Проба() Экспорт Попытка Возврат Родитель.Дочерний(); Исключение Возврат 99; КонецПопытки; КонецФункции").unwrap();
    let error = session.call(&bad, "Проба", vec![]).unwrap_err().to_string();
    assert!(error.contains("ошибка компиляции модуля"), "{error}");
    assert!(error.contains("СовсемНеизвестноеИмя"), "{error}");
    assert!(
        session
            .instantiate("Возврат СовсемНеизвестноеИмя;")
            .is_err()
    );
}
