//! Вызовы модулей конкретных объектов тем же исполнителем, что и обычные модули.

use std::rc::{Rc, Weak};

use super::*;
use bsl_rt::ConfigurationInstance;

struct ObjectMemory {
    owner: Weak<()>,
    program: usize,
    state: Option<ModuleState>,
    init: ModuleInitState,
}

#[derive(Clone, Copy)]
enum StateSource {
    Instance,
    Current,
    Root,
}

fn instance_of(value: &BslValue) -> Option<&ConfigurationInstance> {
    let BslValue::Object(object) = value else {
        return None;
    };
    let bsl_rt::BslObject::Extension(object) = object.as_ref() else {
        return None;
    };
    object.as_dyn().configuration_instance()
}

fn memory(storage: &mut Option<Box<dyn std::any::Any>>) -> Result<&mut ObjectMemory, RtError> {
    storage
        .as_mut()
        .and_then(|value| value.downcast_mut())
        .ok_or_else(|| {
            RtError::DynamicError("память экземпляра не принадлежит исполнителю модулей".into())
        })
}

fn validate_memory(
    memory: &ObjectMemory,
    owner: &Rc<()>,
    program_identity: usize,
) -> Result<(), RtError> {
    if !memory
        .owner
        .upgrade()
        .is_some_and(|original| Rc::ptr_eq(&original, owner))
    {
        return Err(RtError::DynamicError(
            "экземпляр модуля принадлежит другому сеансу".into(),
        ));
    }
    if memory.program != program_identity {
        return Err(RtError::DynamicError(
            "экземпляр создан для другого модуля".into(),
        ));
    }
    Ok(())
}

fn object_state<'a>(
    current: &'a mut ModuleState,
    modules: &'a mut ModulesCtx<'_, '_>,
    source: StateSource,
) -> Result<&'a mut ModuleState, RtError> {
    match source {
        StateSource::Current => Ok(current),
        StateSource::Root => modules
            .root_state
            .as_deref_mut()
            .ok_or(RtError::InvalidBytecode(
                "корневая память объекта отсутствует",
            )),
        StateSource::Instance => Err(RtError::InvalidBytecode(
            "память объекта находится в экземпляре",
        )),
    }
}

fn active_object(
    current: &ModuleState,
    modules: &ModulesCtx<'_, '_>,
) -> Option<(StateSource, BslValue)> {
    if let Ok(value) = current.reference()
        && instance_of(&value).is_some()
    {
        return Some((StateSource::Current, value));
    }
    if let Some(root) = modules.root_state.as_deref()
        && let Ok(value) = root.reference()
        && instance_of(&value).is_some()
    {
        return Some((StateSource::Root, value));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn call(
    module: &str,
    export: Option<&str>,
    require_export: bool,
    entry_mode: bsl_rt::ConfigurationInstanceEntryMode,
    instance: &ConfigurationInstance,
    receiver: BslValue,
    mut args: Vec<BslValue>,
    current_module: u32,
    current: &mut ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    host: &mut HostIo<'_, '_>,
    caller_shapes: Option<&mut bsl_rt::RuntimeShapes>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let caller_shapes = caller_shapes.ok_or_else(|| {
        RtError::DynamicError("вызову объекта не передан контекст имён и форм".into())
    })?;
    let catalog = modules.catalog.ok_or(RtError::CapabilityMissing {
        capability: bsl_rt::Capability::ConfigurationModuleCaller,
        path: bsl_rt::ContextKind::Full,
    })?;
    let target = catalog
        .catalog
        .module_id(module)
        .ok_or_else(|| RtError::DynamicError(format!("Модуль конфигурации «{module}» не найден")))?
        .index() as u32;
    let program = catalog.program(target)?;
    let linked = catalog.linked(target)?;
    let owner = host
        .env
        .as_ref()
        .ok_or(RtError::CapabilityMissing {
            capability: bsl_rt::Capability::ConfigurationModuleCaller,
            path: bsl_rt::ContextKind::Full,
        })?
        .module_instance_owner();
    let program_identity = program as *const Program as usize;
    let initialized = {
        let mut storage = instance.storage()?;
        if storage.is_none() {
            *storage = Some(Box::new(ObjectMemory {
                owner: Rc::downgrade(&owner),
                program: program_identity,
                state: Some(ModuleState::new(program)),
                init: ModuleInitState::NotStarted,
            }));
        }
        let memory = memory(&mut storage)?;
        validate_memory(memory, &owner, program_identity)?;
        memory.init
    };
    if initialized == ModuleInitState::Ready && export.is_none() {
        return Ok((BslValue::Undefined, Vec::new()));
    }
    if initialized != ModuleInitState::Ready && export.is_some() {
        // На 1С экспорт через `ЭтотОбъект` недоступен из тела инициализации:
        // чистая функция, возвращающая 17 без чтения реквизитов, отвергнута
        // в `measure-catalog-object-properties` (`body.export.reentry`).
        return Err(RtError::DynamicError(
            "модуль объекта ещё не инициализирован".into(),
        ));
    }
    if initialized != ModuleInitState::NotStarted && export.is_none() {
        return Err(RtError::DynamicError(
            "инициализация модуля объекта не может быть повторена".into(),
        ));
    }
    let function = match export {
        None => 0,
        Some(name) => {
            let index = program
                .function_names
                .iter()
                .position(|candidate| bsl_rt::folded_eq(candidate, name))
                .ok_or_else(|| {
                    RtError::DynamicError(format!(
                        "Процедура или функция «{name}» модуля «{module}» не найдена"
                    ))
                })?;
            if require_export
                && !program
                    .exported_functions
                    .get(index)
                    .copied()
                    .unwrap_or(false)
            {
                return Err(RtError::DynamicError(format!(
                    "Процедура или функция «{name}» модуля «{module}» не экспортируется"
                )));
            }
            index + 1
        }
    };
    let chunk = at(&program.chunks, function, "номер чанка вне таблицы функций")?;
    let count = args.len();
    if count > chunk.n_params as usize
        || chunk
            .param_has_default
            .iter()
            .skip(count)
            .any(|has_default| !has_default)
    {
        return Err(RtError::DynamicError(format!(
            "Неверное число аргументов при вызове модуля «{module}»"
        )));
    }
    args.resize(chunk.n_params as usize, BslValue::Undefined);
    push_own_registers(&mut args, chunk);

    let active = active_object(current, modules);
    let own_active = active.as_ref().filter(|(_, value)| {
        instance_of(value).is_some_and(|active| std::ptr::eq(active, instance))
    });
    let (source, state) = {
        let mut storage = instance.storage()?;
        let memory = memory(&mut storage)?;
        if export.is_none() {
            memory.init = ModuleInitState::Initializing;
        }
        match memory.state.take() {
            Some(state) => (StateSource::Instance, state),
            None => {
                let source = own_active.map(|(source, _)| *source).ok_or_else(|| {
                    RtError::DynamicError("активная память экземпляра объекта недоступна".into())
                })?;
                (
                    source,
                    object_state(current, modules, source)?.take_for_execution(),
                )
            }
        }
    };
    // Память приостановленного объекта доступна обратному вызову из другого
    // объекта. Между вызовами не клонируем переменные и не удерживаем получателя.
    let parked = active.filter(|(_, value)| {
        instance_of(value).is_some_and(|active| !std::ptr::eq(active, instance))
    });
    if let Some((source, value)) = &parked {
        let active_instance = instance_of(value).expect("получено из активного объекта");
        memory(&mut *active_instance.storage()?)?.state =
            Some(object_state(current, modules, *source)?.take_for_execution());
    }
    let park_common = current_module != ROOT_MODULE && !matches!(source, StateSource::Current);
    if park_common {
        let common = modules
            .session
            .instances
            .get_mut(current_module as usize)
            .ok_or(RtError::InvalidBytecode(
                "текущий модуль вне сессии конфигурации",
            ))?;
        std::mem::swap(&mut common.state, current);
    }

    let mut state = state;
    let original_reference = state.replace_reference(receiver);
    let mut execution = ProgramExecution::new_linked(
        program,
        function,
        args,
        linked,
        state,
        SchedulerConfig::default(),
    );
    let frame = &mut execution.async_state.tasks[0]
        .as_mut()
        .expect("создана корневая задача")
        .frames[0];
    frame.param_aliases = (0..chunk.n_params as usize)
        .map(|idx| ParamSlot {
            idx,
            provided: idx < count,
        })
        .collect();
    frame.own_base = chunk.n_params as usize;
    // Исключение относится только к вложенному платформенному событию.
    // Счётчики и защёлка вызывающего не меняются даже при ошибке обработчика.
    let mut security_modes = host.security_modes.clone();
    match entry_mode {
        bsl_rt::ConfigurationInstanceEntryMode::Inherit => {}
        bsl_rt::ConfigurationInstanceEntryMode::WithoutSafeMode => {
            security_modes.safe_disabled = true;
        }
        bsl_rt::ConfigurationInstanceEntryMode::OwnExtensionSafeMode(safe) => {
            if safe {
                security_modes.safe = true;
                security_modes.safe_disabled = false;
            } else {
                security_modes.safe_disabled = true;
            }
        }
    }
    execution.inherit_security_modes(security_modes);
    if entry_mode == bsl_rt::ConfigurationInstanceEntryMode::OwnExtensionSafeMode(false) {
        // Платформа даёт событию собственный снимаемый уровень подавления.
        // Его счётчик и защёлка принадлежат только новому корневому кадру.
        let root = execution.async_state.tasks[0]
            .as_mut()
            .expect("создана корневая задача");
        root.security_modes.synchronize_frames(root.frames.len());
        root.security_modes.by_frame[0].safe_disabled.set(1);
        root.security_modes.by_frame[0]
            .safe_disabled_latch
            .set(true);
    }
    execution.root_catalog_module = Some(target);
    execution.attach_catalog(catalog.catalog);
    // Entry уже делит префикс имён и форм каталога, но может дополнять его.
    // Новые динамические поля должны получать идентичности вызывающего,
    // иначе его последующее чтение структуры использует другой `NameId`.
    std::mem::swap(&mut execution.runtime_shapes, caller_shapes);
    execution.swap_session_modules(modules.session);
    let result = loop {
        match execution.poll_linked(program, linked, Some(catalog), host, usize::MAX, None) {
            Ok(ProgramPoll::Complete(value, stack)) => {
                break Ok((value, stack[..chunk.n_params as usize].to_vec()));
            }
            Ok(ProgramPoll::Runnable | ProgramPoll::Waiting) => {}
            Err(error) => break Err(error),
        }
    };
    execution.swap_session_modules(modules.session);
    std::mem::swap(&mut execution.runtime_shapes, caller_shapes);
    let mut state = execution.take_module_memory().0;
    if let Some(reference) = original_reference {
        state.replace_reference(reference);
    }
    {
        let mut storage = instance.storage()?;
        let memory = memory(&mut storage)?;
        if export.is_none() {
            memory.init = if result.is_ok() {
                ModuleInitState::Ready
            } else {
                ModuleInitState::Failed
            };
        }
        match source {
            StateSource::Instance => memory.state = Some(state),
            _ => *object_state(current, modules, source)? = state,
        }
    }
    if park_common {
        std::mem::swap(
            &mut modules.session.instances[current_module as usize].state,
            current,
        );
    }
    if let Some((source, value)) = parked {
        let active_instance = instance_of(&value).expect("получено из активного объекта");
        let state = memory(&mut *active_instance.storage()?)?
            .state
            .take()
            .ok_or(RtError::InvalidBytecode(
                "память приостановленного объекта потеряна",
            ))?;
        *object_state(current, modules, source)? = state;
    }
    result
}

// Экспортная поверхность уже находится в программе. Эта проверка не берёт
// память экземпляра и не выполняет ни тело, ни экспортный обработчик.
pub(super) fn has_export(
    module: &str,
    name: &str,
    modules: &ModulesCtx<'_, '_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = compiled_program(module, modules)?;
    let found = program
        .function_names
        .iter()
        .enumerate()
        .any(|(index, candidate)| {
            program.exported_functions[index] && bsl_rt::folded_eq(candidate, name)
        });
    Ok((BslValue::Boolean(found), Vec::new()))
}

fn compiled_program<'a>(
    module: &str,
    modules: &'a ModulesCtx<'_, '_>,
) -> Result<&'a Program, RtError> {
    let catalog = modules.catalog.ok_or(RtError::CapabilityMissing {
        capability: bsl_rt::Capability::ConfigurationModuleCaller,
        path: bsl_rt::ContextKind::Full,
    })?;
    let target = catalog.catalog.module_id(module).ok_or_else(|| {
        RtError::DynamicError(format!("Модуль конфигурации «{module}» не найден"))
    })?;
    Ok(&catalog.catalog.module_code(target)?.program)
}

pub(super) fn function_parameter_count(
    module: &str,
    name: &str,
    modules: &ModulesCtx<'_, '_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = compiled_program(module, modules)?;
    let value = match program
        .function_names
        .iter()
        .position(|candidate| bsl_rt::folded_eq(candidate, name))
    {
        Some(index) => BslValue::number_from_i64(i64::from(
            at(&program.chunks, index + 1, "функция вне таблицы чанков")?.n_params,
        )),
        None => BslValue::Undefined,
    };
    Ok((value, Vec::new()))
}

/// Доступ к существующим слотам не исполняет код и не создаёт другую память.
#[allow(clippy::too_many_arguments)]
pub(super) fn variable(
    module: &str,
    name: &str,
    instance: &ConfigurationInstance,
    value: Option<BslValue>,
    current: &mut ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    host: &mut HostIo<'_, '_>,
) -> Result<(BslValue, Vec<BslValue>), RtError> {
    let program = compiled_program(module, modules)?;
    let index = program
        .module_vars
        .iter()
        .enumerate()
        .find_map(|(index, candidate)| {
            (program.exported_module_vars[index] && bsl_rt::folded_eq(candidate, name))
                .then_some(index)
        })
        .ok_or_else(|| RtError::UnknownProperty(name.into()))?;
    let program_identity = program as *const Program as usize;
    let owner = host
        .env
        .as_ref()
        .ok_or(RtError::CapabilityMissing {
            capability: bsl_rt::Capability::ConfigurationModuleCaller,
            path: bsl_rt::ContextKind::Full,
        })?
        .module_instance_owner();
    let mut storage = instance.storage()?;
    let memory = memory(&mut storage)?;
    validate_memory(memory, &owner, program_identity)?;
    if memory.init != ModuleInitState::Ready {
        return Err(RtError::DynamicError(
            "экземпляр модуля ещё не инициализирован".into(),
        ));
    }
    let state = if let Some(state) = memory.state.as_mut() {
        state
    } else {
        let (source, receiver) = active_object(current, modules).ok_or_else(|| {
            RtError::DynamicError("активная память экземпляра объекта недоступна".into())
        })?;
        if !instance_of(&receiver).is_some_and(|active| std::ptr::eq(active, instance)) {
            return Err(RtError::DynamicError(
                "активная память экземпляра объекта недоступна".into(),
            ));
        }
        object_state(current, modules, source)?
    };
    let slot = state.slots.get_mut(index).ok_or(RtError::InvalidBytecode(
        "экспортная переменная вне памяти модуля",
    ))?;
    let result = if let Some(value) = value {
        *slot = value;
        BslValue::Undefined
    } else {
        slot.clone()
    };
    Ok((result, Vec::new()))
}
