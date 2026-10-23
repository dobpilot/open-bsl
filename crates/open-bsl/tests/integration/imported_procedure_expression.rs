use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility, Value};

#[test]
fn imported_procedure_expression_matches_native_argument_and_branch_timing() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder().optimizations(optimizations)
            .configuration(ModuleGraphRecipe {
                modules: vec![
                    ModuleRecipe {
                        name: "ЗамерыЛенивойАрности".into(),
                        source: include_str!("../../../../docs/research/fixtures/imported-procedure-expression/argument-module.bsl").into(),
                        imports: vec![], visibility: ModuleVisibility::Named,
                    },
                    ModuleRecipe {
                        name: "Проба".into(), source: r#"
Функция Проверить() Экспорт
    ТекстПробы = "initial";
    СледПробы = "";
    ОшибокПробы = 0;
    Попытка
        РезультатПробы = ЗамерыЛенивойАрности.ИзменитьТекст(ТекстПробы);
    Исключение
        ОшибокПробы = ОшибокПробы + 1;
    КонецПопытки;
    Попытка
        РезультатПробы = ЗамерыЛенивойАрности.ИзменитьТекст(ЗамерыЛенивойАрности.АргументПробы(СледПробы));
    Исключение
        ОшибокПробы = ОшибокПробы + 1;
    КонецПопытки;
    Если Ложь Тогда
        РезультатПробы = ЗамерыЛенивойАрности.ИзменитьТекст(ТекстПробы);
    КонецЕсли;
    Возврат ТекстПробы + "|" + СледПробы + "|" + Строка(ОшибокПробы);
КонецФункции
"#.into(), imports: vec![], visibility: ModuleVisibility::Named,
                    },
                ], eager_init: false,
            }).configuration_roots(["Проба"]).build().unwrap();
        let entry = engine.compile_entry("Возврат Проба.Проверить();").unwrap();
        assert_eq!(
            engine.new_state().run(&entry).unwrap(),
            Value::Str("initial|argument|2".into())
        );
    }
}
