//! Статически доказанные ядровые приёмники.
//!
//! С реестром компонентов каждый вызов метода компилируется в открытый
//! `CallObjectMethod`: получатель в динамически типизированном BSL может
//! оказаться компонентным объектом, и закрытое перечисление
//! `BuiltinMethod` для него не годится. Открытая операция дороже закрытой из-за перехода в
//! `step_cold` и общей погрузки аргументов, поэтому статически доказанный ядровой приёмник
//! сохраняет закрытую диспетчеризацию.
//!
//! Этот модуль возвращает закрытым опкодам ровно те сайты, где открытость
//! ничего не даёт: переменная считается ядровой, когда ВСЕ её перепривязки
//! в модуле — `Новый T` одного из типов `NEW_TYPES`. Анализ консервативен: любой путь,
//! которым слот может получить значение неизвестного типа, исключает
//! переменную целиком.
//!
//! Перепривязкой считаются: присваивание, роль переменной числового или
//! итерационного цикла, передача голым именем в by-ref параметр
//! пользовательской функции (встроенные и компонентные функции, методы и
//! конструкторы получают аргументы по значению — см. `CallArgs::load` в
//! `bsl-vm`). `Выполнить` и `Вычислить` исполняют произвольный текст,
//! который видит слоты кадра (а из функций — и модульные переменные) и
//! может переприсвоить любой из них: динамика в области снимает
//! отслеживание её локальных, динамика где угодно в модуле — отслеживание
//! модульных переменных. Параметры функций приходят с неизвестным типом и
//! не отслеживаются вовсе.
//!
//! Обход зеркалит порядок разрешения имён в `Resolver`: и локальные
//! объявления (присваиванием, `Перем`, переменной цикла), и выбор
//! «локальная или модульная» делаются по ходу тела, поэтому каждый сайт
//! попадает к той же переменной, к какой его отнесёт резолвер.

use std::collections::{HashMap, HashSet};

use bsl_syntax::{Expr, LValue, Param, Stmt, StmtKind};

use crate::resolver::NEW_TYPES;

/// Статически доказанный ядровой приёмник. Конкретный тип не важен: все ядровые
/// объекты обслуживает один закрытый `CallMethod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CoreReceiver;

/// Итог анализа: по карте на область. Ключ локальных карт — имя переменной
/// в верхнем регистре (слоты локальных назначаются позже, при резолвинге),
/// ключ модульной — номер слота (он известен уже из порядка `Перем`).
pub(crate) struct CoreReceiverMaps {
    /// Тело модуля. Содержит и вердикты модульных переменных: их слоты —
    /// первые локальные этого кадра.
    pub top: HashMap<String, CoreReceiver>,
    /// По одной карте на функцию, в порядке объявления.
    pub functions: Vec<HashMap<String, CoreReceiver>>,
    /// Модульные переменные для `RExpr::ModuleVar` из тел функций.
    pub module_slots: HashMap<u32, CoreReceiver>,
}

/// Наблюдения по одному множеству переменных (локальным одной области или
/// модульным).
#[derive(Default)]
struct Facts {
    /// Имя (верхний регистр) → классификация каждой перепривязки:
    /// `Some(T)` — `Новый T` ядрового типа, `None` — любое другое правое
    /// выражение.
    assigns: HashMap<String, Vec<Option<CoreReceiver>>>,
    /// Имена, исключённые из отслеживания независимо от присваиваний.
    killed: HashSet<String>,
}

impl Facts {
    /// Вердикт по имени: перепривязки есть, и все — один ядровый тип.
    fn verdict(&self, upper: &str) -> Option<CoreReceiver> {
        if self.killed.contains(upper) {
            return None;
        }
        let assigns = self.assigns.get(upper)?;
        let first = (*assigns.first()?)?;
        assigns
            .iter()
            .all(|ctor| *ctor == Some(first))
            .then_some(first)
    }
}

/// Обходчик одной области. `by_val_modes` — режимы параметров
/// пользовательских функций модуля (имя в верхнем регистре → `by_val` по
/// позициям): присутствие имени в карте и отличает пользовательский вызов
/// от встроенного/компонентного, зеркаля порядок разрешения в
/// `resolve_call`.
struct Collector<'a> {
    locals: Facts,
    module: &'a mut Facts,
    has_dynamic: bool,
    /// Имена, разрешающиеся в этой области как локальные, в верхнем
    /// регистре; пополняется по ходу обхода, как `Resolver::declare`.
    declared: HashSet<String>,
    /// Имена модульных переменных. У тела модуля пусто: там они и есть
    /// локальные кадра, их сайты собираются как локальные и учитываются
    /// в модульном вердикте на агрегации.
    module_upper: &'a HashSet<String>,
    by_val_modes: &'a HashMap<String, Vec<bool>>,
    registry: Option<&'a bsl_rt::RuntimeRegistry>,
}

impl Collector<'_> {
    /// Куда относится имя в текущей точке обхода — в локальные факты или
    /// в модульные. Порядок повторяет `Resolver`: локальное объявление
    /// выигрывает, иначе модульная, иначе присваивание объявляет
    /// локальную.
    fn facts_for(&mut self, name: &str, declares: bool) -> Option<&mut Facts> {
        let upper = name.to_uppercase();
        if self.declared.contains(&upper) {
            return Some(&mut self.locals);
        }
        if self.module_upper.contains(&upper) {
            return Some(self.module);
        }
        if declares {
            self.declared.insert(upper);
        }
        Some(&mut self.locals)
    }

    fn assign(&mut self, name: &str, ctor: Option<CoreReceiver>) {
        let upper = name.to_uppercase();
        if let Some(facts) = self.facts_for(name, true) {
            facts.assigns.entry(upper).or_default().push(ctor);
        }
    }

    fn kill(&mut self, name: &str) {
        let upper = name.to_uppercase();
        if let Some(facts) = self.facts_for(name, false) {
            facts.killed.insert(upper);
        }
    }

    /// Переменная цикла: `Resolver` для неё зовёт `declare`, то есть в
    /// функции имя с этого места локальное, даже если совпадает с
    /// модульным.
    fn kill_loop_var(&mut self, name: &str) {
        let upper = name.to_uppercase();
        self.declared.insert(upper.clone());
        self.locals.killed.insert(upper);
    }

    /// `Новый T(...)` ядрового типа? Внешний конструктор с таким именем перехватывает его по
    /// тем же правилам, что и `resolve_new`, и тогда приёмник не доказан. Дескрипторы самого `bsl-rt`
    /// остаются ядровыми: присутствие имени в реестре больше не означает внешнего типа.
    fn classify_ctor(&self, value: &Expr) -> Option<CoreReceiver> {
        let Expr::New { type_name, .. } = value else {
            return None;
        };
        if let Some(registry) = self.registry
            && let Some((library, _)) = registry.lookup_constructor(type_name)
            && registry
                .library(library)
                .is_some_and(|descriptor| descriptor.package() != bsl_rt::PACKAGE_NAME)
        {
            return None;
        }
        let upper = type_name.to_uppercase();
        if !NEW_TYPES.iter().any(|known| known.to_uppercase() == upper) {
            return None;
        }
        Some(CoreReceiver)
    }

    fn walk_block(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Assign { target, value } => {
                match target {
                    LValue::Name(name) => {
                        let ctor = self.classify_ctor(value);
                        self.assign(name, ctor);
                    }
                    // `а.поле = х` и `а[и] = х` мутируют содержимое, но не
                    // перепривязывают слот — тип приёмника не меняется.
                    LValue::Index { obj, index } => {
                        self.walk_expr(obj);
                        self.walk_expr(index);
                    }
                    LValue::Field { obj, .. } => self.walk_expr(obj),
                }
                self.walk_expr(value);
            }
            StmtKind::ExprStmt(expr) => self.walk_expr(expr),
            StmtKind::If {
                cond,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                self.walk_expr(cond);
                self.walk_block(then_branch);
                for (elsif_cond, elsif_body) in elsif_branches {
                    self.walk_expr(elsif_cond);
                    self.walk_block(elsif_body);
                }
                if let Some(else_body) = else_branch {
                    self.walk_block(else_body);
                }
            }
            StmtKind::While { cond, body } => {
                self.walk_expr(cond);
                self.walk_block(body);
            }
            StmtKind::ForNumeric {
                var,
                from,
                to,
                body,
            } => {
                // Границы вычисляются в ОБЪЕМЛЮЩЕЙ области, до того как имя
                // цикла станет локальным: `kill_loop_var` идёт после их
                // обхода. Иначе пре-проход объявил бы имя локальным раньше
                // резолвера и разошёлся бы с ним на границе, ссылающейся на
                // одноимённую переменную.
                self.walk_expr(from);
                self.walk_expr(to);
                self.kill_loop_var(var);
                self.walk_block(body);
            }
            StmtKind::ForEach { var, iter, body } => {
                self.walk_expr(iter);
                self.kill_loop_var(var);
                self.walk_block(body);
            }
            StmtKind::Return(value) => {
                if let Some(expr) = value {
                    self.walk_expr(expr);
                }
            }
            StmtKind::Break | StmtKind::Continue | StmtKind::Label(_) | StmtKind::Goto(_) => {}
            StmtKind::VarDecl(decl) => {
                // `Перем` объявляет локальные; с этого места одноимённая
                // модульная в области не видна — ровно как у резолвера.
                for name in &decl.names {
                    self.declared.insert(name.to_uppercase());
                }
            }
            StmtKind::Try { body, except_body } => {
                self.walk_block(body);
                self.walk_block(except_body);
            }
            StmtKind::Raise(value) => {
                if let Some(expr) = value {
                    self.walk_expr(expr);
                }
            }
            StmtKind::Execute(expr) => {
                self.has_dynamic = true;
                self.walk_expr(expr);
            }
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(_)
            | Expr::Str(_)
            | Expr::Date(_)
            | Expr::Bool(_)
            | Expr::Undefined
            | Expr::Null
            | Expr::Ident(_) => {}
            Expr::Await(expr) | Expr::Unary { expr, .. } => self.walk_expr(expr),
            Expr::Binary { lhs, rhs, .. } => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            Expr::Call { callee, args } => {
                match callee.as_ref() {
                    Expr::Ident(name) => {
                        let upper = name.to_uppercase();
                        if let Some(modes) = self.by_val_modes.get(&upper) {
                            // Пользовательская функция: параметр без
                            // `Знач` алиасит регистр вызывающего и может
                            // переприсвоить переданную голым именем
                            // переменную.
                            for (position, arg) in args.iter().enumerate() {
                                if let Some(Expr::Ident(arg_name)) = arg
                                    && !modes.get(position).copied().unwrap_or(false)
                                {
                                    self.kill(arg_name);
                                }
                            }
                        } else if bsl_rt::folded_eq(name, "Вычислить")
                            || bsl_rt::folded_eq(name, "Eval")
                        {
                            // Строка `Вычислить` видит слоты кадра и может
                            // вызвать функцию с by-ref параметром —
                            // статически текст не виден.
                            self.has_dynamic = true;
                        }
                    }
                    other => self.walk_expr(other),
                }
                for arg in args.iter().flatten() {
                    self.walk_expr(arg);
                }
            }
            Expr::Index { obj, index } => {
                self.walk_expr(obj);
                self.walk_expr(index);
            }
            Expr::Field { obj, .. } => self.walk_expr(obj),
            Expr::New { args, .. } => {
                for arg in args {
                    self.walk_expr(arg);
                }
            }
            Expr::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.walk_expr(cond);
                self.walk_expr(then_expr);
                self.walk_expr(else_expr);
            }
        }
    }
}

/// Анализ модуля целиком: тело + функции в порядке объявления.
pub(crate) fn analyze(
    top_stmts: &[Stmt],
    functions: &[(&[Param], &[Stmt])],
    module_vars: &[String],
    by_val_modes: &HashMap<String, Vec<bool>>,
    registry: Option<&bsl_rt::RuntimeRegistry>,
) -> CoreReceiverMaps {
    let module_upper_set: HashSet<String> =
        module_vars.iter().map(|name| name.to_uppercase()).collect();
    let empty_module_set = HashSet::new();

    // Сайты модульных переменных из тел функций стекаются сюда.
    let mut module_facts = Facts::default();

    // Тело модуля: модульные переменные — его собственные локальные, их
    // сайты остаются в локальных фактах и участвуют в модульном вердикте
    // на агрегации ниже.
    let mut top_collector = Collector {
        locals: Facts::default(),
        module: &mut module_facts,
        has_dynamic: false,
        declared: module_upper_set.clone(),
        module_upper: &empty_module_set,
        by_val_modes,
        registry,
    };
    top_collector.walk_block(top_stmts);
    let top_facts = top_collector.locals;
    let top_dynamic = top_collector.has_dynamic;

    let mut function_locals: Vec<Facts> = Vec::with_capacity(functions.len());
    let mut function_dynamics: Vec<bool> = Vec::with_capacity(functions.len());
    for (params, body) in functions {
        let mut collector = Collector {
            locals: Facts::default(),
            module: &mut module_facts,
            has_dynamic: false,
            declared: params.iter().map(|p| p.name.to_uppercase()).collect(),
            module_upper: &module_upper_set,
            by_val_modes,
            registry,
        };
        // Параметры приходят с неизвестным типом.
        for param in *params {
            collector.locals.killed.insert(param.name.to_uppercase());
            if let Some(default) = &param.default {
                collector.walk_expr(default);
            }
        }
        collector.walk_block(body);
        function_locals.push(collector.locals);
        function_dynamics.push(collector.has_dynamic);
    }

    // Динамика где угодно достаёт до модульных переменных: фрагмент видит
    // их из любого кадра.
    let global_dynamic = top_dynamic || function_dynamics.iter().any(|dynamic| *dynamic);

    let module_slots: HashMap<u32, CoreReceiver> = if global_dynamic {
        HashMap::new()
    } else {
        module_vars
            .iter()
            .enumerate()
            .filter_map(|(slot, name)| {
                let upper = name.to_uppercase();
                if top_facts.killed.contains(&upper) || module_facts.killed.contains(&upper) {
                    return None;
                }
                let mut assigns: Vec<Option<CoreReceiver>> = Vec::new();
                if let Some(top_assigns) = top_facts.assigns.get(&upper) {
                    assigns.extend(top_assigns.iter().copied());
                }
                if let Some(fn_assigns) = module_facts.assigns.get(&upper) {
                    assigns.extend(fn_assigns.iter().copied());
                }
                let first = (*assigns.first()?)?;
                assigns
                    .iter()
                    .all(|ctor| *ctor == Some(first))
                    .then_some((slot as u32, first))
            })
            .collect()
    };

    // Карта тела модуля: свои локальные — по локальному вердикту (динамика
    // верхнего уровня снимает их все), модульные — по модульному, даже
    // если на верхнем уровне присваиваний не было.
    let mut top: HashMap<String, CoreReceiver> = if top_dynamic {
        HashMap::new()
    } else {
        top_facts
            .assigns
            .keys()
            .filter(|upper| !module_upper_set.contains(*upper))
            .filter_map(|upper| top_facts.verdict(upper).map(|core| (upper.clone(), core)))
            .collect()
    };
    for (slot, name) in module_vars.iter().enumerate() {
        if let Some(core) = module_slots.get(&(slot as u32)) {
            top.insert(name.to_uppercase(), *core);
        }
    }

    let functions = function_locals
        .iter()
        .zip(&function_dynamics)
        .map(|(facts, dynamic)| {
            if *dynamic {
                return HashMap::new();
            }
            facts
                .assigns
                .keys()
                .filter_map(|upper| facts.verdict(upper).map(|core| (upper.clone(), core)))
                .collect()
        })
        .collect();

    CoreReceiverMaps {
        top,
        functions,
        module_slots,
    }
}
