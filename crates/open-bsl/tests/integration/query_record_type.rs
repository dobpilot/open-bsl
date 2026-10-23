use open_bsl::{Engine, Value};

#[test]
fn query_record_type_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("ДетальнаяЗапись", "DetailRecord", "Detailed record"),
        ("ИтогПоГруппировке", "GroupTotal", "Total by group"),
        ("ИтогПоИерархии", "TotalByHierarchy", "Total by hierarchy"),
        ("ОбщийИтог", "Overall", "Grand total"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипЗаписиЗапроса.{russian};
            Возврат ЗначениеПробы = QueryRecordType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"QueryRecordType\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"QueryRecordType\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипЗаписиЗапроса\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(ТипЗаписиЗапроса)
                    = \"EnumQueryRecordType\"
                    И Строка(ТипЗнч(QueryRecordType))
                    = \"EnumQueryRecordType\";",
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
