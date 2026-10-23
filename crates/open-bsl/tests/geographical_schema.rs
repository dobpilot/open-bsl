#![cfg(feature = "spreadsheet")]

use open_bsl::{Engine, Value};

#[test]
fn geographical_constructor_links_dormant_code_and_rejects_execution_explicitly() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ГеографическаяСхема", "GeographicalSchema"] {
        let module = engine
            .compile(&format!(
                "Если Ложь Тогда Документ = Новый {name}; КонецЕсли; Возврат 42;"
            ))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::number_from_i64(42)
        );
        let module = engine.compile(&format!("Возврат Новый {name};")).unwrap();
        let error = engine.new_state().run(&module).unwrap_err().to_string();
        assert!(
            error.contains("географической схемы пока не реализована"),
            "{error}"
        );
        assert!(
            engine
                .compile(&format!("Возврат Новый {name}(1);"))
                .is_err()
        );
    }
    let module = engine
        .compile("Возврат Тип(\"ГеографическаяСхема\") = Тип(\"GeographicalSchema\") И Тип(\"ГеографическаяСхема\") <> Тип(\"ТабличныйДокумент\");")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}
