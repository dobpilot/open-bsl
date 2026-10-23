#[cfg(feature = "pdf")]
use open_bsl::{Engine, Value};

#[cfg(feature = "pdf")]
#[test]
fn pdf_writer_resolves_documented_forms_without_discarding_output() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый ЗаписьPDF",
        "Новый ЗаписьPDF(\"output.pdf\")",
        "New PDFWriter(Неопределено)",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nПисатель = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nПисатель = Новый ЗаписьPDF;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(engine.compile("Писатель = Новый ЗаписьPDF(1, 2);").is_err());
}
