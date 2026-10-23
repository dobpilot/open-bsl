use open_bsl::Engine;

#[test]
fn table_representation_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ОтображениеТаблицы.Список) <> "Список"
    Или Строка(ОтображениеТаблицы.ИерархическийСписок) <> "Иерархический список"
    Или Строка(ОтображениеТаблицы.Дерево) <> "Дерево"
    Или Строка(ТипЗнч(ОтображениеТаблицы.Список)) <> "ОтображениеТаблицы"
    Или TableRepresentation.List <> ОтображениеТаблицы.Список
    Или TableRepresentation.HierarchicalList <> ОтображениеТаблицы.ИерархическийСписок
    Или TableRepresentation.Tree <> ОтображениеТаблицы.Дерево Тогда
    ВызватьИсключение "table-representation";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
