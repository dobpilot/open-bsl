use open_bsl::{Engine, Error, Value};

fn transfer_structure(noise: &str) -> Result<Value, Error> {
    let engine = Engine::builder().build().unwrap();
    let producer = engine
        .compile(&format!(
            r#"
{noise}
Функция Создать() Экспорт
    Возврат Новый Структура("Right", "Read");
КонецФункции
Функция Прочитать(Данные) Экспорт
    Возврат Данные.Right;
КонецФункции
"#
        ))
        .unwrap();
    let consumer = engine
        .compile(
            r#"
Функция Прочитать(Данные) Экспорт
    Возврат Данные.rIgHt;
КонецФункции
"#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let producer = state.instantiate(&producer).unwrap();
    let consumer = state.instantiate(&consumer).unwrap();
    let value = producer.call(&mut state, "Создать", vec![]).unwrap().0;
    assert_eq!(
        producer
            .call(&mut state, "Прочитать", vec![value.clone()])
            .unwrap()
            .0,
        Value::Str("Read".into())
    );
    consumer
        .call(&mut state, "Прочитать", vec![value])
        .map(|result| result.0)
}

#[test]
fn structures_pass_between_instances_with_aligned_name_tables() {
    assert_eq!(transfer_structure("").unwrap(), Value::Str("Read".into()));
}

#[test]
fn structures_keep_field_names_between_instances_with_different_name_tables() {
    assert_eq!(
        transfer_structure("Функция Шум(Данные) Экспорт Возврат Данные.ШумовоеПоле; КонецФункции")
            .unwrap(),
        Value::Str("Read".into())
    );
}

#[test]
fn shared_mutation_and_dynamic_fields_survive_a_failed_call() {
    let engine = Engine::builder().build().unwrap();
    let producer = engine
        .compile(
            r#"
Функция Создать() Экспорт Возврат Новый Структура("Right", "Read"); КонецФункции
Функция Прочитать(Данные, Поле) Экспорт
    ЗначениеПоля = Неопределено;
    Если Не Данные.Свойство(Поле, ЗначениеПоля) Тогда ВызватьИсключение "поле потеряно"; КонецЕсли;
    Возврат ЗначениеПоля;
КонецФункции
"#,
        )
        .unwrap();
    let consumer = engine
        .compile(
            r#"
Функция Шум(Данные) Экспорт Возврат Данные.ДругоеПоле; КонецФункции
Процедура Изменить(Данные, Дубль, Поле) Экспорт
    Данные.Right = "Changed";
    Дубль.Вставить(Поле, "After");
    Данные.Вставить("Self", Дубль);
    ВызватьИсключение "ожидаемый отказ";
КонецПроцедуры
"#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let producer = state.instantiate(&producer).unwrap();
    let consumer = state.instantiate(&consumer).unwrap();
    let value = producer.call(&mut state, "Создать", vec![]).unwrap().0;
    assert!(
        consumer
            .call(
                &mut state,
                "Изменить",
                vec![
                    value.clone(),
                    value.clone(),
                    Value::Str("RuntimeField".into())
                ]
            )
            .is_err()
    );
    for (field, expected) in [("Right", "Changed"), ("RuntimeField", "After")] {
        assert_eq!(
            producer
                .call(
                    &mut state,
                    "Прочитать",
                    vec![value.clone(), Value::Str(field.into())]
                )
                .unwrap()
                .0,
            Value::Str(expected.into())
        );
    }
    let self_value = producer
        .call(
            &mut state,
            "Прочитать",
            vec![value.clone(), Value::Str("Self".into())],
        )
        .unwrap()
        .0;
    let (Value::Object(actual), Value::Object(original)) = (&self_value, &value) else {
        panic!("цикл должен содержать исходный объект структуры");
    };
    assert!(std::rc::Rc::ptr_eq(actual, original));
}

#[test]
fn binding_preserves_literal_key_spelling_and_compiled_bytes() {
    let engine = Engine::builder().build().unwrap();
    let first = engine
        .compile("Функция Поле(Данные) Экспорт Возврат Данные.Right; КонецФункции")
        .unwrap();
    let second = engine
        .compile(
            r#"
Функция Создать() Экспорт Возврат Новый Структура("rIgHt", "Read"); КонецФункции
Функция Ключ(Данные) Экспорт
    Для Каждого Поле Из Данные Цикл Возврат Поле.Ключ; КонецЦикла;
КонецФункции
"#,
        )
        .unwrap();
    let bytes = second.bytecode().unwrap();
    for reverse in [false, true] {
        let mut state = engine.new_state();
        let second_instance = if reverse {
            let instance = state.instantiate(&second).unwrap();
            state.instantiate(&first).unwrap();
            instance
        } else {
            state.instantiate(&first).unwrap();
            state.instantiate(&second).unwrap()
        };
        let value = second_instance
            .call(&mut state, "Создать", vec![])
            .unwrap()
            .0;
        assert_eq!(
            second_instance
                .call(&mut state, "Ключ", vec![value])
                .unwrap()
                .0,
            Value::Str("rIgHt".into())
        );
    }
    assert_eq!(second.bytecode().unwrap(), bytes);
}

#[test]
fn dynamic_expression_uses_names_created_earlier_in_the_call() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
Функция Прочитать(Поле) Экспорт
    Данные = Новый Структура;
    Данные.Вставить("Other" + Поле, "wrong");
    Данные.Вставить(Поле, "right");
    Возврат Вычислить("Данные." + Поле);
КонецФункции
"#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    assert_eq!(
        instance
            .call(
                &mut state,
                "Прочитать",
                vec![Value::Str("RuntimeField".into())]
            )
            .unwrap()
            .0,
        Value::Str("right".into())
    );
}

#[test]
fn bytecode_and_optimized_instances_share_fields_in_both_creation_orders() {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        let producer = engine
            .compile(
                r#"
Функция Шум(Данные) Экспорт Возврат Данные.Other; КонецФункции
Функция Создать() Экспорт Возврат Новый Структура("Right", "Read"); КонецФункции
"#,
            )
            .unwrap();
        let consumer = engine
            .compile("Функция Читать(Данные) Экспорт Возврат Данные.Right; КонецФункции")
            .unwrap();
        for loaded in [false, true] {
            let producer = if loaded {
                engine.load_bytecode(&producer.bytecode().unwrap()).unwrap()
            } else {
                producer.clone()
            };
            let consumer = if loaded {
                engine.load_bytecode(&consumer.bytecode().unwrap()).unwrap()
            } else {
                consumer.clone()
            };
            for reverse in [false, true] {
                let mut state = engine.new_state();
                let (producer, consumer) = if reverse {
                    let second = state.instantiate(&consumer).unwrap();
                    (state.instantiate(&producer).unwrap(), second)
                } else {
                    (
                        state.instantiate(&producer).unwrap(),
                        state.instantiate(&consumer).unwrap(),
                    )
                };
                let value = producer.call(&mut state, "Создать", vec![]).unwrap().0;
                assert_eq!(
                    consumer.call(&mut state, "Читать", vec![value]).unwrap().0,
                    Value::Str("Read".into())
                );
            }
        }
    }
}

#[test]
fn common_modules_and_root_callbacks_keep_shared_structure_fields() {
    let engine = Engine::builder()
        .common_module(
            "Общий",
            r#"
Перем Сохраненная;
Процедура Запомнить(Данные) Экспорт Сохраненная = Данные; КонецПроцедуры
Функция Получить() Экспорт Возврат Сохраненная; КонецФункции
Функция ЧерезОбратныйВызов(Получатель, Данные) Экспорт
    Возврат Получатель.Завершить(Данные);
КонецФункции
"#,
        )
        .build()
        .unwrap();
    let producer = engine
        .compile_entry(
            r#"
Функция Создать() Экспорт
    Данные = Новый Структура("Right", "Read");
    Общий.Запомнить(Данные);
    Возврат Данные;
КонецФункции
Функция Прочитать(Данные) Экспорт Возврат Данные.Callback; КонецФункции
"#,
        )
        .unwrap();
    let consumer = engine
        .compile_entry(
            r#"
Функция Шум(Данные) Экспорт Возврат Данные.Other; КонецФункции
Функция Завершить(Данные) Экспорт
    Данные.Вставить("Callback", "Done");
    Возврат Данные.Right;
КонецФункции
Функция Проверить(Данные) Экспорт
    Если Общий.Получить() <> Данные Тогда ВызватьИсключение "копия"; КонецЕсли;
    Возврат Общий.ЧерезОбратныйВызов(ЭтотОбъект, Данные);
КонецФункции
"#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let producer = state.instantiate(&producer).unwrap();
    let consumer = state.instantiate(&consumer).unwrap();
    let value = producer.call(&mut state, "Создать", vec![]).unwrap().0;
    assert_eq!(
        consumer
            .call(&mut state, "Проверить", vec![value.clone()])
            .unwrap()
            .0,
        Value::Str("Read".into())
    );
    assert_eq!(
        producer
            .call(&mut state, "Прочитать", vec![value])
            .unwrap()
            .0,
        Value::Str("Done".into())
    );
}

#[test]
fn runtime_names_do_not_block_open_members_before_the_module_body() {
    let engine = Engine::builder().build().unwrap();
    let filler = engine
        .compile(
            r#"
Функция Заполнить() Экспорт
    Данные = Новый Структура;
    Для ИндексПоля = 0 По 65536 Цикл
        Данные.Вставить("Runtime" + Строка(ИндексПоля), ИндексПоля);
    КонецЦикла;
    Возврат Данные;
КонецФункции
Функция Количество(Данные) Экспорт Возврат Данные.Количество(); КонецФункции
"#,
        )
        .unwrap();
    let overflow = engine
        .compile(
            r#"
Функция НеВызывается(Данные) Экспорт Возврат Данные.НовоеИмяМетода(); КонецФункции
ВызватьИсключение "тело не должно исполняться";
"#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let filler = state.instantiate(&filler).unwrap();
    let value = filler.call(&mut state, "Заполнить", vec![]).unwrap().0;
    let error = match state.instantiate(&overflow) {
        Ok(_) => panic!("связывание должно отклонить индекс до исполнения тела"),
        Err(error) => error,
    };
    assert!(matches!(error, Error::Runtime(_)));
    assert!(error.to_string().contains("тело не должно исполняться"));
    assert_eq!(
        filler
            .call(&mut state, "Количество", vec![value])
            .unwrap()
            .0,
        Value::number_from_i64(65537)
    );
}

#[cfg(feature = "textdoc")]
#[test]
fn typed_component_links_are_rebound_with_their_member_names() {
    let engine = Engine::builder().build().unwrap();
    let noise = engine
        .compile("Функция Шум(Данные) Экспорт Возврат Данные.ДругоеПоле; КонецФункции")
        .unwrap();
    let module = engine
        .compile(
            r#"
Функция Прочитать() Экспорт
    Документ = Новый ТекстовыйДокумент;
    Документ.УстановитьТекст("Read");
    Возврат Документ.ПолучитьТекст();
КонецФункции
Возврат Прочитать();
"#,
        )
        .unwrap();
    let expected = engine.new_state().run(&module).unwrap();
    let mut state = engine.new_state();
    state.instantiate(&noise).unwrap();
    let instance = state.instantiate(&module).unwrap();
    assert_eq!(
        instance.call(&mut state, "Прочитать", vec![]).unwrap().0,
        expected
    );
}
