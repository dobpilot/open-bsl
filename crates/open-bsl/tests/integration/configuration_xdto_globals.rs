#![cfg(feature = "xml")]

use open_bsl::Engine;

#[test]
fn configuration_serializer_globals_preserve_the_configuration_factory_error() {
    let engine = Engine::builder().build().unwrap();
    let mut failures = Vec::new();
    for source in [
        "Возврат ФабрикаXDTO;",
        "Возврат XDTOFactory;",
        "Возврат СериализаторXDTO;",
        "Возврат XDTOSerializer;",
        "СериализаторXDTO.ЗаписатьXML(Неопределено, Истина);",
        "XDTOSerializer.WriteXML(Undefined, True);",
    ] {
        let module = engine.compile(source).unwrap();
        let error = engine.new_state().run(&module).unwrap_err().to_string();
        assert!(
            error.contains("глобальная ФабрикаXDTO"),
            "{source}: {error}"
        );
        failures.push(error);
    }
    assert!(failures.iter().all(|error| error == &failures[0]));
}
