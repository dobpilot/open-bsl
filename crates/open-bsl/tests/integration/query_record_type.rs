use open_bsl::{Engine, Value};

#[test]
fn query_record_type_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("ДетальнаяЗапись", "DetailRecord", "Детальная запись"),
        ("ИтогПоГруппировке", "GroupTotal", "Итог по группировке"),
        ("ИтогПоИерархии", "TotalByHierarchy", "Итог по иерархии"),
        ("ОбщийИтог", "Overall", "Общий итог"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипЗаписиЗапроса.{russian};
            Возврат ЗначениеПробы = QueryRecordType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"ТипЗаписиЗапроса\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"QueryRecordType\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипЗаписиЗапроса\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(ТипЗаписиЗапроса)
                    = \"ПеречислениеТипЗаписиЗапроса\"
                    И Строка(ТипЗнч(QueryRecordType))
                    = \"ПеречислениеТипЗаписиЗапроса\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_query_record_type_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем ТипЗаписиЗапроса;
        ТипЗаписиЗапроса = 17;
        Возврат ТипЗаписиЗапроса;",
        "Функция Проба()
            QueryRecordType = 17;
            Возврат QueryRecordType;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}
