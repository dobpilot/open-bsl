//! Индексы поиска в неизменяемом импортном окружении одного модуля.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

use super::ImportedModule;

pub(super) struct ImportLookup<'a> {
    modules: &'a [ImportedModule],
    aliases: RefCell<HashMap<String, Option<usize>>>,
    functions: OnceCell<HashMap<String, usize>>,
    globals: Vec<usize>,
    receiver_methods: Vec<&'static bsl_rt::MethodDescriptor>,
}

impl<'a> ImportLookup<'a> {
    pub(super) fn new(modules: &'a [ImportedModule]) -> Self {
        Self {
            modules,
            aliases: RefCell::new(HashMap::new()),
            functions: OnceCell::new(),
            globals: modules
                .iter()
                .enumerate()
                .filter_map(|(index, module)| module.global.then_some(index))
                .collect(),
            receiver_methods: modules
                .iter()
                .filter(|module| module.is_self)
                .flat_map(|module| module.receiver_methods)
                .collect(),
        }
    }

    pub(super) fn alias(&self, name: &str) -> Option<&'a ImportedModule> {
        self.cached(&self.aliases, name, || {
            self.modules.iter().position(|module| {
                module.component_access.is_none() && bsl_rt::folded_eq(&module.alias, name)
            })
        })
    }

    pub(super) fn global_function(&self, name: &str) -> Option<&'a ImportedModule> {
        // Глобальная поверхность обычно проверяется для многих разных имён.
        // Первый запрос строит один индекс вместо нового прохода всех экспортов
        // для каждого отсутствующего имени. Модули без таких запросов его не создают.
        let functions = self.functions.get_or_init(|| {
            let mut functions = HashMap::new();
            for &index in &self.globals {
                for function in &self.modules[index].functions {
                    functions
                        .entry(function.name.to_uppercase())
                        .or_insert(index);
                }
            }
            functions
        });
        functions
            .get(&name.to_uppercase())
            .map(|&index| &self.modules[index])
    }

    pub(super) fn receiver_methods(
        &self,
    ) -> impl Iterator<Item = &'static bsl_rt::MethodDescriptor> + '_ {
        self.receiver_methods.iter().copied()
    }

    fn cached(
        &self,
        cache: &RefCell<HashMap<String, Option<usize>>>,
        name: &str,
        search: impl FnOnce() -> Option<usize>,
    ) -> Option<&'a ImportedModule> {
        let key = name.to_uppercase();
        let known = cache.borrow().get(&key).copied();
        let index = known.unwrap_or_else(|| {
            // Отрицательный результат тоже неизменяем. Затенение переменной
            // проверяет вызывающий до кэша: здесь нет результата резолвинга.
            let found = search();
            cache.borrow_mut().insert(key, found);
            found
        })?;
        self.modules.get(index)
    }
}
