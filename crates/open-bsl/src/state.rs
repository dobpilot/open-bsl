//! Сессия исполнения: host-сервисы и запуск модулей.

use std::io::Write;

use crate::Value;
use crate::engine::{Engine, Module};
use crate::error::Error;

/// Настройки сервисов одной сессии исполнения.
pub struct StateBuilder {
    engine: Engine,
    host: HostServices,
    jit: bool,
}

impl StateBuilder {
    pub(crate) fn new(engine: Engine) -> Self {
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
        let registry = self.engine.registry();
        // Набор символов едет в VM вместе с реестром: фрагмент
        // `Выполнить`/`Вычислить` компилируется уже во время исполнения и
        // обязан видеть тот же контекст, что и остальной модуль.
        let symbols = self.engine.preproc_symbols();
        let result = if self.jit {
            bsl_vm::run_program_jit_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
                symbols,
            )
        } else {
            bsl_vm::run_program_with_registry_and_io(
                &module.program,
                registry,
                &mut self.host.stdout,
                &mut self.host.stderr,
                symbols,
            )
        }?;
        Ok(result)
    }
}
