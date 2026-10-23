use super::*;
use open_bsl::{Engine, Error};

fn engine() -> Engine {
    Engine::builder()
        .register_library(library())
        .build()
        .unwrap()
}

#[test]
fn dormant_filter_criteria_calls_compile_with_both_global_aliases() {
    let engine = engine();
    for global in ["КритерииОтбора", "FilterCriteria", "кРиТеРиИоТбОрА"]
    {
        let source = format!(
            "Функция СоставДокументов() Возврат {global}.ДокументыПоКонтрагенту.СоставДокументов(); КонецФункции Возврат 17;"
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::number_from_i64(17)
        );
    }
}

#[test]
fn reading_filter_criteria_is_explicitly_unsupported_and_catchable() {
    let engine = engine();
    for global in ["КритерииОтбора", "FilterCriteria"] {
        let error = engine
            .new_state()
            .exec(&format!("Возврат {global};"))
            .unwrap_err();
        assert!(
            matches!(error, Error::Runtime(RtError::DynamicError(ref message))
            if message == "критерии отбора конфигурации пока не поддерживаются этим хостом")
        );
        assert_eq!(
            engine.new_state().exec(&format!("Попытка ЗначениеПробы = {global}; Исключение Возврат Истина; КонецПопытки Возврат Ложь;")).unwrap(),
            Value::Boolean(true)
        );
    }
}

#[test]
fn variables_keep_priority_over_the_filter_criteria_global() {
    let engine = engine();
    for global in ["КритерииОтбора", "FilterCriteria"] {
        for source in [
            format!("Перем {global}; {global} = 17; Возврат {global};"),
            format!(
                "Функция Проба() {global} = 17; Возврат {global}; КонецФункции Возврат Проба();"
            ),
        ] {
            assert_eq!(
                engine.new_state().exec(&source).unwrap(),
                Value::number_from_i64(17)
            );
        }
    }
}

#[test]
fn filter_criteria_remains_undefined_without_the_orm_library() {
    let engine = Engine::builder().build().unwrap();
    assert!(matches!(
        engine.compile("Возврат КритерииОтбора;"),
        Err(Error::Semantic(_))
    ));
}
