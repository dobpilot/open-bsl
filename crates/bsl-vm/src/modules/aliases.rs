use super::{ModuleState, ModulesCtx, ROOT_MODULE, SessionModules, ensure_module_ready};
use crate::{Frame, ParamSlot, Task, reg_load, reg_store};
use bsl_bytecode::{ArgMode, LinkEntry, Program};
use bsl_rt::{BslValue, RtError};

fn imported_target(program: &Program, link: u16) -> Result<(u32, usize), RtError> {
    match program.links.get(link as usize) {
        Some(&LinkEntry::Variable { module, slot }) => Ok((module.index() as u32, slot as usize)),
        _ => Err(RtError::InvalidBytecode(
            "byimport ведёт мимо таблицы связей или на функцию",
        )),
    }
}

/// Инициализация происходит до изменения аргументов и продвижения Call.
/// Добавленный кадр тела модуля означает повтор вызова после его возврата.
pub(crate) fn ensure_imported_arguments_ready(
    program: &Program,
    modes: &[ArgMode],
    modules: &mut ModulesCtx<'_, '_>,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
) -> Result<bool, RtError> {
    for mode in modes {
        if let ArgMode::ByRefImportedVar(link) = mode {
            let (owner, _) = imported_target(program, *link)?;
            let catalog = modules.catalog.ok_or(RtError::InvalidBytecode(
                "режим byimport вне каталога конфигурации",
            ))?;
            if ensure_module_ready(owner, catalog, modules.session, frames, stack)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Модуль уже подготовлен до материализации аргументов. Ссылка на самого
/// себя запрещена проверкой каталога, поэтому состояние не изъято шагом VM.
pub(crate) fn load_imported_argument(
    program: &Program,
    link: u16,
    session: &SessionModules,
    frames: &[Frame],
    stack: &[BslValue],
) -> Result<(u32, usize, BslValue), RtError> {
    let (owner, slot) = imported_target(program, link)?;
    let instance = session
        .instances
        .get(owner as usize)
        .ok_or(RtError::InvalidBytecode("связь ведёт мимо сессии модулей"))?;
    let value = load_module_argument(owner, slot, frames, stack, &instance.state.slots)?;
    Ok((owner, slot, value))
}

/// Активный параметр без Знач — та же переменная, а не снимок до возврата.
/// Вложенный вызов повторно использует существующую ячейку стека.
pub(crate) fn find_module_alias(frames: &[Frame], owner: u32, slot: usize) -> Option<usize> {
    frames
        .iter()
        .rev()
        .flat_map(|frame| &frame.module_copybacks)
        .find_map(|&(index, module, target)| (module == owner && target == slot).then_some(index))
}

pub(crate) fn load_module_argument(
    owner: u32,
    slot: usize,
    frames: &[Frame],
    stack: &[BslValue],
    module_slots: &[BslValue],
) -> Result<BslValue, RtError> {
    match find_module_alias(frames, owner, slot) {
        Some(index) => reg_load(stack, index),
        None => reg_load(module_slots, slot),
    }
}

pub(crate) fn bind_module_argument(
    owner: u32,
    slot: usize,
    value: BslValue,
    frames: &[Frame],
    pending: &mut Vec<(usize, u32, usize)>,
    stack: &mut Vec<BslValue>,
) -> ParamSlot {
    let existing = pending
        .iter()
        .find_map(|&(index, module, target)| (module == owner && target == slot).then_some(index))
        .or_else(|| find_module_alias(frames, owner, slot));
    let idx = existing.unwrap_or_else(|| {
        let index = stack.len();
        stack.push(value);
        pending.push((index, owner, slot));
        index
    });
    ParamSlot {
        idx,
        provided: true,
    }
}

fn state_for_module<'a>(
    owner: u32,
    root: &'a mut ModuleState,
    session: &'a mut SessionModules,
) -> Result<&'a mut ModuleState, RtError> {
    if owner == ROOT_MODULE {
        Ok(root)
    } else {
        session
            .instances
            .get_mut(owner as usize)
            .map(|instance| &mut instance.state)
            .ok_or(RtError::InvalidBytecode(
                "модуль ссылочного параметра вне сессии",
            ))
    }
}

/// Перед переключением задачи значения общих переменных публикуются в
/// состояниях модулей. Внутри задачи все обращения идут к одной ячейке.
pub(crate) fn publish_aliases(
    task: &Task,
    root: &mut ModuleState,
    session: &mut SessionModules,
) -> Result<(), RtError> {
    for frame in &task.frames {
        for &(index, owner, slot) in &frame.module_copybacks {
            let state = state_for_module(owner, root, session)?;
            reg_store(&mut state.slots, slot, reg_load(&task.stack, index)?)?;
        }
    }
    Ok(())
}

/// Возобновляемый кадр должен видеть записи других задач и динамического
/// исполнителя, а не прежний снимок перед уступкой управления.
pub(crate) fn refresh_aliases(
    task: &mut Task,
    root: &mut ModuleState,
    session: &mut SessionModules,
) -> Result<(), RtError> {
    for frame in &task.frames {
        for &(index, owner, slot) in &frame.module_copybacks {
            let state = state_for_module(owner, root, session)?;
            reg_store(&mut task.stack, index, reg_load(&state.slots, slot)?)?;
        }
    }
    Ok(())
}

pub(crate) fn park_task(
    id: crate::TaskId,
    task: Task,
    scheduler: &mut crate::AsyncState,
    root: &mut ModuleState,
    session: &mut SessionModules,
) -> Result<(), RtError> {
    publish_aliases(&task, root, session)?;
    scheduler.tasks[id] = Some(task);
    Ok(())
}
