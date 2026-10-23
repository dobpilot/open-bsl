use open_bsl::{Engine, Value};

#[test]
fn shortcut_constructor_resolves_without_fabricated_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый СочетаниеКлавиш(Клавиша.A)",
        "Новый Shortcut(Key.V, Ложь, Истина, Истина)",
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
            Value::Boolean(true)
        );
    }
    assert!(
        engine
            .compile("Значение = Новый СочетаниеКлавиш();")
            .is_err()
    );
    assert!(
        engine
            .compile("Значение = Новый Shortcut(Key.A, 1, 2, 3, 4);")
            .is_err()
    );
}
