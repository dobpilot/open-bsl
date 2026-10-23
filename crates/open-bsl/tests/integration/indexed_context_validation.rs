use open_bsl::{Engine, Error};

#[test]
fn context_validation_preserves_unicode_and_first_error() {
    Engine::builder()
        .common_module("I", "")
        .configuration_property_names("ı", Vec::<String>::new())
        .configuration_method_context("ı", &[])
        .build()
        .unwrap();
    for methods in [false, true] {
        let builder = Engine::builder().common_module("Straße", "");
        let builder = if methods {
            builder
                .configuration_method_context("STRASSE", &[])
                .configuration_method_context("straße", &[])
                .configuration_method_context("Нет", &[])
        } else {
            builder
                .configuration_property_names("STRASSE", Vec::<String>::new())
                .configuration_property_names("straße", Vec::<String>::new())
                .configuration_property_names("Нет", Vec::<String>::new())
        };
        let Err(Error::Configuration(message)) = builder.build() else {
            panic!("ожидалась ошибка конфигурации")
        };
        assert_eq!(
            message,
            format!(
                "контекст {} модуля «straße» задан повторно",
                if methods {
                    "методов"
                } else {
                    "свойств"
                }
            )
        );
    }
}
