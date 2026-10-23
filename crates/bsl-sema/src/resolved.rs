use bsl_number::BslNumber;
use bsl_rt::{BuiltinFn, ConstructorCode, FunctionCode, FunctionKind, LibraryKey};
use bsl_syntax::{BinaryOp, UnaryOp};

/// Разрешённая связь с экспортным символом ЧУЖОГО модуля конфигурации.
/// Номер записи в `ResolvedProgram::links` — будущий `LinkSlot` таблицы
/// `Program::links`; вид символа фиксируется на этапе резолвинга.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedLink {
    /// `func` — индекс ЧАНКА целевой программы (позиция функции + 1).
    Function { module: u32, func: u16 },
    /// `slot` — номер в `module_vars` целевого модуля.
    Variable { module: u32, slot: u16 },
    /// Метод конкретного компонентного типа. Позиции дескрипторов в
    /// статических Rust-срезах намеренно не входят в образ.
    ObjectMethod {
        library: LibraryKey,
        type_name: &'static str,
        method: String,
    },
}

/// Номер метки внутри одного тела модуля, процедуры или функции.
/// Имя свёрнуто в этот номер резолвером, поэтому компилятор
/// байт-кода больше не сравнивает строки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(pub u32);

include!("resolved_types.rs");

/// Есть ли в теле `Выполнить`/`Вычислить` — обход по разрешённому дереву,
/// а не по AST: `Вычислить` к этому моменту уже стал `RExpr::DynEval`, а
/// `Выполнить` — `RStmtKind::Execute`, и различать их по имени функции больше
/// не нужно.
///
/// Ищет на ЛЮБОЙ глубине вложенности: `Выполнить` внутри `Если` внутри
/// цикла — такой же повод материализовать имена, как и на верхнем уровне
/// тела.
pub fn block_uses_dynamic(body: &[RStmt]) -> bool {
    body.iter().any(stmt_uses_dynamic)
}

/// Содержит ли динамический код хотя бы одно ВЫРАЖЕНИЕ УМОЛЧАНИЯ параметра.
/// Умолчания компилируются в тот же чанк прологом, поэтому
/// `Ф(а = Вычислить("..."))` обязан пометить чанк как использующий
/// динамику, даже если ТЕЛО функции его не содержит, — иначе `local_names`
/// не материализуются и вложенный фрагмент не увидит область видимости.
pub fn params_use_dynamic(params: &[ResolvedParam]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_uses_dynamic))
}

fn stmt_uses_dynamic(s: &RStmt) -> bool {
    match &s.kind {
        RStmtKind::Execute(_) => true,
        RStmtKind::AssignLocal { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignModuleVar { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignImportedVar { value, .. } => expr_uses_dynamic(value),
        RStmtKind::AssignIndex { obj, index, value } => {
            expr_uses_dynamic(obj) || expr_uses_dynamic(index) || expr_uses_dynamic(value)
        }
        RStmtKind::AssignField { obj, value, .. } => {
            expr_uses_dynamic(obj) || expr_uses_dynamic(value)
        }
        RStmtKind::ExprStmt(e) => expr_uses_dynamic(e),
        RStmtKind::If {
            cond,
            then_branch,
            elsif_branches,
            else_branch,
        } => {
            expr_uses_dynamic(cond)
                || block_uses_dynamic(then_branch)
                || elsif_branches
                    .iter()
                    .any(|(c, b)| expr_uses_dynamic(c) || block_uses_dynamic(b))
                || else_branch.as_deref().is_some_and(block_uses_dynamic)
        }
        RStmtKind::While { cond, body } => expr_uses_dynamic(cond) || block_uses_dynamic(body),
        RStmtKind::ForNumeric { from, to, body, .. } => {
            expr_uses_dynamic(from) || expr_uses_dynamic(to) || block_uses_dynamic(body)
        }
        RStmtKind::ForEach { iter, body, .. } => {
            expr_uses_dynamic(iter) || block_uses_dynamic(body)
        }
        RStmtKind::Return(e) | RStmtKind::Raise(e) => e.as_ref().is_some_and(expr_uses_dynamic),
        RStmtKind::RaiseDetailed(arguments) => arguments.iter().any(expr_uses_dynamic),
        RStmtKind::RaiseCompilationError(_) => false,
        RStmtKind::Try { body, except_body } => {
            block_uses_dynamic(body) || block_uses_dynamic(except_body)
        }
        RStmtKind::Break | RStmtKind::Continue | RStmtKind::Label(_) | RStmtKind::Goto(_) => false,
    }
}

fn expr_uses_dynamic(e: &RExpr) -> bool {
    match e {
        RExpr::DynEval(_) => true,
        RExpr::Await(expr) => expr_uses_dynamic(expr),
        RExpr::ModuleVar(_) => false,
        RExpr::Unary { expr, .. } => expr_uses_dynamic(expr),
        RExpr::Binary { lhs, rhs, .. } => expr_uses_dynamic(lhs) || expr_uses_dynamic(rhs),
        RExpr::Call { args, .. } | RExpr::CallImported { args, .. } => {
            args.iter().any(|a| match a {
                ResolvedArg::Value(e) => expr_uses_dynamic(e),
                ResolvedArg::Default => false,
            })
        }
        RExpr::ImportedVar(_) | RExpr::ImportedModule(_) => false,
        RExpr::CallBuiltinFn { args, .. } | RExpr::CallComponent { args, .. } => {
            args.iter().any(expr_uses_dynamic)
        }
        RExpr::CallMethod { obj, args, .. } => {
            expr_uses_dynamic(obj)
                || args.iter().any(|arg| match arg {
                    ResolvedArg::Value(expr) => expr_uses_dynamic(expr),
                    ResolvedArg::Default => false,
                })
        }
        RExpr::Index { obj, index } => expr_uses_dynamic(obj) || expr_uses_dynamic(index),
        RExpr::Field { obj, .. } => expr_uses_dynamic(obj),
        RExpr::NewArray { dims } => dims.iter().any(expr_uses_dynamic),
        RExpr::NewStructure { values, .. } => values.iter().any(expr_uses_dynamic),
        RExpr::NewDynamicStructure { args } => args.iter().any(expr_uses_dynamic),
        RExpr::CreateObject { args, .. } => args.iter().any(expr_uses_dynamic),
        RExpr::CreateDynamicObject { type_expr, params } => {
            expr_uses_dynamic(type_expr) || params.as_deref().is_some_and(expr_uses_dynamic)
        }
        RExpr::EnumMember(_) | RExpr::EnumTypeRef(_) => false,
        RExpr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_dynamic(cond) || expr_uses_dynamic(then_expr) || expr_uses_dynamic(else_expr)
        }
        RExpr::NewTextWriter { path } => expr_uses_dynamic(path),
        RExpr::NewTypeDescription(names) => expr_uses_dynamic(names),
        RExpr::Number(_)
        | RExpr::Date(_)
        | RExpr::Bool(_)
        | RExpr::Undefined
        | RExpr::Null
        | RExpr::Local(_)
        | RExpr::Str(_)
        | RExpr::NewTable
        | RExpr::NewValueComparison
        | RExpr::NewMap(None) => false,
        RExpr::NewMap(Some(source)) => expr_uses_dynamic(source),
    }
}
