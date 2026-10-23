use open_bsl::{Engine, Value};

#[test]
fn native_open_calls_keep_captured_values_and_check_the_actual_method_arity() {
    let source = r#"
Функция СледующийАргумент(Ключ)
    Ключ = "после";
    Возврат 17;
КонецФункции
Процедура Записать(Получатель)
    Ключ = "до";
    Получатель.Вставить(Ключ, СледующийАргумент(Ключ));
    Если Ключ <> "после" Тогда ВызватьИсключение "byref вложенного вызова"; КонецЕсли;
КонецПроцедуры
Процедура ПроверитьАрность(Получатель)
    Ошибок = 0;
    Попытка Получатель.Количество(123); Исключение Ошибок = Ошибок + 1; КонецПопытки;
    Попытка Получатель.Удалить(); Исключение Ошибок = Ошибок + 1; КонецПопытки;
    Попытка Получатель.Очистить(123); Исключение Ошибок = Ошибок + 1; КонецПопытки;
    Если Ошибок <> 3 Или Получатель.Количество() <> 1 Тогда
        ВызватьИсключение "арность или изменение до отказа";
    КонецЕсли;
КонецПроцедуры
СтруктураДанных = Новый Структура;
Записать(СтруктураДанных);
ПроверитьАрность(СтруктураДанных);
Возврат СтруктураДанных.до;
"#;
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
            assert_eq!(
                engine.new_state().run(&module).unwrap(),
                Value::number_from_i64(17)
            );
        }
    }
}
