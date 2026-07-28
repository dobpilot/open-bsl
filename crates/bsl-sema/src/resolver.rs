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
    /// `Ф(1, , 3)` — позиция пропущена (`,,`), но у соответствующего
    /// параметра `Ф` нет значения по умолчанию: пропустить нечем.
    MissingRequiredArgument {
        name: String,
        position: usize,
    },
    /// `'20240230'` — литерал прошёл лексер (цифры, верная длина), но
    /// такой календарной даты не существует.
    BadDateLiteral(String),
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
    /// Есть ли у параметра в этой позиции значение по умолчанию — длина
    /// заодно и есть арность (по-настоящему нужен только режим передачи
    /// каждого параметра, а он читается из `ResolvedFunction::params` на
    /// этапе кодогена, не отсюда). Пропущенный аргумент (`Ф(1, , 3)`)
    /// допустим только там, где здесь `true` — иначе это ошибка резолвинга
    /// вызывающего кода, а не рантайма вызываемой функции.
    has_default: Vec<bool>,
}

/// Резолвит весь модуль: собирает сигнатуры всех `Процедура`/`Функция` за
/// один проход (чтобы вызовы работали независимо от порядка объявления и
/// поддерживали рекурсию), затем резолвит каждое тело и операторы верхнего
/// уровня.
/// НЕ ИЗМЕРЕНО(SCOPE.MODULE_VARS) — точнее, измерено и НЕ РЕАЛИЗОВАНО:
/// платформа даёт процедурам видеть переменные уровня модуля (`Перем` в
/// начале файла), а здесь области модуля нет вовсе. Каждая функция
/// получает свой пустой `Resolver`, поэтому чтение такой переменной внутри
/// функции — `UndefinedVariable`, а запись молча заводит локальную. Чтобы
/// это закрыть, нужна отдельная область имён модуля и хранилище под неё в
/// VM (кадр верхнего уровня живёт не всё исполнение).
pub fn resolve_program(items: &[Item]) -> Result<ResolvedProgram, SemaError> {
    let mut sigs: HashMap<String, FuncSig> = HashMap::new();
    let mut func_items: Vec<&Item> = Vec::new();
    let mut top_stmts: Vec<AStmt> = Vec::new();

    for item in items {
        match item {
            Item::Function(f) => {
                declare_sig(
                    &mut sigs,
                    &f.name,
                    f.params.iter().map(|p| p.default.is_some()).collect(),
                )?;
                func_items.push(item);
            }
            Item::Procedure(p) => {
                declare_sig(
                    &mut sigs,
                    &p.name,
                    p.params.iter().map(|p| p.default.is_some()).collect(),
                )?;
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
        // Значения по умолчанию резолвятся ПОСЛЕ тела, но той же `r` — её
        // `locals`/`index` к этому моменту уже содержат слоты параметров
        // (объявлены выше, до `resolve_block`), так что дефолт вида
        // `Ф(б = а + 1)`, ссылающийся на предыдущий параметр, резолвится
        // корректно.
        let mut resolved_params = Vec::with_capacity(params.len());
        for p in params {
            let default = match &p.default {
                Some(e) => Some(r.resolve_expr(e)?),
                None => None,
            };
            resolved_params.push(ResolvedParam { by_val: p.by_val, default });
        }
        functions.push(ResolvedFunction {
            name: name.clone(),
            uses_dynamic: crate::resolved::block_uses_dynamic(&resolved_body),
            params: resolved_params,
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
        uses_dynamic: crate::resolved::block_uses_dynamic(&top_body),
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
    has_default: Vec<bool>,
) -> Result<(), SemaError> {
    let key = name.to_uppercase();
    if sigs.contains_key(&key) {
        return Err(SemaError::DuplicateFunction(name.to_string()));
    }
    let index = sigs.len() as u32;
    sigs.insert(key, FuncSig { index, has_default });
    Ok(())
}

/// Резолвит плоский скрипт верхнего уровня без объявлений функций — удобно
/// для тестов. Функции резолвятся только через [`resolve_program`].
pub fn resolve_script(stmts: &[AStmt]) -> Result<Resolved, SemaError> {
    let empty_funcs = HashMap::new();
    let mut r = Resolver {
        locals: Vec::new(),
        index: HashMap::new(),
        funcs: &empty_funcs,
    };
    let body = r.resolve_block(stmts)?;
    Ok(Resolved {
        uses_dynamic: crate::resolved::block_uses_dynamic(&body),
        locals: r.locals,
        body,
    })
}

/// Резолвит фрагмент кода для `Выполнить`/`Вычислить`: `existing_locals` —
/// уже объявленные переменные окружающего скрипта, ЗАСЕВАЮТСЯ в резолвер
/// первыми, поэтому ссылки на них в фрагменте попадают на ТЕ ЖЕ слоты, а
/// не заводят копию. Новые имена, объявленные внутри фрагмента, получают
/// слоты ПОСЛЕ существующих — полный список возвращается вызывающему,
/// который сам решает, персистить ли их (VM для `Выполнить`/`Вычислить`
/// внутри уже скомпилированного кода — не персистит: не может позволить
/// себе расширять статически размеченный кадр; REPL — персистит, у него
/// кадр и так растёт от строки к строке).
///
/// ИЗМЕРЕНО на 8.3.27: фрагмент ВИДИТ процедуры и функции модуля —
/// `Вычислить("Удвоить(21)")` возвращает 42. Поэтому `signatures` тянется
/// сюда из уже скомпилированной программы: пара «имя -> (номер, арность)»
/// в том же порядке, в каком функции лежат в `Program::chunks[1..]`.
/// Пустой список означает «функций нет» (так зовёт REPL до первого
/// объявления), а не «вызывать нельзя».
pub fn resolve_snippet_stmts(
    existing_locals: &[String],
    stmts: &[AStmt],
    signatures: &[(String, usize)],
) -> Result<(Vec<String>, Vec<RStmt>), SemaError> {
    let empty_funcs: HashMap<String, FuncSig> = signatures
        .iter()
        .enumerate()
        .map(|(index, (name, arity))| {
            (
                name.to_uppercase(),
                FuncSig {
                    index: index as u32,
                    has_default: vec![false; *arity],
                },
            )
        })
        .collect();
    let index = existing_locals
        .iter()
        .enumerate()
        .map(|(i, name)| (name.to_uppercase(), i as u32))
        .collect();
    let mut r = Resolver {
        locals: existing_locals.to_vec(),
        index,
        funcs: &empty_funcs,
    };
    let body = r.resolve_block(stmts)?;
    Ok((r.locals, body))
}

/// Типы, которые умеет строить `Новый` — в каноническом написании, оба
/// языка. Список нужен снаружи (автодополнение REPL предлагает их после
/// `Новый`), а `resolve_new` разбирает каждый по-своему: у них разная
/// арность и разный смысл аргументов, одной таблицей не обойтись. Что
/// список не разъедется с `match`, проверяет
/// `every_new_type_is_recognised_by_resolve_new`.
pub const NEW_TYPES: &[&str] = &[
    "Массив",
    "Array",
    "Структура",
    "Structure",
    "Соответствие",
    "Map",
    "ТаблицаЗначений",
    "ValueTable",
    "ЗаписьТекста",
    "TextWriter",
];

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
                    Ok(Some(RStmt::AssignLocal { slot, value }))
                }
                AExpr::Index { obj, index } => {
                    let obj = self.resolve_expr(obj)?;
                    let index = self.resolve_expr(index)?;
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignIndex { obj, index, value }))
                }
                AExpr::Field { obj, name } => {
                    let obj = self.resolve_expr(obj)?;
                    let value = self.resolve_expr(value)?;
                    Ok(Some(RStmt::AssignField {
                        obj,
                        name: name.clone(),
                        value,
                    }))
                }
                _ => Err(SemaError::Unsupported(
                    "присваивание поддержано только в переменную, индекс или поле",
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
            AStmt::ForEach { var, iter, body } => {
                let iter = self.resolve_expr(iter)?;
                // Как и в ForNumeric: переменная объявляется до тела, живёт
                // и после КонецЦикла.
                let slot = self.declare(var);
                let body = self.resolve_block(body)?;
                Ok(Some(RStmt::ForEach { slot, iter, body }))
            }
            AStmt::Break => Ok(Some(RStmt::Break)),
            AStmt::Continue => Ok(Some(RStmt::Continue)),
            AStmt::Return(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Return(r)))
            }
            AStmt::Try { body, except_body } => {
                let body = self.resolve_block(body)?;
                let except_body = self.resolve_block(except_body)?;
                Ok(Some(RStmt::Try { body, except_body }))
            }
            AStmt::Raise(opt) => {
                let r = match opt {
                    Some(e) => Some(self.resolve_expr(e)?),
                    None => None,
                };
                Ok(Some(RStmt::Raise(r)))
            }
            AStmt::VarDecl(vd) => {
                // Только регистрирует слоты; значение по умолчанию —
                // Неопределено, для этого не нужна ни одна инструкция VM
                // (регистры кадра и так инициализируются Неопределено).
                for name in &vd.names {
                    self.declare(name);
                }
                Ok(None)
            }
            AStmt::Execute(e) => Ok(Some(RStmt::Execute(self.resolve_expr(e)?))),
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
            AExpr::Str(s) => Ok(RExpr::Str(s.clone())),
            // Литерал `'ГГГГММДД'`/`'ГГГГММДДЧЧММСС'`. Лексер уже проверил,
            // что внутри только цифры и их 8 или 14, — но НЕ проверил, что
            // получившаяся дата существует (`'20240230'` пройдёт лексер),
            // поэтому разбор может провалиться и здесь, и это ошибка
            // резолвинга, а не рантайма: литерал известен на этапе
            // компиляции, падать на нём во время исполнения незачем.
            AExpr::Date(digits) => bsl_rt::BslDate::parse_digits(digits)
                .map(RExpr::Date)
                .ok_or(SemaError::BadDateLiteral(digits.clone())),
            AExpr::Index { obj, index } => Ok(RExpr::Index {
                obj: Box::new(self.resolve_expr(obj)?),
                index: Box::new(self.resolve_expr(index)?),
            }),
            AExpr::Field { obj, name } => Ok(RExpr::Field {
                obj: Box::new(self.resolve_expr(obj)?),
                name: name.clone(),
            }),
            AExpr::New { type_name, args } => self.resolve_new(type_name, args),
            AExpr::Ternary { .. } => Err(SemaError::Unsupported(
                "тернарный ?() появится позже",
            )),
        }
    }

    /// `Новый Массив(...)`/`Новый Структура(...)`/`Новый
    /// ТаблицаЗначений()`/`Новый Соответствие()` — единственные формы
    /// `Новый`, которые пока распознаются (случай общих пользовательских
    /// типов отложен, объектов кроме коллекций ещё нет).
    fn resolve_new(&mut self, type_name: &str, args: &[AExpr]) -> Result<RExpr, SemaError> {
        match type_name.to_uppercase().as_str() {
            "МАССИВ" | "ARRAY" => {
                let mut dims = Vec::with_capacity(args.len());
                for a in args {
                    dims.push(self.resolve_expr(a)?);
                }
                Ok(RExpr::NewArray { dims })
            }
            "СТРУКТУРА" | "STRUCTURE" => {
                if args.is_empty() {
                    return Ok(RExpr::NewStructure {
                        keys: Vec::new(),
                        values: Vec::new(),
                    });
                }
                // ВНИМАНИЕ: список ключей обязан быть строковым ЛИТЕРАЛОМ —
                // именно это делает число форм структур конечным и
                // известным на этапе компиляции (`ShapeTable::intern`
                // заводит их с `depth = 0`, то есть с полным запасом
                // переходов). Когда сюда добавят вычисляемую строку
                // (`Новый Структура(КлючиИзПеременной)`), интернировать её
                // ключи в таблицу форм НЕЛЬЗЯ: цикл с разными ключами
                // заводил бы бессмертную форму на каждой итерации, а
                // деградация по `MAX_SHAPE_TRANSITIONS` тут не спасает —
                // каждая такая форма создаётся с нулевой глубиной. Такой
                // конструктор должен сразу строить структуру в словарном
                // режиме (`bsl_rt::StructureStorage::Dictionary`), минуя
                // интернирование.
                let key_text = match &args[0] {
                    AExpr::Str(s) => s,
                    _ => {
                        return Err(SemaError::Unsupported(
                            "Новый Структура(...) со списком полей не строковым литералом появится позже",
                        ))
                    }
                };
                let keys: Vec<String> = key_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let rest = &args[1..];
                let values = if rest.is_empty() {
                    keys.iter().map(|_| RExpr::Undefined).collect()
                } else {
                    if rest.len() != keys.len() {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: "Новый Структура".to_string(),
                            expected: keys.len(),
                            found: rest.len(),
                        });
                    }
                    let mut vs = Vec::with_capacity(rest.len());
                    for a in rest {
                        vs.push(self.resolve_expr(a)?);
                    }
                    vs
                };
                Ok(RExpr::NewStructure { keys, values })
            }
            "ТАБЛИЦАЗНАЧЕНИЙ" | "VALUETABLE" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ТаблицаЗначений".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTable)
            }
            "СООТВЕТСТВИЕ" | "MAP" => {
                if !args.is_empty() {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый Соответствие".to_string(),
                        expected: 0,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewMap)
            }
            "ЗАПИСЬТЕКСТА" | "TEXTWRITER" => {
                if args.len() != 1 {
                    return Err(SemaError::ArgumentCountMismatch {
                        name: "Новый ЗаписьТекста".to_string(),
                        expected: 1,
                        found: args.len(),
                    });
                }
                Ok(RExpr::NewTextWriter {
                    path: Box::new(self.resolve_expr(&args[0])?),
                })
            }
            _ => Err(SemaError::Unsupported(
                "Новый поддержан только для Массив/Структура/ТаблицаЗначений/Соответствие пока",
            )),
        }
    }

    fn resolve_call(
        &mut self,
        callee: &AExpr,
        args: &[Option<AExpr>],
    ) -> Result<RExpr, SemaError> {
        match callee {
            AExpr::Ident(name) => {
                if let Some((index, has_default)) = self
                    .funcs
                    .get(&name.to_uppercase())
                    .map(|s| (s.index, s.has_default.clone()))
                {
                    let arity = has_default.len();
                    if args.len() != arity {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: arity,
                            found: args.len(),
                        });
                    }
                    // В отличие от `resolve_required_args` (используется
                    // ниже для builtin'ов, у которых нет объявленных
                    // умолчаний) — пропуск позиции здесь допустим, если у
                    // ЭТОГО параметра есть значение по умолчанию: тогда
                    // компилируется маркер `RExpr::Skipped`, а не ошибка.
                    let mut rargs = Vec::with_capacity(args.len());
                    for (i, a) in args.iter().enumerate() {
                        match a {
                            Some(e) => rargs.push(self.resolve_expr(e)?),
                            None if has_default[i] => rargs.push(RExpr::Skipped),
                            None => {
                                return Err(SemaError::MissingRequiredArgument {
                                    name: name.clone(),
                                    position: i,
                                })
                            }
                        }
                    }
                    return Ok(RExpr::Call {
                        func: index,
                        args: rargs,
                    });
                }
                // `Окр(x[, ЧислоРазрядов[, Режим]])` — единственный
                // builtin с необязательными аргументами, до генерального
                // механизма умолчаний builtin'ов (которого нет — см.
                // `bsl_rt::BuiltinFn::arity`, всегда фиксированная арность).
                // Подставляем недостающие `0` литералами здесь же, а не
                // заводим вариативную арность ради одной функции:
                // `BuiltinFn::Round` в рантайме всегда видит ровно 3
                // аргумента. `0` для режима означает "умолчание" (см.
                // `BslValue::round`).
                if name.eq_ignore_ascii_case("Окр") || name.eq_ignore_ascii_case("Round") {
                    const ROUND_ARITY: usize = 3;
                    if args.is_empty() || args.len() > ROUND_ARITY {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: ROUND_ARITY,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_required_args(args)?;
                    while rargs.len() < ROUND_ARITY {
                        // ИМЕННО `Неопределено`, а не `0`: измерено, что
                        // умолчание платформы совпадает с режимом 1, а не с
                        // режимом 0, — подстановка нуля меняла бы семантику.
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin: bsl_rt::BuiltinFn::Round,
                        args: rargs,
                    });
                }
                if let Some(builtin) = bsl_rt::BuiltinFn::lookup(name) {
                    let (min, max) = builtin.arity_range();
                    if args.len() < min || args.len() > max {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: max,
                            found: args.len(),
                        });
                    }
                    // Недостающие необязательные позиции добиваются
                    // `Неопределено` ЗДЕСЬ, а не вариативной арностью в
                    // рантайме: `call_builtin_fn` тогда всегда индексирует
                    // фиксированный набор аргументов, а «аргумент опущен»
                    // становится обычным значением, которое сама функция и
                    // трактует (`Сред` — до конца строки, `КодСимвола` —
                    // позиция 1, `СтрШаблон` — пустая подстановка).
                    let mut rargs = self.resolve_required_args(args)?;
                    while rargs.len() < max {
                        rargs.push(RExpr::Undefined);
                    }
                    return Ok(RExpr::CallBuiltinFn {
                        builtin,
                        args: rargs,
                    });
                }
                if name.eq_ignore_ascii_case("Вычислить") || name.eq_ignore_ascii_case("Eval") {
                    if args.len() != 1 {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            found: args.len(),
                        });
                    }
                    let mut rargs = self.resolve_required_args(args)?;
                    return Ok(RExpr::DynEval(Box::new(rargs.remove(0))));
                }
                Err(SemaError::UndefinedFunction(name.clone()))
            }
            AExpr::Field { obj, name } => {
                let method = bsl_rt::BuiltinMethod::lookup(name).ok_or(SemaError::Unsupported(
                    "этот метод объекта пока не поддержан",
                ))?;
                // `Добавить` полиморфен по типу получателя (0 аргументов —
                // новая строка таблицы, 1 — элемент массива/колонка), а тип
                // получателя в динамическом BSL здесь ещё не известен:
                // финальную проверку арности для него делает рантайм (см.
                // `bsl_rt::call_builtin_method`). Для остальных методов
                // арность фиксирована и проверяется сразу.
                let expected: Option<usize> = match method {
                    bsl_rt::BuiltinMethod::Count
                    | bsl_rt::BuiltinMethod::Clear
                    | bsl_rt::BuiltinMethod::Close => Some(0),
                    bsl_rt::BuiltinMethod::Delete | bsl_rt::BuiltinMethod::Get => Some(1),
                    bsl_rt::BuiltinMethod::Insert => Some(2),
                    bsl_rt::BuiltinMethod::FindRows
                    | bsl_rt::BuiltinMethod::Sort
                    | bsl_rt::BuiltinMethod::Total => Some(1),
                    bsl_rt::BuiltinMethod::Write => Some(1),
                    bsl_rt::BuiltinMethod::UnloadColumn | bsl_rt::BuiltinMethod::IndexOf => Some(1),
                    bsl_rt::BuiltinMethod::LoadColumn | bsl_rt::BuiltinMethod::Move => Some(2),
                    // `Свойство` — 1 или 2 (см. `BslValue::structure_property`),
                    // `Найти` — 1 или 2 (список колонок необязателен), как и
                    // у `Добавить` арность решает рантайм. Из волны 3 так же
                    // устроены `Скопировать` (0..2), `СкопироватьКолонки`
                    // (0..1) и `Свернуть` (1..2).
                    bsl_rt::BuiltinMethod::Add
                    | bsl_rt::BuiltinMethod::Property
                    | bsl_rt::BuiltinMethod::Find
                    | bsl_rt::BuiltinMethod::Copy
                    | bsl_rt::BuiltinMethod::CopyColumns
                    | bsl_rt::BuiltinMethod::Collapse => None,
                };
                if let Some(expected) = expected {
                    if args.len() != expected {
                        return Err(SemaError::ArgumentCountMismatch {
                            name: name.clone(),
                            expected,
                            found: args.len(),
                        });
                    }
                }
                let rargs = self.resolve_required_args(args)?;
                let obj = self.resolve_expr(obj)?;
                Ok(RExpr::CallMethod {
                    obj: Box::new(obj),
                    method,
                    args: rargs,
                })
            }
            _ => Err(SemaError::Unsupported(
                "вызов не по простому имени/методу появится позже",
            )),
        }
    }

    /// Пока не поддержаны пропущенные аргументы (`Ф(1, , 3)`) ни для
    /// пользовательских функций, ни для встроенных — они появятся вместе
    /// со значениями по умолчанию.
    fn resolve_required_args(&mut self, args: &[Option<AExpr>]) -> Result<Vec<RExpr>, SemaError> {
        let mut rargs = Vec::with_capacity(args.len());
        for a in args {
            match a {
                Some(e) => rargs.push(self.resolve_expr(e)?),
                None => {
                    return Err(SemaError::Unsupported(
                        "пропущенные аргументы Ф(1, , 3) появятся вместе со значениями по умолчанию",
                    ));
                }
            }
        }
        Ok(rargs)
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

    /// `NEW_TYPES` — список для автодополнения, а разбор `Новый` живёт в
    /// `match` по имени типа. Разъехаться им нельзя: имя из списка,
    /// которого `resolve_new` не знает, REPL предлагал бы вхолостую.
    #[test]
    fn every_new_type_is_recognised_by_resolve_new() {
        for type_name in NEW_TYPES {
            let prog = parse(&format!("x = Новый {type_name}();")).unwrap();
            let stmts = items_to_stmts(prog.items);
            // Арность у типов разная (`ЗаписьТекста` требует путь), поэтому
            // ошибка числа аргументов здесь допустима — недопустимо
            // «такой тип не поддержан».
            match resolve_script(&stmts) {
                Ok(_) | Err(SemaError::ArgumentCountMismatch { .. }) => {}
                Err(other) => panic!("Новый {type_name}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_type_outside_the_list_is_not_constructible() {
        let prog = parse("x = Новый СписокЗначений();").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts),
            Err(SemaError::Unsupported(_))
        ));
    }

    #[test]
    fn implicit_declaration_on_first_assignment() {
        let r = resolve_src("PI = 3.14;");
        assert_eq!(r.locals, vec!["PI".to_string()]);
        assert_eq!(
            r.body,
            vec![RStmt::AssignLocal {
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
            RStmt::AssignLocal {
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
    fn builtin_function_call_resolves_without_user_declaration() {
        let r = resolve_src("x = sqrt(4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::CallBuiltinFn {
                    builtin: bsl_rt::BuiltinFn::Sqrt,
                    args: vec![RExpr::Number(BslNumber::from_i64(4))],
                },
            }
        );
    }

    #[test]
    fn builtin_function_arity_mismatch_is_an_error() {
        let prog = parse("x = Pow(2);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert_eq!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::ArgumentCountMismatch {
                name: "Pow".to_string(),
                expected: 2,
                found: 1,
            }
        );
    }

    #[test]
    fn count_method_call_resolves_on_array() {
        let r = resolve_src("a = Новый Массив(3);\nn = a.Count();");
        assert_eq!(
            r.body[1],
            RStmt::AssignLocal {
                slot: 1,
                value: RExpr::CallMethod {
                    obj: Box::new(RExpr::Local(0)),
                    method: bsl_rt::BuiltinMethod::Count,
                    args: vec![],
                },
            }
        );
    }

    #[test]
    fn unknown_method_call_is_unsupported() {
        let prog = parse("a = Новый Массив(3);\nn = a.НетТакогоМетода();").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::Unsupported(_)
        ));
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
    fn skipping_a_parameter_without_a_default_is_an_error() {
        let prog = parse("Функция Ф(а, б)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        assert_eq!(
            resolve_program(&prog.items).unwrap_err(),
            SemaError::MissingRequiredArgument {
                name: "Ф".to_string(),
                position: 1,
            }
        );
    }

    #[test]
    fn skipping_a_defaulted_parameter_resolves_to_skipped_marker() {
        let prog = parse("Функция Ф(а, б = 100)\nВозврат а;\nКонецФункции\nx = Ф(1, );").unwrap();
        let resolved = resolve_program(&prog.items).unwrap();
        match &resolved.top_level.body[0] {
            RStmt::AssignLocal { value: RExpr::Call { args, .. }, .. } => {
                assert_eq!(args[1], RExpr::Skipped);
            }
            other => panic!("expected AssignLocal(Call), got {other:?}"),
        }
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

    #[test]
    fn new_array_resolves_dimensions() {
        let r = resolve_src("a = Новый Массив(3, 4);");
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewArray {
                    dims: vec![
                        RExpr::Number(BslNumber::from_i64(3)),
                        RExpr::Number(BslNumber::from_i64(4)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_literal_keys_and_values() {
        let r = resolve_src(r#"s = Новый Структура("x,y", 1, 2);"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![
                        RExpr::Number(BslNumber::from_i64(1)),
                        RExpr::Number(BslNumber::from_i64(2)),
                    ],
                },
            }
        );
    }

    #[test]
    fn new_structure_keys_only_defaults_values_to_undefined() {
        let r = resolve_src(r#"s = Новый Структура("x,y");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::NewStructure {
                    keys: vec!["x".to_string(), "y".to_string()],
                    values: vec![RExpr::Undefined, RExpr::Undefined],
                },
            }
        );
    }

    #[test]
    fn new_structure_with_dynamic_key_list_is_unsupported() {
        // Проверка формы аргумента (строковый литерал?) идёт раньше
        // резолвинга самого идентификатора, поэтому это Unsupported, а не
        // UndefinedVariable("k"), даже когда k нигде не объявлена.
        let prog = parse("s = Новый Структура(k);").unwrap();
        let stmts = items_to_stmts(prog.items);
        assert!(matches!(
            resolve_script(&stmts).unwrap_err(),
            SemaError::Unsupported(_)
        ));
    }

    #[test]
    fn index_and_field_assignment_targets() {
        let r = resolve_src(
            "a = Новый Массив(1);\ns = Новый Структура(\"x\");\na[0] = 1;\ns.x = 2;",
        );
        assert!(matches!(r.body[2], RStmt::AssignIndex { .. }));
        assert!(matches!(r.body[3], RStmt::AssignField { .. }));
    }

    #[test]
    fn for_each_declares_loop_variable() {
        let r = resolve_src("a = Новый Массив();\nДля Каждого x Из a Цикл\ny = x;\nКонецЦикла");
        assert_eq!(r.locals[0], "a".to_string());
        assert!(matches!(r.body[1], RStmt::ForEach { .. }));
    }

    #[test]
    fn try_except_resolves_both_bodies() {
        let r = resolve_src("Попытка\nx = 1;\nИсключение\ny = 2;\nКонецПопытки");
        assert!(matches!(r.body[0], RStmt::Try { .. }));
    }

    #[test]
    fn raise_with_and_without_expression() {
        let r = resolve_src("Попытка\nВызватьИсключение \"ошибка\";\nИсключение\nВызватьИсключение;\nКонецПопытки");
        match &r.body[0] {
            RStmt::Try { body, except_body } => {
                assert_eq!(body[0], RStmt::Raise(Some(RExpr::Str("ошибка".to_string()))));
                assert_eq!(except_body[0], RStmt::Raise(None));
            }
            other => panic!("expected Try, got {other:?}"),
        }
    }

    #[test]
    fn execute_resolves_to_rstmt_execute() {
        let r = resolve_src(r#"Выполнить("x = 1");"#);
        assert_eq!(
            r.body[0],
            RStmt::Execute(RExpr::Str("x = 1".to_string()))
        );
    }

    #[test]
    fn vychislit_resolves_to_dyn_eval() {
        let r = resolve_src(r#"y = Вычислить("2+2");"#);
        assert_eq!(
            r.body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::DynEval(Box::new(RExpr::Str("2+2".to_string()))),
            }
        );
    }

    #[test]
    fn resolve_snippet_stmts_seeds_existing_locals_and_extends() {
        let existing = vec!["x".to_string()];
        let prog = parse("x = x + 1;\ny = 2;").unwrap();
        let stmts = items_to_stmts(prog.items);
        let (locals, body) = resolve_snippet_stmts(&existing, &stmts, &[]).unwrap();
        assert_eq!(locals, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(
            body[0],
            RStmt::AssignLocal {
                slot: 0,
                value: RExpr::Binary {
                    op: bsl_syntax::BinaryOp::Add,
                    lhs: Box::new(RExpr::Local(0)),
                    rhs: Box::new(RExpr::Number(BslNumber::from_i64(1))),
                },
            }
        );
        assert_eq!(body[1], RStmt::AssignLocal { slot: 1, value: RExpr::Number(BslNumber::from_i64(2)) });
    }
}
