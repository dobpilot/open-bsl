use super::{
    ModuleState, ModulesCtx, ROOT_MODULE, bind_module_argument, ensure_imported_arguments_ready,
    load_imported_argument, load_module_argument,
};
use crate::{
    AsyncState, Frame, MAX_CALL_DEPTH, ParamSlot, Task, TaskCompletion, TaskId, push_own_registers,
    reg_load, reg_store,
};
use bsl_bytecode::{ArgMode, Program};
use bsl_rt::{BslValue, RtError};

/// Уже проверенный аргумент. Подготовка не изменяет стек вызывающего:
/// отказ арности или ссылочной цели остаётся на исходной инструкции.
enum Argument {
    Local(usize),
    Private(BslValue, bool),
    Module(u32, usize, BslValue),
}

/// Связывает открытый вызов с живым экземпляром модуля. `Some` возвращает
/// новую async-задачу, которую драйвер запускает до продолжения вызывающего.
#[allow(clippy::too_many_arguments)]
pub(crate) fn call_reference(
    receiver: &BslValue,
    name: &str,
    result_required: bool,
    base: u8,
    modes: &[ArgMode],
    dst: u8,
    program: &Program,
    state: &ModuleState,
    modules: &mut ModulesCtx<'_, '_>,
    frames: &mut Vec<Frame>,
    stack: &mut Vec<BslValue>,
    async_state: &mut AsyncState,
) -> Result<Option<TaskId>, RtError> {
    let frame = frames
        .last()
        .ok_or(RtError::InvalidBytecode("вызов без кадра"))?;
    let current = frame.module;
    let target = if state.owns_reference(receiver) {
        current
    } else if modules
        .root_state
        .as_deref()
        .is_some_and(|state| state.owns_reference(receiver))
    {
        ROOT_MODULE
    } else {
        modules
            .session
            .instances
            .iter()
            .position(|instance| instance.state.owns_reference(receiver))
            .map(|index| index as u32)
            .ok_or_else(|| {
                RtError::DynamicError("экземпляр модуля не принадлежит текущему исполнению".into())
            })?
    };
    let target_program = if target == current {
        program
    } else if target == ROOT_MODULE {
        modules.root_program
    } else {
        modules
            .catalog
            .ok_or(RtError::InvalidBytecode("модульная ссылка без каталога"))?
            .program(target)?
    };
    let code = if target == current {
        frame.code.clone()
    } else {
        None
    };
    let func = target_program
        .function_names
        .iter()
        .zip(&target_program.exported_functions)
        .position(|(candidate, exported)| *exported && bsl_rt::folded_eq(candidate, name))
        .map(|index| index + 1)
        .ok_or_else(|| RtError::UnknownMethod {
            method: name.into(),
            receiver: "МодульBSL",
        })?;
    let callee = &target_program.chunks[func];
    if result_required && callee.is_procedure {
        return Err(RtError::Raised(BslValue::Str(
            "Вызов процедуры объекта как функции".into(),
        )));
    }
    if modes.len() > callee.n_params as usize {
        return Err(RtError::DynamicError(format!(
            "слишком много аргументов метода {name}"
        )));
    }
    for position in 0..callee.n_params as usize {
        if matches!(modes.get(position), None | Some(ArgMode::Default))
            && !callee.param_has_default[position]
        {
            return Err(RtError::DynamicError(format!(
                "не передан обязательный параметр {} метода {name}",
                position + 1
            )));
        }
    }
    if !callee.is_async && frames.len() >= MAX_CALL_DEPTH {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая рекурсия вызовов",
        });
    }
    if ensure_imported_arguments_ready(program, modes, modules, frames, stack)? {
        return Ok(None);
    }
    let frame = frames
        .last()
        .ok_or(RtError::InvalidBytecode("вызов без кадра"))?;
    let mut arguments = Vec::with_capacity(callee.n_params as usize);
    for position in 0..callee.n_params as usize {
        let mode = modes.get(position).copied().unwrap_or(ArgMode::Default);
        let argument = if mode == ArgMode::Default {
            Argument::Private(BslValue::Undefined, false)
        } else if callee.param_by_val[position] || mode == ArgMode::Value {
            Argument::Private(
                reg_load(stack, frame.reg_index(base + position as u8))?,
                true,
            )
        } else {
            match mode {
                ArgMode::ByRefLocal(slot) => Argument::Local(frame.reg_index(slot)),
                ArgMode::ByRefModuleVar(slot) => Argument::Module(
                    current,
                    slot as usize,
                    load_module_argument(current, slot as usize, frames, stack, &state.slots)?,
                ),
                ArgMode::ByRefImportedVar(link) => {
                    let (owner, slot, value) =
                        load_imported_argument(program, link, modules.session, frames, stack)?;
                    Argument::Module(owner, slot, value)
                }
                ArgMode::ByRefIndex { .. } => {
                    return Err(RtError::InvalidBytecode(
                        "индексная ссылочная цель у модульного метода",
                    ));
                }
                ArgMode::Value | ArgMode::Default => unreachable!("режим обработан выше"),
            }
        };
        arguments.push(argument);
    }
    if callee.is_async {
        let mut child_stack = Vec::with_capacity(callee.n_regs as usize);
        let mut aliases = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let (value, provided) = match argument {
                Argument::Local(index) => (reg_load(stack, index)?, true),
                Argument::Private(value, provided) => (value, provided),
                Argument::Module(_, _, value) => (value, true),
            };
            aliases.push(ParamSlot {
                idx: child_stack.len(),
                provided,
            });
            child_stack.push(value);
        }
        push_own_registers(&mut child_stack, callee);
        let (completion, value) = if callee.is_procedure {
            (TaskCompletion::Detached, BslValue::Undefined)
        } else {
            let (id, promise) = async_state.new_promise()?;
            (TaskCompletion::Promise(id), promise)
        };
        reg_store(stack, frame.reg_index(dst), value)?;
        frames.last_mut().unwrap().pc += 1;
        return Ok(Some(async_state.insert_task(Task {
            frames: vec![Frame {
                module: target,
                code,
                func_id: func,
                pc: 0,
                param_aliases: aliases,
                own_base: callee.n_params as usize,
                call_start: 0,
                return_reg: 0,
                module_copybacks: Vec::new(),
                numeric_for_state: None,
            }],
            stack: child_stack,
            current_exception: None,
            completion,
            quantum_remaining: async_state.scheduler_quantum(),
        })));
    }
    // Собственные копии аргументов расположены после стека вызывающего:
    // оптимизатор вправе подставлять в окно живую локаль вместо Move.
    let call_start = stack.len();
    let mut aliases = Vec::with_capacity(arguments.len());
    let mut copybacks = Vec::new();
    for argument in arguments {
        let (idx, provided) = match argument {
            Argument::Local(index) => (index, true),
            Argument::Private(value, provided) => {
                let index = stack.len();
                stack.push(value);
                (index, provided)
            }
            Argument::Module(owner, slot, value) => {
                let binding =
                    bind_module_argument(owner, slot, value, frames, &mut copybacks, stack);
                (binding.idx, binding.provided)
            }
        };
        aliases.push(ParamSlot { idx, provided });
    }
    let own_base = stack.len();
    push_own_registers(stack, callee);
    frames.last_mut().unwrap().pc += 1;
    frames.push(Frame {
        module: target,
        code,
        func_id: func,
        pc: 0,
        param_aliases: aliases,
        own_base,
        call_start,
        return_reg: dst,
        module_copybacks: copybacks,
        numeric_for_state: None,
    });
    Ok(None)
}
