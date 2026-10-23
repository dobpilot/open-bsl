#[cfg(feature = "crypto")]
use open_bsl::Engine;

#[cfg(feature = "crypto")]
#[test]
fn access_token_declares_only_native_constructor_arities_and_fails_explicitly() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ТокенДоступа", "aCcEsStOkEn"] {
        for args in ["", "Новый Соответствие, Новый Соответствие"]
        {
            let expression = format!("Новый {name}({args})");
            engine
                .compile(&format!(
                    "Процедура Холодная() ТокенПробы = {expression}; КонецПроцедуры"
                ))
                .unwrap();
            let error = engine
                .new_state()
                .exec(&format!("Возврат {expression};"))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("ТокенДоступа: объявлено без реализации"),
                "{error}"
            );
        }
        for args in [
            "Новый Соответствие",
            "Новый Соответствие, Новый Соответствие, Новый Соответствие",
        ] {
            assert!(
                engine
                    .compile(&format!(
                        "Процедура Холодная() ТокенПробы = Новый {name}({args}); КонецПроцедуры"
                    ))
                    .is_err()
            );
        }
    }
}

#[test]
fn access_token_hs256_matches_native_aliases_type_and_presentation() {
    let engine = open_bsl::Engine::builder().build().unwrap();
    let result = engine.new_state().exec("Возврат АлгоритмПодписиТокенаДоступа.HS256 = accesstokensignalgorithm.hs256 И ТипЗнч(AccessTokenSignAlgorithm.HS256) = Тип(\"АлгоритмПодписиТокенаДоступа\") И Тип(\"AccessTokenSignAlgorithm\") = Тип(\"АлгоритмПодписиТокенаДоступа\");").unwrap();
    assert_eq!(result, open_bsl::Value::Boolean(true));
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат Строка(ТипЗнч(AccessTokenSignAlgorithm));")
            .unwrap(),
        open_bsl::Value::Str("EnumAccessTokenSignAlgorithm".into())
    );
    assert_eq!(
        engine
            .new_state()
            .exec("Возврат Строка(AccessTokenSignAlgorithm.HS256);")
            .unwrap(),
        open_bsl::Value::Str("HS256".into())
    );
    assert_eq!(engine.new_state().exec("Попытка РезультатПробы = AccessTokenSignAlgorithm.HS256 < AccessTokenSignAlgorithm.HS256; Возврат Ложь; Исключение Возврат Истина; КонецПопытки;").unwrap(), open_bsl::Value::Boolean(true));
}

#[cfg(feature = "crypto")]
#[test]
fn original_cold_signing_shape_compiles_without_executing_jwt_operations() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"Функция Холодная()
ТокенПробы = Новый ТокенДоступа;
ТокенПробы.Подписать(АлгоритмПодписиТокенаДоступа.HS256, "unused-probe-key");
Возврат Строка(ТокенПробы);
КонецФункции
Возврат 42;"#,
        )
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        open_bsl::Value::Number(open_bsl::BslNumber::from_i64(42))
    );
}
