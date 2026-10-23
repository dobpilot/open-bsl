use open_bsl::Engine;

#[test]
fn query_result_iteration_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ОбходРезультатаЗапроса.Прямой) <> "Прямой"
    Или Строка(ОбходРезультатаЗапроса.ПоГруппировкам) <> "По группировкам"
    Или Строка(ОбходРезультатаЗапроса.ПоГруппировкамСИерархией) <> "По группировкам с иерархией"
    Или Строка(ТипЗнч(ОбходРезультатаЗапроса.Прямой)) <> "ОбходРезультатаЗапроса"
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
