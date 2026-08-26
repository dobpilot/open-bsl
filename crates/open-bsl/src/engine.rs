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
                )?
            }
            None => bsl_compiler::compile_program(&resolved)?,
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
        )?;
        Ok(Module {
            id: self.next_module_id(),
            program,
        })
    }

    /// Каталог общих модулей движка, если он был собран.
    pub(crate) fn catalog(&self) -> Option<&bsl_bytecode::ConfigurationProgram> {
        self.configuration.as_ref().map(|c| &c.catalog)
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
}

/// Стадия композиции статически связанных runtime-компонентов.
pub struct EngineBuilder {
    runtime: bsl_rt::RuntimeBuilder,
    symbols: bsl_syntax::PreprocSymbols,
    recipe: ModuleGraphRecipe,
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
            recipe: ModuleGraphRecipe::default(),
        }
    }

    pub fn register_library(mut self, library: LibraryDescriptor) -> Self {
        self.runtime.register(library);
        self
    }

    /// Передаёт весь граф общих модулей разом. Модули уже добавленные
    /// `common_module` сохраняются; каталог замораживается в `build`.
    #[must_use]
    pub fn configuration(mut self, recipe: ModuleGraphRecipe) -> Self {
        self.recipe.modules.extend(recipe.modules);
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
        let configuration = if self.recipe.modules.is_empty() {
            None
        } else {
            let (catalog, entry_imports) = compile_catalog(&self.recipe, &registry, &self.symbols)?;
            Some(std::rc::Rc::new(EngineConfiguration {
                catalog,
                entry_imports,
            }))
        };
        Ok(Engine {
            inner: Arc::new(EngineInner {
                registry,
                symbols: self.symbols,
                modules: AtomicU64::new(0),
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
) -> Result<
    (
        bsl_bytecode::ConfigurationProgram,
        Vec<bsl_sema::ImportedModule>,
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

    // Topo-сортировка Кана по импортам: цикл — ошибка конфигурации, как и
    // импорт неизвестного модуля.
    let mut in_degree = vec![0usize; modules.len()];
    let mut dependants: Vec<Vec<usize>> = vec![Vec::new(); modules.len()];
    for (i, module) in modules.iter().enumerate() {
        for (_, target) in &module.imports {
            let Some(target) = index_of(target) else {
                return Err(Error::Configuration(format!(
                    "модуль «{}» импортирует неизвестный «{target}»",
                    module.name
                )));
            };
            in_degree[i] += 1;
            dependants[target].push(i);
        }
    }
    let mut queue: Vec<usize> = (0..modules.len()).filter(|i| in_degree[*i] == 0).collect();
    let mut topo = Vec::with_capacity(modules.len());
    while let Some(next) = queue.pop() {
        topo.push(next);
        for &dependant in &dependants[next] {
            in_degree[dependant] -= 1;
            if in_degree[dependant] == 0 {
                queue.push(dependant);
            }
        }
    }
    if topo.len() != modules.len() {
        return Err(Error::Configuration(
            "граф импортов общих модулей содержит цикл".to_string(),
        ));
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
    let (catalog, _) = bsl_compiler::compile_configuration(&pairs, None)?;
    bsl_bytecode::image::verify_configuration(&catalog, None).map_err(Error::Runtime)?;

    let entry_imports = modules
        .iter()
        .zip(&resolved)
        .enumerate()
        .map(|(i, (module, resolved))| {
            bsl_sema::ImportedModule::from_resolved(&module.name, i as u32, resolved)
        })
        .collect();
    Ok((catalog, entry_imports))
}
