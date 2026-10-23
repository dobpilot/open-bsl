use open_bsl::{Engine, Value};

#[test]
fn round_mode_members_match_the_platform_measurement() {
    let result = Engine::builder()
        .build()
        .unwrap()
        .new_state()
        .exec(
            r#"
Возврат Строка(ТипЗнч(РежимОкругления.Окр15как10)) + "|"
    + Строка(РежимОкругления.Окр15как10) + "|"
    + Строка(РежимОкругления.Окр15как20) + "|"
    + Строка(Окр(1.5, 0, РежимОкругления.Окр15как10)) + "|"
    + Строка(Окр(1.5, 0, RoundMode.Round15as20)) + "|"
    + ?(RoundMode.Round15as10 = РежимОкругления.Окр15как10, "true", "false");
"#,
        )
        .unwrap();
    let Value::Str(value) = result else {
        panic!("ожидалась строка");
    };
    assert_eq!(
        value.to_string(),
        "РежимОкругления|Округлять 15 как 10|Округлять 15 как 20|1|2|true"
    );
}
