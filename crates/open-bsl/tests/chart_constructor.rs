#![cfg(feature = "spreadsheet")]

use open_bsl::{Engine, Error, SemaError, Value};

#[test]
fn chart_aliases_link_dormant_original_code() {
    let engine = Engine::builder().build().unwrap();
    for name in ["Диаграмма", "Chart", "дИаГрАмМа", "cHaRt"] {
        for suffix in ["", "()"] {
            let module = engine
                .compile(&format!(
                    "Если Ложь Тогда Результат = Новый {name}{suffix}; КонецЕсли; Возврат 42;"
                ))
                .unwrap();
            assert_eq!(
                engine.new_state().run(&module).unwrap(),
                Value::number_from_i64(42)
            );
        }
    }
}

#[test]
fn chart_type_aliases_preserve_the_native_presentation() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Возврат Тип(\"Диаграмма\") = Type(\"Chart\");")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
    for name in ["Диаграмма", "Chart"] {
        let module = engine
            .compile(&format!("Возврат Строка(Тип(\"{name}\"));"))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Str("Chart".into())
        );
    }
}

#[test]
fn actual_chart_construction_reports_the_missing_object_model() {
    let engine = Engine::builder().build().unwrap();
    for name in ["Диаграмма", "Chart"] {
        let module = engine.compile(&format!("Возврат Новый {name}();")).unwrap();
        let error = engine.new_state().run(&module).unwrap_err().to_string();
        assert!(
            error.contains("объектная модель диаграммы пока не реализована"),
            "{error}"
        );
    }
}

#[test]
fn chart_arguments_use_the_ordinary_component_arity_diagnostic() {
    let engine = Engine::builder().build().unwrap();
    for name in ["Диаграмма", "Chart"] {
        for (arguments, count) in [("0", 1), ("Неопределено, Неопределено", 2)]
        {
            let error = engine
                .compile(&format!("Возврат Новый {name}({arguments});"))
                .err()
                .expect("конструктор с аргументами должен быть отвергнут");
            assert!(
                matches!(
                    error,
                    Error::Semantic(SemaError::ArgumentCountMismatch {
                        expected: 0,
                        found,
                        ..
                    }) if found == count
                ),
                "{error:?}"
            );
        }
    }
}
