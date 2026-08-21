//! Коллекции компонент и списки имён.

use super::*;

// --- коллекции -----------------------------------------------------------

/// Элемент коллекции по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номер за границей: платформа на
/// `Фасеты[9]` и `ОбъявленияЭлементов[9]` отвечает ошибкой, а не
/// `Неопределено` (измерено).
pub fn list_get(schema: &Rc<XsSchemaData>, kind: &XsListKind, i: usize) -> RtResult<BslValue> {
    let items = kind.items();
    match items.get(i) {
        Some(idx) => Ok(component_value(schema, *idx)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: items.len(),
        }),
    }
}

/// `Получить` у коллекции компонент.
///
/// У именованной коллекции аргументом может быть имя, пара (URI, имя),
/// `РасширенноеИмяXML` или число (тогда это номер) — измерены все четыре
/// формы; ненайденное имя даёт `Неопределено`, а номер за границей —
/// ошибку.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] при неподходящем числе аргументов и
/// [`RtError::IndexOutOfBounds`] при номере за границей.
pub fn list_lookup(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let Some(list) = obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SchemaListObject>())
    else {
        return Err(method_error("Получить", obj));
    };
    let (schema, kind) = (&list.schema, &list.kind);
    let named = matches!(kind, XsListKind::Named(_));
    match args {
        [BslValue::Number(n)] => {
            let i = n
                .to_i64_exact()
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(RtError::BadIndex)?;
            list_get(schema, kind, i)
        }
        [BslValue::Str(s)] if named => Ok(named_lookup(schema, kind, None, &s.to_string())),
        [name_arg @ BslValue::Object(_)] if named => match name_arg
            .object_ref()
            .and_then(|object| object.downcast_ref::<ExpandedNameObject>())
        {
            Some(expanded) => {
                let n = &expanded.name;
                Ok(named_lookup(schema, kind, Some(&n.uri), &n.local))
            }
            _ => Err(method_error("Получить", obj)),
        },
        [BslValue::Str(uri), BslValue::Str(local)] if named => Ok(named_lookup(
            schema,
            kind,
            Some(&uri.to_string()),
            &local.to_string(),
        )),
        _ => Err(method_error("Получить", obj)),
    }
}

pub(crate) fn named_lookup(
    schema: &Rc<XsSchemaData>,
    kind: &XsListKind,
    uri: Option<&str>,
    local: &str,
) -> BslValue {
    for idx in kind.items() {
        let node = schema.node(*idx);
        let uri_matches = match uri {
            Some(u) => u == node.ns,
            None => true,
        };
        if node.name == local && uri_matches {
            return component_value(schema, *idx);
        }
    }
    BslValue::Undefined
}

pub(crate) fn method_error(method: &'static str, obj: &BslValue) -> RtError {
    RtError::MethodNotApplicable {
        method,
        receiver: obj.type_name(),
    }
}

/// Элемент `СписокРасширенныхИменXML` по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] при номере за границей.
pub fn name_list_get(names: &[XName], i: usize) -> RtResult<BslValue> {
    match names.get(i) {
        Some(n) => Ok(name_value(n)),
        None => Err(RtError::IndexOutOfBounds {
            index: i as i64,
            len: names.len(),
        }),
    }
}
