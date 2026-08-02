use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::rc::Rc;

use crate::interner::NameId;
use crate::map::MapData;
use crate::shape::{Shape, ShapeTable};
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
    Structure(RefCell<StructureStorage>),
    ValueTable(Rc<RefCell<ValueTableData>>),
    /// `Таблица.Колонки` — отдельный объект-обёртка над теми же данными
    /// (тот же `Rc<RefCell<...>>`), только чтобы `.Добавить(имя)` на нём
    /// означало "добавить колонку", а не "добавить строку" — то же имя
    /// метода, разное поведение в зависимости от того, что за объект.
    TableColumns(Rc<RefCell<ValueTableData>>),
    /// Живёт независимо от текущей позиции строки в колонках — см.
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
    /// Буферизованный `ЗаписьТекста`; `None` после `Закрыть()`.
    TextWriter(RefCell<Option<BufWriter<File>>>),

    /// `ЧтениеJSON`. Разборщик появляется только после
    /// `УстановитьСтроку`/`ОткрытьФайл` — до этого читать нечего, и
    /// платформа на попытку тоже отвечает ошибкой.
    JsonReader(RefCell<JsonReaderState>),
    /// `ЗаписьJSON`. Так же: приёмник назначается отдельным вызовом.
    JsonWriter(RefCell<Option<crate::json::JsonWriter>>),
    /// `ПараметрыЗаписиJSON` — простой набор значений, менять его после
    /// создания незачем, поэтому без `RefCell`.
    JsonWriterSettings(crate::json::JsonWriterSettings),

    /// `ЧтениеXML`. Как и у JSON, разборщик появляется только после
    /// `УстановитьСтроку`/`ОткрытьФайл`.
    XmlReader(RefCell<XmlReaderState>),
    /// `ЗаписьXML`.
    XmlWriter(RefCell<Option<crate::xml::XmlWriter>>),
    /// `ПараметрыЗаписиXML`.
    XmlWriterSettings(crate::xml::XmlWriterSettings),

    /// `ТекстовыйДокумент`.
    TextDocument(Rc<RefCell<crate::textdoc::TextDocData>>),
    /// `Документ.Параметры` — отдельный объект-обёртка над ТЕМИ ЖЕ данными
    /// (тот же `Rc`), как `Таблица.Колонки`: присваивание в его поле
    /// означает «задать параметр макета», а не «положить поле объекта».
    TextDocParams(Rc<RefCell<crate::textdoc::TextDocData>>),
}

/// Состояние `ЧтениеXML`.
///
/// Кроме текущего события хранится ещё две вещи, и обе — из-за того, что у
/// платформы курсор один на две сущности. `attr_cursor` — позиция обхода
/// `ПрочитатьАтрибут`: пока он стоит на атрибуте, `ТипУзла`, `Имя` и
/// `Значение` показывают АТРИБУТ, а не сам элемент (измерено). `depth` —
/// глубина открытых элементов на момент текущего события: по ней
/// `Пропустить` понимает, до какого закрывающего тега глотать, и она обязана
/// быть снята ДО того, как разборщик уйдёт вперёд.
#[derive(Debug, Default)]
pub struct XmlReaderState {
    pub parser: Option<crate::xml::XmlParser>,
    pub current: Option<crate::xml::XmlEvent>,
    pub attr_cursor: Option<usize>,
    pub depth: usize,
}

impl XmlReaderState {
    /// Свежее состояние над готовым разборщиком.
    pub fn over(parser: crate::xml::XmlParser) -> Self {
        XmlReaderState {
            parser: Some(parser),
            current: None,
            attr_cursor: None,
            depth: 0,
        }
    }

    /// Атрибуты текущего узла; у неэлементного узла их нет.
    pub fn attrs(&self) -> &[crate::xml::XmlAttr] {
        match &self.current {
            Some(crate::xml::XmlEvent::ElementStart { attrs, .. }) => attrs,
            _ => &[],
        }
    }

    /// Атрибут, на котором стоит курсор `ПрочитатьАтрибут`.
    pub fn current_attr(&self) -> Option<&crate::xml::XmlAttr> {
        self.attr_cursor.and_then(|i| self.attrs().get(i))
    }
}

/// Состояние `ЧтениеJSON`: сам разборщик и последнее прочитанное событие.
///
/// Событие хранится, потому что `ТипТекущегоЗначения` и `ТекущееЗначение`
/// — СВОЙСТВА, а не результат `Прочитать()`: читатель обязан помнить, где
/// он стоит, между двумя обращениями к нему.
#[derive(Debug, Default)]
pub struct JsonReaderState {
    pub parser: Option<crate::json::JsonParser>,
    pub current: Option<crate::json::JsonEvent>,
}

/// Хранение полей `Структура` — двухрежимное, как в V8.
///
/// `Shaped` — быстрый путь: имя поля резолвится в номер слота через
/// разделяемую `Rc<Shape>`, а инлайн-кэш на месте вызова (`GetProp`/
/// `SetProp`) сравнивает форму указателем и читает слот напрямую.
///
/// `Dictionary` — путь деградации: объект, чья форма ушла глубже
/// `MAX_SHAPE_TRANSITIONS` шагов по цепочке переходов, перестаёт заводить
/// формы совсем (см. doc comment на константе — там же про то, почему
/// выселять формы нельзя). Платит поиском по `HashMap` и промахом
/// инлайн-кэша, зато `Вставить` в цикле с динамическими именами больше не
/// растит `ShapeTable`.
///
/// Переход ТОЛЬКО в одну сторону, `Shaped` -> `Dictionary`. Обратная
/// деградация после `Удалить` выглядит соблазнительно (набор полей ведь
/// снова короткий), но сделала бы поведение инлайн-кэша непредсказуемым:
/// один и тот же сайт вызова то попадал бы, то нет, в зависимости от
/// истории мутаций конкретного объекта.
#[derive(Debug)]
pub enum StructureStorage {
    Shaped {
        shape: Rc<Shape>,
        slots: Vec<BslValue>,
    },
    /// Порядок вставки хранится отдельно от таблицы поиска — ровно как
    /// в `MapData`, и по той же причине: `Для Каждого` обязан быть
    /// детерминированным.
    Dictionary {
        order: Vec<NameId>,
        values: HashMap<NameId, BslValue>,
    },
}

impl StructureStorage {
    pub fn new(shape: Rc<Shape>, slots: Vec<BslValue>) -> Self {
        StructureStorage::Shaped { shape, slots }
    }

    pub fn len(&self) -> usize {
        match self {
            StructureStorage::Shaped { slots, .. } => slots.len(),
            StructureStorage::Dictionary { order, .. } => order.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, field: NameId) -> Option<BslValue> {
        match self {
            StructureStorage::Shaped { shape, slots } => {
                shape.index.get(&field).map(|&s| slots[s as usize].clone())
            }
            StructureStorage::Dictionary { values, .. } => values.get(&field).cloned(),
        }
    }

    /// `false` — поля нет; заводить его здесь нельзя (это `а.Поле = ...`,
    /// а не `Вставить`: в 1С присваивание несуществующему полю структуры
    /// ошибка, а не создание поля).
    pub fn set(&mut self, field: NameId, val: BslValue) -> bool {
        match self {
            StructureStorage::Shaped { shape, slots } => match shape.index.get(&field) {
                Some(&s) => {
                    slots[s as usize] = val;
                    true
                }
                None => false,
            },
            StructureStorage::Dictionary { values, .. } => match values.get_mut(&field) {
                Some(slot) => {
                    *slot = val;
                    true
                }
                None => false,
            },
        }
    }

    /// `Вставить`: поле уже есть — перезапись значения, нет — расширение
    /// набора полей (сменой формы либо, если порог переходов исчерпан,
    /// с переключением объекта в словарный режим).
    pub fn insert(&mut self, field: NameId, val: BslValue, shapes: &mut ShapeTable) {
        match self {
            StructureStorage::Shaped { shape, slots } => {
                if let Some(&slot) = shape.index.get(&field) {
                    slots[slot as usize] = val;
                    return;
                }
                if let Some(next) = shapes.add_field(shape, field) {
                    slots.push(val);
                    *shape = next;
                    return;
                }
            }
            StructureStorage::Dictionary { order, values } => {
                if values.insert(field, val).is_none() {
                    order.push(field);
                }
                return;
            }
        }
        // Сюда доходит только `Shaped`, которому `add_field` отказал:
        // порог переходов исчерпан. Переключаемся на словарь и повторяем
        // вставку уже в нём — рекурсия ровно на один уровень, `degrade`
        // гарантированно оставляет `Dictionary`.
        self.degrade();
        self.insert(field, val, shapes);
    }

    /// `Удалить`: поля нет — no-op (см. `BslValue::structure_delete`).
    /// В словарном режиме порядок ОСТАЛЬНЫХ полей сохраняется — `order`
    /// теряет ровно один элемент, как `slots` в шейповом режиме.
    pub fn remove(&mut self, field: NameId, shapes: &mut ShapeTable) {
        match self {
            StructureStorage::Shaped { shape, slots } => {
                if let Some(&slot) = shape.index.get(&field) {
                    slots.remove(slot as usize);
                    *shape = shapes.remove_field(shape, field);
                }
            }
            StructureStorage::Dictionary { order, values } => {
                if values.remove(&field).is_some() {
                    order.retain(|&n| n != field);
                }
            }
        }
    }

    /// `Очистить()` — сбрасывает НАБОР полей, не только значения.
    /// Словарный объект остаётся словарным: деградация в одну сторону (см.
    /// doc comment на типе), даже когда полей снова ноль.
    pub fn clear(&mut self, shapes: &mut ShapeTable) {
        match self {
            StructureStorage::Shaped { shape, slots } => {
                slots.clear();
                *shape = shapes.empty();
            }
            StructureStorage::Dictionary { order, values } => {
                order.clear();
                values.clear();
            }
        }
    }

    /// i-я по порядку вставки пара (имя поля, значение) — протокол `Для
    /// Каждого`, тот же позиционный обход, что и у `MapData::entry_at`.
    pub fn entry_at(&self, i: usize) -> Option<(NameId, BslValue)> {
        match self {
            StructureStorage::Shaped { shape, slots } => {
                Some((*shape.names.get(i)?, slots.get(i)?.clone()))
            }
            StructureStorage::Dictionary { order, values } => {
                let n = *order.get(i)?;
                Some((n, values.get(&n)?.clone()))
            }
        }
    }

    /// Переключение на словарное хранение с сохранением порядка полей.
    fn degrade(&mut self) {
        let StructureStorage::Shaped { shape, slots } = std::mem::replace(
            self,
            StructureStorage::Dictionary {
                order: Vec::new(),
                values: HashMap::new(),
            },
        ) else {
            return;
        };
        let order = shape.names.clone();
        let values = order.iter().copied().zip(slots).collect();
        *self = StructureStorage::Dictionary { order, values };
    }
}
