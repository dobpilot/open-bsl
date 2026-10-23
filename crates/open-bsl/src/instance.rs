//! Явные экземпляры корневых модулей одного сеанса.

use std::collections::HashMap;
use std::rc::Rc;

use crate::{Error, Module, State, Value};

#[path = "instance_names.rs"]
mod names;

#[derive(Default)]
pub(crate) struct SessionInstances {
    identity: Rc<()>,
    roots: HashMap<u64, ResidentModule>,
    pub(crate) common: Option<bsl_vm::SessionModules>,
    pub(crate) runtime: Option<bsl_rt::RuntimeShapes>,
}

struct ResidentModule {
    module: Module,
    memory: bsl_vm::ModuleMemory,
}

/// Дескриптор инициализированного модуля в одном [`State`].
/// Клон обозначает тот же экземпляр. Переменные принадлежат сеансу;
/// дескриптор не продлевает его жизнь и не переносит данные в другой сеанс.
#[derive(Clone)]
pub struct ModuleInstance {
    session: Rc<()>,
    module_id: u64,
}

impl State {
    /// Инициализирует модуль в этом сеансе и возвращает его дескриптор.
    /// Повторное обращение с тем же [`Module`] или его клоном возвращает
    /// тот же экземпляр. Тело исполняется один раз; значение верхнеуровневого
    /// `Возврат` игнорируется. Общие модули разделяются экземплярами сеанса.
    /// [`State::run`] по-прежнему создаёт отдельный независимый запуск.
    ///
    /// # Errors
    /// Возвращает ошибку, если модуль собран другим движком или для другого
    /// каталога либо инициализация завершилась ошибкой. В последнем
    /// случае экземпляр не публикуется; после обычной ошибки повторная попытка
    /// исполняет тело заново, не откатывая изменения данных хоста и общих модулей.
    /// После критического отказа новое исполнение в этом состоянии запрещено.
    pub fn instantiate(&mut self, module: &Module) -> Result<ModuleInstance, Error> {
        self.check_active()?;
        if !self.engine.owns_module(module) {
            return Err(Error::Configuration(
                "модуль собран другим движком или для другого каталога".into(),
            ));
        }
        if !self.instances.roots.contains_key(&module.id) {
            let runtime = self.instances.runtime.get_or_insert_with(|| {
                let base = self
                    .engine
                    .execution_catalog()
                    .and_then(|catalog| {
                        (0..catalog.module_count())
                            .filter_map(|index| {
                                catalog.prepared_module(bsl_bytecode::ModuleId::new(index as u32))
                            })
                            .max_by_key(|module| module.program.names.len())
                    })
                    .map_or(&module.program, |module| &module.program);
                bsl_rt::RuntimeShapes::seeded(
                    base.names.to_vec(),
                    base.shapes.clone(),
                    Some(self.engine.registry()),
                )
            });
            let mut module = module.clone();
            names::bind(&mut module.program, runtime)?;
            let mut vm = bsl_vm::ProgramExecution::start_with_shared_runtime(
                &module.program,
                self.engine.registry(),
                &self.host.env,
                self.scheduler,
                runtime,
            )?;
            self.drive_instance(&module, &mut vm)?;
            self.instances.roots.insert(
                module.id,
                ResidentModule {
                    module,
                    memory: vm.take_module_memory(),
                },
            );
        }
        Ok(ModuleInstance {
            session: self.instances.identity.clone(),
            module_id: module.id,
        })
    }

    fn drive_instance(
        &mut self,
        module: &Module,
        vm: &mut bsl_vm::ProgramExecution,
    ) -> Result<(Value, Vec<Value>), Error> {
        self.dynamic.bind_module(module.id);
        if let Some(catalog) = self.engine.execution_catalog() {
            vm.attach_catalog(catalog);
            if let Some(session) = &mut self.instances.common {
                vm.swap_session_modules(session);
            }
        }
        // Сеанс возвращается владельцу при любой ошибке: частичные записи
        // сохраняются так же, как при обычном BSL-исключении внутри вызова.
        let result = (|| {
            if let (Some(catalog), Some(order)) = (
                self.engine.execution_catalog(),
                self.engine.eager_init_order(),
            ) {
                vm.schedule_eager_init(catalog, order)?;
            }
            loop {
                let result = match self.engine.execution_catalog() {
                    Some(catalog) => vm.poll_configuration_with_registry_and_io(
                        &module.program,
                        catalog,
                        self.engine.registry(),
                        &mut self.host.stdout,
                        &mut self.host.stderr,
                        &mut self.dynamic,
                        &mut self.host.env,
                        usize::MAX,
                    )?,
                    None => vm.poll_with_registry_and_io(
                        &module.program,
                        self.engine.registry(),
                        &mut self.host.stdout,
                        &mut self.host.stderr,
                        &mut self.dynamic,
                        &mut self.host.env,
                        usize::MAX,
                    )?,
                };
                match result {
                    bsl_vm::ProgramPoll::Complete(value, stack) => return Ok((value, stack)),
                    bsl_vm::ProgramPoll::Runnable | bsl_vm::ProgramPoll::Waiting => {}
                }
            }
        })();
        if self.engine.execution_catalog().is_some() {
            vm.swap_session_modules(self.instances.common.get_or_insert_with(Default::default));
        }
        vm.swap_runtime_shapes(
            self.instances
                .runtime
                .as_mut()
                .expect("таблицы остаются у владельца сеанса"),
        );
        self.finish_execution(result)
    }
}

impl ModuleInstance {
    /// Вызывает экспортную процедуру или функцию в исходном сеансе.
    /// Имя сравнивается без учёта регистра; все аргументы обязательны.
    /// Возвращает результат и конечные значения всех параметров, включая
    /// `Знач`. Асинхронный экспорт исполняется до завершения.
    ///
    /// # Errors
    /// Возвращает ошибку чужого сеанса, неизвестного или неэкспортного имени,
    /// неверного числа аргументов либо исполнения. При BSL-исключении
    /// экземпляр остаётся доступен; изменения не откатываются, а конечные
    /// параметры не возвращаются. Ошибки целостности VM не обещают возможности
    /// продолжения исполнения.
    /// Критический отказ запрещает последующие вызовы в том же состоянии.
    pub fn call(
        &self,
        state: &mut State,
        name: &str,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Error> {
        state.check_active()?;
        if !Rc::ptr_eq(&self.session, &state.instances.identity) {
            return Err(Error::Configuration(
                "экземпляр модуля принадлежит другому сеансу".into(),
            ));
        }
        let resident = state
            .instances
            .roots
            .get(&self.module_id)
            .expect("дескриптор опубликован после инициализации");
        let count = args.len();
        let mut vm = bsl_vm::ProgramExecution::start_export_with_shared_runtime(
            &resident.module.program,
            name,
            args,
            state.engine.registry(),
            &state.host.env,
            state.scheduler,
            state
                .instances
                .runtime
                .as_mut()
                .expect("таблицы созданы при инициализации экземпляра"),
        )?;
        let ResidentModule { module, memory } = state
            .instances
            .roots
            .remove(&self.module_id)
            .expect("проверен выше");
        vm.replace_module_memory(memory);
        let result = state.drive_instance(&module, &mut vm);
        state.instances.roots.insert(
            self.module_id,
            ResidentModule {
                module,
                memory: vm.take_module_memory(),
            },
        );
        result.map(|(value, mut stack)| {
            stack.truncate(count);
            (value, stack)
        })
    }
}
