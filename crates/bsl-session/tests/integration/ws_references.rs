use bsl_config::{ConfigurationCodeSnapshot, ConfigurationModule, ModuleExecution, ModuleKind};
use bsl_orm::{MetadataCatalog, Principal};
use bsl_session::{SessionFactory, SessionKind, SessionResources};
use open_bsl::{Engine, Value};
use std::sync::Arc;

/// Контекст вызова взят из типовой конфигурации и потому лежит в
/// неотслеживаемом каталоге `private/`: без него тест пропускается.
fn factory() -> Option<SessionFactory> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../private/fixtures/ws-references/unf-call-context.txt");
    let Ok(context) = std::fs::read_to_string(&path) else {
        eprintln!("пропуск: нет приватной фикстуры {}", path.display());
        return None;
    };
    let call = context
        .lines()
        .filter(|line| line.starts_with("773:") || line.starts_with("774:"))
        .map(|line| line.split_once(": ").unwrap().1)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(call.contains("WSСсылки.WSСборОтчетностиРосстата.СоздатьWSПрокси"));
    let source = format!(
        "Процедура УстановкаПараметровСеанса(Имена) КонецПроцедуры Функция Проба(URIПространстваИмен, ИмяСервиса, ИмяТочкиПодключения, ИнтернетПрокси, Таймаут, ЗащищенноеСоединение, Местоположение, ИспользоватьАутентификациюОС) Экспорт {call} Возврат Прокси; КонецФункции"
    );
    let code = ConfigurationCodeSnapshot::new(
        "ws-references",
        vec![ConfigurationModule::new(
            "session",
            "МодульСеанса",
            source,
            ModuleKind::Session,
            ModuleExecution::new().with_server(true),
        )],
    )
    .unwrap();
    SessionFactory::new(
        code,
        Arc::new(MetadataCatalog::new("ws-references", vec![]).unwrap()),
        "МодульСеанса",
        SessionKind::ThinClient,
        "8.3.27.2342",
    )
    .map(Some)
    .unwrap()
}
#[test]
fn original_dormant_ws_reference_binds_but_unknown_composition_fails_explicitly() {
    let Some(factory) = factory() else {
        return;
    };
    let mut session = factory
        .open(
            Principal::new("user", "Тест", vec![]),
            SessionResources::new(Box::new(()), 1),
        )
        .unwrap();
    for name in ["WSСсылки", "WSReferences", "wSrEfErEnCeS"] {
        let module = session
            .instantiate(&format!(
                "Функция Проверить() Экспорт Возврат {name}; КонецФункции"
            ))
            .unwrap();
        let error = session
            .call(&module, "Проверить", vec![])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("WS-ссылки конфигурации пока не поддерживаются"),
            "{error}"
        );
    }
    for name in ["WSСсылки", "WSReferences"] {
        let module = session.instantiate(&format!("Перем {name}; Функция Проверить() Экспорт {name} = 23; Возврат {name}; КонецФункции Функция Локальная() Экспорт Перем {name}; {name} = 17; Возврат {name}; КонецФункции")).unwrap();
        assert_eq!(
            session.call(&module, "Проверить", vec![]).unwrap().0,
            Value::number_from_i64(23)
        );
        assert_eq!(
            session.call(&module, "Локальная", vec![]).unwrap().0,
            Value::number_from_i64(17)
        );
    }
    assert!(
        Engine::builder()
            .build()
            .unwrap()
            .compile("Возврат WSСсылки;")
            .is_err()
    );
}
