//! Адресная подготовка фабрики без исходников и свойств недостигнутых объектов.

use std::sync::{Arc, Mutex};

use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, MetadataObjectSource, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{
    ConfigurationIndex, ConfigurationModuleDeclaration, ConfigurationSource, ModuleExecution,
    ModuleKind, Value,
};

#[derive(Debug, Default)]
struct Details(Mutex<Vec<String>>);

impl MetadataObjectSource for Details {
    fn load_object(&self, id: &str) -> Result<MetadataObject, String> {
        self.0.lock().unwrap().push(id.into());
        assert_eq!(id, "catalog");
        Ok(
            MetadataObject::new("catalog", "Цель", MetadataKind::Catalog)
                .with_manager_module("Справочник.Цель.МодульМенеджера"),
        )
    }
}

struct Source {
    index: ConfigurationIndex,
    reads: Mutex<Vec<String>>,
}

impl ConfigurationSource for Source {
    fn index(&self) -> &ConfigurationIndex {
        &self.index
    }

    fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
        self.reads.lock().unwrap().push(id.into());
        Ok(Arc::from(match id {
            "session" => "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры",
            "manager" => "Функция Проверить() Экспорт Возврат 17; КонецФункции",
            _ => panic!("недостигнутый модуль {id}"),
        }))
    }
}

#[test]
fn sessions_share_reached_code_and_metadata_without_loading_unused_objects() {
    let server = ModuleExecution::new().with_server(true);
    let source = Arc::new(Source {
        index: ConfigurationIndex::new(
            "configuration",
            vec![
                ConfigurationModuleDeclaration::new(
                    "session",
                    "МодульСеанса",
                    ModuleKind::Session,
                    server,
                ),
                ConfigurationModuleDeclaration::new(
                    "manager",
                    "Справочник.Цель.МодульМенеджера",
                    ModuleKind::Manager,
                    server,
                )
                .with_owner("catalog"),
                ConfigurationModuleDeclaration::new("unused", "Лишний", ModuleKind::Common, server),
            ],
        )
        .unwrap(),
        reads: Mutex::default(),
    });
    let details = Arc::new(Details::default());
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("catalog", "Цель", MetadataKind::Catalog),
                MetadataObject::new("unused", "Лишний", MetadataKind::Catalog),
            ],
        )
        .unwrap()
        .with_object_source(details.clone()),
    );
    let factory = SessionFactory::new_with_source(
        source.clone(),
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
        std::iter::empty::<String>(),
    )
    .unwrap();
    assert_eq!(*source.reads.lock().unwrap(), ["session"]);
    assert!(details.0.lock().unwrap().is_empty());
    for connection in [1, 2] {
        let mut session = factory
            .open(
                Principal::new("user", "Тест", vec![]),
                SessionResources::new(Box::new(()), connection),
            )
            .unwrap();
        let lookup = session.instantiate(
            r#"Функция Проверить() Экспорт Имя = "Це" + "ль"; Возврат Метаданные.Справочники.Найти(Имя).Имя; КонецФункции"#
        ).unwrap();
        assert_eq!(
            session.call(&lookup, "Проверить", vec![]).unwrap().0,
            Value::Str("Цель".into())
        );
        if connection == 1 {
            assert_eq!(*source.reads.lock().unwrap(), ["session"]);
        }
        let entry = session
            .instantiate(
                "Функция Проверить() Экспорт Возврат Справочники.Цель.Проверить(); КонецФункции",
            )
            .unwrap();
        assert_eq!(
            session.call(&entry, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(17)
        );
    }
    assert_eq!(*source.reads.lock().unwrap(), ["session", "manager"]);
    assert_eq!(*details.0.lock().unwrap(), ["catalog"]);
}

#[test]
fn a_factory_shares_code_while_sessions_keep_parameters_principals_and_connections_separate() {
    struct IsolatedSource {
        index: ConfigurationIndex,
        reads: Mutex<Vec<String>>,
    }
    impl ConfigurationSource for IsolatedSource {
        fn index(&self) -> &ConfigurationIndex {
            &self.index
        }
        fn module_source(&self, id: &str) -> Result<Arc<str>, String> {
            self.reads.lock().unwrap().push(id.into());
            Ok(Arc::from(match id {
                "session" => {
                    "Процедура УстановкаПараметровСеанса(Имена) ПараметрыСеанса.Старт = Справочники.Цель.Следующий(); КонецПроцедуры"
                }
                "manager" => {
                    "Перем Счетчик; Функция Следующий() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции Счетчик = 0;"
                }
                _ => panic!("недостигнутый модуль {id}"),
            }))
        }
    }
    let server = ModuleExecution::new().with_server(true);
    let source = Arc::new(IsolatedSource {
        index: ConfigurationIndex::new(
            "configuration",
            vec![
                ConfigurationModuleDeclaration::new(
                    "session",
                    "МодульСеанса",
                    ModuleKind::Session,
                    server,
                ),
                ConfigurationModuleDeclaration::new(
                    "manager",
                    "Справочник.Цель.МодульМенеджера",
                    ModuleKind::Manager,
                    server,
                )
                .with_owner("catalog"),
                ConfigurationModuleDeclaration::new("unused", "Лишний", ModuleKind::Common, server),
            ],
        )
        .unwrap(),
        reads: Mutex::default(),
    });
    let catalog = Arc::new(
        MetadataCatalog::new(
            "configuration",
            vec![
                MetadataObject::new("catalog", "Цель", MetadataKind::Catalog)
                    .with_manager_module("Справочник.Цель.МодульМенеджера"),
                MetadataObject::new("started", "Старт", MetadataKind::SessionParameter),
            ],
        )
        .unwrap(),
    );
    let factory = SessionFactory::new_with_source(
        source.clone(),
        catalog,
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
        std::iter::empty::<String>(),
    )
    .unwrap();
    let mut first = factory
        .open(
            Principal::new("first", "Первый", vec![]),
            SessionResources::new(Box::new(String::from("first connection")), 1),
        )
        .unwrap();
    let mut second = factory
        .open(
            Principal::new("second", "Второй", vec![]),
            SessionResources::new(Box::new(String::from("second connection")), 2),
        )
        .unwrap();
    assert_eq!(first.principal().id(), "first");
    assert_eq!(second.principal().id(), "second");
    assert_eq!(first.connection::<String>().unwrap(), "first connection");
    assert_eq!(second.connection::<String>().unwrap(), "second connection");
    assert_eq!(
        first.parameters().peek("Старт").unwrap(),
        Some(Value::number_from_i64(1))
    );
    assert_eq!(
        second.parameters().peek("Старт").unwrap(),
        Some(Value::number_from_i64(1))
    );
    first
        .parameters()
        .set("Старт", Value::number_from_i64(99))
        .unwrap();
    assert_eq!(
        second.parameters().peek("Старт").unwrap(),
        Some(Value::number_from_i64(1))
    );
    let driver = "Функция Следующий() Экспорт Возврат Справочники.Цель.Следующий(); КонецФункции";
    let first_module = first.instantiate(driver).unwrap();
    let second_module = second.instantiate(driver).unwrap();
    for expected in [2, 3] {
        assert_eq!(
            first.call(&first_module, "Следующий", vec![]).unwrap().0,
            Value::number_from_i64(expected)
        );
    }
    assert_eq!(
        second.call(&second_module, "Следующий", vec![]).unwrap().0,
        Value::number_from_i64(2)
    );
    assert_eq!(*source.reads.lock().unwrap(), ["session", "manager"]);
}
