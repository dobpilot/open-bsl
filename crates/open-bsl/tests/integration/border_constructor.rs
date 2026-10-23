use open_bsl::{Engine, Value};

#[test]
fn border_constructor_resolves_without_fabricated_value() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Новый Рамка(ТипРамкиЭлементаУправления.БезРамки)",
        "Новый Border(ControlBorderType.Single, 2)",
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
    assert!(engine.compile("Значение = Новый Рамка();").is_err());
    assert!(
        engine
            .compile("Значение = Новый Border(ControlBorderType.Single, 1, 2);")
            .is_err()
    );
}
