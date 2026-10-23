use super::take;
use bsl_session::SessionKind;

fn configured(extra: &[&str]) -> Vec<String> {
    [
        "bsl-cli",
        "--config-db",
        "postgres",
        "--db-host",
        "localhost",
        "--db-name",
        "demo",
        "--db-user",
        "test",
        "--platform-version",
        "8.3.27.2342",
    ]
    .into_iter()
    .chain(extra.iter().copied())
    .chain(["script.bsl"])
    .map(str::to_owned)
    .collect()
}

#[test]
fn foreground_kind_defaults_and_both_explicit_forms_are_preserved() {
    for (extra, expected) in [
        (vec![], SessionKind::ThinClient),
        (
            vec!["--session-kind", "thin-client"],
            SessionKind::ThinClient,
        ),
        (vec!["--session-kind=web-client"], SessionKind::WebClient),
    ] {
        let mut args = configured(&extra);
        assert_eq!(take(&mut args).unwrap().unwrap().session_kind, expected);
        assert_eq!(args, ["bsl-cli", "script.bsl"]);
    }
}

#[test]
fn invalid_kinds_fail_before_consuming_the_invocation() {
    for extra in [
        vec!["--session-kind", "background-job"],
        vec!["--session-kind", "unknown"],
        vec!["--session-kind=web-client", "--session-kind=thin-client"],
    ] {
        let mut args = configured(&extra);
        let original = args.clone();
        assert!(take(&mut args).unwrap_err().contains("--session-kind"));
        assert_eq!(args, original);
    }
    let mut args = vec![
        "bsl-cli".into(),
        "--session-kind=web-client".into(),
        "script.bsl".into(),
    ];
    assert!(take(&mut args).unwrap_err().contains("--config-db"));
}

#[test]
fn kind_text_after_the_path_remains_a_script_argument() {
    let mut args = configured(&[]);
    args.extend(["--session-kind".into(), "web-client".into()]);
    assert_eq!(
        take(&mut args).unwrap().unwrap().session_kind,
        SessionKind::ThinClient
    );
    assert_eq!(
        args,
        ["bsl-cli", "script.bsl", "--session-kind", "web-client"]
    );
}
