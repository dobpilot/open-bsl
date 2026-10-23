use open_bsl::{Engine, Value};

#[test]
fn dynamic_fragments_keep_the_module_owner_in_functions_and_nested_scopes() {
    let functions = r#"
Перем ИзТела Экспорт;
Функция Прямая() Экспорт
    Возврат Вычислить("Нужная");
КонецФункции
Функция Вложенная() Экспорт
    Возврат Вычислить("Вычислить(""Нужная"")");
КонецФункции
Функция ЧерезФункцию() Экспорт
    Возврат Вычислить("Прямая()");
КонецФункции
"#;
    let first = format!(
        "Перем Первый, Нужная; {functions} Первый = 11; Нужная = 22; ИзТела = Вычислить(\"Нужная\");"
    );
    let second = format!(
        "Перем Нужная, Второй; {functions} Нужная = 33; Второй = 44; ИзТела = Вычислить(\"Нужная\");"
    );
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .common_module("ПервыйМодуль", &first)
            .common_module("ВторойМодуль", &second)
            .build()
            .unwrap();
        for order in [
            [("ПервыйМодуль", 22), ("ВторойМодуль", 33)],
            [("ВторойМодуль", 33), ("ПервыйМодуль", 22)],
        ] {
            let mut source = String::new();
            for (module, expected) in order {
                for member in ["ИзТела", "Прямая()", "Вложенная()", "ЧерезФункцию()"]
                {
                    source.push_str(&format!(
                        "Если {module}.{member} <> {expected} Тогда ВызватьИсключение \"{module}.{member}\"; КонецЕсли;\n"
                    ));
                }
            }
            source.push_str("Возврат 1;");
            let compiled = engine.compile_entry(&source).unwrap();
            for bytecode in [false, true] {
                let module = if bytecode {
                    engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
                } else {
                    engine.compile_entry(&source).unwrap()
                };
                let mut state = engine.new_state();
                for _ in 0..2 {
                    assert_eq!(state.run(&module).unwrap(), Value::number_from_i64(1));
                }
            }
        }
    }
}
