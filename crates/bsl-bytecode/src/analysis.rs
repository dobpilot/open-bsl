//! Классификация эффектов инструкций — единый исчерпывающий источник.
//!
//! Кто читает, кто пишет и кто передаёт управление — это знание нужно
//! сразу нескольким потребителям: разметке VLIW-бандлов ([`crate::bundle`])
//! и анализу потока данных компилятора. Второй исчерпывающий `match` по
//! всем опкодам недопустим: он разъедется с первым на новом опкоде, причём
//! молча и в сторону неверной оптимизации. Поэтому таблица живёт здесь
//! одна, и [`effects`] остаётся `match` без ветви-заглушки — пропущенный
//! опкод обязан быть ошибкой сборки, а не значением по умолчанию.
//!
//! # Почему номер регистра — ещё не имя ячейки
//!
//! Анализ считает адресуемые сущности по четырём непересекающимся
//! пространствам, потому что «регистр кадра» в этой VM не всегда
//! собственная ячейка (см. `Frame::param_aliases` в `bsl-vm`):
//!
//! - **Точные регистры** — собственные слоты кадра (`r >= n_params`) и
//!   `Знач`-параметры. `Знач`-аргумент всегда материализуется копией в
//!   свежий временный регистр вызывающего (`compile_call`), так что его
//!   слот приватен и никакому другому регистру кадра не алиасен —
//!   независимость доказывается сравнением номеров.
//! - **Алиасные регистры** — параметры без `Знач` (`r < n_params` и
//!   `!param_by_val[r]`). Они разрешаются в абсолютные слоты чужих кадров,
//!   причём два разных номера могут указывать в один слот (`Ф(х, х)`), а
//!   слот может оказаться и модульной переменной. Поэтому любые два
//!   обращения через это пространство считаются попаданием в одну ячейку.
//! - **Модульные слоты** — `GetModuleVar`/`SetModuleVar` адресуют стек
//!   абсолютно, мимо регистров кадра. В чанке верхнего уровня обычной
//!   программы (`module_base == 0`) слот `s` — это же регистр `s` того же
//!   кадра, что учитывает параметр `module_overlap`. `Вызов` может
//!   транзитивно прочитать и записать любой модульный слот.
//! - **Куча и вывод** — объекты за `Rc` статически неразличимы, поэтому
//!   куча делится только на «читал»/«писал», а порядкочувствительный вывод
//!   (`Сообщить`, файлы) — одна общая ячейка «io». Ячейка инлайн-кэша
//!   `prop_cache` конфликтом не считается: она своя у каждого `pc`.
//!
//! Инструкции передачи управления (`Jump*`, `NumericForNext*`, `Call`,
//! `Return`, `Raise`) могут быть только последним членом бандла, а
//! `RunDynamic` — всегда одиночный: фрагмент видит все именованные локали
//! кадра по именам. Цели переходов, границы и обработчики `Попытка`
//! начинают новый бандл.
//!

use crate::chunk::Chunk;
use crate::instr::{ArgMode, Instr};

/// Пересечение модульных слотов с регистрами кадра: `Some(count)` — чанк
/// верхнего уровня обычной программы, где модульные переменные занимают
/// регистры `0..count`; `None` — пересечения нет. Единственная точка, где
/// решается это правило: и компилятор, и разбор текстового формата обязаны
/// звать её с индексом чанка, чтобы пересчёт совпал побайтово. У фрагмента
/// `Выполнить`/`Вычислить` пересечение зависит от его `module_base`, а тот
/// известен только на прогоне: поэтому `run_dynamic_snippet` в bsl-vm сам
/// пересчитывает разметку `chunks[0]`, передавая `Some(n)` для верхнего
/// уровня (`module_base == 0`, слоты накладываются) и `None` для вложенного.
pub fn module_overlap(chunk_index: usize, module_var_count: usize) -> Option<usize> {
    (chunk_index == 0).then_some(module_var_count)
}

/// Битовый набор номеров регистров кадра (0..=255).
#[derive(Clone, Copy, Default)]
pub(crate) struct RegSet([u64; 4]);

impl RegSet {
    pub(crate) fn insert(&mut self, r: usize) {
        if r < 256 {
            self.0[r / 64] |= 1 << (r % 64);
        }
    }

    pub(crate) fn insert_range(&mut self, base: usize, count: usize) {
        // Насыщение на 256 корректно в консервативную сторону: чанк не от
        // кодогена может заявить диапазон шире регистрового файла.
        for r in base..(base + count).min(256) {
            self.insert(r);
        }
    }

    pub(crate) fn contains(&self, r: usize) -> bool {
        r < 256 && self.0[r / 64] & (1 << (r % 64)) != 0
    }

    pub(crate) fn subtract(&mut self, other: &RegSet) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a &= !*b;
        }
    }

    pub(crate) fn equals(&self, other: &RegSet) -> bool {
        self.0 == other.0
    }

    pub(crate) fn intersects(&self, other: &RegSet) -> bool {
        self.0.iter().zip(other.0.iter()).any(|(a, b)| a & b != 0)
    }

    pub(crate) fn union(&mut self, other: &RegSet) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a |= b;
        }
    }
}

/// Обращения одной инструкции к модульным слотам. `All` — у `Call` и
/// `RunDynamic`: вызванный код может транзитивно тронуть любой слот.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ModSet {
    None,
    One(u16),
    All,
}

impl ModSet {
    pub(crate) fn is_some(self) -> bool {
        !matches!(self, ModSet::None)
    }
}

/// Класс передачи управления: `Trailing` может стоять только последним
/// членом бандла, `Barrier` — всегда одиночный.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Ctl {
    None,
    Trailing,
    Barrier,
}

/// Эффекты одной инструкции в терминах пространств из модульного дока.
#[derive(Clone, Copy)]
pub(crate) struct Eff {
    pub(crate) reads: RegSet,
    /// Чтения, НЕ адресуемые операндом инструкции. Таких два вида:
    /// регистр входит в непрерывное окно `base..base+count`, которое
    /// вызываемый читает по смещению, либо номер регистра записан в
    /// боковой таблице (`call_arg_modes` для аргумента по ссылке), а не в
    /// самой инструкции. И то и другое переименованием не меняется,
    /// поэтому копия, чей приёмник читают так, локальным проходом не
    /// снимается.
    pub(crate) reads_positional: RegSet,
    pub(crate) writes: RegSet,
    pub(crate) reads_alias: bool,
    pub(crate) writes_alias: bool,
    pub(crate) mod_reads: ModSet,
    pub(crate) mod_writes: ModSet,
    pub(crate) heap_read: bool,
    pub(crate) heap_write: bool,
    pub(crate) io: bool,
    pub(crate) ctl: Ctl,
}

impl Default for Eff {
    fn default() -> Self {
        Eff {
            reads: RegSet::default(),
            reads_positional: RegSet::default(),
            writes: RegSet::default(),
            reads_alias: false,
            writes_alias: false,
            mod_reads: ModSet::None,
            mod_writes: ModSet::None,
            heap_read: false,
            heap_write: false,
            io: false,
            ctl: Ctl::None,
        }
    }
}

/// Эффекты инструкции. `overlap` — см. [`module_overlap`]; `chunk` нужен
/// ради `call_arg_modes` (скрытые операнды `Call`) и режимов параметров.
pub(crate) fn effects(instr: &Instr, chunk: &Chunk, overlap: Option<usize>) -> Eff {
    let mut e = Eff::default();
    // Обращение к регистру раскладывается по пространству: параметр без
    // `Знач` — алиасное, остальное — точное.
    let is_alias = |r: u8| {
        (r as usize) < chunk.n_params as usize
            && !chunk.param_by_val.get(r as usize).copied().unwrap_or(false)
    };
    macro_rules! read {
        ($r:expr) => {
            if is_alias($r) {
                e.reads_alias = true;
            } else {
                e.reads.insert($r as usize);
            }
        };
    }
    macro_rules! write {
        ($r:expr) => {
            if is_alias($r) {
                e.writes_alias = true;
            } else {
                e.writes.insert($r as usize);
            }
        };
    }
    // Диапазоны `base..base+count` аллоцируются компилятором только из
    // временных регистров, но чанк может прийти не от кодогена — поэтому
    // диапазон честно проверяется на алиасные номера.
    // Чтение по фиксированному месту: окно аргументов или слот из
    // боковой таблицы. Операнда, который можно переписать, у него нет.
    macro_rules! read_fixed {
        ($r:expr) => {
            if is_alias($r) {
                e.reads_alias = true;
            } else {
                e.reads.insert($r as usize);
                e.reads_positional.insert($r as usize);
            }
        };
    }
    macro_rules! read_range {
        ($base:expr, $count:expr) => {
            for r in ($base as usize)..(($base as usize) + ($count as usize)).min(256) {
                if is_alias(r as u8) {
                    e.reads_alias = true;
                } else {
                    e.reads.insert(r);
                    e.reads_positional.insert(r);
                }
            }
        };
    }
    // Модульный слот в чанке верхнего уровня — это же регистр кадра.
    let mod_read = |e: &mut Eff, slot: u16| {
        e.mod_reads = ModSet::One(slot);
        if overlap.is_some_and(|count| (slot as usize) < count) {
            e.reads.insert(slot as usize);
        }
    };
    let mod_write = |e: &mut Eff, slot: u16| {
        e.mod_writes = ModSet::One(slot);
        if overlap.is_some_and(|count| (slot as usize) < count) {
            e.writes.insert(slot as usize);
        }
    };
    // `Call`/`RunDynamic` могут транзитивно тронуть любой модульный слот.
    let mod_all = |e: &mut Eff| {
        e.mod_reads = ModSet::All;
        e.mod_writes = ModSet::All;
        if let Some(count) = overlap {
            e.reads.insert_range(0, count);
            e.reads_positional.insert_range(0, count);
            e.writes.insert_range(0, count);
        }
    };

    match *instr {
        Instr::Move { dst, src } => {
            read!(src);
            write!(dst);
        }
        Instr::GetModuleVar { dst, slot } => {
            mod_read(&mut e, slot);
            write!(dst);
        }
        Instr::SetModuleVar { slot, src } => {
            read!(src);
            mod_write(&mut e, slot);
        }
        Instr::LoadConst { dst, .. }
        | Instr::LoadBool { dst, .. }
        | Instr::LoadUndefined { dst }
        | Instr::LoadNull { dst } => {
            write!(dst);
        }
        Instr::Add { dst, a, b }
        | Instr::Sub { dst, a, b }
        | Instr::Mul { dst, a, b }
        | Instr::Div { dst, a, b }
        | Instr::Mod { dst, a, b }
        | Instr::Eq { dst, a, b }
        | Instr::NotEq { dst, a, b }
        | Instr::Lt { dst, a, b }
        | Instr::Gt { dst, a, b }
        | Instr::Le { dst, a, b }
        | Instr::Ge { dst, a, b } => {
            read!(a);
            read!(b);
            write!(dst);
        }
        Instr::AddConst { dst, src, .. } => {
            read!(src);
            write!(dst);
        }
        Instr::Neg { dst, src } | Instr::Not { dst, src } => {
            read!(src);
            write!(dst);
        }
        Instr::Jump { .. } => {
            e.ctl = Ctl::Trailing;
        }
        Instr::JumpIfFalse { cond, .. } | Instr::JumpIfTrue { cond, .. } => {
            read!(cond);
            e.ctl = Ctl::Trailing;
        }
        Instr::JumpIfNotEqConst { src, .. } | Instr::JumpIfNotLtConst { src, .. } => {
            read!(src);
            e.ctl = Ctl::Trailing;
        }
        // Регистров не читает вовсе: признак пропущенного аргумента лежит
        // в метаданных кадра, а не в слоте параметра (см. `Instr`).
        Instr::JumpIfNotSkipped { .. } => {
            e.ctl = Ctl::Trailing;
        }
        Instr::NumericForNext { counter, bound, .. }
        | Instr::NumericForNextI64 { counter, bound, .. } => {
            read!(counter);
            read!(bound);
            write!(counter);
            e.ctl = Ctl::Trailing;
        }
        Instr::Call {
            base,
            arg_modes,
            ret,
            ..
        } => {
            match chunk.call_arg_modes.get(arg_modes as usize) {
                Some(modes) => {
                    for (i, m) in modes.iter().enumerate() {
                        match m {
                            ArgMode::Value => {
                                let r = (base as usize + i).min(255) as u8;
                                read_fixed!(r);
                            }
                            // Вызванный видит слот по ссылке: и читает, и
                            // может записать.
                            ArgMode::ByRefLocal(slot) => {
                                read_fixed!(*slot);
                                write!(*slot);
                            }
                            // Модульная переменная по ссылке: вызванный читает
                            // и пишет её module-слот. Отдельно отмечать не
                            // нужно — `mod_all` ниже помечает ВСЕ модульные
                            // слоты консервативно (вызов и так их барьер).
                            ArgMode::ByRefModuleVar(_) => {}
                            // Импортированная переменная по ссылке: чужое
                            // состояние — куча, heap-флаги вызова ниже
                            // упорядочивают его сами.
                            ArgMode::ByRefImportedVar(_) => {}
                            // Пропущенная позиция: вызывающий в этот
                            // регистр ничего не клал и вызванный оттуда
                            // ничего не читает — но пролог умолчаний
                            // пишет туда вычисленное значение.
                            ArgMode::Default => {
                                let r = (base as usize + i).min(255) as u8;
                                write!(r);
                            }
                        }
                    }
                }
                // Битый индекс не от кодогена: считаем вызов барьером.
                None => {
                    e.ctl = Ctl::Barrier;
                }
            }
            write!(ret);
            mod_all(&mut e);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
            if e.ctl == Ctl::None {
                e.ctl = Ctl::Trailing;
            }
        }
        Instr::CallImported {
            base,
            arg_modes,
            ret,
            ..
        } => {
            // Классификация повторяет `Call` консервативно: вызванный чужой
            // модуль наших слотов не видит, но `ByRefModuleVar`-аргументы
            // дают запись при возврате, а точечный учёт нескольких слотов
            // `ModSet::One` не выразит — `mod_all` дешевле и всегда верен.
            match chunk.call_arg_modes.get(arg_modes as usize) {
                Some(modes) => {
                    for (i, m) in modes.iter().enumerate() {
                        match m {
                            ArgMode::Value => {
                                let r = (base as usize + i).min(255) as u8;
                                read_fixed!(r);
                            }
                            ArgMode::ByRefLocal(slot) => {
                                read_fixed!(*slot);
                                write!(*slot);
                            }
                            ArgMode::ByRefModuleVar(_) => {}
                            // Чужое состояние модулей — куча: heap-флаги
                            // ниже уже упорядочивают такие обращения.
                            ArgMode::ByRefImportedVar(_) => {}
                            ArgMode::Default => {
                                let r = (base as usize + i).min(255) as u8;
                                write!(r);
                            }
                        }
                    }
                }
                None => {
                    e.ctl = Ctl::Barrier;
                }
            }
            write!(ret);
            mod_all(&mut e);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
            if e.ctl == Ctl::None {
                e.ctl = Ctl::Trailing;
            }
        }
        // Слоты чужого модуля живут вне регистров и вне `ModSet` текущего:
        // для порядка обращений достаточно кучи — два доступа к одному
        // импортированному слоту не попадут в один бандл, если хотя бы один
        // из них запись. Оба опкода — хвостовые: первое касание модуля
        // пушит кадр его инициализации, и продолжать бандл прежнего кадра
        // после этого нельзя.
        Instr::GetImportedVar { dst, .. } => {
            write!(dst);
            e.heap_read = true;
            e.ctl = Ctl::Trailing;
        }
        Instr::SetImportedVar { src, .. } => {
            read!(src);
            e.heap_write = true;
            e.ctl = Ctl::Trailing;
        }
        Instr::Await { dst, promise } => {
            read!(promise);
            write!(dst);
            e.ctl = Ctl::Barrier;
        }
        Instr::Return { src } => {
            if let Some(src) = src {
                read!(src);
            }
            e.ctl = Ctl::Trailing;
        }
        Instr::GetIndex { dst, obj, idx } => {
            read!(obj);
            read!(idx);
            write!(dst);
            e.heap_read = true;
        }
        Instr::SetIndex { obj, idx, src } => {
            read!(obj);
            read!(idx);
            read!(src);
            e.heap_write = true;
        }
        Instr::GetProp { dst, obj, .. } => {
            read!(obj);
            write!(dst);
            e.heap_read = true;
        }
        Instr::SetProp { obj, src, .. } => {
            read!(obj);
            read!(src);
            e.heap_write = true;
        }
        // Открытые двойники `GetProp`/`SetProp`: та же операция над теми
        // же получателями, отличается только способ разрешения имени,
        // поэтому и эффекты заявлены те же. Прежний `Ctl::Barrier` был
        // перестраховкой и рвал бандлы на каждом обращении к полю — с
        // реестром компонентов так компилируется каждое обращение
        // программы, и барьер стоил `csv_write` десятки процентов.
        Instr::GetObjectProp { dst, obj, .. } => {
            read!(obj);
            write!(dst);
            e.heap_read = true;
        }
        Instr::SetObjectProp { obj, src, .. } => {
            read!(obj);
            read!(src);
            e.heap_write = true;
        }
        Instr::CreateObject {
            dst, base, count, ..
        } => {
            read_range!(base, count);
            write!(dst);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
            e.ctl = Ctl::Barrier;
        }
        Instr::NewArray { dst, base, count } => {
            read_range!(base, count);
            write!(dst);
        }
        Instr::NewStructure {
            dst, base, count, ..
        } => {
            read_range!(base, count);
            write!(dst);
        }
        Instr::NewTable { dst } | Instr::NewValueComparison { dst } | Instr::NewMap { dst } => {
            write!(dst);
        }
        Instr::NewTypeDescription { dst, names } => {
            read!(names);
            write!(dst);
        }
        Instr::NewTextWriter { dst, path } => {
            read!(path);
            write!(dst);
            // Конструктор открывает файл — порядок относительно другого
            // вывода наблюдаем.
            e.io = true;
        }
        Instr::CollectionLen { dst, obj } => {
            read!(obj);
            write!(dst);
            e.heap_read = true;
        }
        Instr::Raise { src } => {
            if let Some(src) = src {
                read!(src);
            }
            e.ctl = Ctl::Trailing;
        }
        Instr::CallBuiltin {
            dst, base, count, ..
        } => {
            read_range!(base, count);
            write!(dst);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
        }
        Instr::CallComponent {
            dst, base, count, ..
        } => {
            read_range!(base, count);
            write!(dst);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
            e.ctl = Ctl::Barrier;
        }
        Instr::CallMethod {
            dst,
            obj,
            base,
            count,
            ..
        } => {
            read!(obj);
            read_range!(base, count);
            write!(dst);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
        }
        // Открытый двойник `CallMethod`: эффекты — как у закрытого, тот
        // обслуживает тех же получателей (включая компонентные объекты) и
        // барьера не несёт.
        Instr::CallObjectMethod {
            dst,
            obj,
            base,
            count,
            ..
        } => {
            read!(obj);
            read_range!(base, count);
            write!(dst);
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
        }
        // Фрагмент читает и пишет все именованные локали кадра по именам
        // и исполняет произвольный код — всегда одиночный.
        Instr::RunDynamic { .. } => {
            e.ctl = Ctl::Barrier;
            // Фрагмент видит ИМЕНОВАННЫЕ локали кадра по именам
            // (`local_names`), а не по номерам регистров, поэтому здесь
            // читается и пишется всё. Для разметки бандлов это ничего не
            // меняет — барьер и так одиночный, — но анализу потока данных
            // без этого локаль казалась бы мёртвой, и устранение копий
            // сняло бы ЖИВОЕ значение.
            e.reads.insert_range(0, 256);
            e.reads_positional.insert_range(0, 256);
            e.writes.insert_range(0, 256);
            e.reads_alias = true;
            e.writes_alias = true;
            // И модульные слоты: фрагмент видит переменные модуля так же,
            // как локали. Без этого таблица неверна для любого будущего
            // потребителя, даже если нынешний спасён флагом алиасов.
            e.mod_reads = ModSet::All;
            e.mod_writes = ModSet::All;
            // Фрагмент может тронуть и объекты за `Rc`, и порядко-
            // чувствительный вывод: `Выполнить("Сообщить(...)")` — это
            // обычный код. Сейчас от неверных выводов спасал бы барьер
            // управления, но таблица заявлена единым источником, и
            // полагаться на то, что каждый её потребитель сам вспомнит
            // про барьер, нельзя.
            e.heap_read = true;
            e.heap_write = true;
            e.io = true;
        }
    }
    e
}

/// Позиции, обязанные начинать бандл: цели переходов и все границы
/// диапазонов `Попытка` (вход в обработчик — тоже переход, только со
/// стороны разматывания).
pub(crate) fn leaders(chunk: &Chunk) -> Vec<bool> {
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

/// Разбиение чанка на базовые блоки и рёбра между ними.
///
/// Минимальный CFG: он нужен анализу живучести, на котором держится
/// оценка устранимых копий. Обработчик `Попытка` достижим из любой точки
/// защищённого диапазона, поэтому его блок объявляется преемником каждого
/// блока диапазона — консервативно и без попытки угадать точку броска.
struct Cfg {
    /// Номер блока для каждой инструкции.
    block_of: Vec<usize>,
    /// Границы блоков: `[начало, конец)`.
    blocks: Vec<(usize, usize)>,
    succs: Vec<Vec<usize>>,
}

fn build_cfg(chunk: &Chunk) -> Cfg {
    let n = chunk.instrs.len();
    let mut is_leader = leaders(chunk);
    if n > 0 {
        is_leader[0] = true;
    }
    // Инструкция после передачи управления начинает новый блок.
    for (pc, instr) in chunk.instrs.iter().enumerate() {
        let ends = instr.jump_target().is_some()
            || matches!(instr, Instr::Return { .. } | Instr::Raise { .. });
        if ends && pc + 1 < n {
            is_leader[pc + 1] = true;
        }
    }

    let mut blocks = Vec::new();
    let mut block_of = vec![0usize; n];
    let mut start = 0;
    for pc in 0..n {
        if pc > 0 && is_leader[pc] {
            blocks.push((start, pc));
            start = pc;
        }
        block_of[pc] = blocks.len();
    }
    if n > 0 {
        blocks.push((start, n));
    }

    let mut succs = vec![Vec::new(); blocks.len()];
    for (b, &(lo, hi)) in blocks.iter().enumerate() {
        let last = &chunk.instrs[hi - 1];
        let _ = lo;
        if let Some(t) = last.jump_target()
            && let Ok(t) = usize::try_from(t)
            && t < n
        {
            succs[b].push(block_of[t]);
        }
        // Проваливается всё, кроме безусловного перехода, возврата и
        // возбуждения исключения.
        let falls = !matches!(
            last,
            Instr::Jump { .. } | Instr::Return { .. } | Instr::Raise { .. }
        );
        if falls && hi < n {
            succs[b].push(block_of[hi]);
        }
    }
    for r in &chunk.exception_ranges {
        if r.handler_pc >= n {
            continue;
        }
        let handler = block_of[r.handler_pc];
        for &b in &block_of[r.start_pc..r.end_pc.min(n)] {
            if !succs[b].contains(&handler) {
                succs[b].push(handler);
            }
        }
    }
    Cfg {
        block_of,
        blocks,
        succs,
    }
}

/// Живые на выходе из каждого блока точные регистры.
fn live_out(chunk: &Chunk, cfg: &Cfg, overlap: Option<usize>) -> Vec<RegSet> {
    live_out_in_order(chunk, cfg, overlap, true)
}

/// То же, но с выбором порядка обхода блоков. Порядок влияет только на
/// число итераций до неподвижной точки; на результат — не должен, и
/// [`verify`] это проверяет сравнением двух прогонов.
fn live_out_in_order(
    chunk: &Chunk,
    cfg: &Cfg,
    overlap: Option<usize>,
    backwards: bool,
) -> Vec<RegSet> {
    let nb = cfg.blocks.len();
    let mut live_in = vec![RegSet::default(); nb];
    let mut live_out = vec![RegSet::default(); nb];
    // Обратный поток до неподвижной точки. Блоков в чанке немного, а
    // порядок обхода на результат не влияет — только на число итераций.
    let mut changed = true;
    while changed {
        changed = false;
        let order: Vec<usize> = if backwards {
            (0..nb).rev().collect()
        } else {
            (0..nb).collect()
        };
        for b in order {
            let mut out = RegSet::default();
            for &s in &cfg.succs[b] {
                out.union(&live_in[s]);
            }
            let (lo, hi) = cfg.blocks[b];
            let mut cur = out;
            for pc in (lo..hi).rev() {
                let e = effects(&chunk.instrs[pc], chunk, overlap);
                cur.subtract(&e.writes);
                cur.union(&e.reads);
            }
            if !cur.equals(&live_in[b]) || !out.equals(&live_out[b]) {
                live_in[b] = cur;
                live_out[b] = out;
                changed = true;
            }
        }
    }
    live_out
}

/// Копии, которые снял бы проход copy propagation с последующим DCE.
///
/// Возвращает по одному флагу на инструкцию; `true` стоит только на
/// `Move`. Оценка **консервативная**, то есть заведомо нижняя граница:
/// назначение таблицы — вместе со счётчиками исполнения ответить, какая
/// доля исполненных копий вообще устранима, и завышенная оценка сделала
/// бы этот ответ бесполезным.
///
/// Копия `Move dst, src` считается устранимой, когда одновременно:
///
/// - и приёмник, и источник — точные регистры. Параметр без `Знач`
///   разрешается в слот чужого кадра, а модульный слот в чанке верхнего
///   уровня накладывается на регистр кадра — в обоих случаях номер
///   регистра не называет ячейку однозначно;
/// - до конца жизни приёмника источник не переписан: иначе чтения
///   приёмника нельзя направить на источник;
/// - приёмник умирает внутри блока — переопределяется в нём же либо не
///   жив на выходе. Значение, живущее дальше по графу, копией и держится,
///   и такую копию локальный проход снять не может;
/// - между копией и смертью приёмника нет инструкции, чьи эффекты выходят
///   за точные регистры: вызова, динамического фрагмента, записи в
///   алиасный параметр или модульный слот.
pub fn removable_copies(chunk: &Chunk, overlap: Option<usize>) -> Vec<bool> {
    let n = chunk.instrs.len();
    let mut out = vec![false; n];
    if n == 0 {
        return out;
    }
    let cfg = build_cfg(chunk);
    let live = live_out(chunk, &cfg, overlap);
    let exact = |r: u8| {
        let r = r as usize;
        let aliased =
            r < chunk.n_params as usize && !chunk.param_by_val.get(r).copied().unwrap_or(false);
        !aliased && !overlap.is_some_and(|k| r < k)
    };

    // Внутри `Попытка` исключение может сработать НА ЛЮБОЙ инструкции, а
    // не только в конце блока. Значит последующая запись в приёмник не
    // объявляет копию мёртвой: если бросок случится между копией и этой
    // записью, обработчик увидит именно скопированное значение. Строить
    // здесь точную живучесть по каждой точке диапазона можно, но это
    // отдельная работа; до неё копии в защищённых диапазонах не
    // рассматриваются вовсе.
    let protected = |pc: usize| {
        chunk
            .exception_ranges
            .iter()
            .any(|r| pc >= r.start_pc && pc < r.end_pc)
    };
    for (i, instr) in chunk.instrs.iter().enumerate() {
        let Instr::Move { dst, src } = *instr else {
            continue;
        };
        if !exact(dst) || !exact(src) || protected(i) {
            continue;
        }
        if dst == src {
            out[i] = true;
            continue;
        }
        let b = cfg.block_of[i];
        let (_, hi) = cfg.blocks[b];
        let mut removable = None;
        for pc in (i + 1)..hi {
            let e = effects(&chunk.instrs[pc], chunk, overlap);
            if e.writes_alias || e.mod_writes.is_some() {
                removable = Some(false);
                break;
            }
            if e.writes.contains(src as usize) {
                removable = Some(false);
                break;
            }
            if e.reads_positional.contains(dst as usize) {
                // Приёмник читают по месту в окне аргументов: снять копию
                // можно только заставив её источник писать прямо в это
                // окно, а это другое преобразование, не переименование.
                removable = Some(false);
                break;
            }
            if e.writes.contains(dst as usize) {
                // Приёмник переопределён: старое значение больше не нужно.
                removable = Some(true);
                break;
            }
        }
        out[i] = removable.unwrap_or_else(|| !live[b].contains(dst as usize));
    }
    out
}

/// Проверка инвариантов анализа — независимая от его построителя.
///
/// Проверяются три вещи. Разбиение на блоки обязано покрывать чанк
/// целиком и ссылаться только на существующие блоки. Живучесть обязана
/// не зависеть от порядка обхода: она считается дважды, прямым и
/// обратным порядком блоков, и результаты сравниваются — зависимость от
/// порядка была одним из дефектов, на которых остановилась предыдущая
/// попытка анализа в этом проекте. И наконец, ни одна копия, признанная
/// устранимой, не смеет трогать неточный регистр.
///
/// # Errors
///
/// Возвращает описание первого нарушенного инварианта.
pub fn verify(chunk: &Chunk, overlap: Option<usize>) -> Result<(), String> {
    let n = chunk.instrs.len();
    if n == 0 {
        return Ok(());
    }
    let cfg = build_cfg(chunk);
    if cfg.blocks.is_empty() {
        return Err("чанк не пуст, а блоков нет".to_string());
    }
    let mut covered = 0usize;
    for (b, &(lo, hi)) in cfg.blocks.iter().enumerate() {
        if lo >= hi || hi > n {
            return Err(format!("блок {b}: границы {lo}..{hi} вне чанка длиной {n}"));
        }
        covered += hi - lo;
        for pc in lo..hi {
            if cfg.block_of[pc] != b {
                return Err(format!(
                    "инструкция {pc} приписана блоку {}, а лежит в {b}",
                    cfg.block_of[pc]
                ));
            }
        }
        for &s in &cfg.succs[b] {
            if s >= cfg.blocks.len() {
                return Err(format!("блок {b}: преемник {s} не существует"));
            }
        }
    }
    if covered != n {
        return Err(format!("блоки покрывают {covered} инструкций из {n}"));
    }

    let a = live_out_in_order(chunk, &cfg, overlap, true);
    let b = live_out_in_order(chunk, &cfg, overlap, false);
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if !x.equals(y) {
            return Err(format!("живучесть блока {i} зависит от порядка обхода"));
        }
    }

    let exact = |r: u8| {
        let r = r as usize;
        let aliased =
            r < chunk.n_params as usize && !chunk.param_by_val.get(r).copied().unwrap_or(false);
        !aliased && !overlap.is_some_and(|k| r < k)
    };

    // Перекрёстная проверка второй поопкодной таблицы против этой:
    // `Instr::rewrite_read_reg` обязана менять ровно те регистры, которые
    // здесь названы прочитанными, за вычетом позиционных (окно аргументов
    // читается по смещению) и записываемых (счётчик цикла читается И
    // пишется, переименовать половину нельзя). Без этой сверки две
    // таблицы разъедутся на первом же новом опкоде.
    for (pc, instr) in chunk.instrs.iter().enumerate() {
        // Два опкода исключены намеренно, и по разным причинам.
        // `RunDynamic` видит именованные локали кадра по ИМЕНАМ, поэтому
        // переименование его операнда запрещено, хотя чтением он и
        // является. У `NumericForNext*` счётчик читается и пишется ОДНИМ
        // операндом: переименовать чтение, не тронув запись, нельзя, а
        // выразить это через множества эффектов — нельзя тем более,
        // потому что у обычного `Add r1, r1, r2` чтение и запись живут в
        // разных операндах и переименование законно.
        if matches!(
            instr,
            Instr::RunDynamic { .. }
                | Instr::NumericForNext { .. }
                | Instr::NumericForNextI64 { .. }
        ) {
            continue;
        }
        let e = effects(instr, chunk, overlap);
        for r in 0..=u8::MAX {
            if !exact(r) {
                continue;
            }
            let mut probe = *instr;
            probe.rewrite_read_reg(r, r.wrapping_add(1));
            let rewritten = probe != *instr;
            let expected = e.reads.contains(r as usize) && !e.reads_positional.contains(r as usize);
            if rewritten != expected {
                return Err(format!(
                    "инструкция {pc} ({}): перезапись регистра {r} даёт {rewritten}, \
                     а классификация эффектов ожидает {expected}",
                    instr.opcode()
                ));
            }
        }
    }

    for (pc, flag) in removable_copies(chunk, overlap).iter().enumerate() {
        if !flag {
            continue;
        }
        match chunk.instrs[pc] {
            Instr::Move { dst, src } => {
                if !exact(dst) || !exact(src) {
                    return Err(format!(
                        "копия {pc}: устранимой признана копия неточного регистра"
                    ));
                }
            }
            _ => return Err(format!("инструкция {pc} помечена копией, но это не Move")),
        }
    }
    Ok(())
}

/// Снять устранимые копии: переименовать чтения приёмника на источник и
/// удалить сам `Move`.
///
/// Возвращает число удалённых инструкций. Разметку бандлов НЕ трогает:
/// она производная и пересчитывается вызывающим через `bundle::compute`.
///
/// За раунд анализ считается ОДИН раз, а применяется столько копий,
/// сколько заведомо не мешают друг другу. Мешать они могут одним
/// способом: удаление `Move` убирает запись в его приёмник, а именно
/// такая запись могла быть основанием считать мёртвой другую копию.
/// Поэтому в раунд берутся копии, чьи участки переименования не
/// пересекаются, — тогда ни одна не опирается на инструкцию, которую
/// снимает соседняя. Раунды повторяются, пока хоть что-то снимается;
/// на практике их единицы, и полный пересчёт на каждую снятую копию
/// (а это квадрат по их числу) не нужен.
pub fn copy_propagate(chunk: &mut Chunk, overlap: Option<usize>) -> usize {
    let mut removed = 0usize;
    loop {
        let flags = removable_copies(chunk, overlap);
        let cfg = build_cfg(chunk);
        let boundary = |pc: usize| {
            chunk
                .exception_ranges
                .iter()
                .any(|r| r.start_pc == pc || r.end_pc == pc || r.handler_pc == pc)
        };

        // Отбор раунда: участок [копия, конец переименования] не должен
        // пересекаться с уже принятым.
        let mut picked: Vec<(usize, u8, u8, usize)> = Vec::new();
        let mut busy_until = 0usize;
        for (i, &f) in flags.iter().enumerate() {
            if !f || boundary(i) || i < busy_until {
                continue;
            }
            let Instr::Move { dst, src } = chunk.instrs[i] else {
                continue;
            };
            let (_, hi) = cfg.blocks[cfg.block_of[i]];
            let mut stop = hi;
            for pc in (i + 1)..hi {
                if effects(&chunk.instrs[pc], chunk, overlap)
                    .writes
                    .contains(dst as usize)
                {
                    stop = pc + 1;
                    break;
                }
            }
            picked.push((i, dst, src, stop));
            busy_until = stop;
        }
        if picked.is_empty() {
            return removed;
        }

        // Применяем с конца: удаление сдвигает только то, что за ним.
        for &(i, dst, src, stop) in picked.iter().rev() {
            for pc in (i + 1)..stop {
                chunk.instrs[pc].rewrite_read_reg(dst, src);
            }
            chunk.instrs.remove(i);
            chunk.prop_cache.remove(i);
            chunk.method_cache.remove(i);
            if i < chunk.bundle_len.len() {
                chunk.bundle_len.remove(i);
            }
            for instr in &mut chunk.instrs {
                if let Some(t) = instr.jump_target()
                    && let Ok(t) = usize::try_from(t)
                    && t > i
                    && let Ok(nt) = i16::try_from(t - 1)
                {
                    instr.set_jump_target(nt);
                }
            }
            let shift = |pc: &mut usize| {
                if *pc > i {
                    *pc -= 1;
                }
            };
            for r in &mut chunk.exception_ranges {
                shift(&mut r.start_pc);
                shift(&mut r.end_pc);
                shift(&mut r.handler_pc);
            }
            removed += 1;
        }
    }
}

/// Свернуть арифметику над заведомо известными константами.
///
/// Возвращает число свёрнутых инструкций. Инструкции НЕ удаляются, а
/// заменяются на `LoadConst`, поэтому ни адреса, ни диапазоны `Попытка`,
/// ни инлайн-кэши не сдвигаются — этот проход не требует пересчёта `pc`
/// и потому проверяется куда проще устранения копий.
///
/// Границы намеренно узкие, и каждая имеет причину:
///
/// - сворачиваются только `Add`, `Sub`, `Mul`, `Div`, `Mod`, и только
///   когда ОБА операнда — числа. Арифметика VM приводит строки и булевы
///   к числу в обёртке над самой операцией (`"5" - 1` измерено на
///   платформе), а обёртка живёт в `bsl-vm`; повторять её здесь значило
///   бы завести второй экземпляр правил приведения, который однажды
///   разойдётся с первым;
/// - операция, вернувшая ошибку, не сворачивается: `1 / 0` обязано
///   бросить на исполнении, а не на компиляции. Ровно поэтому замена
///   безопасна и внутри `Попытка` — сворачивается только то, что и на
///   исполнении не бросило бы;
/// - известным считается лишь регистр, чья последняя запись в ЭТОМ блоке
///   — `LoadConst`. Ни вызова, ни записи через алиас или модульный слот
///   между записью и использованием быть не должно: такие эффекты
///   стирают всю таблицу известных.
pub fn const_propagate(chunk: &mut Chunk, overlap: Option<usize>) -> usize {
    let mut folded = 0usize;
    let cfg = build_cfg(chunk);
    for &(lo, hi) in &cfg.blocks {
        // Регистр -> номер константы, известной на этой позиции.
        let mut known: Vec<Option<u16>> = vec![None; 256];
        for pc in lo..hi {
            let e = effects(&chunk.instrs[pc], chunk, overlap);
            if e.writes_alias || e.mod_writes.is_some() || e.heap_write {
                known.iter_mut().for_each(|k| *k = None);
            }

            if let Instr::Add { dst, a, b }
            | Instr::Sub { dst, a, b }
            | Instr::Mul { dst, a, b }
            | Instr::Div { dst, a, b }
            | Instr::Mod { dst, a, b } = chunk.instrs[pc]
                && let (Some(ka), Some(kb)) = (
                    known.get(a as usize).copied().flatten(),
                    known.get(b as usize).copied().flatten(),
                )
                && let (Some(va), Some(vb)) =
                    (chunk.consts.get(ka as usize), chunk.consts.get(kb as usize))
                && matches!(va, bsl_rt::BslValue::Number(_))
                && matches!(vb, bsl_rt::BslValue::Number(_))
            {
                let out = match chunk.instrs[pc] {
                    Instr::Add { .. } => va.add(vb),
                    Instr::Sub { .. } => va.sub(vb),
                    Instr::Mul { .. } => va.mul(vb),
                    Instr::Div { .. } => va.div(vb),
                    _ => va.rem(vb),
                };
                if let Ok(v) = out
                    && let Some(k) = intern_const(chunk, v)
                {
                    chunk.instrs[pc] = Instr::LoadConst { dst, k };
                    folded += 1;
                }
            }

            // Учёт записи идёт ПОСЛЕ возможной замены: у свёрнутой
            // инструкции приёмник теперь тоже известен.
            match chunk.instrs[pc] {
                Instr::LoadConst { dst, k } => {
                    if let Some(slot) = known.get_mut(dst as usize) {
                        *slot = Some(k);
                    }
                }
                _ => {
                    for r in 0..=u8::MAX {
                        if e.writes.contains(r as usize)
                            && let Some(slot) = known.get_mut(r as usize)
                        {
                            *slot = None;
                        }
                    }
                }
            }
        }
    }
    folded
}

/// Добавить константу в таблицу чанка и вернуть её номер. `None` — таблица
/// переполнена (номера шестнадцатибитные), и тогда свёртка не делается.
///
/// Дедупликации здесь намеренно нет. Единственное готовое сравнение
/// значений — `bsl_rt::folded_eq` — реализует РАВЕНСТВО BSL, а оно
/// приводит типы и складывает регистр строк; склеивать по нему записи
/// таблицы констант значило бы менять то, что читает `LoadConst`. Рост
/// ограничен по построению: свёртка добавляет не больше одной константы
/// на инструкцию и выполняется один раз на компиляции.
fn intern_const(chunk: &mut Chunk, v: bsl_rt::BslValue) -> Option<u16> {
    let i = u16::try_from(chunk.consts.len()).ok()?;
    chunk.consts.push(v);
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn a_copy_dying_inside_its_block_is_removable() {
        // r1 читается один раз и дальше не живёт: чтение можно направить
        // прямо на r0, а копию снять.
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
            Instr::Neg { dst: 2, src: 1 },
            Instr::Return { src: Some(2) },
        ]);
        assert!(removable_copies(&c, None)[1]);
    }

    #[test]
    fn a_copy_whose_source_is_overwritten_stays() {
        // Между копией и чтением r1 источник переписан: направить чтение
        // на r0 значило бы прочитать другое значение.
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
            Instr::LoadConst { dst: 0, k: 1 },
            Instr::Neg { dst: 2, src: 1 },
            Instr::Return { src: Some(2) },
        ]);
        assert!(!removable_copies(&c, None)[1]);
    }

    #[test]
    fn a_copy_alive_past_the_block_stays() {
        // r1 жив на выходе из блока: значение держится копией, и локальный
        // проход её снять не может.
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
            Instr::Jump { target: 3 },
            Instr::Neg { dst: 2, src: 1 },
            Instr::Return { src: Some(2) },
        ]);
        assert!(!removable_copies(&c, None)[1]);
    }

    #[test]
    fn a_copy_of_a_byref_parameter_stays() {
        // Параметр без `Знач` — не собственная ячейка кадра: номер
        // регистра не называет её однозначно.
        let mut c = chunk(vec![
            Instr::Move { dst: 3, src: 0 },
            Instr::Neg { dst: 4, src: 3 },
            Instr::Return { src: Some(4) },
        ]);
        c.n_params = 1;
        c.param_by_val = vec![false];
        assert!(!removable_copies(&c, None)[0]);
    }

    #[test]
    fn a_copy_overlapping_a_module_slot_stays() {
        // В чанке верхнего уровня слоты модуля накладываются на регистры
        // кадра, поэтому r0 — ещё и модульная переменная.
        let c = chunk(vec![
            Instr::Move { dst: 1, src: 0 },
            Instr::Neg { dst: 2, src: 1 },
            Instr::Return { src: Some(2) },
        ]);
        assert!(!removable_copies(&c, Some(2))[0]);
    }

    #[test]
    fn a_call_between_copy_and_death_stops_the_estimate() {
        // Вызов может тронуть модульные слоты и чужие кадры: оценка
        // обязана остаться нижней границей.
        let c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
            Instr::Call {
                func: 0,
                base: 5,
                arg_modes: 0,
                ret: 6,
            },
            Instr::Neg { dst: 2, src: 1 },
            Instr::Return { src: Some(2) },
        ]);
        assert!(!removable_copies(&c, None)[1]);
    }

    #[test]
    fn a_copy_inside_a_protected_range_stays() {
        // Регрессия: исключение может сработать МЕЖДУ копией и записью,
        // которая якобы делает её мёртвой, и тогда обработчик увидит
        // именно скопированное значение. Прежняя редакция считала такую
        // копию устранимой, потому что исключительное ребро моделировала
        // только с конца блока.
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::Move { dst: 1, src: 0 },
            Instr::Div { dst: 2, a: 0, b: 0 },
            Instr::LoadConst { dst: 1, k: 1 },
            Instr::Return { src: Some(1) },
        ]);
        c.exception_ranges = vec![crate::chunk::ExceptionRange {
            start_pc: 0,
            end_pc: 4,
            handler_pc: 4,
        }];
        assert!(!removable_copies(&c, None)[1]);
    }

    fn num(v: i64) -> bsl_rt::BslValue {
        bsl_rt::BslValue::Number(bsl_number::BslNumber::from_i64(v))
    }

    #[test]
    fn arithmetic_over_two_constants_folds() {
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 1 },
            Instr::Add { dst: 2, a: 0, b: 1 },
            Instr::Return { src: Some(2) },
        ]);
        c.consts = vec![num(2), num(3)];
        assert_eq!(const_propagate(&mut c, None), 1);
        let Instr::LoadConst { dst: 2, k } = c.instrs[2] else {
            panic!("сложение не свернулось в константу");
        };
        assert_eq!(c.consts[k as usize], num(5));
    }

    #[test]
    fn a_failing_operation_is_left_to_run_time() {
        // `1 / 0` обязано бросить на исполнении, а не исчезнуть на
        // компиляции: иначе `Попытка` вокруг него перестанет срабатывать.
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 1 },
            Instr::Div { dst: 2, a: 0, b: 1 },
            Instr::Return { src: Some(2) },
        ]);
        c.consts = vec![num(1), num(0)];
        assert_eq!(const_propagate(&mut c, None), 0);
        assert!(matches!(c.instrs[2], Instr::Div { .. }));
    }

    #[test]
    fn a_call_between_the_constant_and_its_use_stops_folding() {
        // Вызов может тронуть модульные слоты и чужие кадры, поэтому
        // таблица известных обнуляется целиком.
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 1 },
            Instr::Call {
                func: 0,
                base: 5,
                arg_modes: 0,
                ret: 6,
            },
            Instr::Add { dst: 2, a: 0, b: 1 },
            Instr::Return { src: Some(2) },
        ]);
        c.consts = vec![num(2), num(3)];
        assert_eq!(const_propagate(&mut c, None), 0);
    }

    #[test]
    fn a_non_numeric_constant_is_not_folded() {
        // Приведение строк и булевых к числу живёт в обёртке VM; второй
        // экземпляр этих правил здесь заводить нельзя.
        let mut c = chunk(vec![
            Instr::LoadConst { dst: 0, k: 0 },
            Instr::LoadConst { dst: 1, k: 1 },
            Instr::Sub { dst: 2, a: 0, b: 1 },
            Instr::Return { src: Some(2) },
        ]);
        c.consts = vec![bsl_rt::BslValue::Str(bsl_rt::BslString::from("5")), num(1)];
        assert_eq!(const_propagate(&mut c, None), 0);
    }
}
