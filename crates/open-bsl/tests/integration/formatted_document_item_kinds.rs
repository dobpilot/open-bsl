use open_bsl::{Engine, Error, Value};

#[test]
fn reached_formatted_document_item_kinds_match_the_native_control() {
    let engine = Engine::builder().build().unwrap();
    for (russian, english, presentation) in [
        ("ПереводСтроки", "LineFeed", "ПереводСтроки"),
        ("Картинка", "Picture", "Картинка"),
        ("Текст", "Text", "Текст"),
    ] {
        let source = format!(
            "ЗначениеПробы = ТипЭлементаФорматированногоДокумента.{russian};
            Возврат ЗначениеПробы = FormattedDocumentItemType.{english}
                И Строка(ЗначениеПробы) = \"{presentation}\"
                И Строка(ТипЗнч(ЗначениеПробы)) = \"ТипЭлементаФорматированногоДокумента\"
                И ТипЗнч(ЗначениеПробы) = Тип(\"ТипЭлементаФорматированногоДокумента\")
                И ТипЗнч(ЗначениеПробы) = Тип(\"FormattedDocumentItemType\");"
        );
        assert_eq!(
            engine.new_state().exec(&source).unwrap(),
            Value::Boolean(true)
        );
    }
}

#[test]
fn guessed_formatted_document_item_aliases_remain_undefined() {
    let engine = Engine::builder().build().unwrap();
    for alias in ["LineBreak", "Image"] {
        assert!(matches!(
            engine.compile(&format!("Возврат FormattedDocumentItemType.{alias};")),
            Err(Error::Semantic(_))
        ));
    }
}
