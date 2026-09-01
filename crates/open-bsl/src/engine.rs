//! Движок и модуль: компиляция и переносимый байт-код.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Процессный счётчик идентичности таблиц host-профилей: каждый
/// `EngineBuilder` получает свой номер, и `HostProfileId` чужого движка
/// отличим от своего.
#[cfg(not(target_arch = "wasm32"))]
static NEXT_PROFILE_NONCE: AtomicU64 = AtomicU64::new(1);

use bsl_rt::LibraryDescriptor;

use crate::dynamic::DynamicCode;
use crate::error::Error;
use crate::state::{State, StateBuilder};

/// Неизменяемая конфигурация компилятора и runtime-компонентов.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
    /// Выбранные оптимизирующие проходы компилятора. Ни один из них не
    /// прошёл ворота допуска, поэтому по умолчанию все выключены.
    optimizations: bsl_compiler::Optimizations,
    /// Собирать ли со сведениями об отладке.
    debug_info: bool,
    /// Каталог общих модулей и импортное окружение entry. Отдельное
    /// `Rc`-поле, а не часть `EngineInner`: программы каталога несут
    /// `Rc`-значения и в `Arc` не переносимы; worker фонового задания
    /// получает Send-рецепт (текстовый bytecode), а не этот объект.
    configuration: Option<std::rc::Rc<EngineConfiguration>>,
}

/// Замороженная конфигурация движка: каталог и экспортные поверхности
/// корневых модулей для `compile_entry`.
struct EngineConfiguration {
    catalog: bsl_bytecode::ConfigurationProgram,
    entry_imports: Vec<bsl_sema::ImportedModule>,
    /// Post-order файлового графа — порядок нележивой инициализации.
    init_order: Vec<u32>,
    /// Выполнять инициализацию до entry (расширение CLI), а не лениво.
    eager_init: bool,
}

struct EngineInner {
    registry: bsl_rt::RuntimeRegistry,
    symbols: bsl_syntax::PreprocSymbols,
    /// Реестр mailbox'ов временного хранилища: общий для клонов движка и
    /// его фонового runtime — задания публикуют write-set'ы сюда.
    /// Используется только нативным путём (сеансы и задания).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    temp_hub: Arc<bsl_rt::TempStorageHub>,
    /// Библиотеки, добавленные `register_library`, — worker фонового
    /// задания регистрирует их заново, чтобы линковать тот же реестр.
    #[cfg(not(target_arch = "wasm32"))]
    extra_libraries: Vec<bsl_rt::LibraryDescriptor>,
    /// Идентичность таблицы host-профилей: `HostProfileId` чужого движка
    /// отвергается по этому значению.
    #[cfg(not(target_arch = "wasm32"))]
    profile_nonce: u64,
    /// Фабрики host-профилей фоновых заданий в порядке регистрации.
    #[cfg(not(target_arch = "wasm32"))]
    job_profiles: Arc<[Arc<dyn crate::jobs::BackgroundStateFactory>]>,
    /// Внешний sink представления сообщений заданий.
    #[cfg(not(target_arch = "wasm32"))]
    job_message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
    /// Разделяемый runtime фоновых заданий: клоны одного `Engine` видят
    /// одни задания. Создаётся лениво при первом обращении; движок без
    /// заданий не платит ни рецептом, ни потоками.
    #[cfg(not(target_arch = "wasm32"))]
    job_runtime: std::sync::OnceLock<Result<Arc<crate::jobs::JobRuntime>, String>>,
    /// Конфигурация runtime, заданная до `build`.
    #[cfg(not(target_arch = "wasm32"))]
    job_config: crate::jobs::BackgroundJobConfig,
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
        let program = match self.catalog() {
            // Внутри конфигурации даже entry без импортов обязан делить
            // пространство имён каталога: его значения ходят через границы
            // модулей вместе со своими `NameId`.
            Some(catalog) => {
                let base = catalog
                    .modules
                    .first()
                    .expect("непустой каталог: build отвергает пустой рецепт");
                bsl_compiler::compile_entry_program(
                    &resolved,
                    &base.program.names,
                    &base.program.shapes,
                    self.build_options(),
                )?
            }
            None => bsl_compiler::compile_program_with(&resolved, self.build_options())?,
        };
        Ok(Module {
            id: self.next_module_id(),
            program,
        })
    }

    /// Компилирует entry с корневыми импортами каталога: квалифицированные
    /// обращения `ИмяМодуля.Метод(...)` разрешаются по его экспортным
    /// поверхностям. У движка без конфигурации ведёт себя как
    /// [`Engine::compile`].
    ///
    /// # Errors
    ///
    /// Возвращает ошибку синтаксиса, семантики или генерации байт-кода.
    pub fn compile_entry(&self, source: &str) -> Result<Module, Error> {
        let Some(catalog) = self.catalog() else {
            return self.compile(source);
        };
        let syntax = bsl_syntax::parse_with_symbols(source, &self.inner.symbols)?;
        let resolved = bsl_sema::resolve_program_with_imports(
            &syntax.items,
            &self.inner.registry,
            &self
                .configuration
                .as_ref()
                .expect("каталог проверен веткой выше")
                .entry_imports,
        )?;
        let base = catalog
            .modules
            .first()
            .expect("непустой каталог: build отвергает пустой рецепт");
        let program = bsl_compiler::compile_entry_program(
            &resolved,
            &base.program.names,
            &base.program.shapes,
            self.build_options(),
        )?;
        Ok(Module {
            id: self.next_module_id(),
            program,
        })
    }

    /// Принимает entry конфигурационного образа: программа проходит
    /// периметр `verify_configuration` против каталога этого движка и
    /// становится обычным `Module`.
    ///
    /// # Errors
    ///
    /// `Error::Configuration` без каталога; ошибки периметра образа.
    pub fn load_entry(&self, entry: bsl_bytecode::EntryProgram) -> Result<Module, Error> {
        let Some(catalog) = self.catalog() else {
            return Err(Error::Configuration(
                "entry конфигурационного образа требует движка с каталогом".to_string(),
            ));
        };
        bsl_bytecode::image::verify_configuration(catalog, Some(&entry)).map_err(Error::Runtime)?;
        Ok(Module {
            id: self.next_module_id(),
            program: entry.program,
        })
    }

    /// Печатает переносимый образ: конфигурацию с entry, если у движка
    /// есть каталог, иначе одиночную программу. `--emit-bytecode` пишет
    /// весь граф, а не набор несвязанных файлов.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку печати текстового формата.
    pub fn image_bytecode(&self, entry: &Module, source: Option<&str>) -> Result<String, Error> {
        match self.catalog() {
            Some(catalog) => {
                let image = bsl_bytecode::BytecodeImage::Configuration {
                    catalog: catalog.clone(),
                    entry: Some(bsl_bytecode::EntryProgram {
                        id: bsl_bytecode::EntryId::new(entry.id),
                        program: entry.program.clone(),
                    }),
                };
                Ok(bsl_bytecode::write_image(&image, source)?)
            }
            None => Ok(bsl_bytecode::write_program(&entry.program, source)?),
        }
    }

    /// Реестр mailbox'ов временного хранилища этого движка.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn temp_hub(&self) -> &Arc<bsl_rt::TempStorageHub> {
        &self.inner.temp_hub
    }

    /// Библиотеки, добавленные `register_library`, — для рецепта worker.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn extra_libraries(&self) -> &[bsl_rt::LibraryDescriptor] {
        &self.inner.extra_libraries
    }

    /// Фабрики host-профилей движка.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn job_profiles(&self) -> Arc<[Arc<dyn crate::jobs::BackgroundStateFactory>]> {
        Arc::clone(&self.inner.job_profiles)
    }

    /// Проверка идентификатора host-профиля: он выдан ЭТИМ движком и
    /// указывает на зарегистрированную фабрику. 0 в качестве индекса
    /// зарезервирован под системный профиль и снаружи не выдаётся.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn validate_host_profile(
        &self,
        id: crate::jobs::HostProfileId,
    ) -> Result<u32, Error> {
        let known = id.engine == self.inner.profile_nonce
            && id.index >= 1
            && ((id.index - 1) as usize) < self.inner.job_profiles.len();
        if known {
            Ok(id.index)
        } else {
            Err(Error::Configuration(
                "host-профиль не зарегистрирован в этом движке".to_string(),
            ))
        }
    }

    /// Внешний sink представления сообщений заданий.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn job_message_display(
        &self,
    ) -> Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>> {
        self.inner.job_message_display.clone()
    }

    /// Каталог общих модулей движка, если он был собран.
    pub(crate) fn catalog(&self) -> Option<&bsl_bytecode::ConfigurationProgram> {
        self.configuration.as_ref().map(|c| &c.catalog)
    }

    /// Runtime фоновых заданий этого движка. Создаётся при первом
    /// обращении; клоны движка разделяют его. Ошибка — ловимая на стороне
    /// BSL: у движка нет каталога либо рецепт не собрался.
    ///
    /// # Errors
    ///
    /// `Error::Configuration` с причиной.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn job_runtime(&self) -> Result<Arc<crate::jobs::JobRuntime>, Error> {
        self.inner
            .job_runtime
            .get_or_init(|| {
                crate::jobs::runtime_for_engine(self, self.inner.job_config.clone()).map(Arc::new)
            })
            .clone()
            .map_err(Error::Configuration)
    }

    /// Порядок нележивой инициализации, если она включена рецептом.
    pub(crate) fn eager_init_order(&self) -> Option<&[u32]> {
        self.configuration
            .as_ref()
            .filter(|c| c.eager_init)
            .map(|c| c.init_order.as_slice())
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
    /// Опции сборки этого движка. Собираются В ОДНОМ месте, чтобы три
    /// точки компиляции не разошлись в том, что именно запрошено.
    fn build_options(&self) -> bsl_compiler::BuildOptions {
        bsl_compiler::BuildOptions {
            optimizations: self.optimizations,
            debug_info: self.debug_info,
        }
    }

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

/// Описание одного общего модуля будущего каталога: стабильное имя,
/// исходник и импорты по именам других модулей рецепта.
#[derive(Debug, Clone)]
pub struct ModuleRecipe {
    pub name: String,
    pub source: String,
    /// Пары «псевдоним -> имя модуля рецепта». Обычно псевдоним совпадает
    /// с именем; отдельный нужен файловому загрузчику `bsl-cli`, где
    /// вложенные псевдонимы локальны импортёру.
    pub imports: Vec<(String, String)>,
}

/// Не зависящее от файловой системы описание графа общих модулей.
/// Строится загрузчиком (`bsl-cli` для `//@используй`) либо host-кодом и
/// передаётся в [`EngineBuilder::configuration`] до `build`.
#[derive(Debug, Clone, Default)]
pub struct ModuleGraphRecipe {
    pub modules: Vec<ModuleRecipe>,
    /// Инициализировать тела модулей до первой инструкции entry, в
    /// post-order файлового графа. Семантика расширения CLI
    /// `//@используй`; по умолчанию инициализация ленивая — при первом
    /// касании символа модуля.
    pub eager_init: bool,
}

/// Стадия композиции статически связанных runtime-компонентов.
pub struct EngineBuilder {
    optimizations: bsl_compiler::Optimizations,
    debug_info: bool,
    runtime: bsl_rt::RuntimeBuilder,
    symbols: bsl_syntax::PreprocSymbols,
    recipe: ModuleGraphRecipe,
    image: Option<(bsl_bytecode::ConfigurationProgram, bool)>,
    #[cfg(not(target_arch = "wasm32"))]
    job_config: crate::jobs::BackgroundJobConfig,
    #[cfg(not(target_arch = "wasm32"))]
    extra_libraries: Vec<bsl_rt::LibraryDescriptor>,
    #[cfg(not(target_arch = "wasm32"))]
    profile_nonce: u64,
    #[cfg(not(target_arch = "wasm32"))]
    job_profiles: Vec<Arc<dyn crate::jobs::BackgroundStateFactory>>,
    #[cfg(not(target_arch = "wasm32"))]
    job_message_display: Option<Arc<dyn bsl_rt::UserMessageSink + Send + Sync>>,
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
            optimizations: bsl_compiler::Optimizations::default(),
            debug_info: false,
            runtime,
            symbols: bsl_syntax::PreprocSymbols::new(),
            recipe: ModuleGraphRecipe::default(),
            image: None,
            #[cfg(not(target_arch = "wasm32"))]
            job_config: crate::jobs::BackgroundJobConfig::default(),
            #[cfg(not(target_arch = "wasm32"))]
            extra_libraries: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            profile_nonce: NEXT_PROFILE_NONCE.fetch_add(1, Ordering::Relaxed),
            #[cfg(not(target_arch = "wasm32"))]
            job_profiles: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            job_message_display: None,
        }
    }

    /// Конфигурация фонового runtime. Валидация — при сборке движка,
    /// без скрытых clamp.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn background_jobs(mut self, config: crate::jobs::BackgroundJobConfig) -> Self {
        self.job_config = config;
        self
    }

    pub fn register_library(mut self, library: LibraryDescriptor) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        self.extra_libraries.push(library);
        self.runtime.register(library);
        self
    }

    /// Регистрирует host-профиль фоновых заданий и возвращает его
    /// непрозрачный идентификатор — его выбирает
    /// [`crate::StateBuilder::host_profile`]. `&mut self`, а не
    /// builder-цепочка: идентификатор нужен вызывающему.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_host_profile(
        &mut self,
        factory: Arc<dyn crate::jobs::BackgroundStateFactory>,
    ) -> crate::jobs::HostProfileId {
        self.job_profiles.push(factory);
        crate::jobs::HostProfileId {
            engine: self.profile_nonce,
            index: self.job_profiles.len() as u32,
        }
    }

    /// Внешний sink представления сообщений фоновых заданий: история
    /// записи реестра пишется до него и при его backpressure не теряется.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn job_message_sink(
        mut self,
        sink: Arc<dyn bsl_rt::UserMessageSink + Send + Sync>,
    ) -> Self {
        self.job_message_display = Some(sink);
        self
    }

    /// Полная замена символов условной компиляции — путь рецепта worker,
    /// который воспроизводит символы родительского движка.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub(crate) fn preproc_symbols_all(mut self, symbols: bsl_syntax::PreprocSymbols) -> Self {
        self.symbols = symbols;
        self
    }

    /// Передаёт весь граф общих модулей разом. Модули уже добавленные
    /// `common_module` сохраняются; каталог замораживается в `build`.
    #[must_use]
    pub fn configuration(mut self, recipe: ModuleGraphRecipe) -> Self {
        self.recipe.modules.extend(recipe.modules);
        self.recipe.eager_init |= recipe.eager_init;
        self
    }

    /// Принимает уже разобранный каталог конфигурации — путь
    /// `--run-bytecode` и worker фонового задания. Каталог проходит тот же
    /// периметр `verify_configuration`, что и скомпилированный из рецепта;
    /// сочетание с `configuration`/`common_module` — ошибка сборки.
    #[must_use]
    pub fn configuration_image(
        mut self,
        catalog: bsl_bytecode::ConfigurationProgram,
        eager_init: bool,
    ) -> Self {
        self.image = Some((catalog, eager_init));
        self
    }

    /// Добавляет общий модуль без импортов — сокращение для host-кода,
    /// которому не нужен полный рецепт.
    #[must_use]
    pub fn common_module(mut self, name: &str, source: &str) -> Self {
        self.recipe.modules.push(ModuleRecipe {
            name: name.to_string(),
            source: source.to_string(),
            imports: Vec::new(),
        });
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
    /// Включить оптимизирующие проходы компилятора.
    ///
    /// По умолчанию выключены все: ни свёртка констант, ни устранение
    /// копий ещё не проходили ворота допуска из
    /// `docs/research/performance/ssa-hotspot-analysis.md`, поэтому включение — осознанный
    /// выбор вызывающего, а не поведение по умолчанию.
    pub fn optimizations(mut self, opts: bsl_compiler::Optimizations) -> Self {
        self.optimizations = opts;
        self
    }

    /// Собирать со сведениями об отладке: таблица строк исходника и имена
    /// локальных переменных у всех чанков.
    ///
    /// Образ от этого заметно растёт, поэтому по умолчанию выключено.
    /// Несовместимо с оптимизацией, УДАЛЯЮЩЕЙ инструкции: строку из
    /// байт-кода не вывести, её можно только донести, поэтому сочетание
    /// отвергается на компиляции, а не выбирает молча одно из двух.
    #[must_use]
    pub fn debug_info(mut self, enabled: bool) -> Self {
        self.debug_info = enabled;
        self
    }

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
        let registry = self.runtime.build()?;
        if self.image.is_some() && !self.recipe.modules.is_empty() {
            return Err(Error::Configuration(
                "каталог задан и рецептом, и образом — источник должен быть один".to_string(),
            ));
        }
        let configuration = if let Some((catalog, eager_init)) = self.image {
            bsl_bytecode::image::verify_configuration(&catalog, None).map_err(Error::Runtime)?;
            let entry_imports = catalog
                .modules
                .iter()
                .enumerate()
                .map(|(i, module)| {
                    imported_module_from_program(&module.name, i as u32, &module.program)
                })
                .collect();
            let init_order = image_init_order(&catalog);
            Some(std::rc::Rc::new(EngineConfiguration {
                catalog,
                entry_imports,
                init_order,
                eager_init,
            }))
        } else if self.recipe.modules.is_empty() {
            None
        } else {
            let (catalog, entry_imports, init_order) = compile_catalog(
                &self.recipe,
                &registry,
                &self.symbols,
                bsl_compiler::BuildOptions {
                    optimizations: self.optimizations,
                    debug_info: self.debug_info,
                },
            )?;
            Some(std::rc::Rc::new(EngineConfiguration {
                catalog,
                entry_imports,
                init_order,
                eager_init: self.recipe.eager_init,
            }))
        };
        #[cfg(not(target_arch = "wasm32"))]
        self.job_config.validate().map_err(Error::Configuration)?;
        Ok(Engine {
            optimizations: self.optimizations,
            debug_info: self.debug_info,
            inner: Arc::new(EngineInner {
                registry,
                symbols: self.symbols,
                temp_hub: Arc::new(bsl_rt::TempStorageHub::default()),
                modules: AtomicU64::new(0),
                #[cfg(not(target_arch = "wasm32"))]
                job_runtime: std::sync::OnceLock::new(),
                #[cfg(not(target_arch = "wasm32"))]
                job_config: self.job_config,
                #[cfg(not(target_arch = "wasm32"))]
                extra_libraries: self.extra_libraries,
                #[cfg(not(target_arch = "wasm32"))]
                profile_nonce: self.profile_nonce,
                #[cfg(not(target_arch = "wasm32"))]
                job_profiles: Arc::from(self.job_profiles),
                #[cfg(not(target_arch = "wasm32"))]
                job_message_display: self.job_message_display,
            }),
            configuration,
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
    /// Строки исходника, на которых есть хотя бы одна инструкция.
    ///
    /// Пусто, если модуль собран без сведений об отладке. Нужна отладчику,
    /// чтобы не подтверждать точку останова на строке, где остановки не
    /// будет никогда: пустой, комментарии, `КонецЕсли`.
    ///
    /// Отдаётся именно множество строк, а не образ: образ — не часть
    /// публичного договора встраивания, а это отладочные сведения о
    /// собственном исходнике хоста.
    #[must_use]
    pub fn executable_lines(&self) -> std::collections::HashSet<u32> {
        self.program
            .lines
            .iter()
            .flat_map(|rows| rows.iter().copied())
            .filter(|&l| l > 0)
            .collect()
    }

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

/// Компилирует каталог по рецепту: topo-порядок для резолвинга, порядок
/// манифеста для номеров модулей, общий интернер имён и форм, периметр
/// `verify_configuration` до заморозки.
fn compile_catalog(
    recipe: &ModuleGraphRecipe,
    registry: &bsl_rt::RuntimeRegistry,
    symbols: &bsl_syntax::PreprocSymbols,
    build: bsl_compiler::BuildOptions,
) -> Result<
    (
        bsl_bytecode::ConfigurationProgram,
        Vec<bsl_sema::ImportedModule>,
        Vec<u32>,
    ),
    Error,
> {
    let modules = &recipe.modules;
    // Имена каталога уникальны без учёта регистра — правило свёртки то же,
    // что у остальных имён BSL.
    for (i, module) in modules.iter().enumerate() {
        if module.name.is_empty() {
            return Err(Error::Configuration("имя общего модуля пусто".to_string()));
        }
        if modules[..i]
            .iter()
            .any(|other| bsl_rt::folded_eq(&other.name, &module.name))
        {
            return Err(Error::Configuration(format!(
                "имя общего модуля «{}» повторяется",
                module.name
            )));
        }
    }
    let index_of = |name: &str| -> Option<usize> {
        modules
            .iter()
            .position(|module| bsl_rt::folded_eq(&module.name, name))
    };

    // DFS post-order по импортам: зависимости раньше зависимых, при
    // нескольких — в порядке следования импортов, корни — в порядке
    // рецепта. Этот же порядок служит нележивой инициализации
    // (`//@используй`); цикл — ошибка конфигурации, как и импорт
    // неизвестного модуля.
    fn visit(
        modules: &[ModuleRecipe],
        index_of: &dyn Fn(&str) -> Option<usize>,
        state: &mut [u8],
        order: &mut Vec<usize>,
        current: usize,
    ) -> Result<(), Error> {
        match state[current] {
            1 => {
                return Err(Error::Configuration(
                    "граф импортов общих модулей содержит цикл".to_string(),
                ));
            }
            2 => return Ok(()),
            _ => {}
        }
        state[current] = 1;
        for (_, target) in &modules[current].imports {
            let Some(target_index) = index_of(target) else {
                return Err(Error::Configuration(format!(
                    "модуль «{}» импортирует неизвестный «{target}»",
                    modules[current].name
                )));
            };
            visit(modules, index_of, state, order, target_index)?;
        }
        state[current] = 2;
        order.push(current);
        Ok(())
    }
    let mut state = vec![0u8; modules.len()];
    let mut topo: Vec<usize> = Vec::with_capacity(modules.len());
    for i in 0..modules.len() {
        visit(modules, &index_of, &mut state, &mut topo, i)?;
    }

    let mut resolved: Vec<Option<bsl_sema::ResolvedProgram>> = vec![None; modules.len()];
    for &i in &topo {
        let module = &modules[i];
        let imports: Vec<bsl_sema::ImportedModule> = module
            .imports
            .iter()
            .map(|(alias, target)| {
                let target_index = index_of(target).expect("проверено при построении графа выше");
                bsl_sema::ImportedModule::from_resolved(
                    alias,
                    target_index as u32,
                    resolved[target_index]
                        .as_ref()
                        .expect("topo-порядок: цель разрешена раньше импортёра"),
                )
            })
            .collect();
        let syntax = bsl_syntax::parse_with_symbols(&module.source, symbols)?;
        resolved[i] = Some(bsl_sema::resolve_program_with_imports(
            &syntax.items,
            registry,
            &imports,
        )?);
    }
    let resolved: Vec<bsl_sema::ResolvedProgram> = resolved
        .into_iter()
        .map(|module| module.expect("каждый модуль разрешён topo-обходом"))
        .collect();

    let pairs: Vec<(String, &bsl_sema::ResolvedProgram)> = modules
        .iter()
        .zip(&resolved)
        .map(|(module, resolved)| (module.name.clone(), resolved))
        .collect();
    let (catalog, _) = bsl_compiler::compile_configuration(&pairs, None, build)?;
    bsl_bytecode::image::verify_configuration(&catalog, None).map_err(Error::Runtime)?;

    let entry_imports = modules
        .iter()
        .zip(&resolved)
        .enumerate()
        .map(|(i, (module, resolved))| {
            bsl_sema::ImportedModule::from_resolved(&module.name, i as u32, resolved)
        })
        .collect();
    let init_order = topo.iter().map(|&i| i as u32).collect();
    Ok((catalog, entry_imports, init_order))
}

/// Экспортная поверхность модуля, снятая с его ПРОГРАММЫ, — для движка,
/// собранного из образа: `ResolvedProgram` у него нет, а всё нужное
/// (имена, экспортность, сигнатуры чанков) байт-код уже несёт.
fn imported_module_from_program(
    alias: &str,
    module: u32,
    program: &bsl_bytecode::Program,
) -> bsl_sema::ImportedModule {
    let functions = program
        .function_names
        .iter()
        .enumerate()
        .filter(|(i, _)| program.exported_functions[*i])
        .map(|(i, name)| {
            let chunk = &program.chunks[i + 1];
            bsl_sema::ImportedFunction {
                name: name.clone(),
                chunk: (i + 1) as u16,
                is_procedure: chunk.is_procedure,
                is_async: chunk.is_async,
                param_by_val: chunk.param_by_val.clone(),
                param_has_default: chunk.param_has_default.clone(),
            }
        })
        .collect();
    let variables = program
        .module_vars
        .iter()
        .enumerate()
        .filter(|(i, _)| program.exported_module_vars[*i])
        .map(|(i, name)| bsl_sema::ImportedVariable {
            name: name.clone(),
            slot: i as u16,
        })
        .collect();
    bsl_sema::ImportedModule {
        alias: alias.to_string(),
        module,
        functions,
        variables,
    }
}

/// Порядок нележивой инициализации для каталога из образа: DFS post-order
/// по таблицам связей в порядке манифеста. Порядок следования директив
/// исходного файла образ не хранит, поэтому здесь допустимо отличие от
/// прогона исходника при независимых поддеревьях — зависимости по-прежнему
/// раньше зависимых.
fn image_init_order(catalog: &bsl_bytecode::ConfigurationProgram) -> Vec<u32> {
    fn visit(
        catalog: &bsl_bytecode::ConfigurationProgram,
        state: &mut [u8],
        order: &mut Vec<u32>,
        current: usize,
    ) {
        if state[current] != 0 {
            return;
        }
        state[current] = 1;
        for link in &catalog.modules[current].program.links {
            let target = match link {
                bsl_bytecode::LinkEntry::Function { module, .. }
                | bsl_bytecode::LinkEntry::Variable { module, .. } => module.index(),
            };
            visit(catalog, state, order, target);
        }
        state[current] = 2;
        order.push(current as u32);
    }
    let mut state = vec![0u8; catalog.modules.len()];
    let mut order = Vec::with_capacity(catalog.modules.len());
    for i in 0..catalog.modules.len() {
        visit(catalog, &mut state, &mut order, i);
    }
    order
}

#[cfg(test)]
mod debug_info_tests {
    //! Сведения об отладке через фасад. Тесты внутренние, потому что
    //! `Module::program` наружу не показывается намеренно: образ не часть
    //! публичного договора встраивания, и открывать его ради проверки
    //! значило бы менять договор ради теста.

    #[test]
    fn the_builder_can_ask_for_debug_info() {
        let engine = crate::Engine::builder()
            .debug_info(true)
            .build()
            .expect("движок");
        let module = engine
            .compile("а = 1;\nб = 2;\n")
            .expect("компиляция со сведениями об отладке");
        let program = &module.program;
        assert_eq!(program.lines.len(), program.chunks.len());
        let mut seen: Vec<u32> = program.lines[0].clone();
        seen.dedup();
        assert_eq!(seen, vec![1, 2]);
    }

    #[test]
    fn without_asking_the_image_carries_no_lines() {
        let engine = crate::Engine::builder().build().expect("движок");
        let module = engine.compile("а = 1;\n").expect("компиляция");
        assert!(module.program.lines.is_empty());
    }

    #[test]
    fn debug_info_with_a_pass_that_breaks_it_is_refused_through_the_facade() {
        // Обе оптимизации теряют невосстановимое: `copy-elim` удаляет
        // инструкции и рассогласовывает таблицу строк, `ssa-regalloc`
        // переставляет слоты, а имена остаются в исходном порядке — и
        // отладчик показал бы чужое значение молча и правдоподобно.
        for opts in [
            bsl_compiler::Optimizations {
                copy_elim: true,
                ..bsl_compiler::Optimizations::default()
            },
            bsl_compiler::Optimizations {
                ssa_regalloc: true,
                ..bsl_compiler::Optimizations::default()
            },
        ] {
            let engine = crate::Engine::builder()
                .debug_info(true)
                .optimizations(opts)
                .build()
                .expect("движок");
            // `Module` не `Debug`, поэтому разбор случая, а не `expect_err`.
            let Err(err) = engine.compile("а = 1;\n") else {
                panic!("сочетание обязано отвергаться и через фасад: {opts:?}");
            };
            assert!(
                format!("{err}").contains("несовместимы с этой оптимизацией"),
                "{opts:?}: {err}"
            );
        }
    }
}
