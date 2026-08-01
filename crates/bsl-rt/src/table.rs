use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
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
/// `row_id`; таблица держит обратную карту `row_id -> текущая позиция`.
/// Удалённая строка выпадает из карты — обращение к ней после этого
/// возвращает `RtError::RowInvalidated`, а не тихо читает чужие данные.
#[derive(Debug)]
pub struct ValueTableData {
    /// Имена колонок сравниваются регистронезависимо снаружи (см.
    /// `column_index`), но хранятся с оригинальным написанием.
    pub column_names: Vec<String>,
    /// `columns[col][pos]` — значение колонки `col` в строке на текущей
    /// физической позиции `pos`. Все колонки всегда одной длины — длины
    /// строк таблицы.
    pub columns: Vec<Vec<BslValue>>,
    /// `row_ids[pos]` — стабильный id строки, сейчас стоящей на позиции
    /// `pos`.
    pub row_ids: Vec<u64>,
    /// Обратная карта: id -> текущая позиция. Отсутствие ключа значит
    /// "строка удалена".
    pub id_to_pos: HashMap<u64, usize>,
    next_id: u64,
}

impl ValueTableData {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(ValueTableData {
            column_names: Vec::new(),
            columns: Vec::new(),
            row_ids: Vec::new(),
            id_to_pos: HashMap::new(),
            next_id: 0,
        }))
    }

    pub fn row_count(&self) -> usize {
        self.row_ids.len()
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.column_names
            .iter()
            .position(|c| c.eq_ignore_ascii_case(name))
    }

    pub fn add_column(&mut self, name: &str) {
        if self.column_index(name).is_some() {
            return; // колонка с таким именем уже есть — не дублируем.
        }
        self.column_names.push(name.to_string());
        self.columns.push(vec![BslValue::Undefined; self.row_count()]);
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
        self.id_to_pos.insert(id, pos);
        id
    }

    /// Удаляет строку по ТЕКУЩЕЙ физической позиции. Позиции строк после
    /// неё сдвигаются — карта `id_to_pos` чинится под них тут же, поэтому
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
        self.id_to_pos.remove(&removed_id);
        for (i, &id) in self.row_ids.iter().enumerate().skip(pos) {
            self.id_to_pos.insert(id, i);
        }
        Some(())
    }

    pub fn clear(&mut self) {
        for col in &mut self.columns {
            col.clear();
        }
        self.row_ids.clear();
        self.id_to_pos.clear();
        // next_id НЕ сбрасывается: старые id не должны воскресать и
        // случайно совпасть с новыми после Очистить().
    }

    pub fn row_id_at(&self, pos: usize) -> Option<u64> {
        self.row_ids.get(pos).copied()
    }

    pub fn get_cell(&self, row_id: u64, col: usize) -> Option<BslValue> {
        let pos = *self.id_to_pos.get(&row_id)?;
        self.columns.get(col)?.get(pos).cloned()
    }

    pub fn set_cell(&mut self, row_id: u64, col: usize, value: BslValue) -> Option<()> {
        let pos = *self.id_to_pos.get(&row_id)?;
        *self.columns.get_mut(col)?.get_mut(pos)? = value;
        Some(())
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
                sum = sum.add(n)?;
            }
        }
        Ok(sum)
    }

    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`.
    ///
    /// Сортировка УСТОЙЧИВАЯ (`sort_by` в Rust таков) — при равных ключах
    /// исходный порядок строк сохраняется. И, что важнее, переставляются
    /// не только колонки, но и `row_ids` вместе с ними, после чего
    /// `id_to_pos` пересобирается целиком: живой объект
    /// `СтрокаТаблицыЗначений`, взятый ДО сортировки, после неё продолжает
    /// указывать на ту же строку, просто стоящую в другом месте.
    pub fn sort(&mut self, keys: &[SortKey]) {
        let mut order: Vec<usize> = (0..self.row_count()).collect();
        order.sort_by(|&a, &b| {
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

        for col in &mut self.columns {
            *col = order.iter().map(|&i| col[i].clone()).collect();
        }
        self.row_ids = order.iter().map(|&i| self.row_ids[i]).collect();
        self.reindex();
    }

    /// Пересборка карты `id -> позиция` после любой перестановки строк.
    /// Вынесено отдельно, потому что этим кончаются `sort`, `move_row` и
    /// `collapse` — и любая забытая пересборка тихо ломает инвариант живых
    /// строк, а не падает.
    fn reindex(&mut self) {
        self.id_to_pos = self
            .row_ids
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();
    }

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    /// Текущая позиция строки; `None` — строка удалена (`Удалить`,
    /// `Очистить`, `Свернуть`) либо принадлежит другой таблице.
    pub fn pos_of(&self, row_id: u64) -> Option<usize> {
        self.id_to_pos.get(&row_id).copied()
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
            column_names: cols
                .iter()
                .filter_map(|&c| self.column_names.get(c).cloned())
                .collect(),
            columns: Vec::with_capacity(cols.len()),
            row_ids: Vec::with_capacity(rows.len()),
            id_to_pos: HashMap::with_capacity(rows.len()),
            next_id: 0,
        };
        for &c in cols {
            let src = &self.columns[c];
            out.columns
                .push(rows.iter().map(|&pos| src[pos].clone()).collect());
        }
        for pos in 0..rows.len() {
            out.row_ids.push(pos as u64);
            out.id_to_pos.insert(pos as u64, pos);
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
    /// колонок, и `row_ids`, а карта пересобирается (инвариант 12).
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
    // `BslValue` в ключе — то же самое, что делает `Соответствие`
    // (`MapData::values`): у ссылочных типов и хэш, и равенство берутся от
    // АДРЕСА `Rc`, а он под мутацией содержимого не меняется. Ключ группы
    // к тому же живёт только внутри этого вызова.
    #[allow(clippy::mutable_key_type)]
    pub fn collapse(&mut self, group: &[usize], sum: &[usize]) -> Result<(), NumError> {
        let mut slot_of: HashMap<Vec<BslValue>, usize> = HashMap::new();
        let mut keys: Vec<Vec<BslValue>> = Vec::new();
        let mut sums: Vec<Vec<BslNumber>> = Vec::new();
        let mut ids: Vec<u64> = Vec::new();

        for pos in 0..self.row_count() {
            let key: Vec<BslValue> = group
                .iter()
                .map(|&c| self.columns[c][pos].clone())
                .collect();
            let slot = match slot_of.get(&key) {
                Some(&slot) => slot,
                None => {
                    let slot = keys.len();
                    slot_of.insert(key.clone(), slot);
                    keys.push(key);
                    sums.push(vec![BslNumber::from_i64(0); sum.len()]);
                    ids.push(self.row_ids[pos]);
                    slot
                }
            };
            for (k, &c) in sum.iter().enumerate() {
                if let BslValue::Number(n) = &self.columns[c][pos] {
                    let acc = sums[slot][k].add(n)?;
                    sums[slot][k] = acc;
                }
            }
        }

        let mut columns: Vec<Vec<BslValue>> = Vec::with_capacity(group.len() + sum.len());
        for g in 0..group.len() {
            columns.push(keys.iter().map(|k| k[g].clone()).collect());
        }
        for s in 0..sum.len() {
            columns.push(
                sums.iter()
                    .map(|row| BslValue::Number(row[s].clone()))
                    .collect(),
            );
        }
        let names: Vec<String> = group
            .iter()
            .chain(sum.iter())
            .filter_map(|&c| self.column_names.get(c).cloned())
            .collect();
        self.column_names = names;
        self.columns = columns;
        self.row_ids = ids;
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
    match collation_key(a).cmp(&collation_key(b)) {
        Ordering::Equal => {}
        other => return other,
    }
    // Ключи равны — строки отличаются только регистром (или е/ё).
    // Строчная вперёд: у неё код БОЛЬШЕ, поэтому сравнение перевёрнуто.
    b.cmp(a)
}

/// Первичный ключ сравнения: нижний регистр, `ё` сведена к `е`.
fn collation_key(s: &BslString) -> Vec<char> {
    s.to_string()
        .to_lowercase()
        .chars()
        .map(|c| if c == 'ё' { 'е' } else { c })
        .collect()
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
        BslValue::Object(_) => 8,
        BslValue::Skipped => 9,
    }
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
