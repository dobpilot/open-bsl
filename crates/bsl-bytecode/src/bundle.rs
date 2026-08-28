//! Группировка инструкций в VLIW-бандлы.
//!
//! Бандл — это отрезок подряд идущих инструкций чанка, взаимно независимых
//! настолько, что их можно было бы исполнить параллельно по классической
//! семантике VLIW-пакета: **сначала все чтения, потом все записи**. VM
//! исполняет члены строго по порядку (наблюдаемая семантика не меняется ни
//! на бит), но пользуется разметкой, чтобы пройти весь бандл одним заходом
//! диспетчера — без возврата на верх цикла `drive` между членами. Той же
//! разметкой в будущем может пользоваться JIT как готовым планом
//! параллельной выдачи.
//!
//! # Контракт независимости
//!
//! Внутри бандла запрещены зависимости чтение-после-записи (RAW) и
//! запись-после-записи (WAW). Запись-после-чтения (WAR) *разрешена*: при
//! семантике «все чтения до всех записей» поздний член, пишущий туда,
//! откуда ранний читает, даёт тот же результат, что и последовательное
//! исполнение. Без этого послабления LIFO-переиспользование временных
//! регистров компилятором (см. `free_temp`) рвало бы бандлы между любыми
//! соседними операторами.
//!
//! Исключения подчиняются последовательной семантике: если член `k`
//! бросил, наблюдаемое состояние — как после членов `0..k` и только их.
//! Интерпретатору это даётся даром (он и исполняет по порядку, `pc` в
//! момент ошибки стоит на сбойном члене); параллельный исполнитель обязан
//! коммитить эффекты в порядке номеров членов — стандартная точная
//! семантика исключений VLIW-машин.
//!
//! Пространства адресуемых сущностей (точные регистры, алиасные
//! параметры, модульные слоты, куча и io) и правила их вычисления
//! описаны в [`crate::analysis`] — там же живёт сама классификация.
//!
//! # Производная таблица
//!
//! Разметка хранится в [`Chunk::bundle_len`] и в текстовый формат байт-кода
//! не пишется — при разборе она пересчитывается заново (прецеденты:
//! `prop_cache`, `module_base`). Так `--run-bytecode` не обязан доверять
//! разметке из файла: единственный производитель таблицы — [`compute`], и
//! заявленная независимость членов всегда доказана этим же анализом.
//! Пересчёт обязан быть детерминированной функцией сериализуемых полей —
//! это проверяет побайтовый round-trip текстового формата вместе с
//! пометками `; бандл N` в листинге.
//!
//! Даже ошибочная разметка не изменила бы результатов исполнения: VM
//! исполняет члены тем же `step`, каждый член сам двигает `pc`, и потому
//! разметка влияет только на то, как часто диспетчер возвращается на верх
//! цикла. Инварианты самой разметки проверяет [`verify`] на конформанс-
//! корпусе.

use crate::analysis::{Ctl, Eff, ModSet, RegSet, effects};
use crate::chunk::Chunk;

/// Ширина бандла ограничена ёмкостью `u8` в [`Chunk::bundle_len`].
pub const MAX_BUNDLE_LEN: usize = u8::MAX as usize;

/// Записи, накопленные уже принятыми членами бандла. Чтения не копятся:
/// WAR разрешён, поэтому конфликт нового члена возможен только с чужими
/// записями (RAW/WAW) и с порядкочувствительным «io».
#[derive(Default)]
struct Acc {
    writes: RegSet,
    writes_alias: bool,
    mod_writes_all: bool,
    mod_writes: Vec<u16>,
    heap_write: bool,
    io: bool,
}

impl Acc {
    fn absorb(&mut self, e: &Eff) {
        self.writes.union(&e.writes);
        self.writes_alias |= e.writes_alias;
        match e.mod_writes {
            ModSet::None => {}
            ModSet::One(s) => self.mod_writes.push(s),
            ModSet::All => self.mod_writes_all = true,
        }
        self.heap_write |= e.heap_write;
        self.io |= e.io;
    }

    fn mod_writes_hit(&self, s: ModSet) -> bool {
        match s {
            ModSet::None => false,
            ModSet::One(x) => self.mod_writes_all || self.mod_writes.contains(&x),
            ModSet::All => self.mod_writes_all || !self.mod_writes.is_empty(),
        }
    }
}

/// Есть ли у нового члена `e` зависимость RAW либо WAW от уже принятых.
fn conflicts(e: &Eff, acc: &Acc) -> bool {
    // Точные регистры.
    if e.reads.intersects(&acc.writes) || e.writes.intersects(&acc.writes) {
        return true;
    }
    // Алиасные параметры: любые два обращения — возможно одна ячейка,
    // которая вдобавок может быть модульным слотом.
    if (e.reads_alias || e.writes_alias)
        && (acc.writes_alias || acc.mod_writes_all || !acc.mod_writes.is_empty())
    {
        return true;
    }
    if (e.mod_reads.is_some() || e.mod_writes.is_some()) && acc.writes_alias {
        return true;
    }
    // Модульные слоты: точные номера, `All` пересекается с чем угодно.
    if acc.mod_writes_hit(e.mod_reads) || acc.mod_writes_hit(e.mod_writes) {
        return true;
    }
    // Куча: чтение и запись возможно одного объекта.
    if (e.heap_read || e.heap_write) && acc.heap_write {
        return true;
    }
    // Порядкочувствительный вывод.
    e.io && acc.io
}

/// Позиции, обязанные начинать бандл: цели переходов и все границы
/// диапазонов `Попытка` (вход в обработчик — тоже переход, только со
/// стороны разматывания).
fn leaders(chunk: &Chunk) -> Vec<bool> {
    let n = chunk.instrs.len();
    let mut leader = vec![false; n];
    let mut mark = |pc: usize| {
        if pc < n {
            leader[pc] = true;
        }
    };
    for instr in &chunk.instrs {
        // Список опкодов с целью — один, у определения `Instr`.
        if let Some(target) = instr.jump_target()
            && let Ok(t) = usize::try_from(target)
        {
            mark(t);
        }
    }
    for r in &chunk.exception_ranges {
        mark(r.start_pc);
        mark(r.end_pc);
        mark(r.handler_pc);
    }
    leader
}

/// Размечает чанк: `bundle_len[pc]` — ширина бандла, начинающегося на
/// `pc` (все начала, включая одиночные, получают >= 1), 0 — середина
/// бандла. Детерминированная функция сериализуемых полей чанка и
/// `overlap` — см. модульный док.
pub fn compute(chunk: &Chunk, overlap: Option<usize>) -> Vec<u8> {
    let n = chunk.instrs.len();
    let leader = leaders(chunk);
    let mut out = vec![0u8; n];
    let mut s = 0;
    while s < n {
        let first = effects(&chunk.instrs[s], chunk, overlap);
        let mut k = 1;
        if first.ctl == Ctl::None {
            let mut acc = Acc::default();
            acc.absorb(&first);
            while s + k < n && k < MAX_BUNDLE_LEN {
                if leader[s + k] {
                    break;
                }
                let e = effects(&chunk.instrs[s + k], chunk, overlap);
                if e.ctl == Ctl::Barrier || conflicts(&e, &acc) {
                    break;
                }
                k += 1;
                if e.ctl == Ctl::Trailing {
                    break;
                }
                acc.absorb(&e);
            }
        }
        out[s] = k as u8;
        s += k;
    }
    out
}

/// Проверяет инварианты готовой разметки — независимо от жадного
/// построителя, попарным пересчётом: разбиение плотное, ширины в границах,
/// лидеры не внутри бандлов, передача управления только хвостом, каждый
/// не первый член свободен от RAW/WAW со всеми предыдущими. Используется
/// тестами на конформанс-корпусе.
///
/// # Errors
///
/// Возвращает описание первого нарушенного инварианта.
pub fn verify(chunk: &Chunk, overlap: Option<usize>) -> Result<(), String> {
    let n = chunk.instrs.len();
    if chunk.bundle_len.len() != n {
        return Err(format!(
            "длина bundle_len {} не равна числу инструкций {n}",
            chunk.bundle_len.len()
        ));
    }
    let leader = leaders(chunk);
    let mut s = 0;
    while s < n {
        let k = chunk.bundle_len[s] as usize;
        if k == 0 || s + k > n {
            return Err(format!("бандл на {s} шириной {k} выходит за пределы"));
        }
        for i in 1..k {
            let pc = s + i;
            if chunk.bundle_len[pc] != 0 {
                return Err(format!("{pc} внутри бандла {s}, но помечен как начало"));
            }
            if leader[pc] {
                return Err(format!("цель перехода {pc} внутри бандла {s}"));
            }
        }
        for i in 0..k {
            let e = effects(&chunk.instrs[s + i], chunk, overlap);
            match e.ctl {
                Ctl::Barrier if k != 1 => {
                    return Err(format!("барьер {} внутри бандла {s} ширины {k}", s + i));
                }
                Ctl::Trailing if i + 1 != k => {
                    return Err(format!(
                        "передача управления {} не хвостом бандла {s}",
                        s + i
                    ));
                }
                _ => {}
            }
            // Попарно против каждого предыдущего члена.
            for j in 0..i {
                let mut prev = Acc::default();
                prev.absorb(&effects(&chunk.instrs[s + j], chunk, overlap));
                if conflicts(&e, &prev) {
                    return Err(format!(
                        "члены {} и {} бандла {s} зависимы (RAW/WAW)",
                        s + j,
                        s + i
                    ));
                }
            }
        }
        s += k;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // Сами тесты строят чанки из инструкций, а рабочий код модуля после
    // выноса классификации в `analysis` про `Instr` уже не знает.
    use crate::instr::Instr;
    use std::cell::RefCell;

    fn chunk(instrs: Vec<Instr>) -> Chunk {
        let prop_cache = instrs.iter().map(|_| RefCell::new(None)).collect();
        let method_cache = instrs.iter().map(|_| RefCell::new(None)).collect();
        let bundle_len = vec![0; instrs.len()];
        Chunk {
            instrs,
            consts: Vec::new(),
            call_arg_modes: Vec::new(),
            exception_ranges: Vec::new(),
            n_params: 0,
            param_by_val: Vec::new(),
            param_has_default: Vec::new(),
            is_procedure: false,
            is_async: false,
            touches_objects: false,
            n_locals: 8,
            n_regs: 16,
            prop_cache,
            method_cache,
            local_names: Vec::new(),
            bundle_len,
        }
    }

    fn widths(c: &Chunk, overlap: Option<usize>) -> Vec<u8> {
        let mut c = c.clone();
        c.bundle_len = compute(&c, overlap);
        verify(&c, overlap).expect("построенная разметка обязана проходить проверку");
        c.bundle_len
    }

    #[test]
    fn independent_neighbours_bundle_up() {
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 0 },
            Instr::Move { dst: 2, src: 0 },
        ]);
        // 2-й член читает r0 — RAW от 0-го: в бандл не входит.
        assert_eq!(widths(&c, None), vec![2, 0, 1]);
    }

    #[test]
    fn war_is_allowed_waw_is_not() {
        // WAR: 1-й читает r0, 2-й пишет r0 — допустимо в одном пакете.
        let c = chunk(vec![
            Instr::Move { dst: 1, src: 0 },
            Instr::LoadConst { dst: 0, k: 0 },
        ]);
        assert_eq!(widths(&c, None), vec![2, 0]);
        // WAW: обе пишут r1 — порядок записей наблюдаем, рвём.
        let c = chunk(vec![
            Instr::LoadConst { dst: 1, k: 0 },
            Instr::LoadBool { dst: 1, val: true },
        ]);
        assert_eq!(widths(&c, None), vec![1, 1]);
    }

    #[test]
    fn byref_params_may_alias_each_other() {
        let mut c = chunk(vec![
            // Записи в два РАЗНЫХ параметра без `Знач`: номера разные, но
            // слот может быть один (`Ф(х, х)`).
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 0 },
        ]);
        c.n_params = 2;
        c.param_by_val = vec![false, false];
        assert_eq!(widths(&c, None), vec![1, 1]);
        // Те же номера как `Знач`-параметры — приватные копии, бандл цел.
        c.param_by_val = vec![true, true];
        assert_eq!(widths(&c, None), vec![2, 0]);
    }

    #[test]
    fn jump_target_starts_a_bundle_and_jump_trails() {
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 0 },
            Instr::Jump { target: 1 },
        ]);
        // Цель перехода (pc 1) рвёт бандл; сам Jump — хвост следующего.
        assert_eq!(widths(&c, None), vec![1, 2, 0]);
    }

    #[test]
    fn two_prints_keep_their_order() {
        let msg = bsl_rt::BuiltinFn::lookup("Сообщить").expect("есть в таблице");
        let c = chunk(vec![
            Instr::CallBuiltin {
                dst: 2,
                builtin: msg,
                base: 0,
                count: 1,
            },
            Instr::CallBuiltin {
                dst: 3,
                builtin: msg,
                base: 1,
                count: 1,
            },
            Instr::LoadConst { dst: 4, k: 0 },
        ]);
        // io против io — конфликт; чистая запись регистра с вызовом
        // уживается (читаемые им r1 и пишемый r3 не задеты).
        assert_eq!(widths(&c, None), vec![1, 2, 0]);
    }

    #[test]
    fn module_vars_overlap_frame_regs_only_at_top_level() {
        let c = chunk(vec![
            Instr::SetModuleVar { slot: 0, src: 3 },
            Instr::LoadConst { dst: 0, k: 0 },
        ]);
        // В функции модульный слот 0 и регистр 0 — разные ячейки.
        assert_eq!(widths(&c, None), vec![2, 0]);
        // В чанке верхнего уровня — одна: WAW, рвём.
        assert_eq!(widths(&c, Some(1)), vec![1, 1]);
    }

    #[test]
    fn heap_reads_bundle_heap_writes_do_not() {
        let name = bsl_rt::NameId::from_index(0);
        let c = chunk(vec![
            Instr::GetProp {
                dst: 2,
                obj: 0,
                name,
            },
            Instr::GetProp {
                dst: 3,
                obj: 1,
                name,
            },
            Instr::SetProp {
                obj: 1,
                name,
                src: 4,
            },
            Instr::GetProp {
                dst: 5,
                obj: 0,
                name,
            },
        ]);
        // Чтения кучи совместимы; запись после чтений — WAR, допустима
        // хвостом того же пакета; чтение ПОСЛЕ записи — RAW, новый бандл.
        assert_eq!(widths(&c, None), vec![3, 0, 0, 1]);
    }

    #[test]
    fn verify_rejects_a_forged_table() {
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
        ]);
        // Подделка: объявляем зависимую пару независимой.
        c.bundle_len = vec![2, 0];
        assert!(verify(&c, None).is_err());
    }
}
