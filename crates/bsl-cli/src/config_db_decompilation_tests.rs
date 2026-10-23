use super::take;

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
fn protected_bodies_require_an_explicit_flag() {
    for (extra, expected) in [
        (vec![], false),
        (vec!["--decompile-protected-modules"], true),
    ] {
        let mut args = configured(&extra);
        assert_eq!(
            take(&mut args)
                .unwrap()
                .unwrap()
                .decompile_protected_modules,
            expected
        );
        assert_eq!(args, ["bsl-cli", "script.bsl"]);
    }
}

#[test]
fn invalid_decompilation_flags_fail_without_consuming_arguments() {
    for extra in [
        vec![
            "--decompile-protected-modules",
            "--decompile-protected-modules",
        ],
        vec!["--decompile-protected-modules=true"],
        vec!["--decompile-protected-modules=false"],
    ] {
        let mut args = configured(&extra);
        let original = args.clone();
        assert!(
            take(&mut args)
                .unwrap_err()
                .contains("--decompile-protected-modules")
        );
        assert_eq!(args, original);
    }
    let mut args = vec![
        "bsl-cli".into(),
        "--decompile-protected-modules".into(),
        "script.bsl".into(),
    ];
    assert!(take(&mut args).unwrap_err().contains("--config-db"));
}

#[test]
fn the_flag_after_the_path_remains_a_script_argument() {
    let mut args = configured(&[]);
    args.push("--decompile-protected-modules".into());
    assert!(
        !take(&mut args)
            .unwrap()
            .unwrap()
            .decompile_protected_modules
    );
    assert_eq!(
        args,
        ["bsl-cli", "script.bsl", "--decompile-protected-modules"]
    );
}
