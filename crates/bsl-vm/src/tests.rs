//! Тесты цикла диспетчеризации: опкоды, кадры, компоненты, динамика.

use super::*;
use bsl_compiler::compile_program;
use bsl_number::BslNumber;
use bsl_sema::resolve_program;
use bsl_syntax::parse;

/// Компилятор фрагментов `Выполнить`/`Вычислить` для тестов VM.
///
/// Настоящий живёт в фасаде (`open_bsl::DynamicCode`) вместе с кэшем; VM о
/// нём не знает и знать не должна. Здесь — тот же фронтенд без кэша: тестам
/// нужен факт компиляции, а не её скорость.
pub(crate) struct TestDynamic<'a> {
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
    /// Номера областей раздаёт хост — здесь их роль играет этот счётчик.
    /// Кэша у тестового компилятора нет: тестам нужен факт компиляции, а
    /// не её скорость, и устойчивость ключа проверяется там, где кэш и
    /// живёт, — в фасаде.
    scopes: u64,
}

impl<'a> TestDynamic<'a> {
    pub(crate) fn bare() -> Self {
        TestDynamic {
            registry: None,
            scopes: bsl_bytecode::DynamicScope::ROOT,
        }
    }

    fn with_registry(registry: &'a bsl_rt::RuntimeRegistry) -> Self {
        TestDynamic {
            registry: Some(registry),
            scopes: bsl_bytecode::DynamicScope::ROOT,
        }
    }
}

impl bsl_bytecode::DynamicCompiler for TestDynamic<'_> {
    fn compile(
        &mut self,
        request: &bsl_bytecode::DynamicRequest<'_>,
    ) -> Result<std::rc::Rc<bsl_bytecode::DynamicUnit>, String> {
        self.scopes += 1;
        bsl_compiler::compile_dynamic_snippet(
            request,
            self.registry,
            &bsl_syntax::PreprocSymbols::new(),
            std::num::NonZeroU64::new(self.scopes).expect("счётчик увеличен перед вызовом"),
        )
        .map(std::rc::Rc::new)
    }
}

/// Прогон без реестра, но с компилятором фрагментов: `run_program` его не
/// даёт намеренно (см. её doc comment), а большинству тестов здесь
/// `Выполнить`/`Вычислить` нужны.
fn run_with_dynamic(program: &Program, jit_mode: JitMode) -> Result<BslValue, RtError> {
    let mut env = bsl_rt::HostEnv::process();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut dynamic = TestDynamic::bare();
    run_program_with_host(
        program,
        None,
        jit_mode,
        &mut stdout,
        &mut stderr,
        Some(&mut dynamic),
        &mut env,
    )
}

/// То же с реестром компонентов.
fn run_with_dynamic_and_registry(
    program: &Program,
    registry: &bsl_rt::RuntimeRegistry,
    jit_mode: JitMode,
) -> Result<BslValue, RtError> {
    let mut env = bsl_rt::HostEnv::process();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut dynamic = TestDynamic::with_registry(registry);
    run_program_with_host(
        program,
        Some(registry),
        jit_mode,
        &mut stdout,
        &mut stderr,
        Some(&mut dynamic),
        &mut env,
    )
}

fn run_src(src: &str) -> BslValue {
    let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    let resolved = resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"));
    let program = compile_program(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"));
    run_with_dynamic(&program, JitMode::Off).unwrap_or_else(|e| panic!("runtime error: {e:?}"))
}

/// Как `run_src`, но с подключённым `bsl-json`: JSON строится только
/// реестром, а тестам канала обратного вызова (функции восстановления
/// и преобразования зовут функции модуля по имени) нужен именно он.
fn run_src_with_json(src: &str) -> BslValue {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder
        .register(bsl_rt::core_library())
        .register(bsl_json::library());
    let registry = builder.build().expect("композиция bsl-rt + bsl-json");
    let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    let resolved = bsl_sema::resolve_program_with_registry(&prog.items, &registry)
        .unwrap_or_else(|e| panic!("sema error: {e:?}"));
    let program = compile_program(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"));
    run_with_dynamic_and_registry(&program, &registry, JitMode::Off)
        .unwrap_or_else(|e| panic!("runtime error: {e:?}"))
}

/// `ОписаниеТипов` берёт имена оттуда же, откуда `Тип("Имя")`: имя
/// типа компонента обязано работать наравне с нативным. Когда
/// компонентные типы ушли из закрытого реестра `TypeId`, этот путь
/// остался единственным, который их не видел, — а фикстуры такого
/// сочетания не пробуют.
#[test]
fn a_type_description_accepts_a_component_type_by_name() {
    let описание = run_src_with_json("Возврат Строка(Новый ОписаниеТипов(\"ЧтениеJSON\"));");
    assert_eq!(описание.to_string(), "ОписаниеТипов");
    // И пробелы в имени по-прежнему не значимы — как у нативных.
    let пробел = run_src_with_json("Возврат Строка(Новый ОписаниеТипов(\"Чтение JSON\"));");
    assert_eq!(пробел.to_string(), "ОписаниеТипов");
}

fn component_answer(
    _context: &mut bsl_rt::CallContext<'_>,
    _args: &[BslValue],
) -> bsl_rt::RtResult<BslValue> {
    Ok(num("42"))
}

fn component_construct(
    _context: &mut bsl_rt::CallContext<'_>,
    _args: &[BslValue],
) -> bsl_rt::RtResult<BslValue> {
    Ok(num("43"))
}

#[derive(Debug)]
struct HostCounter(std::cell::RefCell<i64>);

static HOST_COUNTER_TYPE: bsl_rt::TypeDescriptor =
    bsl_rt::TypeDescriptor::new("bsl-test-host", "СчётчикХоста");

impl bsl_rt::ObjectProtocol for HostCounter {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        &HOST_COUNTER_TYPE
    }

    fn get_property(
        &self,
        name: &str,
        _context: &mut bsl_rt::CallContext<'_>,
    ) -> bsl_rt::RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Значение") || name.eq_ignore_ascii_case("Value") {
            Ok(num(&self.0.borrow().to_string()))
        } else {
            Err(RtError::UnknownProperty(name.to_string()))
        }
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut bsl_rt::CallContext<'_>,
    ) -> bsl_rt::RtResult<()> {
        if !name.eq_ignore_ascii_case("Значение") && !name.eq_ignore_ascii_case("Value") {
            return Err(RtError::UnknownProperty(name.to_string()));
        }
        let BslValue::Number(value) = value else {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "СчётчикХоста.Значение",
            });
        };
        *self.0.borrow_mut() = value.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Целое число",
            op: "СчётчикХоста.Значение",
        })?;
        Ok(())
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut bsl_rt::CallContext<'_>,
    ) -> bsl_rt::RtResult<BslValue> {
        if !name.eq_ignore_ascii_case("Прибавить")
            && !name.eq_ignore_ascii_case("Add")
            && !name.eq_ignore_ascii_case("Добавить")
        {
            return Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: HOST_COUNTER_TYPE.name,
            });
        }
        let [BslValue::Number(delta)] = arguments else {
            return Err(RtError::MethodNotApplicable {
                method: "Прибавить",
                receiver: HOST_COUNTER_TYPE.name,
            });
        };
        let delta = delta.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Целое число",
            op: "СчётчикХоста.Прибавить",
        })?;
        *self.0.borrow_mut() += delta;
        Ok(num(&self.0.borrow().to_string()))
    }

    fn get_index(&self, index: &BslValue) -> bsl_rt::RtResult<BslValue> {
        if *index == num("0") {
            Ok(num(&self.0.borrow().to_string()))
        } else {
            Err(RtError::BadIndex)
        }
    }

    fn set_index(&self, index: &BslValue, value: BslValue) -> bsl_rt::RtResult<()> {
        if *index != num("0") {
            return Err(RtError::BadIndex);
        }
        let BslValue::Number(value) = value else {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "СчётчикХоста[]",
            });
        };
        *self.0.borrow_mut() = value.to_i64_exact().ok_or(RtError::BadIndex)?;
        Ok(())
    }

    fn collection_len(&self) -> bsl_rt::RtResult<usize> {
        Ok(1)
    }

    fn display(&self) -> String {
        format!("СчётчикХоста({})", self.0.borrow())
    }
}

fn component_counter(
    _context: &mut bsl_rt::CallContext<'_>,
    _args: &[BslValue],
) -> bsl_rt::RtResult<BslValue> {
    Ok(BslValue::new_object(HostCounter(std::cell::RefCell::new(
        0,
    ))))
}

fn component_message(
    context: &mut bsl_rt::CallContext<'_>,
    _args: &[BslValue],
) -> bsl_rt::RtResult<BslValue> {
    writeln!(context.stdout()?, "component")
        .map_err(|error| RtError::IoError(error.to_string()))?;
    Ok(BslValue::Undefined)
}

const TEST_COMPONENT_FUNCTIONS: &[bsl_rt::FunctionDescriptor] = &[
    bsl_rt::FunctionDescriptor {
        code: bsl_rt::FunctionCode::new(7),
        names: &["ОтветПриложения", "ApplicationAnswer"],
        arity: bsl_rt::Arity::exact(0),
        kind: bsl_rt::FunctionKind::Function,
        call: component_answer,
    },
    bsl_rt::FunctionDescriptor {
        code: bsl_rt::FunctionCode::new(8),
        names: &["СообщитьПриложения", "ApplicationMessage"],
        arity: bsl_rt::Arity::exact(0),
        kind: bsl_rt::FunctionKind::Procedure,
        call: component_message,
    },
];
const TEST_COMPONENT_CONSTRUCTORS: &[bsl_rt::ConstructorDescriptor] = &[
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(9),
        names: &["ТестовыйОбъект", "TestObject"],
        arity: bsl_rt::Arity::exact(0),
        call: component_construct,
    },
    bsl_rt::ConstructorDescriptor {
        code: bsl_rt::ConstructorCode::new(10),
        names: &["СчётчикХоста", "HostCounter"],
        arity: bsl_rt::Arity::exact(0),
        call: component_counter,
    },
];

fn test_component_registry() -> bsl_rt::RuntimeRegistry {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder
        .register(bsl_rt::LibraryDescriptor::new(
            bsl_rt::PACKAGE_NAME,
            bsl_rt::PACKAGE_VERSION,
            bsl_rt::ObjectJitPolicy::NativeContextCompatible,
        ))
        .register(
            bsl_rt::LibraryDescriptor::new(
                "bsl-test-host",
                "1.2.3",
                bsl_rt::ObjectJitPolicy::NativeContextCompatible,
            )
            .with_dependencies(&[bsl_rt::LibraryDependency {
                package: bsl_rt::PACKAGE_NAME,
                version: bsl_rt::PACKAGE_VERSION,
            }])
            .with_functions(TEST_COMPONENT_FUNCTIONS)
            .with_constructors(TEST_COMPONENT_CONSTRUCTORS),
        );
    builder.build().unwrap()
}

fn compile_with_registry(src: &str, registry: &bsl_rt::RuntimeRegistry) -> Program {
    let parsed = parse(src).unwrap_or_else(|error| panic!("parse error: {error:?}"));
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, registry)
        .unwrap_or_else(|error| panic!("sema error: {error:?}"));
    compile_program(&resolved).unwrap_or_else(|error| panic!("compile error: {error:?}"))
}

#[test]
fn component_function_resolves_compiles_links_and_runs() {
    let registry = test_component_registry();
    let parsed = parse("Возврат ОтветПриложения();").unwrap();
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, &registry).unwrap();
    let program = compile_program(&resolved).unwrap();

    assert_eq!(program.requirements.len(), 2);
    assert_eq!(program.requirements[1].package, "bsl-test-host");
    assert!(program.chunks[0].instrs.iter().any(|instruction| matches!(
        instruction,
        Instr::CallComponent {
            library: 1,
            function: 7,
            count: 0,
            ..
        }
    )));
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off).unwrap(),
        num("42")
    );
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::On).unwrap(),
        num("42")
    );
}

#[test]
fn component_mismatch_is_rejected_before_execution() {
    let registry = test_component_registry();
    let parsed = parse("Возврат ОтветПриложения();").unwrap();
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, &registry).unwrap();
    let mut program = compile_program(&resolved).unwrap();
    program.requirements[1].version = "9.9.9".to_string();

    assert!(matches!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off),
        Err(RtError::Link(message)) if message.contains("9.9.9")
    ));
}

#[test]
fn component_constructor_resolves_compiles_links_and_runs() {
    let registry = test_component_registry();
    let parsed = parse("Возврат Новый ТестовыйОбъект();").unwrap();
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, &registry).unwrap();
    let program = compile_program(&resolved).unwrap();

    assert!(program.chunks[0].instrs.iter().any(|instruction| matches!(
        instruction,
        Instr::CreateObject {
            library: 1,
            constructor: 9,
            count: 0,
            ..
        }
    )));
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off).unwrap(),
        num("43")
    );
}

/// Полиморфный сайт открытого вызова: одна и та же инструкция видит
/// то конвертированный тип со статической таблицей (`ЗаписьJSON`), то
/// хостовый без неё. Ячейка кэша метода (см. `cached_component_method`)
/// обязана перечитываться при смене таблицы получателя, а тип без
/// имени в таблице — уходить строковым путём с прежней ошибкой; JIT
/// идёт тем же кэшем через шим.
#[test]
fn a_polymorphic_open_call_site_revalidates_its_method_cache() {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder
        .register(bsl_rt::core_library())
        .register(bsl_json::library())
        .register(
            bsl_rt::LibraryDescriptor::new(
                "bsl-test-host",
                "1.2.3",
                bsl_rt::ObjectJitPolicy::NativeContextCompatible,
            )
            .with_dependencies(&[bsl_rt::LibraryDependency {
                package: bsl_rt::PACKAGE_NAME,
                version: bsl_rt::PACKAGE_VERSION,
            }])
            .with_functions(TEST_COMPONENT_FUNCTIONS)
            .with_constructors(TEST_COMPONENT_CONSTRUCTORS),
        );
    let registry = builder.build().unwrap();
    let program = compile_with_registry(
        "з = Новый ЗаписьJSON;\n\
         с = Новый СчётчикХоста();\n\
         рез = \"\";\n\
         Для н = 1 По 4 Цикл\n\
             Если н % 2 = 1 Тогда\n\
                 об = з;\n\
             Иначе\n\
                 об = с;\n\
             КонецЕсли;\n\
             Попытка\n\
                 об.УстановитьСтроку();\n\
                 рез = рез + \"+\";\n\
             Исключение\n\
                 рез = рез + \"-\";\n\
             КонецПопытки;\n\
         КонецЦикла;\n\
         Возврат рез;",
        &registry,
    );
    let expected = BslValue::Str(bsl_rt::BslString::from_str("+-+-"));
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off).unwrap(),
        expected
    );
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::On).unwrap(),
        expected
    );
}

/// Приёмник, статически доказанный ядровым (см. `core_receivers` в
/// `bsl-sema`), и с реестром компилируется в закрытые опкоды: путь
/// `csv_write` — `WriteText` с инкрементом `pc` вместо холодного
/// `CallObjectMethod` на каждую запись.
#[test]
fn a_proven_core_receiver_compiles_closed_even_with_a_registry() {
    let registry = test_component_registry();
    let program = compile_with_registry(
        "ф = Новый ЗаписьТекста(\"пусто.тмп\");\n\
         д = Новый Структура(\"а\", 1);\n\
         ф.Записать(д.а);\n\
         ф.Закрыть();\n\
         м = Новый Массив;\n\
         м.Добавить(1);",
        &registry,
    );
    let instructions = &program.chunks[0].instrs;
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::WriteText { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::CloseText { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::GetProp { .. }))
    );
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instr::CallMethod {
            method: bsl_rt::BuiltinMethod::Add,
            ..
        }
    )));
    assert!(!instructions.iter().any(|instruction| matches!(
        instruction,
        Instr::CallObjectMethod { .. } | Instr::GetObjectProp { .. } | Instr::SetObjectProp { .. }
    )));
}

#[test]
fn component_object_owns_properties_methods_and_indexes() {
    let registry = test_component_registry();
    let program = compile_with_registry(
        "с = Новый СчётчикХоста();\n\
         с.Значение = 10;\n\
         с[0] = 11;\n\
         с.Прибавить(5);\n\
         Возврат с.Добавить(1) + с.Значение + с[0];",
        &registry,
    );

    // Свойства компилируются в закрытые `GetProp`/`SetProp` даже для
    // компонентного получателя (резолвер больше не выпускает открытые
    // двойники — их тела совпадают, см. комментарий у `RExpr::Field`),
    // а вот метод вне ядровой таблицы обязан идти открытым
    // `CallObjectMethod`.
    let instructions = &program.chunks[0].instrs;
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::GetProp { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::SetProp { .. }))
    );
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction, Instr::CallObjectMethod { .. }))
    );

    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off).unwrap(),
        num("51")
    );
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::On).unwrap(),
        num("51")
    );
}

#[test]
fn dynamic_fragment_resolves_its_own_component_requirement() {
    let registry = test_component_registry();
    let program = compile_with_registry("Возврат Вычислить(\"ОтветПриложения()\");", &registry);

    assert_eq!(program.requirements.len(), 1);
    assert_eq!(
        run_with_dynamic_and_registry(&program, &registry, JitMode::Off).unwrap(),
        num("42")
    );
}

#[test]
fn host_streams_are_used_by_builtins_components_dynamic_code_and_jit() {
    let registry = test_component_registry();
    let program = compile_with_registry(
        "Сообщить(\"main\");\n\
         Выполнить(\"Сообщить(\"\"dynamic\"\")\");\n\
         СообщитьПриложения();",
        &registry,
    );

    for jit in [false, true] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let jit_mode = if jit { JitMode::On } else { JitMode::Off };
        let result = run_program_with_registry_and_io(
            &program,
            &registry,
            jit_mode,
            &mut stdout,
            &mut stderr,
            &mut TestDynamic::with_registry(&registry),
            &mut bsl_rt::HostEnv::process(),
        );

        assert_eq!(result.unwrap(), BslValue::Undefined);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "main\ndynamic\ncomponent\n"
        );
        assert!(stderr.is_empty());
    }
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("test writer failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn host_writer_error_is_returned_without_a_panic() {
    let registry = test_component_registry();
    let program = compile_with_registry("Сообщить(\"x\");", &registry);
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();

    assert!(matches!(
        run_program_with_registry_and_io(
            &program,
            &registry,
            JitMode::Off,
            &mut stdout,
            &mut stderr,
            &mut TestDynamic::with_registry(&registry),
            &mut bsl_rt::HostEnv::process(),
        ),
        Err(RtError::IoError(message)) if message.contains("test writer failed")
    ));
    assert!(stderr.is_empty());
}

/// Строка СЛЕВА тянет правый операнд к себе, и приведение это ровно
/// `Строка()` — вместе с разделителем групп. Замеры `CONCAT.RIGHT.*`.
#[test]
fn a_string_on_the_left_pulls_the_right_operand_into_itself() {
    let cases = [
        (r#"Возврат "[" + 5 + "]";"#, "[5]"),
        // Неразрывный пробел, а не обычный: платформа печатает группы
        // именно им, и склейка наследует это целиком.
        (r#"Возврат "[" + 1000.5 + "]";"#, "[1\u{a0}000,5]"),
        (r#"Возврат "[" + Истина + "]";"#, "[Да]"),
        (r#"Возврат "[" + Ложь + "]";"#, "[Нет]"),
        (r#"Возврат "[" + Неопределено + "]";"#, "[]"),
        (r#"Возврат "[" + Null + "]";"#, "[]"),
        (r#"Возврат "[" + Новый Массив + "]";"#, "[Массив]"),
        (
            r#"Возврат "[" + '20240115103000' + "]";"#,
            "[15.01.2024 10:30:00]",
        ),
        // Склейка побеждает арифметику даже когда обе стороны похожи на
        // числа, и даже когда справа булево.
        (r#"Возврат "5" + 1;"#, "51"),
        (r#"Возврат "5" + Истина;"#, "5Да"),
        // Левоассоциативность: от строки склейка идёт до конца.
        (r#"Возврат "х" + 1 + 2;"#, "х12"),
    ];
    for (src, want) in cases {
        let got = run_src(src);
        assert_eq!(str_val(&got), want, "{src}");
    }
}

/// Обратное направление: где слева не строка, к числу тянутся ОБА
/// операнда — и строка, и булево. Замеры `ARITH.*`.
#[test]
fn arithmetic_pulls_strings_and_booleans_to_numbers() {
    let cases = [
        (r#"Возврат "5" - 1;"#, "4"),
        (r#"Возврат "5" * 2;"#, "10"),
        (r#"Возврат "5" / 2;"#, "2.5"),
        (r#"Возврат 1 - "5";"#, "-4"),
        (r#"Возврат 5 + "3";"#, "8"),
        // Разбор строки — тот же, что у `Число()`: пробелы по краям,
        // точка ИЛИ запятая, разделители групп.
        (r#"Возврат " 5 " - 1;"#, "4"),
        (r#"Возврат "5.5" - 1;"#, "4.5"),
        (r#"Возврат "5,5" - 1;"#, "4.5"),
        // Круговой прогон: напечатанное платформой число разбирается
        // обратно вместе с неразрывными пробелами.
        (r#"Возврат ("" + 1000.5) - 0.5;"#, "1000"),
        (r#"Возврат -"5";"#, "-5"),
        (r#"Возврат Истина + 1;"#, "2"),
        (r#"Возврат Ложь + 1;"#, "1"),
        (r#"Возврат Истина - 1;"#, "0"),
        (r#"Возврат Истина * 2;"#, "2"),
        (r#"Возврат -Истина;"#, "-1"),
        (r#"Возврат Истина + "3";"#, "4"),
    ];
    for (src, want) in cases {
        let BslValue::Number(n) = run_src(src) else {
            panic!("ожидалось число: {src}");
        };
        assert_eq!(n.to_canonical(), want, "{src}");
    }
    // Дата плюс ЧИСЛОВАЯ строка — сдвиг на секунды.
    let v = run_src(r#"Возврат Строка('20240115103000' + "60");"#);
    assert_eq!(str_val(&v), "15.01.2024 10:31:00");
}

/// Приведение не безгранично: нечисловая и пустая строка отвергаются, а
/// `Неопределено` нулём не притворяется. Замеры `ARITH.STR.NOT_A_NUMBER`,
/// `ARITH.STR.EMPTY`, `ARITH.UNDEFINED.PLUS`, `CONCAT.ORDER.NUM_NUM_STR`.
#[test]
fn coercion_has_limits() {
    for src in [
        r#"Возврат "абв" - 1;"#,
        r#"Возврат "" - 1;"#,
        r#"Возврат Неопределено + 1;"#,
        r#"Возврат 5 + "]";"#,
        r#"Возврат Неопределено + "]";"#,
        // Слева направо: 1 + 2 складываются, и уже 3 + строка отказывает.
        r#"Возврат 1 + 2 + "х";"#,
    ] {
        let _ = run_src_err(src);
    }
}

fn run_src_err(src: &str) -> RtError {
    let prog = parse(src).unwrap();
    let resolved = resolve_program(&prog.items).unwrap();
    let program = compile_program(&resolved).unwrap();
    run_with_dynamic(&program, JitMode::Off).unwrap_err()
}

fn num(s: &str) -> BslValue {
    BslValue::Number(BslNumber::parse_canonical(s).unwrap())
}

/// Ошибка на НЕ ПЕРВОМ члене VLIW-бандла: `pc` в момент ошибки обязан
/// стоять на сбойном члене, обработчик — поймать её, эффекты ранних
/// членов — быть видимыми, а остаток бандла — не исполниться. Тест
/// заодно проверяет свою предпосылку по разметке: деление действительно
/// лежит внутри бандла, а не начинает его.
#[test]
fn exception_on_a_non_first_bundle_member_lands_in_the_right_handler() {
    let src = "а = 10;\n\
         б = 0;\n\
         рез = 0;\n\
         Попытка\n\
             г = 7;\n\
             в = а / б;\n\
             г = 100;\n\
         Исключение\n\
             рез = г;\n\
         КонецПопытки;\n\
         Возврат рез;";
    let prog = parse(src).unwrap();
    let resolved = resolve_program(&prog.items).unwrap();
    let program = compile_program(&resolved).unwrap();
    let chunk = &program.chunks[0];
    let div_pc = chunk
        .instrs
        .iter()
        .position(|i| matches!(i, Instr::Div { .. }))
        .expect("в скрипте есть деление");
    assert_eq!(
        chunk.bundle_len[div_pc], 0,
        "предпосылка теста: деление — внутри бандла (`г = 7` перед ним \
         независимо); если компилятор стал раскладывать иначе — \
         подберите скрипту новую пару"
    );
    let v = run_with_dynamic(&program, JitMode::Off).unwrap();
    // `г = 7` исполнилось, деление упало, `г = 100` не исполнялось.
    assert_eq!(v, num("7"));
}

// НЕ ИЗМЕРЕНО(EXEC.MAX_CALL_DEPTH) — тесты фиксируют ВЫБРАННОЕ
// поведение: перехватываемая ошибка вместо роста памяти до OOM; сам
// предел платформы не замерен.
#[test]
fn unbounded_recursion_is_a_catchable_error_not_oom() {
    let v = run_src(
        "Функция Ф(Н)\n\
         Возврат Ф(Н + 1);\n\
         КонецФункции\n\
         x = 0;\n\
         Попытка\n\
         x = Ф(0);\n\
         Исключение\n\
         x = 99;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("99"));
}

#[test]
fn unbounded_recursion_outside_try_is_a_stack_overflow_error() {
    let e = run_src_err("Функция Ф()\nВозврат Ф();\nКонецФункции\nВозврат Ф();");
    assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
}

#[test]
fn recursion_well_below_the_limit_still_works() {
    // 900 уровней — нижняя граница из замера: она обязана работать.
    let v = run_src(
        "Функция Ф(Н)\n\
         Если Н = 0 Тогда\n\
         Возврат 0;\n\
         КонецЕсли\n\
         Возврат Ф(Н - 1) + 1;\n\
         КонецФункции\n\
         Возврат Ф(900);",
    );
    assert_eq!(v, num("900"));
}

#[test]
fn write_json_of_a_cyclic_structure_is_catchable() {
    // Сквозной вариант юнит-теста из `bsl-rt`: предел глубины JSON
    // (`JSON.MAX_DEPTH`) должен доходить до `Попытка` обычным путём.
    let v = run_src_with_json(
        "А = Новый Массив;\n\
         А.Добавить(А);\n\
         З = Новый ЗаписьJSON;\n\
         З.УстановитьСтроку();\n\
         x = 0;\n\
         Попытка\n\
         ЗаписатьJSON(З, А);\n\
         Исключение\n\
         x = 99;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("99"));
}

/// Тесты с вложенными `Выполнить` гоняются в потоке со стеком главного
/// потока (8 МиБ): предел `MAX_DYNAMIC_DEPTH` калиброван под него, а
/// libtest по умолчанию даёт тестовому потоку 2 МиБ — там вложенные
/// `drive` на самом пределе честно не помещаются, и тест мерил бы не то.
fn on_main_sized_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(body)
        .expect("поток не создался")
        .join()
        .expect("тест в потоке упал");
}

// НЕ ИЗМЕРЕНО(EXEC.DYNAMIC_DEPTH) — то же: выбран предел вложенности,
// платформа не замерена.
#[test]
fn recursion_through_execute_is_a_catchable_error_not_a_crash() {
    on_main_sized_stack(|| {
        let v = run_src(
            "Процедура П()\n\
             Выполнить(\"П()\");\n\
             КонецПроцедуры\n\
             x = 0;\n\
             Попытка\n\
             П();\n\
             Исключение\n\
             x = 99;\n\
             КонецПопытки\n\
             Возврат x;",
        );
        assert_eq!(v, num("99"));
    });
}

#[test]
fn nested_execute_sees_and_updates_module_vars() {
    // Регрессия: модульный блок копировался во фрагмент с абсолютного
    // нуля, а не с `module_base` текущей программы, поэтому на ВТОРОМ
    // уровне `Выполнить` модульная переменная приезжала мусором
    // (Неопределено), а изменения не возвращались наружу.
    let v = run_src(
        "Перем Счёт;\n\
         Процедура Раз()\n\
         Счёт = Счёт + 1;\n\
         Выполнить(\"Два()\");\n\
         КонецПроцедуры\n\
         Процедура Два()\n\
         Счёт = Счёт + 10;\n\
         КонецПроцедуры\n\
         Счёт = 0;\n\
         Выполнить(\"Раз()\");\n\
         Возврат Счёт;",
    );
    assert_eq!(v, num("11"));
}

#[test]
fn execute_nesting_below_the_limit_still_works() {
    on_main_sized_stack(|| {
        // 40 уровней — нижняя граница из замера: обязана работать.
        let v = run_src(
            "Перем Глубина;\n\
             Процедура П()\n\
             Глубина = Глубина + 1;\n\
             Если Глубина < 40 Тогда\n\
             Выполнить(\"П()\");\n\
             КонецЕсли\n\
             КонецПроцедуры\n\
             Глубина = 0;\n\
             П();\n\
             Возврат Глубина;",
        );
        assert_eq!(v, num("40"));
    });
}

#[test]
fn function_call_and_return_value() {
    let v = run_src("Функция Ф()\nВозврат 42;\nКонецФункции\nВозврат Ф();");
    assert_eq!(v, num("42"));
}

#[test]
fn function_without_return_yields_undefined_and_is_callable_as_statement() {
    let v = run_src("Процедура П()\nx = 1;\nКонецПроцедуры\nП();\nВозврат Неопределено;");
    assert_eq!(v, BslValue::Undefined);
}

#[test]
fn recursion_factorial() {
    let v = run_src(
        "Функция Факториал(n)\n\
         Если n <= 1 Тогда\n\
         Возврат 1;\n\
         КонецЕсли;\n\
         Возврат n * Факториал(n - 1);\n\
         КонецФункции\n\
         Возврат Факториал(5);",
    );
    assert_eq!(v, num("120"));
}

#[test]
fn by_reference_parameter_mutates_callers_variable() {
    // Процедура П(а) а = 5 КонецПроцедуры — меняет переменную вызывающего,
    // т.к. параметры без Знач передаются по ссылке.
    let v = run_src(
        "Процедура П(а)\n\
         а = 5;\n\
         КонецПроцедуры\n\
         x = 1;\n\
         П(x);\n\
         Возврат x;",
    );
    assert_eq!(v, num("5"));
}

#[test]
fn by_value_parameter_does_not_mutate_callers_variable() {
    let v = run_src(
        "Процедура П(Знач а)\n\
         а = 5;\n\
         КонецПроцедуры\n\
         x = 1;\n\
         П(x);\n\
         Возврат x;",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn by_reference_swap_via_two_parameters() {
    let v = run_src(
        "Процедура Обменять(а, б)\n\
         временная = а;\n\
         а = б;\n\
         б = временная;\n\
         КонецПроцедуры\n\
         x = 1;\n\
         y = 2;\n\
         Обменять(x, y);\n\
         Возврат x * 10 + y;",
    );
    // Было x=1,y=2 -> после обмена x=2,y=1 -> 2*10+1 = 21.
    assert_eq!(v, num("21"));
}

#[test]
fn by_reference_argument_that_is_not_a_bare_variable_does_not_crash() {
    // Аргумент — выражение, не переменная: запись в параметр пишет в
    // одноразовую ячейку, наблюдаемого эффекта у вызывающего нет, но и
    // падать тут нечему.
    let v = run_src(
        "Процедура П(а)\n\
         а = 99;\n\
         КонецПроцедуры\n\
         x = 1;\n\
         П(x + 1);\n\
         Возврат x;",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn mutual_forward_calls_between_functions() {
    let v = run_src(
        "Функция ЧетноеЛи(n)\n\
         Если n = 0 Тогда\n\
         Возврат Истина;\n\
         КонецЕсли;\n\
         Возврат НечетноеЛи(n - 1);\n\
         КонецФункции\n\
         Функция НечетноеЛи(n)\n\
         Если n = 0 Тогда\n\
         Возврат Ложь;\n\
         КонецЕсли;\n\
         Возврат ЧетноеЛи(n - 1);\n\
         КонецФункции\n\
         Возврат ЧетноеЛи(6);",
    );
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn division_matches_oracle_27_digits_inside_a_function() {
    let v = run_src("Функция Ф()\nВозврат 1 / 3;\nКонецФункции\nВозврат Ф();");
    assert_eq!(v, num("0.333333333333333333333333333"));
}

#[test]
fn while_and_for_loops_still_work_at_top_level() {
    let v = run_src(
        "sum = 0;\n\
         Для i = 0 По 10 Цикл\n\
         sum = sum + i;\n\
         КонецЦикла\n\
         Возврат sum;",
    );
    assert_eq!(v, num("55"));
}

#[test]
fn numeric_for_specialization_preserves_counter_assignment_and_final_value() {
    let v = run_src(
        "sum = 0;\n\
         Для i = 0 По 10 Цикл\n\
         sum = sum + i;\n\
         Если i = 2 Тогда i = 8; КонецЕсли;\n\
         КонецЦикла\n\
         Возврат sum * 100 + i;",
    );
    // Посещены 0, 1, 2, 9, 10; после последнего шага счётчик равен 11.
    assert_eq!(v, num("2211"));

    let v = run_src("Для i = 5 По 3 Цикл КонецЦикла; Возврат i;");
    assert_eq!(v, num("5"));

    // Пустое тело использует скрытый i64-счётчик, но наружу обязано
    // материализовать то же финальное BSL-значение.
    let v = run_src("Для i = 0 По 10 Цикл КонецЦикла; Возврат i;");
    assert_eq!(v, num("11"));

    // Значения, которые нельзя представить скрытым i64, автоматически
    // остаются на общем decimal/BigInt-пути.
    let v = run_src("Для i = 0.5 По 2.5 Цикл КонецЦикла; Возврат i;");
    assert_eq!(v, num("3.5"));
    let v = run_src(
        "Для i = 100000000000000000000 По 100000000000000000000 Цикл \
         КонецЦикла; Возврат i;",
    );
    assert_eq!(v, num("100000000000000000001"));
}

/// Невыбранная ветвь НЕ исполняется. Это главное свойство оператора:
/// на нём держится оборот `?(Знач <> Неопределено, Знач.Поле, "")`,
/// который при жадном вычислении падал бы. Замеры `TERNARY.LAZY_*`.
#[test]
fn ternary_does_not_evaluate_the_branch_it_did_not_take() {
    // Делитель через `Число("0")`: литеральный ноль компилятор вправе
    // свернуть, и проба мерила бы сворачивание, а не ленивость.
    assert_eq!(
        run_src(r#"Возврат ?(Истина, "ок", 1 / Число("0"));"#),
        BslValue::Str(bsl_rt::BslString::from_str("ок"))
    );
    assert_eq!(
        run_src(r#"Возврат ?(Ложь, 1 / Число("0"), "ок");"#),
        BslValue::Str(bsl_rt::BslString::from_str("ок"))
    );
    // Обращение к полю у `Неопределено` — вторая форма той же проверки:
    // она падает, только если ветвь всё-таки вычислена.
    assert_eq!(
        run_src(r#"З = Неопределено; Возврат ?(З <> Неопределено, З.Поле, "пусто");"#),
        BslValue::Str(bsl_rt::BslString::from_str("пусто"))
    );
}

/// Условие у `?()` — то же самое, что у `Если`, и ветви могут быть
/// разных типов. Замеры `TERNARY.CONDITION_*`, `TERNARY.TYPE_OF_RESULT`.
#[test]
fn ternary_takes_the_same_conditions_as_everything_else() {
    let cases = [
        (r#"Возврат ?(1, "да", "нет");"#, "да"),
        (r#"Возврат ?(0, "да", "нет");"#, "нет"),
        (r#"Возврат ?("истина", "да", "нет");"#, "да"),
        (r#"Возврат ?(" Ложь ", "да", "нет");"#, "нет"),
        (r#"Возврат ?(2 > 1, "да", "нет");"#, "да"),
        // Вложение во все три позиции.
        (r#"Возврат ?(Истина, ?(Ложь, "а", "б"), "в");"#, "б"),
        (r#"Возврат ?(Ложь, "а", ?(Истина, "б", "в"));"#, "б"),
        (r#"Возврат ?(?(Истина, Истина, Ложь), "да", "нет");"#, "да"),
        // Результат — обычное значение и работает дальше по выражению.
        (r#"Возврат "[" + ?(Истина, 1, 2) + "]";"#, "[1]"),
    ];
    for (src, want) in cases {
        assert_eq!(str_val(&run_src(src)), want, "{src}");
    }
    // Тип результата — у выбранной ветви, а не общий.
    assert_eq!(
        run_src(r#"Возврат ?(Истина, 5, "строка");"#),
        BslValue::Number(bsl_number::BslNumber::from_i64(5))
    );
    for src in [
        r#"Возврат ?(Неопределено, "да", "нет");"#,
        r#"Возврат ?("абв", "да", "нет");"#,
        r#"Возврат ?(Новый Массив, "да", "нет");"#,
    ] {
        assert!(run_src_err(src).to_string().contains("Булево"), "{src}");
    }
}

#[test]
fn condition_converts_numbers_and_words_but_not_anything_else() {
    // Здесь стоял обратный тест: `Если 1 Тогда` считалось ошибкой. На
    // платформе это работает (замер `COND.IF_NUMBER_ONE`), и правило
    // одно на все условия языка.
    assert_eq!(
        run_src("Если 1 Тогда\nВозврат \"да\";\nИначе\nВозврат \"нет\";\nКонецЕсли"),
        BslValue::Str(bsl_rt::BslString::from_str("да"))
    );
    assert_eq!(
        run_src("Если 0 Тогда\nВозврат \"да\";\nИначе\nВозврат \"нет\";\nКонецЕсли"),
        BslValue::Str(bsl_rt::BslString::from_str("нет"))
    );
    // Строка — только словом, и мусор по-прежнему ошибка.
    assert_eq!(
        run_src("Если \"Истина\" Тогда\nВозврат 1;\nИначе\nВозврат 2;\nКонецЕсли"),
        BslValue::Number(bsl_number::BslNumber::from_i64(1))
    );
    for src in [
        "Если \"абв\" Тогда\nх = 1;\nКонецЕсли",
        "Если Неопределено Тогда\nх = 1;\nКонецЕсли",
    ] {
        let err = run_src_err(src);
        assert!(
            matches!(
                err,
                RtError::TypeError {
                    expected: "Булево",
                    ..
                }
            ),
            "{src}: {err:?}"
        );
    }
}

#[test]
fn short_circuit_and_skips_right_operand_on_false() {
    // Без ленивости `Неопределено.Свойство` бросил бы NotAnObject —
    // здесь этого не должно случиться, левый операнд уже решил результат.
    let v = run_src("Возврат Ложь И Неопределено.Свойство;");
    assert_eq!(v, BslValue::Boolean(false));
}

#[test]
fn short_circuit_or_skips_right_operand_on_true() {
    let v = run_src("Возврат Истина ИЛИ Неопределено.Свойство;");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn logical_operators_convert_operands_and_yield_a_boolean() {
    // Оба операнда проходят то же приведение, что и любое условие, —
    // и левый, который становится условием перехода, и правый.
    for (src, want) in [
        ("Возврат Истина И 1;", true),
        ("Возврат 1 И Истина;", true),
        ("Возврат 1 И 1;", true),
        ("Возврат Ложь ИЛИ 1;", true),
        ("Возврат 0 ИЛИ Ложь;", false),
        ("Возврат \"Истина\" И 1;", true),
    ] {
        // Результат — БУЛЕВО, а не последний вычисленный операнд:
        // `1 И 1` на платформе даёт «Да», а не единицу (замер
        // `COND.AND_BOTH_NUMBERS`).
        assert_eq!(run_src(src), BslValue::Boolean(want), "{src}");
    }

    // Приведение не безгранично и здесь.
    let err = run_src_err("Возврат Истина И Неопределено;");
    assert!(matches!(
        err,
        RtError::TypeError {
            expected: "Булево",
            ..
        }
    ));
}

#[test]
fn short_circuit_chain_of_three_operands() {
    // Цепочка `А И Б И В`: если А уже Ложь, ни Б, ни В вычисляться не
    // должны.
    let v = run_src("Возврат Ложь И Неопределено.Свойство И Неопределено.ДругоеСвойство;");
    assert_eq!(v, BslValue::Boolean(false));
}

#[test]
fn division_by_zero_is_a_runtime_error() {
    let err = run_src_err("x = 1 / 0;");
    assert!(matches!(
        err,
        RtError::Num(bsl_number::NumError::DivideByZero)
    ));
}

#[test]
fn array_construction_indexing_and_mutation() {
    let v = run_src(
        "a = Новый Массив(3);\n\
         a[0] = 10;\n\
         a[1] = 20;\n\
         a[2] = a[0] + a[1];\n\
         Возврат a[2];",
    );
    assert_eq!(v, num("30"));
}

#[test]
fn nested_array_dimensions() {
    // Новый Массив(3, 4) -> массив из 3 независимых массивов по 4.
    let v = run_src(
        "a = Новый Массив(3, 4);\n\
         a[0][0] = 1;\n\
         a[1][0] = 2;\n\
         Возврат a[0][0] + a[1][0];",
    );
    assert_eq!(v, num("3"));
}

#[test]
fn nested_array_slots_are_independent_objects() {
    let v = run_src(
        "a = Новый Массив(2, 2);\n\
         a[0][0] = 1;\n\
         Возврат a[1][0];",
    );
    // Если бы вложенные массивы были одним общим объектом (баг), тут
    // тоже было бы 1 — а не Неопределено.
    assert_eq!(v, BslValue::Undefined);
}

#[test]
fn array_index_out_of_bounds_is_a_runtime_error() {
    let err = run_src_err("a = Новый Массив(1);\nВозврат a[5];");
    assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
}

#[test]
fn structure_construction_and_field_access() {
    let v = run_src(
        "s = Новый Структура(\"x,y,z\", 1, 2, 3);\n\
         s.y = s.y + 100;\n\
         Возврат s.x + s.y + s.z;",
    );
    assert_eq!(v, num("106"));
}

#[test]
fn get_prop_inline_cache_stays_correct_across_different_shapes() {
    // Один и тот же GetProp (внутри тела Для Каждого) видит структуры
    // ДВУХ разных форм подряд — если бы кэш слепо доверял старому
    // (форма, слот) без проверки Rc::ptr_eq, второе значение
    // прочиталось бы по слоту первой формы и оказалось бы неверным.
    let v = run_src(
        "a = Новый Массив(2);\n\
         a[0] = Новый Структура(\"x,y\", 10, 20);\n\
         a[1] = Новый Структура(\"y,x\", 200, 100);\n\
         сумма = 0;\n\
         Для Каждого elem Из a Цикл\n\
         сумма = сумма + elem.x;\n\
         КонецЦикла\n\
         Возврат сумма;",
    );
    // elem.x -> 10 (форма "x,y", x на слоте 0) + 100 (форма "y,x", x на слоте 1) = 110.
    assert_eq!(v, num("110"));
}

#[test]
fn set_prop_inline_cache_stays_correct_across_different_shapes() {
    let v = run_src(
        "a = Новый Массив(2);\n\
         a[0] = Новый Структура(\"x,y\", 0, 0);\n\
         a[1] = Новый Структура(\"y,x\", 0, 0);\n\
         Для Каждого elem Из a Цикл\n\
         elem.x = 42;\n\
         КонецЦикла\n\
         Возврат a[0].x + a[1].x;",
    );
    assert_eq!(v, num("84"));
}

#[test]
fn structure_keys_only_defaults_to_undefined() {
    let v = run_src("s = Новый Структура(\"x\");\nВозврат s.x;");
    assert_eq!(v, BslValue::Undefined);
}

#[test]
fn unknown_field_is_a_runtime_error() {
    let err = run_src_err("s = Новый Структура(\"x\");\nВозврат s.z;");
    assert!(matches!(err, RtError::UnknownField(_)));
}

#[test]
fn structure_insert_adds_a_new_field_at_runtime() {
    // Задача 4 ревью: `ShapeTable` больше не только компиляционная —
    // `Вставить` заводит поле, которого не было в литерале `Новый
    // Структура(...)`, и оно сразу читается через `.y`.
    let v = run_src(
        "s = Новый Структура(\"x\", 1);\n\
         s.Вставить(\"y\", 2);\n\
         Возврат s.x + s.y;",
    );
    assert_eq!(v, num("3"));
}

#[test]
fn structure_insert_on_existing_field_overwrites_value_not_shape() {
    let v = run_src(
        "s = Новый Структура(\"x\", 1);\n\
         s.Вставить(\"x\", 99);\n\
         Возврат s.x;",
    );
    assert_eq!(v, num("99"));
}

#[test]
fn structure_delete_removes_field_and_shrinks_count() {
    let v = run_src(
        "s = Новый Структура(\"x,y\", 1, 2);\n\
         s.Удалить(\"x\");\n\
         Возврат s.Количество();",
    );
    assert_eq!(v, num("1"));
    let err = run_src_err(
        "s = Новый Структура(\"x,y\", 1, 2);\n\
         s.Удалить(\"x\");\n\
         Возврат s.x;",
    );
    assert!(matches!(err, RtError::UnknownField(_)));
}

#[test]
fn structure_delete_missing_field_is_a_no_op() {
    let v = run_src(
        "s = Новый Структура(\"x\", 1);\n\
         s.Удалить(\"нетполя\");\n\
         Возврат s.Количество();",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn structure_property_returns_value_or_undefined_or_default() {
    let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"x\");");
    assert_eq!(v, num("5"));
    let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"y\");");
    assert_eq!(v, BslValue::Undefined);
    let v = run_src("s = Новый Структура(\"x\", 5);\nВозврат s.Свойство(\"y\", 42);");
    assert_eq!(v, num("42"));
}

#[test]
fn structure_clear_resets_field_set_not_just_values() {
    let v = run_src(
        "s = Новый Структура(\"x,y\", 1, 2);\n\
         s.Очистить();\n\
         Возврат s.Количество();",
    );
    assert_eq!(v, num("0"));
    let err = run_src_err(
        "s = Новый Структура(\"x,y\", 1, 2);\n\
         s.Очистить();\n\
         Возврат s.x;",
    );
    assert!(matches!(err, RtError::UnknownField(_)));
}

#[test]
fn structure_insert_in_a_loop_converges_on_one_shape_like_the_literal_path() {
    // Инвариант "формы интернируются глобально по набору ключей" не
    // должен ломаться рантайм-путём: одинаковый набор полей, заведённый
    // через `Вставить` в цикле на РАЗНЫХ структурах, обязан давать один
    // и тот же `Rc<Shape>`, что и прямой литерал с тем же набором —
    // иначе инлайн-кэш на горячий доступ к полю стал бы полиморфным.
    // Наблюдаем это косвенно: `Для Каждого` со смешанными "путём
    // Вставить" и "литералом" структурами по-прежнему читает верно.
    let v = run_src(
        "a = Новый Массив(2);\n\
         a[0] = Новый Структура();\n\
         a[0].Вставить(\"x\", 10);\n\
         a[0].Вставить(\"y\", 20);\n\
         a[1] = Новый Структура(\"x,y\", 100, 200);\n\
         сумма = 0;\n\
         Для Каждого elem Из a Цикл\n\
         сумма = сумма + elem.x;\n\
         КонецЦикла\n\
         Возврат сумма;",
    );
    assert_eq!(v, num("110"));
}

/// Чанк, собранный мимо кодогена, — ровно тот вход, ради которого
/// индексация в `step` возвращает `InvalidBytecode` вместо паники.
fn corrupt_program(instrs: Vec<Instr>) -> Program {
    Program {
        requirements: vec![bsl_bytecode::LibraryRequirement::bsl_rt()],
        function_names: Vec::new(),
        module_vars: Vec::new(),
        module_base: 0,
        chunks: vec![bsl_bytecode::Chunk {
            param_by_val: Vec::new(),
            param_has_default: Vec::new(),
            is_procedure: false,
            touches_objects: false,
            instrs,
            consts: Vec::new(),
            call_arg_modes: Vec::new(),
            exception_ranges: Vec::new(),
            n_params: 0,
            n_locals: 1,
            n_regs: 1,
            local_names: Vec::new(),
            prop_cache: vec![std::cell::RefCell::new(None)],
            method_cache: vec![std::cell::RefCell::new(None)],
            // Пустая разметка = поинструкционное исполнение — ровно
            // тот путь, на котором и проверяется `InvalidBytecode`.
            bundle_len: Vec::new(),
        }],
        names: Vec::new(),
        shapes: Vec::new(),
        top_level_locals: Vec::new(),
    }
}

#[test]
fn corrupt_bytecode_is_an_error_not_a_panic() {
    // Регистр за границей кадра.
    assert!(matches!(
        run_with_dynamic(
            &corrupt_program(vec![Instr::Move { dst: 200, src: 0 }]),
            JitMode::Off
        ),
        Err(RtError::InvalidBytecode(_))
    ));
    // Номер константы за границей таблицы констант.
    assert!(matches!(
        run_with_dynamic(
            &corrupt_program(vec![Instr::LoadConst { dst: 0, k: 42 }]),
            JitMode::Off
        ),
        Err(RtError::InvalidBytecode(_))
    ));
    // Номер вызываемого чанка за границей таблицы функций.
    assert!(matches!(
        run_with_dynamic(
            &corrupt_program(vec![Instr::Call {
                func: 99,
                base: 0,
                arg_modes: 0,
                ret: 0,
            }]),
            JitMode::Off
        ),
        Err(RtError::InvalidBytecode(_))
    ));
    // Номер формы за границей таблицы форм.
    assert!(matches!(
        run_with_dynamic(
            &corrupt_program(vec![Instr::NewStructure {
                dst: 0,
                shape: 7,
                base: 0,
                count: 0,
            }]),
            JitMode::Off
        ),
        Err(RtError::InvalidBytecode(_))
    ));
    // Программа вообще без чанка верхнего уровня.
    let mut empty = corrupt_program(Vec::new());
    empty.chunks.clear();
    assert!(matches!(
        run_with_dynamic(&empty, JitMode::Off),
        Err(RtError::InvalidBytecode(_))
    ));
}

#[test]
fn corrupt_bytecode_inside_a_call_unwinds_to_an_error_not_a_panic() {
    // Ошибка байт-кода внутри вызванного кадра проходит тем же путём
    // размотки, что и обычная `RtError`, и не роняет процесс на
    // `unwind_to_handler`.
    let mut program = corrupt_program(vec![
        Instr::Call {
            func: 1,
            base: 0,
            arg_modes: 0,
            ret: 0,
        },
        Instr::Return { src: Some(0) },
    ]);
    program.chunks[0].call_arg_modes = vec![Vec::new()];
    program.chunks[0].prop_cache = vec![std::cell::RefCell::new(None); 2];
    let mut callee = program.chunks[0].clone();
    callee.instrs = vec![Instr::Move { dst: 250, src: 0 }];
    program.chunks.push(callee);
    // Имя вызываемой функции обязательно: `func` адресует и таблицу имён, и
    // таблицу чанков, и без записи периметр отверг бы образ ещё при
    // связывании — тест перестал бы проверять РАЗМОТКУ, ради которой написан.
    program.function_names = vec!["Вызванная".to_string()];

    assert!(matches!(
        run_with_dynamic(&program, JitMode::Off),
        Err(RtError::InvalidBytecode(_))
    ));
}

#[test]
fn for_each_over_structure_yields_key_value_pairs_in_insertion_order() {
    let v = run_src(
        "с = Новый Структура(\"б,а,в\", 1, 2, 3);\n\
         рез = \"\";\n\
         Для Каждого киз Из с Цикл\n\
         рез = рез + киз.Ключ + Строка(киз.Значение);\n\
         КонецЦикла\n\
         Возврат рез;",
    );
    // Порядок — объявления, а не алфавитный и не хэшевый.
    assert_eq!(v, BslValue::Str(bsl_rt::BslString::from_str("б1а2в3")));
}

#[test]
fn dictionary_structure_preserves_insertion_order_in_for_each() {
    // Больше `MAX_SHAPE_TRANSITIONS` вставок с динамическими именами —
    // структура заведомо ушла в словарный режим (см.
    // `bsl_rt::StructureStorage`), и `Для Каждого` по ней обязан
    // остаться детерминированным и совпасть с порядком вставки.
    let n = bsl_rt::MAX_SHAPE_TRANSITIONS + 10;
    let src = format!(
        "с = Новый Структура;\n\
         Для ном = 1 По {n} Цикл\n\
         с.Вставить(\"Поле\" + Строка(ном), ном);\n\
         КонецЦикла;\n\
         рез = \"\";\n\
         Для Каждого киз Из с Цикл\n\
         рез = рез + киз.Ключ + \"=\" + Строка(киз.Значение) + \";\";\n\
         КонецЦикла\n\
         Возврат рез;"
    );
    let expected: String = (1..=n).map(|i| format!("Поле{i}={i};")).collect();
    assert_eq!(
        run_src(&src),
        BslValue::Str(bsl_rt::BslString::from_str(&expected))
    );
}

#[test]
fn dictionary_structure_still_answers_field_access_and_delete_from_script() {
    let n = bsl_rt::MAX_SHAPE_TRANSITIONS + 10;
    let src = format!(
        "с = Новый Структура;\n\
         Для ном = 1 По {n} Цикл\n\
         с.Вставить(\"Поле\" + Строка(ном), ном);\n\
         КонецЦикла;\n\
         с.Удалить(\"Поле1\");\n\
         с.Вставить(\"Поле2\", 200);\n\
         Возврат с.Количество() * 1000 + с.Поле2 + с.Свойство(\"Поле1\", 7);"
    );
    let expected = (n as i64 - 1) * 1000 + 200 + 7;
    assert_eq!(run_src(&src), num(&expected.to_string()));
}

#[test]
fn map_insert_get_count_and_missing_key_returns_undefined() {
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(\"a\", 1);\n\
         м.Вставить(\"b\", 2);\n\
         Возврат м.Количество();",
    );
    assert_eq!(v, num("2"));

    let v = run_src("м = Новый Соответствие;\nм.Вставить(\"a\", 1);\nВозврат м.Получить(\"a\");");
    assert_eq!(v, num("1"));

    let v = run_src("м = Новый Соответствие;\nВозврат м.Получить(\"нет\");");
    assert_eq!(v, BslValue::Undefined);
}

#[test]
fn map_key_hash_is_scale_independent() {
    // "Хеш числа обязан быть независим от масштаба" — м[1.0] и м[1.00]
    // должны быть ОДНИМ И ТЕМ ЖЕ ключом, а не двумя разными записями.
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(1.0, \"первое\");\n\
         м.Вставить(1.00, \"второе\");\n\
         Возврат м.Количество();",
    );
    assert_eq!(v, num("1"));
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(1.0, \"первое\");\n\
         Возврат м.Получить(1.00);",
    );
    assert_eq!(str_val(&v), "первое");
}

#[test]
fn map_delete_removes_key_and_is_a_no_op_when_missing() {
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(\"a\", 1);\n\
         м.Удалить(\"a\");\n\
         м.Удалить(\"нет\");\n\
         Возврат м.Количество();",
    );
    assert_eq!(v, num("0"));
}

#[test]
fn map_for_each_yields_key_value_pairs_in_insertion_order() {
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(\"a\", 1);\n\
         м.Вставить(\"b\", 2);\n\
         итог = \"\";\n\
         Для Каждого пара Из м Цикл\n\
         итог = итог + пара.Ключ + Строка(пара.Значение);\n\
         КонецЦикла\n\
         Возврат итог;",
    );
    assert_eq!(str_val(&v), "a1b2");
}

#[test]
fn map_clear_resets_count_to_zero() {
    let v = run_src(
        "м = Новый Соответствие;\n\
         м.Вставить(\"a\", 1);\n\
         м.Очистить();\n\
         Возврат м.Количество();",
    );
    assert_eq!(v, num("0"));
}

#[test]
fn arrays_are_reference_types_across_by_reference_calls() {
    // Массив передаётся в процедуру по ссылке (как всё без Знач), и
    // сам массив — ссылочный тип: мутация видна вызывающему в обоих
    // смыслах сразу (тот же тест, что и by_reference, но для объекта).
    let v = run_src(
        "Процедура Заполнить(a)\n\
         a[0] = 42;\n\
         КонецПроцедуры\n\
         b = Новый Массив(1);\n\
         Заполнить(b);\n\
         Возврат b[0];",
    );
    assert_eq!(v, num("42"));
}

#[test]
fn for_each_over_array() {
    let v = run_src(
        "a = Новый Массив(3);\n\
         a[0] = 1;\n\
         a[1] = 2;\n\
         a[2] = 3;\n\
         sum = 0;\n\
         Для Каждого x Из a Цикл\n\
         sum = sum + x;\n\
         КонецЦикла\n\
         Возврат sum;",
    );
    assert_eq!(v, num("6"));
}

#[test]
fn for_each_break_and_continue() {
    let v = run_src(
        "a = Новый Массив(5);\n\
         a[0] = 1;\n a[1] = 2;\n a[2] = 3;\n a[3] = 4;\n a[4] = 5;\n\
         sum = 0;\n\
         Для Каждого x Из a Цикл\n\
         Если x = 2 Тогда\n\
         Продолжить;\n\
         КонецЕсли;\n\
         Если x = 5 Тогда\n\
         Прервать;\n\
         КонецЕсли;\n\
         sum = sum + x;\n\
         КонецЦикла\n\
         Возврат sum;",
    );
    // 1 + 3 + 4 = 8 (2 пропущен, остановились на 5)
    assert_eq!(v, num("8"));
}

#[test]
fn display_of_array_and_structure_matches_measured_platform_strings() {
    let v = run_src("Возврат Новый Массив();");
    assert_eq!(v.to_string(), "Массив");
    let v = run_src("Возврат Новый Структура();");
    assert_eq!(v.to_string(), "Структура");
}

#[test]
fn try_except_catches_internal_runtime_error() {
    let v = run_src(
        "x = 0;\n\
         Попытка\n\
         x = 1 / 0;\n\
         Исключение\n\
         x = 99;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("99"));
}

#[test]
fn code_after_try_runs_normally_when_nothing_is_raised() {
    let v = run_src(
        "x = 0;\n\
         Попытка\n\
         x = 1;\n\
         Исключение\n\
         x = 99;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn raise_with_value_is_caught_and_carries_the_value() {
    let v = run_src(
        "Попытка\n\
         ВызватьИсключение \"беда\";\n\
         Исключение\n\
         Возврат 1;\n\
         КонецПопытки\n\
         Возврат 0;",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn exception_raised_inside_a_called_function_is_caught_by_callers_try() {
    // Попытка оборачивает ВЫЗОВ, а не сам код исключения — исключение
    // должно долететь через границу кадра и быть пойманным снаружи.
    let v = run_src(
        "Функция Взрыв()\n\
         Возврат 1 / 0;\n\
         КонецФункции\n\
         x = 0;\n\
         Попытка\n\
         x = Взрыв();\n\
         Исключение\n\
         x = 42;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("42"));
}

#[test]
fn uncaught_exception_outside_any_try_propagates_as_an_error() {
    let err = run_src_err("x = 1 / 0;");
    assert!(matches!(
        err,
        RtError::Num(bsl_number::NumError::DivideByZero)
    ));
}

#[test]
fn bare_reraise_inside_except_rethrows_caught_value() {
    // Внешняя Попытка должна поймать то же самое исключение, повторно
    // брошенное из внутреннего Исключение через голый ВызватьИсключение.
    let v = run_src(
        "x = 0;\n\
         Попытка\n\
         Попытка\n\
         ВызватьИсключение \"внутренняя\";\n\
         Исключение\n\
         ВызватьИсключение;\n\
         КонецПопытки\n\
         Исключение\n\
         x = 7;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("7"));
}

#[test]
fn nested_try_inner_handler_wins_over_outer() {
    let v = run_src(
        "x = 0;\n\
         Попытка\n\
         Попытка\n\
         x = 1 / 0;\n\
         Исключение\n\
         x = 1;\n\
         КонецПопытки\n\
         Исключение\n\
         x = 2;\n\
         КонецПопытки\n\
         Возврат x;",
    );
    assert_eq!(v, num("1"));
}

#[test]
fn builtin_sqrt_and_pow() {
    let v = run_src("Возврат sqrt(2);");
    assert_eq!(v, num("1.4142135623731"));

    let v = run_src("Возврат Pow(10, 30);");
    assert_eq!(v, num("1000000000000000000000000000000"));
}

#[test]
fn okrugl_rounds_half_up_in_decimal_not_f64() {
    // Окр(2.675, 2) обязан дать 2.68: ближайший f64 к 2.675 чуть
    // меньше самого числа, через f64 получилось бы 2.67.
    let v = run_src("Возврат Окр(2.675, 2);");
    assert_eq!(v, num("2.68"));
    // Второй аргумент необязателен — по умолчанию 0 разрядов.
    let v = run_src("Возврат Окр(2.4);");
    assert_eq!(v, num("2"));
    let v = run_src("Возврат Окр(2.5);");
    assert_eq!(v, num("3"));
}

#[test]
fn okrugl_third_argument_selects_the_rounding_mode() {
    // ВСЁ НИЖЕ ИЗМЕРЕНО на платформе 8.3.27 (см. platform.tsv и якоря
    // NUM.ROUND.* в реестре). Режим 0 — половина К нулю, режим 1 — ОТ
    // нуля; до замера здесь было наоборот, и режим 0 считался
    // половиной к чётному.
    assert_eq!(run_src("Возврат Окр(2.5, 0, 0);"), num("2"));
    assert_eq!(run_src("Возврат Окр(3.5, 0, 0);"), num("3"));
    assert_eq!(run_src("Возврат Окр(-2.5, 0, 0);"), num("-2"));
    assert_eq!(run_src("Возврат Окр(2.5, 0, 1);"), num("3"));
    assert_eq!(run_src("Возврат Окр(3.5, 0, 1);"), num("4"));
    assert_eq!(run_src("Возврат Окр(-2.5, 0, 1);"), num("-3"));

    // Ничья не только на целых: Окр(2.675, 2, ...) — тоже с платформы.
    assert_eq!(run_src("Возврат Окр(2.675, 2, 0);"), num("2.67"));
    assert_eq!(run_src("Возврат Окр(2.675, 2, 1);"), num("2.68"));
    // Мимо ничьей режим ничего не меняет.
    assert_eq!(run_src("Возврат Окр(2.4, 0, 0);"), num("2"));
    assert_eq!(run_src("Возврат Окр(2.6, 0, 0);"), num("3"));

    // Опущенный третий аргумент — это режим 1, а НЕ 0: измерено, что
    // `Окр(2.5)` даёт 3, а `Окр(2.5, 0, 0)` даёт 2.
    assert_eq!(run_src("Возврат Окр(2.5);"), num("3"));
    assert_eq!(
        run_src("Возврат Окр(2.5);"),
        run_src("Возврат Окр(2.5, 0, 1);")
    );
    // И опущенное число разрядов — ноль.
    assert_eq!(run_src("Возврат Окр(2.675, 2);"), num("2.68"));

    // Неизвестный код режима платформа НЕ считает ошибкой и округляет
    // по умолчанию — измерено (`Окр(2.5, 0, 7)` -> 3).
    assert_eq!(run_src("Возврат Окр(2.5, 0, 7);"), num("3"));
}

#[test]
fn cel_truncates_toward_zero_not_half_up() {
    let v = run_src("Возврат Цел(2.9);");
    assert_eq!(v, num("2"));
    let v = run_src("Возврат Цел(-2.9);");
    assert_eq!(v, num("-2"));
}

#[test]
fn skipped_call_argument_uses_declared_default() {
    let v = run_src(
        "Функция Ф(а, б = 100)\n\
         Возврат а + б;\n\
         КонецФункции\n\
         Возврат Ф(1, );",
    );
    assert_eq!(v, num("101"));
}

/// Закрепляет НАШЕ поведение, а не платформенное: 8.3.27 такое объявление
/// вообще не компилирует, умолчанием у неё может быть только литерал (см.
/// `compile_param_defaults` в `bsl-bytecode`). Платформенные случаи лежат
/// в фикстуре `default-args`.
#[test]
fn skipped_call_argument_default_may_reference_earlier_parameter() {
    let v = run_src(
        "Функция Ф(а, б = а + 1, в = 100)\n\
         Возврат а + б + в;\n\
         КонецФункции\n\
         Возврат Ф(1, , 3);",
    );
    // б = а + 1 = 2 (пропущен), в = 3 (передан явно) -> 1 + 2 + 3 = 6.
    assert_eq!(v, num("6"));
}

#[test]
fn skipped_call_argument_falls_back_to_default_when_all_optional_omitted() {
    let v = run_src(
        "Функция Ф(а, б = а + 1, в = 100)\n\
         Возврат а + б + в;\n\
         КонецФункции\n\
         Возврат Ф(1, ,);",
    );
    // б = а + 1 = 2, в = 100 (оба пропущены) -> 1 + 2 + 100 = 103.
    assert_eq!(v, num("103"));
}

#[test]
fn explicit_argument_overrides_default_even_when_declared() {
    let v = run_src(
        "Функция Ф(а, б = 100)\n\
         Возврат а + б;\n\
         КонецФункции\n\
         Возврат Ф(1, 5);",
    );
    assert_eq!(v, num("6"));
}

/// Явно переданное `Неопределено` — это ПЕРЕДАННЫЙ аргумент, и умолчание
/// его не подменяет. Ровно ради этого различия пропуск и живёт отдельно от
/// значения: пока признаком служил элемент `BslValue`, оба случая
/// отличались лишь тем, какой из двух «пустых» вариантов лежит в слоте.
#[test]
fn explicitly_passed_undefined_is_not_a_skipped_argument() {
    let v = run_src(
        "Функция Ф(а, б = 100)\n\
         Возврат б;\n\
         КонецФункции\n\
         Возврат Ф(1, Неопределено);",
    );
    assert_eq!(v, BslValue::Undefined);

    // Та же функция с пропущенной позицией — умолчание.
    let v = run_src(
        "Функция Ф(а, б = 100)\n\
         Возврат б;\n\
         КонецФункции\n\
         Возврат Ф(1, );",
    );
    assert_eq!(v, num("100"));

    // И переменная со значением `Неопределено` — тоже переданный аргумент,
    // а не пропуск: здесь аргумент ещё и голая переменная, то есть режим
    // передачи `ByRefLocal`, а не `Value`.
    let v = run_src(
        "Функция Ф(а, б = 100)\n\
         Возврат б;\n\
         КонецФункции\n\
         з = Неопределено;\n\
         Возврат Ф(1, з);",
    );
    assert_eq!(v, BslValue::Undefined);
}

/// Передача по ссылке рядом с пропущенной позицией: пропуск не сдвигает
/// нумерацию параметров и не мешает соседу остаться алиасом переменной
/// вызывающего.
#[test]
fn a_by_ref_parameter_still_writes_back_next_to_a_skipped_position() {
    let v = run_src(
        "Процедура П(а, б = 100, в)\n\
         в = а + б;\n\
         КонецПроцедуры\n\
         р = 0;\n\
         П(1, , р);\n\
         Возврат р;",
    );
    assert_eq!(v, num("101"));
}

#[test]
fn builtin_sqrt_of_negative_is_a_runtime_error() {
    let err = run_src_err("Возврат sqrt(-1);");
    assert!(matches!(err, RtError::Num(_)));
}

#[test]
fn count_method_call_on_array() {
    let v = run_src("a = Новый Массив(5);\nВозврат a.Count();");
    assert_eq!(v, num("5"));
}

#[test]
fn message_builtin_prints_and_returns_undefined() {
    // Не проверяем stdout здесь — только что вызов не падает и что
    // Message() возвращает Неопределено, как и положено процедуре без
    // Возврат.
    let v = run_src("Message(\"hello\");\nВозврат 1;");
    assert_eq!(v, num("1"));
}

#[test]
fn nbody_smoke_runs_the_real_benchmark_shape_for_a_few_steps() {
    // Уменьшенная копия tests/conformance/fixtures/n-body.bsl: та же
    // структура (Function/EndFunction, Для Каждого, Новый Структура,
    // деление гигантских констант, sqrt, .Count()), но всего несколько
    // шагов Advance вместо 50 миллионов (брифом же и объявленных
    // невыполнимыми что у нас, что в самой 1С) и без Message — просто
    // Возврат энергии для проверки в тесте.
    let src = include_str!("../tests/nbody_smoke.bsl");
    let v = run_src(src);
    let e = match &v {
        BslValue::Number(n) => n.clone(),
        other => panic!("expected Number, got {other:?}"),
    };
    // Энергия системы отрицательна (связанная система) и не должна
    // выродиться в бесконечность/NaN за несколько шагов.
    assert!(e.is_negative(), "energy should stay negative: {e:?}");
}

fn str_val(v: &BslValue) -> String {
    match v {
        BslValue::Str(s) => s.to_string(),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn stroka_groups_by_default_with_nbsp() {
    // Строка(1000.5) -> "1 000,5" (NBSP, не обычный пробел).
    let v = run_src("Возврат Строка(1000.5);");
    assert_eq!(str_val(&v), "1\u{A0}000,5");
}

#[test]
fn format_with_explicit_spec_suppresses_grouping() {
    let v = run_src(r#"Возврат Формат(1000000, "ЧГ=0; ЧРД=.");"#);
    assert_eq!(str_val(&v), "1000000");
}

#[test]
fn format_specifiers_beyond_the_measured_four_reach_the_formatter() {
    // Все ВЫБРАННЫЕ (не измеренные) значения проверены в bsl-format;
    // здесь — что ключи доходят до него через вызов из BSL и что
    // локаль действует на все три типа сразу.
    let v = run_src(r#"Возврат Формат(42, "ЧГ=0; ЧЦ=5; ЧВН=1");"#);
    assert_eq!(str_val(&v), "00042");
    let v = run_src(r#"Возврат Формат(0, "ЧН=пусто");"#);
    assert_eq!(str_val(&v), "пусто");
    let v = run_src(r#"Возврат Формат(1234, "ЧГ=0; ЧРД=.; ЧС=3");"#);
    assert_eq!(str_val(&v), "1.234");
    let v = run_src(r#"Возврат Формат(Ложь, "БЛ=неа");"#);
    assert_eq!(str_val(&v), "неа");
    let v = run_src(r#"Возврат Формат(1234.5, "Л=en");"#);
    assert_eq!(str_val(&v), "1,234.5");
    let v = run_src(r#"Возврат Формат(Дата(2024,1,15), "Л=en; ДФ='ММММ'");"#);
    assert_eq!(str_val(&v), "January");
}

#[test]
fn an_unknown_locale_falls_back_to_russian() {
    // ИЗМЕРЕНО: незнакомый код локали — НЕ ошибка, платформа молча
    // форматирует по-русски. Раньше здесь было исключение.
    let v = run_src(r#"Возврат Формат(1234.5, "Л=zz_ZZ");"#);
    assert_eq!(str_val(&v), "1\u{a0}234,5");
}

#[test]
fn chislo_parses_grouped_string_back_round_trip() {
    let v = run_src("x = Строка(1000000);\nВозврат Число(x);");
    assert_eq!(v, num("1000000"));
}

#[test]
fn stroka_of_boolean_and_undefined_matches_measured_strings() {
    let v = run_src("Возврат Строка(Истина);");
    assert_eq!(str_val(&v), "Да");
    let v = run_src("Возврат Строка(Неопределено);");
    assert_eq!(str_val(&v), "");
}

#[test]
fn string_concatenation_via_plus() {
    let v = run_src(r#"Возврат "Привет, " + "мир!";"#);
    assert_eq!(str_val(&v), "Привет, мир!");
}

#[test]
fn strdlina_counts_utf16_code_units_including_surrogate_pairs() {
    let v = run_src(r#"Возврат СтрДлина("привет");"#);
    assert_eq!(v, num("6"));
    // Эмодзи вне BMP — суррогатная пара, 2 код-юнита UTF-16.
    let v = run_src("Возврат СтрДлина(\"a\u{1F600}b\");");
    assert_eq!(v, num("4"));
}

#[test]
fn left_right_mid_builtins() {
    let v = run_src(r#"Возврат Лев("Привет", 3);"#);
    assert_eq!(str_val(&v), "При");
    let v = run_src(r#"Возврат Прав("Привет", 3);"#);
    assert_eq!(str_val(&v), "вет");
    let v = run_src(r#"Возврат Сред("Привет", 2, 3);"#);
    assert_eq!(str_val(&v), "рив");
}

#[test]
fn upper_lower_trimall_builtins() {
    let v = run_src(r#"Возврат ВРег("привет");"#);
    assert_eq!(str_val(&v), "ПРИВЕТ");
    let v = run_src(r#"Возврат НРег("ПРИВЕТ");"#);
    assert_eq!(str_val(&v), "привет");
    let v = run_src("Возврат СокрЛП(\"  привет  \");");
    assert_eq!(str_val(&v), "привет");
}

#[test]
fn mid_without_length_runs_to_the_end_of_the_string() {
    // Третий аргумент необязателен (`BuiltinFn::arity_range`): резолвер
    // подставляет `Неопределено`, `Сред` читает его как "до конца".
    let v = run_src(r#"Возврат Сред("Привет", 4);"#);
    assert_eq!(str_val(&v), "вет");
}

#[test]
fn strnaiti_returns_a_position_usable_by_sred_without_conversion() {
    let v = run_src(r#"Возврат СтрНайти("абвгд", "вг");"#);
    assert_eq!(v, num("3"));
    let v = run_src(r#"Возврат СтрНайти("абвгд", "яя");"#);
    assert_eq!(v, num("0"));
    // Позиция — в тех же код-юнитах, что считает СтрДлина: строка с
    // суррогатной парой сдвигает всё дальше на 2, не на 1.
    let v = run_src("Возврат СтрНайти(\"a\u{1F600}бв\", \"бв\");");
    assert_eq!(v, num("4"));
    // И этой позицией можно прямо резать строку.
    let v = run_src("Возврат Сред(\"a\u{1F600}бв\", СтрНайти(\"a\u{1F600}бв\", \"бв\"), 2);");
    assert_eq!(str_val(&v), "бв");
}

#[test]
fn strzamenit_replaces_every_occurrence() {
    let v = run_src(r#"Возврат СтрЗаменить("а-б-в", "-", "+");"#);
    assert_eq!(str_val(&v), "а+б+в");
}

#[test]
fn strrazdelit_and_strsoedinit_round_trip_through_an_array() {
    let v = run_src(r#"Возврат СтрРазделить("а,б,в", ",").Количество();"#);
    assert_eq!(v, num("3"));
    let v = run_src(r#"Возврат СтрРазделить("а,б,в", ",")[1];"#);
    assert_eq!(str_val(&v), "б");
    let v = run_src(r#"Возврат СтрСоединить(СтрРазделить("а,,б", ","), ",");"#);
    assert_eq!(str_val(&v), "а,,б");
}

#[test]
fn one_sided_trims_and_line_helpers() {
    let v = run_src("Возврат СокрЛ(\"  а  \") + \"|\";");
    assert_eq!(str_val(&v), "а  |");
    let v = run_src("Возврат \"|\" + СокрП(\"  а  \");");
    assert_eq!(str_val(&v), "|  а");

    // Перевод строки внутри литерала лексер требует оформлять
    // продолжением через `|`, поэтому текст собирается из `Символ(10)` —
    // так же, как это пишут в реальном коде 1С.
    let v = run_src("пс = Символ(10);\nВозврат СтрЧислоСтрок(\"а\" + пс + \"б\" + пс + \"в\");");
    assert_eq!(v, num("3"));
    let v =
        run_src("пс = Символ(10);\nВозврат СтрПолучитьСтроку(\"а\" + пс + \"б\" + пс + \"в\", 2);");
    assert_eq!(str_val(&v), "б");
}

#[test]
fn strshablon_substitutes_positional_values() {
    let v = run_src(r#"Возврат СтрШаблон("%1 и %2", "раз", "два");"#);
    assert_eq!(str_val(&v), "раз и два");
    // Меньше значений, чем позиций в шаблоне — пустая подстановка, не
    // ошибка резолвинга: арность у СтрШаблон вариативная.
    let v = run_src(r#"Возврат СтрШаблон("[%2]", "раз");"#);
    assert_eq!(str_val(&v), "[]");
    // Число подставляется своим строковым представлением.
    let v = run_src(r#"Возврат СтрШаблон("№%1", 7);"#);
    assert_eq!(str_val(&v), "№7");
}

#[test]
fn simvol_and_kodsimvola_round_trip_and_agree_with_the_nbsp_measurement() {
    // Замер с платформы: разделитель групп разрядов — NBSP, код 160.
    let v = run_src(r#"Возврат КодСимвола(Символ(160));"#);
    assert_eq!(v, num("160"));
    // Позиция по умолчанию — первая.
    let v = run_src(r#"Возврат КодСимвола("абв");"#);
    assert_eq!(v, num(&(('а' as u32).to_string())));
    let v = run_src(r#"Возврат КодСимвола("абв", 2);"#);
    assert_eq!(v, num(&(('б' as u32).to_string())));
}

#[test]
fn znachenie_zapolneno_covers_the_measured_cases() {
    // Измеренная часть: Неопределено/Null/пустая строка/ноль — Ложь.
    for src in [
        "Возврат ЗначениеЗаполнено(Неопределено);",
        "Возврат ЗначениеЗаполнено(NULL);",
        "Возврат ЗначениеЗаполнено(\"\");",
        "Возврат ЗначениеЗаполнено(0);",
    ] {
        assert_eq!(run_src(src), BslValue::Boolean(false), "{src}");
    }
    for src in [
        "Возврат ЗначениеЗаполнено(1);",
        "Возврат ЗначениеЗаполнено(\"а\");",
    ] {
        assert_eq!(run_src(src), BslValue::Boolean(true), "{src}");
    }
}

#[test]
fn znachenie_zapolneno_enables_the_short_circuit_guard_idiom() {
    // Ради чего функция и нужна: правый операнд не должен вычисляться,
    // когда левый уже сказал "значения нет" (см. кодоген `И`).
    let v = run_src(
        "х = Неопределено;\n\
         Возврат ЗначениеЗаполнено(х) И х.Поле = 1;",
    );
    assert_eq!(v, BslValue::Boolean(false));
}

#[test]
fn tipznch_compares_equal_across_different_values_of_one_type() {
    let v = run_src("Возврат ТипЗнч(1) = ТипЗнч(2);");
    assert_eq!(v, BslValue::Boolean(true));
    let v = run_src("Возврат ТипЗнч(1) = ТипЗнч(\"а\");");
    assert_eq!(v, BslValue::Boolean(false));
    // Именно та форма, ради которой заведён `Тип(...)`.
    let v = run_src("Возврат ТипЗнч(Новый Массив) = Тип(\"Массив\");");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn type_name_printed_by_stroka_matches_the_value_it_came_from() {
    // `Строка(Новый Массив)` -> "Массив" измерено на платформе; имя
    // типа обязано совпасть с ним, а не быть латинским `Array`.
    let v = run_src("Возврат Строка(ТипЗнч(Новый Массив));");
    assert_eq!(str_val(&v), "Массив");
    let v = run_src("Возврат Строка(ТипЗнч(1));");
    assert_eq!(str_val(&v), "Число");
    let v = run_src("Возврат Строка(ТипЗнч(\"а\"));");
    assert_eq!(str_val(&v), "Строка");
    // Английское имя на входе принимается, на выходе — всё равно русское.
    let v = run_src("Возврат Строка(Тип(\"Structure\"));");
    assert_eq!(str_val(&v), "Структура");
}

#[test]
fn unknown_type_name_is_a_runtime_error_not_undefined() {
    let err = run_src_err("Возврат Тип(\"НетТакогоТипа\");");
    assert!(matches!(err, RtError::UnknownType(_)));
}

// --- Даты -----------------------------------------------------------

fn date_str(src: &str) -> String {
    str_val(&run_src(src))
}

#[test]
fn empty_date_literal_is_the_zero_of_the_epoch() {
    // Ради чего эпоха сдвинута на 0001-01-01: пустая дата — ноль, и
    // ЗначениеЗаполнено на ней даёт Ложь без отдельной константы.
    let v = run_src("Возврат ЗначениеЗаполнено('00010101');");
    assert_eq!(v, BslValue::Boolean(false));
    let v = run_src("Возврат ЗначениеЗаполнено('20240115');");
    assert_eq!(v, BslValue::Boolean(true));
    let v = run_src("Возврат '00010101' = Дата(1, 1, 1);");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn date_literal_and_constructor_agree_in_both_lengths() {
    let v = run_src("Возврат '20240115103000' = Дата(2024, 1, 15, 10, 30, 0);");
    assert_eq!(v, BslValue::Boolean(true));
    // Опущенное время — полночь.
    let v = run_src("Возврат '20240115' = Дата(2024, 1, 15);");
    assert_eq!(v, BslValue::Boolean(true));
    // Строковая форма конструктора.
    let v = run_src("Возврат Дата(\"20240115103000\") = '20240115103000';");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn nonexistent_calendar_literal_is_rejected_at_resolve_time() {
    // 30 февраля проходит лексер (цифры, длина 8), но не календарь —
    // и это ошибка КОМПИЛЯЦИИ, а не рантайма: литерал известен заранее.
    let prog = parse("Возврат '20240230';").unwrap();
    let err = resolve_program(&prog.items).unwrap_err();
    assert!(matches!(err, bsl_sema::SemaError::BadDateLiteral(_)));
}

#[test]
fn date_arithmetic_is_in_seconds_both_ways() {
    // Дата - Дата -> число секунд.
    let v = run_src("Возврат Дата(2024, 1, 15) - Дата(2024, 1, 14);");
    assert_eq!(v, num("86400"));
    // Дата + Число -> дата, сдвинутая на N секунд.
    let v = run_src("Возврат Дата(2024, 1, 14) + 86400 = Дата(2024, 1, 15);");
    assert_eq!(v, BslValue::Boolean(true));
    // Дата - Число -> дата.
    let v = run_src("Возврат Дата(2024, 1, 15) - 86400 = Дата(2024, 1, 14);");
    assert_eq!(v, BslValue::Boolean(true));
    // Тип результата разный у двух форм вычитания — проверяем прямо.
    let v = run_src("Возврат Строка(ТипЗнч(Дата(2024,1,15) - Дата(2024,1,14)));");
    assert_eq!(str_val(&v), "Число");
    let v = run_src("Возврат Строка(ТипЗнч(Дата(2024,1,15) - 1));");
    assert_eq!(str_val(&v), "Дата");
}

#[test]
fn dates_compare_by_moment_in_time() {
    let v = run_src("Возврат Дата(2024, 1, 14) < Дата(2024, 1, 15);");
    assert_eq!(v, BslValue::Boolean(true));
    let v = run_src("Возврат Дата(2024, 1, 15, 10, 0, 0) > Дата(2024, 1, 15, 9, 59, 59);");
    assert_eq!(v, BslValue::Boolean(true));
    let v = run_src("Возврат Дата(2024, 1, 15) = Дата(2024, 1, 15);");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn date_components_are_readable_individually() {
    let src = "д = Дата(2024, 2, 29, 13, 45, 30);\n";
    assert_eq!(run_src(&format!("{src}Возврат Год(д);")), num("2024"));
    assert_eq!(run_src(&format!("{src}Возврат Месяц(д);")), num("2"));
    assert_eq!(run_src(&format!("{src}Возврат День(д);")), num("29"));
    assert_eq!(run_src(&format!("{src}Возврат Час(д);")), num("13"));
    assert_eq!(run_src(&format!("{src}Возврат Минута(д);")), num("45"));
    assert_eq!(run_src(&format!("{src}Возврат Секунда(д);")), num("30"));
}

#[test]
fn period_boundaries_and_weekday_from_script() {
    // 15 января 2024 — понедельник
    // (НЕ ИЗМЕРЕНО(DATE.WEEKDAY_NUMBERING), что пн = 1).
    assert_eq!(run_src("Возврат ДеньНедели(Дата(2024, 1, 15));"), num("1"));
    assert_eq!(
        date_str("Возврат Строка(НачалоДня(Дата(2024, 2, 17, 13, 45, 30)));"),
        "17.02.2024 0:00:00"
    );
    assert_eq!(
        date_str("Возврат Строка(КонецДня(Дата(2024, 2, 17)));"),
        "17.02.2024 23:59:59"
    );
    assert_eq!(
        date_str("Возврат Строка(НачалоМесяца(Дата(2024, 2, 17)));"),
        "01.02.2024 0:00:00"
    );
    // Високосный февраль — 29-е, не 28-е.
    assert_eq!(
        date_str("Возврат Строка(КонецМесяца(Дата(2024, 2, 17)));"),
        "29.02.2024 23:59:59"
    );
    assert_eq!(
        date_str("Возврат Строка(НачалоГода(Дата(2024, 7, 4)));"),
        "01.01.2024 0:00:00"
    );
    assert_eq!(
        date_str("Возврат Строка(КонецГода(Дата(2024, 7, 4)));"),
        "31.12.2024 23:59:59"
    );
    // Среда 17 января -> понедельник 15-го.
    assert_eq!(
        date_str("Возврат Строка(НачалоНедели(Дата(2024, 1, 17)));"),
        "15.01.2024 0:00:00"
    );
}

#[test]
fn add_month_clamps_the_day_rather_than_failing() {
    // 31 января + 1 месяц -> 29 февраля
    // (НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP), см. add_months).
    assert_eq!(
        date_str("Возврат Строка(ДобавитьМесяц(Дата(2024, 1, 31), 1));"),
        "29.02.2024 0:00:00"
    );
    assert_eq!(
        date_str("Возврат Строка(ДобавитьМесяц(Дата(2024, 1, 15), -1));"),
        "15.12.2023 0:00:00"
    );
}

#[test]
fn out_of_range_dates_are_errors_not_silent_wraparound() {
    let err = run_src_err("Возврат Дата(2024, 2, 30);");
    assert!(matches!(err, RtError::DateOutOfRange { .. }));
    let err = run_src_err("Возврат Дата(9999, 12, 31, 23, 59, 59) + 1;");
    assert!(matches!(err, RtError::DateOutOfRange { .. }));
    let err = run_src_err("Возврат Дата(1, 1, 1) - 1;");
    assert!(matches!(err, RtError::DateOutOfRange { .. }));
}

#[test]
fn format_understands_df_and_dlf_keys() {
    let v = run_src("Возврат Формат(Дата(2024, 1, 15), \"ДФ='дд.ММ.гггг'\");");
    assert_eq!(str_val(&v), "15.01.2024");
    // Месяц и минута различаются регистром — обе в одном шаблоне.
    let v = run_src("Возврат Формат(Дата(2024, 1, 15, 9, 5, 0), \"ДФ='ММ/мм'\");");
    assert_eq!(str_val(&v), "01/05");
    let v = run_src("Возврат Формат(Дата(2024, 1, 15), \"ДЛФ=Д\");");
    assert_eq!(str_val(&v), "15.01.2024");
    let v = run_src("Возврат Формат(Дата(2024, 1, 15, 10, 30, 0), \"ДЛФ=В\");");
    assert_eq!(str_val(&v), "10:30:00");
    // Числовые ключи на дате не мешают и наоборот.
    let v = run_src("Возврат Формат(1000, \"ЧГ=0; ДФ='гггг'\");");
    assert_eq!(str_val(&v), "1000");
}

#[test]
fn tipznch_of_a_date_is_the_localized_type_name() {
    let v = run_src("Возврат Строка(ТипЗнч(Дата(2024, 1, 15)));");
    assert_eq!(str_val(&v), "Дата");
    let v = run_src("Возврат ТипЗнч('20240115') = Тип(\"Дата\");");
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn current_date_lands_inside_the_supported_range() {
    // Точное значение не проверить (оно зависит от часов машины), но
    // год обязан быть правдоподобным — это ловит перепутанную эпоху,
    // ради которой всё и затевалось.
    let v = run_src("Возврат Год(ТекущаяДата());");
    let year = match v {
        BslValue::Number(n) => n.to_i64_exact().unwrap(),
        other => panic!("ожидалось число, получено {other:?}"),
    };
    assert!((2020..=2200).contains(&year), "получен год {year}");
}

#[test]
fn current_universal_date_in_milliseconds_runs_benchmark_style_script() {
    let v = run_src(
        "Процедура CalcНаСервере()\n\
         sum = 0.0;\n\
         flip = -1.0;\n\
         Для i = 1 По 100 Цикл\n\
         flip = -flip;\n\
         sum = sum + flip / (2 * i - 1);\n\
         КонецЦикла;\n\
         КонецПроцедуры\n\
         Т1 = ТекущаяУниверсальнаяДатаВМиллисекундах();\n\
         CalcНаСервере();\n\
         Возврат ТекущаяУниверсальнаяДатаВМиллисекундах() - Т1;",
    );
    let elapsed = match v {
        BslValue::Number(n) => n.to_i64_exact().unwrap(),
        other => panic!("ожидалось число миллисекунд, получено {other:?}"),
    };
    assert!(elapsed >= 0, "часы пошли назад: {elapsed} мс");
}

#[test]
fn string_comparison_is_lexicographic() {
    let v = run_src(r#"Возврат "а" < "б";"#);
    assert_eq!(v, BslValue::Boolean(true));
    let v = run_src(r#"Возврат "яблоко" = "яблоко";"#);
    assert_eq!(v, BslValue::Boolean(true));
}

/// Число слева тянет правый операнд к ЧИСЛУ, поэтому нечисловая строка
/// — ошибка (замер `CONCAT.LEFT.INT`). Ошибка при этом приходит от
/// разбора числа, а не от несоответствия типов: строка сама по себе
/// операнду арифметики не противопоказана, противопоказано её
/// СОДЕРЖИМОЕ.
#[test]
fn a_non_numeric_string_in_arithmetic_is_an_error() {
    let err = run_src_err(r#"Возврат 1 + "a";"#);
    assert!(matches!(err, RtError::Num(_)), "{err:?}");
    // А числовая строка на том же месте проходит.
    assert_eq!(
        run_src(r#"Возврат 1 + "2";"#),
        BslValue::Number(bsl_number::BslNumber::from_i64(3))
    );
}

#[test]
fn array_add_delete_clear_methods() {
    let v = run_src(
        "a = Новый Массив();\n\
         a.Добавить(1);\n\
         a.Добавить(2);\n\
         a.Добавить(3);\n\
         a.Удалить(1);\n\
         Возврат a.Количество();",
    );
    assert_eq!(v, num("2"));

    let v = run_src(
        "a = Новый Массив();\n\
         a.Добавить(1);\n\
         a.Очистить();\n\
         Возврат a.Количество();",
    );
    assert_eq!(v, num("0"));
}

#[test]
fn value_table_add_column_add_row_and_field_access() {
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"Имя\");\n\
         т.Колонки.Добавить(\"Возраст\");\n\
         строка = т.Добавить();\n\
         строка.Имя = \"Аня\";\n\
         строка.Возраст = 30;\n\
         Возврат строка.Возраст;",
    );
    assert_eq!(v, num("30"));
}

#[test]
fn value_table_row_count_and_indexing() {
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         т.Добавить();\n\
         т.Добавить();\n\
         т.Добавить();\n\
         т[1].x = 42;\n\
         Возврат т.Количество() * 100 + т[1].x;",
    );
    assert_eq!(v, num("342"));
}

#[test]
fn value_table_for_each_over_rows() {
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         а = т.Добавить(); а.x = 1;\n\
         б = т.Добавить(); б.x = 2;\n\
         в = т.Добавить(); в.x = 3;\n\
         сумма = 0;\n\
         Для Каждого строка Из т Цикл\n\
         сумма = сумма + строка.x;\n\
         КонецЦикла\n\
         Возврат сумма;",
    );
    assert_eq!(v, num("6"));
}

// --- ТаблицаЗначений, волна 2 ---------------------------------------

/// Таблица с колонками `имя`/`цена`/`кол` и тремя строками — общая
/// затравка для тестов поиска/сортировки/итога.
const GOODS: &str = "т = Новый ТаблицаЗначений();\n\
     т.Колонки.Добавить(\"имя\");\n\
     т.Колонки.Добавить(\"цена\");\n\
     с1 = т.Добавить(); с1.имя = \"груша\"; с1.цена = 30;\n\
     с2 = т.Добавить(); с2.имя = \"яблоко\"; с2.цена = 10;\n\
     с3 = т.Добавить(); с3.имя = \"дыня\"; с3.цена = 20;\n";

#[test]
fn table_find_returns_a_row_or_undefined() {
    let v = run_src(&format!("{GOODS}Возврат т.Найти(\"дыня\").цена;"));
    assert_eq!(v, num("20"));
    // Не найдено — Неопределено, а не ошибка: это штатная проверка.
    let v = run_src(&format!("{GOODS}Возврат т.Найти(\"нет такого\");"));
    assert_eq!(v, BslValue::Undefined);
    // С явным списком колонок ищем только в них.
    let v = run_src(&format!("{GOODS}Возврат т.Найти(20, \"цена\").имя;"));
    assert_eq!(str_val(&v), "дыня");
    let v = run_src(&format!("{GOODS}Возврат т.Найти(20, \"имя\");"));
    assert_eq!(v, BslValue::Undefined);
    // Опечатка в имени колонки — ошибка, а не пустой результат.
    let err = run_src_err(&format!("{GOODS}Возврат т.Найти(20, \"опечатка\");"));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_find_rows_matches_every_field_of_the_search_structure() {
    let src = format!(
        "{GOODS}с4 = т.Добавить(); с4.имя = \"дыня\"; с4.цена = 99;\n\
         Возврат т.НайтиСтроки(Новый Структура(\"имя\", \"дыня\")).Количество();"
    );
    assert_eq!(run_src(&src), num("2"));

    // Два поля — оба обязаны совпасть.
    let src = format!(
        "{GOODS}с4 = т.Добавить(); с4.имя = \"дыня\"; с4.цена = 99;\n\
         Возврат т.НайтиСтроки(Новый Структура(\"имя,цена\", \"дыня\", 99)).Количество();"
    );
    assert_eq!(run_src(&src), num("1"));

    // Ничего не совпало — пустой массив, не Неопределено.
    let src =
        format!("{GOODS}Возврат т.НайтиСтроки(Новый Структура(\"имя\", \"нет\")).Количество();");
    assert_eq!(run_src(&src), num("0"));

    // Поле, которого нет среди колонок, — ошибка.
    let err = run_src_err(&format!(
        "{GOODS}Возврат т.НайтиСтроки(Новый Структура(\"опечатка\", 1));"
    ));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_sort_orders_ascending_and_descending() {
    let read = "рез = \"\";\n\
         Для Каждого с Из т Цикл рез = рез + с.имя + \";\"; КонецЦикла;\n\
         Возврат рез;";
    let v = run_src(&format!("{GOODS}т.Сортировать(\"цена\");\n{read}"));
    assert_eq!(str_val(&v), "яблоко;дыня;груша;");
    let v = run_src(&format!("{GOODS}т.Сортировать(\"цена Убыв\");\n{read}"));
    assert_eq!(str_val(&v), "груша;дыня;яблоко;");
    // Направление по умолчанию — возрастание, `Возр` его лишь называет.
    let v = run_src(&format!("{GOODS}т.Сортировать(\"имя Возр\");\n{read}"));
    assert_eq!(str_val(&v), "груша;дыня;яблоко;");
}

#[test]
fn table_sort_is_stable_and_uses_the_second_key_only_on_ties() {
    let src = "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"группа\");\n\
         т.Колонки.Добавить(\"ном\");\n\
         а = т.Добавить(); а.группа = 1; а.ном = 1;\n\
         б = т.Добавить(); б.группа = 2; б.ном = 2;\n\
         в = т.Добавить(); в.группа = 1; в.ном = 3;\n\
         г = т.Добавить(); г.группа = 2; г.ном = 4;\n";
    let read = "рез = \"\";\n\
         Для Каждого с Из т Цикл рез = рез + Строка(с.ном); КонецЦикла;\n\
         Возврат рез;";
    // Сортировка по одной колонке: внутри группы — исходный порядок.
    let v = run_src(&format!("{src}т.Сортировать(\"группа\");\n{read}"));
    assert_eq!(str_val(&v), "1324");
    // Второй ключ работает только на равенстве первого.
    let v = run_src(&format!(
        "{src}т.Сортировать(\"группа, ном Убыв\");\n{read}"
    ));
    assert_eq!(str_val(&v), "3142");
}

#[test]
fn table_sort_keeps_live_row_objects_pointing_at_their_own_row() {
    // Инвариант идентичности: строка держит row_id, а сортировка
    // переставляет физические позиции — объект, взятый ДО сортировки,
    // обязан остаться той же строкой.
    let v = run_src(&format!(
        "{GOODS}т.Сортировать(\"цена\");\n\
         Возврат с1.имя + \"=\" + Строка(с1.цена);"
    ));
    assert_eq!(str_val(&v), "груша=30");
    // И запись через него попадает в ту же строку, а не в чужую.
    let v = run_src(&format!(
        "{GOODS}т.Сортировать(\"цена\");\n\
         с1.цена = 99;\n\
         Возврат т.Найти(\"груша\").цена;"
    ));
    assert_eq!(v, num("99"));
}

#[test]
fn table_sort_rejects_an_unknown_column_instead_of_silently_doing_nothing() {
    let err = run_src_err(&format!("{GOODS}т.Сортировать(\"опечатка\");"));
    assert!(matches!(err, RtError::UnknownColumn(_)));
    // Неизвестное направление — тоже ошибка.
    let err = run_src_err(&format!("{GOODS}т.Сортировать(\"цена Криво\");"));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_total_sums_the_column() {
    let v = run_src(&format!("{GOODS}Возврат т.Итог(\"цена\");"));
    assert_eq!(v, num("60"));
    // Нечисловые значения ИГНОРИРУЮТСЯ
    // (НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC) — см.
    // `ValueTableData::total`): колонка из одного текста даёт 0.
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"к\");\n\
         а = т.Добавить(); а.к = \"текст\";\n\
         б = т.Добавить(); б.к = 5;\n\
         Возврат т.Итог(\"к\");",
    );
    assert_eq!(v, num("5"));
    let err = run_src_err(&format!("{GOODS}Возврат т.Итог(\"опечатка\");"));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_sort_of_strings_matches_the_measured_collation() {
    // ИЗМЕРЕНО на 8.3.27 (якорь TABLE.SORT.COLLATION): регистр не делит
    // слова на две группы, `ё` идёт как `е`, а при совпадении всего
    // остального строчная встаёт ПЕРЕД прописной.
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"с\");\n\
         Для Каждого з Из СтрРазделить(\"яблоко,Яблоко,ёлка,Ель,zebra,Апельсин,10,2\", \",\") Цикл\n\
         т.Добавить().с = з;\n\
         КонецЦикла;\n\
         т.Сортировать(\"с\");\n\
         рез = \"\";\n\
         Для Каждого с Из т Цикл рез = рез + с.с + \";\"; КонецЦикла;\n\
         Возврат рез;",
    );
    assert_eq!(str_val(&v), "10;2;zebra;Апельсин;ёлка;Ель;яблоко;Яблоко;");
}

#[test]
fn value_table_row_identity_survives_deleting_a_different_row() {
    // Строка держит row_id, не физическую позицию — удаление строки 0
    // не должно сломать ранее полученную ссылку на строку 1.
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         а = т.Добавить(); а.x = 10;\n\
         б = т.Добавить(); б.x = 20;\n\
         т.Удалить(0);\n\
         Возврат б.x;",
    );
    assert_eq!(v, num("20"));
}

#[test]
fn value_table_accessing_deleted_row_is_an_error() {
    let err = run_src_err(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         а = т.Добавить(); а.x = 10;\n\
         т.Удалить(0);\n\
         Возврат а.x;",
    );
    assert!(matches!(err, RtError::RowInvalidated));
}

#[test]
fn value_table_unknown_column_is_an_error() {
    let err = run_src_err(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         строка = т.Добавить();\n\
         Возврат строка.y;",
    );
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

// --- ТаблицаЗначений, волна 3 ----------------------------------------

#[test]
fn table_copy_makes_an_independent_table() {
    // Копия — ДРУГАЯ таблица: правка её ячейки не видна в оригинале.
    let v = run_src(&format!(
        "{GOODS}к = т.Скопировать();\n\
         к[0].цена = 999;\n\
         Возврат т[0].цена;"
    ));
    assert_eq!(v, num("30"));
    let v = run_src(&format!("{GOODS}Возврат т.Скопировать().Количество();"));
    assert_eq!(v, num("3"));
}

#[test]
fn table_copy_takes_only_the_listed_rows_and_columns() {
    let v = run_src(&format!(
        "{GOODS}строки = Новый Массив;\n\
         строки.Добавить(с3);\n\
         строки.Добавить(с1);\n\
         к = т.Скопировать(строки, \"имя\");\n\
         Возврат к[0].имя + \",\" + к[1].имя + \",\" + Строка(к.Колонки.Количество());"
    ));
    // Порядок строк копии — порядок МАССИВА, а не таблицы.
    assert_eq!(str_val(&v), "дыня,груша,1");
    // Колонки, не попавшей в список, в копии нет.
    let err = run_src_err(&format!(
        "{GOODS}к = т.Скопировать(Неопределено, \"имя\");\n\
         Возврат к[0].цена;"
    ));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_difference_function_runs_end_to_end() {
    let v = run_src(
        r#"
Функция РазницаТаблицЗначений(Таблица0, Таблица1, Измерения) Экспорт
ВсеКолонки = "";
Для Каждого Колонка Из Таблица1.Колонки Цикл
    ВсеКолонки = ВсеКолонки + ", " + Колонка.Имя;
КонецЦикла;
ВсеКолонки = Сред(ВсеКолонки, 2);
Таблица = Таблица1.Скопировать();
Таблица.Колонки.Добавить("Знак", Новый ОписаниеТипов("Число"));
Таблица.ЗаполнитьЗначения(1, "Знак");
Для Каждого Строка Из Таблица0 Цикл
    ЗаполнитьЗначенияСвойств(Таблица.Добавить(), Строка);
КонецЦикла;
Таблица.Колонки.Добавить("Счет");
Таблица.ЗаполнитьЗначения(1, "Счет");
Таблица.Свернуть(ВсеКолонки, "Знак, Счет");
Ответ = Таблица.Скопировать(Новый Структура("Счет", 1), ВсеКолонки + ", Знак");
Если ЗначениеЗаполнено(Измерения) Тогда
    Ответ.Сортировать(Измерения, Новый СравнениеЗначений);
КонецЕсли;
Возврат Ответ;
КонецФункции

Т0 = Новый ТаблицаЗначений;
Т0.Колонки.Добавить("Ключ"); Т0.Колонки.Добавить("Значение");
С = Т0.Добавить(); С.Ключ = "только0"; С.Значение = 9;
С = Т0.Добавить(); С.Ключ = "общая"; С.Значение = 2;
Т1 = Новый ТаблицаЗначений;
Т1.Колонки.Добавить("Ключ"); Т1.Колонки.Добавить("Значение");
С = Т1.Добавить(); С.Ключ = "общая"; С.Значение = 2;
С = Т1.Добавить(); С.Ключ = "только1"; С.Значение = 8;
Ответ = РазницаТаблицЗначений(Т0, Т1, "Ключ");
Если Ответ[0].Ключ <> "только0" Или Ответ[1].Ключ <> "только1" Тогда
Возврат -1;
КонецЕсли;
Возврат Ответ.Количество() * 100 + Ответ[0].Знак * 10 + Ответ[1].Знак;
"#,
    );
    // Строка из первой таблицы получает знак 0, из второй — знак 1.
    assert_eq!(v, num("201"));
}

#[test]
fn table_column_exposes_name_and_type_description() {
    let v = run_src(
        "т = Новый ТаблицаЗначений;\n\
         описание = Новый ОписаниеТипов(\"Число\");\n\
         т.Колонки.Добавить(\"Количество\", описание);\n\
         Для Каждого колонка Из т.Колонки Цикл\n\
             Возврат колонка.Имя = \"Количество\"\n\
                 И ТипЗнч(колонка.ТипЗначения) = Тип(\"ОписаниеТипов\")\n\
                 И ТипЗнч(Новый СравнениеЗначений) = Тип(\"СравнениеЗначений\");\n\
         КонецЦикла;",
    );
    assert_eq!(v, BslValue::Boolean(true));
}

#[test]
fn table_copy_columns_keeps_the_structure_and_drops_the_rows() {
    let v = run_src(&format!(
        "{GOODS}к = т.СкопироватьКолонки();\n\
         Возврат Строка(к.Количество()) + \",\" + Строка(к.Колонки.Количество());"
    ));
    assert_eq!(str_val(&v), "0,2");
    // Строку в пустую копию добавить можно — колонки на месте.
    let v = run_src(&format!(
        "{GOODS}к = т.СкопироватьКолонки(\"цена\");\n\
         к.Добавить().цена = 7;\n\
         Возврат к[0].цена;"
    ));
    assert_eq!(v, num("7"));
}

#[test]
fn table_unload_and_load_column_round_trip() {
    let v = run_src(&format!(
        "{GOODS}м = т.ВыгрузитьКолонку(\"цена\");\n\
         Возврат Строка(м.Количество()) + \",\" + Строка(м[0]) + \",\" + Строка(м[2]);"
    ));
    assert_eq!(str_val(&v), "3,30,20");

    let v = run_src(&format!(
        "{GOODS}м = Новый Массив;\n\
         м.Добавить(1); м.Добавить(2); м.Добавить(3);\n\
         т.ЗагрузитьКолонку(м, \"цена\");\n\
         Возврат т.Итог(\"цена\");"
    ));
    assert_eq!(v, num("6"));
}

#[test]
fn table_load_column_ignores_the_length_mismatch() {
    // НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH): фиксируем
    // ВЫБРАННОЕ — короткий массив меняет только начало колонки, длинный
    // не добавляет строк, число строк не меняется ни там, ни там.
    let v = run_src(&format!(
        "{GOODS}м = Новый Массив;\n\
         м.Добавить(1);\n\
         т.ЗагрузитьКолонку(м, \"цена\");\n\
         Возврат Строка(т.Количество()) + \",\" + Строка(т[0].цена) + \",\" + Строка(т[1].цена);"
    ));
    assert_eq!(str_val(&v), "3,1,10");

    let v = run_src(&format!(
        "{GOODS}м = Новый Массив;\n\
         Для i = 1 По 10 Цикл м.Добавить(i); КонецЦикла;\n\
         т.ЗагрузитьКолонку(м, \"цена\");\n\
         Возврат т.Количество();"
    ));
    assert_eq!(v, num("3"));
}

#[test]
fn table_move_keeps_live_rows_valid() {
    // Инвариант 12: сдвиг переставляет строку, но живой объект строки
    // продолжает указывать на СВОИ данные, а не на чужие.
    let v = run_src(&format!(
        "{GOODS}т.Сдвинуть(с1, 2);\n\
         Возврат т[0].имя + \",\" + т[2].имя + \",\" + с1.имя;"
    ));
    assert_eq!(str_val(&v), "яблоко,груша,груша");
    // Индекс отражает новую позицию.
    let v = run_src(&format!(
        "{GOODS}т.Сдвинуть(0, 1);\n\
         Возврат т.Индекс(с1);"
    ));
    assert_eq!(v, num("1"));
}

#[test]
fn table_move_past_the_edge_is_an_error() {
    // НЕ ИЗМЕРЕНО(TABLE.MOVE.OUT_OF_RANGE): взята ошибка, не зажатие.
    let err = run_src_err(&format!("{GOODS}т.Сдвинуть(с1, -1);"));
    assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
    let err = run_src_err(&format!("{GOODS}т.Сдвинуть(2, 1);"));
    assert!(matches!(err, RtError::IndexOutOfBounds { .. }));
}

#[test]
fn table_index_of_a_deleted_row_is_an_error() {
    let v = run_src(&format!("{GOODS}Возврат т.Индекс(с3);"));
    assert_eq!(v, num("2"));
    let err = run_src_err(&format!("{GOODS}т.Удалить(2);\nВозврат т.Индекс(с3);"));
    assert!(matches!(err, RtError::RowInvalidated));
    // Строка ЧУЖОЙ таблицы — ошибка метода, а не молчаливое «не нашли».
    let err = run_src_err(&format!(
        "{GOODS}другая = Новый ТаблицаЗначений();\n\
         другая.Колонки.Добавить(\"x\");\n\
         чужая = другая.Добавить();\n\
         Возврат т.Индекс(чужая);"
    ));
    assert!(matches!(err, RtError::MethodNotApplicable { .. }));
}

const COLLAPSE: &str = "т = Новый ТаблицаЗначений();\n\
     т.Колонки.Добавить(\"группа\");\n\
     т.Колонки.Добавить(\"сумма\");\n\
     т.Колонки.Добавить(\"прочее\");\n\
     а = т.Добавить(); а.группа = \"б\"; а.сумма = 10; а.прочее = \"x\";\n\
     б = т.Добавить(); б.группа = \"а\"; б.сумма = 1; б.прочее = \"y\";\n\
     в = т.Добавить(); в.группа = \"б\"; в.сумма = 20; в.прочее = \"z\";\n";

#[test]
fn table_collapse_groups_and_sums() {
    let v = run_src(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
         рез = \"\";\n\
         Для Каждого с Из т Цикл рез = рез + с.группа + \"=\" + Строка(с.сумма) + \";\"; КонецЦикла;\n\
         Возврат рез;"
    ));
    // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.ROW_ORDER): порядок ПЕРВОГО ВХОЖДЕНИЯ,
    // поэтому «б» впереди «а», хотя по ключу было бы наоборот.
    assert_eq!(str_val(&v), "б=30;а=1;");
}

#[test]
fn table_collapse_drops_the_columns_outside_both_lists() {
    // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.OTHER_COLUMNS): фиксируем ВЫБРАННОЕ —
    // колонка, не попавшая ни в группировку, ни в суммирование,
    // исчезает вместе со своими значениями.
    let v = run_src(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
         Возврат т.Колонки.Количество();"
    ));
    assert_eq!(v, num("2"));
    let err = run_src_err(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
         Возврат т[0].прочее;"
    ));
    assert!(matches!(err, RtError::UnknownColumn(_)));
}

#[test]
fn table_collapse_without_summing_columns_leaves_unique_keys() {
    let v = run_src(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\");\n\
         Возврат Строка(т.Количество()) + \",\" + Строка(т.Колонки.Количество());"
    ));
    assert_eq!(str_val(&v), "2,1");
}

#[test]
fn table_collapse_ignores_non_numeric_values_when_summing() {
    // НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.NON_NUMERIC): то же решение, что у
    // `Итог` — нечисловое значение просто не входит в сумму.
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"г\");\n\
         т.Колонки.Добавить(\"с\");\n\
         а = т.Добавить(); а.г = \"к\"; а.с = 5;\n\
         б = т.Добавить(); б.г = \"к\"; б.с = \"текст\";\n\
         т.Свернуть(\"г\", \"с\");\n\
         Возврат т[0].с;",
    );
    assert_eq!(v, num("5"));
}

#[test]
fn table_collapse_keeps_the_first_row_of_each_group_alive() {
    // Свёрнутая строка сохраняет row_id ПЕРВОЙ строки группы, поэтому
    // взятый до свёртки объект продолжает работать; строки, слитые в
    // неё, ведут себя как удалённые.
    let v = run_src(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
         Возврат а.сумма;"
    ));
    assert_eq!(v, num("30"));
    let err = run_src_err(&format!(
        "{COLLAPSE}т.Свернуть(\"группа\", \"сумма\");\n\
         Возврат в.сумма;"
    ));
    assert!(matches!(err, RtError::RowInvalidated));
}

#[test]
fn table_wave3_methods_reject_unknown_columns() {
    for src in [
        "Возврат т.ВыгрузитьКолонку(\"опечатка\");",
        "т.Свернуть(\"опечатка\");",
        "к = т.Скопировать(Неопределено, \"опечатка\");",
    ] {
        let err = run_src_err(&format!("{GOODS}{src}"));
        assert!(
            matches!(err, RtError::UnknownColumn(_)),
            "ожидалась UnknownColumn на {src}, получено {err:?}"
        );
    }
}

#[test]
fn value_table_clear_resets_row_count() {
    let v = run_src(
        "т = Новый ТаблицаЗначений();\n\
         т.Колонки.Добавить(\"x\");\n\
         т.Добавить();\n\
         т.Добавить();\n\
         т.Очистить();\n\
         Возврат т.Количество();",
    );
    assert_eq!(v, num("0"));
}

#[test]
fn vychislit_evaluates_an_expression() {
    let v = run_src(r#"Возврат Вычислить("2+2");"#);
    assert_eq!(v, num("4"));
}

#[test]
fn vychislit_sees_existing_top_level_variables() {
    let v = run_src("x = 10;\nВозврат Вычислить(\"x * 2\");");
    assert_eq!(v, num("20"));
}

#[test]
fn vypolnit_mutates_an_existing_top_level_variable() {
    let v = run_src("x = 1;\nВыполнить(\"x = 5\");\nВозврат x;");
    assert_eq!(v, num("5"));
}

#[test]
fn vypolnit_can_run_control_flow_and_read_it_back() {
    // Внутри строкового литерала BSL перенос строки без `|` — ошибка
    // лексера (см. multiline_string_requires_continuation_bar в
    // bsl-syntax), поэтому фрагмент для Выполнить пишем в одну строку.
    let v = run_src(
        r#"sum = 0;
Выполнить("Для i = 1 По 5 Цикл sum = sum + i; КонецЦикла");
Возврат sum;"#,
    );
    assert_eq!(v, num("15"));
}

#[test]
fn vypolnit_new_local_does_not_leak_to_enclosing_scope() {
    // временная переменная снипета не должна пережить вызов и не
    // должна сломать окружающий скрипт — просто исчезает.
    let v = run_src(
        "x = 1;\n\
         Выполнить(\"временная = 99; x = x + временная;\");\n\
         Возврат x;",
    );
    assert_eq!(v, num("100"));
}

#[test]
fn vypolnit_field_access_on_structure_created_by_static_code() {
    // Критическая проверка: интернер имён полей фрагмента засеян из
    // program.names, поэтому NameId для "y" совпадает с тем, что уже
    // использует статически созданная структура.
    let v = run_src(
        "s = Новый Структура(\"x,y\", 1, 2);\n\
         Выполнить(\"s.y = s.y + 100\");\n\
         Возврат s.y;",
    );
    assert_eq!(v, num("102"));
}

#[test]
fn vypolnit_can_construct_a_new_structure_inside_the_snippet_itself() {
    // Регрессия: compile_snippet раньше не возвращал СВОЮ (локальную)
    // таблицу форм, и запуск шёл с чужим (внешним) списком форм —
    // NewStructure падал по индексу за границами, как только фрагмент
    // сам создавал структуру (а не просто читал уже существующую).
    // `""` внутри BSL-строкового литерала — экранирование кавычки
    // (доубление, не бэкслеш) для вложенного списка полей "a,b".
    let v = run_src("Возврат Вычислить(\"Новый Структура(\"\"a,b\"\", 1, 2).b\");");
    assert_eq!(v, num("2"));
}

#[test]
fn vychislit_can_construct_and_use_new_values() {
    let v = run_src(r#"Возврат Вычислить("Новый Массив(3).Count()");"#);
    assert_eq!(v, num("3"));
}

#[test]
fn vychislit_reads_a_property_whose_name_never_appears_in_static_code() {
    // Регрессия, тот же класс бага, что и у
    // `vypolnit_can_construct_a_new_structure_inside_the_snippet_itself`,
    // только про `NameId`, а не про `Shape`: `compile_dynamic_snippet`
    // отбрасывал СОБСТВЕННУЮ (расширенную) таблицу имён фрагмента и
    // запускал его поверх `program.names` ОСНОВНОЙ программы. Пока имя
    // поля уже встречалось где-то статически, `NameId` совпадал
    // случайно; для имени, впервые встреченного ТОЛЬКО внутри текста
    // `Вычислить`/`Выполнить`, `NameId` указывал за пределы этой
    // таблицы, и `GetProp` падал с «идентификатор имени вне таблицы
    // имён программы». Колонка строки таблицы значений резолвится
    // СТРОКОЙ по таблице имён (не через `Shape`), поэтому падает
    // именно на пути, который был сломан; имя колонки в статическом
    // коде — только строковый ЛИТЕРАЛ, в таблицу имён он не попадает.
    let v = run_src(
        "т = Новый ТаблицаЗначений;\n\
         т.Колонки.Добавить(\"тайное\");\n\
         с = т.Добавить();\n\
         Выполнить(\"с.тайное = 5\");\n\
         Возврат Вычислить(\"с.тайное\");",
    );
    assert_eq!(v, num("5"));
}

#[test]
fn dynamic_code_with_syntax_error_is_a_dynamic_error() {
    let err = run_src_err(r#"Выполнить("x = ");"#);
    assert!(matches!(err, RtError::DynamicError(_)));
}

#[test]
fn dynamic_code_requires_a_string_argument() {
    let err = run_src_err("Выполнить(1);");
    assert!(matches!(err, RtError::TypeError { .. }));
}

#[test]
fn vypolnit_inside_a_function_sees_and_changes_its_locals() {
    // Раньше это была `DynamicNotAtTopLevel`. Теперь чанк функции,
    // помеченной `uses_dynamic`, несёт таблицу «имя -> слот», и
    // фрагмент работает в области видимости ЭТОЙ функции.
    let v = run_src(
        "Функция Ф()\n\
         х = 1;\n\
         Выполнить(\"х = х + 41\");\n\
         Возврат х;\n\
         КонецФункции\n\
         Возврат Ф();",
    );
    assert_eq!(v, num("42"));
}

#[test]
fn vychislit_inside_a_function_reads_its_locals() {
    let v = run_src(
        "Функция Ф()\n\
         а = 6;\n\
         б = 7;\n\
         Возврат Вычислить(\"а * б\");\n\
         КонецФункции\n\
         Возврат Ф();",
    );
    assert_eq!(v, num("42"));
}

#[test]
fn dynamic_scope_is_the_frame_not_the_top_level() {
    // У функции и у верхнего уровня переменные с ОДНИМ именем — разные.
    // Фрагмент внутри функции обязан видеть её `х`, а не внешний.
    let v = run_src(
        "Функция Ф()\n\
         х = 5;\n\
         Возврат Вычислить(\"х\");\n\
         КонецФункции\n\
         х = 100;\n\
         Возврат Ф() * 1000 + х;",
    );
    assert_eq!(v, num("5100"));
}

#[test]
fn dynamic_code_sees_parameters_including_by_reference_ones() {
    // Параметр по значению виден фрагменту как обычный слот.
    let v = run_src(
        "Функция Ф(Знач а)\n\
         Возврат Вычислить(\"а * 2\");\n\
         КонецФункции\n\
         Возврат Ф(21);",
    );
    assert_eq!(v, num("42"));

    // Параметр БЕЗ `Знач` — алиас на слот вызывающего (см.
    // `Frame::reg_index`): запись из фрагмента обязана быть видна
    // снаружи так же, как запись из обычного кода функции.
    let v = run_src(
        "Процедура П(а)\n\
         Выполнить(\"а = а + 1\");\n\
         КонецПроцедуры\n\
         х = 41;\n\
         П(х);\n\
         Возврат х;",
    );
    assert_eq!(v, num("42"));
}

#[test]
fn dynamic_code_inside_a_loop_reuses_the_compiled_chunk() {
    // Кэш фрагментов — оптимизация, наблюдаемая только по времени, но
    // проверить можно то, что она не ломает семантику: одна и та же
    // строка, исполненная много раз, каждый раз работает с текущим
    // состоянием кадра, а не с запомненным при компиляции.
    let v = run_src(
        "сумма = 0;\n\
         Для ном = 1 По 5 Цикл\n\
         Выполнить(\"сумма = сумма + ном\");\n\
         КонецЦикла;\n\
         Возврат сумма;",
    );
    assert_eq!(v, num("15"));
}

/// Байт-код фрагмента приходит СНАРУЖИ, а не из VM: подставной
/// компилятор игнорирует текст `Вычислить` и отдаёт артефакт для `2 + 2`.
/// Ответ `4` вместо `1` — доказательство делегирования: своего фронтенда у
/// VM больше нет, и подменить ей результат может только хост.
#[test]
fn a_dynamic_fragment_comes_from_the_host_not_from_the_vm() {
    struct Substitute {
        calls: usize,
    }

    impl bsl_bytecode::DynamicCompiler for Substitute {
        fn compile(
            &mut self,
            request: &bsl_bytecode::DynamicRequest<'_>,
        ) -> Result<std::rc::Rc<bsl_bytecode::DynamicUnit>, String> {
            self.calls += 1;
            let substitute = bsl_bytecode::DynamicRequest {
                source: "2 + 2",
                kind: request.kind,
                scope: request.scope,
                locals: request.locals,
                module_vars: request.module_vars,
                functions: request.functions,
                names: request.names,
                requirements: request.requirements,
            };
            bsl_compiler::compile_dynamic_snippet(
                &substitute,
                None,
                &bsl_syntax::PreprocSymbols::new(),
                std::num::NonZeroU64::new(self.calls as u64)
                    .expect("счётчик увеличен перед вызовом"),
            )
            .map(std::rc::Rc::new)
        }
    }

    let program = compile_src("Возврат Вычислить(\"1\");");
    let mut env = bsl_rt::HostEnv::process();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut dynamic = Substitute { calls: 0 };
    let value = run_program_with_host(
        &program,
        None,
        JitMode::Off,
        &mut stdout,
        &mut stderr,
        Some(&mut dynamic),
        &mut env,
    )
    .unwrap();
    assert_eq!(value, num("4"));
    assert_eq!(dynamic.calls, 1);
}

/// Отказ компилятора хоста — обычное исключение В МОМЕНТ ИСПОЛНЕНИЯ:
/// текст ошибки доезжает до `Попытка` целиком, а не роняет прогон.
#[test]
fn a_host_compile_failure_becomes_a_catchable_dynamic_error() {
    struct Refusing;

    impl bsl_bytecode::DynamicCompiler for Refusing {
        fn compile(
            &mut self,
            _request: &bsl_bytecode::DynamicRequest<'_>,
        ) -> Result<std::rc::Rc<bsl_bytecode::DynamicUnit>, String> {
            Err("хост отказал".to_string())
        }
    }

    fn run(src: &str) -> Result<BslValue, RtError> {
        let program = compile_src(src);
        let mut env = bsl_rt::HostEnv::process();
        let mut stdout = std::io::stdout().lock();
        let mut stderr = std::io::stderr().lock();
        let mut dynamic = Refusing;
        run_program_with_host(
            &program,
            None,
            JitMode::Off,
            &mut stdout,
            &mut stderr,
            Some(&mut dynamic),
            &mut env,
        )
    }

    assert!(matches!(
        run("Выполнить(\"х = 1\");"),
        Err(RtError::DynamicError(message)) if message == "хост отказал"
    ));

    let caught = run("рез = \"ок\";\n\
         Попытка\n\
         Выполнить(\"х = 1\");\n\
         рез = \"не сработало\";\n\
         Исключение\n\
         рез = \"поймано\";\n\
         КонецПопытки;\n\
         Возврат рез;")
    .unwrap();
    assert_eq!(str_val(&caught), "поймано");
}

/// Прогон, запущенный входом БЕЗ компилятора фрагментов, динамический код
/// не исполняет — но и не падает молча: это определённая ошибка, которую
/// ловит `Попытка`. Тихая поддержка `Выполнить` мимо хоста означала бы,
/// что фронтенд остался в VM.
#[test]
fn without_a_host_compiler_dynamic_code_is_a_catchable_error() {
    let program = compile_src("Возврат Вычислить(\"1\");");
    assert!(matches!(
        run_program(&program),
        Err(RtError::DynamicError(message)) if message.contains("без компилятора")
    ));

    let program = compile_src(
        "рез = \"ок\";\n\
         Попытка\n\
         Выполнить(\"х = 1\");\n\
         Исключение\n\
         рез = \"поймано\";\n\
         КонецПопытки;\n\
         Возврат рез;",
    );
    assert_eq!(str_val(&run_program(&program).unwrap()), "поймано");
}

#[test]
fn dynamic_compile_error_is_catchable_at_runtime() {
    // Ошибка компиляции фрагмента — обычное исключение в момент
    // исполнения, а не паника: её можно поймать `Попытка`.
    let v = run_src(
        "рез = \"ок\";\n\
         Попытка\n\
         Выполнить(\"это ((( не код\");\n\
         рез = \"не сработало\";\n\
         Исключение\n\
         рез = \"поймано\";\n\
         КонецПопытки;\n\
         Возврат рез;",
    );
    assert_eq!(str_val(&v), "поймано");
}

#[test]
fn dynamic_scope_marking_only_touches_functions_that_need_it() {
    // Таблица имён кадра materialized только у помеченных чанков —
    // остальные несут пустой список и ничего лишнего.
    let prog = parse(
        "Функция СВыполнить()\n\
         х = 1;\n\
         Выполнить(\"х = 2\");\n\
         Возврат х;\n\
         КонецФункции\n\
         Функция Обычная()\n\
         у = 1;\n\
         Возврат у;\n\
         КонецФункции\n\
         Возврат Обычная();",
    )
    .unwrap();
    let resolved = resolve_program(&prog.items).unwrap();
    assert!(resolved.functions[0].uses_dynamic);
    assert!(!resolved.functions[1].uses_dynamic);
    assert!(!resolved.top_level.uses_dynamic);

    let program = compile_program(&resolved).unwrap();
    assert!(
        !program.chunks[1].local_names.is_empty(),
        "помеченная функция"
    );
    assert!(program.chunks[2].local_names.is_empty(), "обычная функция");
}

#[test]
fn dynamic_marking_reaches_nested_statements() {
    // `Выполнить` под циклом под `Если` — такой же повод
    // материализовать имена, как и на верхнем уровне тела.
    let prog = parse(
        "Функция Ф(н)\n\
         Если н > 0 Тогда\n\
         Пока н > 0 Цикл\n\
         Выполнить(\"н = н - 1\");\n\
         КонецЦикла;\n\
         КонецЕсли;\n\
         Возврат н;\n\
         КонецФункции\n\
         Возврат Ф(3);",
    )
    .unwrap();
    let resolved = resolve_program(&prog.items).unwrap();
    assert!(resolved.functions[0].uses_dynamic);
    // И оно действительно работает насквозь.
    assert_eq!(
        run_src(
            "Функция Ф(Знач н)\n\
         Пока н > 0 Цикл\n\
         Выполнить(\"н = н - 1\");\n\
         КонецЦикла;\n\
         Возврат н;\n\
         КонецФункции\n\
         Возврат Ф(3);",
        ),
        num("0")
    );
}

#[test]
fn vypolnit_declaring_procedures_is_rejected() {
    let err = run_src_err(r#"Выполнить("Процедура П() КонецПроцедуры");"#);
    assert!(matches!(err, RtError::DynamicError(_)));
}

#[test]
fn appending_in_place_never_changes_a_string_someone_else_holds() {
    // `Instr::Add` дописывает буфер на месте, когда приёмник и левый
    // операнд — один регистр. Здесь проверяется, что «на месте» не
    // означает «у всех, кто на этот буфер смотрит»: строка в BSL —
    // тип ЗНАЧЕНИЯ, и присваивание обязано вести себя как копия.
    let v = run_src(
        r#"Копия = "начало";
           Строка1 = Копия;
           Массив1 = Новый Массив;
           Массив1.Добавить(Копия);
           Стр = Новый Структура("поле", Копия);
           Копия = Копия + "-хвост";
           Возврат Копия + "|" + Строка1 + "|" + Массив1[0] + "|" + Стр.поле;"#,
    );
    assert_eq!(str_val(&v), "начало-хвост|начало|начало|начало");
}

#[test]
fn appending_a_string_to_itself_does_not_read_a_moving_buffer() {
    // `Х = Х + Х`: у буфера две ссылки (регистр и правый операнд),
    // дописывание на месте невозможно — путь обязан быть копирующим.
    // Ошибись он, и правый операнд читался бы из вектора, который
    // растёт прямо во время чтения.
    let v = run_src(
        r#"Х = "аб";
           Х = Х + Х;
           Х = Х + Х;
           Возврат Х;"#,
    );
    assert_eq!(str_val(&v), "абабабаб");
}

#[test]
fn appending_through_a_byref_parameter_reaches_the_caller() {
    // У параметра по ссылке регистр — АЛИАС на слот вызывающего.
    // Признак «приёмник и операнд один регистр» считается по
    // абсолютным индексам, поэтому дописывание идёт в слот
    // вызывающего, а не в копию.
    let v = run_src(
        r#"Процедура Дописать(Текст)
               Текст = Текст + "-добавка";
           КонецПроцедуры
           Значение = "основа";
           Дописать(Значение);
           Возврат Значение;"#,
    );
    assert_eq!(str_val(&v), "основа-добавка");
}

#[test]
fn adding_a_non_string_to_a_string_leaves_the_variable_intact() {
    // Ошибка не должна оставлять переменную затёртой: значение
    // забирается из регистра ТОЛЬКО когда обе стороны — строки.
    //
    // Пара «строка плюс массив» для этого больше не годится — по
    // измеренному правилу строка слева склеивается с чем угодно
    // (получилось бы «текстМассив»). Отказывает теперь обратный
    // случай: число слева и строка, числом не являющаяся.
    let v = run_src(
        r#"Х = 5;
           Р = "";
           Попытка
               Х = Х + "абв";
           Исключение
               Р = "поймано";
           КонецПопытки;
           Возврат Р + "|" + Х;"#,
    );
    assert_eq!(str_val(&v), "поймано|5");

    // А сама склейка строки с нестрокой теперь именно склейка.
    let v = run_src(r#"Х = "текст"; Х = Х + Новый Массив; Возврат Х;"#);
    assert_eq!(str_val(&v), "текстМассив");
}

#[test]
fn text_writer_writes_utf8_and_flushes_on_close() {
    let path = std::env::temp_dir().join(format!(
        "open-bsl-text-writer-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let src = format!(
        "Файл = Новый ЗаписьТекста(\"{}\");\n\
         Файл.Записать(\"Привет\");\n\
         Файл.Записать(Символ(10));\n\
         Файл.Закрыть();",
        path.display()
    );

    run_src(&src);
    // BOM и CRLF — не наша выдумка, а ИЗМЕРЕННЫЙ вывод 8.3.27:
    // `Новый ЗаписьТекста(Путь)` без прочих аргументов даёт файл,
    // начинающийся с EF BB BF, и разворачивает ПС в CRLF. Проверяем
    // байты, а не строку: именно они и расходились с платформой.
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"\xef\xbb\xbf\xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82\r\n"
    );
    std::fs::remove_file(path).unwrap();
}

// --- Вызов процедуры/функции модуля по имени --------------------------

/// Компилирует модуль, не запуская его: тестам `call_module_function`
/// нужна сама `Program`, а не только значение прогона.
fn compile_src(src: &str) -> Program {
    let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    let resolved = resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"));
    compile_program(&resolved).unwrap_or_else(|e| panic!("compile error: {e:?}"))
}

/// Прогоняет верхний уровень модуля и отдаёт его финальный стек — то
/// самое состояние, поверх которого рантайм зовёт функцию по имени
/// (первые слоты этого стека и есть модульные переменные).
fn module_state(program: &Program) -> Vec<BslValue> {
    let mut stack: Vec<BslValue> = Vec::new();
    push_own_registers(&mut stack, &program.chunks[0]);
    let (_value, stack) =
        drive(program, 0, stack).unwrap_or_else(|e| panic!("runtime error: {e:?}"));
    stack
}

#[test]
fn call_module_function_by_name_returns_value() {
    let program = compile_src(
        "Функция Удвоить(х)\n\
         Возврат х * 2;\n\
         КонецФункции\n",
    );
    let mut stack = module_state(&program);
    // Имя приходит из данных, а не из исходника, поэтому регистр здесь
    // намеренно другой: поиск обязан быть регистронезависимым.
    let (value, params) =
        call_module_function(&program, &mut stack, "уДВОИТЬ", vec![num("21")]).unwrap();
    assert_eq!(value, num("42"));
    assert_eq!(params, vec![num("21")]);
}

#[test]
fn call_module_function_passes_several_args() {
    let program = compile_src(
        "Функция Собрать(а, б, в)\n\
         Возврат а * 100 + б * 10 + в;\n\
         КонецФункции\n",
    );
    let mut stack = module_state(&program);
    // Разряды разные, поэтому перепутанный порядок аргументов даст
    // другое число, а не то же самое.
    let (value, params) = call_module_function(
        &program,
        &mut stack,
        "Собрать",
        vec![num("1"), num("2"), num("3")],
    )
    .unwrap();
    assert_eq!(value, num("123"));
    assert_eq!(params, vec![num("1"), num("2"), num("3")]);
}

#[test]
fn call_module_function_on_procedure_returns_undefined() {
    let program = compile_src(
        "Процедура Пометить(Отказ)\n\
         Отказ = Истина;\n\
         КонецПроцедуры\n",
    );
    let mut stack = module_state(&program);
    let (value, params) = call_module_function(
        &program,
        &mut stack,
        "Пометить",
        vec![BslValue::Boolean(false)],
    )
    .unwrap();
    assert_eq!(value, BslValue::Undefined);
    // Аргументы едут по значению, и запись в параметр без `Знач`
    // наблюдаема ровно одним каналом — финальными значениями слотов.
    assert_eq!(params, vec![BslValue::Boolean(true)]);
}

#[test]
fn call_module_function_mutates_module_var() {
    let program = compile_src(
        "Перем Счетчик;\n\
         Процедура Увеличить()\n\
         Счетчик = Счетчик + 1;\n\
         КонецПроцедуры\n\
         Счетчик = 10;\n",
    );
    let mut stack = module_state(&program);
    // Модульная переменная — первый слот кадра верхнего уровня.
    assert_eq!(stack[0], num("10"));
    let (value, params) =
        call_module_function(&program, &mut stack, "Увеличить", Vec::new()).unwrap();
    assert_eq!(value, BslValue::Undefined);
    assert!(params.is_empty());
    assert_eq!(stack[0], num("11"));
}

#[test]
fn call_module_function_with_dynamic_eval_inside() {
    // Две вложенности сразу: рантайм зовёт функцию по имени
    // (`call_module_function`), а та внутри себя исполняет фрагменты
    // (`run_dynamic_snippet`). Модульный блок обязан доехать в обе
    // стороны через обе границы: `Выполнить` пишет в `Счетчик`,
    // `Вычислить` читает уже НОВОЕ значение, и оно же остаётся в стеке
    // вызывающего после возврата.
    let program = compile_src(
        "Перем Счетчик;\n\
         Функция Крутить()\n\
         Выполнить(\"Счетчик = Счетчик + 1\");\n\
         Возврат Вычислить(\"Счетчик * 10\");\n\
         КонецФункции\n\
         Счетчик = 4;\n",
    );
    let mut stack = module_state(&program);
    assert_eq!(stack[0], num("4"));
    // Не `call_module_function`: у неё компилятора фрагментов нет
    // намеренно. В боевом пути он приезжает вместе с потоками и
    // окружением — тем же `HostIo`, который VM протаскивает всюду.
    let mut env = bsl_rt::HostEnv::process();
    let linked = link_components(
        &program,
        None,
        env.zone(),
        env.files(),
        bsl_bytecode::DynamicScope::ROOT,
    )
    .unwrap();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let mut dynamic = TestDynamic::bare();
    let dynamic_depth = std::cell::Cell::new(0);
    let mut host = HostIo {
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: Some(&mut env),
        dynamic: Some(&mut dynamic),
        dynamic_depth: &dynamic_depth,
    };
    let (value, params) = call_module_function_with_host(
        &program,
        &mut stack,
        "Крутить",
        Vec::new(),
        JitMode::Off,
        &linked,
        &mut host,
    )
    .unwrap();
    // 50, а не 40: `Вычислить` видит запись, сделанную предыдущим
    // `Выполнить`, а не исходное значение слота.
    assert_eq!(value, num("50"));
    assert!(params.is_empty());
    // И мутация из вложенного фрагмента переживает вызов по имени —
    // область здесь кадр функции (`scope_id >= 1`), значит модульный
    // блок ехал неалиасной веткой и обязан быть перенесён обратно.
    assert_eq!(stack[0], num("5"));
}

#[test]
fn call_module_function_unknown_name_is_rt_error() {
    let program = compile_src(
        "Функция Удвоить(х)\n\
         Возврат х * 2;\n\
         КонецФункции\n",
    );
    let mut stack = module_state(&program);
    // Имя приходит из пользовательских данных, поэтому промах — это
    // перехватываемая ошибка, а не паника.
    let err =
        call_module_function(&program, &mut stack, "НетТакойФункции", vec![num("1")]).unwrap_err();
    assert!(matches!(err, RtError::DynamicError(_)), "{err:?}");
}

#[test]
fn call_module_function_wrong_arity_is_rt_error() {
    let program = compile_src(
        "Функция Удвоить(х)\n\
         Возврат х * 2;\n\
         КонецФункции\n",
    );
    let mut stack = module_state(&program);
    let err = call_module_function(&program, &mut stack, "Удвоить", Vec::new()).unwrap_err();
    assert!(matches!(err, RtError::DynamicError(_)), "{err:?}");
    let err = call_module_function(&program, &mut stack, "Удвоить", vec![num("1"), num("2")])
        .unwrap_err();
    assert!(matches!(err, RtError::DynamicError(_)), "{err:?}");
}

// --- Колбэки JSON сквозь `drive` -------------------------------------
//
// `bsl-rt` умеет звать функцию по имени только через замыкание, которое
// строит `call_builtin_with_format`; проверяется здесь именно СВЯЗКА —
// настоящие функции модуля, вызванные из `ПрочитатьJSON`/`ЗаписатьJSON`
// на обычном прогоне. Семантику самих колбэков покрывают юнит-тесты
// `bsl-rt` (`json_callback_tests`), эталон — фикстура json-callbacks,
// снятая с платформы.

/// В позиции модуля здесь стоит `Истина`: ИЗМЕРЕНО, что значимо только
/// её отличие от `Неопределено` (платформа ищет функцию в переданном
/// модуле, у этого интерпретатора модуль ровно один).
#[test]
fn json_callback_calls_a_real_module_function_on_write() {
    let v = run_src_with_json(
        "Функция Преобразовать(Свойство, Значение, ДопПар, Отказ)\n\
         	Возврат \"<\" + Свойство + \"/\" + ДопПар + \">\";\n\
         КонецФункции\n\
         Запись = Новый ЗаписьJSON;\n\
         Запись.УстановитьСтроку(Новый ПараметрыЗаписиJSON(ПереносСтрокJSON.Нет));\n\
         Значение = Новый Структура(\"а\", Новый ТаблицаЗначений);\n\
         ЗаписатьJSON(Запись, Значение, , \"Преобразовать\", Истина, \"ДОП\");\n\
         Возврат Запись.Закрыть();\n",
    );
    assert_eq!(
        v,
        BslValue::Str(bsl_rt::BslString::from_str("{\"а\":\"<а/ДОП>\"}"))
    );
}

/// `Отказ` — параметр БЕЗ `Знач`, и его значение возвращается из вызова
/// финальными слотами параметров (см. `call_module_function`). Тест
/// бьёт именно этот канал: без него отказ не дошёл бы до рантайма.
#[test]
fn json_callback_refusal_travels_back_through_the_parameter_slot() {
    let v = run_src_with_json(
        "Функция Отказная(Свойство, Значение, ДопПар, Отказ)\n\
         	Отказ = Истина;\n\
         	Возврат \"<не должно попасть>\";\n\
         КонецФункции\n\
         Запись = Новый ЗаписьJSON;\n\
         Запись.УстановитьСтроку(Новый ПараметрыЗаписиJSON(ПереносСтрокJSON.Нет));\n\
         Значение = Новый Структура;\n\
         Значение.Вставить(\"а\", 1);\n\
         Значение.Вставить(\"б\", Новый ТаблицаЗначений);\n\
         ЗаписатьJSON(Запись, Значение, , \"Отказная\", Истина, \"ДОП\");\n\
         Возврат Запись.Закрыть();\n",
    );
    assert_eq!(v, BslValue::Str(bsl_rt::BslString::from_str("{\"а\":1}")));
}

/// Функция восстановления зовётся для каждого значения документа, и её
/// результат попадает в собранное значение.
#[test]
fn json_callback_calls_a_real_module_function_on_read() {
    let v = run_src_with_json(
        "Функция Восстановить(Свойство, Значение, ДопПар)\n\
         	Если Свойство = Неопределено Тогда\n\
         		Возврат Значение;\n\
         	КонецЕсли;\n\
         	Возврат Значение * 10;\n\
         КонецФункции\n\
         Чтение = Новый ЧтениеJSON;\n\
         Чтение.УстановитьСтроку(\"{\"\"а\"\":1,\"\"б\"\":2}\");\n\
         Р = ПрочитатьJSON(Чтение, Ложь, , , \"Восстановить\", Истина, \"ДОП\");\n\
         Возврат Р.а + Р.б;\n",
    );
    assert_eq!(v, num("30"));
}

/// Функция модуля, вызванная из колбэка, видит и меняет МОДУЛЬНЫЕ
/// переменные — тот же перенос блока, что и у прямого
/// `call_module_function`.
#[test]
fn json_callback_sees_module_variables() {
    let v = run_src_with_json(
        "Перем Счетчик;\n\
         Функция Восстановить(Свойство, Значение, ДопПар)\n\
         	Счетчик = Счетчик + 1;\n\
         	Возврат Значение;\n\
         КонецФункции\n\
         Счетчик = 0;\n\
         Чтение = Новый ЧтениеJSON;\n\
         Чтение.УстановитьСтроку(\"{\"\"а\"\":1,\"\"б\"\":[2,3]}\");\n\
         ПрочитатьJSON(Чтение, Ложь, , , \"Восстановить\", Истина, \"ДОП\");\n\
         Возврат Счетчик;\n",
    );
    // Пять значений документа: `а`, два элемента массива, сам массив
    // под именем `б` и корень.
    assert_eq!(v, num("5"));
}

/// Ошибка вызова (нет такой функции модуля) доходит до `Попытка`, а не
/// роняет прогон.
#[test]
fn json_callback_unknown_name_is_catchable() {
    let v = run_src_with_json(
        "Попытка\n\
         	Запись = Новый ЗаписьJSON;\n\
         	Запись.УстановитьСтроку();\n\
         	ЗаписатьJSON(Запись, Новый ТаблицаЗначений, , \"НетТакой\", Истина);\n\
         	Возврат \"принято\";\n\
         Исключение\n\
         	Возврат \"поймано\";\n\
         КонецПопытки;\n",
    );
    assert_eq!(v, BslValue::Str(bsl_rt::BslString::from_str("поймано")));
}

/// Исключение ИЗНУТРИ колбэка тоже перехватывается снаружи, а не
/// глотается рантаймом.
#[test]
fn json_callback_raise_propagates_to_the_caller() {
    let v = run_src_with_json(
        "Функция Бросающая(Свойство, Значение, ДопПар, Отказ)\n\
         	ВызватьИсключение \"изнутри\";\n\
         КонецФункции\n\
         Попытка\n\
         	Запись = Новый ЗаписьJSON;\n\
         	Запись.УстановитьСтроку();\n\
         	ЗаписатьJSON(Запись, Новый ТаблицаЗначений, , \"Бросающая\", Истина);\n\
         	Возврат \"принято\";\n\
         Исключение\n\
         	Возврат \"поймано\";\n\
         КонецПопытки;\n",
    );
    assert_eq!(v, BslValue::Str(bsl_rt::BslString::from_str("поймано")));
}

/// Счётчик вложенности `Выполнить`/`Вычислить` принадлежит ПРОГОНУ, а не
/// потоку (шаг 16 плана abi-refactor-f). Раньше он был `thread_local` и
/// делился между сессиями одного потока: набранная одной сессией глубина
/// урезала бы вложенность у другой (например, у вложенного `Engine` за
/// обратным вызовом). Теперь у каждого `HostIo` свой `Cell`, и полностью
/// занятый счётчик одной сессии не мешает другой.
#[test]
fn two_sessions_do_not_share_dynamic_nesting_depth() {
    let first = std::cell::Cell::new(0);
    let second = std::cell::Cell::new(0);

    let mut held = Vec::new();
    for _ in 0..MAX_DYNAMIC_DEPTH {
        held.push(DynamicDepthGuard::enter(&first).expect("в пределах лимита"));
    }
    // Первая сессия заполнена — следующий уровень в ней отвергается.
    assert!(DynamicDepthGuard::enter(&first).is_err());
    // Вторая сессия того же потока не затронута.
    assert!(DynamicDepthGuard::enter(&second).is_ok());
}
