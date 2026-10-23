use std::cell::{Cell, OnceCell};

use bsl_bytecode::{ExecutionCatalog, ModuleId, ModuleProgram};

use super::*;

struct Indexed {
    names: Vec<String>,
    programs: Vec<OnceCell<ModuleProgram>>,
    loads: Cell<usize>,
}

impl Indexed {
    fn new() -> Self {
        Self {
            names: (0..64).map(|index| format!("Модуль{index}")).collect(),
            programs: (0..64).map(|_| OnceCell::new()).collect(),
            loads: Cell::new(0),
        }
    }
}

impl ExecutionCatalog for Indexed {
    fn module_count(&self) -> usize {
        self.names.len()
    }

    fn module_name(&self, id: ModuleId) -> Option<&str> {
        self.names.get(id.index()).map(String::as_str)
    }

    fn prepared_module(&self, id: ModuleId) -> Option<&ModuleProgram> {
        self.programs.get(id.index()).and_then(OnceCell::get)
    }

    fn module_code(&self, id: ModuleId) -> Result<&ModuleProgram, RtError> {
        let name = self
            .module_name(id)
            .ok_or(RtError::InvalidBytecode("нет модуля"))?;
        Ok(self.programs[id.index()].get_or_init(|| {
            self.loads.set(self.loads.get() + 1);
            ModuleProgram {
                name: name.into(),
                entry_visible: true,
                global: false,
                program: tests::compile_module(
                    "Перем Счетчик Экспорт; Счетчик = 0;
                     Функция Следующее() Экспорт Счетчик = Счетчик + 1; Возврат Счетчик; КонецФункции",
                ),
            }
        }))
    }
}

struct NoDynamic;

#[test]
fn prepared_catalog_allocates_instruction_caches_only_for_entered_modules() {
    let catalog = Indexed::new();
    let target = ModuleId::new(17);
    catalog.module_code(target).unwrap();
    let caches = RunCaches::for_catalog(&catalog);
    assert!(caches.iter().all(|cache| cache.prop.is_empty()));
    let program = &catalog.prepared_module(target).unwrap().program;
    let ready = caches[target.index()].ready(program);
    assert_eq!(ready.prop.len(), program.chunks.len());
    assert!(std::ptr::eq(ready, caches[target.index()].ready(program)));
    assert!(caches.iter().enumerate().all(|(index, cache)| {
        index == target.index()
            || cache
                .deferred
                .as_ref()
                .is_some_and(|cell| cell.get().is_none())
    }));
    let second = RunCaches::for_catalog(&catalog);
    assert!(second.iter().all(|cache| cache.prop.is_empty()));
    assert_eq!(catalog.loads.get(), 1);
}

impl bsl_bytecode::DynamicCompiler for NoDynamic {
    fn compile(
        &mut self,
        _: &bsl_bytecode::DynamicRequest<'_>,
    ) -> Result<std::rc::Rc<bsl_bytecode::DynamicUnit>, String> {
        Err("фрагменты не используются в этом тесте".into())
    }
}

fn registry() -> bsl_rt::RuntimeRegistry {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    builder.build().unwrap()
}

#[test]
fn indexed_attachment_does_not_load_bodies_or_allocate_instruction_caches() {
    let catalog = Indexed::new();
    assert_eq!(catalog.module_id("мОДУЛЬ17"), Some(ModuleId::new(17)));
    let entry = tests::compile_module("Возврат 1;");
    let registry = registry();
    let env = bsl_rt::HostEnv::process();
    let mut execution = ProgramExecution::start_with_registry(&entry, &registry, &env).unwrap();
    execution.attach_catalog(&catalog);
    assert_eq!(catalog.loads.get(), 0);
    assert!(
        execution
            .session_modules
            .instances
            .iter()
            .all(|module| module.state.slots.is_empty())
    );
    assert!(
        execution
            .catalog_caches
            .iter()
            .all(|cache| cache.prop.is_empty())
    );
}

#[test]
fn indexed_handler_prepares_only_target_and_reuses_code_with_isolated_state() {
    let catalog = Indexed::new();
    let registry = registry();
    let mut env = bsl_rt::HostEnv::process();
    let mut dynamic = NoDynamic;
    let mut first = SessionModules::for_catalog(&catalog);
    let call =
        |session: &mut SessionModules, env: &mut bsl_rt::HostEnv, dynamic: &mut NoDynamic| {
            call_configuration_handler(
                &catalog,
                session,
                "мОДУЛЬ17",
                "Следующее",
                Vec::new(),
                bsl_rt::ConfigurationHandlerEntryMode::Inherit,
                &registry,
                &mut Vec::new(),
                &mut Vec::new(),
                dynamic,
                env,
                None,
            )
            .unwrap()
            .0
        };
    assert_eq!(
        call(&mut first, &mut env, &mut dynamic),
        BslValue::number_from_i64(1)
    );
    assert_eq!(
        call(&mut first, &mut env, &mut dynamic),
        BslValue::number_from_i64(2)
    );
    let mut second = SessionModules::for_catalog(&catalog);
    assert_eq!(
        call(&mut second, &mut env, &mut dynamic),
        BslValue::number_from_i64(1)
    );
    assert_eq!(catalog.loads.get(), 1);
    assert_eq!(
        catalog
            .programs
            .iter()
            .filter(|program| program.get().is_some())
            .count(),
        1
    );
}
