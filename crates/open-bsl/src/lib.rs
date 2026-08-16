//! Устойчивый фасад для встраивания интерпретатора BSL в Rust-приложение.
//!
//! [`Engine`] собирает неизменяемый набор статических компонентов и
//! компилирует [`Module`]. [`State`] владеет сервисами конкретной сессии,
//! включая независимые потоки вывода.

use std::fmt;
use std::io::Write;
use std::sync::Arc;

pub use bsl_rt::BslValue as Value;
pub use bsl_rt::{
    Arity, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode, FunctionDescriptor,
    FunctionKind, LibraryDependency, LibraryDescriptor,
};

/// Ошибка одной из фаз публичного конвейера.
#[derive(Debug)]
pub enum Error {
    Parse(bsl_syntax::Diagnostic),
    Semantic(bsl_sema::SemaError),
    Compile(bsl_bytecode::CompileError),
    Registry(bsl_rt::RegistryError),
    Runtime(bsl_rt::RtError),
    Bytecode(bsl_bytecode::TextError),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "ошибка синтаксиса: {error:?}"),
            Self::Semantic(error) => write!(formatter, "ошибка семантики: {error:?}"),
            Self::Compile(error) => write!(formatter, "ошибка компиляции: {error:?}"),
            Self::Registry(error) => write!(formatter, "ошибка компонентов: {error}"),
            Self::Runtime(error) => write!(formatter, "ошибка исполнения: {error}"),
            Self::Bytecode(error) => write!(formatter, "ошибка байт-кода: {error:?}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<bsl_syntax::Diagnostic> for Error {
    fn from(error: bsl_syntax::Diagnostic) -> Self {
        Self::Parse(error)
    }
}

impl From<bsl_sema::SemaError> for Error {
    fn from(error: bsl_sema::SemaError) -> Self {
        Self::Semantic(error)
    }
}

impl From<bsl_bytecode::CompileError> for Error {
    fn from(error: bsl_bytecode::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<bsl_rt::RegistryError> for Error {
    fn from(error: bsl_rt::RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<bsl_rt::RtError> for Error {
    fn from(error: bsl_rt::RtError) -> Self {
        Self::Runtime(error)
    }
}

impl From<bsl_bytecode::TextError> for Error {
    fn from(error: bsl_bytecode::TextError) -> Self {
        Self::Bytecode(error)
    }
}

/// Неизменяемая конфигурация компилятора и runtime-компонентов.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    registry: bsl_rt::RuntimeRegistry,
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    /// Компилирует исходный модуль с каталогом компонентов этого движка.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку синтаксиса, семантики или генерации байт-кода.
    pub fn compile(&self, source: &str) -> Result<Module, Error> {
        let syntax = bsl_syntax::parse(source)?;
        let resolved =
            bsl_sema::resolve_program_with_registry(&syntax.items, &self.inner.registry)?;
        let program = bsl_bytecode::compile_program(&resolved)?;
        Ok(Module { program })
    }

    /// Загружает текстовый байт-код. Совместимость компонентов проверяется
    /// при запуске, до первой инструкции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку формата байт-кода.
    pub fn load_bytecode(&self, source: &str) -> Result<Module, Error> {
        Ok(Module {
            program: bsl_bytecode::parse_program(source)?,
        })
    }

    pub fn new_state(&self) -> State {
        self.state_builder().build()
    }

    pub fn state_builder(&self) -> StateBuilder {
        StateBuilder::new(self.clone())
    }
}

/// Стадия композиции статически связанных runtime-компонентов.
pub struct EngineBuilder {
    runtime: bsl_rt::RuntimeBuilder,
}

impl EngineBuilder {
    pub fn new() -> Self {
        let mut runtime = bsl_rt::RuntimeBuilder::new();
        runtime.register(bsl_rt::core_library());
        Self { runtime }
    }

    pub fn register_library(mut self, library: LibraryDescriptor) -> Self {
        self.runtime.register(library);
        self
    }

    /// Проверяет зависимости и замораживает каталог компонентов.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при конфликте идентичностей, имён, кодов или
    /// версий компонентов.
    pub fn build(self) -> Result<Engine, Error> {
        Ok(Engine {
            inner: Arc::new(EngineInner {
                registry: self.runtime.build()?,
            }),
        })
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Скомпилированный переносимый модуль.
#[derive(Clone)]
pub struct Module {
    program: bsl_bytecode::Program,
}

impl Module {
    pub fn requirements(&self) -> &[bsl_rt::LibraryRequirement] {
        &self.program.requirements
    }

    /// Сериализует модуль в переносимый текстовый байт-код.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если внутренние таблицы модуля не помещаются в
    /// ограничения формата.
    pub fn bytecode(&self) -> Result<String, Error> {
        Ok(bsl_bytecode::write_program(&self.program, None)?)
    }
}

/// Настройки сервисов одной сессии исполнения.
pub struct StateBuilder {
    engine: Engine,
    host: HostServices,
    jit: bool,
}

impl StateBuilder {
    fn new(engine: Engine) -> Self {
        Self {
            engine,
            host: HostServices::process(),
            jit: false,
        }
    }

    pub fn stdout(mut self, writer: impl Write + 'static) -> Self {
        self.host.stdout = Box::new(writer);
        self
    }

    pub fn stderr(mut self, writer: impl Write + 'static) -> Self {
        self.host.stderr = Box::new(writer);
        self
    }

    pub fn jit(mut self, enabled: bool) -> Self {
        self.jit = enabled;
        self
    }

    pub fn build(self) -> State {
        State {
            engine: self.engine,
            host: self.host,
            jit: self.jit,
        }
    }
}

/// Изменяемые возможности host-приложения, принадлежащие одной сессии.
/// Они не входят в реестр компонентов и не сериализуются в байт-код.
pub struct HostServices {
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

impl HostServices {
    fn process() -> Self {
        Self {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
        }
    }
}

/// Изолированные изменяемые host-сервисы одной BSL-сессии.
pub struct State {
    engine: Engine,
    host: HostServices,
    jit: bool,
}

impl State {
    /// Создаёт состояние с базовым рантаймом и потоками процесса.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку сборки базового реестра компонентов.
    pub fn new() -> Result<Self, Error> {
        Ok(Engine::builder().build()?.new_state())
    }

    /// Компилирует и исполняет исходный модуль.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку любой фазы компиляции или исполнения.
    pub fn exec(&mut self, source: &str) -> Result<Value, Error> {
        let module = self.engine.compile(source)?;
        self.run(&module)
    }

    /// Вычисляет BSL-выражение в отдельном модуле.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку любой фазы компиляции или исполнения.
    pub fn eval(&mut self, expression: &str) -> Result<Value, Error> {
        self.exec(&format!("Возврат ({expression});"))
    }

    /// Исполняет заранее скомпилированный модуль.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку связывания компонентов или исполнения.
    pub fn run(&mut self, module: &Module) -> Result<Value, Error> {
        let registry = &self.engine.inner.registry;
        let result = if self.jit {
            bsl_vm::run_program_jit_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
            )
        } else {
            bsl_vm::run_program_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
            )
        }?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Default)]
    struct SharedWriter(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedWriter {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl SharedWriter {
        fn text(&self) -> String {
            String::from_utf8(self.0.borrow().clone()).unwrap()
        }
    }

    fn host_answer(
        _context: &mut CallContext<'_>,
        _arguments: &[Value],
    ) -> bsl_rt::RtResult<Value> {
        Ok(Value::Boolean(true))
    }

    const HOST_FUNCTIONS: &[FunctionDescriptor] = &[FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ОтветХоста", "HostAnswer"],
        arity: Arity::exact(0),
        kind: FunctionKind::Function,
        call: host_answer,
    }];

    fn host_library() -> LibraryDescriptor {
        LibraryDescriptor {
            package: "example-host",
            version: "1.0.0",
            dependencies: &[LibraryDependency {
                package: bsl_rt::PACKAGE_NAME,
                version: bsl_rt::PACKAGE_VERSION,
            }],
            functions: HOST_FUNCTIONS,
            constructors: &[],
        }
    }

    #[test]
    fn state_exec_and_eval_hide_the_internal_pipeline() {
        let engine = Engine::builder().build().unwrap();
        let mut state = engine.new_state();

        assert_eq!(state.eval("2 + 2").unwrap().to_string(), "4");
        assert!(matches!(
            state.exec("Возврат Истина;").unwrap(),
            Value::Boolean(true)
        ));
    }

    #[test]
    fn module_round_trips_through_the_public_bytecode_api() {
        let engine = Engine::builder().build().unwrap();
        let module = engine.compile("Возврат 42;").unwrap();
        let restored = engine.load_bytecode(&module.bytecode().unwrap()).unwrap();

        assert_eq!(engine.new_state().run(&restored).unwrap().to_string(), "42");
    }

    #[test]
    fn states_use_independent_stdout_and_stderr() {
        let engine = Engine::builder().build().unwrap();
        let out_a = SharedWriter::default();
        let err_a = SharedWriter::default();
        let out_b = SharedWriter::default();
        let err_b = SharedWriter::default();
        let mut state_a = engine
            .state_builder()
            .stdout(out_a.clone())
            .stderr(err_a.clone())
            .build();
        let mut state_b = engine
            .state_builder()
            .stdout(out_b.clone())
            .stderr(err_b.clone())
            .jit(true)
            .build();

        state_a.exec("Сообщить(\"a\");").unwrap();
        state_b.exec("Сообщить(\"b\");").unwrap();

        assert_eq!(out_a.text(), "a\n");
        assert_eq!(out_b.text(), "b\n");
        assert_eq!(err_a.text(), "");
        assert_eq!(err_b.text(), "");
    }

    #[test]
    fn engine_builder_exposes_a_registered_component_to_source_code() {
        let engine = Engine::builder()
            .register_library(host_library())
            .build()
            .unwrap();
        let module = engine.compile("Возврат HostAnswer();").unwrap();

        assert_eq!(module.requirements().len(), 2);
        assert_eq!(module.requirements()[1].package, "example-host");
        assert!(matches!(
            engine.new_state().run(&module).unwrap(),
            Value::Boolean(true)
        ));
    }
}
