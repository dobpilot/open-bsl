//! Публичные пути VM не зависят от размещения её внутренних модулей.

use bsl_bytecode::Program;
use bsl_rt::{BslValue, RtError};
use bsl_vm::{
    CatalogContext, DebugAction, DebugHook, DebugPosition, DebugValues, ExecutionWaker,
    ProgramExecution, ProgramPoll, ROOT_MODULE, SchedulerConfig, SessionModules,
};

struct Inspector;

impl DebugValues for Inspector {
    fn locals(&mut self, _index: usize) -> Vec<(String, BslValue)> {
        Vec::new()
    }

    fn evaluate(&mut self, _index: usize, _source: &str) -> Result<BslValue, String> {
        Ok(BslValue::Undefined)
    }

    fn line_of(&mut self, _index: usize) -> Option<u32> {
        None
    }
}

impl DebugHook for Inspector {
    fn before_instruction(&mut self, at: &mut DebugPosition<'_>) -> DebugAction {
        assert!(at.frames.is_empty());
        assert_eq!(at.line, None);
        assert!(at.values.locals(0).is_empty());
        DebugAction::Continue
    }
}

#[test]
fn debug_interfaces_keep_their_root_paths_and_signatures() {
    let mut values = Inspector;
    let mut hook = Inspector;
    let mut position = DebugPosition {
        frames: &[],
        line: None,
        values: &mut values,
    };
    assert!(matches!(
        hook.before_instruction(&mut position),
        DebugAction::Continue
    ));
    assert!(matches!(DebugAction::Terminate, DebugAction::Terminate));
}

#[test]
fn execution_interfaces_keep_their_root_paths() {
    let _: Option<CatalogContext<'_>> = None;
    let _: Option<SessionModules> = None;
    let _: Option<ProgramExecution> = None;
    let _: Option<ExecutionWaker> = None;
    let _: u32 = ROOT_MODULE;
    let scheduler = SchedulerConfig {
        safe_points_per_quantum: 7,
    };
    assert_eq!(scheduler.safe_points_per_quantum, 7);
    for state in [ProgramPoll::Runnable, ProgramPoll::Waiting] {
        match state {
            ProgramPoll::Complete(_, _) => panic!("проверяется незавершённый прогон"),
            ProgramPoll::Runnable | ProgramPoll::Waiting => {}
        }
    }
    let _: fn(&Program) -> Result<BslValue, RtError> = bsl_vm::run_program;
    let _ = bsl_vm::run_program_with_registry_and_io;
    let _ = bsl_vm::run_repl_chunk_with_registry;
    let _ = bsl_vm::call_module_function;
    let _ = bsl_vm::call_module_function_with_registry_and_io;
}
