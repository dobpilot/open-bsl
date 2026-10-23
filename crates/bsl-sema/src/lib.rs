//! Семантический анализ BSL.
//!
//! Крейт связывает имена из AST с локальными слотами, модульными переменными,
//! функциями и встроенными операциями. Результат предназначен для компилятора
//! байт-кода.

mod core_receivers;
mod resolved;
mod resolver;
mod sendable;

pub use sendable::SendableResolvedProgram;

pub use resolved::{
    LabelId, RExpr, RStmt, RStmtKind, Resolved, ResolvedArg, ResolvedFunction, ResolvedLink,
    ResolvedParam, ResolvedProgram,
};
pub use resolver::{
    ComponentModuleAccess, ImportedFunction, ImportedModule, ImportedModuleProvider,
    ImportedVariable, NEW_TYPES, ResolvedImportedSnippet, ResolvedSnippetWithRequirements,
    SemaError, SnippetSignature, resolve_async_snippet_stmts,
    resolve_async_snippet_stmts_with_registry, resolve_configuration_module_with_imports,
    resolve_configuration_module_with_name_error_policy,
    resolve_configuration_module_with_provider,
    resolve_configuration_module_with_receiver_property_names, resolve_program,
    resolve_program_with_imports, resolve_program_with_provider, resolve_program_with_registry,
    resolve_repl_stmts, resolve_repl_stmts_with_registry, resolve_script, resolve_snippet_stmts,
    resolve_snippet_stmts_with_imports, resolve_snippet_stmts_with_provider,
    resolve_snippet_stmts_with_receiver_property_names, resolve_snippet_stmts_with_registry,
};
