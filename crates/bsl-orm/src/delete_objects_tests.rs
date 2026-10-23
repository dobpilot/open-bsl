use super::{engine, state};
use crate::Value;

#[test]
fn deletion_rejects_writes_without_changing_caller_values() {
    let engine = engine();
    let mut state = state(&engine);
    for name in ["УдалитьОбъекты", "DeleteObjects", "dElEtEoBjEcTs"] {
        let source = format!(
            r#"
            СсылкиПробы = Новый Массив;
            СсылкиПробы.Добавить("retain");
            ТаблицаПробы = Новый ТаблицаЗначений;
            ТаблицаПробы.Колонки.Добавить("Данные");
            ТаблицаПробы.Добавить().Данные = "retain";
            Попытка
                {name}(СсылкиПробы, Истина, ТаблицаПробы);
                Возврат "unexpected success";
            Исключение
                ОтказПерехваченПробы = Истина;
            КонецПопытки;
            Возврат ОтказПерехваченПробы И СсылкиПробы.Количество() = 1 И СсылкиПробы[0] = "retain"
                И ТаблицаПробы.Количество() = 1 И ТаблицаПробы[0].Данные = "retain";
            "#
        );
        assert_eq!(state.exec(&source).unwrap(), Value::Boolean(true), "{name}");
    }
}

#[test]
fn deletion_accepts_only_the_documented_argument_counts() {
    let engine = engine();
    let mut state = state(&engine);
    for count in 0..=7 {
        let arguments = vec!["Неопределено"; count].join(", ");
        let error = state
            .exec(&format!("DeleteObjects({arguments});"))
            .unwrap_err()
            .to_string();
        if (1..=6).contains(&count) {
            assert!(
                error.contains("недоступно в хосте только для чтения"),
                "{count}: {error}"
            );
        } else {
            assert!(error.contains("аргумент"), "{count}: {error}");
        }
    }
}
