use open_bsl::{Engine, Value};

#[test]
fn quoted_multiline_fragments_have_platform_line_breaks() {
    let result = Engine::builder()
        .build()
        .unwrap()
        .new_state()
        .exec(
            r#"
Первое = "А" "Б";
Второе = "В"
"Г";
Третье = "Д"
// строка комментария
"Е";
Возврат Первое + "|" + Второе + "|" + Третье;
"#,
        )
        .unwrap();
    let Value::Str(result) = result else {
        panic!("ожидалась строка");
    };
    assert_eq!(result.to_string(), "А\nБ|В\nГ|Д\nЕ");
}
