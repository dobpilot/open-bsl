use open_bsl::{BslObject, Engine, Value, format_value};

#[test]
fn conditional_appearance_enumeration_matches_native_identity_and_presentations() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.new_state();
    // Меняется только приёмник сообщений; выражения остаются нативным контролем.
    let source = include_str!(
        "../../../../tests/conformance/measure/1c/measure-data-composition-conditional-appearance-use.bsl"
    )
    .replace("Сообщить(", "РезультатыПробы.Добавить(");
    let result = state
        .exec(&format!(
            "РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"
        ))
        .unwrap();
    let Value::Object(result) = result else {
        panic!("массив результатов");
    };
    let BslObject::Array(result) = result.as_ref() else {
        panic!("массив результатов");
    };
    let lines = result
        .borrow()
        .iter()
        .map(|value| format_value(value, None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 13);
    assert_eq!(
        lines.join("\n"),
        include_str!("../../../../tests/conformance/measure/1c/measure-data-composition-conditional-appearance-use.platform.txt")
            .trim_end()
    );
}
