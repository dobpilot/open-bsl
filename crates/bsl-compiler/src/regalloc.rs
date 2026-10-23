//! Живые диапазоны значений SSA и раскладка их по регистрам.
//!
//! Последняя ступень конвейера шага 5
//! (`docs/research/performance/ssa-hotspot-analysis.md`). В отличие от
//! графа и самой SSA, эта ступень МЕНЯЕТ выпускаемый байт-код: кодоген
//! применяет раскладку под ключом `--optimize=ssa-regalloc`. Ворот она не
//! проходила и по умолчанию выключена, в том числе не входит в голый
//! `--optimize`: проход, меняющий семантику там, где его модель неполна,
//! не должен попадать во «все».
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
        // Определение внутри блока нельзя возвращать в его `live_in`:
        // до входа в блок этого значения ещё не существует. Раньше
        // общий список чтений снова добавлял его после снятия `defs`.
        if !matches!(ssa.values[u.value], Value::Def { block, .. } | Value::Phi { block, .. } if block == u.block)
        {
            used_in[u.block].push(u.value);
        }
    }
    let live_values = live_phi_dependencies(ssa);
    // Операнд живого `φ` живёт до КОНЦА соответствующего предшественника,
    // а не до входа в блок с `φ`: именно там он и передаётся.
    let mut phi_in_pred: Vec<Vec<ValueId>> = vec![Vec::new(); nb];
    for (b, phis) in ssa.phis.iter().enumerate() {
        for &phi in phis {
            if !live_values.contains(&phi) {
                continue;
            }
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

// Мёртвое `φ` не читает входы и не пишет в кадр. Иначе слияние сотен
// неиспользуемых локалей на одном ребре создаёт фиктивный конфликт.
fn live_phi_dependencies(ssa: &Ssa) -> HashSet<ValueId> {
    let mut live_values = HashSet::new();
    let mut pending: Vec<ValueId> = ssa.uses.iter().map(|use_site| use_site.value).collect();
    while let Some(value) = pending.pop() {
        if live_values.insert(value)
            && let Value::Phi { operands, .. } = &ssa.values[value]
        {
            pending.extend(operands.iter().copied());
        }
    }
    live_values
}

/// Проходит все точки программы, отдавая множество живых значений в
/// каждой из них.
///
/// Одна редакция правила «что живо где» на двух потребителей: раскладку и
/// её проверку. Второй экземпляр разъехался бы с первым, и проверка стала
/// бы подтверждать не то, что построено.
fn for_each_point<F: FnMut(&HashSet<ValueId>)>(cfg: &Cfg<'_>, ssa: &Ssa, mut f: F) {
    let out = live_out(cfg, ssa);
    let live_values = live_phi_dependencies(ssa);
    let mut uses_at: HashMap<(usize, Option<usize>), Vec<ValueId>> = HashMap::new();
    for u in &ssa.uses {
        uses_at.entry((u.block, u.stmt)).or_default().push(u.value);
    }

    for (b, block_out) in out.iter().enumerate() {
        if ssa.entry[b].is_none() {
            continue;
        }
        let mut live: HashSet<ValueId> = block_out.clone();
        // Условие терминатора читается последним — значит живо позже всех
        // операторов блока.
        if let Some(vs) = uses_at.get(&(b, None)) {
            live.extend(vs.iter().copied());
        }
        f(&live);
        // Обратный проход. Точка сообщается ДО снятия определения:
        // определённое значение живо начиная отсюда и конфликтует со всем,
        // что живо вокруг. Снять его раньше, чем сообщить, значит
        // объявить, что оно ни с чем не пересекается, — и раскладка выдаст
        // общий регистр двум одновременно живым значениям.
        for (i, s) in cfg.blocks[b].stmts.iter().enumerate().rev() {
            let _ = s;
            // Определение сообщается ВМЕСТЕ с живыми, даже если само оно
            // мертво. Мёртвое определение всё равно ПИШЕТ в регистр, и
            // конфликтует со всем, что в этой точке живо: `Н = Поток.Перейти(0)`,
            // чьё значение никто не читает, обязано не делить регистр с
            // `Поток`, иначе запись уничтожит поток. Пока сообщалось одно
            // лишь живое, такие определения были невидимы.
            let mut with_defs = live.clone();
            for &(stmt_index, id) in &ssa.defs[b] {
                if stmt_index == i {
                    with_defs.insert(id);
                }
            }
            f(&with_defs);
            for &(stmt_index, id) in &ssa.defs[b] {
                if stmt_index == i {
                    live.remove(&id);
                }
            }
            if let Some(vs) = uses_at.get(&(b, Some(i))) {
                live.extend(vs.iter().copied());
            }
            f(&live);
        }
        // Используемые `φ` пишут на входе в блок и конфликтуют с
        // остальными живыми значениями. Неиспользуемые слияния байт-кода
        // не порождают и места в кадре не занимают.
        let mut with_phis = live.clone();
        with_phis.extend(
            ssa.phis[b]
                .iter()
                .copied()
                .filter(|phi| live_values.contains(phi)),
        );
        f(&with_phis);
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

/// Раскладка ЛОКАЛЬНЫХ СЛОТОВ по регистрам кадра.
///
/// Отличается от [`allocate`] уровнем: та раздаёт регистры значениям SSA,
/// а кодоген оперирует слотами, и слот живёт столько, сколько живёт хоть
/// одно его значение. Поэтому слоты конфликтуют, если конфликтует хоть
/// одна пара их значений.
///
/// `pinned` — сколько первых слотов обязаны сохранить свой номер. Это не
/// осторожность, а три жёстких инварианта. Параметры вызываемая функция
/// получает ПО ПОЗИЦИИ: `Frame::param_aliases` сопоставляет `i`-й
/// параметр `i`-му регистру, и сдвинуть его значило бы отдать вызванному
/// чужую ячейку. У кадра нулевого уровня первые регистры И ЕСТЬ
/// модульные переменные — их видят все функции модуля. А чанк с
/// `Выполнить` несёт таблицу «имя -> слот», по которой фрагмент находит
/// переменные, и переименование слотов сделало бы её ложью; такие чанки
/// вызывающий просто не раскладывает.
///
/// # Errors
///
/// Сообщение, если жадная раскладка не нашла свободного регистра.
pub fn allocate_slots(
    cfg: &Cfg<'_>,
    ssa: &Ssa,
    n_slots: usize,
    pinned: usize,
) -> Result<Vec<u8>, String> {
    allocate_slots_with_order(cfg, ssa, n_slots, pinned, false)
}

/// Для большого исходного кадра сначала размещает самые конфликтующие
/// слоты, чтобы ранние короткоживущие имена не заняли все цвета графа.
pub fn allocate_slots_sparse(
    cfg: &Cfg<'_>,
    ssa: &Ssa,
    n_slots: usize,
    pinned: usize,
) -> Result<Vec<u8>, String> {
    allocate_slots_with_order(cfg, ssa, n_slots, pinned, true)
}

fn allocate_slots_with_order(
    cfg: &Cfg<'_>,
    ssa: &Ssa,
    n_slots: usize,
    pinned: usize,
    high_degree_first: bool,
) -> Result<Vec<u8>, String> {
    let pinned = pinned.min(n_slots);
    if pinned > u8::MAX as usize {
        return Err("закреплённых слотов больше, чем регистров кадра".to_string());
    }
    // Слот каждого значения; у `Bottom` слота нет.
    let mut slot_of: Vec<Option<u32>> = vec![None; ssa.values.len()];
    for (id, v) in ssa.values.iter().enumerate() {
        slot_of[id] = match v {
            Value::Def { slot, .. } | Value::Phi { slot, .. } | Value::Entry { slot } => {
                Some(*slot)
            }
            Value::Bottom => None,
        };
    }

    // У каждой точки бывает много живых значений. Битовая строка сразу
    // добавляет все их конфликты слоту, не вставляя каждую пару в `HashSet`.
    let words = n_slots.div_ceil(u64::BITS as usize);
    let mut interfere = vec![vec![0_u64; words]; n_slots];
    for_each_point(cfg, ssa, |live| {
        let mut mask = vec![0_u64; words];
        for &id in live {
            if let Some(slot) = slot_of[id].map(|slot| slot as usize)
                && slot < n_slots
            {
                mask[slot / 64] |= 1_u64 << (slot % 64);
            }
        }
        for (word_index, mut word) in mask.iter().copied().enumerate() {
            while word != 0 {
                let slot = word_index * 64 + word.trailing_zeros() as usize;
                for (neighbours, live_mask) in interfere[slot].iter_mut().zip(&mask) {
                    *neighbours |= live_mask;
                }
                interfere[slot][word_index] &= !(1_u64 << (slot % 64));
                word &= word - 1;
            }
        }
    });

    let mut reg = vec![0_u8; n_slots];
    let mut colored = vec![false; n_slots];
    for slot in 0..pinned {
        reg[slot] = slot as u8;
        colored[slot] = true;
    }
    let mut order: Vec<usize> = (pinned..n_slots).collect();
    if high_degree_first {
        // Это лишь порядок выбора цвета: проверка пересечений и граница
        // кадра остаются теми же, что у обычной раскладки.
        order.sort_unstable_by_key(|&slot| {
            (
                std::cmp::Reverse(
                    interfere[slot]
                        .iter()
                        .map(|word| word.count_ones())
                        .sum::<u32>(),
                ),
                slot,
            )
        });
    }
    for slot in order {
        let mut taken = [false; 256];
        for (word_index, mut word) in interfere[slot].iter().copied().enumerate() {
            while word != 0 {
                let neighbour = word_index * 64 + word.trailing_zeros() as usize;
                if colored[neighbour] {
                    taken[reg[neighbour] as usize] = true;
                }
                word &= word - 1;
            }
        }
        let Some(free) = (pinned..=u8::MAX as usize).find(|&r| !taken[r]) else {
            return Err(format!("слоту {slot} не нашлось регистра"));
        };
        reg[slot] = free as u8;
        colored[slot] = true;
    }
    Ok(reg)
}

/// Проверка раскладки СЛОТОВ — той самой, которую применяет кодоген.
///
/// Отдельно от [`verify`], проверяющей раскладку значений: применяется
/// другая, и проверять надо применяемую. Свойство одно и то же — в каждой
/// точке программы у слотов, живых одновременно, регистры различны, — но
/// живость слота есть живость любого из его значений, и это разные
/// множества.
///
/// # Errors
///
/// Пара слотов, живых одновременно и делящих регистр.
pub fn verify_slots(cfg: &Cfg<'_>, ssa: &Ssa, n_slots: usize, alloc: &[u8]) -> Result<(), String> {
    let mut slot_of: Vec<Option<u32>> = vec![None; ssa.values.len()];
    for (id, v) in ssa.values.iter().enumerate() {
        slot_of[id] = match v {
            Value::Def { slot, .. } | Value::Phi { slot, .. } | Value::Entry { slot } => {
                Some(*slot)
            }
            Value::Bottom => None,
        };
    }
    let mut bad: Option<String> = None;
    for_each_point(cfg, ssa, |live| {
        if bad.is_some() {
            return;
        }
        let mut seen: HashMap<u8, usize> = HashMap::new();
        for &id in live {
            let Some(slot) = slot_of[id] else { continue };
            let slot = slot as usize;
            if slot >= n_slots {
                continue;
            }
            let r = alloc[slot];
            match seen.get(&r) {
                Some(&other) if other != slot => {
                    bad = Some(format!(
                        "слоты {slot} и {other} живы одновременно и делят регистр {r}"
                    ));
                    return;
                }
                _ => {
                    seen.insert(r, slot);
                }
            }
        }
    });
    match bad {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
