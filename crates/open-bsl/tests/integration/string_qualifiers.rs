use open_bsl::Engine;

fn run_in_every_mode(source: &str) {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let compiled = engine.compile(source).unwrap();
        for bytecode in [false, true] {
            let module = if bytecode {
                engine.load_bytecode(&compiled.bytecode().unwrap()).unwrap()
            } else {
                engine.compile(source).unwrap()
            };
            engine.new_state().run(&module).unwrap();
        }
    }
}

#[test]
fn string_qualifiers_match_the_measured_value_surface() {
    run_in_every_mode(
        r#"
ПоУмолчанию = Новый КвалификаторыСтроки;
Если ПоУмолчанию.Длина <> 0
    Или ПоУмолчанию.ДопустимаяДлина <> ДопустимаяДлина.Переменная
    Или Строка(ТипЗнч(ПоУмолчанию)) <> "Квалификаторы строки"
    Или Строка(ТипЗнч(ДопустимаяДлина.Переменная)) <> "ДопустимаяДлина" Тогда
    ВызватьИсключение "defaults";
КонецЕсли;

Фиксированные = New StringQualifiers(50, AllowedLength.Fixed);
Если Фиксированные.Length <> 50
    Или Фиксированные.AllowedLength <> AllowedLength.Fixed
    Или Строка(Фиксированные.AllowedLength) <> "Фиксированная" Тогда
    ВызватьИсключение "explicit";
КонецЕсли;

Если (Новый КвалификаторыСтроки(-1)).Длина <> 0
    Или (Новый КвалификаторыСтроки(1.5)).Длина <> 1 Тогда
    ВызватьИсключение "normalization";
КонецЕсли;

Ошибок = 0;
Попытка Фиксированные.Длина = 1; Исключение Ошибок = Ошибок + 1; КонецПопытки;
Попытка Фиксированные.ДопустимаяДлина = ДопустимаяДлина.Переменная; Исключение Ошибок = Ошибок + 1; КонецПопытки;
Если Ошибок <> 2 Тогда ВызватьИсключение "read-only"; КонецЕсли;
"#,
    );
}

#[test]
fn string_qualifiers_reject_wrong_argument_types() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Значение = Новый КвалификаторыСтроки(\"1\");",
        "Значение = Новый КвалификаторыСтроки(1, Неопределено);",
        "Значение = Новый КвалификаторыСтроки(1, ЧастиДаты.ДатаВремя);",
    ] {
        let module = engine.compile(source).unwrap();
        assert!(engine.new_state().run(&module).is_err(), "{source}");
    }
}

#[test]
fn qualified_type_description_accepts_the_reached_constructor_family() {
    run_in_every_mode(
        r#"
Строковый = Новый ОписаниеТипов("Строка", Неопределено, Новый КвалификаторыСтроки(20, ДопустимаяДлина.Фиксированная));
English = New TypeDescription("String", Undefined, New StringQualifiers(7, AllowedLength.Fixed));
Перекрестный = Новый ОписаниеТипов("Строка", Неопределено, Новый КвалификаторыДаты(ЧастиДаты.ДатаВремя));
"#,
    );

    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Значение = Новый ОписаниеТипов(\"Строка\", Неопределено, 1);")
        .unwrap();
    assert!(engine.new_state().run(&module).is_err());
}
