use open_bsl::{Engine, Value};

#[test]
fn text_placement_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Выступать", "Overflow", "Overflow"),
        ("Забивать", "Block", "Conceal"),
        ("Обрезать", "Cut", "Clip"),
        ("Переносить", "Wrap", "Wrap"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипРазмещенияТекстаКомпоновкиДанных.{russian};
            Возврат ЗначениеПробы = DataCompositionTextPlacementType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"DataCompositionTextPlacementType\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"DataCompositionTextPlacementType\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипРазмещенияТекстаКомпоновкиДанных\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(ТипРазмещенияТекстаКомпоновкиДанных)
                    = \"EnumDataCompositionTextPlacementType\"
                    И Строка(ТипЗнч(DataCompositionTextPlacementType))
                    = \"EnumDataCompositionTextPlacementType\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn text_placement_does_not_substitute_the_spreadsheet_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    assert!(
        state
            .exec("Возврат DataCompositionTextPlacementType.Auto;")
            .is_err()
    );
    assert_eq!(
        state
            .exec(
                "Возврат ТипРазмещенияТекстаКомпоновкиДанных.Переносить
                    <> ТипРазмещенияТекстаТабличногоДокумента.Переносить
                    И ТипЗнч(DataCompositionTextPlacementType.Wrap)
                    <> ТипЗнч(SpreadsheetDocumentTextPlacementType.Wrap);",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
