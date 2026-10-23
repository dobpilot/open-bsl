use open_bsl::{Engine, Value};

#[test]
fn group_placement_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Конец", "End", "End"),
        ("Начало", "Begin", "Begin"),
        ("НачалоИКонец", "BeginAndEnd", "Begin and end"),
        ("Нет", "None", "None"),
    ] {
        let source = format!(
            "ЗначениеПробы = РасположениеГруппировкиКомпоновкиДанных.{russian};
            Возврат ЗначениеПробы = DataCompositionGroupPlacement.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"DataCompositionGroupPlacement\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"DataCompositionGroupPlacement\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"РасположениеГруппировкиКомпоновкиДанных\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(РасположениеГруппировкиКомпоновкиДанных)
                    = \"EnumDataCompositionGroupPlacement\"
                    И Строка(ТипЗнч(DataCompositionGroupPlacement))
                    = \"EnumDataCompositionGroupPlacement\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_group_placement_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем РасположениеГруппировкиКомпоновкиДанных;
        РасположениеГруппировкиКомпоновкиДанных = 17;
        Возврат РасположениеГруппировкиКомпоновкиДанных;",
        "Функция Проба()
            DataCompositionGroupPlacement = 17;
            Возврат DataCompositionGroupPlacement;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}
