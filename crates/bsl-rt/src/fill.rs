//! `ЗаполнитьЗначенияСвойств(Приемник, Источник[, СписокСвойств[,
//! ИсключаяСвойства]])` — перенос значений одноимённых свойств.
//!
//! Отдельным модулем, а не веткой в `builtin.rs`: единственная встроенная
//! ФУНКЦИЯ, которая ходит по набору полей чужого объекта, — и делает это
//! для двух разных носителей свойств (`Структура` и
//! `СтрокаТаблицыЗначений`), у которых имена хранятся по-разному
//! (`NameId` против строковых имён колонок).
//!
//! # Модель, ИЗМЕРЕННАЯ на 8.3.27
//!
//! Платформа не проверяет типы аргументов вообще: приёмником и источником
//! годится что угодно, включая `Массив` и `Соответствие` (замеры
//! `FILL.TYPES.ARRAY`, `FILL.TYPES.MAP`, `FILL.TYPES.MAP_TARGET` — все
//! приняты без ошибки). Работает она с ПОНЯТИЕМ СВОЙСТВА: у значения без
//! свойств нечего прочитать и некуда записать, поэтому такой вызов просто
//! ничего не делает. Ключи `Соответствие` свойствами НЕ становятся —
//! измерено, что одноимённый ключу элемент приёмника не заполняется.
//!
//! Дальше поведение зависит от того, задан ли `СписокСвойств`:
//!
//! * без списка переносится ПЕРЕСЕЧЕНИЕ имён, всё лишнее с обеих сторон
//!   молча пропускается (`FILL.TARGET.NEW_FIELDS`: лишнее свойство
//!   источника не заводит поля у приёмника-структуры);
//! * со списком каждое перечисленное имя ОБЯЗАНО быть и у источника, и у
//!   приёмника, иначе ошибка (`FILL.LIST.ONLY_IN_SOURCE`,
//!   `FILL.LIST.ONLY_IN_TARGET`, `FILL.LIST.UNKNOWN` — все три дали
//!   ошибку). Список — не фильтр, а требование.
//!
//! Эта пара правил выглядит непоследовательной, но она измерена, и логика
//! в ней есть: набор полей объекта пишет не человек, а список свойств —
//! человек, и опечатка в нём должна быть слышна.

use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

use crate::interner::NameInterner;
use crate::object::BslObject;
use crate::{BslString, BslValue, RtError, RtResult, ValueTableData};

/// Разрешённые индексы прямого переноса и, если список ошибочен, имя, на
/// котором обход с конца должен остановиться после записи корректного хвоста.
struct RowCopyPlan {
    cells: Vec<(usize, usize)>,
    rejected: Option<String>,
}

/// Последний план достаточно держать одним: прикладной код переносит строки
/// длинными сериями между одной парой таблиц. `Weak` не продлевает жизнь
/// таблиц, а ревизии схем защищают от `Добавить`/`Свернуть` между вызовами.
struct CachedRowCopyPlan {
    source: Weak<RefCell<ValueTableData>>,
    target: Weak<RefCell<ValueTableData>>,
    source_schema_revision: u64,
    target_schema_revision: u64,
    list: Option<BslString>,
    exclude: Option<BslString>,
    plan: Rc<RowCopyPlan>,
}

thread_local! {
    static LAST_ROW_COPY_PLAN: RefCell<Option<CachedRowCopyPlan>> = const { RefCell::new(None) };
}

/// Имя свойства из `СписокСвойств`/`ИсключаяСвойства`: как написано (для
/// сообщения об ошибке) и в верхнем регистре (для сравнения — имена
/// свойств регистронезависимы, как и всё остальное в BSL).
struct Name {
    written: String,
    /// Нормализованное написание нужно только общему пути. При точном
    /// совпадении колонок таблицы Unicode-преобразование не выполняется.
    upper: OnceCell<String>,
}

impl Name {
    fn upper(&self) -> &str {
        self.upper
            .get_or_init(|| self.written.to_uppercase())
            .as_str()
    }
}

/// Разбор списка имён: `Неопределено` — параметр не задан, строка — имена
/// через запятую.
///
/// # Errors
///
/// [`RtError::TypeError`], если параметр задан, но не строка (измерено:
/// `FILL.LIST.NOT_STRING` — платформа тоже отвергает).
fn parse_name_list(v: &BslValue, op: &'static str) -> RtResult<Option<Vec<Name>>> {
    match v {
        BslValue::Undefined => Ok(None),
        BslValue::Str(s) => Ok(Some(
            s.to_string()
                .split(',')
                .map(str::trim)
                // Пустое имя выбрасывается, поэтому `СписокСвойств = ""`
                // даёт ПУСТОЙ список, а не «список не задан»: не
                // переносится ничего. Измерено (`FILL.LIST.EMPTY`).
                .filter(|n| !n.is_empty())
                .map(|n| Name {
                    written: n.to_string(),
                    upper: OnceCell::new(),
                })
                .collect(),
        )),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Ключ списка без разбора имён. Клонирование `BslString` клонирует `Rc`,
/// поэтому попадание в кэш не создаёт ни UTF-8-строку, ни отдельные имена.
fn name_list_key(v: &BslValue, op: &'static str) -> RtResult<Option<BslString>> {
    match v {
        BslValue::Undefined => Ok(None),
        BslValue::Str(s) => Ok(Some(s.clone())),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// `ЗаполнитьЗначенияСвойств`. Возвращаемого значения у процедуры нет —
/// приёмник меняется на месте (`Структура` и `СтрокаТаблицыЗначений` —
/// ссылочные типы, `&BslValue` для мутации достаточно).
///
/// `names` — рантайм-интернер имён; из него берутся строковые написания
/// полей источника-структуры и в него же (только на чтение) резолвятся
/// имена полей приёмника.
///
/// # Errors
///
/// [`RtError::UnknownProperty`], если имя из `СписокСвойств` отсутствует у
/// источника или у приёмника; [`RtError::TypeError`], если список свойств
/// задан не строкой; [`RtError::RowInvalidated`], если строка таблицы уже
/// удалена.
pub fn fill_property_values(
    target: &BslValue,
    source: &BslValue,
    list: &BslValue,
    exclude: &BslValue,
    names: &NameInterner,
) -> RtResult<()> {
    if let Some(result) = fill_table_rows(target, source, list, exclude) {
        return result;
    }
    let only = parse_name_list(list, "ЗаполнитьЗначенияСвойств(СписокСвойств)")?;
    let except = parse_name_list(exclude, "ЗаполнитьЗначенияСвойств(ИсключаяСвойства)")?;
    // Свойства источника снимаются ЦЕЛИКОМ и заранее, а не читаются по
    // одному внутри цикла записи. Причина не в скорости: приёмник и
    // источник могут оказаться одним объектом, а `RefCell` структуры не
    // переживёт `borrow_mut` под удерживаемым `borrow`.
    let available = properties(source, names)?;

    let Some(only) = only else {
        // Без списка — пересечение имён, и `ИсключаяСвойства` вычитается
        // из него. Всё, чего нет у другой стороны, пропускается молча.
        for (name, value) in available {
            let upper = name.to_uppercase();
            if let Some(except) = &except {
                if except.iter().any(|n| n.upper() == upper) {
                    continue;
                }
            }
            write_property(target, &name, value, names)?;
        }
        return Ok(());
    };

    // Список задан — `ИсключаяСвойства` при этом не анализируется вовсе
    // (измерено, `FILL.EXCLUDE.WITH_LIST`).
    //
    // Обход идёт С КОНЦА СПИСКА, и это не прихоть: платформа не проверяет
    // список заранее, а пишет свойство за свойством и падает на первом
    // неразрешимом имени — двигаясь от последнего к первому. Три замера
    // сходятся только на этом порядке: `FILL.LIST.REJECT_LAST` (плохое имя
    // последнее -> не записано НИЧЕГО), `FILL.LIST.REJECT_FIRST` (первое ->
    // записано ВСЁ остальное), `FILL.LIST.PARTIAL_ON_ERROR` (в середине ->
    // записан только хвост после него). На успешном вызове порядок записи
    // не наблюдаем — каждое имя пишется ровно раз, — поэтому единственное,
    // что этим воспроизводится, и есть состояние приёмника после
    // пойманного исключения.
    for wanted in only.iter().rev() {
        let Some((name, value)) = available
            .iter()
            .find(|(name, _)| name.to_uppercase() == wanted.upper())
        else {
            return Err(RtError::UnknownProperty(wanted.written.clone()));
        };
        if !has_property(target, name, names) {
            return Err(RtError::UnknownProperty(wanted.written.clone()));
        }
        write_property(target, name, value.clone(), names)?;
    }
    Ok(())
}

/// Прямой перенос между двумя живыми строками таблиц.
///
/// План заранее разрешает соответствие индексов колонок. Между разными
/// таблицами ячейки копируются напрямую под одним заимствованием источника и
/// приёмника; для одной таблицы значения сначала снимаются во временный
/// вектор. Это сохраняет корректность копирования строки в саму себя и
/// убирает из горячего цикла создание пар `(имя, значение)`, повторный поиск
/// имён и вызов `set_field_by_name` для каждой ячейки.
///
/// `None` означает, что общий путь обязан обработать вызов: один из объектов
/// не строка таблицы либо строка уже инвалидирована. Последний случай важен
/// для точного порядка ошибок и частичных записей измеренной семантики.
fn fill_table_rows(
    target: &BslValue,
    source: &BslValue,
    list: &BslValue,
    exclude: &BslValue,
) -> Option<RtResult<()>> {
    let BslValue::Object(target_object) = target else {
        return None;
    };
    let BslObject::TableRow(target_data, target_row_id) = &**target_object else {
        return None;
    };
    let BslValue::Object(source_object) = source else {
        return None;
    };
    let BslObject::TableRow(source_data, source_row_id) = &**source_object else {
        return None;
    };

    // Типы списков проверяются ДО валидности строк — так же, как на общем
    // пути. Для инвалидированной строки затем отступаем к нему: там точный
    // порядок `UnknownProperty`/`RowInvalidated` зависит от наличия колонок.
    let list_key = match name_list_key(list, "ЗаполнитьЗначенияСвойств(СписокСвойств)")
    {
        Ok(key) => key,
        Err(error) => return Some(Err(error)),
    };
    let exclude_key = match name_list_key(exclude, "ЗаполнитьЗначенияСвойств(ИсключаяСвойства)")
    {
        Ok(key) => key,
        Err(error) => return Some(Err(error)),
    };

    // Два неизменяемых `borrow` допустимы и для одной таблицы. Перед её
    // `borrow_mut` значения снимаются отдельно, поэтому строка может
    // копироваться в себя.
    let source_data_ref = source_data.borrow();
    let target_data_ref = target_data.borrow();
    let source_pos = source_data_ref.pos_of(*source_row_id)?;
    let target_pos = target_data_ref.pos_of(*target_row_id)?;
    let source_schema_revision = source_data_ref.schema_revision();
    let target_schema_revision = target_data_ref.schema_revision();
    // При заданном списке исключения семантически игнорируются. Тип выше всё
    // равно проверен, а из ключа лишнее значение можно убрать.
    let cached_exclude = if list_key.is_none() {
        exclude_key.as_ref()
    } else {
        None
    };

    let cached_plan = LAST_ROW_COPY_PLAN.with(|slot| {
        let slot = slot.borrow();
        let cached = slot.as_ref()?;
        let source_matches = cached
            .source
            .upgrade()
            .is_some_and(|data| Rc::ptr_eq(&data, source_data));
        let target_matches = cached
            .target
            .upgrade()
            .is_some_and(|data| Rc::ptr_eq(&data, target_data));
        (source_matches
            && target_matches
            && cached.source_schema_revision == source_schema_revision
            && cached.target_schema_revision == target_schema_revision
            && cached.list.as_ref() == list_key.as_ref()
            && cached.exclude.as_ref() == cached_exclude)
            .then(|| cached.plan.clone())
    });

    let plan = if let Some(plan) = cached_plan {
        plan
    } else {
        let only = match parse_name_list(list, "ЗаполнитьЗначенияСвойств(СписокСвойств)")
        {
            Ok(names) => names,
            Err(error) => return Some(Err(error)),
        };
        let except = match parse_name_list(exclude, "ЗаполнитьЗначенияСвойств(ИсключаяСвойства)")
        {
            Ok(names) => names,
            Err(error) => return Some(Err(error)),
        };
        let plan = Rc::new(resolve_row_copy_plan(
            &source_data_ref,
            &target_data_ref,
            only.as_deref(),
            except.as_deref(),
        ));
        LAST_ROW_COPY_PLAN.with(|slot| {
            *slot.borrow_mut() = Some(CachedRowCopyPlan {
                source: Rc::downgrade(source_data),
                target: Rc::downgrade(target_data),
                source_schema_revision,
                target_schema_revision,
                list: list_key.clone(),
                exclude: cached_exclude.cloned(),
                plan: plan.clone(),
            });
        });
        plan
    };

    if Rc::ptr_eq(source_data, target_data) {
        let values: Vec<BslValue> = plan
            .cells
            .iter()
            .map(|(source_col, _)| source_data_ref.columns[*source_col][source_pos].clone())
            .collect();
        drop(target_data_ref);
        drop(source_data_ref);
        let mut target_data_ref = target_data.borrow_mut();
        for ((_, target_col), value) in plan.cells.iter().zip(values) {
            // То же приведение к типу колонки, что у прямого присваивания
            // (измерено, `ADJ.FILL.STR`).
            let value = target_data_ref.adjust_to_column_type(*target_col, value);
            target_data_ref.columns[*target_col][target_pos] = value;
        }
    } else {
        drop(target_data_ref);
        let mut target_data_ref = target_data.borrow_mut();
        for (source_col, target_col) in &plan.cells {
            let value = target_data_ref.adjust_to_column_type(
                *target_col,
                source_data_ref.columns[*source_col][source_pos].clone(),
            );
            target_data_ref.columns[*target_col][target_pos] = value;
        }
    }

    Some(match &plan.rejected {
        Some(name) => Err(RtError::UnknownProperty(name.clone())),
        None => Ok(()),
    })
}

fn resolve_row_copy_plan(
    source: &ValueTableData,
    target: &ValueTableData,
    only: Option<&[Name]>,
    except: Option<&[Name]>,
) -> RowCopyPlan {
    let mut cells = Vec::new();
    let mut rejected = None;
    if let Some(only) = only {
        // Порядок с конца измерен на платформе: до ошибочного имени должны
        // успеть записаться корректные имена, стоящие после него.
        for wanted in only.iter().rev() {
            let source_col = source
                .column_names
                .iter()
                .position(|name| name == &wanted.written)
                .or_else(|| {
                    source
                        .column_names
                        .iter()
                        .position(|name| name.to_uppercase() == wanted.upper())
                });
            let Some(source_col) = source_col else {
                rejected = Some(wanted.written.clone());
                break;
            };
            let source_name = &source.column_names[source_col];
            let Some(target_col) = target
                .column_names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(source_name))
            else {
                rejected = Some(wanted.written.clone());
                break;
            };
            cells.push((source_col, target_col));
        }
    } else {
        for (source_col, source_name) in source.column_names.iter().enumerate() {
            if except.is_some_and(|except| {
                except.iter().any(|name| {
                    source_name == &name.written || source_name.to_uppercase() == name.upper()
                })
            }) {
                continue;
            }
            if let Some(target_col) = target
                .column_names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(source_name))
            {
                cells.push((source_col, target_col));
            }
        }
    }
    RowCopyPlan { cells, rejected }
}

/// Пары «имя свойства — значение» в порядке объявления полей (для
/// структуры — порядок вставки, для строки таблицы — порядок колонок).
///
/// У значения, которое свойств не имеет, список ПУСТ, а не ошибочен:
/// измерено на `Массив` и `Соответствие`, и на прочие типы правило
/// распространяется тем же рассуждением — «нечего перечислять» не зависит
/// от того, какой именно это тип без свойств.
fn properties(source: &BslValue, names: &NameInterner) -> RtResult<Vec<(String, BslValue)>> {
    let BslValue::Object(o) = source else {
        return Ok(Vec::new());
    };
    match &**o {
        BslObject::Structure(s) => {
            let s = s.borrow();
            let mut out = Vec::with_capacity(s.len());
            for i in 0..s.len() {
                let Some((field, value)) = s.entry_at(i) else {
                    break;
                };
                // Имя без написания в интернере невозможно у живой
                // структуры (поле заводится только через интернер), но
                // `NameInterner` — публичный тип, и звать это паникой
                // из-за чужой сборки таблицы имён незачем: поле без
                // написания просто не переносится.
                if let Some(name) = names.name(field) {
                    out.push((name.to_string(), value));
                }
            }
            Ok(out)
        }
        BslObject::TableRow(data, row_id) => {
            let data = data.borrow();
            let mut out = Vec::with_capacity(data.column_names.len());
            for (col, name) in data.column_names.iter().enumerate() {
                let value = data.get_cell(*row_id, col).ok_or(RtError::RowInvalidated)?;
                out.push((name.clone(), value));
            }
            Ok(out)
        }
        _ => Ok(Vec::new()),
    }
}

/// Есть ли у приёмника свойство с таким именем. Нужно отдельно от записи
/// ровно одному месту — проверке `СписокСвойств` до начала переноса.
fn has_property(target: &BslValue, name: &str, names: &NameInterner) -> bool {
    let BslValue::Object(o) = target else {
        return false;
    };
    match &**o {
        // Имя, которого нет в интернере, не может быть полем ни одной
        // живой структуры — значит, у приёмника его точно нет.
        BslObject::Structure(s) => match names.lookup(name) {
            Some(field) => s.borrow().get(field).is_some(),
            None => false,
        },
        BslObject::TableRow(data, _) => data.borrow().column_index(name).is_some(),
        _ => false,
    }
}

/// Запись одного свойства. Нет такого свойства у приёмника — не ошибка, а
/// пропуск: набор полей приёмника не растёт (измерено,
/// `FILL.TARGET.NEW_FIELDS`), а у приёмника-строки таблицы иначе и не
/// вышло бы — колонки на лету не заводятся. Случай, когда пропуск всё-таки
/// должен быть ошибкой, отсекается раньше, проверкой `СписокСвойств`.
fn write_property(
    target: &BslValue,
    name: &str,
    value: BslValue,
    names: &NameInterner,
) -> RtResult<()> {
    let BslValue::Object(o) = target else {
        return Ok(());
    };
    match &**o {
        BslObject::Structure(_) => {
            let Some(field) = names.lookup(name) else {
                return Ok(());
            };
            match target.set_field(field, value) {
                Err(RtError::UnknownField(_)) => Ok(()),
                other => other,
            }
        }
        BslObject::TableRow(..) => match target.set_field_by_name(name, value) {
            Err(RtError::UnknownColumn(_)) => Ok(()),
            other => other,
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_shapes::RuntimeShapes;
    use crate::{BslString, ValueTableData};
    use bsl_number::BslNumber;
    use std::rc::Rc;

    fn rt() -> RuntimeShapes {
        RuntimeShapes::seeded(Vec::new(), Vec::new())
    }

    fn num(n: i64) -> BslValue {
        BslValue::Number(BslNumber::from_i64(n))
    }

    fn str_val(s: &str) -> BslValue {
        BslValue::Str(BslString::from_str(s))
    }

    /// Структура с перечисленными полями — тем же путём, каким её строит
    /// `Вставить` в рантайме.
    fn structure(fields: &[(&str, BslValue)], rt: &mut RuntimeShapes) -> BslValue {
        let empty = rt.shapes.empty();
        let s = BslValue::new_structure(empty, Vec::new());
        for (name, value) in fields {
            let id = rt.names.intern(name);
            s.structure_insert(id, value.clone(), &mut rt.shapes)
                .unwrap();
        }
        s
    }

    fn field(s: &BslValue, name: &str, rt: &RuntimeShapes) -> BslValue {
        s.get_field(rt.names.lookup(name).expect("имя не интернировано"))
            .unwrap()
    }

    /// Единственная строка таблицы с готовыми колонками.
    fn table_row(columns: &[&str]) -> BslValue {
        let data = ValueTableData::new();
        let row_id = {
            let mut d = data.borrow_mut();
            for c in columns {
                d.add_column(c);
            }
            d.add_row()
        };
        BslValue::Object(Rc::new(BslObject::TableRow(data, row_id)))
    }

    fn add_table_column(row: &BslValue, name: &str) {
        let BslValue::Object(object) = row else {
            panic!("ожидалась строка таблицы");
        };
        let BslObject::TableRow(data, _) = &**object else {
            panic!("ожидалась строка таблицы");
        };
        data.borrow_mut().add_column(name);
    }

    /// Вызов без обоих списков.
    fn fill(target: &BslValue, source: &BslValue, rt: &RuntimeShapes) -> RtResult<()> {
        fill_property_values(
            target,
            source,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &rt.names,
        )
    }

    #[test]
    fn only_shared_names_are_copied() {
        let mut rt = rt();
        let target = structure(
            &[("Цена", BslValue::Undefined), ("Количество", num(7))],
            &mut rt,
        );
        let source = structure(&[("Цена", num(100)), ("Скидка", num(5))], &mut rt);

        fill(&target, &source, &rt).unwrap();

        assert_eq!(field(&target, "Цена", &rt), num(100));
        // Не в источнике — не тронуто.
        assert_eq!(field(&target, "Количество", &rt), num(7));
        // Лишнее свойство источника не завело нового поля.
        assert_eq!(target.collection_len().unwrap(), 2);
    }

    #[test]
    fn property_names_match_case_insensitively() {
        let mut rt = rt();
        let target = structure(&[("ЦЕНА", BslValue::Undefined)], &mut rt);
        let source = structure(&[("цена", num(42))], &mut rt);
        fill(&target, &source, &rt).unwrap();
        assert_eq!(field(&target, "Цена", &rt), num(42));
    }

    #[test]
    fn a_property_list_hides_the_exclusion_list() {
        let mut rt = rt();
        let target = structure(
            &[("А", BslValue::Undefined), ("Б", BslValue::Undefined)],
            &mut rt,
        );
        let source = structure(&[("А", num(1)), ("Б", num(2))], &mut rt);

        fill_property_values(&target, &source, &str_val("А, Б"), &str_val("Б"), &rt.names).unwrap();
        assert_eq!(field(&target, "А", &rt), num(1));
        assert_eq!(field(&target, "Б", &rt), num(2));
    }

    #[test]
    fn exclusion_list_alone_skips_named_properties() {
        let mut rt = rt();
        let target = structure(&[("А", num(0)), ("Б", num(0))], &mut rt);
        let source = structure(&[("А", num(1)), ("Б", num(2))], &mut rt);
        fill_property_values(
            &target,
            &source,
            &BslValue::Undefined,
            &str_val("б"),
            &rt.names,
        )
        .unwrap();
        assert_eq!(field(&target, "А", &rt), num(1));
        assert_eq!(field(&target, "Б", &rt), num(0));
    }

    #[test]
    fn empty_list_transfers_nothing() {
        let mut rt = rt();
        let target = structure(&[("А", num(0))], &mut rt);
        let source = structure(&[("А", num(1))], &mut rt);
        fill_property_values(
            &target,
            &source,
            &str_val(""),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap();
        assert_eq!(field(&target, "А", &rt), num(0));
    }

    /// Список — требование, а не фильтр: имя обязано быть у ОБЕИХ сторон.
    /// Все три случая измерены на платформе.
    #[test]
    fn a_name_in_the_list_must_exist_on_both_sides() {
        let mut rt = rt();
        let target = structure(&[("А", num(0)), ("Б", num(0))], &mut rt);
        let source = structure(&[("А", num(1)), ("В", num(3))], &mut rt);

        // Нет ни у кого.
        let e = fill_property_values(
            &target,
            &source,
            &str_val("ТакогоНет"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(e, RtError::UnknownProperty("ТакогоНет".to_string()));

        // Есть у источника, нет у приёмника.
        let e = fill_property_values(
            &target,
            &source,
            &str_val("В"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(e, RtError::UnknownProperty("В".to_string()));

        // Есть у приёмника, нет у источника.
        let e = fill_property_values(
            &target,
            &source,
            &str_val("Б"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(e, RtError::UnknownProperty("Б".to_string()));

        // Ни одна из трёх неудач не тронула приёмник.
        assert_eq!(field(&target, "А", &rt), num(0));
    }

    /// Состояние приёмника после отказа по списку. Все три случая измерены
    /// и вместе доказывают, что список обходится с конца.
    #[test]
    fn a_rejected_list_leaves_the_suffix_after_the_bad_name_written() {
        let mut rt = rt();
        let source = structure(&[("А", num(1)), ("Б", num(2))], &mut rt);

        // Плохое имя последнее — обход начинается с него, не записано ничего.
        let target = structure(&[("А", num(9)), ("Б", num(9))], &mut rt);
        fill_property_values(
            &target,
            &source,
            &str_val("А,Б,ТакогоНет"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(
            (field(&target, "А", &rt), field(&target, "Б", &rt)),
            (num(9), num(9))
        );

        // Плохое имя первое — до него успевает пройти всё остальное.
        let target = structure(&[("А", num(9)), ("Б", num(9))], &mut rt);
        fill_property_values(
            &target,
            &source,
            &str_val("ТакогоНет,А,Б"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(
            (field(&target, "А", &rt), field(&target, "Б", &rt)),
            (num(1), num(2))
        );

        // В середине — записан только хвост после него.
        let target = structure(&[("А", num(9)), ("Б", num(9))], &mut rt);
        fill_property_values(
            &target,
            &source,
            &str_val("А,ТакогоНет,Б"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();
        assert_eq!(
            (field(&target, "А", &rt), field(&target, "Б", &rt)),
            (num(9), num(2))
        );
    }

    #[test]
    fn structure_and_table_row_fill_each_other() {
        let mut rt = rt();
        let row = table_row(&["Цена", "Количество"]);
        let source = structure(&[("Цена", num(10)), ("Лишнее", num(0))], &mut rt);
        fill(&row, &source, &rt).unwrap();
        assert_eq!(row.get_field_by_name("Цена").unwrap(), num(10));

        // Обратно: строка таблицы как источник.
        let back = structure(&[("Цена", BslValue::Undefined)], &mut rt);
        fill(&back, &row, &rt).unwrap();
        assert_eq!(field(&back, "Цена", &rt), num(10));
    }

    #[test]
    fn table_rows_copy_by_column_name_instead_of_position() {
        let rt = rt();
        let source = table_row(&["А", "Б"]);
        source.set_field_by_name("А", num(1)).unwrap();
        source.set_field_by_name("Б", num(2)).unwrap();
        let target = table_row(&["Б", "А"]);
        target.set_field_by_name("А", num(9)).unwrap();
        target.set_field_by_name("Б", num(9)).unwrap();

        fill_property_values(
            &target,
            &source,
            &str_val("А, Б"),
            // При заданном списке исключение по измеренной семантике не
            // действует; fast path обязан сохранить это правило.
            &str_val("Б"),
            &rt.names,
        )
        .unwrap();

        assert_eq!(target.get_field_by_name("А").unwrap(), num(1));
        assert_eq!(target.get_field_by_name("Б").unwrap(), num(2));
    }

    #[test]
    fn table_rows_without_a_list_copy_intersection_and_honor_exclusion() {
        let rt = rt();
        let source = table_row(&["А", "Б", "Лишнее"]);
        source.set_field_by_name("А", num(1)).unwrap();
        source.set_field_by_name("Б", num(2)).unwrap();
        source.set_field_by_name("Лишнее", num(3)).unwrap();
        let target = table_row(&["Б", "А", "ТолькоПриемник"]);
        target.set_field_by_name("А", num(9)).unwrap();
        target.set_field_by_name("Б", num(9)).unwrap();
        target.set_field_by_name("ТолькоПриемник", num(9)).unwrap();

        fill_property_values(
            &target,
            &source,
            &BslValue::Undefined,
            &str_val("б"),
            &rt.names,
        )
        .unwrap();

        assert_eq!(target.get_field_by_name("А").unwrap(), num(1));
        assert_eq!(target.get_field_by_name("Б").unwrap(), num(9));
        assert_eq!(target.get_field_by_name("ТолькоПриемник").unwrap(), num(9));
    }

    #[test]
    fn table_row_fast_path_preserves_partial_write_before_list_error() {
        let rt = rt();
        let source = table_row(&["А", "Б"]);
        source.set_field_by_name("А", num(1)).unwrap();
        source.set_field_by_name("Б", num(2)).unwrap();
        let target = table_row(&["А", "Б"]);
        target.set_field_by_name("А", num(9)).unwrap();
        target.set_field_by_name("Б", num(9)).unwrap();

        let error = fill_property_values(
            &target,
            &source,
            &str_val("А, ТакогоНет, Б"),
            &BslValue::Undefined,
            &rt.names,
        )
        .unwrap_err();

        assert_eq!(error, RtError::UnknownProperty("ТакогоНет".to_string()));
        assert_eq!(target.get_field_by_name("А").unwrap(), num(9));
        assert_eq!(target.get_field_by_name("Б").unwrap(), num(2));
    }

    #[test]
    fn table_row_fast_path_can_copy_a_row_into_itself() {
        let rt = rt();
        let row = table_row(&["А", "Б"]);
        row.set_field_by_name("А", num(1)).unwrap();
        row.set_field_by_name("Б", num(2)).unwrap();

        fill(&row, &row, &rt).unwrap();

        assert_eq!(row.get_field_by_name("А").unwrap(), num(1));
        assert_eq!(row.get_field_by_name("Б").unwrap(), num(2));
    }

    #[test]
    fn table_row_copy_plan_is_reused_and_invalidated_by_schema_change() {
        let rt = rt();
        let source = table_row(&["А"]);
        let target = table_row(&["А"]);
        source.set_field_by_name("А", num(1)).unwrap();

        fill(&target, &source, &rt).unwrap();
        let first_plan = LAST_ROW_COPY_PLAN.with(|slot| {
            slot.borrow()
                .as_ref()
                .expect("план не помещён в кэш")
                .plan
                .clone()
        });

        source.set_field_by_name("А", num(2)).unwrap();
        fill(&target, &source, &rt).unwrap();
        let reused_plan = LAST_ROW_COPY_PLAN.with(|slot| {
            slot.borrow()
                .as_ref()
                .expect("план пропал из кэша")
                .plan
                .clone()
        });
        assert!(Rc::ptr_eq(&first_plan, &reused_plan));
        assert_eq!(target.get_field_by_name("А").unwrap(), num(2));

        add_table_column(&source, "Б");
        add_table_column(&target, "Б");
        source.set_field_by_name("Б", num(3)).unwrap();
        fill(&target, &source, &rt).unwrap();
        let rebuilt_plan = LAST_ROW_COPY_PLAN.with(|slot| {
            slot.borrow()
                .as_ref()
                .expect("план не перестроен")
                .plan
                .clone()
        });

        assert!(!Rc::ptr_eq(&reused_plan, &rebuilt_plan));
        assert_eq!(target.get_field_by_name("Б").unwrap(), num(3));
    }

    /// Приёмник и источник — один объект: `borrow_mut` под удерживаемым
    /// `borrow` уронил бы `RefCell`, поэтому свойства снимаются заранее.
    #[test]
    fn filling_an_object_from_itself_does_not_panic() {
        let mut rt = rt();
        let s = structure(&[("А", num(1))], &mut rt);
        fill(&s, &s, &rt).unwrap();
        assert_eq!(field(&s, "А", &rt), num(1));
    }

    /// ИЗМЕРЕНО: платформа принимает любой тип с обеих сторон. У значения
    /// без свойств просто нечего взять и некуда положить.
    #[test]
    fn values_without_properties_are_accepted_and_do_nothing() {
        let mut rt = rt();
        let s = structure(&[("А", num(1))], &mut rt);
        let array = BslValue::new_array(Vec::new());
        let map = BslValue::new_map();
        map.map_insert(str_val("А"), num(7)).unwrap();

        fill(&array, &s, &rt).unwrap();
        assert_eq!(array.collection_len().unwrap(), 0);

        fill(&map, &s, &rt).unwrap();
        assert_eq!(map.collection_len().unwrap(), 1, "ключей не прибавилось");

        // Источник без свойств оставляет приёмник как был — в частности,
        // одноимённый ключ соответствия свойством не считается.
        fill(&s, &array, &rt).unwrap();
        assert_eq!(field(&s, "А", &rt), num(1));
        fill(&s, &map, &rt).unwrap();
        assert_eq!(field(&s, "А", &rt), num(1));

        // И примитив тоже принимается.
        fill(&s, &num(5), &rt).unwrap();
        assert_eq!(field(&s, "А", &rt), num(1));
    }

    #[test]
    fn a_non_string_property_list_is_a_type_error() {
        let mut rt = rt();
        let s = structure(&[("А", num(1))], &mut rt);
        let e = fill_property_values(&s, &s, &num(1), &BslValue::Undefined, &rt.names).unwrap_err();
        assert!(matches!(
            e,
            RtError::TypeError {
                expected: "Строка",
                ..
            }
        ));
    }
}
