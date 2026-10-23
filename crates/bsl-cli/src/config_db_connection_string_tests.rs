use super::take;

fn configured(extra: &[&str]) -> Vec<String> {
    [
        "bsl-cli",
        "--config-db",
        "postgres",
        "--db-host",
        "sql-host",
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
fn connection_string_option_is_consumed_in_both_forms() {
    for extra in [
        vec!["--infobase-connection-string", "Srvr=cluster;Ref=demo;"],
        vec!["--infobase-connection-string=Srvr=cluster;Ref=demo;"],
    ] {
        let mut args = configured(&extra);
        let options = take(&mut args).unwrap().unwrap();
        assert_eq!(
            options.infobase_connection_string.as_deref(),
            Some("Srvr=cluster;Ref=demo;")
        );
        assert_eq!(args, ["bsl-cli", "script.bsl"]);
    }
}

#[test]
fn invalid_connection_string_options_preserve_the_invocation() {
    for extra in [
        vec!["--infobase-connection-string="],
        vec!["--infobase-connection-string", ""],
        vec![
            "--infobase-connection-string=a",
            "--infobase-connection-string=b",
        ],
    ] {
        let mut args = configured(&extra);
        let original = args.clone();
        assert!(take(&mut args).is_err());
        assert_eq!(args, original);
    }
    let mut args = vec![
        "bsl-cli".into(),
        "--infobase-connection-string=Srvr=cluster;Ref=demo;".into(),
        "script.bsl".into(),
    ];
    let original = args.clone();
    assert!(take(&mut args).is_err());
    assert_eq!(args, original);
}

#[test]
fn connection_string_option_is_documented_in_help() {
    assert!(crate::help().contains("--infobase-connection-string"));
}

#[test]
fn omitted_connection_string_is_not_inferred_from_sql_parameters() {
    let mut args = configured(&[]);
    assert!(
        take(&mut args)
            .unwrap()
            .unwrap()
            .infobase_connection_string
            .is_none()
    );
}
