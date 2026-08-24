use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use crate::fold::{PassHasher, folded_eq, folded_hash};

/// Корзина одного свёрнутого хеша. Коллизии настолько редки, что
/// обычный случай не должен платить ни аллокацией, ни прыжком по
/// указателю за возможность их пережить.
#[derive(Debug)]
enum Bucket {
    One(NameId),
    Many(Vec<NameId>),
}

impl Bucket {
    fn ids(&self) -> &[NameId] {
        match self {
            Bucket::One(id) => std::slice::from_ref(id),
            Bucket::Many(ids) => ids,
        }
    }

    fn push(&mut self, id: NameId) {
        match self {
            Bucket::One(first) => *self = Bucket::Many(vec![*first, id]),
            Bucket::Many(ids) => ids.push(id),
        }
    }
}

/// Идентификатор интернированного имени поля. Сравнение имён при доступе к
/// полю сводится к сравнению `NameId` (`u32`), а не строк.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(u32);

impl NameId {
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Обратная сборка из индекса. Нужна ровно одному потребителю —
    /// загрузчику текстового байт-кода (`bsl_bytecode::text`), который
    /// читает уже готовую таблицу имён из файла и восстанавливает по ней
    /// `NameId` инструкций. Ни компилятору, ни VM это не нужно: там
    /// `NameId` всегда приходит из интернера.
    pub fn from_index(index: u32) -> Self {
        NameId(index)
    }
}

/// Интернер имён полей. Используется компилятором байт-кода (не VM в
/// рантайме): все имена полей известны на этапе компиляции модуля, поэтому
/// интернирование — целиком компиляционный процесс, а VM только читает
/// готовую таблицу `Program::names` по индексу для сообщений об ошибках.
///
/// Доступ к полю регистронезависим (`а.ИМЯ` и `а.имя` — одно поле), но
/// оригинальное написание первого вхождения сохраняется.
/// Индекс — корзины по числовому свёрнутому ключу (`folded_hash`):
/// ни поиск, ни вставка не строят верхнерегистровую строку. Хеш не
/// инъективен, поэтому корзина хранит ВСЕХ претендентов, и попадание
/// доказывается настоящим сравнением (`folded_eq`) с сохранённым
/// написанием — коллизия чисел не может склеить два разных имени.
#[derive(Debug)]
pub struct NameInterner {
    names: Vec<String>,
    index: HashMap<u64, Bucket, BuildHasherDefault<PassHasher>>,
    /// Номер поколения — общий счётчик всех интернеров процесса. Ключ
    /// валидности `NameId`, закэшированного в строке ([`BslString`]):
    /// идентификатор из интернера другого поколения там недействителен.
    generation: u32,
}

/// Источник поколений. Атомарный не ради потоков (рантайм однопоточный),
/// а потому что это единственный безопасный `static`-счётчик без
/// `unsafe`.
static NEXT_GENERATION: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

impl Default for NameInterner {
    fn default() -> Self {
        NameInterner {
            names: Vec::new(),
            index: HashMap::default(),
            generation: NEXT_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl NameInterner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Интернирование строки BSL с кэшем результата в ней самой:
    /// повторный `Вставить`/`Свойство` с тем же объектом-ключом — это
    /// чтение одной ячейки, без свёртки, хеша и UTF-8-копии.
    pub fn intern_bsl(&mut self, s: &crate::BslString) -> NameId {
        if let Some(id) = s.cached_name_id(self.generation) {
            return id;
        }
        let id = self.intern(&s.to_string());
        s.cache_name_id(self.generation, id);
        id
    }

    fn find_in_bucket(&self, hash: u64, name: &str) -> Option<NameId> {
        self.index
            .get(&hash)?
            .ids()
            .iter()
            .copied()
            .find(|id| folded_eq(&self.names[id.index()], name))
    }

    /// Затравка рантайм-интернера уже готовой (компиляционной) таблицей
    /// имён — `Вставить`/`Свойство` на структуре могут получить строковый
    /// ключ, которого не было ни в одном литерале модуля, и им нужно
    /// продолжить ИМЕННО эту нумерацию `NameId`, а не начать с нуля заново
    /// (иначе рантайм-`NameId` столкнулись бы с компиляционными).
    pub fn from_existing(names: Vec<String>) -> Self {
        let mut interner = NameInterner {
            names: Vec::new(),
            index: HashMap::with_capacity_and_hasher(names.len(), Default::default()),
            generation: NEXT_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
        for (i, n) in names.iter().enumerate() {
            interner.insert_id(folded_hash(n), NameId(i as u32));
        }
        interner.names = names;
        interner
    }

    fn insert_id(&mut self, hash: u64, id: NameId) {
        match self.index.entry(hash) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(Bucket::One(id));
            }
            std::collections::hash_map::Entry::Occupied(mut slot) => slot.get_mut().push(id),
        }
    }

    pub fn intern(&mut self, name: &str) -> NameId {
        let hash = folded_hash(name);
        if let Some(id) = self.find_in_bucket(hash, name) {
            return id;
        }
        let id = NameId(self.names.len() as u32);
        self.names.push(name.to_string());
        self.insert_id(hash, id);
        id
    }

    /// Поиск БЕЗ интернирования: `None` — такого имени в таблице ещё нет.
    ///
    /// Нужен `ЗаполнитьЗначенияСвойств`, который проверяет, есть ли у
    /// приёмника поле с именем свойства источника. Поле живой структуры
    /// всегда уже интернировано (иначе его нечем было бы завести), поэтому
    /// отсутствие имени в таблице — это и есть ответ «такого поля нет».
    /// Через [`intern`](Self::intern) тот же вопрос задавать нельзя: он
    /// растил бы таблицу на каждое чужое имя, а имена не выселяются.
    pub fn lookup(&self, name: &str) -> Option<NameId> {
        self.find_in_bucket(folded_hash(name), name)
    }

    /// Оригинальное написание имени. Нужно там, где `NameId` приходится
    /// возвращать пользовательскому коду обратно СТРОКОЙ — `Для Каждого` по
    /// `Структура` отдаёт `КлючИЗначение.Ключ` как `Строка`, а внутри поле
    /// хранится только идентификатором.
    pub fn name(&self, id: NameId) -> Option<&str> {
        self.names.get(id.index()).map(|s| s.as_str())
    }

    /// Готовая таблица `NameId -> оригинальное написание`, для встраивания
    /// в `Program`.
    pub fn into_names(self) -> Vec<String> {
        self.names
    }
}

/// Есть ли в списке два имени, равных ПО СВЁРТКЕ, — и какое из них второе.
///
/// Таблица имён образа обязана быть биекцией `NameId <-> имя`: её строит
/// интернер, у которого одно написание даёт один идентификатор. Повтор
/// ломает эту биекцию — одно написание начинает адресовать два разных поля,
/// и обращение по имени попадает в первое из них.
///
/// Правило свёртки здесь ровно то же, что и у самого интернирования
/// (свёрнутый хэш плюс подтверждение [`folded_eq`] внутри корзины), потому
/// что второе правило рано или поздно разошлось бы с первым. Проход
/// линейный: сравнения идут только внутри корзины одного хэша, а не
/// каждого с каждым.
pub fn first_folded_duplicate(names: &[String]) -> Option<usize> {
    let mut buckets: HashMap<u64, Vec<usize>, BuildHasherDefault<PassHasher>> =
        HashMap::with_capacity_and_hasher(names.len(), Default::default());
    for (i, name) in names.iter().enumerate() {
        let bucket = buckets.entry(folded_hash(name)).or_default();
        if bucket
            .iter()
            .any(|&earlier| folded_eq(&names[earlier], name))
        {
            return Some(i);
        }
        bucket.push(i);
    }
    None
}
