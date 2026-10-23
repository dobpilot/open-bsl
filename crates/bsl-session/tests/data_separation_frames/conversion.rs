use super::*;

#[test]
fn mode_flag_conversion_matches_native_values_without_changing_failed_counters() {
    let factory = factory();
    // Наблюдатель читает только счётчик кадра. Авторитетный геттер зоны
    // не подменяется в реализации; текст диагностик проверяется отдельно.
    let source = include_str!(
        "../../../../docs/research/fixtures/data-separation-frames-1017/measure-flags.bsl"
    )
    .replace(
        "БезопасныйРежимРазделенияДанных(\"НаименованиеЯзык1\")",
        "Константы.НаблюдениеРежима.Получить()",
    );
    let value_rows = |text: &str| {
        text.lines()
            .filter(|line| !line.starts_with("UNKNOWN.") && !line.starts_with("NAME.NUMBER="))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let expected = value_rows(include_str!(
        "../../../../docs/research/fixtures/data-separation-frames-1017/measure-flags.platform.txt"
    ));
    assert_eq!(expected.len(), 24);
    for number in [1, 2] {
        let mut session = factory
            .open(
                Principal::new("probe", "", vec![]),
                SessionResources::new(Box::new(()), number)
                    .with_data_access(Rc::new(ModeObserver))
                    .with_data_separation_common_attributes(["НаименованиеЯзык1".to_string()]),
            )
            .unwrap();
        session.execute(&source).unwrap();
        let actual = collect(&mut session);
        assert_eq!(value_rows(&actual), expected);
        assert!(actual.lines().any(|line| {
            line.starts_with("UNKNOWN.NUMBER=") && line.contains("не найден")
        }));
        assert!(
            actual.lines().any(|line| {
                line.starts_with("UNKNOWN.BADWORD=") && line.contains("Булево")
            })
        );
    }
}
