use std::process::Command;

#[test]
fn timing_child() {
    if std::env::var_os("OPEN_BSL_TIMING_TEST_CHILD").is_none() {
        return;
    }
    // Ребёнок ведёт себя как хост: читает переключатель один раз и передаёт
    // его фасаду, как `bsl-cli`.
    open_bsl::set_startup_timings(
        std::env::var_os("OPEN_BSL_STARTUP_TIMINGS").is_some_and(|value| value == "1"),
    );
    open_bsl::Engine::builder()
        .common_module("Проба", "Функция Ответ() Экспорт Возврат 42; КонецФункции")
        .configuration_roots(["Проба"])
        .build()
        .unwrap();
}

#[test]
fn startup_timings_are_opt_in_and_have_numeric_stage_durations() {
    for setting in [None, Some("0"), Some("true"), Some("1")] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "startup_timings::timing_child",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("OPEN_BSL_TIMING_TEST_CHILD", "1")
            .env_remove("OPEN_BSL_STARTUP_TIMINGS");
        if let Some(value) = setting {
            command.env("OPEN_BSL_STARTUP_TIMINGS", value);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        if setting != Some("1") {
            assert!(stderr.is_empty(), "{stderr}");
            continue;
        }
        for stage in [
            "engine.rooted.exports",
            "engine.rooted.selection",
            "engine.catalog.codegen",
            "engine.catalog.verify",
        ] {
            assert!(
                stderr.contains(&format!("open-bsl.startup\t{stage}\t")),
                "{stderr}"
            );
        }
        for line in stderr.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0], "open-bsl.startup");
            assert!(fields[2].parse::<f64>().unwrap() >= 0.0);
        }
    }
}
