use std::cell::Cell;
use std::rc::Rc;

use open_bsl::{Engine, Error, RtError, Value};

use crate::{global_tests, host_data_tests};

#[test]
fn initialization_runs_once_and_calls_preserve_module_variables() {
    let engine = Engine::builder()
        .register_library(global_tests::library())
        .build()
        .unwrap();
    let reads = Rc::new(Cell::new(0_i64));
    let module = engine
        .compile(
            r#"
            Перем Счётчик;
            ПолучитьГлобал();
            Счётчик = 10;
            Возврат 999;

            Процедура Увеличить(Шаг, Знач Копия) Экспорт
                Счётчик = Счётчик + Шаг;
                Шаг = Шаг + 1;
                Копия = Копия + 2;
            КонецПроцедуры

            Функция Прочитать() Экспорт
                Возврат Счётчик;
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = engine.state_builder().host_data(reads.clone()).build();
    let instance = state.instantiate(&module).unwrap();
    let same = state.instantiate(&module.clone()).unwrap();
    assert_eq!(reads.get(), 1, "тело модуля исполняется ровно один раз");

    let (value, parameters) = instance
        .call(
            &mut state,
            "уВеЛиЧиТь",
            vec![Value::number_from_i64(3), Value::number_from_i64(4)],
        )
        .unwrap();
    assert_eq!(value, Value::Undefined);
    assert_eq!(
        parameters,
        vec![Value::number_from_i64(4), Value::number_from_i64(6)]
    );
    assert_eq!(
        same.call(&mut state, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(13)
    );

    // Обычный `run` остаётся отдельным запуском и снова исполняет тело.
    assert_eq!(state.run(&module).unwrap(), Value::number_from_i64(999));
    assert_eq!(reads.get(), 2);
    assert_eq!(
        instance.call(&mut state, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(13)
    );
}

#[test]
fn lookup_export_and_arity_are_checked_before_execution() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
            Перем Счётчик;
            Счётчик = 0;
            Процедура Скрытая() Счётчик = 100; КонецПроцедуры
            Процедура Экспортная(Аргумент) Экспорт Счётчик = Счётчик + 1; КонецПроцедуры
            Функция Прочитать() Экспорт Возврат Счётчик; КонецФункции
            "#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();

    for (name, args) in [
        ("Отсутствует", vec![]),
        ("Скрытая", vec![]),
        ("Экспортная", vec![]),
        ("Экспортная", vec![Value::Undefined, Value::Undefined]),
    ] {
        assert!(matches!(
            instance.call(&mut state, name, args),
            Err(Error::Runtime(RtError::DynamicError(_)))
        ));
    }
    assert_eq!(
        instance.call(&mut state, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(0),
        "ошибочные вызовы не исполняют тело цели"
    );
}

#[test]
fn an_exception_keeps_completed_writes_and_the_instance_remains_callable() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
            Перем Счётчик;
            Счётчик = 0;
            Процедура ИзменитьИУпасть() Экспорт
                Счётчик = Счётчик + 1;
                ВызватьИсключение "ожидаемая ошибка";
            КонецПроцедуры
            Функция Прочитать() Экспорт Возврат Счётчик; КонецФункции
            "#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    assert!(
        instance
            .call(&mut state, "ИзменитьИУпасть", vec![])
            .is_err()
    );
    assert_eq!(
        instance.call(&mut state, "Прочитать", vec![]).unwrap().0,
        Value::number_from_i64(1)
    );
}

#[test]
fn failed_initialization_is_not_published_and_can_be_retried() {
    let engine = Engine::builder()
        .register_library(global_tests::library())
        .build()
        .unwrap();
    let reads = Rc::new(Cell::new(0_i64));
    let module = engine
        .compile("ПолучитьГлобал(); ВызватьИсключение \"ошибка инициализации\";")
        .unwrap();
    let mut state = engine.state_builder().host_data(reads.clone()).build();
    assert!(state.instantiate(&module).is_err());
    assert!(state.instantiate(&module).is_err());
    assert_eq!(reads.get(), 2);
}

#[test]
fn common_modules_are_shared_inside_a_state_and_isolated_between_states() {
    let engine = Engine::builder()
        .common_module(
            "ОбщееСостояние",
            r#"
            Перем Значение;
            Значение = 0;
            Процедура Записать(Новое) Экспорт Значение = Новое; КонецПроцедуры
            Функция Прочитать() Экспорт Возврат Значение; КонецФункции
            "#,
        )
        .build()
        .unwrap();
    let writer = engine
        .compile_entry("Процедура Записать() Экспорт ОбщееСостояние.Записать(42); КонецПроцедуры")
        .unwrap();
    let reader = engine
        .compile_entry(
            "Функция Прочитать() Экспорт Возврат ОбщееСостояние.Прочитать(); КонецФункции",
        )
        .unwrap();

    let mut first = engine.new_state();
    let first_writer = first.instantiate(&writer).unwrap();
    let first_reader = first.instantiate(&reader).unwrap();
    first_writer.call(&mut first, "Записать", vec![]).unwrap();
    assert_eq!(
        first_reader
            .call(&mut first, "Прочитать", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(42)
    );

    let mut second = engine.new_state();
    let second_reader = second.instantiate(&reader).unwrap();
    assert_eq!(
        second_reader
            .call(&mut second, "Прочитать", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(0)
    );
    assert!(matches!(
        first_reader.call(&mut second, "Прочитать", vec![]),
        Err(Error::Configuration(_))
    ));
}

#[test]
fn only_the_compiling_engine_can_instantiate_a_module() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile("Функция Ответ() Экспорт Возврат 42; КонецФункции")
        .unwrap();
    let mut clone_state = engine.clone().new_state();
    assert!(clone_state.instantiate(&module).is_ok());

    let foreign = Engine::builder().build().unwrap();
    assert!(matches!(
        foreign.new_state().instantiate(&module),
        Err(Error::Configuration(_))
    ));
}

#[test]
fn exports_keep_dynamic_code_and_async_execution_services() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
            Перем Счётчик;
            Счётчик = 1;
            Процедура Динамически() Экспорт
                Выполнить("Счётчик = Счётчик + 40");
            КонецПроцедуры
            Асинх Функция Внутренняя()
                Возврат Счётчик + 1;
            КонецФункции
            Асинх Функция Асинхронно() Экспорт
                Возврат Ждать Внутренняя();
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    instance.call(&mut state, "Динамически", vec![]).unwrap();
    assert_eq!(
        instance.call(&mut state, "Асинхронно", vec![]).unwrap().0,
        Value::number_from_i64(42)
    );
}

#[test]
fn module_identity_survives_separate_export_executions() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(
            r#"
            Функция Ссылка() Экспорт Возврат ЭтотОбъект; КонецФункции
            Функция Ответ() Экспорт Возврат 42; КонецФункции
            Функция ЧерезСсылку(Получатель) Экспорт
                Возврат Получатель.Ответ();
            КонецФункции
            "#,
        )
        .unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let first = instance.call(&mut state, "Ссылка", vec![]).unwrap().0;
    let second = instance.call(&mut state, "Ссылка", vec![]).unwrap().0;
    assert_eq!(first, second);
    assert_eq!(
        instance
            .call(&mut state, "ЧерезСсылку", vec![first])
            .unwrap()
            .0,
        Value::number_from_i64(42)
    );
}

#[test]
fn facade_only_flow_combines_global_host_data_and_persistent_calls() {
    let engine = Engine::builder()
        .register_library(global_tests::library())
        .register_library(host_data_tests::library())
        .build()
        .unwrap();
    let module = engine
        .compile(
            r#"
            Процедура УстановитьПараметрыСеанса() Экспорт
                Значение = Справочники.Ответ;
            КонецПроцедуры
            Функция ПрочитатьПараметрСеанса() Экспорт
                Возврат ДанныеХоста();
            КонецФункции
            "#,
        )
        .unwrap();
    let data = Rc::new(Cell::new(0_i64));
    let mut first = engine.state_builder().host_data(data.clone()).build();
    let instance = first.instantiate(&module).unwrap();
    instance
        .call(&mut first, "УстановитьПараметрыСеанса", vec![])
        .unwrap();
    assert_eq!(data.get(), 1);
    assert_eq!(
        instance
            .call(&mut first, "ПрочитатьПараметрСеанса", vec![])
            .unwrap()
            .0,
        Value::number_from_i64(1)
    );

    let mut second = engine.new_state();
    let second_instance = second.instantiate(&module).unwrap();
    assert!(matches!(
        second_instance.call(&mut second, "ПрочитатьПараметрСеанса", vec![]),
        Err(Error::Runtime(RtError::CapabilityMissing { .. }))
    ));
}
