//! SSA над графом из [`crate::cfg`]: значения, `φ` и `Bottom`.
//!
//! Вторая половина шага 5 плана `docs/research/performance/ssa-hotspot-analysis.md`. Как и
//! граф, ничего в выпускаемом байт-коде не меняет: представление строится
//! и проверяется, но кодоген его пока не читает.
//!
//! # Почему Цитрон, а не построение по ходу обхода
//!
//! Классическое размещение `φ` по фронтам доминирования итеративно по
//! построению: рабочий список по блокам, подъём по цепочке
//! непосредственных доминаторов, переименование обходом дерева
//! доминаторов с явным стеком. Популярная альтернатива (Braun и соавторы,
//! построение SSA прямо во время обхода) рекурсивна ПО ГРАФУ — читая
//! значение, она спускается к предшественникам, — а это ровно та
//! величина, на которой сломалась прошлая попытка: цепочка из восьми
//! тысяч последовательных `Если` даёт десятки тысяч блоков. Сделать её
//! итеративной можно, но проще взять алгоритм, который итеративен сам.
//!
//! # `Bottom` — это не `Any`
//!
//! Вход с НЕДОСТИЖИМОГО ребра даёт [`Value::Bottom`]: значения там нет.
//! Прошлая попытка вносила в достижимый `φ` вход `Any` от мёртвого
//! предшественника и тем разрешала себе выводы о пути, которого не
//! существует. `Any` означает «набор неизвестен на достижимом пути», и
//! путать эти два состояния нельзя.

use crate::cfg::{BlockId, Cfg};
use bsl_sema::{RExpr, RStmt, ResolvedArg};

pub type ValueId = usize;

/// Откуда взялось значение слота.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Значение на входе в тело: параметр либо ещё не присвоенная
    /// локальная переменная.
    Entry { slot: u32 },
    /// Записано оператором блока.
    Def { block: BlockId, slot: u32 },
    /// Слияние: по одному операнду на каждого предшественника блока, в
    /// порядке `Block::preds`.
    Phi {
        block: BlockId,
        slot: u32,
        operands: Vec<ValueId>,
    },
    /// Значения нет: вход с недостижимого ребра.
    Bottom,
}

/// Чтение значения: где и какого.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Use {
    pub block: BlockId,
    /// Номер оператора в блоке; `None` — условие терминатора.
    pub stmt: Option<usize>,
    pub value: ValueId,
    pub slot: u32,
}

pub struct Ssa {
    pub values: Vec<Value>,
    /// `φ` каждого блока.
    pub phis: Vec<Vec<ValueId>>,
    /// Определения внутри блока: номер оператора в `Block::stmts` и
    /// значение, которое он записал. Порядок — порядок операторов.
    pub defs: Vec<Vec<(usize, ValueId)>>,
    /// Все использования значений.
    pub uses: Vec<Use>,
    /// Значение каждого слота на ВХОДЕ в блок, уже после его `φ`.
    pub entry: Vec<Option<Vec<ValueId>>>,
    /// То же на выходе; `None` у обоих — блок недостижим, и говорить о
    /// значениях в нём не о чем.
    pub exit: Vec<Option<Vec<ValueId>>>,
    /// Единственное значение `Bottom`: оно одно на всё тело.
    pub bottom: ValueId,
}

/// Фронты доминирования, разреженно.
///
/// Алгоритм Купера—Харви—Кеннеди: для каждого блока с двумя и более
/// предшественниками подняться от каждого предшественника по цепочке
/// непосредственных доминаторов до самого доминатора блока. Плотной
/// матрицы `блоки × блоки` тут нет, и в этом весь смысл.
#[must_use]
pub fn dominance_frontiers(cfg: &Cfg<'_>, idom: &[Option<BlockId>]) -> Vec<Vec<BlockId>> {
    let mut df: Vec<Vec<BlockId>> = vec![Vec::new(); cfg.blocks.len()];
    for b in 0..cfg.blocks.len() {
        if cfg.blocks[b].preds.len() < 2 {
            continue;
        }
        // Подъём идёт ДО доминатора блока и останавливается на нём: сам
        // доминатор во фронт не входит.
        let stop = idom[b];
        if stop.is_none() && b != cfg.entry {
            // Недостижимый блок фронта не порождает.
            continue;
        }
        for &p in &cfg.blocks[b].preds {
            if idom[p].is_none() && p != cfg.entry {
                // От недостижимого предшественника подниматься некуда.
                continue;
            }
            let mut runner = p;
            while Some(runner) != stop {
                if !df[runner].contains(&b) {
                    df[runner].push(b);
                }
                // Выше входа цепочки нет: у него собственного доминатора
                // не бывает, и остановиться надо ЗДЕСЬ, а не оборвать
                // подъём раньше времени. Прежняя редакция проверяла
                // `idom[runner].is_some()` в условии цикла и потому не
                // доходила до заголовка цикла, чей предшественник —
                // входной блок: `φ` не ставилась вовсе.
                if runner == cfg.entry {
                    break;
                }
                let Some(next) = idom[runner] else { break };
                runner = next;
            }
        }
    }
    df
}

/// Слоты, читаемые выражением, в порядке появления.
///
/// Разбор исчерпывающий по вариантам, несущим подвыражения: пропущенный
/// вариант молча потерял бы использование, а на потерянном использовании
/// строится неверный вывод «значение мертво». Поэтому `match` без
/// catch-all по узлам с подвыражениями, и явная ветвь-лист для всего
/// остального.
fn expr_reads(e: &RExpr, out: &mut Vec<u32>) {
    match e {
        RExpr::Local(slot) => out.push(*slot),
        RExpr::Await(x)
        | RExpr::Unary { expr: x, .. }
        | RExpr::NewTypeDescription(x)
        | RExpr::NewTextWriter { path: x, .. }
        | RExpr::DynEval(x)
        | RExpr::Field { obj: x, .. } => expr_reads(x, out),
        RExpr::Binary { lhs, rhs, .. } => {
            expr_reads(lhs, out);
            expr_reads(rhs, out);
        }
        RExpr::Index { obj, index } => {
            expr_reads(obj, out);
            expr_reads(index, out);
        }
        RExpr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_reads(cond, out);
            expr_reads(then_expr, out);
            expr_reads(else_expr, out);
        }
        RExpr::CallMethod { obj, args, .. } => {
            expr_reads(obj, out);
            for a in args {
                expr_reads(a, out);
            }
        }
        RExpr::NewArray { dims: xs }
        | RExpr::NewStructure { values: xs, .. }
        | RExpr::CallBuiltinFn { args: xs, .. }
        | RExpr::CallComponent { args: xs, .. }
        | RExpr::CreateObject { args: xs, .. } => {
            for x in xs {
                expr_reads(x, out);
            }
        }
        RExpr::Call { args, .. } | RExpr::CallImported { args, .. } => {
            for a in args {
                match a {
                    ResolvedArg::Value(v) => expr_reads(v, out),
                    // Пропуск позиции значения не читает.
                    ResolvedArg::Default => {}
                }
            }
        }
        // Листья: ни один слот не читается.
        RExpr::Number(_)
        | RExpr::Date(_)
        | RExpr::Bool(_)
        | RExpr::Str(_)
        | RExpr::Undefined
        | RExpr::Null
        | RExpr::ModuleVar(_)
        | RExpr::ImportedVar(_)
        | RExpr::EnumMember { .. }
        | RExpr::EnumTypeRef(_)
        | RExpr::NewTable
        | RExpr::NewValueComparison
        | RExpr::NewMap => {}
    }
}

/// Слоты, читаемые оператором.
fn stmt_reads(s: &RStmt, out: &mut Vec<u32>) {
    match s {
        RStmt::AssignLocal { value, .. }
        | RStmt::AssignModuleVar { value, .. }
        | RStmt::AssignImportedVar { value, .. }
        | RStmt::ExprStmt(value)
        | RStmt::Execute(value) => expr_reads(value, out),
        RStmt::AssignIndex { obj, index, value } => {
            expr_reads(obj, out);
            expr_reads(index, out);
            expr_reads(value, out);
        }
        RStmt::AssignField { obj, value, .. } => {
            expr_reads(obj, out);
            expr_reads(value, out);
        }
        RStmt::Return(e) | RStmt::Raise(e) => {
            if let Some(e) = e {
                expr_reads(e, out);
            }
        }
        // Границы и коллекция вычисляются один раз до цикла, но приписать
        // их чтение заголовку безопасно: завышение живости стоит регистра,
        // занижение стоило бы неверного кода.
        RStmt::ForNumeric { from, to, .. } => {
            expr_reads(from, out);
            expr_reads(to, out);
        }
        RStmt::ForEach { iter, .. } => expr_reads(iter, out),
        // Управляющие формы до блоков не доходят: их разобрал построитель
        // графа, а условия живут в терминаторах.
        _ => {}
    }
}

/// Слоты, которые оператор записывает.
///
/// `Выполнить` записывает ВСЕ: фрагмент видит область видимости по
/// именам и может присвоить любой локальной. Знать, какой именно, здесь
/// нечем — имена после резолвинга остались только у чанков с
/// `uses_dynamic`, — поэтому единственно верный ответ консервативный.
/// План требует того же: динамический фрагмент уничтожает знание обо
/// всех видимых переменных.
fn stmt_writes(s: &RStmt, n_slots: usize) -> Vec<u32> {
    // Разбор ИСЧЕРПЫВАЮЩИЙ, без ветви-заглушки, и это единственная защита
    // перечня от расползания: новый вид оператора обязан быть ошибкой
    // сборки, а не молча попасть в «ничего не пишет». Пропуск здесь даёт
    // не худший код, а неверный, — так уже случалось трижды.
    let mut out = match s {
        RStmt::AssignLocal { slot, .. } => vec![*slot],
        // Цикл присваивает свою переменную на каждой итерации.
        RStmt::ForNumeric { slot, .. } | RStmt::ForEach { slot, .. } => vec![*slot],
        // Пишут не в локаль: модульный слот, чужой модуль, поле или
        // элемент уже существующего объекта.
        RStmt::AssignModuleVar { .. }
        | RStmt::AssignImportedVar { .. }
        | RStmt::AssignIndex { .. }
        | RStmt::AssignField { .. }
        // Не пишут вовсе; управляющие формы к тому же разобраны графом и
        // до блоков не доходят.
        | RStmt::ExprStmt(_)
        | RStmt::If { .. }
        | RStmt::While { .. }
        | RStmt::Break
        | RStmt::Continue
        | RStmt::Label(_)
        | RStmt::Goto(_)
        | RStmt::Return(_)
        | RStmt::Try { .. }
        | RStmt::Raise(_)
        // `Выполнить` пишет всё, но обеими формами фрагмента ведает
        // `has_dynamic` ниже — здесь он лишь не пишет ничего сам по себе.
        | RStmt::Execute(_) => Vec::new(),
    };
    // Динамический фрагмент пишет ВСЕ слоты, и форм у него две:
    // `Выполнить` как оператор и `Вычислить` как ВЫРАЖЕНИЕ, которое может
    // стоять где угодно — хоть внутри `Возврат`. Вторую форму этот
    // перечень когда-то пропускал, и пропуск нашла не внимательность, а
    // сверка предсказаний с таблицей эффектов байт-кода
    // (`every_slot_the_generator_writes_is_predicted_by_the_analysis`).
    if has_dynamic(s) {
        out.extend(0..n_slots as u32);
    }
    // Аргумент, переданный функции BSL, может быть параметром БЕЗ `Знач`,
    // и тогда вызов пишет прямо в переменную вызывающего. План требует
    // уничтожать знание о таком слоте после вызова, и здесь это сделано
    // консервативно: убивается всякая локальная, стоящая аргументом, без
    // разбора режима параметра.
    //
    // Точный ответ требует таблицы функций, которой у этого модуля нет:
    // `RExpr::Call` несёт номер функции, а режимы её параметров живут в
    // `ResolvedProgram::functions`. Завышение здесь стоит упущенной
    // оптимизации, занижение стоило бы неверного кода — цена
    // несопоставима, и до появления таблицы выбор очевиден.
    //
    // Встроенных функций и методов это не касается: у них параметров без
    // `Знач` нет, о чём резолвер и говорит на месте.
    let mut reads = Vec::new();
    stmt_reads(s, &mut reads);
    let _ = reads;
    let mut killed = Vec::new();
    collect_byref_args(s, &mut killed);
    out.extend(killed);
    out
}

/// Содержит ли оператор динамический фрагмент в любой из двух форм.
fn has_dynamic(s: &RStmt) -> bool {
    if matches!(s, RStmt::Execute(_)) {
        return true;
    }
    let mut exprs = Vec::new();
    stmt_exprs(s, &mut exprs);
    while let Some(e) = exprs.pop() {
        if matches!(e, RExpr::DynEval(_)) {
            return true;
        }
        sub_exprs(e, &mut exprs);
    }
    false
}

/// Локальные, стоящие аргументами вызова функции BSL.
fn collect_byref_args(s: &RStmt, out: &mut Vec<u32>) {
    let mut exprs = Vec::new();
    stmt_exprs(s, &mut exprs);
    while let Some(e) = exprs.pop() {
        match e {
            RExpr::Call { args, .. } | RExpr::CallImported { args, .. } => {
                for a in args {
                    if let ResolvedArg::Value(RExpr::Local(slot)) = a {
                        out.push(*slot);
                    }
                    if let ResolvedArg::Value(v) = a {
                        exprs.push(v);
                    }
                }
            }
            _ => sub_exprs(e, &mut exprs),
        }
    }
}

/// Непосредственные подвыражения узла — для итеративного обхода.
fn sub_exprs<'a>(e: &'a RExpr, out: &mut Vec<&'a RExpr>) {
    match e {
        RExpr::Await(x)
        | RExpr::Unary { expr: x, .. }
        | RExpr::NewTypeDescription(x)
        | RExpr::NewTextWriter { path: x, .. }
        | RExpr::DynEval(x)
        | RExpr::Field { obj: x, .. } => out.push(x),
        RExpr::Binary { lhs, rhs, .. } => {
            out.push(lhs);
            out.push(rhs);
        }
        RExpr::Index { obj, index } => {
            out.push(obj);
            out.push(index);
        }
        RExpr::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            out.push(cond);
            out.push(then_expr);
            out.push(else_expr);
        }
        RExpr::CallMethod { obj, args, .. } => {
            out.push(obj);
            out.extend(args.iter());
        }
        RExpr::NewArray { dims: xs }
        | RExpr::NewStructure { values: xs, .. }
        | RExpr::CallBuiltinFn { args: xs, .. }
        | RExpr::CallComponent { args: xs, .. }
        | RExpr::CreateObject { args: xs, .. } => out.extend(xs.iter()),
        _ => {}
    }
}

/// Выражения верхнего уровня оператора.
fn stmt_exprs<'a>(s: &'a RStmt, out: &mut Vec<&'a RExpr>) {
    match s {
        RStmt::AssignLocal { value, .. }
        | RStmt::AssignModuleVar { value, .. }
        | RStmt::AssignImportedVar { value, .. }
        | RStmt::ExprStmt(value)
        | RStmt::Execute(value) => out.push(value),
        RStmt::AssignIndex { obj, index, value } => {
            out.push(obj);
            out.push(index);
            out.push(value);
        }
        RStmt::AssignField { obj, value, .. } => {
            out.push(obj);
            out.push(value);
        }
        RStmt::Return(e) | RStmt::Raise(e) => out.extend(e.iter()),
        _ => {}
    }
}

/// Строит SSA для тела, у которого `n_slots` локальных слотов.
///
/// # Panics
///
/// Не паникует: недостижимые блоки и неизвестные слоты выражаются
/// значениями, а не отказом.
#[must_use]
pub fn build(cfg: &Cfg<'_>, n_slots: usize) -> Ssa {
    let nb = cfg.blocks.len();
    let idom = cfg.immediate_dominators();
    let df = dominance_frontiers(cfg, &idom);
    let rpo = cfg.reverse_postorder();
    let reachable: Vec<bool> = {
        let mut r = vec![false; nb];
        for &b in &rpo {
            r[b] = true;
        }
        r
    };

    let mut values = vec![Value::Bottom];
    let bottom: ValueId = 0;

    // --- размещение φ: рабочий список по блокам определения слота ------
    let mut defs_of: Vec<Vec<BlockId>> = vec![Vec::new(); n_slots];
    for (b, block) in cfg.blocks.iter().enumerate() {
        for s in &block.stmts {
            for slot in stmt_writes(s, n_slots) {
                if (slot as usize) < n_slots && !defs_of[slot as usize].contains(&b) {
                    defs_of[slot as usize].push(b);
                }
            }
        }
    }
    let mut phis: Vec<Vec<ValueId>> = vec![Vec::new(); nb];
    let mut phi_slot: Vec<Vec<u32>> = vec![Vec::new(); nb];
    for (slot, sites) in defs_of.iter().enumerate() {
        let mut work: Vec<BlockId> = sites.clone();
        let mut placed: Vec<bool> = vec![false; nb];
        let mut seen: Vec<bool> = vec![false; nb];
        for &b in sites {
            seen[b] = true;
        }
        while let Some(b) = work.pop() {
            for &d in &df[b] {
                if placed[d] || !reachable[d] {
                    continue;
                }
                placed[d] = true;
                let id = values.len();
                values.push(Value::Phi {
                    block: d,
                    slot: slot as u32,
                    operands: Vec::new(),
                });
                phis[d].push(id);
                phi_slot[d].push(slot as u32);
                if !seen[d] {
                    seen[d] = true;
                    work.push(d);
                }
            }
        }
    }

    // --- переименование: обход дерева доминаторов явным стеком ---------
    let mut children: Vec<Vec<BlockId>> = vec![Vec::new(); nb];
    for (b, dom) in idom.iter().enumerate() {
        if let Some(d) = *dom
            && d != b
        {
            children[d].push(b);
        }
    }
    let entry_state: Vec<ValueId> = (0..n_slots)
        .map(|slot| {
            let id = values.len();
            values.push(Value::Entry { slot: slot as u32 });
            id
        })
        .collect();

    let mut defs: Vec<Vec<(usize, ValueId)>> = vec![Vec::new(); nb];
    let mut uses: Vec<Use> = Vec::new();
    let mut entry: Vec<Option<Vec<ValueId>>> = vec![None; nb];
    let mut exit: Vec<Option<Vec<ValueId>>> = vec![None; nb];
    let mut stack: Vec<(BlockId, Vec<ValueId>)> = vec![(cfg.entry, entry_state)];
    while let Some((b, incoming)) = stack.pop() {
        let mut state = incoming;
        for (i, &phi) in phis[b].iter().enumerate() {
            state[phi_slot[b][i] as usize] = phi;
        }
        entry[b] = Some(state.clone());
        for (i, s) in cfg.blocks[b].stmts.iter().enumerate() {
            // Чтения собираются ДО записи: правая часть присваивания
            // видит прежнее значение слота, а не своё собственное.
            let mut read = Vec::new();
            stmt_reads(s, &mut read);
            if matches!(s, RStmt::Execute(_)) {
                // Фрагмент читает область видимости по именам — считаем
                // прочитанным всё, иначе значение объявилось бы мёртвым.
                read.extend(0..n_slots as u32);
            }
            for slot in read {
                if (slot as usize) < n_slots {
                    uses.push(Use {
                        block: b,
                        stmt: Some(i),
                        value: state[slot as usize],
                        slot,
                    });
                }
            }
            for slot in stmt_writes(s, n_slots) {
                if (slot as usize) < n_slots {
                    let id = values.len();
                    values.push(Value::Def { block: b, slot });
                    state[slot as usize] = id;
                    defs[b].push((i, id));
                }
            }
        }
        // Выражение терминатора читается после всех операторов блока.
        //
        // Разбор ИСЧЕРПЫВАЮЩИЙ, и это не педантизм: пока сюда попадал один
        // лишь `Branch`, чтения из `Возврат` были невидимы, и значение,
        // использованное только в возврате, выглядело мёртвым. Так три
        // константы `getConst` получили общий регистр.
        let term_expr = match &cfg.blocks[b].term {
            crate::cfg::Terminator::Branch { cond, .. } => *cond,
            crate::cfg::Terminator::Return(e) | crate::cfg::Terminator::Raise(e) => *e,
            crate::cfg::Terminator::Goto(_) | crate::cfg::Terminator::Exit => None,
        };
        if let Some(c) = term_expr {
            let mut read = Vec::new();
            expr_reads(c, &mut read);
            for slot in read {
                if (slot as usize) < n_slots {
                    uses.push(Use {
                        block: b,
                        stmt: None,
                        value: state[slot as usize],
                        slot,
                    });
                }
            }
        }
        exit[b] = Some(state.clone());
        for &c in &children[b] {
            stack.push((c, state.clone()));
        }
    }

    // --- операнды φ: по одному на предшественника, в его порядке -------
    let phi_ids: Vec<ValueId> = phis.iter().flatten().copied().collect();
    for id in phi_ids {
        let Value::Phi { block, slot, .. } = values[id].clone() else {
            continue;
        };
        let operands: Vec<ValueId> = cfg.blocks[block]
            .preds
            .iter()
            .map(|&p| match &exit[p] {
                // Недостижимый предшественник даёт `Bottom`, а не
                // «неизвестно»: пути оттуда нет.
                None => bottom,
                Some(st) => st[slot as usize],
            })
            .collect();
        if let Value::Phi { operands: o, .. } = &mut values[id] {
            *o = operands;
        }
    }

    Ssa {
        values,
        phis,
        defs,
        uses,
        entry,
        exit,
        bottom,
    }
}

/// Проверка инвариантов SSA, независимая от построителя.
///
/// # Errors
///
/// Описание первого нарушенного инварианта.
pub fn verify(cfg: &Cfg<'_>, ssa: &Ssa) -> Result<(), String> {
    let idom = cfg.immediate_dominators();
    // Единственность определения: каждое значение объявлено ровно один
    // раз, и его номер совпадает с местом в таблице.
    for (id, v) in ssa.values.iter().enumerate() {
        if let Value::Phi {
            block, operands, ..
        } = v
        {
            if !ssa.phis[*block].contains(&id) {
                return Err(format!("φ {id} не числится в блоке {block}"));
            }
            let n = cfg.blocks[*block].preds.len();
            if operands.len() != n {
                return Err(format!(
                    "φ {id} в блоке {block}: операндов {}, предшественников {n}",
                    operands.len()
                ));
            }
            for (k, &op) in operands.iter().enumerate() {
                if op >= ssa.values.len() {
                    return Err(format!("φ {id}: операнд {op} вне таблицы"));
                }
                let pred = cfg.blocks[*block].preds[k];
                // Операнд с недостижимого ребра обязан быть `Bottom`, а
                // достижимого — нет: иначе `φ` утверждала бы отсутствие
                // значения там, где путь есть.
                let pred_reachable = ssa.exit[pred].is_some();
                if pred_reachable == (op == ssa.bottom) {
                    return Err(format!(
                        "φ {id}, предшественник {pred}: достижимость {pred_reachable} \
                         не согласована с Bottom"
                    ));
                }
            }
        }
    }
    for (b, dom) in idom.iter().enumerate() {
        let has_state = ssa.exit[b].is_some();
        let dominated = b == cfg.entry || dom.is_some();
        if has_state != dominated {
            return Err(format!(
                "блок {b}: состояние {has_state} не согласовано с доминатором"
            ));
        }
    }

    // ГЛАВНЫЙ инвариант SSA: определение доминирует над КАЖДЫМ своим
    // использованием. Без него представление называлось бы SSA, ничего
    // не гарантируя: значение, определённое в одной ветви, оказалось бы
    // видно в другой.
    //
    // Проверяется по настоящим использованиям, а не только по значениям
    // на входе в блок. Вход — лишь следствие: он говорит, что значение
    // ДОШЛО, но не что его кто-то читает, и на нём инвариант звучал бы
    // слабее, чем называется.
    for u in &ssa.uses {
        let Some(def) = defining_block(&ssa.values[u.value]) else {
            continue;
        };
        if !dominates(&idom, cfg.entry, def, u.block) {
            return Err(format!(
                "использование значения {} в блоке {} (оператор {:?}): определено в {def}, который его не доминирует",
                u.value, u.block, u.stmt
            ));
        }
    }

    // Вход блока — тем же правилом: дошедшее значение обязано быть
    // определено доминирующим блоком, даже если здесь его не читают.
    for (b, state) in ssa.entry.iter().enumerate() {
        let Some(state) = state else { continue };
        for (slot, &v) in state.iter().enumerate() {
            let Some(def) = defining_block(&ssa.values[v]) else {
                // `Entry` и `Bottom` блока определения не имеют: первое
                // приходит извне тела, второго не существует вовсе.
                continue;
            };
            if !dominates(&idom, cfg.entry, def, b) {
                return Err(format!(
                    "блок {b}, слот {slot}: значение {v} определено в {def},                      который его не доминирует"
                ));
            }
        }
    }
    Ok(())
}

/// Блок, в котором значение определено, или `None` у `Entry` и `Bottom`.
fn defining_block(v: &Value) -> Option<BlockId> {
    match v {
        Value::Def { block, .. } | Value::Phi { block, .. } => Some(*block),
        Value::Entry { .. } | Value::Bottom => None,
    }
}

/// Доминирует ли `a` над `b`: подъём по цепочке непосредственных
/// доминаторов, итеративно — цепочка длиной с граф.
fn dominates(idom: &[Option<BlockId>], entry: BlockId, a: BlockId, b: BlockId) -> bool {
    let mut cur = b;
    loop {
        if cur == a {
            return true;
        }
        if cur == entry {
            return false;
        }
        match idom[cur] {
            Some(next) if next != cur => cur = next,
            _ => return false,
        }
    }
}

// ---------------------------------------------------------------------
// Распространение констант по φ
// ---------------------------------------------------------------------

/// Решётка констант: `Bottom` ⊑ число ⊑ `Top`.
///
/// Только числа — по той же причине, по которой их одни сворачивает и
/// кодоген: приведение строк, дат и булевых живёт в обёртках `bsl-vm`, и
/// вторая редакция правил приведения здесь была бы дефектом, а не
/// расширением.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    /// Значение ещё не достигнуто либо путь до него недостижим. Это НЕ
    /// «неизвестно»: `Bottom` — единица объединения, `Top` — его
    /// поглощающий элемент, и слияние с недостижимой ветви обязано
    /// оставлять константу константой.
    Bottom,
    Number(bsl_number::BslNumber),
    /// Константой не является.
    Top,
}

impl Const {
    /// Объединение по решётке. Монотонно и без цикла: значение движется
    /// только вверх, `Bottom` -> число -> `Top`, и вернуться не может.
    #[must_use]
    pub fn meet(&self, other: &Const) -> Const {
        match (self, other) {
            (Const::Bottom, x) | (x, Const::Bottom) => x.clone(),
            (Const::Top, _) | (_, Const::Top) => Const::Top,
            (Const::Number(a), Const::Number(b)) => {
                if a == b {
                    Const::Number(a.clone())
                } else {
                    Const::Top
                }
            }
        }
    }
}

/// Вычисляет выражение в решётке при известных значениях слотов.
fn eval(e: &bsl_sema::RExpr, slots: &[Const]) -> Const {
    use bsl_rt::BslValue;
    use bsl_sema::RExpr;
    let num = |c: &Const| match c {
        Const::Number(n) => Some(BslValue::Number(n.clone())),
        _ => None,
    };
    let back = |v: bsl_rt::RtResult<BslValue>| match v {
        Ok(BslValue::Number(n)) => Const::Number(n),
        // Операция с ошибкой константы не даёт: `1 / 0` обязано бросить на
        // исполнении, а не превратиться в значение решётки.
        _ => Const::Top,
    };
    match e {
        RExpr::Number(n) => Const::Number(n.clone()),
        RExpr::Local(slot) => slots.get(*slot as usize).cloned().unwrap_or(Const::Top),
        RExpr::Unary {
            op: bsl_syntax::UnaryOp::Neg,
            expr,
        } => match num(&eval(expr, slots)) {
            Some(v) => back(v.neg()),
            None => Const::Top,
        },
        RExpr::Binary { op, lhs, rhs } => {
            let (Some(a), Some(b)) = (num(&eval(lhs, slots)), num(&eval(rhs, slots))) else {
                return Const::Top;
            };
            use bsl_syntax::BinaryOp;
            match op {
                BinaryOp::Add => back(a.add(&b)),
                BinaryOp::Sub => back(a.sub(&b)),
                BinaryOp::Mul => back(a.mul(&b)),
                BinaryOp::Div => back(a.div(&b)),
                BinaryOp::Mod => back(a.rem(&b)),
                _ => Const::Top,
            }
        }
        _ => Const::Top,
    }
}

/// Распространяет константы по значениям SSA до неподвижной точки.
///
/// Возвращает решётку для каждого значения. Это первый потребитель
/// представления и первая вещь, которой блочно-локальный проход над
/// байт-кодом сделать не может: константа переживает слияние ветвей, если
/// обе дают одно значение, и переживает слияние с НЕДОСТИЖИМОЙ ветвью,
/// потому что `Bottom` — единица объединения.
///
/// Обход по рабочему списку блоков: рекурсии по графу здесь нет, как и
/// везде в этом модуле.
#[must_use]
pub fn propagate_constants(cfg: &Cfg<'_>, ssa: &Ssa, n_slots: usize) -> Vec<Const> {
    let mut lat = vec![Const::Bottom; ssa.values.len()];
    // Значение на входе в тело — параметр либо ещё не присвоенная
    // переменная; ни то, ни другое константой не считается.
    for (id, v) in ssa.values.iter().enumerate() {
        if matches!(v, Value::Entry { .. }) {
            lat[id] = Const::Top;
        }
    }

    let rpo = cfg.reverse_postorder();
    let mut changed = true;
    while changed {
        changed = false;
        for &b in &rpo {
            let Some(entry) = &ssa.entry[b] else { continue };
            // φ блока: объединение операндов по предшественникам.
            for &phi in &ssa.phis[b] {
                let Value::Phi { operands, .. } = &ssa.values[phi] else {
                    continue;
                };
                let mut merged = Const::Bottom;
                for &op in operands {
                    merged = merged.meet(&lat[op]);
                }
                if lat[phi] != merged {
                    lat[phi] = merged;
                    changed = true;
                }
            }
            // Состояние слотов на входе, затем по операторам блока.
            let mut slots: Vec<Const> = (0..n_slots).map(|i| lat[entry[i]].clone()).collect();
            for &(stmt_index, id) in &ssa.defs[b] {
                // Определение, пришедшее НЕ от присваивания, — это
                // `Выполнить`: он пишет слот по имени, и чем именно,
                // отсюда не видно. Такое значение обязано быть `Top`, а
                // не `Bottom`: `Bottom` — единица объединения, и слияние
                // с ним сохранило бы константу, которой фрагмент уже нет.
                let (slot, c) = match cfg.blocks[b].stmts[stmt_index] {
                    bsl_sema::RStmt::AssignLocal { slot, value } => (slot, eval(value, &slots)),
                    _ => match &ssa.values[id] {
                        Value::Def { slot, .. } => (slot, Const::Top),
                        _ => continue,
                    },
                };
                if lat[id] != c {
                    lat[id] = c.clone();
                    changed = true;
                }
                if let Some(cell) = slots.get_mut(*slot as usize) {
                    *cell = c;
                }
            }
        }
    }
    lat
}

/// Известные значения слотов ПЕРЕД каждым оператором и перед условием
/// каждого терминатора.
///
/// Ключ — адрес узла разрешённого дерева. Приём тот же, что у памяти об
/// отказах в свёртке: дерево живёт неизменным всю компиляцию, узлы не
/// переезжают и не освобождаются, поэтому адрес — их устойчивое имя.
/// Кодогену этого достаточно: он обходит то же самое дерево и спрашивает
/// про тот же узел.
#[must_use]
pub fn constants_at_nodes(
    cfg: &Cfg<'_>,
    ssa: &Ssa,
    lat: &[Const],
    n_slots: usize,
) -> std::collections::HashMap<usize, Vec<Const>> {
    let mut out = std::collections::HashMap::new();
    for (b, entry) in ssa.entry.iter().enumerate() {
        let Some(entry) = entry else { continue };
        let mut slots: Vec<Const> = (0..n_slots).map(|i| lat[entry[i]].clone()).collect();
        for (i, s) in cfg.blocks[b].stmts.iter().enumerate() {
            out.insert(std::ptr::from_ref(*s) as usize, slots.clone());
            // После оператора состояние слотов меняется — ровно так же,
            // как его считало распространение.
            for &(stmt_index, id) in &ssa.defs[b] {
                if stmt_index != i {
                    continue;
                }
                if let Value::Def { slot, .. } = &ssa.values[id]
                    && let Some(cell) = slots.get_mut(*slot as usize)
                {
                    *cell = lat[id].clone();
                }
            }
        }
        // Выражения терминаторов ключуются по себе: их операторов в
        // `stmts` нет, а состояние для них — то, что осталось после всех
        // операторов блока.
        match &cfg.blocks[b].term {
            crate::cfg::Terminator::Branch { cond: Some(c), .. }
            | crate::cfg::Terminator::Return(Some(c))
            | crate::cfg::Terminator::Raise(Some(c)) => {
                out.insert(std::ptr::from_ref(*c) as usize, slots);
            }
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------------
// Домен представлений: ярус значения
// ---------------------------------------------------------------------

/// Доказанное внутреннее представление числа.
///
/// Основание шага 8 плана. Программа по-прежнему видит единый тип
/// `Число`; ярус — это утверждение компилятора о том, каким хранилищем
/// значение выражается точно.
///
/// Диапазон здесь обязателен, а не украшение. `Int64` без него не
/// доказывает ничего: сумма двух чисел, помещающихся в `i64`, в `i64` уже
/// может не поместиться, и план прямо запрещает молчаливое заворачивание.
/// Поэтому ярус несёт границы, а операция сохраняет его только когда
/// границы доказывают безопасность.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tier {
    /// Значение не достигнуто либо путь недостижим.
    Bottom,
    /// Целое с масштабом ноль, лежащее в `[lo, hi]` и, значит, в `i64`.
    Int64 { lo: i64, hi: i64 },
    /// Доказано, что число, но представление — нет.
    Number,
    /// Не доказано даже, что число.
    Top,
}

impl Tier {
    /// Объединение по решётке: `Bottom` — единица, `Top` — поглощающий.
    #[must_use]
    pub fn meet(&self, other: &Tier) -> Tier {
        match (self, other) {
            (Tier::Bottom, x) | (x, Tier::Bottom) => x.clone(),
            (Tier::Top, _) | (_, Tier::Top) => Tier::Top,
            (Tier::Int64 { lo: a, hi: b }, Tier::Int64 { lo: c, hi: d }) => Tier::Int64 {
                lo: *a.min(c),
                hi: *b.max(d),
            },
            // Число и целое дают число: общего представления у них нет.
            _ => Tier::Number,
        }
    }

    /// Расширение: обязательная операция, без которой решётка НЕ СХОДИТСЯ.
    ///
    /// Цепь ярусов конечна только по видам (`Bottom` -> целое -> число ->
    /// `Top`), но у целого есть ещё и диапазон, и в цикле он растёт на
    /// каждой итерации: 0..0, 0..1, 0..2 и так до бесконечности. Поэтому
    /// расширившийся диапазон не уточняется, а поднимается до `Число`:
    /// доказать ярус целого для счётчика без анализа границ цикла нечем,
    /// а бесконечно приближаться к ответу — не анализ.
    #[must_use]
    pub fn widen(old: &Tier, new: &Tier) -> Tier {
        match (old, new) {
            (Tier::Int64 { lo: a, hi: b }, Tier::Int64 { lo: c, hi: d }) if c < a || d > b => {
                Tier::Number
            }
            _ => new.clone(),
        }
    }

    /// Ярус готовой константы.
    #[must_use]
    pub fn of(c: &Const) -> Tier {
        match c {
            Const::Bottom => Tier::Bottom,
            Const::Top => Tier::Top,
            Const::Number(n) => match n.to_i64_exact() {
                Some(v) => Tier::Int64 { lo: v, hi: v },
                None => Tier::Number,
            },
        }
    }
}

/// Арифметика над диапазонами. `None` — за пределами `i64`, и тогда ярус
/// опускается до `Number`: сузить его было бы тем самым молчаливым
/// заворачиванием, которое план запрещает.
fn range_op(op: bsl_syntax::BinaryOp, a: (i64, i64), b: (i64, i64)) -> Option<(i64, i64)> {
    use bsl_syntax::BinaryOp;
    let (al, ah) = (i128::from(a.0), i128::from(a.1));
    let (bl, bh) = (i128::from(b.0), i128::from(b.1));
    let (lo, hi) = match op {
        BinaryOp::Add => (al + bl, ah + bh),
        BinaryOp::Sub => (al - bh, ah - bl),
        BinaryOp::Mul => {
            let c = [al * bl, al * bh, ah * bl, ah * bh];
            (*c.iter().min()?, *c.iter().max()?)
        }
        // Деление BSL — точное десятичное, и целое от него не гарантировано
        // даже при целых операндах: `1 / 2` даёт `0,5`. Ярус целого здесь
        // не доказывается.
        _ => return None,
    };
    let (min, max) = (i128::from(i64::MIN), i128::from(i64::MAX));
    if lo < min || hi > max {
        return None;
    }
    Some((lo as i64, hi as i64))
}

/// Ярус выражения при известных ярусах слотов.
fn tier_of(e: &RExpr, slots: &[Tier]) -> Tier {
    match e {
        RExpr::Number(n) => match n.to_i64_exact() {
            Some(v) => Tier::Int64 { lo: v, hi: v },
            None => Tier::Number,
        },
        RExpr::Local(slot) => slots.get(*slot as usize).cloned().unwrap_or(Tier::Top),
        RExpr::Unary {
            op: bsl_syntax::UnaryOp::Neg,
            expr,
        } => match tier_of(expr, slots) {
            Tier::Int64 { lo, hi } => match (lo.checked_neg(), hi.checked_neg()) {
                (Some(a), Some(b)) => Tier::Int64 {
                    lo: a.min(b),
                    hi: a.max(b),
                },
                _ => Tier::Number,
            },
            Tier::Number => Tier::Number,
            other => other,
        },
        RExpr::Binary { op, lhs, rhs } => {
            let (a, b) = (tier_of(lhs, slots), tier_of(rhs, slots));
            match (&a, &b) {
                (Tier::Int64 { lo: al, hi: ah }, Tier::Int64 { lo: bl, hi: bh }) => {
                    match range_op(*op, (*al, *ah), (*bl, *bh)) {
                        Some((lo, hi)) => Tier::Int64 { lo, hi },
                        // Границы безопасности не доказали — остаётся
                        // число без доказанного представления.
                        None => Tier::Number,
                    }
                }
                (Tier::Bottom, _) | (_, Tier::Bottom) => Tier::Bottom,
                (Tier::Top, _) | (_, Tier::Top) => Tier::Top,
                _ => Tier::Number,
            }
        }
        _ => Tier::Top,
    }
}

/// Распространяет ярусы представлений по значениям SSA.
///
/// Устроено как и распространение констант: рабочий список блоков,
/// объединение в `φ`, без рекурсии по графу. Опкодов эта решётка не
/// добавляет и байт-кода не меняет — план запрещает вводить
/// типизированные опкоды раньше, чем появится доказательная база, и это
/// её статическая половина.
#[must_use]
pub fn propagate_tiers(cfg: &Cfg<'_>, ssa: &Ssa, n_slots: usize) -> Vec<Tier> {
    let mut tier = vec![Tier::Bottom; ssa.values.len()];
    for (id, v) in ssa.values.iter().enumerate() {
        if matches!(v, Value::Entry { .. }) {
            tier[id] = Tier::Top;
        }
    }
    let rpo = cfg.reverse_postorder();
    let mut changed = true;
    while changed {
        changed = false;
        for &b in &rpo {
            let Some(entry) = &ssa.entry[b] else { continue };
            for &phi in &ssa.phis[b] {
                let Value::Phi { operands, .. } = &ssa.values[phi] else {
                    continue;
                };
                let mut merged = Tier::Bottom;
                for &op in operands {
                    merged = merged.meet(&tier[op]);
                }
                let merged = Tier::widen(&tier[phi], &merged);
                if tier[phi] != merged {
                    tier[phi] = merged;
                    changed = true;
                }
            }
            let mut slots: Vec<Tier> = (0..n_slots).map(|i| tier[entry[i]].clone()).collect();
            for &(stmt_index, id) in &ssa.defs[b] {
                let (slot, t) = match cfg.blocks[b].stmts[stmt_index] {
                    RStmt::AssignLocal { slot, value } => (slot, tier_of(value, &slots)),
                    _ => match &ssa.values[id] {
                        Value::Def { slot, .. } => (slot, Tier::Top),
                        _ => continue,
                    },
                };
                let t = Tier::widen(&tier[id], &t);
                if tier[id] != t {
                    tier[id] = t.clone();
                    changed = true;
                }
                if let Some(cell) = slots.get_mut(*slot as usize) {
                    *cell = t;
                }
            }
        }
    }
    tier
}
