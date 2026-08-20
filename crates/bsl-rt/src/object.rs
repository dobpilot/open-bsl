use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::rc::Rc;

use crate::BslValue;
use crate::interner::NameId;
use crate::map::MapData;
use crate::shape::{Shape, ShapeTable};
use crate::table::ValueTableData;

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
    /// Элемент `Таблица.Колонки`. Имя служит устойчивым идентификатором:
    /// добавление соседних колонок не должно менять уже полученный объект.
    TableColumn(Rc<RefCell<ValueTableData>>, String),
    /// Живёт независимо от текущей позиции строки в колонках — см.
    /// `ValueTableData::pos_of`. `row_id` может перестать резолвиться
    /// после `Удалить`/`Очистить`.
    TableRow(Rc<RefCell<ValueTableData>>, u64),
    /// `ОписаниеТипов` в поддержанном подмножестве хранит перечисленные
    /// зарегистрированные типы. Ограничение значений колонки пока не
    /// применяется: объект нужен метаданным колонки и её конструктору.
    TypeDescription(Vec<crate::TypeId>),
    /// `СравнениеЗначений` — стратегия стандартного сравнения платформы,
    /// передаваемая вторым аргументом в `ТаблицаЗначений.Сортировать`.
    ValueComparison,
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

    /// `ДвоичныеДанные` — ИММУТАБЕЛЬНЫЙ буфер байтов, поэтому без
    /// `RefCell`: менять содержимое платформа не даёт, всякая операция
    /// (`РазделитьДвоичныеДанные`, `СоединитьДвоичныеДанные`) отдаёт новое
    /// значение. `Rc<[u8]>` вместо `Vec<u8>` ровно за этим: раз содержимое
    /// не меняется, копия значения — это счётчик ссылок, а не копия байтов.
    ///
    /// Равенство ПО СОДЕРЖИМОМУ, а не по тождеству объекта — измерено:
    /// два `Новый ДвоичныеДанные` от одного файла равны, от разных файлов
    /// нет. Отсюда же и хэш по байтам (см. `impl Hash for BslValue`).
    BinaryData(Rc<[u8]>),

    /// `БуферДвоичныхДанных` — ПРОТИВОПОЛОЖНОСТЬ соседа сверху: изменяемый
    /// буфер фиксированного размера, поэтому `Rc<RefCell<..>>`, а не
    /// `Rc<[u8]>`. Равенство здесь ПО ТОЖДЕСТВУ, а не по содержимому
    /// (измерено: два свежих нулевых буфера одного размера не равны), и
    /// потому отдельной ветки в `PartialEq`/`Hash` у него нет — работает
    /// общий путь через `Rc::ptr_eq`.
    BinaryBuffer(Rc<RefCell<crate::bindata::BinBufData>>),
    /// `УникальныйИдентификатор` — шестнадцать байтов UUID. Иммутабельное
    /// значение: равенство по байтам, печать канонической формой нижнего
    /// регистра (см. `crate::uuid`).
    Uuid([u8; 16]),

    /// Непрозрачное значение внутреннего формата (`ЗначениеИзСтрокиВнутр`):
    /// вид, который эта реализация не материализует — ссылки объектов базы
    /// (`СправочникСсылка`, `ДокументСсылка`, `ПеречислениеСсылка`),
    /// `СписокЗначений`, `УникальныйИдентификатор`, неизвестные `{"T",…}`.
    /// Хранится КАНОНИЧЕСКИ ОТРИСОВАННОЕ поддерево исходного текста и при
    /// обратной сериализации возвращается байт в байт — так значения
    /// неподдержанных типов переживают транзит 1С -> open-bsl -> 1С без
    /// потерь. Никаких операций над таким значением нет: только хранение,
    /// сравнение на равенство (по тексту) и обратная запись.
    VstrOpaque(String),

    /// Объект статически подключённого компонента.
    Extension(crate::ObjectRef),
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
