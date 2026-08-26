//! Движок и модуль: компиляция и переносимый байт-код.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bsl_rt::LibraryDescriptor;

use crate::dynamic::DynamicCode;
use crate::error::Error;
use crate::state::{State, StateBuilder};

/// Неизменяемая конфигурация компилятора и runtime-компонентов.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    registry: bsl_rt::RuntimeRegistry,
    symbols: bsl_syntax::PreprocSymbols,
    /// Сколько модулей движок уже выдал. Номер модуля устойчив: он
    /// присваивается один раз, при компиляции, и не меняется от запуска к
    /// запуску — на этом стоит кэш динамических фрагментов сессии
    /// (`DynamicCode`), который иначе накапливал бы недостижимые записи на
    /// каждый повторный `State::run`.
    modules: AtomicU64,
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
        let syntax = bsl_syntax::parse_with_symbols(source, &self.inner.symbols)?;
        let resolved =
            bsl_sema::resolve_program_with_registry(&syntax.items, &self.inner.registry)?;
        let program = bsl_compiler::compile_program(&resolved)?;
        Ok(Module {
            id: self.next_module_id(),
            program,
        })
    }

    /// Загружает текстовый байт-код. Совместимость компонентов проверяется
    /// при запуске, до первой инструкции.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку формата байт-кода.
    pub fn load_bytecode(&self, source: &str) -> Result<Module, Error> {
        Ok(Module {
            id: self.next_module_id(),
            program: bsl_bytecode::parse_program(source)?,
        })
    }

    /// Номер очередного модуля. `Relaxed` достаточно: от счётчика нужна
    /// только различимость, а не порядок относительно других записей.
    fn next_module_id(&self) -> u64 {
        self.inner.modules.fetch_add(1, Ordering::Relaxed)
    }

    /// Каталог компонентов движка — для клиентов, собирающих фрагменты
    /// самостоятельно (REPL с накоплением локалей).
    pub fn registry(&self) -> &bsl_rt::RuntimeRegistry {
        &self.inner.registry
    }

    pub fn new_state(&self) -> State {
        self.state_builder().build()
    }

    /// Компилятор `Выполнить`/`Вычислить` с этим каталогом компонентов и
    /// этими символами условной компиляции.
    ///
    /// Нужен клиентам, которые запускают чанки в обход [`State`], — REPL с
    /// накоплением локалей. У обычного встраивания свой такой компилятор
    /// уже есть внутри `State`, и заводить второй не надо: у каждого свой
    /// кэш фрагментов.
    #[must_use]
    pub fn dynamic_code(&self) -> DynamicCode {
        DynamicCode::new(self.clone())
    }

    /// Символы условной компиляции этого движка. Внутренний сервис:
    /// единственный вызывающий — `DynamicCode::compile` (см. `dynamic.rs`),
    /// в публичной сигнатуре не встречается, поэтому `pub(crate)`.
    pub(crate) fn preproc_symbols(&self) -> bsl_syntax::PreprocSymbols {
        self.inner.symbols
    }

    pub fn state_builder(&self) -> StateBuilder {
        StateBuilder::new(self.clone())
    }
}

/// Стадия композиции статически связанных runtime-компонентов.
pub struct EngineBuilder {
    runtime: bsl_rt::RuntimeBuilder,
    symbols: bsl_syntax::PreprocSymbols,
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
        #[cfg(feature = "http")]
        runtime.register(bsl_http::library());
        #[cfg(feature = "crypto")]
        runtime.register(bsl_crypto::library());
        #[cfg(feature = "zip")]
        runtime.register(bsl_zip::library());
        #[cfg(feature = "pdf")]
        runtime.register(bsl_pdf::library());
        #[cfg(feature = "xml")]
        runtime.register(bsl_xml::library());
        #[cfg(feature = "spreadsheet")]
        runtime.register(bsl_spreadsheet::library());
        Self {
            runtime,
            symbols: bsl_syntax::PreprocSymbols::new(),
        }
    }

    pub fn register_library(mut self, library: LibraryDescriptor) -> Self {
        self.runtime.register(library);
        self
    }

    /// Включает или выключает символ условной компиляции (`#Если Клиент`).
    ///
    /// По умолчанию истинны `Сервер`, `НаСервере` и `ВнешнееСоединение`:
    /// open-bsl — это BSL, исполняемый внешней программой без интерфейса.
    /// Хост, который и правда клиентское приложение, вправе сказать это
    /// движку. Русское и английское написания — один символ, а не два.
    /// Незнакомое имя игнорируется: в условии оно и без того ложно.
    #[must_use]
    pub fn preproc_symbol(mut self, name: &str, value: bool) -> Self {
        self.symbols.set(name, value);
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
                symbols: self.symbols,
                modules: AtomicU64::new(0),
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
    /// Номер, выданный движком при компиляции. В байт-код не входит и
    /// наружу не показывается: он нужен ровно затем, чтобы сессия
    /// отличала области ОДНОГО модуля от областей другого, запущенного той
    /// же сессией. Клон модуля номер сохраняет — это тот же модуль.
    pub(crate) id: u64,
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
