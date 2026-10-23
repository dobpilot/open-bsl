use super::simplify;

fn clean(source: &str) -> String {
    let result = simplify(source).unwrap();
    bsl_syntax::parse(&result).unwrap();
    assert_eq!(simplify(&result).unwrap(), result);
    result
}

#[test]
fn all_agreed_boolean_forms_fold_without_touching_literal_text() {
    let source = "Функция Проба()\nПерем а, б, в, г;\nа = (Ложь = Ложь);\nб = (Ложь = Истина);\nв = (Не Истина);\nг = (Истина = Истина);\nСообщить(а); Сообщить(б); Сообщить(в); Сообщить(г);\nСообщить(\"(Не Истина) (Истина = Истина)\");\n// (Ложь = Ложь)\nВозврат 0;\nКонецФункции";
    let result = clean(source);
    for assignment in ["а = Истина;", "б = Ложь;", "в = Ложь;", "г = Истина;"]
    {
        assert!(result.contains(assignment), "{result}");
    }
    assert!(result.contains("\"(Не Истина) (Истина = Истина)\""));
    assert!(result.contains("// (Ложь = Ложь)"));
}

#[test]
fn a_fallthrough_label_is_removed_only_without_other_incoming_jumps() {
    let result = clean(
        "Процедура Проба()\nПерейти ~__НативныйБлок_1_13;\n~__НативныйБлок_1_13:\nСообщить(1);\nКонецПроцедуры",
    );
    assert!(!result.contains("__НативныйБлок_1_13"));
    let shared = clean(
        "Процедура Проба()\nПерейти ~__НативныйБлок_1_13;\n~__НативныйБлок_1_13:\nСообщить(1);\nПерейти ~__НативныйБлок_1_13;\nКонецПроцедуры",
    );
    assert_eq!(shared.matches("Перейти ~__НативныйБлок_1_13;").count(), 1);
    assert!(shared.contains("~__НативныйБлок_1_13:"));
}

#[test]
fn unread_pure_constants_and_unused_local_declarations_are_removed() {
    let result = clean(
        "Процедура Проба(Аргумент)\nПерем Мёртвая, Пустая, Живая;\nМёртвая = (\"текст\");\n__open_bsl_image_result_1 = (Не Истина);\nЖивая = 7;\nСообщить(Живая);\nКонецПроцедуры",
    );
    assert!(result.contains("Процедура Проба(Аргумент)"));
    assert!(result.contains("Перем Живая;"));
    assert!(!result.contains("Мёртвая"));
    assert!(!result.contains("Пустая"));
    assert!(!result.contains("__open_bsl_image_result_1"));
}

#[test]
fn reads_are_case_insensitive_and_reference_arguments_are_preserved() {
    let result = clean(
        "Процедура Проба()\nПерем Значение;\nЗначение = Истина;\nИзменить(зНАЧЕНИЕ);\nКонецПроцедуры",
    );
    assert!(result.contains("Перем Значение;"));
    assert!(result.contains("Значение = Истина;"));
    assert!(result.contains("Изменить(зНАЧЕНИЕ);"));
}

#[test]
fn calls_property_reads_and_module_storage_are_not_discarded() {
    let source = "Перем Глобальная;\nПроцедура Проба()\nПерем Результат, Свойство;\nРезультат = ПобочныйЭффект();\nСвойство = Объект.Поле;\n__open_bsl_image_result_8 = Новый Структура;\nКонецПроцедуры\nГлобальная = Истина;";
    assert_eq!(clean(source), source);
}

#[test]
fn dynamic_code_keeps_the_complete_local_scope() {
    for statement in [
        "Выполнить Текст;",
        "Execute(Текст);",
        "Вычислить(Текст);",
        "Eval(Текст);",
    ] {
        let source = format!(
            "Процедура Проба(Текст)\nПерем Значение;\nЗначение = Истина;\n{statement}\nКонецПроцедуры"
        );
        assert_eq!(clean(&source), source);
    }
}

#[test]
fn a_method_named_execute_does_not_disable_cleanup() {
    let result = clean(
        "Процедура Проба(Объект)\nПерем Мёртвая;\nМёртвая = Истина;\nОбъект.Выполнить();\nКонецПроцедуры",
    );
    assert!(!result.contains("Мёртвая"));
    assert!(result.contains("Объект.Выполнить();"));
}

#[test]
fn comments_inside_declarations_and_assignments_are_not_deleted() {
    let source =
        "Процедура Проба()\nПерем А, // описание\nБ;\nА = Истина // причина\n;\nКонецПроцедуры";
    assert_eq!(clean(source), source);
}

#[test]
fn a_removed_constant_exposes_another_fallthrough_jump() {
    let result = clean(
        "Процедура Проба()\nПерейти ~__НативныйБлок_1_2;\n__open_bsl_image_result_1 = Истина;\n~__НативныйБлок_1_2:\nСообщить(1);\nКонецПроцедуры",
    );
    assert!(!result.contains("__НативныйБлок_1_2"));
    assert!(result.contains("Сообщить(1);"));
}

#[test]
fn unread_negation_of_false_is_removed_without_folding_other_expressions() {
    let result = clean(
        "Процедура Проба()\n__open_bsl_image_result_1 = (Не Ложь);\nСообщить((Истина = Ложь));\nКонецПроцедуры",
    );
    assert!(!result.contains("__open_bsl_image_result_1"));
    assert!(result.contains("Сообщить((Истина = Ложь));"));
}

#[test]
fn a_generated_name_bound_in_module_storage_is_not_removed_from_a_method() {
    let source = "Перем __open_bsl_image_result_1;\nПроцедура Проба()\n__open_bsl_image_result_1 = Истина;\nКонецПроцедуры\n__open_bsl_image_result_1 = Ложь;";
    assert_eq!(clean(source), source);
}

#[test]
fn keyword_members_are_not_declarations_or_scope_boundaries() {
    let source = "Процедура Проба(Объект)\nПерем Результат;\nРезультат = Объект.Var.Метод();\nОбъект.EndProcedure();\nОбъект.Function();\nКонецПроцедуры";
    assert_eq!(super::simplify(source).unwrap(), source);
}
