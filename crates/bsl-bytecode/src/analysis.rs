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
    macro_rules! read_range {
        ($base:expr, $count:expr) => {
            for r in ($base as usize)..(($base as usize) + ($count as usize)).min(256) {
                if is_alias(r as u8) {
                    e.reads_alias = true;
                } else {
                    e.reads.insert(r);
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
                                read!(r);
                            }
                            // Вызванный видит слот по ссылке: и читает, и
                            // может записать.
                            ArgMode::ByRefLocal(slot) => {
                                read!(*slot);
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
                                read!(r);
                            }
                            ArgMode::ByRefLocal(slot) => {
                                read!(*slot);
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
        }
    }
    e
}
