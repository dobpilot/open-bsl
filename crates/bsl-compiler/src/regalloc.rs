//! Живые диапазоны значений SSA и раскладка их по регистрам.
//!
//! Последняя аналитическая ступень конвейера шага 5
//! (`docs/research/performance/ssa-hotspot-analysis.md`). Как и граф с
//! самой SSA, ничего в выпускаемом байт-коде не меняет: раскладка
//! СЧИТАЕТСЯ и ПРОВЕРЯЕТСЯ, но кодоген по-прежнему раздаёт регистры сам.
//! Разделение намеренное — план требует от шага 5 работы без изменения
//! байт-кода, и переключение кодогена остаётся отдельным шагом со своими
//! воротами.
//!
//! # Почему пересечение считается по точкам, а не по блокам
//!
//! Блочная грубость была бы безопасна — она только ЗАВЫШАЕТ пересечение,
//! а значит требует лишних регистров, но никогда не сливает конфликтующие
//! значения. Однако кадр вмещает 255 регистров, и завышение упирается в
//! этот предел на ровном месте: у `csv_write` в одном блоке живут десятки
//! временных. Поэтому живость ведётся по операторам внутри блока, обратным
//! проходом, и значение конфликтует ровно с тем, что живо в точке его
//! определения.

use crate::cfg::Cfg;
use crate::ssa::{Ssa, Value, ValueId};
use std::collections::{HashMap, HashSet};

/// Раскладка: регистр на каждое значение.
pub struct Allocation {
    /// Регистр значения; `None` у `Bottom` — его не существует.
    pub reg: Vec<Option<u8>>,
    /// Сколько регистров потребовалось.
    pub used: usize,
}

/// Значения, живые на выходе из каждого блока.
///
/// Обратный поток до неподвижной точки, итеративно: рекурсии по графу
/// здесь нет, как и во всём этом конвейере.
#[must_use]
pub fn live_out(cfg: &Cfg<'_>, ssa: &Ssa) -> Vec<HashSet<ValueId>> {
    let nb = cfg.blocks.len();
    // Использования по блокам и по операторам — чтобы не перебирать весь
    // список на каждой итерации.
    let mut used_in: Vec<Vec<ValueId>> = vec![Vec::new(); nb];
    for u in &ssa.uses {
        used_in[u.block].push(u.value);
    }
    // Операнд `φ` живёт до КОНЦА соответствующего предшественника, а не
    // до входа в блок с `φ`: именно там он и передаётся.
    let mut phi_in_pred: Vec<Vec<ValueId>> = vec![Vec::new(); nb];
    for (b, phis) in ssa.phis.iter().enumerate() {
        for &phi in phis {
            let Value::Phi { operands, .. } = &ssa.values[phi] else {
                continue;
            };
            for (k, &op) in operands.iter().enumerate() {
                let pred = cfg.blocks[b].preds[k];
                phi_in_pred[pred].push(op);
            }
        }
    }

    let mut live_in: Vec<HashSet<ValueId>> = vec![HashSet::new(); nb];
    let mut out: Vec<HashSet<ValueId>> = vec![HashSet::new(); nb];
    let rpo = cfg.reverse_postorder();
    let mut changed = true;
    while changed {
        changed = false;
        for &b in rpo.iter().rev() {
            let mut o: HashSet<ValueId> = HashSet::new();
            for s in cfg.succs(b) {
                o.extend(live_in[s].iter().copied());
            }
            o.extend(phi_in_pred[b].iter().copied());
            let mut i = o.clone();
            // `φ` блока определяются на его входе — из живого на входе
            // они уходят.
            for &phi in &ssa.phis[b] {
                i.remove(&phi);
            }
            for &(_, id) in &ssa.defs[b] {
                i.remove(&id);
            }
            i.extend(used_in[b].iter().copied());
            if i != live_in[b] || o != out[b] {
                live_in[b] = i;
                out[b] = o;
                changed = true;
            }
        }
    }
    out
}

/// Проходит все точки программы, отдавая множество живых значений в
/// каждой из них.
///
/// Одна редакция правила «что живо где» на двух потребителей: раскладку и
/// её проверку. Второй экземпляр разъехался бы с первым, и проверка стала
/// бы подтверждать не то, что построено.
fn for_each_point<F: FnMut(&HashSet<ValueId>)>(cfg: &Cfg<'_>, ssa: &Ssa, mut f: F) {
    let out = live_out(cfg, ssa);
    let mut uses_at: HashMap<(usize, Option<usize>), Vec<ValueId>> = HashMap::new();
    for u in &ssa.uses {
        uses_at.entry((u.block, u.stmt)).or_default().push(u.value);
    }

    for (b, block_out) in out.iter().enumerate() {
        if ssa.entry[b].is_none() {
            continue;
        }
        let mut live: HashSet<ValueId> = block_out.clone();
        // Условие терминатора читается последним — значит живо позже
        // всех операторов блока.
        if let Some(vs) = uses_at.get(&(b, None)) {
            live.extend(vs.iter().copied());
        }
        f(&live);
        // Обратный проход: сначала снимаем определение оператора (до него
        // значение не жило), затем добавляем то, что он читает.
        for (i, s) in cfg.blocks[b].stmts.iter().enumerate().rev() {
            let _ = s;
            for &(stmt_index, id) in &ssa.defs[b] {
                if stmt_index == i {
                    live.remove(&id);
                }
            }
            f(&live);
            if let Some(vs) = uses_at.get(&(b, Some(i))) {
                live.extend(vs.iter().copied());
            }
            f(&live);
        }
        // На входе в блок живы его `φ` и всё, что дошло снаружи.
        for &phi in &ssa.phis[b] {
            live.remove(&phi);
        }
        f(&live);
    }
}

/// Строит раскладку: значения, живые одновременно, получают разные
/// регистры.
///
/// # Errors
///
/// Сообщение, если значений, живых одновременно, больше, чем регистров в
/// кадре. Это честный отказ, а не молчаливое переиспользование: кадр
/// вмещает 255, и кодоген на этом же пределе отказывает уже сегодня.
pub fn allocate(cfg: &Cfg<'_>, ssa: &Ssa) -> Result<Allocation, String> {
    // Пересечения: всё, что живо в одной точке, конфликтует попарно.
    let mut interfere: HashMap<ValueId, HashSet<ValueId>> = HashMap::new();
    for_each_point(cfg, ssa, |live| {
        let vs: Vec<ValueId> = live.iter().copied().collect();
        for (i, &a) in vs.iter().enumerate() {
            for &b in &vs[i + 1..] {
                interfere.entry(a).or_default().insert(b);
                interfere.entry(b).or_default().insert(a);
            }
        }
    });

    // Жадная раскраска в порядке номеров значений: детерминированно и
    // без эвристик, которых пока нечем обосновать.
    let mut reg: Vec<Option<u8>> = vec![None; ssa.values.len()];
    let mut used = 0usize;
    for id in 0..ssa.values.len() {
        if matches!(ssa.values[id], Value::Bottom) {
            continue;
        }
        let taken: HashSet<u8> = interfere
            .get(&id)
            .map(|s| s.iter().filter_map(|&o| reg[o]).collect())
            .unwrap_or_default();
        let Some(free) = (0..=u8::MAX).find(|r| !taken.contains(r)) else {
            return Err(format!(
                "значению {id} не нашлось регистра: живых одновременно больше 255"
            ));
        };
        reg[id] = Some(free);
        used = used.max(free as usize + 1);
    }
    Ok(Allocation { reg, used })
}

/// Проверка раскладки, независимая от того, кто её строил.
///
/// # Errors
///
/// Описание первого нарушения: два одновременно живых значения в одном
/// регистре либо значение без регистра.
pub fn verify(cfg: &Cfg<'_>, ssa: &Ssa, alloc: &Allocation) -> Result<(), String> {
    for (id, v) in ssa.values.iter().enumerate() {
        if matches!(v, Value::Bottom) {
            continue;
        }
        if alloc.reg[id].is_none() {
            return Err(format!("значение {id} осталось без регистра"));
        }
    }
    // Проверяется СВОЙСТВО, а не построение: в каждой точке программы у
    // всех живых значений регистры различны. Как раскладка к этому
    // пришла — её дело.
    let mut bad: Option<String> = None;
    for_each_point(cfg, ssa, |live| {
        if bad.is_some() {
            return;
        }
        let mut seen: HashMap<u8, ValueId> = HashMap::new();
        for &id in live {
            let Some(r) = alloc.reg[id] else { continue };
            if let Some(&other) = seen.get(&r) {
                bad = Some(format!(
                    "значения {id} и {other} живы одновременно и делят регистр {r}"
                ));
                return;
            }
            seen.insert(r, id);
        }
    });
    match bad {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
