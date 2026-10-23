use open_bsl::Engine;

#[test]
fn data_composition_filter_application_type_has_the_complete_measured_surface() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Если Строка(ТипПримененияОтбораКомпоновкиДанных.Элементы) <> "Обычное"
    Или Строка(ТипПримененияОтбораКомпоновкиДанных.Иерархия) <> "После группировки или для иерархии"
    Или Строка(ТипПримененияОтбораКомпоновкиДанных.ТолькоИерархия) <> "Только иерархия"
    Или Строка(ТипЗнч(ТипПримененияОтбораКомпоновкиДанных.Элементы)) <> "ТипПримененияОтбораКомпоновкиДанных"
    Или DataCompositionFilterApplicationType.Items <> ТипПримененияОтбораКомпоновкиДанных.Элементы
    Или DataCompositionFilterApplicationType.Hierarchy <> ТипПримененияОтбораКомпоновкиДанных.Иерархия
    Или DataCompositionFilterApplicationType.HierarchyOnly <> ТипПримененияОтбораКомпоновкиДанных.ТолькоИерархия Тогда
    ВызватьИсключение "data-composition-filter-application-type";
КонецЕсли;
"#,
        )
        .unwrap();
    engine.new_state().run(&module).unwrap();
}
