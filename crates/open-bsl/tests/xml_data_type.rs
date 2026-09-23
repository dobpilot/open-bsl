use open_bsl::Engine;

#[test]
fn xml_type_of_and_xml_data_type_match_the_measured_surface() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let module = engine
            .compile(
                r#"
ПространствоXSD = "http://www.w3.org/2001/XMLSchema";
Пары = Новый Массив;
Пары.Добавить(Новый Структура("Значение,Имя", "текст", "string"));
Пары.Добавить(Новый Структура("Значение,Имя", 1, "decimal"));
Пары.Добавить(Новый Структура("Значение,Имя", Истина, "boolean"));
Пары.Добавить(Новый Структура("Значение,Имя", '20240101', "dateTime"));
Для Каждого Пара Из Пары Цикл
    XMLТип = XMLТипЗнч(Пара.Значение);
    Если XMLТип.ИмяТипа <> Пара.Имя Или XMLТип.URIПространстваИмен <> ПространствоXSD Тогда
        ВызватьИсключение "mapping";
    КонецЕсли;
КонецЦикла;
Если XMLТипЗнч(Неопределено) <> Неопределено Или XMLТипЗнч(Новый Массив) <> Неопределено Тогда
    ВызватьИсключение "unmapped";
КонецЕсли;
English = XMLTypeOf("text");
Constructed = New XMLDataType("string", ПространствоXSD);
Если English <> Constructed Или English.TypeName <> "string" Или English.NamespaceURI <> ПространствоXSD Тогда
    ВызватьИсключение "english";
КонецЕсли;
ОшибкаБыла = Ложь;
Попытка Constructed.ИмяТипа = "decimal"; Исключение ОшибкаБыла = Истина; КонецПопытки;
Если Не ОшибкаБыла Тогда ВызватьИсключение "read-only"; КонецЕсли;
"#,
            )
            .unwrap();
        engine.new_state().run(&module).unwrap();
    }
}
