use super::*;

#[test]
fn declarations_preserve_order_flags_defaults_and_skip_body_syntax() {
    let source = "Перем Первая, Вторая Экспорт;
        Процедура Скрытая() Синтаксически = ; КонецПроцедуры
        Асинх Функция Открытая(Знач Аргумент, Второй = 5) Экспорт
            Перем Локальная; Возврат (;
        КонецФункции";
    let declarations = scan_declarations_with_symbols(source, &PreprocSymbols::new()).unwrap();
    assert_eq!(declarations.variables.len(), 1);
    assert_eq!(declarations.variables[0].names, ["Первая", "Вторая"]);
    assert_eq!(declarations.functions.len(), 2);
    assert!(declarations.functions[0].is_procedure);
    let public = &declarations.functions[1];
    assert_eq!(public.name, "Открытая");
    assert!(public.is_async && public.export && public.params[0].by_val);
    assert!(public.params[1].default.is_some());
    assert!(parse(source).is_err());
}

#[test]
fn declarations_use_the_existing_lexer_and_preprocessor() {
    let source = "#Если Сервер Тогда\nФункция Серверная() Экспорт\nВозврат \"КонецФункции\";\nКонецФункции\n#Иначе\nФункция Клиентская() Экспорт\nКонецФункции\n#КонецЕсли";
    let mut symbols = PreprocSymbols::new();
    symbols.set("Сервер", true);
    let declarations = scan_declarations_with_symbols(source, &symbols).unwrap();
    assert_eq!(declarations.functions.len(), 1);
    assert_eq!(declarations.functions[0].name, "Серверная");
}
