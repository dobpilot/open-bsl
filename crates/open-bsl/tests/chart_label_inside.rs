use open_bsl::{Engine, Error, SemaError, Value};

#[test]
fn inside_edge_label_aliases_resolve_without_case_sensitivity() {
    let engine = Engine::builder().build().unwrap();
    for name in [
        "ПоложениеПодписейКДиаграмме.КрайВнутри",
        "ChartLabelLocation.EdgeInside",
        "ПОЛОЖЕНИЕПОДПИСЕЙКДИАГРАММЕ.КРАЙВНУТРИ",
        "chartlabellocation.edgeinside",
    ] {
        let module = engine
            .compile(&format!("Возврат {name} = ChartLabelLocation.EdgeInside;"))
            .unwrap();
        assert_eq!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        );
    }
}

#[test]
fn inside_edge_preserves_its_native_type_and_both_type_aliases() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Возврат ТипЗнч(ПоложениеПодписейКДиаграмме.КрайВнутри) = Тип(\"ПоложениеПодписейКДиаграмме\") И Тип(\"ПоложениеПодписейКДиаграмме\") = Type(\"ChartLabelLocation\");")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn inside_edge_keeps_the_existing_auto_label_distinct_and_unchanged() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Возврат Строка(ChartLabelLocation.EdgeInside) + \"|\" + Строка(ChartLabelLocation.EdgeAuto) + \"|\" + ?(ChartLabelLocation.EdgeInside <> ChartLabelLocation.EdgeAuto, \"YES\", \"NO\");")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Str("Edge inside|Edge auto|YES".into())
    );
}

#[test]
fn invalid_label_alias_and_primitive_ordering_keep_their_errors() {
    let engine = Engine::builder().build().unwrap();
    let error = engine
        .compile("Возврат ChartLabelLocation.InsideEdge;")
        .err()
        .expect("неизвестный псевдоним должен быть отвергнут");
    assert!(
        matches!(&error, Error::Semantic(SemaError::UndefinedVariable(name)) if name == "ChartLabelLocation.InsideEdge"),
        "{error:?}"
    );
    let module = engine
        .compile("Попытка Результат = ChartLabelLocation.EdgeAuto < ChartLabelLocation.EdgeInside; Возврат Ложь; Исключение Возврат Истина; КонецПопытки;")
        .unwrap();
    assert_eq!(
        engine.new_state().run(&module).unwrap(),
        Value::Boolean(true)
    );
}
