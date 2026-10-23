use super::*;
fn options(extra: &[&str]) -> Result<Option<Options>, String> {
    let mut args = vec![
        "bsl-cli",
        "--config-db",
        "postgres",
        "--db-host",
        "localhost",
        "--db-name",
        "demo",
        "--db-user",
        "reader",
        "--platform-version",
        "8.3.27.2342",
    ];
    args.extend(extra);
    args.push("main.bsl");
    take(&mut args.into_iter().map(str::to_string).collect())
}
#[test]
fn selectors_are_exclusive_and_accept_both_argument_forms() {
    assert!(
        options(&["--config-xml=directory"])
            .unwrap()
            .unwrap()
            .file_source
            .is_some()
    );
    assert!(
        options(&["--config-edt", "directory"])
            .unwrap()
            .unwrap()
            .file_source
            .is_some()
    );
    for args in [
        vec!["--config-xml=a", "--config-edt=b"],
        vec!["--config-xml=a", "--config-xml=b"],
        vec!["--config-xml="],
        vec!["--config-extension=x=a"],
    ] {
        assert!(options(&args).is_err());
    }
    assert!(
        options(&[
            "--config-xml=a",
            "--config-extension=id=b",
            "--config-extension=ID=c"
        ])
        .is_err()
    );
}
#[test]
fn file_mode_requires_a_database_and_leaves_script_arguments_alone() {
    let mut args = ["bsl-cli", "--config-xml=directory", "main.bsl"]
        .map(str::to_string)
        .to_vec();
    assert!(take(&mut args).is_err());
    let mut args = ["bsl-cli", "main.bsl", "--config-xml=argument"]
        .map(str::to_string)
        .to_vec();
    let before = args.clone();
    assert_eq!(take(&mut args).unwrap(), None);
    assert_eq!(args, before);
}

#[test]
fn protected_module_policy_is_available_for_either_native_file_format() {
    for selector in ["--config-xml=directory", "--config-edt=directory"] {
        assert!(
            options(&[selector, "--decompile-protected-modules"])
                .unwrap()
                .unwrap()
                .decompile_protected_modules
        );
    }
}
