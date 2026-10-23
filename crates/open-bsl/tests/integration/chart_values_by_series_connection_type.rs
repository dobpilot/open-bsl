use open_bsl::{Engine, Value};

#[test]
fn chart_values_by_series_connection_type_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Нет", "None", "Нет"),
        ("СоединениеКраев", "EdgesConnection", "Соединение краев"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипСоединенияЗначенийПоСериямДиаграммы.{russian};
            Возврат ЗначениеПробы = ChartValuesBySeriesConnectionType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"ТипСоединенияЗначенийПоСериямДиаграммы\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"ChartValuesBySeriesConnectionType\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипСоединенияЗначенийПоСериямДиаграммы\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(ТипСоединенияЗначенийПоСериямДиаграммы)
                    = \"ПеречислениеТипСоединенияЗначенийПоСериямДиаграммы\"
                    И Строка(ТипЗнч(ChartValuesBySeriesConnectionType))
                    = \"ПеречислениеТипСоединенияЗначенийПоСериямДиаграммы\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_chart_values_by_series_connection_type_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем ТипСоединенияЗначенийПоСериямДиаграммы;
        ТипСоединенияЗначенийПоСериямДиаграммы = 17;
        Возврат ТипСоединенияЗначенийПоСериямДиаграммы;",
        "Функция Проба()
            ChartValuesBySeriesConnectionType = 17;
            Возврат ChartValuesBySeriesConnectionType;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}

#[test]
fn primitive_ordering_of_chart_series_connections_is_catchable() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(engine.new_state().exec(
        "Попытка ЗначениеПробы = ChartValuesBySeriesConnectionType.None < ChartValuesBySeriesConnectionType.EdgesConnection; Исключение Возврат Истина; КонецПопытки Возврат Ложь;"
    ).unwrap(), Value::Boolean(true));
}
