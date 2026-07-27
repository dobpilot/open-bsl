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
    /// Равенство — то же `eq_value`, что и у оператора `=`: у ссылочных
    /// типов это тождество объекта, а не структурное сравнение.
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
    /// НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC): что делает платформа с
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
        self.id_to_pos = self
            .row_ids
            .iter()
            .enumerate()
            .map(|(pos, &id)| (id, pos))
            .collect();
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
/// НЕ ИЗМЕРЕНО(TABLE.SORT.COLLATION), и это заведомое приближение. 1С
/// сортирует строки ПО
/// ЛОКАЛИ, а не по кодовым точкам: `ё` стоит после `е`, а не в конце
/// алфавита; регистр обычно игнорируется либо работает вторичным ключом;
/// кириллица и латиница упорядочены не по номерам символов. Здесь сделан
/// самый дешёвый вариант, дающий хотя бы регистронезависимость: сначала
/// сравнение приведённых к верхнему регистру строк, затем — исходных как
/// вторичный ключ (чтобы `а` и `А` не считались одним и тем же и порядок
/// оставался детерминированным).
///
/// Что НЕ делается и делаться должно после замера: `ё`/`е`, взаимный
/// порядок алфавитов, порядок цифр относительно букв. Если замер покажет,
/// что приближения не хватает, сюда встаёт полная UCA (`feruca`,
/// `icu_collator`) — заменяется ровно эта функция.
pub fn collate(a: &BslString, b: &BslString) -> Ordering {
    a.to_uppercase()
        .cmp(&b.to_uppercase())
        .then_with(|| a.cmp(b))
}

/// Порядок РАЗНОТИПНЫХ значений в сортировке. Нужен только затем, чтобы
/// сортировка колонки со смешанным содержимым была тотальной и
/// детерминированной, а не зависела от того, какие два элемента алгоритм
/// сравнил первыми.
///
/// НЕ ИЗМЕРЕНО(TABLE.SORT.TYPE_ORDER): в каком порядке платформа ставит
/// типы между собой. Взято
/// «пустые значения вперёд, дальше числа, даты, булево, строки, объекты» —
/// произвольно, но устойчиво.
fn type_rank(v: &BslValue) -> u8 {
    match v {
        BslValue::Undefined => 0,
        BslValue::Null => 1,
        BslValue::Number(_) => 2,
        BslValue::Date(_) => 3,
        BslValue::Boolean(_) => 4,
        BslValue::Str(_) => 5,
        BslValue::Type(_) => 6,
        BslValue::Object(_) => 7,
        BslValue::Skipped => 8,
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
