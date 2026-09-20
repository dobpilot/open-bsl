//! Отладочное выражение использует контекст выбранного остановленного кадра.

use bsl_vm::{DebugAction, DebugHook, DebugPosition};
use open_bsl::{Engine, ExecutionPoll, ModuleGraphRecipe, ModuleRecipe, Value};
use std::{cell::Cell, rc::Rc};

struct InspectModules(Rc<Cell<bool>>);
impl DebugHook for InspectModules {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        if at.frames.len() != 4 || at.frames[3].1 != 1 || self.0.replace(true) {
            return DebugAction::Continue;
        }
        for (index, line) in [6, 3, 4, 3].into_iter().enumerate() {
            assert_eq!(at.values.line_of(index), Some(line), "frame {index}");
        }
        assert!(at.values.locals(99).is_empty());
        assert_eq!(at.values.line_of(99), None);
        assert!(at.values.evaluate(99, "1").is_err());
        for (index, parameter, module_value) in
            [(1, "АргВхода", 11), (2, "АргА", 20), (3, "АргБ", 30)]
        {
            let locals = at.values.locals(index);
            assert!(
                locals
                    .iter()
                    .any(|(name, value)| name == parameter && *value == Value::number_from_i64(11)),
                "frame {index}: {locals:?}"
            );
            assert_eq!(
                at.values.evaluate(index, parameter).unwrap(),
                Value::number_from_i64(11)
            );
            assert_eq!(
                at.values.evaluate(index, "ЭтотОбъект.Прочитать()").unwrap(),
                Value::number_from_i64(module_value)
            );
        }
        assert_eq!(
            at.values.evaluate(2, "Б.Счетчик").unwrap(),
            Value::number_from_i64(30)
        );
        assert!(at.values.evaluate(3, "А.Счетчик").is_err());
        assert!(at.values.evaluate(2, "ЭтотОбъект.Сбой(АргА)").is_err());
        assert!(
            at.values
                .evaluate(2, "Сбой(АргА)")
                .unwrap_err()
                .contains("debug failure")
        );
        assert_eq!(
            at.values.evaluate(2, "Счетчик").unwrap(),
            Value::number_from_i64(21)
        );
        for (index, name) in [(1, "АргВхода"), (2, "АргА"), (3, "АргБ")] {
            assert_eq!(
                at.values.evaluate(index, name).unwrap(),
                Value::number_from_i64(41)
            );
        }
        assert!(
            at.values
                .evaluate(2, "ЭтотОбъект.Позже()")
                .unwrap()
                .promise_identity()
                .is_some()
        );
        assert_eq!(
            at.values.evaluate(2, "Снимок.Количество()").unwrap(),
            Value::number_from_i64(0)
        );
        DebugAction::Continue
    }
}

#[test]
fn selected_frames_keep_modules_imports_references_and_writes_after_error() {
    let engine = Engine::builder()
        .debug_info(true)
        .configuration(ModuleGraphRecipe {
            modules: vec![
                ModuleRecipe {
                    name: "А".into(),
                    imports: vec![("Б".into(), "Б".into())],
                    source: r#"
Перем Счетчик Экспорт; Перем Снимок;
Функция Проверить(АргА) Экспорт
    Возврат Б.Остановить(АргА);
КонецФункции
Функция Прочитать() Экспорт Возврат Счетчик; КонецФункции
Функция Сбой(Аргумент)
    Аргумент = 41; Счетчик = Счетчик + 1;
    ВызватьИсключение "debug failure";
КонецФункции
Асинх Функция Позже() Экспорт
    Ждать 0; Снимок.Добавить(Счетчик); Возврат Счетчик;
КонецФункции
Функция Завершить(Знач Аргумент) Экспорт
    Снимок.Добавить(Аргумент); Возврат Снимок;
КонецФункции
Счетчик = 20;
Снимок = Новый Массив;
"#
                    .into(),
                },
                ModuleRecipe {
                    name: "Б".into(),
                    imports: vec![],
                    source: r#"
Перем Счетчик Экспорт;
Функция Остановить(АргБ) Экспорт Возврат АргБ; КонецФункции
Функция Прочитать() Экспорт Возврат Счетчик; КонецФункции
Счетчик = 30;
"#
                    .into(),
                },
            ],
            eager_init: false,
        })
        .build()
        .unwrap();
    let module = engine
        .compile_entry(
            r#"
Перем Корень;
Функция Начать(АргВхода) Возврат А.Проверить(АргВхода); КонецФункции
Функция Прочитать() Экспорт Возврат Корень; КонецФункции
Корень = 11;
Возврат А.Завершить(Начать(Корень));
"#,
        )
        .unwrap();
    let image = engine.image_bytecode(&module, None).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } =
        bsl_bytecode::parse_image(&image).unwrap()
    else {
        panic!("catalog")
    };
    let loaded_engine = Engine::builder()
        .debug_info(true)
        .configuration_image(catalog, false)
        .build()
        .unwrap();
    let loaded = loaded_engine.load_entry(entry.unwrap()).unwrap();
    for (engine, module) in [(&engine, &module), (&loaded_engine, &loaded)] {
        let inspected = Rc::new(Cell::new(false));
        let mut state = engine.new_state();
        let mut execution = state.start(module).unwrap();
        execution.set_debug_hook(Box::new(InspectModules(inspected.clone())));
        loop {
            if let ExecutionPoll::Complete(value) = execution.poll(usize::MAX).unwrap() {
                let names = bsl_rt::NameInterner::default();
                assert_eq!(
                    value.get_index(&Value::number_from_i64(0), &names).unwrap(),
                    Value::number_from_i64(41)
                );
                assert_eq!(
                    value.get_index(&Value::number_from_i64(1), &names).unwrap(),
                    Value::number_from_i64(21)
                );
                break;
            }
        }
        assert!(inspected.get());
    }
}

struct InspectInitialization(Rc<Cell<bool>>);
impl DebugHook for InspectInitialization {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        if at.frames.len() == 3 && at.frames[2].1 == 0 && !self.0.replace(true) {
            assert_eq!(at.values.line_of(1), Some(3));
        }
        DebugAction::Continue
    }
}

#[test]
fn a_lazy_module_body_keeps_the_callers_current_instruction_line() {
    let engine = Engine::builder().debug_info(true).configuration(ModuleGraphRecipe {
        modules: vec![
            ModuleRecipe { name: "А".into(), imports: vec![("Б".into(), "Б".into())], source: "Функция Запуск() Экспорт\n    Локальная = 1;\n    Возврат Б.Результат();\nКонецФункции".into() },
            ModuleRecipe { name: "Б".into(), imports: vec![], source: "Перем Значение;\nФункция Результат() Экспорт Возврат Значение; КонецФункции\nЗначение = 42;".into() },
        ], eager_init: false,
    }).build().unwrap();
    let module = engine.compile_entry("Возврат А.Запуск();").unwrap();
    let inspected = Rc::new(Cell::new(false));
    let mut state = engine.new_state();
    let mut execution = state.start(&module).unwrap();
    execution.set_debug_hook(Box::new(InspectInitialization(inspected.clone())));
    loop {
        if let ExecutionPoll::Complete(value) = execution.poll(usize::MAX).unwrap() {
            assert_eq!(value, Value::number_from_i64(42));
            break;
        }
    }
    assert!(inspected.get());
}

struct InspectPromise {
    line: u32,
    expression: String,
    inspected: Rc<Cell<bool>>,
    failing_eval: bool,
}
impl DebugHook for InspectPromise {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        if at.line != Some(self.line) || self.inspected.replace(true) {
            return DebugAction::Continue;
        }
        let index = at.frames.len() - 1;
        let result = at.values.evaluate(index, &self.expression);
        let promise = if self.failing_eval {
            assert!(result.unwrap_err().contains("evaluation failure"));
            at.values.evaluate(index, "ОбещаниеОтладки").unwrap()
        } else {
            result.unwrap()
        };
        assert!(promise.promise_identity().is_some());
        assert_eq!(
            at.values.evaluate(index, "СоседДошел").unwrap(),
            Value::Boolean(false)
        );
        assert_eq!(
            at.values.evaluate(index, "ОбещаниеОтладки").unwrap(),
            promise
        );
        DebugAction::Continue
    }
}

#[test]
fn evaluated_promises_survive_without_running_paused_tasks() {
    let source = r#"
Перем ОбещаниеОтладки, СоседДошел, ФайлОтладки, РезультатОтладки;
Асинх Процедура Сосед()
    Ждать 0; СоседДошел = Истина;
КонецПроцедуры
Асинх Функция Позже() Экспорт Ждать 0; Возврат 42; КонецФункции
Асинх Функция ОшибочноеОбещание() Экспорт Ждать 0; ВызватьИсключение "promise failure"; КонецФункции
Функция Запомнить(Знач П)
    ОбещаниеОтладки = П; Возврат П;
КонецФункции
Функция ЗапомнитьИОтказать(Знач П)
    ОбещаниеОтладки = П; ВызватьИсключение "evaluation failure";
КонецФункции
Асинх Процедура Запуск()
ФайлОтладки = Новый Файл("/open-bsl-debug-evaluate-missing-file");
СоседДошел = Ложь;
Сосед();
КонтрольОтладки = 0;
Попытка
    РезультатОтладки.Добавить(Ждать ОбещаниеОтладки);
Исключение
    РезультатОтладки.Добавить(ИнформацияОбОшибке().Описание);
КонецПопытки;
КонецПроцедуры
РезультатОтладки = Новый Массив;
Запуск();
Возврат РезультатОтладки;
"#;
    let line = source
        .lines()
        .position(|line| line == "КонтрольОтладки = 0;")
        .unwrap() as u32
        + 1;
    for (expression, expected) in [
        ("Запомнить(ЭтотОбъект.Позже())", Value::number_from_i64(42)),
        (
            "Запомнить(Вычислить(\"ЭтотОбъект.Позже()\"))",
            Value::number_from_i64(42),
        ),
        (
            "Запомнить(ФайлОтладки.СуществуетАсинх())",
            Value::Boolean(false),
        ),
        (
            "ЗапомнитьИОтказать(ЭтотОбъект.Позже())",
            Value::number_from_i64(42),
        ),
        (
            "Запомнить(ЭтотОбъект.ОшибочноеОбещание())",
            Value::Str(open_bsl::BslString::from_str("promise failure")),
        ),
    ] {
        let engine = Engine::builder().debug_info(true).build().unwrap();
        let module = engine.compile(source).unwrap();
        let loaded = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();
        for module in [&module, &loaded] {
            let inspected = Rc::new(Cell::new(false));
            let mut state = engine.new_state();
            let mut execution = state.start(module).unwrap();
            execution.set_debug_hook(Box::new(InspectPromise {
                line,
                expression: expression.into(),
                inspected: inspected.clone(),
                failing_eval: expression.starts_with("ЗапомнитьИОтказать"),
            }));
            loop {
                if let ExecutionPoll::Complete(value) = execution.poll(usize::MAX).unwrap() {
                    assert_eq!(
                        value
                            .get_index(&Value::number_from_i64(0), &bsl_rt::NameInterner::default())
                            .unwrap(),
                        expected
                    );
                    break;
                }
            }
            assert!(inspected.get());
        }
    }
}

#[derive(Clone, Debug)]
struct HeldFiles {
    release: std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>,
    timed_out: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl open_bsl::FileSystem for HeldFiles {
    fn background_access(&self) -> Option<std::sync::Arc<dyn open_bsl::FileSystem + Send + Sync>> {
        Some(std::sync::Arc::new(self.clone()))
    }
    fn read(&self, _: &str) -> std::io::Result<Vec<u8>> {
        panic!("unexpected read")
    }
    fn write(&self, _: &str, _: &[u8]) -> std::io::Result<()> {
        panic!("unexpected write")
    }
    fn read_dir<'a>(
        &'a self,
        _: &str,
    ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<open_bsl::DirEntry>> + 'a>> {
        panic!("unexpected read_dir")
    }
    fn create_dir_all(&self, _: &str) -> std::io::Result<()> {
        panic!("unexpected create_dir_all")
    }
    fn open(
        &self,
        _: &str,
        _: open_bsl::FileOpenOptions,
    ) -> std::io::Result<Box<dyn open_bsl::FileHandle>> {
        panic!("unexpected open")
    }
    fn path_separator(&self) -> std::io::Result<String> {
        Ok("/".into())
    }
    fn metadata(&self, path: &str) -> std::io::Result<open_bsl::FileMetadata> {
        assert_eq!(path, "virtual/file");
        if self
            .release
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .is_err()
        {
            self.timed_out
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Err(std::io::ErrorKind::NotFound.into())
    }
}

struct ReleaseFile {
    inspected: Rc<Cell<bool>>,
    release: std::sync::mpsc::Sender<()>,
    timed_out: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl DebugHook for ReleaseFile {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        if at.frames.last().map(|frame| frame.1) != Some(2) || self.inspected.replace(true) {
            return DebugAction::Continue;
        }
        let index = at.frames.len() - 1;
        let promise = at
            .values
            .evaluate(index, "Запомнить(ФайлОтладки.СуществуетАсинх())")
            .unwrap();
        assert!(promise.promise_identity().is_some());
        assert!(!self.timed_out.load(std::sync::atomic::Ordering::SeqCst));
        self.release.send(()).unwrap();
        DebugAction::Continue
    }
}

#[test]
fn delayed_file_operation_outlives_evaluate_in_the_original_host() {
    let engine = Engine::builder().debug_info(true).build().unwrap();
    let module = engine
        .compile(
            r#"
Перем ФайлОтладки, ОбещаниеОтладки, РезультатОтладки;
Функция Запомнить(Знач П) ОбещаниеОтладки = П; Возврат П; КонецФункции
Асинх Процедура Запуск() РезультатОтладки.Добавить(Ждать ОбещаниеОтладки); КонецПроцедуры
ФайлОтладки = Новый Файл("virtual/file");
РезультатОтладки = Новый Массив;
Запуск(); Возврат РезультатОтладки;
"#,
        )
        .unwrap();
    let (release, receiver) = std::sync::mpsc::channel();
    let timed_out = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let inspected = Rc::new(Cell::new(false));
    let mut state = engine
        .state_builder()
        .files(HeldFiles {
            release: std::sync::Arc::new(std::sync::Mutex::new(receiver)),
            timed_out: timed_out.clone(),
        })
        .build();
    let mut execution = state.start(&module).unwrap();
    execution.set_debug_hook(Box::new(ReleaseFile {
        inspected: inspected.clone(),
        release,
        timed_out: timed_out.clone(),
    }));
    loop {
        if let ExecutionPoll::Complete(value) = execution.poll(usize::MAX).unwrap() {
            assert_eq!(
                value
                    .get_index(&Value::number_from_i64(0), &bsl_rt::NameInterner::default())
                    .unwrap(),
                Value::Boolean(false)
            );
            break;
        }
    }
    assert!(inspected.get());
    assert!(!timed_out.load(std::sync::atomic::Ordering::SeqCst));
}
