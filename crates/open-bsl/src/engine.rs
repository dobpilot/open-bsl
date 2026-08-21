//! Движок и модуль: компиляция и переносимый байт-код.

use std::sync::Arc;

use bsl_rt::LibraryDescriptor;

use crate::error::Error;
use crate::state::{State, StateBuilder};

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

    /// Каталог компонентов движка — для клиентов, собирающих фрагменты
    /// самостоятельно (REPL с накоплением локалей).
    pub fn registry(&self) -> &bsl_rt::RuntimeRegistry {
        &self.inner.registry
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
        #[cfg(feature = "binbuf")]
        runtime.register(bsl_binbuf::library());
        #[cfg(feature = "regexp")]
        runtime.register(bsl_regexp::library());
        #[cfg(feature = "textdoc")]
        runtime.register(bsl_textdoc::library());
        #[cfg(feature = "json")]
        runtime.register(bsl_json::library());
        #[cfg(feature = "stream")]
        runtime.register(bsl_stream::library());
        #[cfg(feature = "zip")]
        runtime.register(bsl_zip::library());
        #[cfg(feature = "pdf")]
        runtime.register(bsl_pdf::library());
        #[cfg(feature = "xml")]
        runtime.register(bsl_xml::library());
        #[cfg(feature = "spreadsheet")]
        runtime.register(bsl_spreadsheet::library());
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
    pub(crate) program: bsl_bytecode::Program,
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
    /// Печатает байт-код вместе с текстом исходника в листинге.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку печати текстового формата.
    pub fn bytecode_with_source(&self, source: &str) -> Result<String, Error> {
        bsl_bytecode::write_program(&self.program, Some(source)).map_err(Error::Bytecode)
    }

    pub fn bytecode(&self) -> Result<String, Error> {
        Ok(bsl_bytecode::write_program(&self.program, None)?)
    }
}
