use std::collections::HashMap;
use std::rc::Rc;

use crate::interner::NameId;

/// Форма структуры ("hidden class"): упорядоченный список полей (порядок
/// вставки — важен для `Для Каждого`) плюс индекс имя -> слот для доступа
/// за O(1).
#[derive(Debug, PartialEq)]
pub struct Shape {
    pub names: Vec<NameId>,
    pub index: HashMap<NameId, u32>,
}

/// Таблица интернирования форм — целиком компиляционная сущность (см.
/// `NameInterner`). Формы интернируются ГЛОБАЛЬНО по нормализованному
/// списку полей в рамках одной компиляции модуля: если бы каждый вызов
/// `Новый Структура("x,y,z", ...)` заводил свой объект формы, инлайн-кэш на
/// горячий доступ к полю видел бы N разных указателей вместо одного —
/// полиморфный кэш вместо мономорфного.
#[derive(Debug, Default)]
pub struct ShapeTable {
    by_keys: HashMap<Vec<NameId>, u32>,
    shapes: Vec<Rc<Shape>>,
}

impl ShapeTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Возвращает индекс формы в итоговой таблице (см. `into_shapes`),
    /// заводя новую только если такого набора полей ещё не было.
    pub fn intern(&mut self, names: &[NameId]) -> u32 {
        if let Some(&id) = self.by_keys.get(names) {
            return id;
        }
        let id = self.shapes.len() as u32;
        let index = names.iter().enumerate().map(|(i, &n)| (n, i as u32)).collect();
        let shape = Rc::new(Shape {
            names: names.to_vec(),
            index,
        });
        self.shapes.push(shape);
        self.by_keys.insert(names.to_vec(), id);
        id
    }

    /// Готовая таблица форм в порядке интернирования, для встраивания в
    /// `Program`; VM клонирует `Rc<Shape>` из неё при исполнении
    /// `NewStructure` — новых форм в рантайме не заводится.
    pub fn into_shapes(self) -> Vec<Rc<Shape>> {
        self.shapes
    }
}
