//! Полиморфная индексация, свойства и операции коллекций значения BSL.

use super::BslValue;
use crate::map::MapData;
use crate::{
    BslObject, BslString, NameId, NameInterner, RtError, RtResult, Shape, ShapeTable,
    StructureStorage, folded_eq,
};
use crate::{TypeRef, bindata};
use std::rc::Rc;

impl BslValue {
    // --- Коллекции ----------------------------------------------------

    pub fn new_array(items: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Array(std::cell::RefCell::new(items))))
    }

    pub fn new_structure(shape: Rc<Shape>, slots: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Structure(std::cell::RefCell::new(
            StructureStorage::new(shape, slots),
        ))))
    }

    pub fn new_map() -> Self {
        BslValue::Object(Rc::new(BslObject::Map(std::cell::RefCell::new(
            MapData::new(),
        ))))
    }

    /// `names` нужен единственной ветке — `Структура`: её поля хранятся
    /// идентификаторами (`NameId`), а `Для Каждого` обязан отдать ключ
    /// пользовательскому коду СТРОКОЙ (`КлючИЗначение.Ключ`). Тащить сюда
    /// интернер целиком дешевле, чем держать в каждой структуре ещё и
    /// строковые имена рядом с формой.
    pub fn get_index(&self, idx: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.get_index(idx),
                BslObject::Array(v) => {
                    let v = v.borrow();
                    let i = Self::index_as_usize(idx)?;
                    v.get(i).cloned().ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: v.len(),
                    })
                }
                BslObject::ValueTable(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let row_id = {
                        let d = data.borrow();
                        d.row_id_at(i).ok_or(RtError::IndexOutOfBounds {
                            index: i as i64,
                            len: d.row_count(),
                        })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(
                        data.clone(),
                        row_id,
                    ))))
                }
                BslObject::TableColumns(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let name = {
                        let d = data.borrow();
                        d.column_names
                            .get(i)
                            .cloned()
                            .ok_or(RtError::IndexOutOfBounds {
                                index: i as i64,
                                len: d.column_names.len(),
                            })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableColumn(
                        data.clone(),
                        name,
                    ))))
                }
                // `СтрокаТаблицы[Ключ]` — значение ячейки: строковый ключ —
                // имя колонки (тот же путь, что `Строка.Имя` через
                // `get_field_by_name`), числовой — её номер.
                BslObject::TableRow(..) => match idx {
                    BslValue::Str(name) => self.get_field_by_name(&name.to_string()),
                    _ => {
                        let i = Self::index_as_usize(idx)?;
                        let name = match &**o {
                            BslObject::TableRow(data, _) => {
                                let d = data.borrow();
                                d.column_names
                                    .get(i)
                                    .cloned()
                                    .ok_or(RtError::IndexOutOfBounds {
                                        index: i as i64,
                                        len: d.column_names.len(),
                                    })?
                            }
                            _ => unreachable!("вариант проверен объемлющим match"),
                        };
                        self.get_field_by_name(&name)
                    }
                },
                // Строковый индекс — ключ, как в BSL. Числовой пока остаётся
                // ПОЗИЦИОННЫМ: `Для Каждого` компилируется в
                // общий для всех коллекций протокол `CollectionLen` + рост
                // числового индекса `0..len` через эту же функцию (см.
                // `bsl-bytecode::compiler::RStmtKind::ForEach`) — компилятор не
                // знает на этапе компиляции, что `idx` в рантайме окажется
                // `Соответствие`, и не может эмитить для него другой путь.
                // Доступ ПО КЛЮЧУ у `Соответствие` поэтому сознательно НЕ
                // здесь, а в `.Получить(Ключ)` (`map_get`) — если бы `[]`
                // тоже читал по ключу, `м[0]` было бы неразрешимо
                // неоднозначно между "0-я по счёту пара" и "значение по
                // ключу 0" для карты с целочисленными ключами.
                BslObject::Map(data) if matches!(idx, BslValue::Str(_)) => {
                    Ok(data.borrow().get(idx).unwrap_or(BslValue::Undefined))
                }
                BslObject::Map(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let data = data.borrow();
                    let (k, v) = data.entry_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: data.len(),
                    })?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(k, v))))
                }
                // `Для Каждого КиЗ Из Структура` — тот же протокол, что и у
                // `Соответствие` (`CollectionLen` + позиционный обход), и та
                // же пара `Ключ`/`Значение` на выходе. Порядок — вставки, в
                // обоих режимах хранения (`StructureStorage::entry_at`).
                BslObject::Structure(s) if matches!(idx, BslValue::Str(_)) => {
                    let BslValue::Str(name) = idx else {
                        unreachable!("вариант проверен защитой match")
                    };
                    let text = name.to_string();
                    let field = names
                        .lookup(&text)
                        .ok_or_else(|| RtError::UnknownColumn(text.clone()))?;
                    s.borrow()
                        .get(field)
                        .ok_or_else(|| RtError::UnknownColumn(text))
                }
                BslObject::Structure(s) => {
                    let i = Self::index_as_usize(idx)?;
                    let s = s.borrow();
                    let (n, v) = s.entry_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: s.len(),
                    })?;
                    let key = names.name(n).ok_or(RtError::UnknownField(n))?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(
                        BslValue::Str(BslString::from_str(key)),
                        v,
                    ))))
                }
                // `Буфер[Позиция]` -> `Число` 0..255. Индекс здесь свой, не
                // общий `index_as_usize`: у буфера дробная позиция не
                // ошибка, а отбрасывается к нулю (измерено).
                BslObject::BinaryBuffer(_) => bindata::get_byte(self, idx),
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn set_index(&self, idx: &BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.set_index(idx, val),
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    let slot = v.get_mut(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    })?;
                    *slot = val;
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().insert(idx.clone(), val);
                    Ok(())
                }
                // Буфер меняется по числовой позиции; соответствие выше —
                // по значению ключа.
                BslObject::BinaryBuffer(_) => bindata::set_byte(self, idx, &val),
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    /// Длина коллекции — используется и `Для Каждого` (компилируется в
    /// индексный цикл поверх этой длины), и `Количество()`.
    pub fn collection_len(&self) -> RtResult<usize> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.collection_len(),
                BslObject::Array(v) => Ok(v.borrow().len()),
                BslObject::Structure(s) => Ok(s.borrow().len()),
                BslObject::ValueTable(data) => Ok(data.borrow().row_count()),
                BslObject::TableColumns(data) => Ok(data.borrow().column_names.len()),
                BslObject::TableColumn(..)
                | BslObject::TypeDescription(_)
                | BslObject::ValueComparison => Err(RtError::NotIndexable),
                BslObject::TableRow(..) => Err(RtError::NotIndexable),
                BslObject::Map(data) => Ok(data.borrow().len()),
                BslObject::KeyValuePair(..) => Err(RtError::NotIndexable),
                BslObject::VstrOpaque(_) => Err(RtError::NotIndexable),
                // Число байтов отдаёт `Размер()`, а `Количество()` у этого
                // типа нет вовсе — как нет и обхода `Для Каждого`:
                // двоичные данные не коллекция, доступа к отдельному байту
                // здесь не заведено (он появится с `БуферДвоичныхДанных`).
                BslObject::BinaryData(..) => Err(RtError::NotIndexable),
                // Число байтов буфера отдаёт СВОЙСТВО `Размер`, а
                // `Количество()` платформа на нём отвергает — измерено.
                // (`Для Каждого` по буферу она при этом принимает; обход
                // здесь не заведён, потому что в задачу этого типа он не
                // входит, и своего эталона у него ещё нет.)
                BslObject::BinaryBuffer(..) => Err(RtError::NotIndexable),
                BslObject::Uuid(..) => Err(RtError::NotIndexable),
                BslObject::TextWriter(..) => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn get_field(&self, name: NameId) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => s.borrow().get(name).ok_or(RtError::UnknownField(name)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field(&self, name: NameId, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    if s.borrow_mut().set(name, val) {
                        Ok(())
                    } else {
                        Err(RtError::UnknownField(name))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Рантайм-мутация формы структуры ---------------------------------
    //
    // `Вставить`/`Удалить`/`Свойство` (двухаргументная форма) — в отличие
    // от `get_field`/`set_field` выше, которые лишь ЧИТАЮТ уже готовую
    // форму, эти три меняют её: `ShapeTable` здесь больше не голая таблица
    // компиляции, а рантайм-контекст (`RuntimeShapes`, см. одноимённый
    // модуль), поэтому и подписи ниже берут `&mut ShapeTable`, а не
    // работают в изоляции. Инлайн-кэш `GetProp`/`SetProp` (`Rc::ptr_eq` на
    // `s.shape`) сам заметит смену формы после любой из них — ничего
    // специально инвалидировать не нужно.

    /// `Структура.Вставить(Ключ, Значение)`. Поле уже есть — просто новое
    /// значение на том же слоте, форма не меняется (у 1С `Вставить`
    /// повторного поля — это не ошибка и не дубликат, а перезапись).
    pub fn structure_insert(
        &self,
        field: NameId,
        val: BslValue,
        shapes: &mut ShapeTable,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().insert(field, val, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Удалить(Ключ)`. Поля нет — no-op (симметрично
    /// `MapData::remove`, см. его doc comment): убрать то, чего и так нет,
    /// не повод падать.
    pub fn structure_delete(&self, field: NameId, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().remove(field, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Свойство(Ключ)` / `Структура.Свойство(Ключ,
    /// ЗначениеПоУмолчанию)`.
    ///
    /// Одноместная форма возвращает `Булево` наличия поля, как платформа.
    /// ОТКЛОНЕНИЕ остаётся только у двухместной формы: настоящий второй
    /// параметр — выходной ПО ССЫЛКЕ, но `CallMethod` пока не несёт
    /// `ArgMode::ByRefLocal`. До появления такого ABI он трактуется как
    /// значение по умолчанию безопасного геттера.
    pub fn structure_property(
        &self,
        field: NameId,
        default: Option<BslValue>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let value = s.borrow().get(field);
                    match default {
                        None => Ok(BslValue::Boolean(value.is_some())),
                        Some(default) => Ok(value.unwrap_or(default)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Очистить()` — сбрасывает набор полей целиком (форма
    /// становится пустой), не только значения на месте: у 1С `Очистить()`
    /// на структуре убирает и сами поля, следующий `Свойство`/`.Х` их уже
    /// не найдёт.
    pub fn structure_clear(&self, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().clear(shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Вставить(Ключ, Значение)`.
    pub fn map_insert(&self, key: BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => {
                    data.borrow_mut().insert(key, val);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Получить(Ключ)` — `Неопределено`, если ключа нет, не
    /// ошибка (соответствует `MapData::get`/реальной 1С).
    pub fn map_get(&self, key: &BslValue) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => Ok(data.borrow().get(key).unwrap_or(BslValue::Undefined)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Снимок пар `Соответствие` в порядке вставки. Нужен компонентам,
    /// которые переводят коллекцию в нейтральный DTO до внешнего эффекта.
    pub fn map_entries(&self) -> RtResult<Vec<(BslValue, BslValue)>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::Map(data) => {
                    let data = data.borrow();
                    Ok((0..data.len())
                        .filter_map(|index| data.entry_at(index))
                        .collect())
                }
                _ => Err(RtError::TypeError {
                    expected: "Соответствие",
                    op: "получение пар соответствия",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Соответствие",
                op: "получение пар соответствия",
            }),
        }
    }

    /// Инлайн-кэш для `GetProp` (см. брифовский план оптимизаций: «слот
    /// хранит (`shape_ptr`, `slot_idx`)»). `cache` — одна ячейка на конкретную
    /// инструкцию, живущая в состоянии запуска VM (по ячейке на пару
    /// «чанк, `pc`»), между исполнениями этой инструкции внутри прогона. Промах — обычный поиск
    /// по `Shape::index` плюс запись в кэш; форма меняется редко (обычно
    /// вообще никогда для данной инструкции — иначе откуда там структура
    /// другой формы), так что кэш почти всегда мономорфный.
    ///
    /// Держим `Rc<Shape>` целиком, а не голый указатель: так кэш не может
    /// протухнуть на чужой адрес, если форма где-то освободится — он сам
    /// продлевает ей жизнь, пока висит в кэше.
    ///
    /// Словарная структура (`StructureStorage::Dictionary`) формы не имеет
    /// вообще, поэтому ВСЕГДА промахивается мимо кэша и идёт в `HashMap` —
    /// и, что важнее, НЕ ТРОГАЕТ ячейку кэша. Если бы словарный объект
    /// затирал её (хоть чем — своим отсутствием формы, `None`), то шейповые
    /// объекты на том же сайте вызова теряли бы быстрый путь после каждого
    /// прохода словарного, то есть навсегда в смешанном цикле.
    pub fn get_field_cached(
        &self,
        name: NameId,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &*s.borrow() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref()
                            && Rc::ptr_eq(cached_shape, shape)
                        {
                            return Ok(slots[*slot as usize].clone());
                        }
                        match shape.index.get(&name) {
                            Some(&slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                Ok(slots[slot as usize].clone())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    StructureStorage::Dictionary { values, .. } => values
                        .get(&name)
                        .cloned()
                        .ok_or(RtError::UnknownField(name)),
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `SetProp` — см. `get_field_cached`.
    pub fn set_field_cached(
        &self,
        name: NameId,
        val: BslValue,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &mut *s.borrow_mut() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref()
                            && Rc::ptr_eq(cached_shape, shape)
                        {
                            slots[*slot as usize] = val;
                            return Ok(());
                        }
                        match shape.index.get(&name).copied() {
                            Some(slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                slots[slot as usize] = val;
                                Ok(())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    // Кэш не трогаем — см. `get_field_cached`.
                    StructureStorage::Dictionary { values, .. } => match values.get_mut(&name) {
                        Some(slot) => {
                            *slot = val;
                            Ok(())
                        }
                        None => Err(RtError::UnknownField(name)),
                    },
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Резолвинг поля/псевдо-свойства по ИМЕНИ (не `NameId`) — нужен для
    /// объектов, чьи "поля" известны только в рантайме: колонки
    /// `СтрокиТаблицыЗначений` заводятся через `.Колонки.Добавить(имя)`, а
    /// не как статичная форма структуры, поэтому по ним нельзя
    /// интернировать `NameId` на этапе компиляции. `Структура` в эту
    /// функцию не заходит — у неё есть более быстрый путь через
    /// `get_field`/`NameId`, здесь она просто не находится.
    ///
    /// Имена сравниваются через [`folded_eq`], а не `eq_ignore_ascii_case`:
    /// последняя не сворачивает кириллицу, и `КЗ.значение` не совпадало с
    /// `Значение` — при том что в языке имена регистронезависимы. Путь
    /// холодный (у `Структуры` свой), а `folded_eq` начинает с побайтового
    /// равенства, так что каноничное написание не платит ничего.
    pub fn get_field_by_name(&self, name: &str) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    if folded_eq(name, "Колонки") || folded_eq(name, "Columns") {
                        Ok(BslValue::Object(Rc::new(BslObject::TableColumns(
                            data.clone(),
                        ))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::TableRow(data, row_id) => {
                    let data = data.borrow();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.get_cell(*row_id, col).ok_or(RtError::RowInvalidated)
                }
                BslObject::TableColumn(data, column_name) => {
                    let column = data
                        .borrow()
                        .column_index(column_name)
                        .ok_or_else(|| RtError::UnknownColumn(column_name.clone()))?;
                    if folded_eq(name, "Имя") || folded_eq(name, "Name") {
                        Ok(BslValue::Str(BslString::from_str(column_name)))
                    } else if folded_eq(name, "ТипЗначения") || folded_eq(name, "ValueType")
                    {
                        let types: Vec<TypeRef> = data
                            .borrow()
                            .column_types
                            .get(column)
                            .cloned()
                            .flatten()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|t| t.id)
                            .collect();
                        Ok(BslValue::Object(Rc::new(BslObject::TypeDescription(types))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                // У буфера `Размер` и `ПорядокБайтов` — именно СВОЙСТВА:
                // `Б.Размер()` со скобками платформа отвергает (измерено),
                // поэтому оба живут здесь, а не в таблице методов.
                BslObject::BinaryBuffer(_) => {
                    if folded_eq(name, "Размер") || folded_eq(name, "Size") {
                        bindata::size(self)
                    } else if folded_eq(name, "ПорядокБайтов") || folded_eq(name, "ByteOrder")
                    {
                        bindata::get_order(self)
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::KeyValuePair(k, v) => {
                    if folded_eq(name, "Ключ") || folded_eq(name, "Key") {
                        Ok(k.clone())
                    } else if folded_eq(name, "Значение") || folded_eq(name, "Value") {
                        Ok(v.clone())
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field_by_name(&self, name: &str, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                // Пишется только `ПорядокБайтов`: `Размер` доступен лишь на
                // чтение, присваивание в него платформа отвергает
                // (измерено — прежний размер при этом уцелел).
                BslObject::BinaryBuffer(_) => {
                    if folded_eq(name, "ПорядокБайтов") || folded_eq(name, "ByteOrder")
                    {
                        bindata::set_order(self, val)
                    } else if folded_eq(name, "Размер") || folded_eq(name, "Size") {
                        Err(RtError::TypeError {
                            expected: "Свойство, доступное для записи",
                            op: "Размер",
                        })
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                // Узлы DOM: пишутся значение, данные и текстовое
                BslObject::TableRow(data, row_id) => {
                    let mut data = data.borrow_mut();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.set_cell(*row_id, col, val)
                        .ok_or(RtError::RowInvalidated)
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Методы, полиморфные по типу получателя --------------------------
    //
    // `Добавить`/`Удалить`/`Очистить` в реальной 1С означают разное в
    // зависимости от типа получателя (элемент массива, строка таблицы,
    // колонка, ...) — то же имя метода, разное поведение и разная арность.
    // Резолвинг имени в `bsl-sema` не может знать заранее, каким объектом
    // оказится `obj` в рантайме (BSL — динамически типизированный), поэтому
    // диспетчеризация и проверка арности — здесь, в рантайме, а не на этапе
    // компиляции.

    /// `Массив.Добавить(значение)`.
    pub fn push_element(&self, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().push(val);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Удалить(индекс)` / `ТаблицаЗначений.Удалить(индекс)`.
    pub fn delete_element(&self, idx: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    if i >= len {
                        return Err(RtError::IndexOutOfBounds {
                            index: i as i64,
                            len,
                        });
                    }
                    v.remove(i);
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    let mut d = data.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = d.row_count();
                    d.delete_row_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    })
                }
                // `Соответствие.Удалить(Ключ)` — по значению ключа, не по
                // позиции (в отличие от Array/ValueTable выше): в этом
                // случае `idx` в имени параметра функции вводит в
                // заблуждение, но сигнатура (`&BslValue`) уже общая для
                // всех получателей, менять её ради одного случая не стоит.
                BslObject::Map(data) => {
                    data.borrow_mut().remove(idx);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Удалить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Удалить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Очистить()` / `ТаблицаЗначений.Очистить()` /
    /// `Соответствие.Очистить()`.
    pub fn clear_collection(&self) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().clear();
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Очистить",
                receiver: self.type_name(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::num;
    use crate::{MAX_SHAPE_TRANSITIONS, RuntimeShapes};

    /// Имена членов в языке регистронезависимы, и кириллица здесь не
    /// исключение: `eq_ignore_ascii_case` сворачивала только ASCII, из-за
    /// чего `КЗ.значение` строчными не находило `Значение`. Корпус
    /// фикстур этого не ловил: имя приходит сюда в написании ПЕРВОГО
    /// вхождения в программе, а фикстуры пишут его канонично.
    #[test]
    fn native_property_names_fold_cyrillic_in_both_directions() {
        let pair = BslValue::Object(Rc::new(BslObject::KeyValuePair(
            BslValue::Str(BslString::from_str("к")),
            BslValue::Str(BslString::from_str("з")),
        )));
        for name in ["Значение", "значение", "ЗНАЧЕНИЕ", "Value", "value"] {
            assert_eq!(
                pair.get_field_by_name(name).expect(name),
                BslValue::Str(BslString::from_str("з")),
                "член «{name}»"
            );
        }
        for name in ["Ключ", "ключ", "КЛЮЧ"] {
            assert_eq!(
                pair.get_field_by_name(name).expect(name),
                BslValue::Str(BslString::from_str("к")),
                "член «{name}»"
            );
        }
        assert!(pair.get_field_by_name("НетТакого").is_err());
    }

    #[test]
    fn array_index_get_set_roundtrip() {
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        assert_eq!(
            arr.get_index(&num("1"), &NameInterner::new()).unwrap(),
            num("2")
        );
        arr.set_index(&num("1"), num("99")).unwrap();
        assert_eq!(
            arr.get_index(&num("1"), &NameInterner::new()).unwrap(),
            num("99")
        );
        assert_eq!(arr.collection_len().unwrap(), 3);
    }

    #[test]
    fn array_out_of_bounds_is_an_error() {
        let arr = BslValue::new_array(vec![num("1")]);
        assert!(matches!(
            arr.get_index(&num("5"), &NameInterner::new()).unwrap_err(),
            RtError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn arrays_and_structures_are_reference_types() {
        // b = a делает b тем же объектом, что и a: мутация через одну
        // переменную видна через другую (Rc, не глубокое копирование).
        let a = BslValue::new_array(vec![num("1")]);
        let b = a.clone();
        b.set_index(&num("0"), num("42")).unwrap();
        assert_eq!(
            a.get_index(&num("0"), &NameInterner::new()).unwrap(),
            num("42")
        );
        assert!(a.eq_value(&b));

        let c = BslValue::new_array(vec![num("42")]);
        assert!(
            !a.eq_value(&c),
            "структурно равные, но разные объекты — не равны"
        );
    }

    #[test]
    fn structure_field_get_set_by_interned_name() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let y = names.intern("y");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x, y]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1"), num("2")]);
        assert_eq!(s.get_field(x).unwrap(), num("1"));
        s.set_field(y, num("99")).unwrap();
        assert_eq!(s.get_field(y).unwrap(), num("99"));
    }

    #[test]
    fn unknown_field_is_an_error() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let z = names.intern("z");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1")]);
        assert!(matches!(
            s.get_field(z).unwrap_err(),
            RtError::UnknownField(_)
        ));
    }

    // --- Словарный режим структуры ------------------------------------
    //
    // Порог `MAX_SHAPE_TRANSITIONS` — единственная защита от того, что
    // `Вставить` с динамическим именем в цикле навсегда интернирует форму
    // на каждой итерации (см. doc comment на константе). Тесты ниже
    // фиксируют и сам переход, и то, ради чего он затеян: таблица форм
    // после него перестаёт расти.

    /// Пустая структура плюс рантайм-контекст форм — общая затравка для
    /// тестов деградации.
    fn fresh_structure() -> (BslValue, RuntimeShapes) {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let empty = rt.shapes.empty();
        (BslValue::new_structure(empty, Vec::new()), rt)
    }

    fn is_dictionary(v: &BslValue) -> bool {
        match v {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    matches!(&*s.borrow(), StructureStorage::Dictionary { .. })
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// `Вставить("Поле<i>", i)` — ровно тот путь, на котором форма уходит
    /// вглубь: каждый ключ новый, так что каждый переход заводил бы форму.
    fn insert_generated_fields(s: &BslValue, rt: &mut RuntimeShapes, count: u32) {
        for i in 0..count {
            let f = rt.names.intern(&format!("Поле{i}"));
            s.structure_insert(f, num(&i.to_string()), &mut rt.shapes)
                .unwrap();
        }
    }

    #[test]
    fn structure_degrades_to_dictionary_after_threshold_inserts() {
        let (s, mut rt) = fresh_structure();

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS);
        assert!(
            !is_dictionary(&s),
            "ровно {MAX_SHAPE_TRANSITIONS} переходов должны укладываться в порог"
        );

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 1);
        assert!(
            is_dictionary(&s),
            "переход за порог обязан деградировать объект"
        );
    }

    #[test]
    fn dictionary_structure_field_get_set_roundtrip() {
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 5);
        assert!(is_dictionary(&s));

        // Поля, заведённые ДО деградации (перенесённые из слотов), и после
        // неё — читаются и пишутся одинаково.
        let first = rt.names.intern("Поле0");
        let last = rt
            .names
            .intern(&format!("Поле{}", MAX_SHAPE_TRANSITIONS + 4));
        assert_eq!(s.get_field(first).unwrap(), num("0"));
        assert_eq!(
            s.get_field(last).unwrap(),
            num(&(MAX_SHAPE_TRANSITIONS + 4).to_string())
        );

        s.set_field(first, num("777")).unwrap();
        s.set_field(last, num("888")).unwrap();
        assert_eq!(s.get_field(first).unwrap(), num("777"));
        assert_eq!(s.get_field(last).unwrap(), num("888"));

        let missing = rt.names.intern("НетТакогоПоля");
        assert!(matches!(
            s.get_field(missing).unwrap_err(),
            RtError::UnknownField(_)
        ));
        assert!(matches!(
            s.set_field(missing, num("1")).unwrap_err(),
            RtError::UnknownField(_)
        ));
    }

    #[test]
    fn dictionary_structure_delete_keeps_order_of_the_rest() {
        let total = MAX_SHAPE_TRANSITIONS + 3;
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, total);
        assert!(is_dictionary(&s));

        let victim = rt.names.intern("Поле1");
        s.structure_delete(victim, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);

        // Ожидаемый порядок — исходный без удалённого: `order` теряет ровно
        // один элемент, остальные не переставляются.
        let expected: Vec<String> = (0..total)
            .filter(|i| *i != 1)
            .map(|i| format!("Поле{i}"))
            .collect();
        let actual: Vec<String> = (0..expected.len())
            .map(
                |i| match s.get_index(&num(&i.to_string()), &rt.names).unwrap() {
                    BslValue::Object(o) => match &*o {
                        BslObject::KeyValuePair(k, _) => k.to_string(),
                        other => panic!("ожидался КлючИЗначение, получено {other:?}"),
                    },
                    other => panic!("ожидался объект, получено {other:?}"),
                },
            )
            .collect();
        assert_eq!(actual, expected);

        // Удаление отсутствующего — no-op и в словарном режиме тоже.
        let missing = rt.names.intern("НетТакогоПоля");
        s.structure_delete(missing, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);
    }

    #[test]
    fn shape_table_stops_growing_after_degradation() {
        // Суть всей задачи: без порога здесь было бы 10_000 бессмертных
        // форм со списками имён нарастающей длины — квадратичная память.
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, 10_000);

        assert!(is_dictionary(&s));
        assert_eq!(s.collection_len().unwrap(), 10_000);
        // Пустая форма + по одной на каждый разрешённый переход, и ни одной
        // сверх того.
        assert_eq!(rt.shapes.len(), MAX_SHAPE_TRANSITIONS as usize + 1);
    }

    #[test]
    fn dictionary_structure_does_not_poison_inline_cache_for_shaped_objects() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // `Поле0` — первое имя и у `shaped`, и у сгенерированной серии, так
        // что оба объекта проходят через одну и ту же форму `[Поле0]`.
        let x = rt.names.intern("Поле0");

        let shaped = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        shaped
            .structure_insert(x, num("1"), &mut rt.shapes)
            .unwrap();

        let dict = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        insert_generated_fields(&dict, &mut rt, MAX_SHAPE_TRANSITIONS + 2);
        assert!(is_dictionary(&dict));
        assert!(!is_dictionary(&shaped));
        assert_eq!(dict.get_field(x).unwrap(), num("0"));

        // Один и тот же сайт вызова (одна ячейка кэша) видит оба объекта.
        let cache: std::cell::RefCell<Option<(Rc<Shape>, u32)>> = std::cell::RefCell::new(None);

        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("1"));
        let filled = cache.borrow().clone();
        assert!(filled.is_some(), "шейповый объект обязан заполнить кэш");

        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("0"));
        let after_dict = cache.borrow().clone();
        match (&filled, &after_dict) {
            (Some((a, ai)), Some((b, bi))) => {
                assert!(Rc::ptr_eq(a, b), "словарный объект затёр форму в кэше");
                assert_eq!(ai, bi, "словарный объект затёр слот в кэше");
            }
            _ => panic!("словарный объект обнулил ячейку кэша"),
        }

        // И быстрый путь для шейпового объекта по-прежнему работает.
        shaped.set_field_cached(x, num("42"), &cache).unwrap();
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
        // Запись через тот же сайт в словарный объект тоже не портит кэш.
        dict.set_field_cached(x, num("43"), &cache).unwrap();
        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("43"));
        let after_dict_set = cache.borrow().clone();
        match (&filled, &after_dict_set) {
            (Some((a, _)), Some((b, _))) => assert!(Rc::ptr_eq(a, b)),
            _ => panic!("словарный объект обнулил ячейку кэша при записи"),
        }
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
    }
}
