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

use crate::chunk::Chunk;
use crate::instr::{ArgMode, Instr};

/// Ширина бандла ограничена ёмкостью `u8` в [`Chunk::bundle_len`].
pub const MAX_BUNDLE_LEN: usize = u8::MAX as usize;

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
struct RegSet([u64; 4]);

impl RegSet {
    fn insert(&mut self, r: usize) {
        if r < 256 {
            self.0[r / 64] |= 1 << (r % 64);
        }
    }

    fn insert_range(&mut self, base: usize, count: usize) {
        // Насыщение на 256 корректно в консервативную сторону: чанк не от
        // кодогена может заявить диапазон шире регистрового файла.
        for r in base..(base + count).min(256) {
            self.insert(r);
        }
    }

    fn intersects(&self, other: &RegSet) -> bool {
        self.0.iter().zip(other.0.iter()).any(|(a, b)| a & b != 0)
    }

    fn union(&mut self, other: &RegSet) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a |= b;
        }
    }
}

/// Обращения одной инструкции к модульным слотам. `All` — у `Call` и
/// `RunDynamic`: вызванный код может транзитивно тронуть любой слот.
#[derive(Clone, Copy, PartialEq)]
enum ModSet {
    None,
    One(u16),
    All,
}

impl ModSet {
    fn is_some(self) -> bool {
        !matches!(self, ModSet::None)
    }
}

/// Класс передачи управления: `Trailing` может стоять только последним
/// членом бандла, `Barrier` — всегда одиночный.
#[derive(Clone, Copy, PartialEq)]
enum Ctl {
    None,
    Trailing,
    Barrier,
}

/// Эффекты одной инструкции в терминах пространств из модульного дока.
#[derive(Clone, Copy)]
struct Eff {
    reads: RegSet,
    writes: RegSet,
    reads_alias: bool,
    writes_alias: bool,
    mod_reads: ModSet,
    mod_writes: ModSet,
    heap_read: bool,
    heap_write: bool,
    io: bool,
    ctl: Ctl,
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

/// Эффекты инструкции. `overlap` — см. [`module_overlap`]; `chunk` нужен
/// ради `call_arg_modes` (скрытые операнды `Call`) и режимов параметров.
fn effects(instr: &Instr, chunk: &Chunk, overlap: Option<usize>) -> Eff {
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
        // из них запись.
        Instr::GetImportedVar { dst, .. } => {
            write!(dst);
            e.heap_read = true;
        }
        Instr::SetImportedVar { src, .. } => {
            read!(src);
            e.heap_write = true;
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
