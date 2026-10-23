use open_bsl::Engine;

#[test]
fn table_representation_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ОтображениеТаблицы.Список) <> "List"
    Или Строка(ОтображениеТаблицы.ИерархическийСписок) <> "Hierarchical list"
    Или Строка(ОтображениеТаблицы.Дерево) <> "Tree"
    Или Строка(ТипЗнч(ОтображениеТаблицы.Список)) <> "TableRepresentation"
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
