//! Пример «Счётчик» — источник правды для README: тест собирает его и
//! запускает, проверяя код возврата и вывод. `CARGO_BIN_EXE_<имя>` для
//! examples не существует, поэтому пример собирается вложенным cargo —
//! это медленнее обычного теста, но единственный способ гонять именно
//! артефакт примера.

use std::process::Command;

#[test]
fn the_readme_counter_example_prints_two() {
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "counter", "-p", "open-bsl"])
        .output()
        .expect("вложенный cargo run не запустился");
    assert!(
        output.status.success(),
        "пример завершился с ошибкой:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2");
}
