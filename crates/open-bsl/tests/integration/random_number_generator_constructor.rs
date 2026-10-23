use open_bsl::{Engine, Value};

#[test]
fn random_number_generator_resolves_documented_constructors_without_inventing_a_sequence() {
    let engine = Engine::builder().build().unwrap();
    engine
        .compile(
            "Процедура НеВызывается()\nГ = Новый ГенераторСлучайныхЧисел(255);\nКонецПроцедуры\nВозврат 1;",
        )
        .unwrap();
    engine
        .compile("Г = Новый RandomNumberGenerator();")
        .unwrap();
    assert!(
        engine
            .compile("Г = Новый ГенераторСлучайныхЧисел(1, 2);")
            .is_err()
    );
    assert_eq!(
        engine
            .new_state()
            .exec(
                "Попытка\nНовый ГенераторСлучайныхЧисел();\nИсключение\nВозврат Истина;\nКонецПопытки;\nВозврат Ложь;",
            )
            .unwrap(),
        Value::Boolean(true)
    );
}
