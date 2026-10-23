use bsl_orm::{CompatibilityMode, MetadataCatalog, Principal, ScriptVariant, SessionData, library};
use open_bsl::{Engine, Value};
use std::{rc::Rc, sync::Arc};

#[test]
fn system_predefined_full_name_follows_each_sessions_acquired_script_variant() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    for (variant, expected) in [
        (ScriptVariant::Russian, "НаправлениеСортировки.Возр"),
        (ScriptVariant::English, "SortDirection.Asc"),
        (ScriptVariant::Russian, "НаправлениеСортировки.Возр"),
    ] {
        let catalog = MetadataCatalog::new("configuration", Vec::new())
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
                    .exec(&format!("Возврат {name}(SortDirection.Asc);"))
                    .unwrap(),
                Value::Str(expected.into()),
                "{variant:?}: {name}",
            );
        }
    }
}

#[test]
fn system_predefined_full_name_preserves_the_unconfigured_host_fallback() {
    let engine = Engine::builder()
        .register_library(library())
        .build()
        .unwrap();
    let mut state = engine.state_builder().build();
    assert_eq!(
        state
            .exec("Возврат GetPredefinedValueFullName(SortDirection.Asc);")
            .unwrap(),
        Value::Str("SortDirection.Asc".into()),
    );
}
