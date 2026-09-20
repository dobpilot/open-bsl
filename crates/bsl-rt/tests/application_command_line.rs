use bsl_rt::ApplicationTarget;

fn split(command: &str) -> (String, Vec<String>) {
    let ApplicationTarget::Executable { program, arguments } =
        ApplicationTarget::from_unix_command_line(command).unwrap()
    else {
        panic!("разбор команды не выбирает ассоциацию документа")
    };
    (
        program,
        arguments
            .iter()
            .map(|value| value.expose().to_string())
            .collect(),
    )
}

#[test]
fn unix_quotes_preserve_empty_arguments_unicode_and_adjacent_parts() {
    assert_eq!(
        split("  '/tmp/my tool' a\" b\" '' \"\" 'текст файла' one\\ two\tlast\n"),
        (
            "/tmp/my tool".into(),
            vec!["a b", "", "", "текст файла", "one two", "last"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );
    assert_eq!(
        split("p a''b 'a'\"b\"c"),
        ("p".into(), vec!["ab".into(), "abc".into()])
    );
}

#[test]
fn escapes_follow_quote_context_and_never_expand_shell_text() {
    assert_eq!(
        split(r#"p '\q' "\q" "\$NAME" "\`x\`" "a\"b" "\\" $NAME * ';' | > # ~"#),
        (
            "p".into(),
            vec![
                r"\q", r"\q", "$NAME", "`x`", "a\"b", "\\", "$NAME", "*", ";", "|", ">", "#", "~"
            ]
            .into_iter()
            .map(String::from)
            .collect()
        )
    );
    assert_eq!(
        split("p a\\\nb \"c\\\nd\" '\n'"),
        ("p".into(), vec!["ab".into(), "cd".into(), "\n".into()])
    );
    assert_eq!(split("\\\np\\\n"), ("p".into(), vec![]));
}

#[test]
fn invalid_command_lines_do_not_leak_the_input() {
    for input in [
        "",
        " \t\n",
        "'' x",
        "\"\"",
        "p 'secret-token",
        "p \"secret-token",
        "p secret-token\\",
        "p a\0b",
    ] {
        let error = ApplicationTarget::from_unix_command_line(input).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!error.to_string().contains("secret-token"));
    }
    let target = ApplicationTarget::from_unix_command_line("p 'secret-token'").unwrap();
    assert!(!format!("{target:?}").contains("secret-token"));
}
