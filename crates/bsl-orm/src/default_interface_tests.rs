use super::*;

fn state(catalog: MetadataCatalog) -> open_bsl::State {
    open_bsl::Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            Arc::new(catalog),
            Principal::new("user", "Проба", vec![]),
        )))
        .build()
}

#[test]
fn unacquired_default_interface_is_a_declared_property_with_explicit_error() {
    let mut state = state(MetadataCatalog::new("unknown-interface", vec![]).unwrap());
    for name in ["ОсновнойИнтерфейс", "DefaultInterface", "dEfAuLtInTeRfAcE"] {
        let error = state
            .exec(&format!("Возврат Метаданные.{name};"))
            .unwrap_err();
        assert!(error.to_string().contains("не приобретён"), "{error}");
    }
}

#[test]
fn acquired_empty_default_interface_replays_native_reads_and_original_expression() {
    let catalog = MetadataCatalog::new("empty-interface", vec![])
        .unwrap()
        .with_default_interface_id([0; 16]);
    let mut state = state(catalog);
    for name in ["ОсновнойИнтерфейс", "DefaultInterface", "оСнОвНоЙиНтЕрФеЙс"]
    {
        assert_eq!(
            state.exec(&format!("Возврат Метаданные.{name};")).unwrap(),
            Value::Undefined
        );
    }
    assert_eq!(state.exec(r#"СвойстваПробы = Новый Структура;
СвойстваПробы.Вставить("ОсновнойИнтерфейс", ?(Метаданные.ОсновнойИнтерфейс = Неопределено, "", Метаданные.ОсновнойИнтерфейс.Имя));
Возврат СвойстваПробы.ОсновнойИнтерфейс;"#).unwrap(), Value::Str("".into()));
    for name in ["ОсновнойИнтерфейс", "DefaultInterface"] {
        let error = state
            .exec(&format!("Метаданные.{name} = Неопределено;"))
            .unwrap_err();
        assert!(error.to_string().contains("только для чтения"), "{error}");
    }
}

#[test]
fn configured_interface_is_not_fabricated_and_does_not_change_another_catalog() {
    let mut present = state(
        MetadataCatalog::new("present-interface", vec![])
            .unwrap()
            .with_default_interface_id(
                open_bsl::uuid::parse("e9008419-cc3a-4a87-8b7b-50eb53b985bf").unwrap(),
            ),
    );
    let mut empty = state(
        MetadataCatalog::new("another-empty-interface", vec![])
            .unwrap()
            .with_default_interface_id([0; 16]),
    );
    for name in ["ОсновнойИнтерфейс", "DefaultInterface"] {
        let error = present
            .exec(&format!("Возврат Метаданные.{name};"))
            .unwrap_err();
        assert!(error.to_string().contains("не поддерживаются"), "{error}");
        assert_eq!(
            empty.exec(&format!("Возврат Метаданные.{name};")).unwrap(),
            Value::Undefined
        );
    }
}
