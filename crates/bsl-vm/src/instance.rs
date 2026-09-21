//! Передача живого состояния модуля между последовательными запусками.
//! Кэши и задачи остаются собственностью отдельного `ProgramExecution`.

use super::*;

/// Переменные и идентичность модуля между вызовами хоста.
/// Состояние применимо только к неизменённой программе, создавшей его.
pub struct ModuleMemory(ModuleState);

impl ProgramExecution {
    /// Создаёт запуск экспортной процедуры или функции без исполнения тела
    /// модуля. До первого poll хост обязан вернуть память этого модуля
    /// через [`Self::replace_module_memory`]. Асинхронный экспорт исполняется
    /// как корневой вызов до завершения, включая порождённые задачи.
    ///
    /// # Errors
    /// Возвращает ошибку неизвестного или неэкспортного имени, точной арности,
    /// связывания компонентов либо некорректной программы.
    pub fn start_export_with_registry_and_scheduler(
        program: &Program,
        name: &str,
        args: Vec<BslValue>,
        registry: &bsl_rt::RuntimeRegistry,
        host_env: &bsl_rt::HostEnv,
        scheduler: SchedulerConfig,
    ) -> Result<Self, RtError> {
        let index = program
            .function_names
            .iter()
            .position(|candidate| bsl_rt::folded_eq(candidate, name))
            .ok_or_else(|| {
                RtError::DynamicError(format!(
                    "Процедура или функция «{name}» в модуле не найдена"
                ))
            })?;
        if !program
            .exported_functions
            .get(index)
            .copied()
            .unwrap_or(false)
        {
            return Err(RtError::DynamicError(format!(
                "Процедура или функция «{name}» не экспортируется"
            )));
        }
        let chunk = at(
            &program.chunks,
            index + 1,
            "номер чанка вне таблицы функций",
        )?;
        if args.len() != chunk.n_params as usize {
            return Err(RtError::DynamicError(format!(
                "Неверное число аргументов при вызове «{name}»: передано {}, а параметров {}",
                args.len(),
                chunk.n_params
            )));
        }
        let mut execution =
            Self::start_with_registry_and_scheduler(program, registry, host_env, scheduler)?;
        let root = execution.async_state.tasks[0]
            .as_mut()
            .expect("новый запуск содержит корневую задачу");
        root.frames[0].func_id = index + 1;
        root.stack = args;
        push_own_registers(&mut root.stack, chunk);
        Ok(execution)
    }

    /// Возвращает память модуля после успеха или ошибки. Оставляет в
    /// исполнителе пустое состояние: продолжать его опрос после этого нельзя.
    pub fn take_module_memory(&mut self) -> ModuleMemory {
        ModuleMemory(std::mem::replace(
            &mut self.module_state,
            ModuleState::empty(),
        ))
    }

    /// Подключает память ранее инициализированного модуля до первого poll.
    /// Программа должна совпадать с программой, создавшей эту память.
    pub fn replace_module_memory(&mut self, memory: ModuleMemory) {
        self.module_state = memory.0;
    }

    /// Обменивает состояние общих модулей с владельцем сеанса. Каталог
    /// должен совпадать с переданным в `attach_catalog`. Хост возвращает
    /// состояние сеансу и при ошибке, поскольку изменения не откатываются.
    pub fn swap_session_modules(&mut self, session: &mut SessionModules) {
        std::mem::swap(&mut self.session_modules, session);
    }
}
