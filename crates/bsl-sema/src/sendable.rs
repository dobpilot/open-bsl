//! Передача семантического дерева без изменения владения числами runtime.

use crate::resolved::ResolvedProgram;

mod portable {
    use crate::resolved::{LabelId, ResolvedLink};
    use bsl_rt::{BuiltinFn, ConstructorCode, FunctionCode, FunctionKind, LibraryKey};
    use bsl_syntax::{BinaryOp, UnaryOp};
    type BslNumber = String;
    include!("resolved_types.rs");
}

/// Владеющий результат анализа, безопасный для передачи между потоками.
/// Числа хранятся в точной канонической записи; изменять её снаружи нельзя.
pub struct SendableResolvedProgram(portable::ResolvedProgram);

impl ResolvedProgram {
    /// Переносит дерево в форму без ссылок на локальные счётчики `Rc`.
    #[must_use]
    pub fn into_sendable(self) -> SendableResolvedProgram {
        SendableResolvedProgram(freeze::resolved_program(self))
    }
}

impl SendableResolvedProgram {
    /// Восстанавливает исходное дерево и точные числовые значения в текущем потоке.
    #[must_use]
    pub fn into_resolved(self) -> ResolvedProgram {
        thaw::resolved_program(self.0)
    }
}

fn freeze_number(number: bsl_number::BslNumber) -> String {
    number.to_canonical()
}
fn thaw_number(number: String) -> bsl_number::BslNumber {
    // Строка получена только из `BslNumber`: внешнего конструктора или
    // доступа к изменению переносимого представления нет.
    bsl_number::BslNumber::parse_canonical(&number)
        .expect("каноническая запись числа должна восстанавливаться")
}

// Оба дерева используют одни объявления узлов. Исчерпывающие преобразования
// заставляют обработать новый вариант или поле в обе стороны при сборке.
macro_rules! conversion {
    ($module:ident, $source:path, $target:path, $number:path) => {
        mod $module {
            use $source as source;
            use $target as target;
            pub(super) fn r_expr(value: source::RExpr) -> target::RExpr {
                match value {
                    source::RExpr::Number(value0) => target::RExpr::Number(($number)(value0)),
                    source::RExpr::Date(value0) => target::RExpr::Date(value0),
                    source::RExpr::Bool(value0) => target::RExpr::Bool(value0),
                    source::RExpr::Undefined => target::RExpr::Undefined,
                    source::RExpr::Null => target::RExpr::Null,
                    source::RExpr::Await(value0) => target::RExpr::Await(Box::new(r_expr(*value0))),
                    source::RExpr::Local(value0) => target::RExpr::Local(value0),
                    source::RExpr::ModuleVar(value0) => target::RExpr::ModuleVar(value0),
                    source::RExpr::Unary { op, expr } => target::RExpr::Unary {
                        op,
                        expr: Box::new(r_expr(*expr)),
                    },
                    source::RExpr::Binary { op, lhs, rhs } => target::RExpr::Binary {
                        op,
                        lhs: Box::new(r_expr(*lhs)),
                        rhs: Box::new(r_expr(*rhs)),
                    },
                    source::RExpr::Call { func, args } => target::RExpr::Call {
                        func,
                        args: args.into_iter().map(resolved_arg).collect(),
                    },
                    source::RExpr::CallImported {
                        link,
                        param_by_val,
                        args,
                    } => target::RExpr::CallImported {
                        link,
                        param_by_val,
                        args: args.into_iter().map(resolved_arg).collect(),
                    },
                    source::RExpr::ImportedVar(value0) => target::RExpr::ImportedVar(value0),
                    source::RExpr::ImportedModule(value0) => target::RExpr::ImportedModule(value0),
                    source::RExpr::CallBuiltinFn { builtin, args } => {
                        target::RExpr::CallBuiltinFn {
                            builtin,
                            args: args.into_iter().map(r_expr).collect(),
                        }
                    }
                    source::RExpr::CallComponent {
                        library,
                        function,
                        kind,
                        args,
                    } => target::RExpr::CallComponent {
                        library,
                        function,
                        kind,
                        args: args.into_iter().map(r_expr).collect(),
                    },
                    source::RExpr::CallMethod {
                        obj,
                        method,
                        open,
                        component_type,
                        component_link,
                        args,
                    } => target::RExpr::CallMethod {
                        obj: Box::new(r_expr(*obj)),
                        method,
                        open,
                        component_type,
                        component_link,
                        args: args.into_iter().map(resolved_arg).collect(),
                    },
                    source::RExpr::Str(value0) => target::RExpr::Str(value0),
                    source::RExpr::Index { obj, index } => target::RExpr::Index {
                        obj: Box::new(r_expr(*obj)),
                        index: Box::new(r_expr(*index)),
                    },
                    source::RExpr::Field { obj, name } => target::RExpr::Field {
                        obj: Box::new(r_expr(*obj)),
                        name,
                    },
                    source::RExpr::NewArray { dims } => target::RExpr::NewArray {
                        dims: dims.into_iter().map(r_expr).collect(),
                    },
                    source::RExpr::NewStructure { keys, values } => target::RExpr::NewStructure {
                        keys,
                        values: values.into_iter().map(r_expr).collect(),
                    },
                    source::RExpr::NewDynamicStructure { args } => {
                        target::RExpr::NewDynamicStructure {
                            args: args.into_iter().map(r_expr).collect(),
                        }
                    }
                    source::RExpr::CreateObject {
                        library,
                        constructor,
                        args,
                    } => target::RExpr::CreateObject {
                        library,
                        constructor,
                        args: args.into_iter().map(r_expr).collect(),
                    },
                    source::RExpr::CreateDynamicObject { type_expr, params } => {
                        target::RExpr::CreateDynamicObject {
                            type_expr: Box::new(r_expr(*type_expr)),
                            params: params.map(|value| Box::new(r_expr(*value))),
                        }
                    }
                    source::RExpr::NewTable => target::RExpr::NewTable,
                    source::RExpr::NewTypeDescription(value0) => {
                        target::RExpr::NewTypeDescription(Box::new(r_expr(*value0)))
                    }
                    source::RExpr::NewValueComparison => target::RExpr::NewValueComparison,
                    source::RExpr::NewMap(value0) => {
                        target::RExpr::NewMap(value0.map(|value| Box::new(r_expr(*value))))
                    }
                    source::RExpr::EnumMember(value0) => target::RExpr::EnumMember(value0),
                    source::RExpr::EnumTypeRef(value0) => target::RExpr::EnumTypeRef(value0),
                    source::RExpr::Ternary {
                        cond,
                        then_expr,
                        else_expr,
                    } => target::RExpr::Ternary {
                        cond: Box::new(r_expr(*cond)),
                        then_expr: Box::new(r_expr(*then_expr)),
                        else_expr: Box::new(r_expr(*else_expr)),
                    },
                    source::RExpr::NewTextWriter { path } => target::RExpr::NewTextWriter {
                        path: Box::new(r_expr(*path)),
                    },
                    source::RExpr::DynEval(value0) => {
                        target::RExpr::DynEval(Box::new(r_expr(*value0)))
                    }
                }
            }
            pub(super) fn resolved_arg(value: source::ResolvedArg) -> target::ResolvedArg {
                match value {
                    source::ResolvedArg::Value(value0) => {
                        target::ResolvedArg::Value(r_expr(value0))
                    }
                    source::ResolvedArg::Default => target::ResolvedArg::Default,
                }
            }
            pub(super) fn r_stmt(value: source::RStmt) -> target::RStmt {
                target::RStmt {
                    kind: r_stmt_kind(value.kind),
                    line: value.line,
                }
            }
            pub(super) fn r_stmt_kind(value: source::RStmtKind) -> target::RStmtKind {
                match value {
                    source::RStmtKind::AssignLocal { slot, value } => {
                        target::RStmtKind::AssignLocal {
                            slot,
                            value: r_expr(value),
                        }
                    }
                    source::RStmtKind::AssignModuleVar { slot, value } => {
                        target::RStmtKind::AssignModuleVar {
                            slot,
                            value: r_expr(value),
                        }
                    }
                    source::RStmtKind::AssignImportedVar { link, value } => {
                        target::RStmtKind::AssignImportedVar {
                            link,
                            value: r_expr(value),
                        }
                    }
                    source::RStmtKind::AssignIndex { obj, index, value } => {
                        target::RStmtKind::AssignIndex {
                            obj: r_expr(obj),
                            index: r_expr(index),
                            value: r_expr(value),
                        }
                    }
                    source::RStmtKind::AssignField { obj, name, value } => {
                        target::RStmtKind::AssignField {
                            obj: r_expr(obj),
                            name,
                            value: r_expr(value),
                        }
                    }
                    source::RStmtKind::ExprStmt(value0) => {
                        target::RStmtKind::ExprStmt(r_expr(value0))
                    }
                    source::RStmtKind::If {
                        cond,
                        then_branch,
                        elsif_branches,
                        else_branch,
                    } => target::RStmtKind::If {
                        cond: r_expr(cond),
                        then_branch: then_branch.into_iter().map(r_stmt).collect(),
                        elsif_branches: elsif_branches
                            .into_iter()
                            .map(|value| {
                                let (part0, part1) = value;
                                (r_expr(part0), part1.into_iter().map(r_stmt).collect())
                            })
                            .collect(),
                        else_branch: else_branch
                            .map(|value| value.into_iter().map(r_stmt).collect()),
                    },
                    source::RStmtKind::While { cond, body } => target::RStmtKind::While {
                        cond: r_expr(cond),
                        body: body.into_iter().map(r_stmt).collect(),
                    },
                    source::RStmtKind::ForNumeric {
                        slot,
                        from,
                        to,
                        body,
                    } => target::RStmtKind::ForNumeric {
                        slot,
                        from: r_expr(from),
                        to: r_expr(to),
                        body: body.into_iter().map(r_stmt).collect(),
                    },
                    source::RStmtKind::ForEach { slot, iter, body } => target::RStmtKind::ForEach {
                        slot,
                        iter: r_expr(iter),
                        body: body.into_iter().map(r_stmt).collect(),
                    },
                    source::RStmtKind::Break => target::RStmtKind::Break,
                    source::RStmtKind::Continue => target::RStmtKind::Continue,
                    source::RStmtKind::Label(value0) => target::RStmtKind::Label(value0),
                    source::RStmtKind::Goto(value0) => target::RStmtKind::Goto(value0),
                    source::RStmtKind::Return(value0) => {
                        target::RStmtKind::Return(value0.map(r_expr))
                    }
                    source::RStmtKind::Try { body, except_body } => target::RStmtKind::Try {
                        body: body.into_iter().map(r_stmt).collect(),
                        except_body: except_body.into_iter().map(r_stmt).collect(),
                    },
                    source::RStmtKind::Raise(value0) => {
                        target::RStmtKind::Raise(value0.map(r_expr))
                    }
                    source::RStmtKind::RaiseDetailed(value0) => {
                        target::RStmtKind::RaiseDetailed(value0.into_iter().map(r_expr).collect())
                    }
                    source::RStmtKind::RaiseCompilationError(value0) => {
                        target::RStmtKind::RaiseCompilationError(value0)
                    }
                    source::RStmtKind::Execute(value0) => {
                        target::RStmtKind::Execute(r_expr(value0))
                    }
                }
            }
            pub(super) fn resolved_param(value: source::ResolvedParam) -> target::ResolvedParam {
                target::ResolvedParam {
                    by_val: value.by_val,
                    default: value.default.map(r_expr),
                    line: value.line,
                }
            }
            pub(super) fn resolved_function(
                value: source::ResolvedFunction,
            ) -> target::ResolvedFunction {
                target::ResolvedFunction {
                    name: value.name,
                    is_async: value.is_async,
                    params: value.params.into_iter().map(resolved_param).collect(),
                    locals: value.locals,
                    body: value.body.into_iter().map(r_stmt).collect(),
                    is_procedure: value.is_procedure,
                    uses_dynamic: value.uses_dynamic,
                    export: value.export,
                }
            }
            pub(super) fn resolved(value: source::Resolved) -> target::Resolved {
                target::Resolved {
                    locals: value.locals,
                    body: value.body.into_iter().map(r_stmt).collect(),
                    uses_dynamic: value.uses_dynamic,
                }
            }
            pub(super) fn resolved_program(
                value: source::ResolvedProgram,
            ) -> target::ResolvedProgram {
                target::ResolvedProgram {
                    requirements: value.requirements,
                    functions: value.functions.into_iter().map(resolved_function).collect(),
                    top_level: resolved(value.top_level),
                    module_vars: value.module_vars,
                    module_var_exports: value.module_var_exports,
                    links: value.links,
                    deferred_module_dependencies: value.deferred_module_dependencies,
                    imports: value.imports,
                }
            }
        }
    };
}
conversion!(
    freeze,
    crate::resolved,
    super::portable,
    super::freeze_number
);
conversion!(thaw, super::portable, crate::resolved, super::thaw_number);

#[cfg(test)]
#[path = "sendable_tests.rs"]
mod tests;
