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
    ComponentModuleAccess, ImportedFunction, ImportedModule, ImportedModuleProvider,
    ImportedVariable, NEW_TYPES, ResolveOptions, ResolvedImportedSnippet, SemaError,
    SnippetOptions, SnippetSignature, resolve_program, resolve_program_with, resolve_script,
    resolve_snippet_with,
};
