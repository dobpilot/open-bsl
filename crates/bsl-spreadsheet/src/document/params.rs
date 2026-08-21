//! «Параметры» макета: имена, заданные ячейками.

use super::*;

// --- Параметры макета -----------------------------------------------------

/// Достать `Rc` данных документа из `SpreadDocParams`.
pub(crate) fn param_data(obj: &BslValue) -> RtResult<Rc<RefCell<SpreadDocData>>> {
    match obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SpreadParamsObject>())
    {
        Some(params) => Ok(params.data.clone()),
        None => Err(bad("Параметры: не объект параметров макета")),
    }
}

/// Чтение `Область.Параметры.Имя`. Неизвестное имя — `Неопределено`,
/// как у структуры: `ЗаполнитьЗначенияСвойств` различает «свойство есть»
/// и «свойства нет» по `has_property`, а не по значению.
pub fn get_param(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let doc = param_data(obj)?;
    let d = doc.borrow();
    let upper = name.to_uppercase();
    Ok(d.params
        .iter()
        .find(|(k, _)| **k == upper)
        .map(|(_, v)| v.clone())
        .unwrap_or(BslValue::Undefined))
}

/// `Область.Параметры.Имя = Значение` — запись в карту параметров.
pub fn set_param(obj: &BslValue, name: &str, val: BslValue) -> RtResult<()> {
    let doc = param_data(obj)?;
    let mut d = doc.borrow_mut();
    let upper = name.to_uppercase();
    if let Some(slot) = d.params.iter_mut().find(|(k, _)| **k == upper) {
        *slot.1 = val;
    } else {
        d.params.insert(upper, val);
    }
    Ok(())
}

/// Все имена параметров макета в документе — имена ячеек с непустым
/// `CellData::parameter`. Нужны `fill.rs`, чтобы различать «у приёмника
/// такое свойство есть» и «нет».
pub fn param_names(doc: &SpreadDocData) -> Vec<String> {
    let mut out = Vec::new();
    for row in doc.rows.values() {
        for cell in row.cells.values() {
            if !cell.parameter.is_empty()
                && !out.iter().any(|n: &String| folded_eq(n, &cell.parameter))
            {
                out.push(cell.parameter.clone());
            }
        }
    }
    out
}

/// Есть ли у документа параметр с таким именем.
pub fn has_param(doc: &SpreadDocData, name: &str) -> bool {
    let upper = name.to_uppercase();
    doc.rows.values().any(|row| {
        row.cells
            .values()
            .any(|cell| cell.parameter.to_uppercase() == upper)
    })
}

/// Применить значения параметров к ячейкам: для каждой ячейки с непустым
/// `CellData::parameter` найти значение в `params` по имени и положить
/// его представление в `CellData::text`. Вызывается из ВМ перед `Вывести`,
/// потому что форматирование значения живёт в `bsl-format`.
pub fn apply_params(obj: &BslValue, params: &[(String, String)]) -> RtResult<()> {
    if params.is_empty() {
        return Ok(());
    }
    // Имена параметров в верхнем регистре — сравнение регистронезависимо.
    let upper_params: Vec<(String, &str)> = params
        .iter()
        .map(|(name, text)| (name.to_uppercase(), text.as_str()))
        .collect();
    let doc = data(obj).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let mut d = doc.borrow_mut();
    for row in d.rows.values_mut() {
        for cell in row.cells.values_mut() {
            if cell.parameter.is_empty() {
                continue;
            }
            let param_upper = cell.parameter.to_uppercase();
            if let Some((_, text)) = upper_params.iter().find(|(name, _)| *name == param_upper) {
                cell.text = text.to_string();
            }
        }
    }
    Ok(())
}

/// Снять значения параметров из карты документа вместе со строкой формата
/// каждой параметрической ячейки: пары `(имя, значение, формат)`. Формат
/// берётся из `CellData::format_spec` — строка BSL вроде «ЧДЦ=2».
/// Вызывается из ВМ для форматирования перед `apply_params`.
pub fn take_params(obj: &BslValue) -> RtResult<Vec<(String, BslValue, Option<String>)>> {
    let doc = data(obj).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let mut d = doc.borrow_mut();
    let mut out = Vec::new();
    // Имена параметров и форматы ячеек снимаются в один проход: для каждой
    // ячейки с непустым `parameter` запоминаем имя, формат и значение из
    // карты параметров.
    let mut seen = std::collections::HashSet::new();
    for row in d.rows.values() {
        for cell in row.cells.values() {
            if cell.parameter.is_empty() {
                continue;
            }
            let upper = cell.parameter.to_uppercase();
            if !seen.insert(upper.clone()) {
                continue;
            }
            let value = d
                .params
                .iter()
                .find(|(k, _)| **k == upper)
                .map(|(_, v)| v.clone())
                .unwrap_or(BslValue::Undefined);
            out.push((cell.parameter.clone(), value, cell.format_spec.clone()));
        }
    }
    d.params.clear();
    Ok(out)
}
