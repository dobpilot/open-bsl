#[cfg(feature = "pdf")]
use open_bsl::{Engine, Value};

#[cfg(feature = "pdf")]
#[test]
fn pdf_representation_description_resolves_without_fabricating_properties() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый ОписаниеОтображаемогоОбъектаPDF",
        "New PDFRepresentationObjectDescription()",
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nОписание = {source};\nКонецПроцедуры"
            ))
            .unwrap();
    }
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nОписание = Новый ОписаниеОтображаемогоОбъектаPDF;\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Описание = Новый ОписаниеОтображаемогоОбъектаPDF(1);")
            .is_err()
    );
}
