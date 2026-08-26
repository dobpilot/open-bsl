#[test]
fn complete_connector_http_2_6_0_compiles_without_source_changes() {
    let source = include_str!("fixtures/connector-http-2.6.0.bsl");
    open_bsl::Engine::builder()
        .build()
        .expect("стандартный набор компонентов корректен")
        .compile(source)
        .expect("неизменённый модуль Connector должен компилироваться");
}
