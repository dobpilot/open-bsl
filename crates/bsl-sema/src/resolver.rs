use std::collections::HashMap;

use bsl_number::BslNumber;
use bsl_syntax::{Expr as AExpr, Item, Stmt as AStmt};

use crate::resolved::{RExpr, RStmt, Resolved, ResolvedFunction, ResolvedParam, ResolvedProgram};

#[derive(Debug, Clone, PartialEq)]
pub enum SemaError {
    /// Идентификатор читается раньше первого присваивания/объявления.
    UndefinedVariable(String),
    /// Вызов имени, для которого нет объявленной `Процедура`/`Функция` в
    /// этом модуле (методов объектов и встроенных функций пока нет).
    UndefinedFunction(String),
    DuplicateFunction(String),
    ArgumentCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// Конструкция языка, для которой ещё нет резолвинга (коллекции,
    /// `Выполнить`/`Вычислить`, значения по умолчанию/пропуски аргументов,
    /// ... — приходят в последующих milestone'ах).
    Unsupported(&'static str),
}

/// Сигнатура функции/процедуры, собранная до резолвинга тел — нужна, чтобы
/// вызовы разрешались независимо от порядка объявления (в том числе
/// рекурсия и взаимные вызовы).
struct FuncSig {
    index: u32,
    by_val: Vec<bool>,
}

/// Резолвит весь модуль: собирает сигнатуры всех `Процедура`/`Функция` за
/// один проход (чтобы вызовы работали независимо от порядка объявления и
/// поддерживали рекурсию), затем резолвит каждое тело и операторы верхнего
/// уровня.
pub fn resolve_program(items: &[Item]) -> Result<ResolvedProgram, SemaError> {
    let mut sigs: HashMap<String, FuncSig> = HashMap::new();
    let mut func_items: Vec<&Item> = Vec::new();
    let mut top_stmts: Vec<AStmt> = Vec::new();

    for item in items {
        match item {
            Item::Function(f) => {
                declare_sig(&mut sigs, &f.name, f.params.iter().map(|p| p.by_val).collect())?;
                func_items.push(item);
            }
            Item::Procedure(p) => {
                declare_sig(&mut sigs, &p.name, p.params.iter().map(|p| p.by_val).collect())?;
                func_items.push(item);
            }
            Item::VarDecl(vd) => top_stmts.push(AStmt::VarDecl(vd.clone())),
            Item::Stmt(s) => top_stmts.push(s.clone()),
        }
    }

    let mut functions = Vec::with_capacity(func_items.len());
    for item in &func_items {
        let (name, params, body) = match item {
            Item::Function(f) => (&f.name, &f.params, &f.body),
            Item::Procedure(p) => (&p.name, &p.params, &p.body),
            _ => unreachable!(),
        };
        let mut r = Resolver {
            locals: Vec::new(),
            index: HashMap::new(),
            funcs: &sigs,
        };
        for p in params {
            r.declare(&p.name);
        }
        let resolved_body = r.resolve_block(body)?;
        functions.push(ResolvedFunction {
            name: name.clone(),
            params: params
                .iter()
                .map(|p| ResolvedParam { by_val: p.by_val })
                .collect(),
            locals: r.locals,
            body: resolved_body,
        });
    }

    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &sigs,
    };
    let top_body = r.resolve_block(&top_stmts)?;
    let top_level = Resolved {
        locals: r.locals,
        body: top_body,
    };

    Ok(ResolvedProgram {
        functions,
        top_level,
    })
}

fn declare_sig(
    sigs: &mut HashMap<String, FuncSig>,
    name: &str,
    by_val: Vec<bool>,
) -> Result<(), SemaError> {
    let key = name.to_uppercase();
    if sigs.contains_key(&key) {
        return Err(SemaError::DuplicateFunction(name.to_string()));
    }
    let index = sigs.len() as u32;
    sigs.insert(key, FuncSig { index, by_val });
    Ok(())
}

/// Резолвит плоский скрипт верхнего уровня без объявлений функций — удобно
/// для тестов и (в будущем) для `Выполнить`, которому объявления процедур
/// недоступны. Функции резолвятся только через [`resolve_program`].
pub fn resolve_script(stmts: &[AStmt]) -> Result<Resolved, SemaError> {
    let empty_funcs = HashMap::new();
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &empty_funcs,
    };
    let body = r.resolve_block(stmts)?;
    Ok(Resolved {
        locals: r.locals,
        body,
    })
}

struct Resolver<'a> {
    locals: Vec<String>,
    /// Ключ — имя в верхнем регистре: доступ к переменным регистронезависим.
    index: HashMap<String, u32>,
    funcs: &'a HashMap<String, FuncSig>,
}

impl<'a> Resolver<'a> {
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
            AStmt::ExprStmt(e) => Ok(Some(RStmt::ExprStmt(self.resolve_expr(e)?))),
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
            AStmt::Return(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Return(r)))
            }
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
            AExpr::Call { callee, args } => self.resolve_call(callee, args),
            AExpr::Str(_) => Err(SemaError::Unsupported(
                "строки появятся вместе со строковым рантаймом (bsl-rt)",
            )),
            AExpr::Date(_) => Err(SemaError::Unsupported("даты появятся позже")),
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
                "тернарный ?() появится позже",
            )),
        }
    }

    fn resolve_call(
        &mut self,
        callee: &AExpr,
        args: &[Option<AExpr>],
    ) -> Result<RExpr, SemaError> {
        let name = match callee {
            AExpr::Ident(n) => n,
            _ => {
                return Err(SemaError::Unsupported(
                    "вызов не по простому имени (методы объектов) появится позже",
                ))
            }
        };
        let sig = self
            .funcs
            .get(&name.to_uppercase())
            .ok_or_else(|| SemaError::UndefinedFunction(name.clone()))?;
        if args.len() != sig.by_val.len() {
            return Err(SemaError::ArgumentCountMismatch {
                name: name.clone(),
                expected: sig.by_val.len(),
                found: args.len(),
            });
        }
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Some(e) => rargs.push(self.resolve_expr(e)?),
                None => {
                    return Err(SemaError::Unsupported(
                        "пропущенные аргументы Ф(1, , 3) появятся вместе со значениями по умолчанию",
                    ))
                }
            }
        }
        Ok(RExpr::Call {
            func: sig.index,
            args: rargs,
        })
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

    fn resolve_program_src(src: &str) -> ResolvedProgram {
        let prog = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
        resolve_program(&prog.items).unwrap_or_else(|e| panic!("sema error: {e:?}"))
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
    fn calling_undeclared_function_is_an_error() {
        let prog = parse("Ф();").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::UndefinedFunction("Ф".to_string())
        );
    }

    #[test]
    fn functions_can_be_called_before_their_declaration() {
        // Main вызывает Helper, объявленную ниже по тексту — должно работать,
        // сигнатуры собираются за отдельный проход до резолвинга тел.
        let rp = resolve_program_src(
            "Функция Main()\nВозврат Helper();\nКонецФункции\n\nФункция Helper()\nВозврат 1;\nКонецФункции",
        );
        assert_eq!(rp.functions.len(), 2);
        assert_eq!(rp.functions[0].name, "Main");
        assert_eq!(
            rp.functions[0].body,
            vec![RStmt::Return(Some(RExpr::Call {
                func: 1,
                args: vec![],
            }))]
        );
    }

    #[test]
    fn params_occupy_the_first_slots_by_val_flag_recorded() {
        let rp = resolve_program_src("Процедура П(Знач а, б)\nКонецПроцедуры");
        let f = &rp.functions[0];
        assert_eq!(f.locals[0], "а");
        assert_eq!(f.locals[1], "б");
        assert_eq!(f.params[0].by_val, true);
        assert_eq!(f.params[1].by_val, false);
    }

    #[test]
    fn argument_count_mismatch_is_an_error() {
        let prog = parse("Функция Ф(а)\nВозврат а;\nКонецФункции\nx = Ф(1, 2);").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Ф".to_string(),
                expected: 1,
                found: 2,
            }
        );
    }

    #[test]
    fn duplicate_function_name_is_an_error() {
        let prog = parse("Функция Ф()\nКонецФункции\nПроцедура ф()\nКонецПроцедуры").unwrap();
        assert!(matches!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::DuplicateFunction(_)
        ));
    }

    #[test]
    fn top_level_can_call_functions_declared_anywhere_in_module() {
        let rp = resolve_program_src("Процедура П(x)\nКонецПроцедуры\nП(1);");
        assert_eq!(
            rp.top_level.body,
            vec![RStmt::ExprStmt(RExpr::Call {
                func: 0,
                args: vec![RExpr::Number(BslNumber::from_i64(1))],
            })]
        );
    }
}
