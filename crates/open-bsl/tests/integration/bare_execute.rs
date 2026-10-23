use open_bsl::{Engine, Value};

#[test]
fn bare_execute_uses_the_same_caller_scope_as_parenthesized_execute() {
    let result = Engine::builder()
        .build()
        .unwrap()
        .new_state()
        .exec(
            r#"
Первое = 0;
Второе = 0;
Выполнить "Первое = " + Строка(40 + 2);
Выполнить("Второе = 24");
Возврат Строка(Первое) + "|" + Строка(Второе);
"#,
        )
        .unwrap();
    let Value::Str(result) = result else {
        panic!("ожидалась строка");
    };
    assert_eq!(result.to_string(), "42|24");
}
