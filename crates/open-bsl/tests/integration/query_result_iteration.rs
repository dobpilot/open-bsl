use open_bsl::Engine;

#[test]
fn query_result_iteration_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ОбходРезультатаЗапроса.Прямой) <> "Line"
    Или Строка(ОбходРезультатаЗапроса.ПоГруппировкам) <> "By group"
    Или Строка(ОбходРезультатаЗапроса.ПоГруппировкамСИерархией) <> "By group with hierarchy"
    Или Строка(ТипЗнч(ОбходРезультатаЗапроса.Прямой)) <> "QueryResultIteration"
    Или QueryResultIteration.Linear <> ОбходРезультатаЗапроса.Прямой
    Или QueryResultIteration.ByGroups <> ОбходРезультатаЗапроса.ПоГруппировкам
    Или QueryResultIteration.ByGroupsWithHierarchy <> ОбходРезультатаЗапроса.ПоГруппировкамСИерархией Тогда
    ВызватьИсключение "query-result-iteration";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
