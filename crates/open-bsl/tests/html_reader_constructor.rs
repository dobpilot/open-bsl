use open_bsl::{Engine, Value};

#[test]
fn html_reader_constructor_resolves_without_fabricated_reader() {
    let engine = Engine::builder().build().unwrap();
    for source in ["Новый ЧтениеHTML", "Новый HTMLReader()"] {
        engine
            .compile(&format!(
                "Процедура НеВызывается()\nЧтение = {source};\nКонецПроцедуры\nВозврат 1;"
            ))
            .unwrap();
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Попытка\nЧтение = {source};\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;"
                ))
                .unwrap(),
            Value::Boolean(true)
        );
    }
    assert!(engine.compile("Чтение = Новый HTMLReader(1);").is_err());
}
