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

#[path = "engine_preparation.rs"]
mod preparation;

#[path = "engine_on_demand.rs"]
pub(crate) mod on_demand;

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
    indexed: Option<std::rc::Rc<on_demand::OnDemandCatalog>>,
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

/// Источник свойств получателя модуля, подготавливаемых вместе с его телом.
/// Метаданные и исходники могут поступать из разных хранилищ.
pub trait ModuleContextSource: Send + Sync + std::fmt::Debug {
    /// Приобретает метаданные уже достигнутой группы модулей без исполнения BSL.
    /// По умолчанию каждый контекст приобретается при отдельном обращении.
    ///
    /// # Errors
    /// Возвращает ошибку приобретения необходимых метаданных группы.
    fn prepare_contexts(&self, _modules: &[&str]) -> Result<(), String> {
        Ok(())
    }

    /// Возвращает имена свойств собственного объекта или менеджера модуля.
    ///
    /// # Errors
    /// Возвращает ошибку получения необходимых метаданных получателя.
    fn receiver_properties(&self, module: &str) -> Result<Vec<String>, String>;
}

struct EngineInner {
    registry: Arc<bsl_rt::RuntimeRegistry>,
    /// Собственные методы сохраняются для компиляции динамических фрагментов.
    method_contexts: Vec<(String, &'static [bsl_rt::MethodDescriptor])>,
    property_contexts: Vec<(String, Arc<[String]>)>,
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
        let resolved = bsl_sema::resolve_program_with(
            &syntax.items,
            &bsl_sema::ResolveOptions::new(&self.inner.registry),
        )?;
        let program = if let Some(indexed) = &self.indexed {
            indexed.compile_resolved(&resolved)?
        } else {
            match self.catalog() {
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
            }
        };
        Ok(Module {
            id: self.next_module_id(),
            owner: Arc::downgrade(&self.inner),
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
        if let Some(indexed) = &self.indexed {
            return Ok(Module {
                id: self.next_module_id(),
                owner: Arc::downgrade(&self.inner),
                program: indexed.compile_entry(source)?,
            });
        }
        let Some(catalog) = self.catalog() else {
            return self.compile(source);
        };
        let syntax = bsl_syntax::parse_with_symbols(source, &self.inner.symbols)?;
        let resolved = bsl_sema::resolve_program_with(
            &syntax.items,
            &bsl_sema::ResolveOptions::new(&self.inner.registry).imports(
                &self
                    .configuration
                    .as_ref()
                    .expect("каталог проверен веткой выше")
                    .entry_imports,
            ),
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
            owner: Arc::downgrade(&self.inner),
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
        if let Some(catalog) = &self.indexed {
            bsl_bytecode::image::verify_linked_program(&entry.program, catalog.as_ref(), None)?;
            return Ok(Module {
                id: self.next_module_id(),
                owner: Arc::downgrade(&self.inner),
                program: entry.program,
            });
        }
        let Some(catalog) = self.catalog() else {
            return Err(Error::Configuration(
                "entry конфигурационного образа требует движка с каталогом".to_string(),
            ));
        };
        bsl_bytecode::image::verify_configuration(catalog, Some(&entry)).map_err(Error::Runtime)?;
        Ok(Module {
            id: self.next_module_id(),
            owner: Arc::downgrade(&self.inner),
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn indexed_worker_source(&self) -> Option<on_demand::WorkerSource> {
        self.indexed.as_ref().map(|catalog| catalog.worker_source())
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

    pub(crate) fn receiver_methods(
        &self,
        module: bsl_bytecode::ModuleId,
    ) -> &'static [bsl_rt::MethodDescriptor] {
        let Some(target) = self
            .execution_catalog()
            .and_then(|catalog| catalog.module_name(module))
        else {
            return &[];
        };
        self.inner
            .method_contexts
            .iter()
            .find(|(name, _)| bsl_rt::folded_eq(name, target))
            .map_or(&[], |(_, methods)| *methods)
    }

    pub(crate) fn method_contexts(&self) -> &[(String, &'static [bsl_rt::MethodDescriptor])] {
        &self.inner.method_contexts
    }

    pub(crate) fn receiver_properties(&self, module: bsl_bytecode::ModuleId) -> &[String] {
        if let Some(properties) = self
            .indexed
            .as_ref()
            .and_then(|catalog| catalog.prepared_properties(module))
        {
            return properties;
        }
        self.execution_catalog()
            .and_then(|catalog| catalog.module_name(module))
            .map_or(&[], |target| {
                receiver_properties(&self.inner.property_contexts, target)
            })
    }

    pub(crate) fn property_contexts(&self) -> &[(String, Arc<[String]>)] {
        &self.inner.property_contexts
    }

    /// Каталог общих модулей движка, если он был собран.
    pub(crate) fn catalog(&self) -> Option<&bsl_bytecode::ConfigurationProgram> {
        self.configuration.as_ref().map(|c| &c.catalog)
    }

    pub(crate) fn execution_catalog(&self) -> Option<&dyn bsl_bytecode::ExecutionCatalog> {
        self.indexed
            .as_deref()
            .map(|catalog| catalog as &dyn bsl_bytecode::ExecutionCatalog)
            .or_else(|| {
                self.catalog()
                    .map(|catalog| catalog as &dyn bsl_bytecode::ExecutionCatalog)
            })
    }

    pub(crate) fn imported_provider(&self) -> Option<&dyn bsl_sema::ImportedModuleProvider> {
        self.indexed
            .as_deref()
            .map(|catalog| catalog as &dyn bsl_sema::ImportedModuleProvider)
    }

    pub(crate) fn dynamic_imports(
        &self,
        imports: &[bsl_bytecode::ModuleImport],
    ) -> Result<Vec<bsl_sema::ImportedModule>, String> {
        if let Some(indexed) = &self.indexed {
            return indexed.dynamic_imports(imports);
        }
        imports
            .iter()
            .map(|import| {
                let target = self
                    .catalog()
                    .and_then(|catalog| catalog.module(import.module))
                    .ok_or_else(|| {
                        "импорт динамического фрагмента отсутствует в каталоге".to_string()
                    })?;
                Ok(imported_module_from_program(
                    &import.alias,
                    import.module.index() as u32,
                    &target.program,
                    import.global,
                ))
            })
            .collect()
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
            owner: Arc::downgrade(&self.inner),
            program: bsl_bytecode::parse_program(source)?,
        })
    }

    pub(crate) fn owns_module(&self, module: &Module) -> bool {
        std::sync::Weak::ptr_eq(&module.owner, &Arc::downgrade(&self.inner))
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
    /// Видимость модуля для остальных модулей конфигурации.
    pub visibility: ModuleVisibility,
}

/// Видимость экспортов модуля в статическом каталоге конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleVisibility {
    /// Модуль виден entry и только явно импортировавшим его модулям.
    #[default]
    Explicit,
    /// Модуль автоматически виден под своим именем.
    Named,
    /// Модуль виден под своим именем, а его экспорты — также без квалификатора.
    Global,
    /// Модуль доступен только через host-мост и явные импорты.
    Private,
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
    defer_common_module_name_errors: bool,
    runtime: bsl_rt::RuntimeBuilder,
    symbols: bsl_syntax::PreprocSymbols,
    recipe: ModuleGraphRecipe,
    indexed_source: Option<Arc<dyn bsl_config::ConfigurationSource>>,
    indexed_source_conflict: bool,
    module_context_source: Option<Arc<dyn ModuleContextSource>>,
    configuration_roots: Vec<String>,
    configuration_entry_sources: Vec<String>,
    component_modules: Vec<(String, String, String)>,
    method_contexts: Vec<(String, &'static [bsl_rt::MethodDescriptor])>,
    property_contexts: Vec<(String, Arc<[String]>)>,
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
            defer_common_module_name_errors: false,
            runtime,
            symbols: bsl_syntax::PreprocSymbols::new(),
            recipe: ModuleGraphRecipe::default(),
            indexed_source: None,
            indexed_source_conflict: false,
            module_context_source: None,
            configuration_roots: Vec::new(),
            configuration_entry_sources: Vec::new(),
            component_modules: Vec::new(),
            method_contexts: Vec::new(),
            property_contexts: Vec::new(),
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

    /// Откладывает ошибки неизвестных имён общих модулей до обращения к ним.
    /// По умолчанию компиляция строга; политика действует только на `Named`
    /// и `Global`, сохраняя строгую диагностику остальных модулей.
    /// Неизвестная переменная, глобальная функция или конструктор в теле
    /// метода запрещает весь модуль при обращении к нему;
    /// отсутствующий метод известного импорта — только вызывающий метод.
    #[must_use]
    pub fn defer_common_module_name_errors(mut self, enabled: bool) -> Self {
        self.defer_common_module_name_errors = enabled;
        self
    }

    pub fn register_library(mut self, library: LibraryDescriptor) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        self.extra_libraries.push(library);
        self.runtime.register(library);
        self
    }

    /// Объявляет тип значения, зависящий от конфигурации хоста.
    ///
    /// Описание сохраняется в неизменяемом каталоге движка; конфликты имён
    /// и идентичностей возвращаются методом [`Self::build`].
    #[must_use]
    pub fn register_value_type(mut self, ty: Arc<bsl_rt::ValueTypeDescriptor>) -> Self {
        self.runtime.register_value_type(ty);
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

    /// Задаёт индекс конфигурации с загрузкой исходников по требованию.
    /// Корни и входные тексты готовятся при сборке; остальные тела — при
    /// первом обращении. У источника остаётся единственный эффективный текст.
    #[must_use]
    pub fn configuration_source(
        mut self,
        source: Arc<dyn bsl_config::ConfigurationSource>,
    ) -> Self {
        self.indexed_source_conflict |=
            self.indexed_source.is_some() || !self.recipe.modules.is_empty();
        self.recipe.modules = source
            .index()
            .modules()
            .iter()
            .filter(|module| executes_on_server(module))
            .map(|module| ModuleRecipe {
                name: module.name().into(),
                source: String::new(),
                imports: module
                    .imports()
                    .iter()
                    .map(|import| {
                        let name = source
                            .index()
                            .module(import.module())
                            .expect("проверенный индекс")
                            .name();
                        (import.alias().into(), name.into())
                    })
                    .collect(),
                visibility: match module.kind() {
                    bsl_config::ModuleKind::Common if module.execution().global() => {
                        ModuleVisibility::Global
                    }
                    bsl_config::ModuleKind::Common => ModuleVisibility::Named,
                    bsl_config::ModuleKind::Session => ModuleVisibility::Explicit,
                    _ => ModuleVisibility::Private,
                },
            })
            .collect();
        self.indexed_source = Some(source);
        self
    }

    /// Задаёт адресный источник свойств получателей индексного каталога.
    #[must_use]
    pub fn configuration_context_source(mut self, source: Arc<dyn ModuleContextSource>) -> Self {
        self.module_context_source = Some(source);
        self
    }

    /// Задаёт корневые модули подготовки каталога.
    ///
    /// Для готового рецепта компилируется его статическое замыкание. У
    /// индексного источника готовятся корни и явные импорты; объявления
    /// остальных целей проверяются при связывании, а тела — при обращении.
    /// Пустой список сохраняет компиляцию всего каталога. Имена и зависимости
    /// определяет штатный семантический резолвер open-bsl.
    #[must_use]
    pub fn configuration_roots(
        mut self,
        roots: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.configuration_roots
            .extend(roots.into_iter().map(Into::into));
        self
    }

    /// Учитывает семантические зависимости известных входных текстов при
    /// выборе замыкания [`Self::configuration_roots`]. Тексты не исполняются
    /// и не становятся модулями каталога; без фильтра корней каталог уже полон.
    /// Имена и затенение разрешает тот же резолвер, что у [`Engine::compile_entry`].
    #[must_use]
    pub fn configuration_entry_sources(
        mut self,
        sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.configuration_entry_sources
            .extend(sources.into_iter().map(Into::into));
        self
    }

    /// Связывает свойство компонентного глобала с закрытым модулем каталога.
    ///
    /// Для готового рецепта обращение к свойству включает модуль и его
    /// зависимости в граф. У индексного источника тело готовится при
    /// фактическом обращении. Связь сохраняет обычный компонентный путь
    /// чтения и вызова, без открытия BSL-импорта закрытого модуля.
    #[must_use]
    pub fn configuration_component_module(
        mut self,
        global: impl Into<String>,
        property: impl Into<String>,
        module: impl Into<String>,
    ) -> Self {
        self.component_modules
            .push((global.into(), property.into(), module.into()));
        self
    }

    /// Объявляет методы собственного получателя модуля конфигурации.
    ///
    /// Голый вызов объявленного имени идёт через текущий `ЭтотОбъект` и
    /// обычную диспетчеризацию методов. Другие модули этих имён не получают.
    #[must_use]
    pub fn configuration_method_context(
        mut self,
        module: impl Into<String>,
        methods: &'static [bsl_rt::MethodDescriptor],
    ) -> Self {
        self.method_contexts.push((module.into(), methods));
        self
    }

    /// Объявляет конечную таблицу свойств собственного получателя модуля.
    /// Голые чтения и присваивания идут через `ЭтотОбъект`; явно объявленные
    /// переменные и параметры сохраняют приоритет. Остальные имена не меняются.
    #[must_use]
    pub fn configuration_property_context(
        self,
        module: impl Into<String>,
        properties: &'static [bsl_rt::PropertyDescriptor],
    ) -> Self {
        self.configuration_property_names(
            module,
            properties
                .iter()
                .flat_map(|property| property.names)
                .copied()
                .map(str::to_owned),
        )
    }

    /// Объявляет конечный список имён свойств получателя модуля.
    /// Имена принадлежат движку и могут поступать из метаданных конфигурации.
    /// Значения и правила записи остаются ответственностью самого объекта.
    /// Переменные и параметры сохраняют приоритет над свойствами.
    #[must_use]
    pub fn configuration_property_names(
        mut self,
        module: impl Into<String>,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.property_contexts.push((
            module.into(),
            names.into_iter().map(Into::into).collect::<Vec<_>>().into(),
        ));
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
            visibility: ModuleVisibility::Explicit,
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
        if self.indexed_source_conflict
            || self.indexed_source.as_ref().is_some_and(|source| {
                self.recipe.modules.len()
                    != source
                        .index()
                        .modules()
                        .iter()
                        .filter(|module| {
                            module.kind() != bsl_config::ModuleKind::Common
                                || module.execution().server()
                        })
                        .count()
            })
        {
            return Err(Error::Configuration(
                "каталог задан несколькими источниками".into(),
            ));
        }
        if self.module_context_source.is_some() && self.indexed_source.is_none() {
            return Err(Error::Configuration(
                "адресный контекст требует индексный источник конфигурации".into(),
            ));
        }
        // Один индекс обслуживает все контексты, включая сборку движка
        // фонового работника. Повторный обход каталога для каждого владельца
        // давал квадратичную проверку ещё до подготовки исполняемых тел.
        let module_names: std::collections::HashSet<_> = match &self.image {
            Some((catalog, _)) => catalog
                .modules
                .iter()
                .map(|module| module.name.to_uppercase())
                .collect(),
            None => self
                .recipe
                .modules
                .iter()
                .map(|module| module.name.to_uppercase())
                .collect(),
        };
        let mut property_owners = std::collections::HashSet::new();
        for (name, _) in &self.property_contexts {
            let key = name.to_uppercase();
            if !module_names.contains(&key) {
                return Err(Error::Configuration(format!(
                    "контекст свойств задан для неизвестного модуля «{name}»"
                )));
            }
            if !property_owners.insert(key) {
                return Err(Error::Configuration(format!(
                    "контекст свойств модуля «{name}» задан повторно"
                )));
            }
        }
        let mut method_owners = std::collections::HashSet::new();
        for (name, _) in &self.method_contexts {
            let key = name.to_uppercase();
            if !module_names.contains(&key) {
                return Err(Error::Configuration(format!(
                    "контекст методов задан для неизвестного модуля «{name}»"
                )));
            }
            if !method_owners.insert(key) {
                return Err(Error::Configuration(format!(
                    "контекст методов модуля «{name}» задан повторно"
                )));
            }
        }
        let registry = Arc::new(self.runtime.build()?);
        if !self.configuration_entry_sources.is_empty()
            && (self.image.is_some() || self.recipe.modules.is_empty())
        {
            return Err(Error::Configuration(
                "корневые тексты требуют рецепт конфигурации, а не готовый образ".into(),
            ));
        }
        if self.image.is_some()
            && (!self.recipe.modules.is_empty() || !self.configuration_roots.is_empty())
        {
            return Err(Error::Configuration(
                "каталог задан и рецептом, и образом — источник должен быть один".to_string(),
            ));
        }
        let indexed = if let Some(source) = self.indexed_source {
            let catalog = on_demand::OnDemandCatalog::new(
                self.recipe.clone(),
                source,
                registry.clone(),
                self.symbols,
                bsl_compiler::BuildOptions {
                    optimizations: self.optimizations,
                    debug_info: self.debug_info,
                },
                self.method_contexts.clone(),
                self.property_contexts.clone(),
                &self.component_modules,
                self.defer_common_module_name_errors,
                self.module_context_source,
            )?;
            catalog.prepare_roots(&self.configuration_roots, &self.configuration_entry_sources)?;
            Some(std::rc::Rc::new(catalog))
        } else {
            None
        };
        let configuration = if indexed.is_some() {
            None
        } else if let Some((catalog, eager_init)) = self.image {
            bsl_bytecode::image::verify_configuration(&catalog, None).map_err(Error::Runtime)?;
            let entry_imports = catalog
                .modules
                .iter()
                .enumerate()
                .filter(|(_, module)| module.entry_visible)
                .map(|(i, module)| {
                    imported_module_from_program(
                        &module.name,
                        i as u32,
                        &module.program,
                        module.global,
                    )
                })
                .collect();
            let init_order = image_init_order(&catalog);
            Some(std::rc::Rc::new(EngineConfiguration {
                catalog,
                entry_imports,
                init_order,
                eager_init,
            }))
        } else if self.recipe.modules.is_empty() && self.configuration_roots.is_empty() {
            None
        } else {
            let (catalog, entry_imports, init_order) = compile_catalog(
                &self.recipe,
                &self.configuration_roots,
                &self.configuration_entry_sources,
                &self.component_modules,
                &self.method_contexts,
                &self.property_contexts,
                &registry,
                &self.symbols,
                self.defer_common_module_name_errors,
                bsl_compiler::BuildOptions {
                    optimizations: self.optimizations,
                    debug_info: self.debug_info,
                },
                preparation::workers(),
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
                method_contexts: self.method_contexts,
                property_contexts: self.property_contexts,
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
            indexed,
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
    owner: std::sync::Weak<EngineInner>,
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

/// Часы читаются только в явно включённом нативном диагностическом запуске.
/// Обычная сборка, включая браузерный WASM, от них не зависит.
pub(crate) fn startup_phase() -> Option<std::time::Instant> {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("OPEN_BSL_STARTUP_TIMINGS").is_some_and(|value| value == "1") {
        return Some(std::time::Instant::now());
    }
    None
}

/// Выводит длительность завершённого этапа только по запросу диагностического запуска.
pub(crate) fn startup_timing(stage: &str, start: Option<std::time::Instant>) {
    if let Some(start) = start {
        eprintln!(
            "open-bsl.startup\t{stage}\t{:.6}",
            start.elapsed().as_secs_f64()
        );
    }
}

/// Компилирует каталог по рецепту: topo-порядок для резолвинга, порядок
/// манифеста для номеров модулей, общий интернер имён и форм, периметр
/// `verify_configuration` до заморозки.
#[allow(clippy::too_many_arguments)]
fn compile_catalog(
    recipe: &ModuleGraphRecipe,
    roots: &[String],
    entry_sources: &[String],
    component_modules: &[(String, String, String)],
    method_contexts: &[(String, &'static [bsl_rt::MethodDescriptor])],
    property_contexts: &[(String, Arc<[String]>)],
    registry: &bsl_rt::RuntimeRegistry,
    symbols: &bsl_syntax::PreprocSymbols,
    defer_name_errors: bool,
    build: bsl_compiler::BuildOptions,
    workers: usize,
) -> Result<
    (
        bsl_bytecode::ConfigurationProgram,
        Vec<bsl_sema::ImportedModule>,
        Vec<u32>,
    ),
    Error,
> {
    if roots.is_empty() {
        return compile_whole_catalog(
            recipe,
            method_contexts,
            property_contexts,
            registry,
            symbols,
            defer_name_errors,
            build,
        );
    }
    let rooted = rooted_recipe(
        recipe,
        roots,
        entry_sources,
        component_modules,
        method_contexts,
        property_contexts,
        registry,
        symbols,
        defer_name_errors,
        workers,
    )?;
    let modules = &rooted.recipe.modules;
    let index_of = |name: &str| {
        modules
            .iter()
            .position(|module| bsl_rt::folded_eq(&module.name, name))
    };
    let init_order = configuration_init_order(modules)?;
    let common = common_module_imports(modules);
    let compact_imports = (0..modules.len())
        .map(|current| {
            Ok(bsl_bytecode::ModuleImports::sharing(
                module_aliases(modules, current, &index_of)?
                    .into_iter()
                    .filter(|(_, target, _)| *target != current)
                    .map(|(alias, target, global)| bsl_bytecode::ModuleImport {
                        alias: alias.to_string(),
                        module: bsl_bytecode::ModuleId::new(target as u32),
                        global,
                    }),
                &common,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    finish_catalog(
        modules,
        rooted.resolved,
        compact_imports,
        init_order,
        Some(&rooted.original_to_selected),
        build,
    )
}

fn receiver_properties<'a>(contexts: &'a [(String, Arc<[String]>)], module: &str) -> &'a [String] {
    contexts
        .iter()
        .find(|(name, _)| bsl_rt::folded_eq(name, module))
        .map_or(&[], |(_, properties)| properties.as_ref())
}

fn validate_module_names(modules: &[ModuleRecipe]) -> Result<(), Error> {
    // Имена каталога уникальны без учёта регистра — правило свёртки то же,
    // что у остальных имён BSL.
    let mut names = std::collections::HashSet::with_capacity(modules.len());
    for module in modules {
        if module.name.is_empty() {
            return Err(Error::Configuration("имя общего модуля пусто".to_string()));
        }
        if !names.insert(module.name.to_uppercase()) {
            return Err(Error::Configuration(format!(
                "имя общего модуля «{}» повторяется",
                module.name
            )));
        }
    }
    Ok(())
}

fn module_surfaces(
    modules: &[ModuleRecipe],
    syntax: &[bsl_syntax::Program],
) -> Result<Vec<bsl_sema::ImportedModule>, Error> {
    syntax
        .iter()
        .enumerate()
        .map(|(i, syntax)| {
            imported_module_from_syntax(
                &modules[i].name,
                i as u32,
                &syntax.items,
                modules[i].visibility == ModuleVisibility::Global,
            )
            .map_err(Error::from)
        })
        .collect()
}

fn reject_duplicate_global_exports(
    modules: &[ModuleRecipe],
    surfaces: &[bsl_sema::ImportedModule],
) -> Result<(), Error> {
    // Проверяем в прежнем порядке, сохраняя написание первого конфликта.
    // Повтор внутри одного модуля не относится к межмодульным конфликтам.
    let mut owners = std::collections::HashMap::new();
    for (i, module) in surfaces
        .iter()
        .enumerate()
        .filter(|(i, _)| modules[*i].visibility == ModuleVisibility::Global)
    {
        for function in &module.functions {
            if *owners.entry(function.name.to_uppercase()).or_insert(i) != i {
                return Err(Error::Configuration(format!(
                    "глобальный экспорт «{}» объявлен несколькими общими модулями",
                    function.name
                )));
            }
        }
    }
    Ok(())
}

fn common_module_imports(modules: &[ModuleRecipe]) -> bsl_bytecode::ModuleImports {
    modules
        .iter()
        .enumerate()
        .filter(|(_, module)| {
            matches!(
                module.visibility,
                ModuleVisibility::Named | ModuleVisibility::Global
            )
        })
        .map(|(i, module)| bsl_bytecode::ModuleImport {
            alias: module.name.clone(),
            module: bsl_bytecode::ModuleId::new(i as u32),
            global: module.visibility == ModuleVisibility::Global,
        })
        .collect()
}

fn module_aliases<'a>(
    modules: &'a [ModuleRecipe],
    current: usize,
    index_of: &dyn Fn(&str) -> Option<usize>,
) -> Result<Vec<(&'a str, usize, bool)>, preparation::Failure> {
    let module = &modules[current];
    let mut aliases = Vec::new();
    for (alias, target) in &module.imports {
        let target_index = index_of(target).ok_or_else(|| {
            preparation::Failure::Configuration(format!(
                "модуль «{}» импортирует неизвестный «{target}»",
                module.name
            ))
        })?;
        aliases.push((alias.as_str(), target_index, false));
    }
    let explicit = aliases.len();
    for (target_index, target) in modules.iter().enumerate() {
        if !matches!(
            target.visibility,
            ModuleVisibility::Named | ModuleVisibility::Global
        ) {
            continue;
        }
        // Явный псевдоним сохраняет приоритет. Неявные имена уже проверены
        // на уникальность; собственное имя остаётся доступным резолверу.
        if let Some((_, existing, _)) = aliases[..explicit]
            .iter()
            .find(|(alias, _, _)| bsl_rt::folded_eq(alias, &target.name))
        {
            if *existing != target_index {
                return Err(preparation::Failure::Configuration(format!(
                    "псевдоним «{}» модуля «{}» ссылается на два модуля",
                    target.name, module.name
                )));
            }
            continue;
        }
        aliases.push((
            &target.name,
            target_index,
            target.visibility == ModuleVisibility::Global,
        ));
    }
    Ok(aliases)
}

fn module_imports(
    modules: &[ModuleRecipe],
    surfaces: &[bsl_sema::ImportedModule],
    current: usize,
    index_of: &dyn Fn(&str) -> Option<usize>,
    method_contexts: &[(String, &'static [bsl_rt::MethodDescriptor])],
) -> Result<Vec<bsl_sema::ImportedModule>, preparation::Failure> {
    let mut imports = Vec::new();
    for (name, methods) in method_contexts {
        if bsl_rt::folded_eq(name, &modules[current].name) {
            imports.push(bsl_sema::ImportedModule {
                alias: String::new(),
                module: surfaces[current].module,
                is_self: true,
                global: false,
                functions: Vec::new(),
                variables: Vec::new(),
                component_access: None,
                receiver_methods: methods,
            });
        }
    }
    for (alias, target, global) in module_aliases(modules, current, index_of)? {
        let mut imported = surfaces[target].clone();
        imported.alias = alias.to_string();
        imported.is_self = target == current;
        imported.global = global;
        if imported.is_self {
            imported.variables.clear();
        }
        imports.push(imported);
    }
    Ok(imports)
}

fn resolved_configuration_dependencies(
    resolved: &bsl_sema::ResolvedProgram,
) -> impl Iterator<Item = usize> + '_ {
    resolved
        .links
        .iter()
        .filter_map(|link| match link {
            bsl_sema::ResolvedLink::Function { module, .. }
            | bsl_sema::ResolvedLink::Variable { module, .. } => Some(*module as usize),
            bsl_sema::ResolvedLink::ObjectMethod { .. } => None,
        })
        .chain(
            resolved
                .deferred_module_dependencies
                .iter()
                .map(|module| *module as usize),
        )
}

struct RootedCatalog {
    recipe: ModuleGraphRecipe,
    resolved: Vec<bsl_sema::ResolvedProgram>,
    original_to_selected: Vec<Option<u32>>,
}

#[allow(clippy::too_many_arguments)]
fn rooted_recipe(
    recipe: &ModuleGraphRecipe,
    roots: &[String],
    entry_sources: &[String],
    component_modules: &[(String, String, String)],
    method_contexts: &[(String, &'static [bsl_rt::MethodDescriptor])],
    property_contexts: &[(String, Arc<[String]>)],
    registry: &bsl_rt::RuntimeRegistry,
    symbols: &bsl_syntax::PreprocSymbols,
    defer_name_errors: bool,
    workers: usize,
) -> Result<RootedCatalog, Error> {
    let modules = &recipe.modules;
    validate_module_names(modules)?;
    let index_of = |name: &str| -> Option<usize> {
        modules
            .iter()
            .position(|module| bsl_rt::folded_eq(&module.name, name))
    };
    // Для выбора замыкания нужны только малые экспортные поверхности.
    // Не держим AST всего каталога: в большой конфигурации он на порядки
    // больше замыкания модуля сеанса. Полный AST повторно строится ниже
    // только для модулей, до которых дошёл статический обход.
    let phase = startup_phase();
    let surfaces = preparation::ordered_map(modules, workers, |i, module| {
        let syntax = bsl_syntax::parse_with_symbols(&module.source, symbols)
            .map_err(preparation::Failure::Parse)?;
        imported_module_from_syntax(
            &module.name,
            i as u32,
            &syntax.items,
            module.visibility == ModuleVisibility::Global,
        )
    })
    .into_iter()
    .enumerate()
    .map(|(i, result)| result.map_err(|error| error.in_module(&modules[i].name)))
    .collect::<Result<Vec<_>, Error>>()?;
    reject_duplicate_global_exports(modules, &surfaces)?;

    startup_timing("engine.rooted.exports", phase);
    let phase = startup_phase();
    let component_imports = component_modules
        .iter()
        .map(|(global, property, target)| {
            let target_index = index_of(target).ok_or_else(|| Error::Configuration(format!(
            "компонентное свойство «{global}.{property}» ссылается на неизвестный модуль «{target}»"
        )))?;
            let (library_index, function) = registry.lookup_global(global).ok_or_else(|| {
                Error::Configuration(format!("компонентный глобал «{global}» не зарегистрирован"))
            })?;
            let library = registry
                .library(library_index)
                .expect("индекс получен из реестра");
            let imported = bsl_sema::ImportedModule {
                alias: String::new(),
                module: surfaces[target_index].module,
                global: false,
                is_self: false,
                functions: Vec::new(),
                variables: Vec::new(),
                component_access: Some(bsl_sema::ComponentModuleAccess {
                    library: bsl_rt::LibraryKey::new(library.package()),
                    function,
                    property: property.clone(),
                }),
                receiver_methods: &[],
            };
            Ok(imported)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let mut selected = vec![false; modules.len()];
    let mut analyzed: Vec<Option<bsl_sema::ResolvedProgram>> =
        (0..modules.len()).map(|_| None).collect();
    let mut pending = Vec::new();
    for root in roots {
        let Some(index) = index_of(root) else {
            return Err(Error::Configuration(format!(
                "корневой модуль «{root}» отсутствует в каталоге"
            )));
        };
        if !selected[index] {
            selected[index] = true;
            pending.push(index);
        }
    }

    if !entry_sources.is_empty() {
        let mut imports = modules
            .iter()
            .enumerate()
            .filter(|(_, module)| {
                matches!(
                    module.visibility,
                    ModuleVisibility::Named | ModuleVisibility::Global
                )
            })
            .map(|(index, _)| surfaces[index].clone())
            .collect::<Vec<_>>();
        imports.extend(component_imports.iter().cloned());
        for source in entry_sources {
            let syntax = bsl_syntax::parse_with_symbols(source, symbols)?;
            let resolved = bsl_sema::resolve_program_with(
                &syntax.items,
                &bsl_sema::ResolveOptions::new(registry).imports(&imports),
            )?;
            for target in resolved_configuration_dependencies(&resolved) {
                if !selected[target] {
                    selected[target] = true;
                    pending.push(target);
                }
            }
        }
    }

    let analyzer = preparation::Analyzer {
        modules,
        surfaces: &surfaces,
        component_imports: &component_imports,
        method_contexts,
        property_contexts,
        registry,
        symbols,
        defer_name_errors,
    };
    let mut cursor = 0;
    while cursor < pending.len() {
        // Окно содержит только уже выбранные модули. Координатор принимает
        // результаты в прежнем порядке, независимо от завершения потоков.
        let end = (cursor + 4).min(pending.len());
        let batch = &pending[cursor..end];
        let results = analyzer.batch(batch, workers, None);
        let batch = batch.to_vec();
        cursor = end;
        for (current, result) in batch.into_iter().zip(results) {
            for (_, target) in &modules[current].imports {
                let Some(target_index) = index_of(target) else {
                    return Err(Error::Configuration(format!(
                        "модуль «{}» импортирует неизвестный «{target}»",
                        modules[current].name
                    )));
                };
                if !selected[target_index] {
                    selected[target_index] = true;
                    pending.push(target_index);
                }
            }
            let resolved = result.map_err(|error| error.in_module(&modules[current].name))?;
            for target in resolved_configuration_dependencies(&resolved) {
                if !selected[target] {
                    selected[target] = true;
                    pending.push(target);
                }
            }
            analyzed[current] = Some(resolved);
        }
    }

    // Ошибочное тело могло оборваться до регистрации зависимости. После
    // отсечения видимых модулей сохраняем прежнюю перепроверку диагностики.
    if modules.iter().enumerate().any(|(index, module)| {
        !selected[index]
            && matches!(
                module.visibility,
                ModuleVisibility::Named | ModuleVisibility::Global
            )
    }) {
        let deferred = |statement: &bsl_sema::RStmt| {
            statement.line == 0
                && matches!(
                    statement.kind,
                    bsl_sema::RStmtKind::Raise(_) | bsl_sema::RStmtKind::RaiseCompilationError(_)
                )
        };
        let recheck = analyzed
            .iter()
            .enumerate()
            .filter_map(|(index, body)| {
                body.as_ref()
                    .filter(|body| {
                        body.top_level.body.iter().any(deferred)
                            || body
                                .functions
                                .iter()
                                .any(|function| function.body.iter().any(deferred))
                    })
                    .map(|_| index)
            })
            .collect::<Vec<_>>();
        for batch in recheck.chunks(4) {
            let results = analyzer.batch(batch, workers, Some(&selected));
            for (&current, result) in batch.iter().zip(results) {
                let resolved = result.map_err(|error| {
                    // Проверка импортов прежде предшествовала второму анализу.
                    let reachable = modules
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| selected[*index])
                        .map(|(_, module)| module.clone())
                        .collect::<Vec<_>>();
                    configuration_init_order(&reachable)
                        .err()
                        .unwrap_or_else(|| error.in_module(&modules[current].name))
                })?;
                analyzed[current] = Some(resolved);
            }
        }
    }

    startup_timing("engine.rooted.selection", phase);
    let mut original_to_selected = vec![None; modules.len()];
    let mut resolved = Vec::new();
    for (original, body) in analyzed.into_iter().enumerate() {
        if let Some(body) = body {
            original_to_selected[original] = Some(resolved.len() as u32);
            resolved.push(body);
        }
    }
    let recipe = ModuleGraphRecipe {
        modules: modules
            .iter()
            .enumerate()
            .filter(|(index, _)| selected[*index])
            .map(|(_, module)| module.clone())
            .collect(),
        eager_init: recipe.eager_init,
    };
    Ok(RootedCatalog {
        recipe,
        resolved,
        original_to_selected,
    })
}

fn configuration_init_order(modules: &[ModuleRecipe]) -> Result<Vec<u32>, Error> {
    let index_of = |name: &str| {
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

    Ok(topo.into_iter().map(|i| i as u32).collect())
}

fn compile_whole_catalog(
    recipe: &ModuleGraphRecipe,
    method_contexts: &[(String, &'static [bsl_rt::MethodDescriptor])],
    property_contexts: &[(String, Arc<[String]>)],
    registry: &bsl_rt::RuntimeRegistry,
    symbols: &bsl_syntax::PreprocSymbols,
    defer_name_errors: bool,
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
    validate_module_names(modules)?;
    let index_of = |name: &str| -> Option<usize> {
        modules
            .iter()
            .position(|module| bsl_rt::folded_eq(&module.name, name))
    };

    let init_order = configuration_init_order(modules)?;

    let phase = startup_phase();
    let syntax = modules
        .iter()
        .map(|module| {
            bsl_syntax::parse_with_symbols(&module.source, symbols).map_err(|error| {
                Error::InModule {
                    module: module.name.clone(),
                    source: Box::new(Error::Parse(error)),
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let surfaces = module_surfaces(modules, &syntax)?;
    reject_duplicate_global_exports(modules, &surfaces)?;

    startup_timing("engine.catalog.parse", phase);
    let phase = startup_phase();
    // Общее окружение хранится один раз; отдельные программы исключают себя
    // и сохраняют явные псевдонимы без копирования остальных импортов.
    let common_imports = common_module_imports(modules);
    let mut compact_imports = Vec::with_capacity(modules.len());
    let mut resolved = Vec::with_capacity(modules.len());
    // Разобранный модуль больше не нужен после разрешения имён.
    // Освобождаем его сразу, не удерживая весь AST рядом с результатами.
    for (i, parsed) in syntax.into_iter().enumerate() {
        let imports = module_imports(modules, &surfaces, i, &index_of, method_contexts)?;
        resolved.push(
            bsl_sema::resolve_program_with(
                &parsed.items,
                &bsl_sema::ResolveOptions::new(registry)
                    .imports(&imports)
                    .configuration_module()
                    .receiver_properties(receiver_properties(property_contexts, &modules[i].name))
                    .defer_name_errors(
                        defer_name_errors
                            && matches!(
                                modules[i].visibility,
                                ModuleVisibility::Named | ModuleVisibility::Global
                            ),
                    ),
            )
            .map_err(|error| Error::InModule {
                module: modules[i].name.clone(),
                source: Box::new(Error::Semantic(error)),
            })?,
        );
        // Не удерживаем N полных окружений в промежуточном представлении.
        let imports = std::mem::take(&mut resolved.last_mut().unwrap().imports);
        compact_imports.push(bsl_bytecode::ModuleImports::sharing(
            imports
                .into_iter()
                .map(|(alias, module, global)| bsl_bytecode::ModuleImport {
                    alias,
                    module: bsl_bytecode::ModuleId::new(module),
                    global,
                }),
            &common_imports,
        ));
    }

    startup_timing("engine.catalog.resolve", phase);
    finish_catalog(modules, resolved, compact_imports, init_order, None, build)
}

fn finish_catalog(
    modules: &[ModuleRecipe],
    resolved: Vec<bsl_sema::ResolvedProgram>,
    compact_imports: Vec<bsl_bytecode::ModuleImports>,
    init_order: Vec<u32>,
    mapping: Option<&[Option<u32>]>,
    build: bsl_compiler::BuildOptions,
) -> Result<
    (
        bsl_bytecode::ConfigurationProgram,
        Vec<bsl_sema::ImportedModule>,
        Vec<u32>,
    ),
    Error,
> {
    let phase = startup_phase();
    let pairs: Vec<(String, &bsl_sema::ResolvedProgram)> = modules
        .iter()
        .zip(&resolved)
        .map(|(module, resolved)| (module.name.clone(), resolved))
        .collect();
    let (mut catalog, _) = bsl_compiler::compile_configuration(&pairs, None, build)?;
    for ((compiled, recipe), imports) in
        catalog.modules.iter_mut().zip(modules).zip(compact_imports)
    {
        compiled.program.imports = imports;
        compiled.entry_visible = recipe.visibility != ModuleVisibility::Private;
        compiled.global = recipe.visibility == ModuleVisibility::Global;
    }
    if let Some(mapping) = mapping {
        let remap = |old: usize| -> Result<u32, Error> {
            mapping.get(old).copied().flatten().ok_or_else(|| {
                Error::Configuration("ссылка на модуль вне выбранного каталога".into())
            })
        };
        for module in &mut catalog.modules {
            for link in &mut module.program.links {
                if let bsl_bytecode::LinkEntry::Function { module, .. }
                | bsl_bytecode::LinkEntry::Variable { module, .. } = link
                {
                    *module = bsl_bytecode::ModuleId::new(remap(module.index())?);
                }
            }
            for chunk in &mut module.program.chunks {
                for instruction in &mut chunk.instrs {
                    if let bsl_bytecode::Instr::GetImportedModule { module, .. } = instruction {
                        *module = remap(*module as usize)?;
                    }
                }
            }
            if build.debug_info {
                bsl_bytecode::image::finalize_unbundled(&mut module.program);
            } else {
                bsl_bytecode::image::finalize(&mut module.program);
            }
        }
    }
    let entry_imports = modules
        .iter()
        .zip(&resolved)
        .enumerate()
        .map(|(i, (module, resolved))| {
            let mut imported =
                bsl_sema::ImportedModule::from_resolved(&module.name, i as u32, resolved);
            imported.global = module.visibility == ModuleVisibility::Global;
            imported
        })
        .filter(|module| modules[module.module as usize].visibility != ModuleVisibility::Private)
        .collect();
    // После извлечения экспортов промежуточное представление больше не нужно.
    // Проверяем байткод без одновременного хранения разрешённых тел модулей.
    drop(pairs);
    drop(resolved);
    startup_timing("engine.catalog.codegen", phase);
    let phase = startup_phase();
    bsl_bytecode::image::verify_configuration(&catalog, None).map_err(Error::Runtime)?;
    startup_timing("engine.catalog.verify", phase);
    Ok((catalog, entry_imports, init_order))
}

/// Экспортная поверхность модуля, снятая с его ПРОГРАММЫ, — для движка,
/// собранного из образа: `ResolvedProgram` у него нет, а всё нужное
/// (имена, экспортность, сигнатуры чанков) байт-код уже несёт.
pub(crate) fn imported_module_from_program(
    alias: &str,
    module: u32,
    program: &bsl_bytecode::Program,
    global: bool,
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
        is_self: false,
        global,
        functions,
        variables,
        component_access: None,
        receiver_methods: &[],
    }
}

fn imported_module_from_syntax(
    alias: &str,
    module: u32,
    items: &[bsl_syntax::Item],
    global: bool,
) -> Result<bsl_sema::ImportedModule, preparation::Failure> {
    let mut functions = Vec::new();
    let mut method_index = 0usize;
    let mut variables: Vec<bsl_sema::ImportedVariable> = Vec::new();
    let mut module_variables: Vec<(String, bool)> = Vec::new();
    let mut variable_positions = std::collections::HashMap::new();
    for item in items {
        match item {
            bsl_syntax::Item::Function(function) => {
                method_index += 1;
                let chunk = u16::try_from(method_index).map_err(|_| {
                    preparation::Failure::Configuration(format!(
                        "в модуле «{alias}» слишком много методов"
                    ))
                })?;
                if function.export {
                    functions.push(bsl_sema::ImportedFunction {
                        name: function.name.clone(),
                        chunk,
                        is_procedure: false,
                        is_async: function.is_async,
                        param_by_val: function
                            .params
                            .iter()
                            .map(|parameter| function.is_async || parameter.by_val)
                            .collect(),
                        param_has_default: function
                            .params
                            .iter()
                            .map(|parameter| parameter.default.is_some())
                            .collect(),
                    });
                }
            }
            bsl_syntax::Item::Procedure(procedure) => {
                method_index += 1;
                let chunk = u16::try_from(method_index).map_err(|_| {
                    preparation::Failure::Configuration(format!(
                        "в модуле «{alias}» слишком много методов"
                    ))
                })?;
                if procedure.export {
                    functions.push(bsl_sema::ImportedFunction {
                        name: procedure.name.clone(),
                        chunk,
                        is_procedure: true,
                        is_async: procedure.is_async,
                        param_by_val: procedure
                            .params
                            .iter()
                            .map(|parameter| procedure.is_async || parameter.by_val)
                            .collect(),
                        param_has_default: procedure
                            .params
                            .iter()
                            .map(|parameter| parameter.default.is_some())
                            .collect(),
                    });
                }
            }
            bsl_syntax::Item::VarDecl(declaration) => {
                for name in &declaration.names {
                    let slot = *variable_positions
                        .entry(name.to_uppercase())
                        .or_insert_with(|| {
                            let slot = module_variables.len();
                            module_variables.push((name.clone(), declaration.export));
                            slot
                        });
                    module_variables[slot].1 |= declaration.export;
                }
            }
            bsl_syntax::Item::Stmt(_) => {}
        }
    }
    for (slot, (name, export)) in module_variables.into_iter().enumerate() {
        if export {
            variables.push(bsl_sema::ImportedVariable {
                name,
                slot: u16::try_from(slot).map_err(|_| {
                    preparation::Failure::Configuration(format!(
                        "в модуле «{alias}» слишком много модульных переменных"
                    ))
                })?,
            });
        }
    }
    Ok(bsl_sema::ImportedModule {
        alias: alias.to_string(),
        module,
        is_self: false,
        global,
        functions,
        variables,
        component_access: None,
        receiver_methods: &[],
    })
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
        for import in &catalog.modules[current].program.imports {
            visit(catalog, state, order, import.module.index());
        }
        for link in &catalog.modules[current].program.links {
            let target = match link {
                bsl_bytecode::LinkEntry::Function { module, .. }
                | bsl_bytecode::LinkEntry::Variable { module, .. } => module.index(),
                bsl_bytecode::LinkEntry::ObjectMethod { .. } => continue,
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

/// Входит ли модуль индекса в серверный каталог движка: общий модуль — только
/// с флагом «Сервер», остальные виды модулей — всегда. Тот же отбор задаёт
/// номера модулей, по которым фоновые задания находят свою цель.
pub(crate) fn executes_on_server(module: &bsl_config::ConfigurationModuleDeclaration) -> bool {
    module.kind() != bsl_config::ModuleKind::Common || module.execution().server()
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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "engine_background_runtime_tests.rs"]
mod background_runtime_tests;

#[cfg(test)]
#[path = "engine_import_storage_tests.rs"]
mod import_storage_tests;

#[cfg(test)]
#[path = "engine_rooted_analysis_tests.rs"]
mod rooted_analysis_tests;

#[cfg(test)]
#[path = "engine_module_names_tests.rs"]
mod module_names_tests;

#[cfg(test)]
#[path = "engine_global_declarations_tests.rs"]
mod global_declarations_tests;
