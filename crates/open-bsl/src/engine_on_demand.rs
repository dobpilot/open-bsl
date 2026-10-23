//! Адресная подготовка модулей одного источника конфигурации.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use bsl_bytecode::{ExecutionCatalog, ModuleId, ModuleProgram, Program};
use bsl_config::ConfigurationSource;
use bsl_sema::{ImportedModule, ImportedModuleProvider};

use super::*;

#[path = "engine_declaration_views.rs"]
mod declaration_views;

#[derive(Default)]
struct Namespace {
    names: Arc<[String]>,
    shapes: Vec<Rc<bsl_rt::Shape>>,
}

pub(super) struct OnDemandCatalog {
    pub(super) recipe: ModuleGraphRecipe,
    source: Arc<dyn ConfigurationSource>,
    module_positions: HashMap<String, usize>,
    source_positions: Vec<Option<usize>>,
    registry: Arc<bsl_rt::RuntimeRegistry>,
    symbols: bsl_syntax::PreprocSymbols,
    build: bsl_compiler::BuildOptions,
    method_contexts: Vec<(String, &'static [bsl_rt::MethodDescriptor])>,
    property_contexts: Vec<(String, Arc<[String]>)>,
    context_source: Option<Arc<dyn ModuleContextSource>>,
    properties: Vec<OnceCell<Result<Arc<[String]>, String>>>,
    common_imports: bsl_bytecode::ModuleImports,
    component_imports: Vec<ImportedModule>,
    component_modules: Vec<(String, String, String)>,
    defer_name_errors: bool,
    sources: Vec<RefCell<Option<Arc<str>>>>,
    surfaces: Vec<OnceCell<Result<ImportedModule, String>>>,
    import_surfaces: Vec<ImportedModule>,
    declaration_views: Vec<OnceCell<Result<ModuleProgram, String>>>,
    pub(super) programs: Vec<OnceCell<ModuleProgram>>,
    namespace: RefCell<Namespace>,
    preparing: Cell<bool>,
    verifier: RefCell<bsl_bytecode::image::VerificationCache>,
}

impl OnDemandCatalog {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        recipe: ModuleGraphRecipe,
        source: Arc<dyn ConfigurationSource>,
        registry: Arc<bsl_rt::RuntimeRegistry>,
        symbols: bsl_syntax::PreprocSymbols,
        build: bsl_compiler::BuildOptions,
        method_contexts: Vec<(String, &'static [bsl_rt::MethodDescriptor])>,
        property_contexts: Vec<(String, Arc<[String]>)>,
        component_modules: &[(String, String, String)],
        defer_name_errors: bool,
        context_source: Option<Arc<dyn ModuleContextSource>>,
    ) -> Result<Self, Error> {
        validate_module_names(&recipe.modules)?;
        // Явные импорты сохраняют прежнюю проверку отсутствующих целей и
        // циклов. Для неё достаточно объявлений; исходники не нужны.
        configuration_init_order(&recipe.modules)?;
        let count = recipe.modules.len();
        if count > u32::MAX as usize {
            return Err(Error::Configuration("слишком много модулей".into()));
        }
        let module_positions = recipe
            .modules
            .iter()
            .enumerate()
            .map(|(index, module)| (module.name.to_uppercase(), index))
            .collect::<HashMap<_, _>>();
        let mut declarations = HashMap::new();
        for (index, module) in source.index().modules().iter().enumerate() {
            declarations
                .entry(module.name().to_uppercase())
                .or_insert(index);
        }
        let source_positions = recipe
            .modules
            .iter()
            .map(|module| declarations.get(&module.name.to_uppercase()).copied())
            .collect();
        let mut component_imports = Vec::new();
        for (global, property, target) in component_modules {
            let module = module_positions
                .get(&target.to_uppercase())
                .copied()
                .ok_or_else(|| Error::Configuration(format!("неизвестный модуль «{target}»")))?;
            let (library, function) = registry.lookup_global(global).ok_or_else(|| {
                Error::Configuration(format!("компонентный глобал «{global}» не зарегистрирован"))
            })?;
            component_imports.push(ImportedModule {
                alias: String::new(),
                module: module as u32,
                is_self: false,
                global: false,
                functions: Vec::new(),
                variables: Vec::new(),
                receiver_methods: &[],
                component_access: Some(bsl_sema::ComponentModuleAccess {
                    library: bsl_rt::LibraryKey::new(
                        registry.library(library).expect("индекс реестра").package(),
                    ),
                    function,
                    property: property.clone(),
                }),
            });
        }
        let common_imports = common_module_imports(&recipe.modules);
        let mut catalog = Self {
            recipe,
            source,
            module_positions,
            source_positions,
            registry,
            symbols,
            build,
            method_contexts,
            property_contexts,
            context_source,
            properties: (0..count).map(|_| OnceCell::new()).collect(),
            common_imports,
            component_imports,
            component_modules: component_modules.to_vec(),
            defer_name_errors,
            sources: (0..count).map(|_| RefCell::new(None)).collect(),
            surfaces: (0..count).map(|_| OnceCell::new()).collect(),
            import_surfaces: Vec::new(),
            declaration_views: (0..count).map(|_| OnceCell::new()).collect(),
            programs: (0..count).map(|_| OnceCell::new()).collect(),
            namespace: RefCell::new(Namespace::default()),
            preparing: Cell::new(false),
            verifier: RefCell::default(),
        };
        // Голое глобальное имя должно иметь единственный источник ещё до
        // подготовки тел. Именованные модули здесь не читаются.
        let surfaces = (0..count)
            .map(|index| {
                if catalog.recipe.modules[index].visibility == ModuleVisibility::Global {
                    catalog.surface(index).cloned()
                } else {
                    Ok(catalog.empty_surface(index))
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Configuration)?;
        reject_duplicate_global_exports(&catalog.recipe.modules, &surfaces)?;
        // Этот набор объявлений неизменен. Не создаём его заново для каждого
        // импортёра; собственные псевдонимы по-прежнему настраиваются на копии.
        catalog.import_surfaces = surfaces;
        // После извлечения экспортов исходники недостигнутых глобальных
        // модулей не удерживаются фабрикой. Тело читается при подготовке.
        for (index, module) in catalog.recipe.modules.iter().enumerate() {
            if module.visibility == ModuleVisibility::Global {
                catalog.sources[index].borrow_mut().take();
            }
        }
        Ok(catalog)
    }

    // Индекс источника может включать объявления вне серверного рецепта.
    // Соответствие по имени сохраняет прежний выбор, не загружая текстов.
    fn declaration(
        &self,
        index: usize,
    ) -> Result<&bsl_config::ConfigurationModuleDeclaration, String> {
        self.source_positions[index]
            .and_then(|position| self.source.index().modules().get(position))
            .ok_or_else(|| "объявление модуля отсутствует в источнике".to_string())
    }

    fn source(&self, index: usize) -> Result<Arc<str>, String> {
        if let Some(source) = self.sources[index].borrow().as_ref() {
            return Ok(source.clone());
        }
        let declaration = self.declaration(index)?;
        let source = self.source.module_source(declaration.id())?;
        *self.sources[index].borrow_mut() = Some(source.clone());
        Ok(source)
    }

    fn properties(&self, index: usize) -> Result<&[String], Error> {
        self.properties[index]
            .get_or_init(|| {
                let name = &self.recipe.modules[index].name;
                let mut properties = receiver_properties(&self.property_contexts, name).to_vec();
                if let Some(source) = &self.context_source {
                    properties.extend(source.receiver_properties(name)?);
                }
                Ok(properties.into())
            })
            .as_ref()
            .map(|properties| properties.as_ref())
            .map_err(|message| Error::Configuration(message.clone()))
    }

    pub(super) fn prepared_properties(&self, module: ModuleId) -> Option<&[String]> {
        self.properties
            .get(module.index())?
            .get()?
            .as_ref()
            .ok()
            .map(|properties| properties.as_ref())
    }

    fn empty_surface(&self, index: usize) -> ImportedModule {
        ImportedModule {
            alias: self.recipe.modules[index].name.clone(),
            module: index as u32,
            is_self: false,
            global: self.recipe.modules[index].visibility == ModuleVisibility::Global,
            functions: Vec::new(),
            variables: Vec::new(),
            component_access: None,
            receiver_methods: &[],
        }
    }

    fn surface(&self, index: usize) -> Result<&ImportedModule, String> {
        self.surfaces[index]
            .get_or_init(|| {
                let source = if let Some(source) = self.sources[index].borrow().clone() {
                    source
                } else {
                    let declaration = self.declaration(index)?;
                    match self.source.module_declarations(declaration.id())? {
                        bsl_config::ModuleDeclarationSource::Complete(source) => {
                            *self.sources[index].borrow_mut() = Some(source.clone());
                            source
                        }
                        bsl_config::ModuleDeclarationSource::Declarations(source) => source,
                    }
                };
                let declarations =
                    bsl_syntax::scan_declarations_with_symbols(&source, &self.symbols)
                        .map_err(|error| error.to_string())?;
                let mut surface = self.empty_surface(index);
                for (position, function) in declarations.functions.iter().enumerate() {
                    if function.export {
                        surface.functions.push(bsl_sema::ImportedFunction {
                            name: function.name.clone(),
                            chunk: (position + 1)
                                .try_into()
                                .map_err(|_| "слишком много методов модуля".to_string())?,
                            is_procedure: function.is_procedure,
                            is_async: function.is_async,
                            param_by_val: function
                                .params
                                .iter()
                                .map(|param| function.is_async || param.by_val)
                                .collect(),
                            param_has_default: function
                                .params
                                .iter()
                                .map(|param| param.default.is_some())
                                .collect(),
                        });
                    }
                }
                let mut variables: Vec<(String, bool)> = Vec::new();
                // Индекс ускоряет объединение, а вектор сохраняет первое
                // написание и слоты, включая неэкспортные переменные.
                let mut variable_positions = HashMap::new();
                for declaration in &declarations.variables {
                    for name in &declaration.names {
                        let slot = *variable_positions
                            .entry(name.to_uppercase())
                            .or_insert_with(|| {
                                let slot = variables.len();
                                variables.push((name.clone(), declaration.export));
                                slot
                            });
                        variables[slot].1 |= declaration.export;
                    }
                }
                for (slot, (name, export)) in variables.into_iter().enumerate() {
                    if export {
                        surface.variables.push(bsl_sema::ImportedVariable {
                            name,
                            slot: slot
                                .try_into()
                                .map_err(|_| "слишком много переменных модуля".to_string())?,
                        });
                    }
                }
                Ok(surface)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    pub(super) fn imports(&self, current: Option<usize>) -> Result<Vec<ImportedModule>, Error> {
        let surfaces = &self.import_surfaces;
        let mut imports = if let Some(current) = current {
            module_imports(
                &self.recipe.modules,
                surfaces,
                current,
                &|name| self.module_id(name).map(ModuleId::index),
                &self.method_contexts,
            )
            .map_err(Error::from)?
        } else {
            surfaces
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    matches!(
                        self.recipe.modules[*index].visibility,
                        ModuleVisibility::Named | ModuleVisibility::Global
                    )
                })
                .map(|(_, surface)| surface.clone())
                .collect()
        };
        if current.is_some() {
            imports.extend(self.component_imports.iter().cloned());
        }
        Ok(imports)
    }

    pub(super) fn dynamic_imports(
        &self,
        imports: &[bsl_bytecode::ModuleImport],
    ) -> Result<Vec<ImportedModule>, String> {
        imports
            .iter()
            .map(|import| {
                let index = import.module.index();
                if index >= self.module_count() {
                    return Err("импорт отсутствует в индексе".into());
                }
                let mut surface = if import.global {
                    self.surface(index)?.clone()
                } else {
                    self.empty_surface(index)
                };
                surface.alias = import.alias.clone();
                surface.global = import.global;
                Ok(surface)
            })
            .collect()
    }

    pub(super) fn prepare_roots(&self, roots: &[String], entries: &[String]) -> Result<(), Error> {
        for root in roots {
            let module = self.module_id(root).ok_or_else(|| {
                Error::Configuration(format!("корневой модуль «{root}» отсутствует в каталоге"))
            })?;
            if self.prepared_module(module).is_none() {
                self.prepare(module)?;
            }
        }
        for source in entries {
            self.compile_entry(source)?;
        }
        Ok(())
    }

    pub(super) fn compile_entry(&self, source: &str) -> Result<Program, Error> {
        let syntax = bsl_syntax::parse_with_symbols(source, &self.symbols)?;
        let imports = self.imports(None)?;
        let resolved = bsl_sema::resolve_program_with(
            &syntax.items,
            &bsl_sema::ResolveOptions::new(&self.registry)
                .imports(&imports)
                .provider(Some(self)),
        )?;
        for target in resolved_configuration_dependencies(&resolved) {
            let module = ModuleId::new(target as u32);
            if self.prepared_module(module).is_none() {
                self.prepare(module)?;
            }
        }
        self.compile_resolved(&resolved)
    }

    pub(super) fn compile_resolved(
        &self,
        resolved: &bsl_sema::ResolvedProgram,
    ) -> Result<Program, Error> {
        let namespace = self.namespace.borrow();
        Ok(bsl_compiler::compile_entry_program(
            resolved,
            &namespace.names,
            &namespace.shapes,
            self.build,
        )?)
    }

    fn prepare(&self, first: ModuleId) -> Result<(), Error> {
        if self.preparing.replace(true) {
            return Err(Error::Configuration(
                "повторная подготовка каталога во время загрузки исходника".into(),
            ));
        }
        // Флаг сбрасывается и при панике подготовки: иначе каталог навсегда
        // остался бы в состоянии «идёт подготовка».
        struct Preparing<'a>(&'a Cell<bool>);
        impl Drop for Preparing<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _preparing = Preparing(&self.preparing);
        self.prepare_batch(first)
    }

    fn prepare_batch(&self, first: ModuleId) -> Result<(), Error> {
        let trace = startup_phase().is_some();
        let phase = || trace.then(std::time::Instant::now);
        let elapsed = |start: Option<std::time::Instant>| {
            start.map_or(std::time::Duration::ZERO, |start| start.elapsed())
        };
        let mut times = [std::time::Duration::ZERO; 8];
        let mut pending = vec![first.index()];
        let mut seen = vec![false; self.module_count()];
        seen[first.index()] = true;
        let mut bodies = Vec::new();
        let mut indices = Vec::new();
        let mut cursor = 0;
        let mut context_end = 0;
        while cursor < pending.len() {
            if cursor == context_end {
                context_end = pending.len();
                if let Some(source) = &self.context_source {
                    // Очередь уже доказана семантическими или явными связями.
                    // Группа позволяет приобрести один снимок метаданных для
                    // всех этих получателей, не затрагивая будущие зависимости.
                    let modules = pending[cursor..context_end]
                        .iter()
                        .filter(|index| self.programs[**index].get().is_none())
                        .map(|index| self.recipe.modules[*index].name.as_str())
                        .collect::<Vec<_>>();
                    if !modules.is_empty() {
                        let start = phase();
                        source
                            .prepare_contexts(&modules)
                            .map_err(Error::Configuration)?;
                        times[0] += elapsed(start);
                    }
                }
            }
            let index = pending[cursor];
            cursor += 1;
            if self.programs[index].get().is_some() {
                continue;
            }
            let recipe = &self.recipe.modules[index];
            let start = phase();
            let source = self.source(index).map_err(Error::Configuration)?;
            times[1] += elapsed(start);
            let start = phase();
            let syntax = bsl_syntax::parse_with_symbols(&source, &self.symbols)
                .map_err(|error| preparation::Failure::Parse(error).in_module(&recipe.name))?;
            times[2] += elapsed(start);
            let start = phase();
            let imports = self.imports(Some(index))?;
            times[3] += elapsed(start);
            let start = phase();
            let properties = self.properties(index)?;
            times[4] += elapsed(start);
            let start = phase();
            let mut resolved = bsl_sema::resolve_program_with(
                &syntax.items,
                &bsl_sema::ResolveOptions::new(&self.registry)
                    .imports(&imports)
                    .configuration_module()
                    .receiver_properties(properties)
                    .defer_name_errors(
                        self.defer_name_errors
                            && matches!(
                                recipe.visibility,
                                ModuleVisibility::Named | ModuleVisibility::Global
                            ),
                    )
                    .provider(Some(self)),
            )
            .map_err(|error| preparation::Failure::Semantic(error).in_module(&recipe.name))?;
            times[5] += elapsed(start);
            // Статическая ссылка требует объявлений, а не тела цели.
            // Нативный контроль `native-transitive-body` разрешает исправный
            // экспорт родителя до компиляции ошибочного невызываемого потомка.
            // Замер `unused.platform.txt` в
            // `docs/research/fixtures/session-factory-hotspot-1021/native-transitive-body/`.
            // Явные импорты сохраняют прежнюю политику подготовки.
            for target in recipe
                .imports
                .iter()
                .filter_map(|(_, name)| self.module_id(name).map(ModuleId::index))
            {
                if !seen[target] {
                    seen[target] = true;
                    pending.push(target);
                }
            }
            // Связи уже выделены; ёмкость временного списка не нужна в очереди тел.
            resolved.imports = Vec::new();
            let program_imports = imports
                .iter()
                .filter(|module| !module.is_self && module.component_access.is_none())
                .map(|module| bsl_bytecode::ModuleImport {
                    alias: module.alias.clone(),
                    module: ModuleId::new(module.module),
                    global: module.global,
                })
                .collect::<Vec<_>>();
            indices.push((
                index,
                bsl_bytecode::ModuleImports::sharing(program_imports, &self.common_imports),
            ));
            bodies.push(resolved);
        }
        let pairs = indices
            .iter()
            .zip(&bodies)
            .map(|((index, _), body)| (self.recipe.modules[*index].name.clone(), body))
            .collect::<Vec<_>>();
        // Каждый добавляемый набор использует штатный компилятор каталога,
        // включая его уплотнение узких индексов. Полные таблицы прежних
        // наборов не копируются; имена связываются с исполнением при входе.
        let start = phase();
        let (compiled, _) = bsl_compiler::compile_configuration(&pairs, None, self.build)?;
        times[6] += elapsed(start);
        let programs = compiled.modules.into_iter().map(|module| module.program);
        let prepared = indices
            .into_iter()
            .zip(programs)
            .map(|((index, imports), mut program)| {
                program.imports = imports;
                (
                    index,
                    ModuleProgram {
                        name: self.recipe.modules[index].name.clone(),
                        entry_visible: self.recipe.modules[index].visibility
                            != ModuleVisibility::Private,
                        global: self.recipe.modules[index].visibility == ModuleVisibility::Global,
                        program,
                    },
                )
            })
            .collect::<Vec<_>>();
        let overlay = BatchCatalog {
            base: self,
            prepared: &prepared,
        };
        let start = phase();
        for (index, module) in &prepared {
            self.validate_declarations(*index, &module.program)?;
            self.verifier.borrow_mut().verify_linked(
                &module.program,
                &overlay,
                Some(ModuleId::new(*index as u32)),
            )?;
        }
        times[7] += elapsed(start);
        if self.namespace.borrow().names.is_empty()
            && let Some((_, last)) = prepared.last()
        {
            *self.namespace.borrow_mut() = Namespace {
                names: last.program.names.clone(),
                shapes: last.program.shapes.clone(),
            };
        }
        for (index, program) in prepared {
            self.programs[index]
                .set(program)
                .map_err(|_| Error::Configuration("модуль подготовлен повторно".into()))?;
            self.sources[index].borrow_mut().take();
        }
        if trace {
            // Суммы по выбранному набору отделяют работу с метаданными от
            // компилятора; обычный запуск не читает часы внутри прохода.
            for (name, duration) in [
                "indexed.context",
                "indexed.source",
                "indexed.parse",
                "indexed.imports",
                "indexed.properties",
                "indexed.resolve",
                "indexed.compile",
                "indexed.verify",
            ]
            .into_iter()
            .zip(times)
            {
                eprintln!("open-bsl.startup\t{name}\t{:.6}", duration.as_secs_f64());
            }
        }
        if crate::startup_timings_enabled() {
            let count = self
                .programs
                .iter()
                .filter(|program| program.get().is_some())
                .count();
            eprintln!("open-bsl.work\tindexed.prepared_modules\t{count}");
        }
        Ok(())
    }
}

impl ImportedModuleProvider for OnDemandCatalog {
    fn module_surface(&self, module: u32) -> Result<&ImportedModule, bsl_sema::SemaError> {
        self.surface(module as usize).map_err(|message| {
            bsl_sema::SemaError::ImportedModuleUnavailable {
                module: self.recipe.modules[module as usize].name.clone(),
                message,
            }
        })
    }
}

impl ExecutionCatalog for OnDemandCatalog {
    fn module_id(&self, name: &str) -> Option<ModuleId> {
        self.module_positions
            .get(&name.to_uppercase())
            .map(|&index| ModuleId::new(index as u32))
    }
    fn module_count(&self) -> usize {
        self.recipe.modules.len()
    }
    fn module_name(&self, module: ModuleId) -> Option<&str> {
        self.recipe
            .modules
            .get(module.index())
            .map(|module| module.name.as_str())
    }
    fn prepared_module(&self, module: ModuleId) -> Option<&ModuleProgram> {
        self.programs.get(module.index()).and_then(OnceCell::get)
    }
    fn module_code(&self, module: ModuleId) -> Result<&ModuleProgram, bsl_rt::RtError> {
        let cell = self
            .programs
            .get(module.index())
            .ok_or(bsl_rt::RtError::InvalidBytecode(
                "номер модуля вне каталога конфигурации",
            ))?;
        if cell.get().is_none() {
            self.prepare(module)
                .map_err(|error| bsl_rt::RtError::DynamicError(error.to_string()))?;
        }
        cell.get().ok_or(bsl_rt::RtError::InvalidBytecode(
            "подготовка не вернула модуль",
        ))
    }
}

struct BatchCatalog<'a> {
    base: &'a OnDemandCatalog,
    prepared: &'a [(usize, ModuleProgram)],
}

impl ExecutionCatalog for BatchCatalog<'_> {
    fn module_count(&self) -> usize {
        self.base.module_count()
    }
    fn module_name(&self, module: ModuleId) -> Option<&str> {
        self.base.module_name(module)
    }
    fn prepared_module(&self, module: ModuleId) -> Option<&ModuleProgram> {
        self.prepared
            .iter()
            .find(|(index, _)| *index == module.index())
            .map(|(_, module)| module)
            .or_else(|| self.base.prepared_module(module))
    }
    fn module_code(&self, module: ModuleId) -> Result<&ModuleProgram, bsl_rt::RtError> {
        if let Some(prepared) = self.prepared_module(module) {
            return Ok(prepared);
        }
        // Этот каталог используется только верификатором и не попадает в VM.
        self.base
            .declaration_view(module.index())
            .map_err(|error| bsl_rt::RtError::DynamicError(error.to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub(crate) struct WorkerSource {
    pub source: Arc<dyn ConfigurationSource>,
    context_source: Option<Arc<dyn ModuleContextSource>>,
    component_modules: Vec<(String, String, String)>,
    defer_name_errors: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl WorkerSource {
    pub(crate) fn configure(&self, mut builder: EngineBuilder) -> EngineBuilder {
        builder = builder
            .configuration_source(self.source.clone())
            .defer_common_module_name_errors(self.defer_name_errors);
        if let Some(context) = &self.context_source {
            builder = builder.configuration_context_source(context.clone());
        }
        for (global, property, module) in &self.component_modules {
            builder = builder.configuration_component_module(global, property, module);
        }
        builder
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl OnDemandCatalog {
    pub(super) fn worker_source(&self) -> WorkerSource {
        WorkerSource {
            source: self.source.clone(),
            context_source: self.context_source.clone(),
            component_modules: self.component_modules.clone(),
            defer_name_errors: self.defer_name_errors,
        }
    }
}

#[cfg(test)]
#[path = "engine_on_demand_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "engine_catalog_lookup_tests.rs"]
mod lookup_tests;
