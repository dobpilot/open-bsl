#[test]
fn owned_transfer_preserves_large_numbers_defaults_and_control_flow() {
    let source = r#"
Перем Общая Экспорт;
Функция Проба(Знач Параметр = 12345678901234567890123456789012345678901234567890.123456789) Экспорт
    Общая = Новый Структура("Поле", Параметр);
    Для Индекс = 1 По 3 Цикл
        Попытка
            Если Индекс = 2 Тогда Продолжить; КонецЕсли;
            Общая.Поле = Общая.Поле + 0.000000000000000000000000000000000000000000000001;
        Исключение
            ВызватьИсключение;
        КонецПопытки;
    КонецЦикла;
    Возврат ?(Истина, Общая.Поле, Неопределено);
КонецФункции
"#;
    let syntax = bsl_syntax::parse(source).unwrap();
    let resolved = crate::resolve_program(&syntax.items).unwrap();
    let expected = resolved.clone();
    // Дерево передаётся между потоками подготовки как есть: одно
    // определение без переносимой копии.
    let moved = std::thread::spawn(move || resolved).join().unwrap();
    assert_eq!(moved, expected);
}
