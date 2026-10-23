use open_bsl::{Engine, Value};

#[test]
fn html_writer_constructs_and_document_batch_fails_explicitly() {
    let engine = Engine::builder().build().unwrap();
    for (source, unavailable) in [
        ("Новый ЗаписьHTML", false),
        ("Новый HTMLWriter()", false),
        ("Новый ПакетОтображаемыхДокументов", true),
        ("Новый RepresentableDocumentBatch()", true),
    ] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЗначение = {source};\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЗначение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(unavailable)
        );
    }
    assert!(engine.compile("Значение = Новый HTMLWriter(1);").is_err());
    assert!(
        engine
            .compile("Значение = Новый RepresentableDocumentBatch(1);")
            .is_err()
    );
}
