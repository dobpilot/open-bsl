use open_bsl::{Engine, Value};

#[test]
fn value_tree_constructor_resolves_but_does_not_fabricate_a_flat_table() {
    let engine = Engine::builder().build().unwrap();
    assert!(engine.compile("Дерево = Новый ДеревоЗначений(1);").is_err());
    engine.compile("Дерево = New ValueTree();").unwrap();

    let result = engine
        .new_state()
        .exec(
            r#"
Попытка
    Дерево = Новый ДеревоЗначений;
Исключение
    Возврат Истина;
КонецПопытки;
Возврат Ложь;
"#,
        )
        .unwrap();
    assert_eq!(result, Value::Boolean(true));
}
