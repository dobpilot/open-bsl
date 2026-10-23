use super::*;

#[test]
fn only_current_parameter_tokens_are_converted() {
    let used = Value::number_from_i64(41);
    let unused = Value::new_dictionary_structure(vec![]);
    let supplied = [
        ("ЗНАЧЕНИЕ", &used),
        ("Строка", &unused),
        ("Комментарий", &unused),
    ];
    let selected = query_parameters(
        r#"ВЫБРАТЬ &значение КАК Значение, "&Строка" КАК Строка // &Комментарий"#,
        supplied.into_iter(),
    )
    .unwrap();
    assert_eq!(
        selected,
        vec![QueryParameter::new(
            "ЗНАЧЕНИЕ",
            ParameterValue::Number {
                unscaled: 41,
                scale: 0
            }
        )]
    );
    assert!(parameter_value(&unused).is_err());
}

#[test]
fn a_changed_text_does_not_destroy_retained_parameters() {
    let used = Value::number_from_i64(41);
    let supplied = [("Значение", &used)];
    assert!(
        query_parameters("SELECT 42", supplied.into_iter())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        query_parameters("SELECT &Значение", supplied.into_iter())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn missing_references_remain_the_sdk_error() {
    let selected = query_parameters("SELECT &Отсутствующий", std::iter::empty()).unwrap();
    assert!(selected.is_empty());
    let metadata = open_sdbl::metadata::resolve_metadata(
        open_sdbl::metadata::DbNames::from_entries(vec![]),
        vec![],
        open_sdbl::metadata::SchemaStorage {
            tables: vec![],
            anomalies: vec![],
        },
        vec![],
    )
    .snapshot;
    let error = open_sdbl::query::QueryCompiler::new(&metadata, open_sdbl::query::PostgresBackend)
        .compile_with(
            "SELECT &Отсутствующий",
            &CompileOptions::new().parameters(&selected),
        )
        .unwrap_err();
    assert_eq!(error.missing_parameter_name(), Some("Отсутствующий"));
}
