use open_bsl::{Engine, Value};

#[test]
fn settings_item_state_matches_native_aliases_types_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for (russian, english, presentation) in [
        ("Включен", "Enabled", "Enabled"),
        ("Отключен", "Disabled", "Disabled"),
        ("УдаленПользователем", "DeletedByUser", "Deleted by user"),
    ] {
        let source = format!(
            "ЗначениеПробы = СостояниеЭлементаНастройкиКомпоновкиДанных.{russian};
            Возврат ЗначениеПробы = DataCompositionSettingsItemState.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"DataCompositionSettingsItemState\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"DataCompositionSettingsItemState\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"СостояниеЭлементаНастройкиКомпоновкиДанных\");"
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true));
    }
    assert_eq!(
        state
            .exec(
                "Возврат Строка(СостояниеЭлементаНастройкиКомпоновкиДанных)
                    = \"EnumDataCompositionSettingsItemState\"
                    И Строка(ТипЗнч(DataCompositionSettingsItemState))
                    = \"EnumDataCompositionSettingsItemState\";",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn variables_keep_precedence_over_the_settings_item_state_enum() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    for source in [
        "Перем СостояниеЭлементаНастройкиКомпоновкиДанных;
        СостояниеЭлементаНастройкиКомпоновкиДанных = 17;
        Возврат СостояниеЭлементаНастройкиКомпоновкиДанных;",
        "Функция Проба()
            DataCompositionSettingsItemState = 17;
            Возврат DataCompositionSettingsItemState;
        КонецФункции
        Возврат Проба();",
    ] {
        assert_eq!(state.exec(source).unwrap(), Value::number_from_i64(17));
    }
}
