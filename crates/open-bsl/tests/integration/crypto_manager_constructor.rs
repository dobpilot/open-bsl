use open_bsl::{Engine, Value};

#[test]
fn crypto_manager_constructor_resolves_documented_arities_without_provider() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый МенеджерКриптографии",
        "Новый CryptoManager(Ложь)",
        "Новый МенеджерКриптографии(\"модуль\", \"\", 1)",
        "Новый CryptoManager(\"module\", \"\", 1, Ложь)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЗначение = {source};\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЗначение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert!(
        engine
            .compile("Значение = Новый CryptoManager(\"module\", \"path\");")
            .is_err()
    );
}
