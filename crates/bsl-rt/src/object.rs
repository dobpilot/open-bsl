use std::cell::RefCell;
use std::rc::Rc;

use crate::map::MapData;
use crate::shape::Shape;
use crate::table::ValueTableData;
use crate::BslValue;

/// Объекты BSL — `enum`, а не vtable: тонкий указатель (`Rc<BslObject>`) и
/// исчерпывающий `match` вместо динамической диспетчеризации. Управление
/// памятью — счётчик ссылок (`Rc`), не GC: `Clone` — retain, `Drop` —
/// release, и это совместимость с 1С (там утечки циклов наблюдаемы), а не
/// компромисс производительности.
///
/// Мутабельность — через `RefCell`: массивы/структуры в BSL — ссылочные
/// типы (`b = a` делает `b` тем же объектом, что и `a`, а не копией), и
/// методы `BslValue::get_index`/`set_index`/... поэтому берут `&self`, а не
/// `&mut self`.
#[derive(Debug)]
pub enum BslObject {
    Array(RefCell<Vec<BslValue>>),
    Structure(RefCell<StructureData>),
    ValueTable(Rc<RefCell<ValueTableData>>),
    /// `Таблица.Колонки` — отдельный объект-обёртка над теми же данными
    /// (тот же `Rc<RefCell<...>>`), только чтобы `.Добавить(имя)` на нём
    /// означало "добавить колонку", а не "добавить строку" — то же имя
    /// метода, разное поведение в зависимости от того, что за объект.
    TableColumns(Rc<RefCell<ValueTableData>>),
    /// Живёт независимо от текущей позиции строки в колонках — see
    /// `ValueTableData::id_to_pos`. `row_id` может перестать резолвиться
    /// после `Удалить`/`Очистить`.
    TableRow(Rc<RefCell<ValueTableData>>, u64),
    /// `Соответствие`. Ключ — любое значение (см. `impl Hash for
    /// BslValue`), а не только `Строка`/`Число` — как и в самой 1С.
    Map(RefCell<MapData>),
    /// Элемент `Для Каждого` по `Соответствие` (`КлючИЗначение.Ключ`/
    /// `.Значение`). Не структура с интернированной формой: поля здесь
    /// всегда ровно эти два и известны только рантайму, резолвятся строкой
    /// через `get_field_by_name` — тем же путём, что и колонки
    /// `СтрокаТаблицыЗначений`, а не через `Shape`/`NameId`.
    KeyValuePair(BslValue, BslValue),
}

#[derive(Debug)]
pub struct StructureData {
    pub shape: Rc<Shape>,
    pub slots: Vec<BslValue>,
}
