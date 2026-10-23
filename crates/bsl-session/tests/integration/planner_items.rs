use open_bsl::{BslObject, Engine, Value, fixed_map_entries, fixed_map_from_map};

#[test]
fn planner_item_defaults_assignments_and_isolation_match_native_controls() {
    let engine = Engine::builder()
        .register_library(bsl_session::library())
        .build()
        .unwrap();
    let module = engine.compile(r#"
        Первый = Новый ЭлементПланировщика;
        Второй = Новый PlannerItem;
        Проверки = Новый Массив;
        Проверки.Добавить(Первый.Значение = Неопределено);
        Проверки.Добавить(Первый.Начало = Дата(1, 1, 1));
        Проверки.Добавить(Первый.Конец = Дата(1, 1, 1));
        Проверки.Добавить(Первый.Текст = "");
        Проверки.Добавить(Первый.Подсказка = "");
        Проверки.Добавить(Первый.Удаленный = Неопределено);
        Проверки.Добавить(Первый.Родитель = Неопределено);
        Проверки.Добавить(Первый.ЗначенияИзмерений.Количество() = 0);
        Первый.Value = 42;
        Первый.Text = 42;
        Первый.ToolTip = 42;
        Первый.Begin = Дата(2024, 1, 2);
        Первый.Begin = 42;
        Первый.End = Дата(2024, 1, 3);
        Первый.Deleted = Истина;
        Проверки.Добавить(Первый.Значение = 42);
        Проверки.Добавить(Первый.Текст = "42");
        Проверки.Добавить(Первый.Подсказка = "42");
        Проверки.Добавить(Первый.Начало = Дата(1, 1, 1));
        Проверки.Добавить(Первый.Конец = Дата(2024, 1, 3));
        Проверки.Добавить(Первый.Удаленный = Неопределено);
        Проверки.Добавить(Второй.Значение = Неопределено);
        Проверки.Добавить(Второй.Текст = "");
        Проверки.Добавить(Второй.Начало = Дата(1, 1, 1));
        Отказ = Ложь;
        Попытка Первый.Родитель = Неопределено; Исключение Отказ = Истина; КонецПопытки;
        Проверки.Добавить(Отказ);
        Отказ = Ложь;
        Попытка Первый.ЗначенияИзмерений = Новый Соответствие; Исключение Отказ = Истина; КонецПопытки;
        Проверки.Добавить(Отказ);
        Возврат Проверки;
    "#).unwrap();
    let value = engine.new_state().run(&module).unwrap();
    let Value::Object(object) = value else {
        panic!("массив результатов");
    };
    let BslObject::Array(values) = object.as_ref() else {
        panic!("массив результатов");
    };
    assert_eq!(*values.borrow(), vec![Value::Boolean(true); 19]);
    for args in ["1", "1, 2", "1, 2, 3"] {
        assert!(
            engine
                .compile(&format!("Возврат Новый ЭлементПланировщика({args});"))
                .is_err()
        );
    }
}

#[test]
fn facade_fixed_map_factory_preserves_the_existing_immutable_snapshot() {
    let source = Value::new_map();
    let Value::Object(object) = &source else {
        unreachable!()
    };
    let BslObject::Map(entries) = object.as_ref() else {
        unreachable!()
    };
    entries
        .borrow_mut()
        .insert(Value::Str("ключ".into()), Value::number_from_i64(1));
    let fixed = fixed_map_from_map(&source).unwrap();
    entries
        .borrow_mut()
        .insert(Value::Str("ключ".into()), Value::number_from_i64(2));
    assert_eq!(
        fixed_map_entries(&fixed).unwrap(),
        vec![(Value::Str("ключ".into()), Value::number_from_i64(1))]
    );
    assert!(fixed_map_from_map(&Value::Undefined).is_err());
}
