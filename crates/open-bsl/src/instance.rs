//! Явные экземпляры корневых модулей одного сеанса.

use std::collections::HashMap;
use std::rc::Rc;

use crate::{Error, Module, State, Value};

#[derive(Default)]
pub(crate) struct SessionInstances {
    identity: Rc<()>,
    roots: HashMap<u64, ResidentModule>,
    common: Option<bsl_vm::SessionModules>,
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
    /// каталога, либо если инициализация завершилась ошибкой. В последнем
    /// случае экземпляр не публикуется; повторная попытка исполняет тело
    /// заново, не откатывая уже сделанные изменения данных хоста и общих модулей.
    pub fn instantiate(&mut self, module: &Module) -> Result<ModuleInstance, Error> {
        if !self.engine.owns_module(module) {
            return Err(Error::Configuration(
                "модуль собран другим движком или для другого каталога".into(),
            ));
        }
        if !self.instances.roots.contains_key(&module.id) {
            let mut vm = bsl_vm::ProgramExecution::start_with_registry_and_scheduler(
                &module.program,
                self.engine.registry(),
                &self.host.env,
                self.scheduler,
            )?;
            self.drive_instance(module, &mut vm)?;
            self.instances.roots.insert(
                module.id,
                ResidentModule {
                    module: module.clone(),
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
        if let Some(catalog) = self.engine.catalog() {
            vm.attach_catalog(catalog);
            if let Some(session) = &mut self.instances.common {
                vm.swap_session_modules(session);
            }
        }
        // Сеанс возвращается владельцу при любой ошибке: частичные записи
        // сохраняются так же, как при обычном BSL-исключении внутри вызова.
        let result = (|| {
            if let (Some(catalog), Some(order)) =
                (self.engine.catalog(), self.engine.eager_init_order())
            {
                vm.schedule_eager_init(catalog, order)?;
            }
            loop {
                let result = match self.engine.catalog() {
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
        if self.engine.catalog().is_some() {
            vm.swap_session_modules(self.instances.common.get_or_insert_with(Default::default));
        }
        result
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
    pub fn call(
        &self,
        state: &mut State,
        name: &str,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Error> {
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
        let mut vm = bsl_vm::ProgramExecution::start_export_with_registry_and_scheduler(
            &resident.module.program,
            name,
            args,
            state.engine.registry(),
            &state.host.env,
            state.scheduler,
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
