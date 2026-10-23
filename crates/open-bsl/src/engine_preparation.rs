//! Ограниченная параллельная подготовка чистых данных компилятора.

use super::*;

pub(super) enum Failure {
    Parse(bsl_syntax::Diagnostic),
    Semantic(bsl_sema::SemaError),
    Configuration(String),
}

impl From<Failure> for Error {
    fn from(error: Failure) -> Self {
        match error {
            Failure::Parse(error) => Self::Parse(error),
            Failure::Semantic(error) => Self::Semantic(error),
            Failure::Configuration(message) => Self::Configuration(message),
        }
    }
}

impl Failure {
    pub(super) fn in_module(self, name: &str) -> Error {
        match self {
            Self::Configuration(message) => Error::Configuration(message),
            error => Error::InModule {
                module: name.into(),
                source: Box::new(error.into()),
            },
        }
    }
}

pub(super) fn workers() -> usize {
    #[cfg(target_arch = "wasm32")]
    return 1;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let requested = match std::env::var_os("OPEN_BSL_PREPARE_THREADS").as_deref() {
            Some(value) if value == "1" => 1,
            _ => 2,
        };
        requested.min(std::thread::available_parallelism().map_or(1, usize::from))
    }
}

pub(super) fn ordered_map<T: Sync, R: Send>(
    items: &[T],
    workers: usize,
    apply: impl Fn(usize, &T) -> R + Sync,
) -> Vec<R> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = workers;
        items
            .iter()
            .enumerate()
            .map(|(i, item)| apply(i, item))
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if workers <= 1 || items.len() <= 1 {
            return items
                .iter()
                .enumerate()
                .map(|(i, item)| apply(i, item))
                .collect();
        }
        let next = std::sync::atomic::AtomicUsize::new(0);
        let run = || {
            let mut results = Vec::new();
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(index) else { break };
                results.push((index, apply(index, item)));
            }
            results
        };
        std::thread::scope(|scope| {
            // Вызывающий поток тоже выполняет работу. При отказе создания
            // помощника он завершает очередь сам, без нового вида ошибки API.
            let helper = std::thread::Builder::new()
                .name("bsl-prepare".into())
                // Разбор глубокой вложенности в `bsl-syntax` откалиброван
                // под стек главного потока в 8 МиБ, а не стандартные 2 МиБ.
                .stack_size(8 * 1024 * 1024)
                .spawn_scoped(scope, run)
                .ok();
            let mut results = run();
            if let Some(helper) = helper {
                results.extend(
                    helper
                        .join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
                );
            }
            results.sort_unstable_by_key(|(index, _)| *index);
            results.into_iter().map(|(_, result)| result).collect()
        })
    }
}

pub(super) struct Analyzer<'a> {
    pub modules: &'a [ModuleRecipe],
    pub surfaces: &'a [bsl_sema::ImportedModule],
    pub component_imports: &'a [bsl_sema::ImportedModule],
    pub method_contexts: &'a [(String, &'static [bsl_rt::MethodDescriptor])],
    pub property_contexts: &'a [(String, Arc<[String]>)],
    pub registry: &'a bsl_rt::RuntimeRegistry,
    pub symbols: &'a bsl_syntax::PreprocSymbols,
    pub defer_name_errors: bool,
}

impl Analyzer<'_> {
    pub(super) fn batch(
        &self,
        indices: &[usize],
        workers: usize,
        selected: Option<&[bool]>,
    ) -> Vec<Result<bsl_sema::ResolvedProgram, Failure>> {
        if workers <= 1 || cfg!(target_arch = "wasm32") {
            return indices
                .iter()
                .map(|&current| self.resolve(current, selected))
                .collect();
        }
        ordered_map(indices, workers, |_, &current| {
            self.resolve(current, selected)
        })
    }

    pub(super) fn resolve(
        &self,
        current: usize,
        selected: Option<&[bool]>,
    ) -> Result<bsl_sema::ResolvedProgram, Failure> {
        let index_of = |name: &str| {
            self.modules
                .iter()
                .position(|m| bsl_rt::folded_eq(&m.name, name))
        };
        let mut imports = module_imports(
            self.modules,
            self.surfaces,
            current,
            &index_of,
            self.method_contexts,
        )?;
        if let Some(selected) = selected {
            imports.retain(|import| selected[import.module as usize]);
        } else {
            imports.extend(self.component_imports.iter().cloned());
        }
        let module = &self.modules[current];
        let syntax =
            bsl_syntax::parse_with_symbols(&module.source, self.symbols).map_err(Failure::Parse)?;
        let mut resolved = bsl_sema::resolve_program_with(
            &syntax.items,
            &bsl_sema::ResolveOptions::new(self.registry)
                .imports(&imports)
                .configuration_module()
                .receiver_properties(receiver_properties(self.property_contexts, &module.name))
                .defer_name_errors(
                    self.defer_name_errors
                        && matches!(
                            module.visibility,
                            ModuleVisibility::Named | ModuleVisibility::Global
                        ),
                ),
        )
        .map_err(Failure::Semantic)?;
        // Не переносим полное окружение в сохраняемые результаты: число
        // одновременно живых AST и окружений ограничено рабочими потоками.
        resolved.imports = Vec::new();
        drop(syntax);
        drop(imports);
        Ok(resolved)
    }
}

#[cfg(test)]
#[path = "engine_preparation_tests.rs"]
mod tests;
