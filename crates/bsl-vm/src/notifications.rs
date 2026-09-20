//! Отложенная доставка через обычные Await и открытые вызовы VM.
//! Файловый поток не владеет BSL-значениями и не исполняет обработчиков.

use super::{AsyncState, Frame, LinkedComponents, PromiseState, Task, TaskCompletion};
use bsl_bytecode::{ArgMode, Chunk, ExceptionRange, Instr, Program};
use bsl_rt::{
    BslValue, FileNotificationOperation, HostPromiseSpawner, NotificationDescription, PromiseId,
    RtError,
};
use std::{cell::RefCell, rc::Rc};

fn method_name(program: &mut Program, name: &bsl_rt::BslString) -> Result<u16, RtError> {
    let name = name.to_string();
    let existing = program
        .names
        .iter()
        .position(|entry| bsl_rt::folded_eq(entry, &name));
    let index = u16::try_from(existing.unwrap_or(program.names.len()))
        .map_err(|_| RtError::InvalidBytecode("имя обработчика не помещается в таблицу"))?;
    if existing.is_none() {
        program.names.push(name);
    }
    Ok(index)
}

/// Образ подготовлен и проверен до регистрации операции. Статические чанки
/// сохраняют номера и ссылки владельца, как у динамического фрагмента.
fn prepare(
    program: &Program,
    linked: &LinkedComponents<'_>,
    module: u32,
    description: &NotificationDescription,
    with_result: bool,
) -> Result<Rc<super::snippet::DynamicImage>, RtError> {
    // Используется обычный открытый вызов VM, без отдельного интерпретатора callback.
    let mut program = program.clone();
    let mut chunk = Chunk::new();
    chunk.n_regs = 9;
    chunk.n_locals = 7;
    chunk.is_async = true;
    chunk.instrs.push(Instr::Await { dst: 4, promise: 0 });
    if let Some((name, _)) = description.handler() {
        let method = method_name(&mut program, name)?;
        chunk.instrs.push(Instr::Move {
            dst: if with_result { 5 } else { 4 },
            src: 3,
        });
        chunk
            .call_arg_modes
            .push(vec![ArgMode::Value; if with_result { 2 } else { 1 }]);
        chunk.instrs.push(Instr::CallObjectProcedure {
            dst: 8,
            obj: 1,
            method,
            base: 4,
            arg_modes: 0,
        });
    }
    chunk.instrs.push(Instr::LoadBool { dst: 7, val: false });
    chunk.instrs.push(Instr::Return { src: Some(7) });
    let handler_pc = chunk.instrs.len();
    chunk.exception_ranges.push(ExceptionRange {
        start_pc: 0,
        end_pc: 1,
        handler_pc,
    });
    chunk.instrs.extend([
        Instr::CallBuiltin {
            dst: 4,
            builtin: bsl_rt::BuiltinFn::ErrorInfo,
            base: 0,
            count: 0,
        },
        Instr::LoadBool { dst: 5, val: true },
        Instr::Move { dst: 6, src: 3 },
    ]);
    if let Some((name, _)) = description.error_handler() {
        let method = method_name(&mut program, name)?;
        let arg_modes = chunk.call_arg_modes.len() as u16;
        chunk
            .call_arg_modes
            .push(vec![ArgMode::Value, ArgMode::ByRefLocal(5), ArgMode::Value]);
        chunk.instrs.push(Instr::CallObjectProcedure {
            dst: 8,
            obj: 2,
            method,
            base: 4,
            arg_modes,
        });
    }
    chunk.instrs.push(Instr::Return { src: Some(5) });
    program.chunks[0] = chunk;
    program.top_level_locals.clear();
    if !program.lines.is_empty() {
        program.lines[0].clear();
    }
    bsl_bytecode::image::finalize_unbundled(&mut program);
    bsl_bytecode::image::verify(&program)?;
    let scope_module = if module == super::ROOT_MODULE {
        linked.scope_module
    } else {
        Some(bsl_bytecode::ModuleId::new(module))
    };
    let tables = super::link_components(
        &program,
        linked.registry,
        linked.zone.clone(),
        linked.files.clone(),
        linked.random.clone(),
        linked.network.clone(),
        linked.background_jobs.clone(),
        linked.temp_storage.clone(),
        linked.message_sink.clone(),
        linked.scope,
    )?
    .with_scope_module(scope_module);
    Ok(Rc::new(super::snippet::DynamicImage {
        shapes: RefCell::new(super::drive_prologue(&program, &tables)),
        caches: super::RunCaches::for_program(&program),
        tables: tables.tables,
        program,
    }))
}

pub(super) fn register(
    state: &mut AsyncState,
    program: &Program,
    linked: &LinkedComponents<'_>,
    module: u32,
    operation: FileNotificationOperation,
    description: NotificationDescription,
    with_result: bool,
) -> Result<(), RtError> {
    let code = prepare(program, linked, module, &description, with_result)?;
    let promise = match operation {
        FileNotificationOperation::Ready(result) => state.ready_file_promise(result)?,
        FileNotificationOperation::Pending {
            request,
            files,
            zone,
        } => state.spawn_file_operation(request, files, zone)?,
    };
    let (_, promise_id) = promise.promise_identity().ok_or(RtError::InvalidBytecode(
        "регистрация оповещения не вернула обещание",
    ))?;
    let mut stack = vec![BslValue::Undefined; 9];
    stack[0] = promise;
    stack[1] = description
        .handler()
        .map_or(BslValue::Undefined, |(_, receiver)| receiver.clone());
    stack[2] = description
        .error_handler()
        .map_or(BslValue::Undefined, |(_, receiver)| receiver.clone());
    stack[3] = description.data().clone();
    let task = Task {
        frames: vec![Frame {
            module,
            code: Some(code),
            func_id: 0,
            pc: 0,
            param_aliases: Vec::new(),
            own_base: 0,
            call_start: 0,
            return_reg: 0,
            module_copybacks: Vec::new(),
            numeric_for_state: None,
        }],
        stack,
        current_exception: None,
        completion: TaskCompletion::Notification(promise_id),
        quantum_remaining: 0,
    };
    let id = state.insert_task(task);
    state
        .ready
        .push_back(super::scheduler::ReadyEvent::Task(id));
    Ok(())
}

pub(super) fn finish(
    state: &AsyncState,
    promise: PromiseId,
    standard: BslValue,
) -> Result<(), RtError> {
    // file-begin-standard-flags: 0 и "False" также подавляют ошибку.
    // Неприводимое значение не заменяет исходную ошибку ошибкой преобразования.
    if matches!(standard.as_condition(), Ok(false)) {
        return Ok(());
    }
    // Возвращается исходная ошибка, а не current_exception: обработчик
    // мог перехватить собственное исключение и заменить текущую информацию.
    match state.promises.get(promise.get() as usize) {
        Some(PromiseState::Ready(Err(error))) => Err(error.clone()),
        _ => Err(RtError::InvalidBytecode(
            "оповещение завершилось без исходной ошибки",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconvertible_standard_flag_keeps_the_original_io_error() {
        let mut state = AsyncState::new(
            Task {
                frames: Vec::new(),
                stack: Vec::new(),
                current_exception: None,
                completion: TaskCompletion::Root,
                quantum_remaining: 0,
            },
            1,
        );
        let (id, _) = state.new_promise().unwrap();
        let error = RtError::IoError("исходный отказ host".into());
        state.resolve_promise(id, Err(error.clone())).unwrap();
        for value in [
            BslValue::Undefined,
            BslValue::Null,
            BslValue::new_array(Vec::new()),
            BslValue::Str("".into()),
        ] {
            assert_eq!(finish(&state, id, value), Err(error.clone()));
        }
    }
}
