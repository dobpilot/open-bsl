use open_bsl::{Engine, Value};

const INITIALIZATION: &str = r#"
Процедура Готово(Результат, Данные) Экспорт
    Если Не Данные.ВозвратБыл Или Результат <> Данные.Файл Тогда
        ВызватьИсключение "порядок или тождество";
    КонецЕсли;
    Данные.События.Добавить(Результат.ПолноеИмя);
КонецПроцедуры
ФайлТеста = Новый Файл("before");
События = Новый Массив;
Данные = Новый Структура("ВозвратБыл,Файл,События", Ложь, ФайлТеста, События);
ФайлТеста.НачатьИнициализацию(Новый ОписаниеОповещения("Готово", ЭтотОбъект, Данные), "after");
Если ФайлТеста.ПолноеИмя <> "after" Или События.Количество() <> 0 Тогда
    ВызватьИсключение "инициализация до возврата";
КонецЕсли;
Данные.ВозвратБыл = Истина;
Возврат События;
"#;

#[test]
fn initialization_delivers_after_the_call_and_survives_root_return() {
    for source in [INITIALIZATION.to_owned(), INITIALIZATION.replace(
        "ФайлТеста.НачатьИнициализацию(Новый ОписаниеОповещения(\"Готово\", ЭтотОбъект, Данные), \"after\");",
        "Выполнить(\"ФайлТеста.BeginInitialization(Новый NotifyDescription(\"\"Готово\"\", ThisObject, Данные), \"\"after\"\");\");",
    )] {
        run_modes(&source, |result| {
            assert_eq!(array(&result), vec![Value::Str("after".into())]);
        });
    }
}

fn array(value: &Value) -> Vec<Value> {
    let Value::Object(object) = value else {
        panic!("ожидается массив")
    };
    let bsl_rt::BslObject::Array(array) = &**object else {
        panic!("ожидается массив")
    };
    array.borrow().clone()
}

fn run_modes(source: &str, check: impl Fn(Value)) {
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let engine = Engine::builder()
            .optimizations(optimizations)
            .build()
            .unwrap();
        for bytecode in [false, true] {
            let module = engine.compile(source).unwrap();
            let module = if bytecode {
                engine.load_bytecode(&module.bytecode().unwrap()).unwrap()
            } else {
                module
            };
            check(engine.new_state().run(&module).unwrap());
        }
    }
}

#[test]
fn description_is_checked_before_mutation_and_empty_description_is_accepted() {
    for name in ["НачатьИнициализацию", "BeginInitialization"] {
        run_modes(
            &format!(
                r#"
ФайлТеста = Новый Файл("before");
Ошибки = 0;
Для Каждого Неверное Из СтрРазделить("Undefined,Null,False,0,"""",""text"",New Array,New Structure", ",") Цикл
    Попытка
        Выполнить("ФайлТеста.{name}(" + Неверное + ", ""after"");");
    Исключение
        Ошибки = Ошибки + 1;
    КонецПопытки;
    Если ФайлТеста.ПолноеИмя <> "before" Тогда ВызватьИсключение "путь изменён"; КонецЕсли;
КонецЦикла;
Если Ошибки <> 8 Тогда ВызватьИсключение "принято неверное описание"; КонецЕсли;
ФайлТеста.{name}(Новый NotifyDescription, "empty");
Если ФайлТеста.ПолноеИмя <> "empty" Тогда ВызватьИсключение "пустое описание"; КонецЕсли;
Результаты = Новый Соответствие;
ФайлТеста.{name}(Новый NotifyDescription("Insert", Результаты, "same"), "after");
Если Результаты.Количество() <> 0 Тогда ВызватьИсключение "синхронная доставка"; КонецЕсли;
Ответ = Новый Массив;
Ответ.Добавить(Результаты);
Ответ.Добавить(ФайлТеста);
Возврат Ответ;
"#
            ),
            |result| {
                let values = array(&result);
                assert_eq!(
                    values[0].map_get(&values[1]).unwrap(),
                    Value::Str("same".into())
                );
            },
        );
    }
}

#[test]
fn bad_setter_value_is_delivered_with_a_mutable_standard_flag_and_original_data() {
    run_modes(
        r#"
Процедура Успех(Данные) Экспорт
    ВызватьИсключение "успех ошибочной операции";
КонецПроцедуры
Процедура Ошибка(Знач Информация, Стандартная, Знач Данные) Экспорт
    Если Не Данные.ВозвратБыл Или Не Стандартная Тогда ВызватьИсключение "порядок ошибки"; КонецЕсли;
    Если ТипЗнч(Информация) <> Тип("ИнформацияОбОшибке") Тогда ВызватьИсключение "тип ошибки"; КонецЕсли;
    Стандартная = Ложь;
    Данные.События.Добавить("error");
КонецПроцедуры
Данные = Новый Структура("ВозвратБыл,События", Ложь, Новый Массив);
Описание = Новый NotifyDescription("Успех", ЭтотОбъект, Данные, "Ошибка", ЭтотОбъект);
ФайлТеста = Новый Файл("unused");
ФайлТеста.BeginSettingHidden(Описание, Новый Массив);
Если Данные.События.Количество() <> 0 Тогда ВызватьИсключение "немедленная ошибка"; КонецЕсли;
Данные.ВозвратБыл = Истина;
Возврат Данные.События;
"#,
        |result| assert_eq!(array(&result), vec![Value::Str("error".into())]),
    );
}

#[test]
fn standard_processing_returns_the_operation_error_not_the_handlers_caught_error() {
    let engine = Engine::builder().build().unwrap();
    let error = engine.new_state().exec(r#"
Процедура Успех(Данные) Экспорт
КонецПроцедуры
Процедура Ошибка(Информация, Стандартная, Данные) Экспорт
    Попытка ВызватьИсключение "другая ошибка"; Исключение КонецПопытки;
КонецПроцедуры
ФайлТеста = Новый Файл("unused");
ФайлТеста.BeginSettingHidden(Новый NotifyDescription("Успех", ЭтотОбъект, Неопределено, "Ошибка", ЭтотОбъект), Новый Массив);
"#).unwrap_err();
    assert!(
        matches!(
            error,
            open_bsl::Error::Runtime(open_bsl::RtError::TypeError { .. })
        ),
        "{error:?}"
    );
}

#[test]
fn a_callback_can_schedule_another_callback_with_the_same_name() {
    run_modes(
        r#"
Процедура Продолжение(Результат, Данные) Экспорт
    Данные.Добавить(Результат.ПолноеИмя);
    Если Данные.Количество() = 1 Тогда
        Результат.BeginInitialization(Новый NotifyDescription("ПРОДОЛЖЕНИЕ", ЭтотОбъект, Данные), "second");
    КонецЕсли;
КонецПроцедуры
Данные = Новый Массив;
ФайлТеста = Новый Файл("initial");
// Имя уже присутствует в таблице открытых вызовов исходной программы.
Если Ложь Тогда ЭтотОбъект.Продолжение(ФайлТеста, Данные); КонецЕсли;
ФайлТеста.BeginInitialization(Новый NotifyDescription("Продолжение", ЭтотОбъект, Данные), "first");
Возврат Данные;
"#,
        |result| {
            assert_eq!(
                array(&result),
                vec![Value::Str("first".into()), Value::Str("second".into())]
            )
        },
    );
}

#[test]
fn standard_flag_matches_native_values_and_preserves_the_original_error() {
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-begin-standard-flags.platform.txt"
    );
    // Независимый замер с ошибкой доступа отличает исходный отказ от
    // потенциально похожей ошибки преобразования стандартного флага.
    let size_oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-begin-size-flags.platform.txt"
    );
    let kind_oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-begin-standard-flag-kinds.platform.txt"
    );
    assert_eq!(
        size_oracle,
        oracle
            .replace("file-begin-standard-flags-1", "file-begin-size-flags-1")
            .replace(
                "Type mismatch (parameter number '1')",
                "Error accessing file: /open-bsl-standard-flags-no-file-20260917"
            )
    );
    let mut observed = std::collections::BTreeMap::new();
    let mut current = "";
    let mut assigned = 0;
    for line in oracle.lines() {
        if let Some(value) = line.strip_prefix("flag.enter\t") {
            (current, _) = value.split_once("|initial=1").unwrap();
            assert!(observed.insert(current, false).is_none());
        } else if let Some(message) = line.strip_prefix("client.error\t") {
            assert_eq!(message, "Type mismatch (parameter number '1')");
            assert!(!std::mem::replace(observed.get_mut(current).unwrap(), true));
        } else if line.starts_with("flag.assigned\t") {
            assigned += 1;
        }
    }
    assert_eq!(observed.len(), 11);
    assert_eq!(assigned, 11);
    assert!(oracle.ends_with("flag.count\t11\nfile.end\tfile-begin-standard-flags-1\n"));
    current = "";
    assigned = 0;
    for line in kind_oracle.lines() {
        if let Some(value) = line.strip_prefix("flagkind.assigned\t") {
            (current, _) = value.split_once("|type=").unwrap();
            assert!(observed.insert(current, false).is_none());
            assigned += 1;
        } else if let Some(message) = line.strip_prefix("client.error\t") {
            assert_eq!(message, "Type mismatch (parameter number '1')");
            assert!(!std::mem::replace(observed.get_mut(current).unwrap(), true));
        }
    }
    assert_eq!(assigned, 11);
    assert_eq!(observed.len(), 22);
    assert!(
        kind_oracle.ends_with("flagkind.count\t11\nfile.end\tfile-begin-standard-flag-kinds-1\n")
    );
    for (key, expression) in [
        ("false", "Ложь"),
        ("true", "Истина"),
        ("zero", "0"),
        ("one", "1"),
        ("negative", "-1"),
        ("undefined", "Неопределено"),
        ("null", "Null"),
        ("empty", "\"\""),
        ("string", "\"False\""),
        ("array", "Новый Массив"),
        ("structure", "Новый Структура"),
        ("date", "Дата(2020, 1, 2)"),
        ("type", "Тип(\"Строка\")"),
        ("enum", "КодировкаТекста.UTF8"),
        ("map", "Новый Соответствие"),
        ("file", "Новый Файл(\"flag-kind.txt\")"),
        (
            "binary",
            "ПолучитьДвоичныеДанныеИзСтроки(\"AB\", КодировкаТекста.UTF8, Ложь)",
        ),
        ("buffer", "Новый БуферДвоичныхДанных(2)"),
        (
            "uuid",
            "Новый УникальныйИдентификатор(\"01234567-89ab-cdef-0123-456789abcdef\")",
        ),
        ("type_description", "Новый ОписаниеТипов(\"Строка\")"),
        ("notification", "Новый ОписаниеОповещения"),
        ("module", "ЭтотОбъект"),
    ] {
        for method in ["НачатьУстановкуНевидимости", "BeginSettingHidden"]
        {
            let call = format!(
                "ФайлТеста.{method}(Новый NotifyDescription(\"Успех\", ЭтотОбъект, Неопределено, \"Ошибка\", ЭтотОбъект), Новый Массив);"
            );
            for dynamic in [false, true] {
                let call = if dynamic {
                    format!("Выполнить(\"{}\");", call.replace('"', "\"\""))
                } else {
                    call.clone()
                };
                let source = format!(
                    r#"
Процедура Успех(Данные) Экспорт
    ВызватьИсключение "ошибка была потеряна";
КонецПроцедуры
Процедура Ошибка(Информация, Стандартная, Данные) Экспорт
    Стандартная = {expression};
КонецПроцедуры
ФайлТеста = Новый Файл("unused");
{call}
Возврат 17;
"#
                );
                for optimizations in [
                    bsl_compiler::Optimizations::default(),
                    bsl_compiler::Optimizations::all(),
                ] {
                    let engine = Engine::builder()
                        .optimizations(optimizations)
                        .build()
                        .unwrap();
                    let module = engine.compile(&source).unwrap();
                    let loaded = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();
                    for module in [&module, &loaded] {
                        let result = engine.new_state().run(module);
                        if observed[key] {
                            assert!(
                                matches!(
                                    result,
                                    Err(open_bsl::Error::Runtime(open_bsl::RtError::TypeError {
                                        op: "Условие",
                                        ..
                                    }))
                                ),
                                "{key}, {method}, dynamic={dynamic}: {result:?}"
                            );
                        } else {
                            assert!(
                                result.is_ok(),
                                "{key}, {method}, dynamic={dynamic}: {result:?}"
                            );
                            assert_eq!(
                                result.unwrap(),
                                Value::Number(open_bsl::BslNumber::from_i64(17)),
                                "{key}, {method}, dynamic={dynamic}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn callback_exceptions_follow_the_measured_handler_kind() {
    #[derive(Clone, Default)]
    struct Output(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-begin-handler-errors.platform.txt"
    );
    assert!(oracle.ends_with("callback.count\t6\nfile.end\tfile-begin-handler-errors-1\n"));
    assert!(!oracle.contains("callback.rerouted"));
    assert_eq!(
        oracle
            .lines()
            .filter(|line| line.starts_with("client.error\t"))
            .count(),
        4
    );
    assert_eq!(
        oracle
            .lines()
            .filter(|line| line.starts_with("callback.enter\t"))
            .count(),
        6
    );
    assert_eq!(
        oracle
            .lines()
            .filter(|line| line.starts_with("callback.await\t"))
            .count(),
        2
    );
    let definitions =
        include_str!("../../../tests/conformance/measure/filesystem/file-begin-handler-errors.bsl")
            .split("Сообщить(\"file.begin\"")
            .next()
            .unwrap();
    for (key, handler) in [
        ("procedure", "ОшибкаСинхПроцедуры"),
        ("function", "ОшибкаСинхФункции"),
        ("async.procedure.before", "ОшибкаАсинхПроцедурыДо"),
        ("async.function.before", "ОшибкаАсинхФункцииДо"),
        ("async.procedure.after", "ОшибкаАсинхПроцедурыПосле"),
        ("async.function.after", "ОшибкаАсинхФункцииПосле"),
    ] {
        let message = format!("handler-error:{key}");
        let error_expected = oracle
            .lines()
            .any(|line| line == format!("client.error\t{message}"));
        let call = format!(
            "ФайлОшибочныхОбработчиков.BeginInitialization(Новый NotifyDescription(\"{handler}\", ЭтотОбъект, \"{key}\", \"НеожиданнаяОшибкаОперации\", ЭтотОбъект), \"/open-bsl-handler-errors-no-file-20260917\");"
        );
        for dynamic in [false, true] {
            let call = if dynamic {
                format!("Выполнить(\"{}\");", call.replace('"', "\"\""))
            } else {
                call.clone()
            };
            let source = format!(
                r#"{definitions}
ЧислоОшибочныхОбработчиков = 0;
ФайлОшибочныхОбработчиков = Новый Файл("unused");
{call}
Возврат 17;
"#
            );
            for optimizations in [
                bsl_compiler::Optimizations::default(),
                bsl_compiler::Optimizations::all(),
            ] {
                let engine = Engine::builder()
                    .optimizations(optimizations)
                    .build()
                    .unwrap();
                let module = engine.compile(&source).unwrap();
                let loaded = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();
                for module in [&module, &loaded] {
                    let output = Output::default();
                    let result = engine
                        .state_builder()
                        .stdout(output.clone())
                        .build()
                        .run(module);
                    let output = String::from_utf8(output.0.borrow().clone()).unwrap();
                    assert_eq!(
                        output
                            .lines()
                            .filter(|line| line.starts_with("callback.enter\t"))
                            .count(),
                        1
                    );
                    assert!(output.contains(&format!("callback.enter\t{key}\n")));
                    assert!(!output.contains("callback.rerouted"));
                    if key.ends_with(".after") {
                        assert!(output.contains(&format!("callback.await\t{key}|0\n")));
                    }
                    if error_expected {
                        assert!(
                            matches!(result, Err(open_bsl::Error::Runtime(open_bsl::RtError::Raised(Value::Str(ref text)))) if text.to_string() == message),
                            "{key}, dynamic={dynamic}: {result:?}"
                        );
                    } else {
                        assert_eq!(
                            result.unwrap(),
                            Value::Number(open_bsl::BslNumber::from_i64(17))
                        );
                    }
                }
            }
        }
    }
}
