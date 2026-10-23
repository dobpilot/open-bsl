use open_bsl::{Engine, Value};

#[test]
fn fast_infoset_writer_resolves_without_substituting_xml_writer() {
    let engine = Engine::builder().build().unwrap();
    for source in ["Новый ЗаписьFastInfoset", "Новый fastinfosetwriter()"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nПисатель = {source};\nКонецПроцедуры"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nПисатель = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert!(
        engine
            .compile("Писатель = Новый FastInfosetWriter(1);")
            .is_err()
    );
}

#[test]
fn fast_infoset_reader_resolves_without_substituting_xml_reader() {
    let engine = Engine::builder().build().unwrap();
    for source in ["Новый ЧтениеFastInfoset", "Новый fastinfosetreader()"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЧитатель = {source};\nКонецПроцедуры"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЧитатель = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert!(
        engine
            .compile("Читатель = Новый FastInfosetReader(1);")
            .is_err()
    );
}
