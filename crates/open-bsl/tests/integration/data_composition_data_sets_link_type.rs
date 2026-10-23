use open_bsl::{Engine, Value};

#[test]
fn data_composition_data_sets_link_type_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Внешняя", "Outer", "External"),
        ("Внутренняя", "Inner", "Internal"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипСвязиНаборовДанныхКомпоновкиДанных.{russian};
            Возврат ЗначениеПробы = DataCompositionDataSetsLinkType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"DataCompositionDataSetsLinkType\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"DataCompositionDataSetsLinkType\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипСвязиНаборовДанныхКомпоновкиДанных\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(ТипСвязиНаборовДанныхКомпоновкиДанных)
                    = \"EnumDataCompositionDataSetsLinkType\"
                    И Строка(ТипЗнч(DataCompositionDataSetsLinkType))
                    = \"EnumDataCompositionDataSetsLinkType\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_data_composition_data_sets_link_type_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем ТипСвязиНаборовДанныхКомпоновкиДанных;
        ТипСвязиНаборовДанныхКомпоновкиДанных = 17;
        Возврат ТипСвязиНаборовДанныхКомпоновкиДанных;",
        "Функция Проба()
            DataCompositionDataSetsLinkType = 17;
            Возврат DataCompositionDataSetsLinkType;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}

#[test]
fn primitive_ordering_of_data_composition_data_set_links_is_catchable() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(engine.new_state().exec(
        "Попытка ЗначениеПробы = DataCompositionDataSetsLinkType.Inner < DataCompositionDataSetsLinkType.Outer; Исключение Возврат Истина; КонецПопытки Возврат Ложь;"
    ).unwrap(), Value::Boolean(true));
}
