use std::collections::HashMap;

use bsl_number::BslNumber;
use bsl_syntax::{Expr as AExpr, Stmt as AStmt};

use crate::resolved::{RExpr, RStmt, Resolved};

#[derive(Debug, Clone, PartialEq)]
pub enum SemaError {
    /// Идентификатор читается раньше первого присваивания/объявления.
    UndefinedVariable(String),
    /// Конструкция языка, для которой ещё нет резолвинга (процедуры/функции,
    /// коллекции, `Выполнить`/`Вычислить`, ... — приходят в последующих
    /// milestone'ах).
    Unsupported(&'static str),
}

/// Резолвит плоский скрипт верхнего уровня (без объявлений процедур/функций
/// — им нужны отдельные кадры, это M4). Реализует неявное объявление
/// переменных: `PI = 3.14` без `Перем` объявляет `PI` в момент первого
/// присваивания; чтение необъявленной переменной — ошибка.
pub fn resolve_script(stmts: &[AStmt]) -> Result<Resolved, SemaError> {
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
    };
    let body = r.resolve_block(stmts)?;
    Ok(Resolved {
        locals: r.locals,
        body,
    })
}

struct Resolver {
    locals: Vec<String>,
    /// Ключ — имя в верхнем регистре: доступ к переменным регистронезависим.
    index: HashMap<String, u32>,
}

impl Resolver {
    fn declare(&mut self, name: &str) -> u32 {
        let key = name.to_uppercase();
        if let Some(&slot) = self.index.get(&key) {
            return slot;
        }
        let slot = self.locals.len() as u32;
        self.locals.push(name.to_string());
        self.index.insert(key, slot);
        slot
    }

    fn lookup(&self, name: &str) -> Option<u32> {
        self.index.get(&name.to_uppercase()).copied()
    }

    fn resolve_block(&mut self, stmts: &[AStmt]) -> Result<Vec<RStmt>, SemaError> {
        let mut out = Vec::new();
        for s in stmts {
            if let Some(rs) = self.resolve_stmt(s)? {
                out.push(rs);
            }
        }
        Ok(out)
    }

    fn resolve_stmt(&mut self, s: &AStmt) -> Result<Option<RStmt>, SemaError> {
        match s {
            AStmt::Assign { target, value } => match target {
                AExpr::Ident(name) => {
                    let slot = self.declare(name);
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::Assign { slot, value }))
                }
                _ => Err(SemaError::Unsupported(
                    "присваивание в путь (индекс/поле) появится вместе с коллекциями (M5)",
                )),
            },
            AStmt::ExprStmt(_) => Err(SemaError::Unsupported(
                "вызовы процедур/функций появятся вместе с кадрами (M4)",
            )),
            AStmt::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let cond = self.resolve_expr(cond)?;
                let then_branch = self.resolve_block(then_branch)?;
                let mut elsifs = Vec::new();
                for (c, b) in elsif_branches {
                    elsifs.push((self.resolve_expr(c)?, self.resolve_block(b)?));
                }
                let else_branch = match else_branch {
                    Some(b) => Some(self.resolve_block(b)?),
                    None => None,
                };
                Ok(Some(RStmt::If {
                    cond,
                    then_branch,
                    elsif_branches: elsifs,
                    else_branch,
                }))
            }
            AStmt::While { cond, body } => {
                let cond = self.resolve_expr(cond)?;
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::While { cond, body }))
            }
            AStmt::ForNumeric {
                var,
                from,
                to,
                body,
            } => {
                let from = self.resolve_expr(from)?;
                let to = self.resolve_expr(to)?;
                // Переменная цикла объявляется до тела: тело может ссылаться
                // на неё, а сама она остаётся живой и после `КонецЦикла`.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForNumeric {
                    slot,
                    from,
                    to,
                    body,
                }))
            }
            AStmt::ForEach { .. } => Err(SemaError::Unsupported(
                "Для Каждого появится вместе с коллекциями (M5)",
            )),
            AStmt::Break => Ok(Some(RStmt::Break)),
            AStmt::Continue => Ok(Some(RStmt::Continue)),
            AStmt::Return(_) => Err(SemaError::Unsupported(
                "Возврат появится вместе с кадрами процедур/функций (M4)",
            )),
            AStmt::Try { .. } => Err(SemaError::Unsupported(
                "Попытка появится вместе с таблицей защищённых диапазонов (M6)",
            )),
            AStmt::Raise(_) => Err(SemaError::Unsupported(
                "ВызватьИсключение появится вместе с исключениями (M6)",
            )),
            AStmt::VarDecl(vd) => {
                // Только регистрирует слоты; значение по умолчанию —
                // Неопределено, для этого не нужна ни одна инструкция VM
                // (регистры кадра и так инициализируются Неопределено).
                for name in &vd.names {
                    self.declare(name);
                }
                Ok(None)
            }
            AStmt::Execute(_) => Err(SemaError::Unsupported(
                "Выполнить появится вместе с компиляцией строк в рантайме (M9)",
            )),
        }
    }

    fn resolve_expr(&mut self, e: &AExpr) -> Result<RExpr, SemaError> {
        match e {
            AExpr::Number(text) => {
                let n = BslNumber::parse_canonical(text).unwrap_or_else(|err| {
                    panic!("лексер пропустил некорректный числовой литерал {text:?}: {err}")
                });
                Ok(RExpr::Number(n))
            }
            AExpr::Bool(b) => Ok(RExpr::Bool(*b)),
            AExpr::Undefined => Ok(RExpr::Undefined),
            AExpr::Null => Ok(RExpr::Null),
            AExpr::Ident(name) => match self.lookup(name) {
                Some(slot) => Ok(RExpr::Local(slot)),
                None => Err(SemaError::UndefinedVariable(name.clone())),
            },
            AExpr::Unary { op, expr } => Ok(RExpr::Unary {
                op: *op,
                expr: Box::new(self.resolve_expr(expr)?),
            }),
            AExpr::Binary { op, lhs, rhs } => Ok(RExpr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_expr(lhs)?),
                rhs: Box::new(self.resolve_expr(rhs)?),
            }),
            AExpr::Str(_) => Err(SemaError::Unsupported(
                "строки появятся вместе со строковым рантаймом (M4/bsl-rt)",
            )),
            AExpr::Date(_) => Err(SemaError::Unsupported("даты появятся позже")),
            AExpr::Call { .. } => Err(SemaError::Unsupported(
                "вызовы появятся вместе с кадрами (M4)",
            )),
            AExpr::Index { .. } => Err(SemaError::Unsupported(
                "индексация появится вместе с коллекциями (M5)",
            )),
            AExpr::Field { .. } => Err(SemaError::Unsupported(
                "доступ к полям появится вместе со структурами (M5)",
            )),
            AExpr::New { .. } => Err(SemaError::Unsupported(
                "Новый появится вместе с коллекциями (M5)",
            )),
            AExpr::Ternary { .. } => Err(SemaError::Unsupported(
                "тернарный ?() появится вместе с кадрами (M4)",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_syntax::parse;

    fn resolve_src(src: &str) -> Resolved {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        let stmts = items_to_stmts(prog.items);
        resolve_script(&stmts).unwrap_or_else(|e| panic!("sema error: {e:?}"))
    }

    /// В тестовых скриптах верхнего уровня допускаем только `Перем` и
    /// обычные операторы — объявления процедур/функций сюда не проверяем
    /// (для них нужны кадры, это M4).
    fn items_to_stmts(items: Vec<bsl_syntax::Item>) -> Vec<AStmt> {
        items
            .into_iter()
            .map(|item| match item {
                bsl_syntax::Item::Stmt(s) => s,
                bsl_syntax::Item::VarDecl(vd) => AStmt::VarDecl(vd),
                other => panic!("expected only statements/Перем in test script, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn implicit_declaration_on_first_assignment() {
        let r = resolve_src("PI = 3.14;");
        assert_eq!(r.locals, vec!["PI".to_string()]);
        assert_eq!(
            r.body,
            vec![RStmt::Assign {
                slot: 0,
                value: RExpr::Number(BslNumber::parse_canonical("3.14").unwrap()),
            }]
        );
    }

    #[test]
    fn reading_undefined_variable_is_an_error() {
        let prog = parse("y = x;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let err = resolve_script(&stmts).unwrap_err();
        assert_eq!(err, SemaError::UndefinedVariable("x".to_string()));
    }

    #[test]
    fn case_insensitive_identifier_is_the_same_variable() {
        let r = resolve_src("x = 1;\nX = x + 1;");
        assert_eq!(r.locals, vec!["x".to_string()]);
        assert_eq!(
            r.body[1],
            RStmt::Assign {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
    }

    #[test]
    fn for_loop_variable_is_declared_and_alive_in_body() {
        let r = resolve_src("Для i = 0 По 10 Цикл\ny = i;\nКонецЦикла");
        assert_eq!(r.locals, vec!["i".to_string(), "y".to_string()]);
    }

    #[test]
    fn var_decl_registers_slot_without_runtime_effect() {
        let r = resolve_src("Перем a, b;\na = 1;");
        assert_eq!(r.locals, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(r.body.len(), 1);
    }

    #[test]
    fn unsupported_constructs_report_clearly() {
        let prog = parse("Ф();").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::Unsupported(_)
        ));
    }
}
