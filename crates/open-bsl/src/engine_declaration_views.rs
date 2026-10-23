//! Объявления для проверки связей без подготовки исполняемых тел.

use super::*;
use bsl_bytecode::Chunk;

impl OnDemandCatalog {
    // Структура нужна только обычной проверке связей: инструкции отсутствуют,
    // и исполняемый каталог никогда не возвращает её вместо настоящего кода.
    pub(super) fn declaration_view(&self, index: usize) -> Result<&ModuleProgram, Error> {
        self.declaration_views[index]
            .get_or_init(|| {
                let surface = self.surface(index)?;
                let functions = surface
                    .functions
                    .iter()
                    .map(|function| usize::from(function.chunk))
                    .max()
                    .unwrap_or(0);
                let variables = surface
                    .variables
                    .iter()
                    .map(|variable| usize::from(variable.slot) + 1)
                    .max()
                    .unwrap_or(0);
                let mut program = Program {
                    requirements: Vec::new(),
                    chunks: (0..=functions).map(|_| Chunk::new()).collect(),
                    names: Vec::new().into(),
                    shapes: Vec::new(),
                    top_level_locals: Vec::new(),
                    module_vars: vec![String::new(); variables],
                    function_names: vec![String::new(); functions],
                    exported_functions: vec![false; functions],
                    exported_module_vars: vec![false; variables],
                    links: Vec::new(),
                    imports: Default::default(),
                    lines: Vec::new(),
                };
                for function in &surface.functions {
                    let index = usize::from(function.chunk);
                    let chunk = &mut program.chunks[index];
                    chunk.n_params = function
                        .param_by_val
                        .len()
                        .try_into()
                        .map_err(|_| "слишком много параметров объявления".to_string())?;
                    chunk.param_by_val = function.param_by_val.clone();
                    chunk.param_has_default = function.param_has_default.clone();
                    chunk.is_procedure = function.is_procedure;
                    chunk.is_async = function.is_async;
                    program.function_names[index - 1] = function.name.clone();
                    program.exported_functions[index - 1] = true;
                }
                for variable in &surface.variables {
                    let index = usize::from(variable.slot);
                    program.module_vars[index] = variable.name.clone();
                    program.exported_module_vars[index] = true;
                }
                Ok(ModuleProgram {
                    name: self.recipe.modules[index].name.clone(),
                    entry_visible: self.recipe.modules[index].visibility
                        != ModuleVisibility::Private,
                    global: self.recipe.modules[index].visibility == ModuleVisibility::Global,
                    program,
                })
            })
            .as_ref()
            .map_err(|message| Error::Configuration(message.clone()))
    }

    // Источник может отдавать объявления отдельно от исходника. Проверяем их
    // согласованность до публикации реального кода, а не доверяем обещанию хоста.
    pub(super) fn validate_declarations(
        &self,
        index: usize,
        program: &Program,
    ) -> Result<(), Error> {
        let surface = self.surface(index).map_err(Error::Configuration)?;
        let functions_match = surface.functions.len()
            == program
                .exported_functions
                .iter()
                .filter(|export| **export)
                .count()
            && surface.functions.iter().all(|function| {
                let index = usize::from(function.chunk);
                let Some(chunk) = program.chunks.get(index) else {
                    return false;
                };
                program.exported_functions.get(index - 1) == Some(&true)
                    && program
                        .function_names
                        .get(index - 1)
                        .is_some_and(|name| bsl_rt::folded_eq(name, &function.name))
                    && usize::from(chunk.n_params) == function.param_by_val.len()
                    && chunk.param_by_val == function.param_by_val
                    && chunk.param_has_default == function.param_has_default
                    && chunk.is_procedure == function.is_procedure
                    && chunk.is_async == function.is_async
            });
        let variables_match = surface.variables.len()
            == program
                .exported_module_vars
                .iter()
                .filter(|export| **export)
                .count()
            && surface.variables.iter().all(|variable| {
                let index = usize::from(variable.slot);
                program.exported_module_vars.get(index) == Some(&true)
                    && program
                        .module_vars
                        .get(index)
                        .is_some_and(|name| bsl_rt::folded_eq(name, &variable.name))
            });
        if functions_match && variables_match {
            Ok(())
        } else {
            Err(Error::Configuration(format!(
                "объявления модуля «{}» не соответствуют скомпилированному коду",
                self.recipe.modules[index].name
            )))
        }
    }
}
