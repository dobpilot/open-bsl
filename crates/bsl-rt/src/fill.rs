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

use crate::interner::NameInterner;
use crate::object::BslObject;
use crate::{BslValue, RtError, RtResult};

/// Имя свойства из `СписокСвойств`/`ИсключаяСвойства`: как написано (для
/// сообщения об ошибке) и в верхнем регистре (для сравнения — имена
/// свойств регистронезависимы, как и всё остальное в BSL).
struct Name {
    written: String,
    upper: String,
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
                    upper: n.to_uppercase(),
                })
                .collect(),
        )),
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
                if except.iter().any(|n| n.upper == upper) {
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
            .find(|(name, _)| name.to_uppercase() == wanted.upper)
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
