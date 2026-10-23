use crate::Engine;
use bsl_rt::RuntimeShapes;

#[test]
fn late_members_keep_local_indices_after_seventy_thousand_runtime_names() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.state_builder().build();
    state.instances.runtime = Some(RuntimeShapes::seeded(
        (0..70_000).map(|index| format!("Занято{index}")).collect(),
        Vec::new(),
        Some(engine.registry()),
    ));
    let producer = engine
        .compile("Функция Создать() Экспорт Возврат Новый Структура(\"Поле\", 17); КонецФункции")
        .unwrap();
    let producer = state.instantiate(&producer).unwrap();
    let value = producer.call(&mut state, "Создать", vec![]).unwrap().0;
    let consumer = engine.compile(
        "Функция Проверить(Данные) Экспорт М = Новый Массив; М.Добавить(Данные.пОЛЕ); Возврат М[0]; КонецФункции Процедура Позже(Объект) Объект.ОченьРедкийМетод(); КонецПроцедуры"
    ).unwrap();
    let consumer = state.instantiate(&consumer).unwrap();
    assert_eq!(
        consumer
            .call(&mut state, "Проверить", vec![value])
            .unwrap()
            .0,
        crate::Value::number_from_i64(17)
    );
}

#[test]
fn dynamic_code_keeps_static_functions_and_shared_fields_after_namespace_growth() {
    let engine = Engine::builder().build().unwrap();
    let mut state = engine.state_builder().build();
    state.instances.runtime = Some(RuntimeShapes::seeded(
        (0..70_000).map(|index| format!("Занято{index}")).collect(),
        Vec::new(),
        Some(engine.registry()),
    ));
    let module = engine
        .compile(
            r#"Функция Прочитать(Данные)
            Возврат Данные.Поле;
        КонецФункции
        Функция Проверить() Экспорт
            Данные = Новый Структура("Поле", 17);
            Выполнить("Данные.Поле = Прочитать(Данные) + 1");
            Возврат Вычислить("Прочитать(Данные)");
        КонецФункции"#,
        )
        .unwrap();
    let instance = state.instantiate(&module).unwrap();
    assert_eq!(
        instance.call(&mut state, "Проверить", vec![]).unwrap().0,
        crate::Value::number_from_i64(18)
    );
}
