use open_bsl::{Engine, Value};

#[test]
fn documented_addin_connection_type_members_resolve_in_dormant_code() {
    let engine = Engine::builder().build().unwrap();
    let value = engine
        .new_state()
        .exec(
            r#"
Возврат ТипПодключенияВнешнейКомпоненты.Изолированно = AddInConnectionType.Isolated
    И ТипПодключенияВнешнейКомпоненты.НеИзолированно = AddInConnectionType.NotIsolated
    И ТипПодключенияВнешнейКомпоненты.Изолированно <> ТипПодключенияВнешнейКомпоненты.НеИзолированно;
"#,
        )
        .unwrap();
    assert_eq!(value, Value::Boolean(true));
}
