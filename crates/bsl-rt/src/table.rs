use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
}
