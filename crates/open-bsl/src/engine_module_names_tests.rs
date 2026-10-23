use super::*;

fn validate(names: &[&str]) -> Result<(), Error> {
    let modules = names
        .iter()
        .map(|name| ModuleRecipe {
            name: (*name).into(),
            source: String::new(),
            imports: Vec::new(),
            visibility: ModuleVisibility::Named,
        })
        .collect::<Vec<_>>();
    validate_module_names(&modules)
}

#[test]
fn duplicate_module_names_preserve_unicode_folding_and_diagnostic_spelling() {
    for (first, second) in [("Модуль", "мОДУЛЬ"), ("I", "ı"), ("Straße", "STRASSE")] {
        assert!(bsl_rt::folded_eq(first, second));
        let Error::Configuration(message) = validate(&[first, second]).unwrap_err() else {
            panic!("ожидалась ошибка конфигурации");
        };
        assert_eq!(message, format!("имя общего модуля «{second}» повторяется"));
    }
}

#[test]
fn module_name_validation_keeps_input_order_and_does_not_trim() {
    validate(&[]).unwrap();
    validate(&["Модуль", " Модуль", "Модуль ", "Другой"]).unwrap();
    let Error::Configuration(message) = validate(&["A", "a", ""]).unwrap_err() else {
        panic!("ожидалась ошибка конфигурации");
    };
    assert_eq!(message, "имя общего модуля «a» повторяется");
    let Error::Configuration(message) = validate(&["A", "", "a"]).unwrap_err() else {
        panic!("ожидалась ошибка конфигурации");
    };
    assert_eq!(message, "имя общего модуля пусто");
}
