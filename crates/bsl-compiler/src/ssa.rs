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
use bsl_sema::RStmt;

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

pub struct Ssa {
    pub values: Vec<Value>,
    /// `φ` каждого блока.
    pub phis: Vec<Vec<ValueId>>,
    /// Определения внутри блока: номер оператора в `Block::stmts` и
    /// значение, которое он записал. Порядок — порядок операторов.
    pub defs: Vec<Vec<(usize, ValueId)>>,
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
        for &p in &cfg.blocks[b].preds {
            let mut runner = p;
            // Недостижимый предшественник цепочки доминаторов не имеет —
            // подниматься от него некуда.
            while idom[runner].is_some() && Some(runner) != idom[b] && runner != b {
                if !df[runner].contains(&b) {
                    df[runner].push(b);
                }
                let Some(next) = idom[runner] else { break };
                runner = next;
            }
        }
    }
    df
}

/// Слот, который оператор записывает.
fn stmt_write(s: &RStmt) -> Option<u32> {
    match s {
        RStmt::AssignLocal { slot, .. } => Some(*slot),
        _ => None,
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
            if let Some(slot) = stmt_write(s)
                && (slot as usize) < n_slots
                && !defs_of[slot as usize].contains(&b)
            {
                defs_of[slot as usize].push(b);
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
            if let Some(slot) = stmt_write(s)
                && (slot as usize) < n_slots
            {
                let id = values.len();
                values.push(Value::Def { block: b, slot });
                state[slot as usize] = id;
                defs[b].push((i, id));
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

    // ГЛАВНЫЙ инвариант SSA: значение, входящее в блок, определено там,
    // откуда путь в этот блок проходит обязательно. Без него
    // представление называлось бы SSA, ничего не гарантируя: значение,
    // определённое в одной ветви, оказалось бы видно в другой.
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
                let bsl_sema::RStmt::AssignLocal { slot, value } = cfg.blocks[b].stmts[stmt_index]
                else {
                    continue;
                };
                let c = eval(value, &slots);
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
