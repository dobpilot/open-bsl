use std::collections::HashMap;

use crate::BslValue;

/// Данные `Соответствие`. Порядок вставки хранится отдельно от таблицы
/// быстрого поиска (как в `dict` большинства динамических языков) — нужен
/// для `Для Каждого`, детерминированного и не зависящего от того, как лёг
/// хэш. `values` даёт O(1) `Вставить`/`Получить`/`Удалить` вместо линейного
/// перебора: возможен благодаря тому, что `BslValue: Hash + Eq` уже
/// согласован со СТРОГИМ `PartialEq` (см. `impl Hash for BslValue` в
/// `lib.rs`), а не с ослабленным `eq_value` оператора `=` —
/// хэш числа не зависит от масштаба представления ровно потому, что
/// `BslNumber` нормализует хвостовые нули после каждой операции.
#[derive(Debug, Default)]
pub struct MapData {
    order: Vec<BslValue>,
    values: HashMap<BslValue, BslValue>,
}

impl MapData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: BslValue, value: BslValue) {
        if !self.values.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.values.insert(key, value);
    }

    pub fn get(&self, key: &BslValue) -> Option<BslValue> {
        self.values.get(key).cloned()
    }

    /// no-op, если ключа нет — как и `Структура.Удалить` на отсутствующем
    /// поле: убрать то, чего и так нет, не ошибка.
    pub fn remove(&mut self, key: &BslValue) {
        if self.values.remove(key).is_some() {
            self.order.retain(|k| k != key);
        }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.values.clear();
    }

    /// i-я по порядку вставки пара (ключ, значение) — протокол `Для
    /// Каждого`: компилятор всегда обходит коллекции числовой позицией
    /// `0..CollectionLen`, независимо от типа (см. `bsl-bytecode::compiler`
    /// `RStmtKind::ForEach`), а не по ключу — `GetIndex` на `Соответствие`
    /// поэтому позиционный, а не по ключу (см. doc comment на
    /// `BslValue::get_index`).
    pub fn entry_at(&self, i: usize) -> Option<(BslValue, BslValue)> {
        let k = self.order.get(i)?;
        let v = self.values.get(k)?;
        Some((k.clone(), v.clone()))
    }
}
