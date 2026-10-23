use open_bsl::{Engine, Error, RtError, Value};

#[test]
fn lookup_aliases_and_arity_are_enforced_through_the_facade() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.state_builder().build();
    assert_eq!(
        state.exec("СписокПробы = Новый СписокЗначений; ЭлементПробы = СписокПробы.Добавить(17); Возврат СписокПробы.fInDbYvAlUe(17) = ЭлементПробы И СписокПробы.найтипозначению(17) = ЭлементПробы;").unwrap(),
        Value::Boolean(true)
    );
    for arguments in ["", "17, 18"] {
        let error = state.exec(&format!("СписокПробы = Новый СписокЗначений; Возврат СписокПробы.НайтиПоЗначению({arguments});")).unwrap_err();
        assert!(matches!(
            error,
            Error::Runtime(RtError::MethodNotApplicable {
                method: "НайтиПоЗначению",
                receiver: "СписокЗначений"
            })
        ));
    }
}
