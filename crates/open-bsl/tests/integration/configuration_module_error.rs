use std::error::Error as _;

use open_bsl::{Engine, Error};

#[test]
fn configuration_error_names_the_module_and_preserves_the_phase() {
    let Err(error) = Engine::builder()
        .common_module("Служебный", "Возврат НетТакойПеременной;")
        .build()
    else {
        panic!("неизвестная переменная должна быть отвергнута");
    };
    assert!(error.to_string().contains("модуль «Служебный»"));
    assert!(matches!(error, Error::InModule { .. }));
    assert!(matches!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<Error>()),
        Some(Error::Semantic(_))
    ));

    let Err(error) = Engine::builder()
        .common_module("Сломанный", "х = ;")
        .build()
    else {
        panic!("ошибка синтаксиса должна быть отвергнута");
    };
    assert!(error.to_string().contains("модуль «Сломанный»"));
    assert!(matches!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<Error>()),
        Some(Error::Parse(_))
    ));

    let Err(error) = Engine::builder()
        .common_module("Корневой", "Возврат НетТакойПеременной;")
        .configuration_roots(["Корневой"])
        .build()
    else {
        panic!("корневой модуль должен быть отвергнут");
    };
    assert!(error.to_string().contains("модуль «Корневой»"));
    assert!(matches!(
        error
            .source()
            .and_then(|source| source.downcast_ref::<Error>()),
        Some(Error::Semantic(_))
    ));
}

#[test]
fn configuration_method_error_keeps_its_phase_and_standalone_diagnostic() {
    let source = "Функция Проверить() Экспорт Возврат НетТакойПеременной; КонецФункции";
    let engine = Engine::builder().build().unwrap();
    let standalone = engine.compile(source).err().expect("ошибка семантики");
    assert!(matches!(&standalone, Error::Semantic(_)));
    let configured = Engine::builder()
        .common_module("Служебный", source)
        .configuration_roots(["Служебный"])
        .build()
        .err()
        .expect("неизвестное имя метода должно быть отвергнуто");
    assert!(configured.to_string().contains("модуль «Служебный»"));
    let phase = configured
        .source()
        .and_then(|source| source.downcast_ref::<Error>())
        .expect("исходная ошибка фазы");
    assert!(matches!(phase, Error::Semantic(_)));
    assert_eq!(phase.to_string(), standalone.to_string());
    assert!(phase.source().is_some());
}
