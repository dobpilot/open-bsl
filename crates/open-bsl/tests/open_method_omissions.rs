use open_bsl::{Engine, Error, Value};

#[test]
fn dormant_writer_open_keeps_omissions_for_a_known_core_receiver() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Процедура Начать(Путь) Запись = Новый ЗаписьТекста; Запись.Открыть(Путь,,,Истина); КонецПроцедуры Возврат 17;",
        "Procedure Start(Path) Writer = New TextWriter; Writer.Open(Path,,,True); EndProcedure Return 17;",
    ] {
        assert_eq!(
            engine.new_state().exec(source).unwrap(),
            Value::number_from_i64(17)
        );
    }
}

#[test]
fn an_unknown_method_with_omissions_remains_a_catchable_execution_error() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(
        engine.new_state().exec(
            "Получатель = Новый Структура; Попытка Получатель.НесуществующийМетод(,1); Исключение Возврат Истина; КонецПопытки; Возврат Ложь;"
        ).unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn a_known_closed_method_keeps_its_previous_omission_rejection() {
    let engine = Engine::builder().build().unwrap();
    for source in [
        "Получатель = Новый Массив; Получатель.Найти(,);",
        "Получатель = Новый Массив; Получатель.Установить(,1);",
    ] {
        assert!(matches!(engine.compile(source), Err(Error::Semantic(_))));
    }
}
