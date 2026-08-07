use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{hash_map::RandomState, HashMap};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

use bsl_number::{BslNumber, NumError};

use crate::string::BslString;
use crate::BslValue;

/// `ТаблицаЗначений` — колоночное хранение: каждая колонка своя
/// `Vec<BslValue>`, а не `Vec` строк-объектов. Ускоряет то, что реально
/// работает по колонкам (`Сортировать`, `Свернуть`, `Итог`,
/// `ВыгрузитьКолонку`) — ни один из которых в этом заходе ещё не сделан,
/// но хранение уже рассчитано на них.
///
/// Плата за колоночность — идентичность строк: `СтрокаТаблицы` должна
/// пережить сортировку/удаление СОСЕДНИХ строк, а физическая позиция
/// строки в колонках как раз меняется при таких операциях. Решение — то
/// же, что описано в брифе: строка хранит не позицию, а стабильный
/// `row_id`; таблица держит обратный индекс `row_id -> текущая позиция`.
/// Удалённая строка выпадает из индекса — обращение к ней после этого
/// возвращает `RtError::RowInvalidated`, а не тихо читает чужие данные.
#[derive(Debug)]
pub struct ValueTableData {
    /// Имена колонок сравниваются регистронезависимо снаружи (см.
    /// `column_index`), но хранятся с оригинальным написанием.
    pub column_names: Vec<String>,
    /// Ограничение типа каждой колонки. `None` означает составной тип без
    /// явного ограничения, как у `Колонки.Добавить(Имя)`.
    pub column_types: Vec<Option<Vec<ColumnType>>>,
    /// Транзитные атрибуты каждой колонки для внутреннего формата — по
    /// одному на колонку, всегда той же длины, что `column_names`.
    pub column_vstr: Vec<ColumnVstr>,
    /// Сырая предпоследняя лексема хвоста блока строк внутреннего формата
    /// (`…,X,Y}`), сохранённая при чтении ради тождественного транзита.
    /// Смысл не реверсирован: на свежих таблицах и после удаления колонок
    /// она равна максимальному идентификатору колонки, но у таблиц,
    /// полученных `Скопировать` со списком колонок, платформа пишет `-1`.
    pub vstr_tail_x: Option<String>,
    /// `columns[col][pos]` — значение колонки `col` в строке на текущей
    /// физической позиции `pos`. Все колонки всегда одной длины — длины
    /// строк таблицы.
    pub columns: Vec<Vec<BslValue>>,
    /// `row_ids[pos]` — стабильный id строки, сейчас стоящей на позиции
    /// `pos`.
    pub row_ids: Vec<u64>,
    /// Обратный индекс выбирает плотное или разреженное представление в
    /// зависимости от заполненности пространства `row_id`.
    row_positions: RowPositions,
    next_id: u64,
    /// Меняется при каждой перестройке набора или порядка колонок. Нужна
    /// кэшу прямого переноса строк: один и тот же объект таблицы после
    /// `Свернуть` уже имеет другую схему и старый план индексов неприменим.
    schema_revision: u64,
    /// Свёрнутые образы имён колонок для `column_index` — ленивые, с
    /// ревизией схемы на момент сборки. Имена мутируют только методы
    /// этой структуры, и каждый из них поднимает `schema_revision`,
    /// поэтому устаревший кэш просто пересобирается при следующем
    /// поиске, а ручной синхронизации по местам мутаций нет.
    folded_names: RefCell<Option<FoldedNames>>,
}

/// Кэш свёрнутых имён колонок: ревизия схемы на момент сборки и образы.
#[derive(Debug)]
struct FoldedNames {
    revision: u64,
    names: Vec<Box<[u8]>>,
}

const MISSING_POSITION: usize = usize::MAX;

/// Один тип из `ОписаниеТипов` колонки вместе с квалификаторами — сырыми
/// лексемами внутреннего формата (`{"N",10,2,0}` несёт `["10","2","0"]`).
/// Семантика квалификаторов применяется приведением значений
/// ([`ValueTableData::adjust_to_column_type`]); написание — транзитом.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnType {
    pub id: crate::TypeId,
    pub quals: Vec<String>,
}

impl ColumnType {
    pub fn plain(id: crate::TypeId) -> Self {
        ColumnType {
            id,
            quals: Vec::new(),
        }
    }
}

/// Транзитные атрибуты колонки для внутреннего формата
/// (`ЗначениеВСтрокуВнутр`): сырое описание типов с квалификаторами и
/// оформление. Живут только ради тождественного транзита — колонка,
/// прочитанная из строки платформы, обязана записаться обратно байт в
/// байт, — а сама таблица заголовок, ширину и квалификаторы пока не
/// использует.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColumnVstr {
    /// Стабильный идентификатор колонки из исходной строки: у платформы
    /// колонки, как и строки, несут внутренние номера, и после удаления
    /// колонок номера разрежены (видно на реальных выгрузках). `None` —
    /// колонка создана кодом, идентификатором служит её позиция.
    pub id: Option<String>,
    /// Канонически отрисованное `{"Pattern",…}` из исходной строки;
    /// `None` — колонка создана кодом, и описание строится из
    /// `column_types`.
    pub raw_pattern: Option<String>,
    /// Заголовок колонки; пустая строка — по умолчанию.
    pub title: String,
    /// Ширина колонки сырой лексемой; пустая строка — «0» по умолчанию.
    pub width: String,
}

/// Обратный индекс стабильных идентификаторов строк.
///
/// Последовательная таблица обходится пустым `Sparse`: `pos_of` сначала
/// проверяет прямое равенство `row_id == position`. После сортировки id
/// остаются плотными, но меняют порядок — тогда `Dense` занимает один
/// `usize` на строку вместо полного бакета `HashMap`. После массовых
/// удалений выгоднее снова разреженная карта.
#[derive(Debug)]
enum RowPositions {
    Sparse(HashMap<u64, usize>),
    Dense(Vec<usize>),
}

impl RowPositions {
    fn identity() -> Self {
        Self::Sparse(HashMap::new())
    }

    fn get(&self, row_id: u64) -> Option<usize> {
        match self {
            Self::Sparse(positions) => positions.get(&row_id).copied(),
            Self::Dense(positions) => positions
                .get(usize::try_from(row_id).ok()?)
                .copied()
                .filter(|position| *position != MISSING_POSITION),
        }
    }

    fn insert(&mut self, row_id: u64, position: usize) {
        match self {
            Self::Sparse(positions) => {
                positions.insert(row_id, position);
            }
            Self::Dense(positions) => {
                let Ok(index) = usize::try_from(row_id) else {
                    let mut sparse = HashMap::with_capacity(positions.len());
                    for (id, &position) in positions.iter().enumerate() {
                        if position != MISSING_POSITION {
                            sparse.insert(id as u64, position);
                        }
                    }
                    sparse.insert(row_id, position);
                    *self = Self::Sparse(sparse);
                    return;
                };
                if index >= positions.len() {
                    positions.resize(index + 1, MISSING_POSITION);
                }
                positions[index] = position;
            }
        }
    }

    fn remove(&mut self, row_id: u64) {
        match self {
            Self::Sparse(positions) => {
                positions.remove(&row_id);
            }
            Self::Dense(positions) => {
                if let Ok(index) = usize::try_from(row_id) {
                    if let Some(position) = positions.get_mut(index) {
                        *position = MISSING_POSITION;
                    }
                }
            }
        }
    }

    fn rebuild(row_ids: &[u64], next_id: u64) -> Self {
        if row_ids
            .iter()
            .enumerate()
            .all(|(position, row_id)| usize::try_from(*row_id).ok() == Some(position))
        {
            return Self::identity();
        }

        if let Ok(dense_len) = usize::try_from(next_id) {
            if dense_len <= row_ids.len().saturating_mul(2) {
                let mut positions = vec![MISSING_POSITION; dense_len];
                for (position, &row_id) in row_ids.iter().enumerate() {
                    positions[row_id as usize] = position;
                }
                return Self::Dense(positions);
            }
        }

        Self::Sparse(
            row_ids
                .iter()
                .enumerate()
                .filter(|(position, row_id)| usize::try_from(**row_id).ok() != Some(*position))
                .map(|(position, &row_id)| (row_id, position))
                .collect(),
        )
    }
}

/// Конверсия значения в целевой примитивный тип; `None` — не конвертится.
/// Реализованы только измеренные пары (`ADJ.*`): всё прочее — «не
/// конвертится», что уводит в значение по умолчанию либо `Неопределено`.
fn convert_to(value: &BslValue, target: crate::TypeId) -> Option<BslValue> {
    match (value, target) {
        (BslValue::Number(n), crate::TypeId::String) => {
            Some(BslValue::Str(BslString::from_str(&n.to_canonical())))
        }
        // `Истина` в колонке строк — локализованное представление «Да»
        // (измерено `ADJ.S.BOOL`); тот же текст даёт `Display`.
        (BslValue::Boolean(b), crate::TypeId::String) => {
            Some(BslValue::Str(BslString::from_str(if *b {
                "Да"
            } else {
                "Нет"
            })))
        }
        (BslValue::Str(s), crate::TypeId::Number) => BslNumber::parse_canonical(&s.to_string())
            .ok()
            .map(BslValue::Number),
        (BslValue::Boolean(b), crate::TypeId::Number) => {
            Some(BslValue::Number(BslNumber::from_i64(i64::from(*b))))
        }
        (BslValue::Number(n), crate::TypeId::Boolean) => {
            Some(BslValue::Boolean(*n != BslNumber::from_i64(0)))
        }
        _ => None,
    }
}

/// Значение по умолчанию типа — им кончается неудачная конверсия в
/// колонке с единственным типом (измерено: `ADJ.B.STR` — `Ложь`,
/// `ADJ.N.UNDEF` — `0`).
fn default_of(id: crate::TypeId) -> BslValue {
    match id {
        crate::TypeId::Boolean => BslValue::Boolean(false),
        crate::TypeId::Number => BslValue::Number(BslNumber::from_i64(0)),
        crate::TypeId::String => BslValue::Str(BslString::from_str("")),
        crate::TypeId::Date => crate::date::BslDate::from_civil(1, 1, 1, 0, 0, 0)
            .map(BslValue::Date)
            .unwrap_or(BslValue::Undefined),
        _ => BslValue::Undefined,
    }
}

/// Квалификаторы типа: число — округление дробной части и насыщение целой,
/// строка — обрезка по длине, дата — обнуление «чужих» частей.
fn apply_qualifiers(value: BslValue, t: &ColumnType) -> BslValue {
    match (&value, t.id) {
        (BslValue::Number(n), crate::TypeId::Number) => {
            let digits: u32 = t.quals.first().and_then(|q| q.parse().ok()).unwrap_or(0);
            if digits == 0 {
                return value;
            }
            let frac: u32 = t.quals.get(1).and_then(|q| q.parse().ok()).unwrap_or(0);
            let rounded = n.round_to_scale(frac as i32);
            // Насыщение границей разрядности: 123456.7 в `Число(5,2)` —
            // 999.99 (измерено `ADJ.NQ.HUGE`); отрицательная граница
            // принята симметричной.
            let mut nines = String::new();
            for i in 0..digits {
                if i == digits - frac {
                    nines.push('.');
                }
                nines.push('9');
            }
            let limit =
                BslNumber::parse_canonical(&nines).unwrap_or_else(|_| BslNumber::from_i64(0));
            let neg_limit = limit.neg();
            if rounded.cmp(&limit) == std::cmp::Ordering::Greater {
                BslValue::Number(limit)
            } else if rounded.cmp(&neg_limit) == std::cmp::Ordering::Less {
                BslValue::Number(neg_limit)
            } else {
                BslValue::Number(rounded)
            }
        }
        (BslValue::Str(s), crate::TypeId::String) => {
            let len: usize = t.quals.first().and_then(|q| q.parse().ok()).unwrap_or(0);
            if len == 0 {
                return value;
            }
            let units = s.units();
            if units.len() <= len {
                value
            } else {
                BslValue::Str(BslString::from_units(units[..len].to_vec()))
            }
        }
        (BslValue::Date(d), crate::TypeId::Date) => match t.quals.first().map(String::as_str) {
            // `{"D","D"}` — только дата: время обнуляется (`ADJ.DD.DT`).
            Some("D") => {
                let c = d.to_civil();
                crate::date::BslDate::from_civil(c.year, c.month, c.day, 0, 0, 0)
                    .map(BslValue::Date)
                    .unwrap_or(value)
            }
            // `{"D","T"}` — только время: дата сворачивается к пустой
            // (`ADJ.DV.DT` — `0001-01-01` с исходным временем).
            Some("T") => {
                let c = d.to_civil();
                crate::date::BslDate::from_civil(1, 1, 1, c.hour, c.minute, c.second)
                    .map(BslValue::Date)
                    .unwrap_or(value)
            }
            _ => value,
        },
        _ => value,
    }
}

/// Быстрый хэшер для временного отпечатка строки группировки.
///
/// Начальное состояние случайно для каждого вызова `collapse`, поэтому
/// входные данные не могут заранее подобрать длинную цепочку коллизий.
/// Сама коллизия безопасна: перед объединением строки полный ключ всё
/// равно сравнивается по значениям.
struct GroupHasher(u64);

impl GroupHasher {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn mix(&mut self, value: u64) {
        const MULTIPLIER: u64 = 0x517c_c1b7_2722_0a95;
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for GroupHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, mut bytes: &[u8]) {
        while bytes.len() >= 8 {
            let (word, rest) = bytes.split_at(8);
            let mut array = [0_u8; 8];
            array.copy_from_slice(word);
            self.mix(u64::from_ne_bytes(array));
            bytes = rest;
        }
        if !bytes.is_empty() {
            let mut tail = [0_u8; 8];
            tail[..bytes.len()].copy_from_slice(bytes);
            self.mix(u64::from_ne_bytes(tail) ^ ((bytes.len() as u64) << 56));
        }
    }

    fn write_u16(&mut self, value: u16) {
        self.mix(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
}

/// `HashMap` для уже готовых отпечатков не должна повторно считать SipHash.
/// Сам отпечаток засолен случайным состоянием `collapse`, а совпадения всё
/// равно подтверждаются полным сравнением ключа.
#[derive(Default)]
struct FingerprintHasher(u64);

impl Hasher for FingerprintHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // В этой карте ключ всегда `u64`; реализация нужна только для
        // полноты контракта `Hasher` и сохраняет все байты при ином вызове.
        let mut value = 0_u64;
        for (shift, byte) in bytes.iter().take(8).enumerate() {
            value |= u64::from(*byte) << (shift * 8);
        }
        self.0 = value;
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

type FingerprintMap = HashMap<u64, usize, BuildHasherDefault<FingerprintHasher>>;

impl ValueTableData {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(ValueTableData {
            folded_names: RefCell::new(None),
            column_names: Vec::new(),
            column_types: Vec::new(),
            column_vstr: Vec::new(),
            vstr_tail_x: None,
            columns: Vec::new(),
            row_ids: Vec::new(),
            row_positions: RowPositions::identity(),
            next_id: 0,
            schema_revision: 0,
        }))
    }

    pub fn row_count(&self) -> usize {
        self.row_ids.len()
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        // Свёрнутое сравнение, а не `eq_ignore_ascii_case`: регистр
        // кириллицы платформа тоже не различает (измерено, фикстура
        // `table-column-case`). Запрос сворачивается один раз в буфер на
        // стеке и сравнивается memcmp с кэшем образов колонок — свёртка
        // каждой колонки на каждый поиск была видна в профиле fill.
        let folded = self.folded_column_names();
        let mut stack = [0u8; 64];
        match crate::fold::folded_bytes_into(name, &mut stack) {
            Some(query) => folded.iter().position(|c| &**c == query),
            None => {
                let query = crate::fold::folded_bytes(name);
                folded.iter().position(|c| **c == *query)
            }
        }
    }

    /// Кэш свёрнутых имён колонок; пересобирается, если схема менялась.
    fn folded_column_names(&self) -> std::cell::Ref<'_, Vec<Box<[u8]>>> {
        {
            let mut slot = self.folded_names.borrow_mut();
            let stale = !slot
                .as_ref()
                .is_some_and(|cache| cache.revision == self.schema_revision);
            if stale {
                *slot = Some(FoldedNames {
                    revision: self.schema_revision,
                    names: self
                        .column_names
                        .iter()
                        .map(|n| crate::fold::folded_bytes(n).into_boxed_slice())
                        .collect(),
                });
            }
        }
        std::cell::Ref::map(self.folded_names.borrow(), |slot| {
            &slot.as_ref().expect("кэш только что построен").names
        })
    }

    pub fn add_column(&mut self, name: &str) {
        self.add_typed_column(name, None);
    }

    pub fn add_typed_column(&mut self, name: &str, value_types: Option<Vec<crate::TypeId>>) {
        self.add_constrained_column(
            name,
            value_types.map(|ids| ids.into_iter().map(ColumnType::plain).collect()),
        );
    }

    pub fn add_constrained_column(&mut self, name: &str, value_types: Option<Vec<ColumnType>>) {
        if self.column_index(name).is_some() {
            return; // колонка с таким именем уже есть — не дублируем.
        }
        self.column_names.push(name.to_string());
        self.column_types.push(value_types);
        self.column_vstr.push(ColumnVstr::default());
        self.columns
            .push(vec![BslValue::Undefined; self.row_count()]);
        self.schema_revision = self.schema_revision.wrapping_add(1);
    }

    /// Текущая версия порядка колонок для инвалидизации внешних планов.
    pub(crate) fn schema_revision(&self) -> u64 {
        self.schema_revision
    }

    /// Назначает строкам СТАБИЛЬНЫЕ идентификаторы, пришедшие извне, —
    /// нужен чтению внутреннего формата (`ЗначениеИзСтрокиВнутр`): у
    /// платформы вторая лексема узла строки — именно её внутренний номер,
    /// и после `Свернуть`/`Скопировать` номера разрежены, а не 0..n-1.
    /// Терять их при транзите нельзя. Длина обязана совпасть с текущим
    /// числом строк; дубликаты — ошибка вызывающего (здесь не проверяются:
    /// формат платформы их не порождает).
    pub fn set_row_ids(&mut self, ids: Vec<u64>) {
        debug_assert_eq!(ids.len(), self.row_ids.len());
        self.next_id = ids.iter().max().map_or(0, |m| m + 1);
        self.row_positions = RowPositions::rebuild(&ids, self.next_id);
        self.row_ids = ids;
    }

    /// Добавляет строку (все колонки — `Неопределено`) и возвращает её
    /// стабильный `row_id`.
    pub fn add_row(&mut self) -> u64 {
        for col in &mut self.columns {
            col.push(BslValue::Undefined);
        }
        let id = self.next_id;
        self.next_id += 1;
        let pos = self.row_ids.len();
        self.row_ids.push(id);
        if usize::try_from(id).ok() != Some(pos) {
            self.row_positions.insert(id, pos);
        }
        id
    }

    /// Удаляет строку по ТЕКУЩЕЙ физической позиции. Позиции строк после
    /// неё сдвигаются — обратный индекс чинится под них тут же, поэтому
    /// снаружи это не видно: только что действовавшие id продолжают
    /// указывать на верные (сдвинутые) позиции.
    pub fn delete_row_at(&mut self, pos: usize) -> Option<()> {
        if pos >= self.row_count() {
            return None;
        }
        for col in &mut self.columns {
            col.remove(pos);
        }
        let removed_id = self.row_ids.remove(pos);
        self.row_positions.remove(removed_id);
        for (i, &id) in self.row_ids.iter().enumerate().skip(pos) {
            self.row_positions.insert(id, i);
        }
        Some(())
    }

    pub fn clear(&mut self) {
        for col in &mut self.columns {
            col.clear();
        }
        self.row_ids.clear();
        self.row_positions = RowPositions::identity();
        // next_id НЕ сбрасывается: старые id не должны воскресать и
        // случайно совпасть с новыми после Очистить().
    }

    pub fn row_id_at(&self, pos: usize) -> Option<u64> {
        self.row_ids.get(pos).copied()
    }

    pub fn get_cell(&self, row_id: u64, col: usize) -> Option<BslValue> {
        let pos = self.pos_of(row_id)?;
        self.columns.get(col)?.get(pos).cloned()
    }

    pub fn set_cell(&mut self, row_id: u64, col: usize, value: BslValue) -> Option<()> {
        let pos = self.pos_of(row_id)?;
        let value = self.adjust_to_column_type(col, value);
        *self.columns.get_mut(col)?.get_mut(pos)? = value;
        Some(())
    }

    /// Приведение значения к `ОписаниеТипов` колонки — так делает
    /// платформа при ЛЮБОЙ записи в ячейку (прямое присваивание,
    /// `ЗаполнитьЗначенияСвойств`, `ЗаполнитьЗначения` — измерено, пробы
    /// `ADJ.*`). Правила, снятые батареей из 23 проб:
    ///
    /// * колонка без ограничения — значение как есть;
    /// * тип значения входит в список — как есть, но с применением
    ///   квалификаторов: число округляется до разрядности дробной части и
    ///   НАСЫЩАЕТСЯ границей целой (`123456.7` в `Число(5,2)` — `999.99`),
    ///   строка обрезается по длине, дата обнуляет «чужие» части
    ///   (`{"D","D"}` — только дата, `{"D","T"}` — только время);
    /// * иначе — конверсия по порядку проб Строка, Число, Булево
    ///   (`5` в `Булево,Строка` — `"5"`; `1` в `Булево,Null` — `Истина`);
    /// * конверсия не удалась: единственный тип — его значение по
    ///   умолчанию (`"х"` в `Булево` — `Ложь`), составной — `Неопределено`.
    // НЕ ИЗМЕРЕНО(TABLE.ADJUST) — крайние ветки: `NULL` в колонке без
    // `Null` в списке, дробное число в колонку `Строка` (локаль
    // представления), конверсия строк в дату, значение новой строки
    // `Добавить()` в типизированной колонке. Выбраны консервативные
    // варианты — см. ветки ниже.
    pub fn adjust_to_column_type(&self, col: usize, value: BslValue) -> BslValue {
        let Some(Some(types)) = self.column_types.get(col) else {
            return value;
        };
        if types.is_empty() {
            return value;
        }
        let value_type = match value.type_of() {
            Ok(BslValue::Type(id)) => id,
            _ => return value,
        };
        if let Some(t) = types.iter().find(|t| t.id == value_type) {
            return apply_qualifiers(value, t);
        }
        // `Неопределено` допустимо в любой колонке С СОСТАВНЫМ списком
        // (измерено `ADJ.BL.UNDEF`); в колонке с единственным типом оно
        // приводится к значению по умолчанию (`ADJ.B.UNDEF`, `ADJ.N.UNDEF`).
        if matches!(value, BslValue::Undefined) && types.len() > 1 {
            return BslValue::Undefined;
        }
        // Порядок проб конверсии — Строка, Число, Булево: `5` в
        // `Булево,Строка` даёт `"5"`, а `1` в `Булево,Null` — `Истина`
        // (измерено `ADJ.BS.NUM` и `ADJ.BL.NUM`).
        for probe in [
            crate::TypeId::String,
            crate::TypeId::Number,
            crate::TypeId::Boolean,
        ] {
            let Some(t) = types.iter().find(|t| t.id == probe) else {
                continue;
            };
            if let Some(converted) = convert_to(&value, probe) {
                return apply_qualifiers(converted, t);
            }
        }
        if types.len() == 1 {
            return apply_qualifiers(default_of(types[0].id), &types[0]);
        }
        BslValue::Undefined
    }

    /// `ЗаполнитьЗначения(Значение, Колонки)` — записывает одно значение
    /// во все строки перечисленных колонок. Пустой список колонок означает
    /// все колонки таблицы.
    pub fn fill_values(&mut self, value: &BslValue, cols: &[usize]) {
        // Значение проходит то же приведение к типу колонки, что и прямое
        // присваивание, — измерено (`ADJ.FILLALL.STR`).
        if cols.is_empty() {
            for col in 0..self.columns.len() {
                let adjusted = self.adjust_to_column_type(col, value.clone());
                self.columns[col].fill(adjusted);
            }
            return;
        }
        for &col in cols {
            if col < self.columns.len() {
                let adjusted = self.adjust_to_column_type(col, value.clone());
                self.columns[col].fill(adjusted);
            }
        }
    }

    /// `Найти(Значение[, Колонки])` — первый `row_id`, у которого в одной
    /// из `cols` (или в любой, если список пуст) лежит равное значение.
    /// Равенство здесь СТРОГОЕ (`PartialEq`), а не то ослабленное, что у
    /// оператора `=`: булево единицей не находится. Измерено на массиве
    /// (`EQ.ARRAY.FIND_BOOL_BY_NUMBER`), где платформа тоже не находит.
    /// У ссылочных типов это, как и раньше, тождество объекта.
    pub fn find(&self, value: &BslValue, cols: &[usize]) -> Option<u64> {
        let all: Vec<usize> = (0..self.columns.len()).collect();
        let cols = if cols.is_empty() { &all[..] } else { cols };
        for pos in 0..self.row_count() {
            for &c in cols {
                if self.columns.get(c).and_then(|col| col.get(pos)) == Some(value) {
                    return self.row_ids.get(pos).copied();
                }
            }
        }
        None
    }

    /// `НайтиСтроки(СтруктураПоиска)` — все `row_id`, у которых КАЖДАЯ
    /// пара `(колонка, значение)` совпала. Порядок результата — порядок
    /// строк в таблице.
    pub fn find_rows(&self, criteria: &[(usize, BslValue)]) -> Vec<u64> {
        (0..self.row_count())
            .filter(|&pos| {
                criteria.iter().all(|(c, want)| {
                    self.columns.get(*c).and_then(|col| col.get(pos)) == Some(want)
                })
            })
            .filter_map(|pos| self.row_ids.get(pos).copied())
            .collect()
    }

    /// `Итог("Колонка")` — сумма числовых значений колонки.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC)`: что делает платформа с
    /// нечисловыми значениями в колонке — игнорирует, падает или считает их
    /// нулём. Взято ИГНОРИРОВАНИЕ (нечисловые просто не входят в сумму):
    /// это единственный вариант, при котором `Итог` по колонке со смешанным
    /// содержимым остаётся вызываемым, а колонка из одних нечисловых даёт
    /// `0`, а не ошибку.
    pub fn total(&self, col: usize) -> Result<BslNumber, NumError> {
        let mut sum = BslNumber::from_i64(0);
        let Some(values) = self.columns.get(col) else {
            return Ok(sum);
        };
        for v in values {
            if let BslValue::Number(n) = v {
                sum.add_assign(n)?;
            }
        }
        Ok(sum)
    }

    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`.
    ///
    /// Сортировка УСТОЙЧИВАЯ (`sort_by` в Rust таков) — при равных ключах
    /// исходный порядок строк сохраняется. И, что важнее, переставляются
    /// не только колонки, но и `row_ids` вместе с ними, после чего
    /// обратный индекс пересобирается целиком: живой объект
    /// `СтрокаТаблицыЗначений`, взятый ДО сортировки, после неё продолжает
    /// указывать на ту же строку, просто стоящую в другом месте.
    pub fn sort(&mut self, keys: &[SortKey]) {
        let mut order: Vec<usize> = (0..self.row_count()).collect();
        // Декорация ПЕРВОГО ключа: большинство сравнений решается им, и
        // упакованный префикс сравнивается парой машинных инструкций
        // вместо прохода компаратора по значениям. Совпадение префиксов
        // ничего не утверждает — тогда значения сравниваются полностью.
        // Выгодна она только строковым колонкам: у прочих типов префикс
        // нулевой, и колонка без единой строки получала бы чистый
        // накладной расход на каждое сравнение — замерено на table_sort.
        // Пакуются символы ПОСЛЕ общего коллационного префикса колонки:
        // на данных вида «имя_12345» первые символы у всех совпадают, и
        // без сдвига декорация не различала бы ничего.
        let deco: Option<(Vec<SortPrefix>, bool)> = keys.first().and_then(|key| {
            let col = self.columns.get(key.column)?;
            let skip = column_collation_lcp(col)?;
            Some((
                col.iter().map(|v| sort_prefix(v, skip)).collect(),
                key.descending,
            ))
        });
        order.sort_by(|&a, &b| {
            if let Some((deco, descending)) = &deco {
                let ord = deco[a].compare(&deco[b]);
                if ord != Ordering::Equal {
                    return if *descending { ord.reverse() } else { ord };
                }
            }
            for key in keys {
                let Some(col) = self.columns.get(key.column) else {
                    continue;
                };
                let ord = compare_for_sort(&col[a], &col[b]);
                let ord = if key.descending { ord.reverse() } else { ord };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            Ordering::Equal
        });

        // `order[new] = old` — готовая карта сборки: каждая колонка
        // строится заново ПЕРЕНОСОМ значений в новом порядке, без клонов
        // и счётчиков ссылок — на старом месте остаётся `Неопределено`,
        // и старый вектор освобождается пустышками. Последовательная
        // запись новой колонки заметно дружелюбнее к кэшу, чем прежние
        // цепочки обменов со случайным доступом с обеих сторон, — на
        // сортировке 1.8 млн строк × 19 колонок это видно в замере.
        for col in &mut self.columns {
            let mut gathered = Vec::with_capacity(col.len());
            gathered.extend(
                order
                    .iter()
                    .map(|&old| std::mem::replace(&mut col[old], BslValue::Undefined)),
            );
            *col = gathered;
        }
        let row_ids = order.iter().map(|&old| self.row_ids[old]).collect();
        self.row_ids = row_ids;
        self.reindex();
    }

    /// Пересборка индекса `id -> позиция` после любой перестановки строк.
    /// Вынесено отдельно, потому что этим кончаются `sort`, `move_row` и
    /// `collapse` — и любая забытая пересборка тихо ломает инвариант живых
    /// строк, а не падает.
    fn reindex(&mut self) {
        self.row_positions = RowPositions::rebuild(&self.row_ids, self.next_id);
    }

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    /// Текущая позиция строки; `None` — строка удалена (`Удалить`,
    /// `Очистить`, `Свернуть`) либо принадлежит другой таблице.
    pub fn pos_of(&self, row_id: u64) -> Option<usize> {
        if let Ok(pos) = usize::try_from(row_id) {
            if self.row_ids.get(pos) == Some(&row_id) {
                return Some(pos);
            }
        }
        self.row_positions.get(row_id)
    }

    /// `Скопировать([Строки], [Колонки])` — НОВАЯ таблица с выбранными
    /// строками и колонками.
    ///
    /// `rows` — позиции в текущем порядке (пусто у вызывающего не бывает:
    /// «все строки» он разворачивает сам, см. `BslValue::table_copy`);
    /// `cols` — индексы колонок в нужном порядке.
    ///
    /// Строки копии получают СВОИ `row_id`, начиная с нуля: копия — другая
    /// таблица, и живой объект строки оригинала обязан продолжать указывать
    /// в оригинал, а не начать резолвиться ещё и в копии.
    pub fn copy_of(&self, rows: &[usize], cols: &[usize]) -> ValueTableData {
        let mut out = ValueTableData {
            folded_names: RefCell::new(None),
            column_names: cols
                .iter()
                .filter_map(|&c| self.column_names.get(c).cloned())
                .collect(),
            column_types: cols
                .iter()
                .filter_map(|&c| self.column_types.get(c).cloned())
                .collect(),
            column_vstr: cols
                .iter()
                .filter_map(|&c| self.column_vstr.get(c).cloned())
                // Копия — новая таблица: идентификаторы колонок платформа
                // выдаёт заново, унаследованные из чтения сбрасываются.
                .map(|extra| ColumnVstr { id: None, ..extra })
                .collect(),
            vstr_tail_x: None,
            columns: Vec::with_capacity(cols.len()),
            row_ids: Vec::with_capacity(rows.len()),
            row_positions: RowPositions::identity(),
            next_id: 0,
            schema_revision: 0,
        };
        for &c in cols {
            let src = &self.columns[c];
            out.columns
                .push(rows.iter().map(|&pos| src[pos].clone()).collect());
        }
        for pos in 0..rows.len() {
            out.row_ids.push(pos as u64);
        }
        out.next_id = rows.len() as u64;
        out
    }

    /// `ВыгрузитьКолонку(Колонка)` — значения колонки в ТЕКУЩЕМ порядке
    /// строк.
    pub fn unload_column(&self, col: usize) -> Vec<BslValue> {
        self.columns.get(col).cloned().unwrap_or_default()
    }

    /// `ЗагрузитьКолонку(Массив, Колонка)`.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH)`: что делает платформа,
    /// когда длина массива не совпадает с числом строк — падает, обрезает,
    /// добивает строками или оставляет хвост как есть. Взято САМОЕ
    /// БЕЗОБИДНОЕ: лишние значения массива игнорируются, недостающие
    /// оставляют ячейку прежней. Число строк таблицы этот метод не меняет
    /// ни при каком раскладе — иначе он молча инвалидировал бы живые строки.
    pub fn load_column(&mut self, col: usize, values: &[BslValue]) {
        let Some(column) = self.columns.get_mut(col) else {
            return;
        };
        for (cell, v) in column.iter_mut().zip(values) {
            *cell = v.clone();
        }
    }

    /// `Сдвинуть(Строка, Смещение)` — перестановка строки на `offset`
    /// позиций. Живые объекты строк переживают её: переезжают и значения
    /// колонок, и `row_ids`, а индекс пересобирается (инвариант 12).
    ///
    /// `None` — целевая позиция вне таблицы; что делать с этим, решает
    /// вызывающий.
    pub fn move_row(&mut self, from: usize, offset: i64) -> Option<usize> {
        let len = self.row_count();
        if from >= len {
            return None;
        }
        let to = i64::try_from(from).ok()?.checked_add(offset)?;
        if to < 0 || to as usize >= len {
            return None;
        }
        let to = to as usize;
        for col in &mut self.columns {
            let v = col.remove(from);
            col.insert(to, v);
        }
        let id = self.row_ids.remove(from);
        self.row_ids.insert(to, id);
        self.reindex();
        Some(to)
    }

    /// `Свернуть(КолонкиГруппировки, КолонкиСуммирования)` — группировка НА
    /// МЕСТЕ.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.OTHER_COLUMNS)`: что происходит с
    /// колонками, не попавшими ни в группировку, ни в суммирование. Взято
    /// УДАЛЕНИЕ — колонка, у которой в свёрнутой строке нет ни одного
    /// осмысленного значения (в группе их было несколько разных), не
    /// сохранима без произвольного выбора «какое из». Порядок колонок
    /// результата — сначала группировочные, потом суммируемые, в порядке
    /// перечисления; это часть того же вопроса.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.ROW_ORDER)`: порядок строк результата.
    /// Взят порядок ПЕРВОГО ВХОЖДЕНИЯ группы — он сохраняет исходную
    /// сортировку таблицы, тогда как сортировка по ключам её бы затёрла.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.COLLAPSE.NON_NUMERIC)`: суммирование колонки с
    /// нечисловыми значениями. Взято ИГНОРИРОВАНИЕ — то же решение, что и в
    /// `total`, и разъезжаться этим двум местам незачем.
    ///
    /// Идентичность строк: за каждой группой остаётся `row_id` её ПЕРВОЙ
    /// строки, остальные исчезают вместе со строками. Это ровно то же, что
    /// делает `Удалить`, и инвариант живых строк не нарушает: обращение к
    /// пропавшей строке даёт `RowInvalidated`, а не чужие данные.
    // Равенство значений ключа — то же самое, что у `Соответствие`: для
    // ссылочных типов сравнивается адрес `Rc`, для типов-значений — само
    // значение. Отпечаток служит только индексом кандидатов; коллизии
    // разрешаются полным сравнением и не влияют на результат.
    pub fn collapse(&mut self, group: &[usize], sum: &[usize]) -> Result<(), NumError> {
        let random_state = RandomState::new();
        let seed = random_state.build_hasher().finish();
        self.collapse_with_fingerprint(group, sum, |table, pos, group| {
            let mut hasher = GroupHasher::new(seed);
            group.len().hash(&mut hasher);
            for &column in group {
                table.columns[column][pos].hash(&mut hasher);
            }
            hasher.finish()
        })
    }

    fn collapse_with_fingerprint(
        &mut self,
        group: &[usize],
        sum: &[usize],
        mut fingerprint: impl FnMut(&Self, usize, &[usize]) -> u64,
    ) -> Result<(), NumError> {
        let original_row_count = self.row_count();
        // Значение карты — голова цепочки групп с одинаковым отпечатком.
        // Обычно цепочка состоит из одного элемента; `collision_next`
        // делает редкие настоящие коллизии корректными без `Vec` в каждом
        // бакете карты.
        let mut head_by_fingerprint = FingerprintMap::default();
        let mut collision_next: Vec<Option<usize>> = Vec::new();
        // Ключ группы представлен позицией её первой строки. Значения
        // остаются в исходных колонках до конца группировки и не
        // клонируются в отдельное построчное хранилище.
        let mut representatives: Vec<usize> = Vec::new();
        // Суммы лежат одним буфером: отдельный `Vec` на каждую группу на
        // больших уникальных таблицах превращался в миллионы аллокаций.
        let mut sums: Vec<BslNumber> = Vec::new();
        let mut ids: Vec<u64> = Vec::new();

        for pos in 0..self.row_count() {
            const RESERVE_SAMPLE: usize = 4096;
            if pos == RESERVE_SAMPLE && ids.len() * 20 >= RESERVE_SAMPLE * 19 {
                // Почти уникальная выборка означает, что буферы, скорее
                // всего, вырастут до размера исходной таблицы. Один точный
                // резерв дешевле повторных копирований и не оставляет
                // ёмкость на следующей степени двойки. Для обычной
                // агрегации с множеством повторов эта ветка не срабатывает.
                let remaining_groups = self.row_count().saturating_sub(ids.len());
                head_by_fingerprint.reserve(remaining_groups);
                collision_next.reserve(remaining_groups);
                representatives.reserve(remaining_groups);
                ids.reserve(remaining_groups);
                sums.reserve(remaining_groups.saturating_mul(sum.len()));
            }

            let row_fingerprint = fingerprint(self, pos, group);
            let mut candidate = head_by_fingerprint.get(&row_fingerprint).copied();
            let mut matching_slot = None;
            while let Some(slot) = candidate {
                let representative = representatives[slot];
                let equal = group
                    .iter()
                    .all(|&input| self.columns[input][representative] == self.columns[input][pos]);
                if equal {
                    matching_slot = Some(slot);
                    break;
                }
                candidate = collision_next[slot];
            }

            let slot = match matching_slot {
                Some(slot) => slot,
                None => {
                    let slot = ids.len();
                    let previous = head_by_fingerprint.insert(row_fingerprint, slot);
                    collision_next.push(previous);
                    representatives.push(pos);
                    sums.extend((0..sum.len()).map(|_| BslNumber::from_i64(0)));
                    ids.push(self.row_ids[pos]);
                    slot
                }
            };
            for (k, &c) in sum.iter().enumerate() {
                if let BslValue::Number(n) = &self.columns[c][pos] {
                    let offset = slot * sum.len() + k;
                    sums[offset].add_assign(n)?;
                }
            }
        }

        let group_count = ids.len();
        let mut columns: Vec<Vec<BslValue>> = Vec::with_capacity(group.len() + sum.len());
        if group_count == original_row_count {
            // Ни одна строка не слилась с другой: группировочные колонки
            // уже имеют точный итоговый порядок. Переносим их буферы без
            // клонирования десятков миллионов `BslValue`.
            let mut original_columns = std::mem::take(&mut self.columns);
            for (output, &input) in group.iter().enumerate() {
                if let Some(previous) = group[..output].iter().position(|&col| col == input) {
                    columns.push(columns[previous].clone());
                } else {
                    columns.push(std::mem::take(&mut original_columns[input]));
                }
            }
        } else {
            for &input in group {
                columns.push(
                    representatives
                        .iter()
                        .map(|&position| self.columns[input][position].clone())
                        .collect(),
                );
            }
        }
        for s in 0..sum.len() {
            let mut column = Vec::with_capacity(group_count);
            for slot in 0..group_count {
                column.push(BslValue::Number(sums[slot * sum.len() + s].clone()));
            }
            columns.push(column);
        }
        let names: Vec<String> = group
            .iter()
            .chain(sum.iter())
            .filter_map(|&c| self.column_names.get(c).cloned())
            .collect();
        let value_types: Vec<Option<Vec<ColumnType>>> = group
            .iter()
            .chain(sum.iter())
            .filter_map(|&c| self.column_types.get(c).cloned())
            .collect();
        let vstr: Vec<ColumnVstr> = group
            .iter()
            .chain(sum.iter())
            .filter_map(|&c| self.column_vstr.get(c).cloned())
            .map(|extra| ColumnVstr { id: None, ..extra })
            .collect();
        self.column_names = names;
        self.column_types = value_types;
        self.column_vstr = vstr;
        self.vstr_tail_x = None;
        self.columns = columns;
        self.row_ids = ids;
        self.schema_revision = self.schema_revision.wrapping_add(1);
        self.reindex();
        Ok(())
    }
}

/// Одна колонка в задании сортировки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortKey {
    pub column: usize,
    /// `Убыв`/`Desc`; по умолчанию (`Возр`/`Asc` или ничего) — по возрастанию.
    pub descending: bool,
}

/// Сравнение СТРОК для сортировки — вынесено в отдельную функцию нарочно,
/// чтобы замену на настоящую коллацию можно было сделать в одном месте.
///
/// ИЗМЕРЕНО на платформе 8.3.27. Список
/// `яблоко,Яблоко,ёлка,Ель,zebra,Апельсин,10,2` платформа упорядочила так:
///
/// ```text
/// 10, 2, zebra, Апельсин, ёлка, Ель, яблоко, Яблоко
/// ```
///
/// Отсюда три правила, и все три пришлось чинить:
///
/// 1. `ё` сравнивается КАК `е`: иначе «ёлка» (U+0451) уехала бы в конец
///    алфавита, а платформа ставит её перед «Ель» — то есть сравнивала
///    «елка» с «ель» и решила по третьей букве.
/// 2. Регистр — вторичный ключ, и СТРОЧНАЯ идёт ПЕРЕД прописной
///    («яблоко» перед «Яблоко»). У нас было наоборот: сравнение исходных
///    строк ставит прописную первой, потому что у неё меньше код.
/// 3. Цифры перед латиницей, латиница перед кириллицей — это совпало с
///    порядком кодовых точек и правкой не потребовало.
///
/// Чего замер НЕ покрыл: взаимный порядок «ёлка»/«елка» при совпадении
/// остального (у нас — строчная-ё после строчной-е по тому же вторичному
/// ключу) и диакритика прочих алфавитов. Полная UCA по-прежнему заменяет
/// ровно эту функцию.
pub fn collate(a: &BslString, b: &BslString) -> Ordering {
    match collation_key(a).cmp(collation_key(b)) {
        Ordering::Equal => {}
        other => return other,
    }
    // Ключи равны — строки отличаются только регистром (или е/ё).
    // Строчная вперёд: у неё код БОЛЬШЕ, поэтому сравнение перевёрнуто.
    b.cmp(a)
}

/// Первичный ключ сравнения: нижний регистр, `ё` сведена к `е`.
fn collation_key(s: &BslString) -> impl Iterator<Item = char> + '_ {
    s.lowercase_chars()
        .iter()
        .copied()
        .map(|c| if c == 'ё' { 'е' } else { c })
}

/// Порядок РАЗНОТИПНЫХ значений в сортировке.
///
/// ИЗМЕРЕНО на 8.3.27: колонка со значениями «текст», 5, датой, `Истина` и
/// `Неопределено` отсортировалась как
/// `Неопределено, Булево, Число, Строка, Дата`. Строка ПЕРЕД датой —
/// именно то, что угадать было нельзя: до замера здесь стоял порядок
/// «числа, даты, булево, строки», и он расходился на трёх парах из шести.
fn type_rank(v: &BslValue) -> u8 {
    match v {
        BslValue::Undefined => 0,
        // Null рядом с Неопределено: сам по себе НЕ ИЗМЕРЕН (в колонке
        // замера его не было), но соседство с Неопределено — наименее
        // произвольное из возможных.
        BslValue::Null => 1,
        BslValue::Boolean(_) => 2,
        BslValue::Number(_) => 3,
        BslValue::Str(_) => 4,
        BslValue::Date(_) => 5,
        BslValue::Type(_) => 6,
        // Член перечисления рядом с типом: оба — служебные значения, ни
        // одно из двух в колонке замера не участвовало. Соседство с `Тип`
        // наименее произвольное из возможных, как и `Null` рядом с
        // `Неопределено` выше.
        BslValue::Enum(_) => 7,
        // Голое имя перечисления — тем же рассуждением, рядом с членом
        // перечисления: тоже не участвовало в замере.
        BslValue::EnumType(_) => 8,
        BslValue::Object(_) => 9,
        BslValue::Skipped => 10,
    }
}

/// Упакованный первичный префикс значения для быстрого пути сортировки:
/// ранг типа и первые четыре символа строки в первичной коллации (нижний
/// регистр, `ё` сведена к `е`) по u16 на символ, старший — первый.
/// НЕРАВЕНСТВО префиксов совпадает с вердиктом [`compare_for_sort`];
/// равенство ничего не значит и уводит в полное сравнение. Строка с
/// символом вне базовой плоскости в префиксе (`clean == false`) в u16 не
/// помещается — такому значению быстрый путь не доверяет вовсе.
/// Нестроковые значения несут нулевой ключ: между собой они решаются
/// полным сравнением, как раньше, а от строк их отделяет ранг типа.
struct SortPrefix {
    rank: u8,
    clean: bool,
    key: u64,
}

impl SortPrefix {
    fn compare(&self, other: &Self) -> Ordering {
        match self.rank.cmp(&other.rank) {
            Ordering::Equal if self.clean && other.clean => self.key.cmp(&other.key),
            Ordering::Equal => Ordering::Equal,
            other => other,
        }
    }
}

/// Свёрнутый в первичную коллацию символ: нижний регистр уже дала
/// [`BslString::lowercase_chars`], здесь остаётся `ё` → `е`.
fn fold_collation(c: char) -> char {
    if c == 'ё' {
        'е'
    } else {
        c
    }
}

/// Общий первичный префикс всех СТРОК колонки в символах; `None` — в
/// колонке нет ни одной строки и декорация не окупится. Значения других
/// типов на префикс не влияют: между собой их разводит ранг типа.
fn column_collation_lcp(col: &[BslValue]) -> Option<usize> {
    let mut lcp: Option<(&BslString, usize)> = None;
    for v in col {
        let BslValue::Str(s) = v else { continue };
        match &mut lcp {
            None => {
                let len = s.lowercase_chars().len();
                lcp = Some((s, len));
            }
            Some((first, len)) => {
                let a = first.lowercase_chars();
                let b = s.lowercase_chars();
                let cap = (*len).min(b.len());
                let mut common = 0;
                while common < cap && fold_collation(a[common]) == fold_collation(b[common]) {
                    common += 1;
                }
                *len = common;
                if common == 0 {
                    break;
                }
            }
        }
    }
    lcp.map(|(_, len)| len)
}

fn sort_prefix(v: &BslValue, skip: usize) -> SortPrefix {
    let rank = type_rank(v);
    let BslValue::Str(s) = v else {
        return SortPrefix {
            rank,
            clean: true,
            key: 0,
        };
    };
    let chars = s.lowercase_chars();
    let mut key = 0u64;
    let mut clean = true;
    for i in skip..skip + 4 {
        let unit = match chars.get(i) {
            Some(&c) => {
                let cp = fold_collation(c) as u32;
                if cp > 0xFFFF {
                    clean = false;
                    0
                } else {
                    cp as u16
                }
            }
            None => 0,
        };
        key = (key << 16) | u64::from(unit);
    }
    SortPrefix { rank, clean, key }
}

/// Сравнение двух значений колонки. Одинаковые типы сравниваются по
/// существу (строки — через `collate`), разные — по рангу типа.
fn compare_for_sort(a: &BslValue, b: &BslValue) -> Ordering {
    match (a, b) {
        (BslValue::Number(x), BslValue::Number(y)) => x.cmp(y),
        (BslValue::Str(x), BslValue::Str(y)) => collate(x, y),
        (BslValue::Date(x), BslValue::Date(y)) => x.cmp(y),
        (BslValue::Boolean(x), BslValue::Boolean(y)) => x.cmp(y),
        // Объекты между собой не упорядочиваются осмысленно (у них нет
        // ни значения, ни стабильного ключа) — считаем равными, и
        // устойчивость сортировки сохранит их исходный порядок.
        _ => type_rank(a).cmp(&type_rank(b)),
    }
}

/// Разбор задания сортировки `"Кол1 Возр, Кол2 Убыв"`.
///
/// Неизвестное имя колонки — ошибка вызывающего, поэтому здесь
/// возвращается имя, а не тихо пропускается: `Сортировать("Опечатка")`,
/// молча ничего не отсортировавшая, — худший из возможных исходов.
pub fn parse_sort_spec(
    spec: &str,
    resolve: impl Fn(&str) -> Option<usize>,
) -> Result<Vec<SortKey>, String> {
    let mut keys = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut words = part.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        let descending = match words.next() {
            None => false,
            Some(dir) if dir.eq_ignore_ascii_case("Возр") || dir.eq_ignore_ascii_case("Asc") => {
                false
            }
            Some(dir) if dir.eq_ignore_ascii_case("Убыв") || dir.eq_ignore_ascii_case("Desc") => {
                true
            }
            Some(dir) => return Err(dir.to_string()),
        };
        let column = resolve(name).ok_or_else(|| name.to_string())?;
        keys.push(SortKey { column, descending });
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_row_ids_need_no_reverse_map_and_fallback_after_delete() {
        let table = ValueTableData::new();
        let mut table = table.borrow_mut();
        table.add_column("к");
        let first = table.add_row();
        let second = table.add_row();
        let third = table.add_row();

        assert!(matches!(
            &table.row_positions,
            RowPositions::Sparse(positions) if positions.is_empty()
        ));
        assert_eq!(table.pos_of(first), Some(0));
        assert_eq!(table.pos_of(second), Some(1));
        assert_eq!(table.pos_of(third), Some(2));

        table.delete_row_at(0).unwrap();
        assert_eq!(table.pos_of(first), None);
        assert_eq!(table.pos_of(second), Some(0));
        assert_eq!(table.pos_of(third), Some(1));
    }

    #[test]
    fn reordered_dense_row_ids_use_a_dense_reverse_index() {
        let table = ValueTableData::new();
        let mut table = table.borrow_mut();
        table.add_column("к");
        let first = table.add_row();
        let second = table.add_row();
        let third = table.add_row();

        table.move_row(0, 2).unwrap();

        assert!(matches!(
            &table.row_positions,
            RowPositions::Dense(positions) if positions.len() == 3
        ));
        assert_eq!(table.pos_of(first), Some(2));
        assert_eq!(table.pos_of(second), Some(0));
        assert_eq!(table.pos_of(third), Some(1));
    }

    #[test]
    fn collapse_reuses_unique_group_columns_and_keeps_duplicate_specs() {
        let table = ValueTableData::new();
        let mut table = table.borrow_mut();
        table.add_column("к");
        for value in ["а", "б"] {
            let row = table.add_row();
            table.set_cell(row, 0, BslValue::Str(BslString::from_str(value)));
        }
        let original_buffer = table.columns[0].as_ptr();

        table
            .collapse_with_fingerprint(&[0, 0], &[], |_, position, _| position as u64)
            .unwrap();

        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].as_ptr(), original_buffer);
        assert_eq!(table.columns[0], table.columns[1]);
    }

    #[test]
    fn collapse_resolves_fingerprint_collisions_by_the_full_key() {
        let table = ValueTableData::new();
        let mut table = table.borrow_mut();
        table.add_column("г");
        table.add_column("с");

        for (key, value) in [("а", 1), ("б", 2), ("а", 3)] {
            let row = table.add_row();
            table.set_cell(row, 0, BslValue::Str(BslString::from_str(key)));
            table.set_cell(row, 1, BslValue::Number(BslNumber::from_i64(value)));
        }

        // Все строки намеренно получают один отпечаток. Разные ключи не
        // должны слиться, а повторный ключ обязан попасть в свою группу.
        table
            .collapse_with_fingerprint(&[0], &[1], |_, _, _| 0)
            .unwrap();

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.columns[0][0], BslValue::Str(BslString::from_str("а")));
        assert_eq!(table.columns[0][1], BslValue::Str(BslString::from_str("б")));
        assert_eq!(
            table.columns[1][0],
            BslValue::Number(BslNumber::from_i64(4))
        );
        assert_eq!(
            table.columns[1][1],
            BslValue::Number(BslNumber::from_i64(2))
        );
    }

    #[test]
    fn sort_moves_values_without_cloning() {
        // Колонки после сортировки собираются в новые буферы, но значения
        // ПЕРЕНОСЯТСЯ: объект в ячейке обязан остаться тем же самым `Rc`,
        // а не копией — на этом держится и цена сортировки, и семантика
        // ссылочных значений.
        let table = ValueTableData::new();
        let mut table = table.borrow_mut();
        table.add_column("к");
        table.add_column("м");
        let marker = BslValue::new_array(Vec::new());
        let BslValue::Object(marker_rc) = &marker else {
            panic!("массив — объект");
        };
        let marker_ptr = Rc::as_ptr(marker_rc);
        for value in [3, 1, 2] {
            let row = table.add_row();
            table.set_cell(row, 0, BslValue::Number(BslNumber::from_i64(value)));
            if value == 3 {
                table.set_cell(row, 1, marker.clone());
            }
        }

        table.sort(&[SortKey {
            column: 0,
            descending: false,
        }]);

        assert_eq!(
            table.columns[0],
            [1, 2, 3].map(|value| BslValue::Number(BslNumber::from_i64(value)))
        );
        // Маркер уехал вместе со своей строкой в конец и остался тем же
        // объектом.
        let BslValue::Object(moved) = &table.columns[1][2] else {
            panic!("маркер не переехал вместе со строкой");
        };
        assert_eq!(Rc::as_ptr(moved), marker_ptr);
    }

    // НЕ ИЗМЕРЕНО(TABLE.ADJUST) — тесты фиксируют ИЗМЕРЕННУЮ часть правил
    // (батарея ADJ.*) и выбранное поведение неснятых веток.
    fn adjusted(types: &[(crate::TypeId, &[&str])], value: BslValue) -> BslValue {
        let table = ValueTableData::new();
        let mut t = table.borrow_mut();
        t.add_constrained_column(
            "К",
            Some(
                types
                    .iter()
                    .map(|(id, quals)| ColumnType {
                        id: *id,
                        quals: quals.iter().map(|q| q.to_string()).collect(),
                    })
                    .collect(),
            ),
        );
        let id = t.add_row();
        t.set_cell(id, 0, value);
        t.columns[0][0].clone()
    }

    fn n(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    fn txt(s: &str) -> BslValue {
        BslValue::Str(BslString::from_str(s))
    }

    #[test]
    fn adjust_follows_the_measured_platform_rules() {
        use crate::TypeId::*;
        // Составной «Булево,Null»: строка и Неопределено — Неопределено,
        // число конвертируется в Булево, NULL остаётся (ADJ.BL.*).
        let bl: &[(crate::TypeId, &[&str])] = &[(Boolean, &[]), (Null, &[])];
        assert_eq!(adjusted(bl, txt("х")), BslValue::Undefined);
        assert_eq!(adjusted(bl, txt("")), BslValue::Undefined);
        assert_eq!(adjusted(bl, n("1")), BslValue::Boolean(true));
        assert_eq!(adjusted(bl, BslValue::Undefined), BslValue::Undefined);
        assert_eq!(adjusted(bl, BslValue::Null), BslValue::Null);
        // Единственный тип: неудача конверсии — значение по умолчанию
        // (ADJ.B.STR, ADJ.B.UNDEF, ADJ.N.UNDEF), удача — конверсия
        // (ADJ.N.STR, ADJ.N.TRUE, ADJ.S.NUM, ADJ.S.BOOL).
        let b: &[(crate::TypeId, &[&str])] = &[(Boolean, &[])];
        assert_eq!(adjusted(b, txt("х")), BslValue::Boolean(false));
        assert_eq!(adjusted(b, BslValue::Undefined), BslValue::Boolean(false));
        let num: &[(crate::TypeId, &[&str])] = &[(Number, &[])];
        assert_eq!(adjusted(num, txt("5")), n("5"));
        assert_eq!(adjusted(num, BslValue::Boolean(true)), n("1"));
        assert_eq!(adjusted(num, BslValue::Undefined), n("0"));
        let s: &[(crate::TypeId, &[&str])] = &[(String, &[])];
        assert_eq!(adjusted(s, n("5")), txt("5"));
        assert_eq!(adjusted(s, BslValue::Boolean(true)), txt("Да"));
        // Порядок проб конверсии: Строка раньше Булево (ADJ.BS.NUM).
        let bs: &[(crate::TypeId, &[&str])] = &[(Boolean, &[]), (String, &[])];
        assert_eq!(adjusted(bs, n("5")), txt("5"));
        // Квалификаторы: округление и насыщение числа (ADJ.NQ.*),
        // обрезка строки (ADJ.SQ.LONG).
        let nq: &[(crate::TypeId, &[&str])] = &[(Number, &["5", "2", "0"])];
        assert_eq!(adjusted(nq, n("123.456")), n("123.46"));
        assert_eq!(adjusted(nq, n("-7.891")), n("-7.89"));
        assert_eq!(adjusted(nq, n("123456.7")), n("999.99"));
        let sq: &[(crate::TypeId, &[&str])] = &[(String, &["3", "1"])];
        assert_eq!(adjusted(sq, txt("абвгд")), txt("абв"));
        // Части даты: «только дата» обнуляет время, «только время» — дату
        // (ADJ.DD.DT, ADJ.DV.DT).
        let date = BslValue::Date(crate::date::BslDate::from_civil(2024, 5, 6, 7, 8, 9).unwrap());
        let dd: &[(crate::TypeId, &[&str])] = &[(Date, &["D"])];
        assert_eq!(
            adjusted(dd, date.clone()),
            BslValue::Date(crate::date::BslDate::from_civil(2024, 5, 6, 0, 0, 0).unwrap())
        );
        let dv: &[(crate::TypeId, &[&str])] = &[(Date, &["T"])];
        assert_eq!(
            adjusted(dv, date),
            BslValue::Date(crate::date::BslDate::from_civil(1, 1, 1, 7, 8, 9).unwrap())
        );
    }
}
