//! Семантический анализ BSL.
//!
//! Крейт связывает имена из AST с локальными слотами, модульными переменными,
//! функциями и встроенными операциями. Результат предназначен для компилятора
//! байт-кода.

mod core_receivers;
mod resolved;
mod resolver;

pub use resolved::{
    RExpr, RStmt, Resolved, ResolvedArg, ResolvedFunction, ResolvedParam, ResolvedProgram,
};
pub use resolver::{
    NEW_TYPES, ResolvedSnippetWithRequirements, SemaError, SnippetSignature, resolve_program,
    resolve_program_with_registry, resolve_repl_stmts, resolve_repl_stmts_with_registry,
    resolve_script, resolve_snippet_stmts, resolve_snippet_stmts_with_registry,
};
