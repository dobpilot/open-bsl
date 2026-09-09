//! Регрессии BSL-ожидания по серверным замерам `job-wait-2026-09-08`.

#![cfg(not(target_arch = "wasm32"))]

use open_bsl::{Engine, format_value};

#[test]
fn nested_manager_wait_returns_the_first_terminal_job_and_can_be_repeated() {
    for workers in [1, 3] {
        let engine = Engine::builder()
            .background_jobs(open_bsl::jobs::BackgroundJobConfig {
                workers: Some(workers),
                ..Default::default()
            })
            .common_module(
                "Работы",
                r#"
Процедура Короткая() Экспорт
КонецПроцедуры
Процедура Длинная() Экспорт
    ПределПробы = ТекущаяУниверсальнаяДатаВМиллисекундах() + 5000;
    Пока ТекущаяУниверсальнаяДатаВМиллисекундах() < ПределПробы Цикл КонецЦикла;
КонецПроцедуры
Процедура Родитель() Экспорт
    Первое = ФоновыеЗадания.Выполнить("Работы.Короткая");
    Второе = ФоновыеЗадания.Выполнить("Работы.Длинная");
    Группа = Новый Массив;
    Группа.Добавить(Первое);
    Группа.Добавить(Второе);
    Ответ = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Группа);
    Повтор = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Ответ);
    Верно = Ответ[0].Состояние = СостояниеФоновогоЗадания.Завершено
        И Повтор[0].Состояние = СостояниеФоновогоЗадания.Завершено
        И Ответ[1].Состояние = СостояниеФоновогоЗадания.Активно
        И Повтор[1].Состояние = СостояниеФоновогоЗадания.Активно;
    Второе.Отменить();
    Второе.ОжидатьЗавершенияВыполнения();
    Сообщить(Верно);
КонецПроцедуры
"#,
            )
            .build()
            .unwrap();
        let module = engine
            .compile_entry(
                r#"
Задание = ФоновыеЗадания.Выполнить("Работы.Родитель");
Задание = Задание.ОжидатьЗавершенияВыполнения();
Если Задание.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда
    ВызватьИсключение "родитель не завершён";
КонецЕсли;
Возврат Задание.ПолучитьСообщенияПользователю()[0].Текст;
"#,
            )
            .unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert_eq!(
            format_value(&value, None).unwrap(),
            "Да",
            "workers={workers}"
        );
    }
}

#[test]
fn fresh_and_stale_terminal_snapshots_and_repeated_waits_follow_the_server_contract() {
    let engine = Engine::builder()
        .common_module(
            "Работы",
            r#"
Процедура Успех() Экспорт
КонецПроцедуры
Процедура Ошибка() Экспорт
    ВызватьИсключение "probe";
КонецПроцедуры
Процедура Длинная() Экспорт
    ПределПробы = ТекущаяУниверсальнаяДатаВМиллисекундах() + 5000;
    Пока ТекущаяУниверсальнаяДатаВМиллисекундах() < ПределПробы Цикл КонецЦикла;
КонецПроцедуры
"#,
        )
        .build()
        .unwrap();
    for (method, cancel) in [
        ("Успех", ""),
        ("Ошибка", ""),
        ("Длинная", "Первое.Отменить();"),
    ] {
        for fresh in [false, true] {
            for timeout in ["Неопределено", "0.01"] {
                let replace = if fresh {
                    "Первое = Готовое;"
                } else {
                    ""
                };
                let source = format!(
                    r#"
Первое = ФоновыеЗадания.Выполнить("Работы.{method}");
{cancel}
Готовое = Первое.ОжидатьЗавершенияВыполнения();
{replace}
Второе = ФоновыеЗадания.Выполнить("Работы.Длинная");
Группа = Новый Массив;
Группа.Добавить(Первое);
Группа.Добавить(Второе);
Ответ = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Группа, {timeout});
Повтор = ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Ответ, {timeout});
Верно = Ответ.Количество() = 2 И Повтор.Количество() = 2
    И Ответ[0].Состояние = Готовое.Состояние
    И Повтор[0].Состояние = Готовое.Состояние
    И Ответ[1].Состояние = СостояниеФоновогоЗадания.Активно
    И Повтор[1].Состояние = СостояниеФоновогоЗадания.Активно;
Второе.Отменить();
Второе.ОжидатьЗавершенияВыполнения();
Возврат Верно;
"#
                );
                let module = engine.compile_entry(&source).unwrap();
                let value = engine.new_state().run(&module).unwrap();
                assert_eq!(
                    format_value(&value, None).unwrap(),
                    "Да",
                    "{method}, fresh={fresh}, timeout={timeout}"
                );
            }
        }
    }
}

#[test]
fn extreme_bsl_wait_timeouts_are_catchable_in_foreground_and_worker() {
    let mut checks = String::new();
    for limit in ["1000000000000000000", "-1000000000000000000"] {
        for call in [
            format!("Задание.ОжидатьЗавершенияВыполнения({limit})"),
            format!("Задание.ОжидатьЗавершения({limit})"),
            format!("ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Группа, {limit})"),
            format!("ФоновыеЗадания.ОжидатьЗавершения(Группа, {limit})"),
        ] {
            checks.push_str(&format!(
                "Попытка\n{call};\nИсключение\nОшибок = Ошибок + 1;\nКонецПопытки;\n"
            ));
        }
    }
    let source = format!(
        r#"
Процедура Пустая() Экспорт
КонецПроцедуры
Функция Проверить() Экспорт
    Задание = ФоновыеЗадания.Выполнить("Пробы.Пустая");
    Задание.ОжидатьЗавершенияВыполнения();
    Группа = Новый Массив;
    Группа.Добавить(Задание);
    Ошибок = 0;
    {checks}
    Возврат Ошибок;
КонецФункции
Процедура Worker() Экспорт
    Сообщить(Проверить());
КонецПроцедуры
"#
    );
    let engine = Engine::builder()
        .common_module("Пробы", &source)
        .build()
        .unwrap();
    for entry in [
        "Возврат Пробы.Проверить();",
        r#"Задание = ФоновыеЗадания.Выполнить("Пробы.Worker");
        Задание = Задание.ОжидатьЗавершенияВыполнения();
        Если Задание.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда
            ВызватьИсключение "worker failed";
        КонецЕсли;
        Возврат Задание.ПолучитьСообщенияПользователю()[0].Текст;"#,
    ] {
        let module = engine.compile_entry(entry).unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert_eq!(format_value(&value, None).unwrap(), "8");
    }
}

#[test]
fn canceling_the_child_returns_its_canceled_snapshot_to_the_parent() {
    let engine = Engine::builder()
        .common_module(
            "ЗамерУточнений",
            include_str!(
                "../../../docs/research/compatibility/job-wait-2026-09-08/followup-worker.bsl"
            ),
        )
        .build()
        .unwrap();
    for manager in ["Ложь", "Истина"] {
        let source = format!(
            r#"
АргументыПробы = Новый Массив;
АргументыПробы.Добавить({manager});
ЗаданиеПробы = ФоновыеЗадания.Выполнить("ЗамерУточнений.Вложенное", АргументыПробы);
ЗаданиеПробы = ЗаданиеПробы.ОжидатьЗавершенияВыполнения();
Если ЗаданиеПробы.Состояние <> СостояниеФоновогоЗадания.Завершено Тогда
    ВызватьИсключение ЗаданиеПробы.ИнформацияОбОшибке.Описание;
КонецЕсли;
СообщенияПробы = ЗаданиеПробы.ПолучитьСообщенияПользователю();
Возврат СообщенияПробы[0].Текст;
"#
        );
        let module = engine.compile_entry(&source).unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert_eq!(format_value(&value, None).unwrap(), "canceled=Да");
    }
}

#[test]
fn legacy_wait_rejects_timeout_failure_and_cancellation() {
    let engine = Engine::builder()
        .common_module(
            "Работы",
            r#"
Процедура Вечно() Экспорт
    Пока Истина Цикл КонецЦикла;
КонецПроцедуры
Процедура Ошибка() Экспорт
    ВызватьИсключение "probe";
КонецПроцедуры
Процедура Пустая() Экспорт
КонецПроцедуры
"#,
        )
        .build()
        .unwrap();
    for (method, setup, expected) in [
        ("Вечно", "", "ошибка"),
        ("Ошибка", "Задание.ОжидатьЗавершенияВыполнения();", "ошибка"),
        (
            "Вечно",
            "Задание.Отменить(); Задание.ОжидатьЗавершенияВыполнения();",
            "ошибка",
        ),
        ("Пустая", "Задание.ОжидатьЗавершенияВыполнения();", "ok"),
    ] {
        for call in [
            "Задание.ОжидатьЗавершения(0)",
            "ФоновыеЗадания.ОжидатьЗавершения(Группа, 0)",
        ] {
            let source = format!(
                r#"
Задание = ФоновыеЗадания.Выполнить("Работы.{method}");
{setup}
Группа = Новый Массив;
Группа.Добавить(Задание);
Попытка
    {call};
    Итог = "ok";
Исключение
    Итог = "ошибка";
КонецПопытки;
Задание.Отменить();
Задание.ОжидатьЗавершенияВыполнения();
Возврат Итог;
"#
            );
            let module = engine.compile_entry(&source).unwrap();
            let value = engine.new_state().run(&module).unwrap();
            assert_eq!(
                format_value(&value, None).unwrap(),
                expected,
                "{method}: {call}"
            );
        }
    }
}

#[test]
fn legacy_wait_is_a_procedure_even_in_nested_and_dynamic_expressions() {
    let engine = Engine::builder()
        .common_module("Работы", "Процедура Пустая() Экспорт\nКонецПроцедуры")
        .build()
        .unwrap();
    for code in [
        "Результат = Задание.ОжидатьЗавершения();",
        "Результат = ФоновыеЗадания.ОжидатьЗавершения(Группа);",
        "Группа.Добавить(Задание.ОжидатьЗавершения());",
        "Результат = Вычислить(\"Задание.ОжидатьЗавершения()\");",
        "Результат = ФоновыеЗадания.ОжидатьЗавершения();",
    ] {
        let source = format!(
            r#"
Задание = ФоновыеЗадания.Выполнить("Работы.Пустая");
Задание.ОжидатьЗавершенияВыполнения();
Группа = Новый Массив;
Попытка
    {code}
    Возврат "accepted";
Исключение
    Возврат ИнформацияОбОшибке().Описание;
КонецПопытки;
"#
        );
        let module = engine.compile_entry(&source).unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert!(
            format_value(&value, None)
                .unwrap()
                .contains("Вызов процедуры объекта как функции"),
            "{code}: {value:?}"
        );
    }
}

#[test]
fn manager_wait_requires_an_array_through_the_bsl_surface() {
    let engine = Engine::builder()
        .common_module("Работы", "Процедура Пустая() Экспорт\nКонецПроцедуры")
        .build()
        .unwrap();
    for (argument, expected) in [
        ("", "ошибка"),
        ("Задание", "ошибка"),
        ("Неопределено", "ошибка"),
        ("1", "ошибка"),
        ("Новый Структура", "ошибка"),
        ("Новый Массив", "0"),
    ] {
        let source = format!(
            r#"
Задание = ФоновыеЗадания.Выполнить("Работы.Пустая");
Задание.ОжидатьЗавершенияВыполнения();
Попытка
    Результат = ФоновыеЗадания.ОжидатьЗавершенияВыполнения({argument});
    Возврат Строка(Результат.Количество());
Исключение
    Возврат "ошибка";
КонецПопытки;
"#
        );
        let module = engine.compile_entry(&source).unwrap();
        let value = engine.new_state().run(&module).unwrap();
        assert_eq!(format_value(&value, None).unwrap(), expected, "{argument}");
    }
}

#[test]
fn canceling_a_waiting_parent_does_not_cancel_its_child() {
    use open_bsl::{GraphLimits, JobStateDto, RuntimeShapes, SerializedValueGraph};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    for (workers, wait) in [1, 3].into_iter().flat_map(|workers| {
        [
            "Ребёнок.ОжидатьЗавершенияВыполнения()",
            "ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Группа)",
            "Ребёнок.ОжидатьЗавершенияВыполнения(60)",
            "ФоновыеЗадания.ОжидатьЗавершенияВыполнения(Группа, 60)",
        ]
        .map(|wait| (workers, wait))
    }) {
        let source = format!(
            r#"
Процедура Ребёнок() Экспорт
    Сообщить("child_started");
    ПределПробы = ТекущаяУниверсальнаяДатаВМиллисекундах() + 5000;
    Пока ТекущаяУниверсальнаяДатаВМиллисекундах() < ПределПробы Цикл КонецЦикла;
    Сообщить("child_finished");
КонецПроцедуры
Процедура Родитель() Экспорт
    Ребёнок = ФоновыеЗадания.Выполнить("Работы.Ребёнок");
    Группа = Новый Массив;
    Группа.Добавить(Ребёнок);
    Попытка
        {wait};
        Сообщить("after_wait");
    Исключение
        Сообщить("caught");
    КонецПопытки;
КонецПроцедуры
"#
        );
        let engine = Engine::builder()
            .background_jobs(open_bsl::jobs::BackgroundJobConfig {
                workers: Some(workers),
                ..Default::default()
            })
            .common_module("Работы", &source)
            .build()
            .unwrap();
        let runtime = engine.job_runtime().unwrap();
        let params = Arc::new(
            SerializedValueGraph::capture(
                &[],
                &RuntimeShapes::seeded(Vec::new(), Vec::new(), None),
                &GraphLimits::default(),
            )
            .unwrap(),
        );
        let parent = runtime
            .submit_by_name("Работы.Родитель", params, None, None)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        // При одном worker начало ребёнка доказывает вход родителя в helping.
        let child = loop {
            if let Some(child) = runtime.snapshots().into_iter().find(|job| {
                job.method_name == "Работы.Ребёнок"
                    && !runtime.take_messages(job.id, false).unwrap().is_empty()
            }) {
                break child;
            }
            assert!(Instant::now() < deadline, "ребёнок не запущен");
            std::thread::sleep(Duration::from_millis(1));
        };
        runtime.cancel(parent.id).unwrap();
        let canceled_before_child = runtime
            .wait_terminal(&[parent.id], Some(Duration::from_secs(2)))
            .unwrap();
        let child_state = runtime.snapshot(child.id).unwrap().state;
        // Ребёнок сам завершает работу: отмена родителя не должна оставлять
        // его без обслуживающего worker. Пять секунд также ограничивают
        // ошибочный старый путь, который продолжал ждать ребёнка.
        assert!(
            runtime
                .wait_terminal(&[parent.id, child.id], Some(Duration::from_secs(10)))
                .unwrap()
        );
        assert!(
            canceled_before_child,
            "родитель продолжал ждать ребёнка: {wait}"
        );
        assert!(!child_state.is_terminal(), "каскадная отмена ребёнка");
        assert_eq!(
            runtime.snapshot(child.id).unwrap().state,
            JobStateDto::Completed
        );
        assert_eq!(
            runtime.snapshot(parent.id).unwrap().state,
            JobStateDto::Canceled
        );
        assert!(runtime.take_messages(parent.id, false).unwrap().is_empty());
    }
}
