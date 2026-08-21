//! Набор схем.

use super::*;

// --- набор схем ----------------------------------------------------------

/// `НаборСхемXML.Добавить(СхемаXML)`.
///
/// # Errors
///
/// [`RtError::Xsd`], если аргумент не схема либо в наборе уже есть ДРУГАЯ
/// схема с тем же целевым пространством имён (измерено: платформа на этом
/// отказывает, а повторное добавление ТОЙ ЖЕ схемы молча пропускает).
pub fn schema_set_add(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let Some(set) = obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SchemaSetObject>())
    else {
        return Err(method_error("Добавить", obj));
    };
    let list = &set.schemas;
    let [arg] = args else {
        return Err(method_error("Добавить", obj));
    };
    let schema = match as_component(arg) {
        Some((s, 0)) => s,
        _ => return Err(RtError::Xsd("Добавить принимает СхемаXML".to_string())),
    };
    let mut list = list.borrow_mut();
    if list.iter().any(|s| Rc::ptr_eq(s, &schema)) {
        return Ok(BslValue::Undefined);
    }
    if list.iter().any(|s| s.target_ns() == schema.target_ns()) {
        return Err(RtError::Xsd(format!(
            "в наборе уже есть схема пространства имён «{}»",
            schema.target_ns()
        )));
    }
    list.push(schema);
    Ok(BslValue::Undefined)
}

/// Схема набора по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] при номере за границей — измерено, что
/// платформа отвечает именно ошибкой.
pub fn schema_set_get(obj: &BslValue, i: usize) -> RtResult<BslValue> {
    let Some(set) = obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SchemaSetObject>())
    else {
        return Err(method_error("Получить", obj));
    };
    let list = set.schemas.borrow();
    match list.get(i) {
        Some(s) => Ok(component_value(s, 0)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: list.len(),
        }),
    }
}
