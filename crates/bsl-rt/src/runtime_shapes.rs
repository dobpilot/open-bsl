use std::rc::Rc;

use crate::interner::NameInterner;
use crate::shape::{Shape, ShapeTable};

/// Контекст рантайм-мутации структур (`Вставить`/`Удалить`/`Свойство`):
/// имена и формы вместе, потому что превращение строкового ключа в
/// `NameId` (интернер) и переход между формами по этому `NameId` (таблица
/// форм) — одна операция с точки зрения вызывающего VM-кода, а не два
/// независимых состояния, которые можно рассинхронизировать.
///
/// Живёт на время одного `run_program`/`run_repl_chunk`/динамического
/// сниппета — затравлена уже готовыми компиляционными именами/формами ЭТОЙ
/// программы (`seeded`), а не общая на весь процесс: у каждого `Program`
/// свои `names`/`shapes`, и рантайм-расширения этой таблицы актуальны
/// только для объектов, живущих внутри одного и того же исполнения.
pub struct RuntimeShapes {
    pub names: NameInterner,
    pub shapes: ShapeTable,
    /// Типы объектов, объявленные компонентами ЭТОГО прогона
    /// (`LibraryDescriptor::types`). По ним `Тип("Имя")` находит то, чего
    /// нет в закрытом реестре `TypeId`: у host-типа идентификатора там
    /// нет и быть не может. Пусто, пока связывание не заполнило список.
    pub component_types: Vec<&'static crate::TypeDescriptor>,
}

impl RuntimeShapes {
    pub fn seeded(names: Vec<String>, shapes: Vec<Rc<Shape>>) -> Self {
        RuntimeShapes {
            names: NameInterner::from_existing(names),
            shapes: ShapeTable::from_existing(shapes),
            component_types: Vec::new(),
        }
    }

    /// Тип компонента по имени — регистронезависимо, как всё в языке.
    pub fn component_type(&self, name: &str) -> Option<&'static crate::TypeDescriptor> {
        self.component_types
            .iter()
            .copied()
            .find(|descriptor| crate::folded_eq(descriptor.name, name))
    }
}
