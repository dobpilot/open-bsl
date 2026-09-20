//! Первая ошибка доставки сверяется с клиентским наблюдателем 1С.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use open_bsl::{Engine, Error, RtError, Value};

#[derive(Clone, Default)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn error_handler_kinds_preserve_the_measured_first_error() {
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-begin-error-handlers.platform.txt"
    );
    assert!(oracle.ends_with("errorhandler.count\t12\nfile.end\tfile-begin-error-handlers-1\n"));
    assert!(!oracle.contains("errorhandler.unexpected_success"));
    assert_eq!(
        oracle
            .lines()
            .filter(|row| row.starts_with("errorhandler.enter\t"))
            .count(),
        12
    );
    assert_eq!(
        oracle
            .lines()
            .filter(|row| row.starts_with("client.error\t"))
            .count(),
        12
    );
    assert_eq!(
        oracle
            .lines()
            .filter(|row| row.starts_with("errorhandler.after\t"))
            .count(),
        6
    );
    let definitions =
        include_str!("../../../tests/conformance/measure/filesystem/file-begin-error-handlers.bsl")
            .split("Сообщить(\"file.begin\"")
            .next()
            .unwrap();
    for (key, handler, before, throws) in [
        ("procedure.suppress", "СинхПроцедураОшибки", true, false),
        ("function.suppress", "СинхФункцияОшибки", true, false),
        ("procedure.throw", "СинхПроцедураОшибки", true, true),
        ("function.throw", "СинхФункцияОшибки", true, true),
        (
            "async.procedure.suppress.before",
            "АсинхПроцедураОшибки",
            true,
            false,
        ),
        (
            "async.function.suppress.before",
            "АсинхФункцияОшибки",
            true,
            false,
        ),
        (
            "async.procedure.suppress.after",
            "АсинхПроцедураОшибки",
            false,
            false,
        ),
        (
            "async.function.suppress.after",
            "АсинхФункцияОшибки",
            false,
            false,
        ),
        (
            "async.procedure.throw.before",
            "АсинхПроцедураОшибки",
            true,
            true,
        ),
        (
            "async.function.throw.before",
            "АсинхФункцияОшибки",
            true,
            true,
        ),
        (
            "async.procedure.throw.after",
            "АсинхПроцедураОшибки",
            false,
            true,
        ),
        (
            "async.function.throw.after",
            "АсинхФункцияОшибки",
            false,
            true,
        ),
    ] {
        let original_error =
            format!("Error accessing file: /open-bsl-error-handler-20260917-{key}");
        let handler_error = format!("error-handler:{key}");
        let errors: Vec<_> = oracle
            .lines()
            .filter_map(|row| row.strip_prefix("client.error\t"))
            .filter(|error| *error == original_error || *error == handler_error)
            .collect();
        for english in [false, true] {
            let definitions = if english {
                definitions.replace("НачатьПолучениеРазмера", "BeginGettingSize")
            } else {
                definitions.to_owned()
            };
            for dynamic in [false, true] {
                let call = format!(
                    "ЗапуститьОбработчикОшибки(\"{key}\", \"{handler}\", {}, {});",
                    if before { "Истина" } else { "Ложь" },
                    if throws { "Истина" } else { "Ложь" }
                );
                let call = if dynamic {
                    format!("Выполнить(\"{}\");", call.replace('"', "\"\""))
                } else {
                    call
                };
                let source =
                    format!("{definitions}\nЧислоОбработчиковОшибки = 0;\n{call}\nВозврат 17;");
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
                        assert!(
                            output.contains(&format!(
                                "errorhandler.enter\t{key}|initial=1|type=Error information\n"
                            )),
                            "{key}: {output}"
                        );
                        assert!(!output.contains("errorhandler.unexpected_success"));
                        match errors.first().copied() {
                            None => assert_eq!(
                                result.unwrap(),
                                Value::Number(open_bsl::BslNumber::from_i64(17))
                            ),
                            Some(error) if error == original_error => assert!(
                                matches!(result, Err(Error::Runtime(RtError::IoError(_)))),
                                "{key}, english={english}, dynamic={dynamic}: {result:?}"
                            ),
                            Some(_) => assert!(
                                matches!(result, Err(Error::Runtime(RtError::Raised(Value::Str(ref text)))) if text.to_string() == handler_error),
                                "{key}: {result:?}"
                            ),
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn detached_errors_keep_the_native_queue_prefix_before_the_first_report() {
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/async-procedure-error-order.platform.txt"
    );
    assert_eq!(
        oracle
            .lines()
            .filter(|row| row.starts_with("client.error\t"))
            .count(),
        3
    );
    assert!(!oracle.contains("order.caught"));
    let (prefix, rest) = oracle
        .split_once("client.error\tasync-order:first\n")
        .unwrap();
    assert!(rest.contains("order.notification\tafter\n"));
    assert!(rest.contains("order.after_await\t0\n"));
    let script = include_str!(
        "../../../tests/conformance/measure/filesystem/async-procedure-error-order.bsl"
    );
    for dynamic in [false, true] {
        let source = if dynamic {
            script
                .replace(
                    "НемедленнаяОшибкаОчереди(\"first\");",
                    "Выполнить(\"НемедленнаяОшибкаОчереди(\"\"first\"\");\");",
                )
                .replace(
                    "НемедленнаяОшибкаОчереди(\"second\");",
                    "Выполнить(\"НемедленнаяОшибкаОчереди(\"\"second\"\");\");",
                )
        } else {
            script.to_owned()
        };
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
                assert!(
                    matches!(result, Err(Error::Runtime(RtError::Raised(Value::Str(ref text)))) if text.to_string() == "async-order:first"),
                    "{result:?}"
                );
                let output = String::from_utf8(output.0.borrow().clone()).unwrap();
                assert_eq!(output, prefix, "dynamic={dynamic}");
            }
        }
    }
}
