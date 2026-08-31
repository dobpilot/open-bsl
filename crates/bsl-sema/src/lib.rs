//! Семантический анализ BSL.
//!
//! Крейт связывает имена из AST с локальными слотами, модульными переменными,
//! функциями и встроенными операциями. Результат предназначен для компилятора
//! байт-кода.

mod core_receivers;
mod resolved;
mod resolver;

pub use resolved::{
    LabelId, RExpr, RStmt, RStmtKind, Resolved, ResolvedArg, ResolvedFunction, ResolvedLink,
    ResolvedParam, ResolvedProgram,
};
pub use resolver::{
    ImportedFunction, ImportedModule, ImportedVariable, NEW_TYPES, ResolvedSnippetWithRequirements,
    SemaError, SnippetSignature, resolve_async_snippet_stmts,
    resolve_async_snippet_stmts_with_registry, resolve_program, resolve_program_with_imports,
    resolve_program_with_registry, resolve_repl_stmts, resolve_repl_stmts_with_registry,
    resolve_script, resolve_snippet_stmts, resolve_snippet_stmts_with_registry,
};
