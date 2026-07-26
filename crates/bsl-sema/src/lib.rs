mod resolved;
mod resolver;

pub use resolved::{RExpr, RStmt, Resolved, ResolvedFunction, ResolvedParam, ResolvedProgram};
pub use resolver::{resolve_program, resolve_script, resolve_snippet_stmts, SemaError};
