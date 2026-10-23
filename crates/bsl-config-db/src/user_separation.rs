//! Перенос полного списка разделителей SDK в неизменяемый снимок пользователя.

use bsl_orm::Principal;
use open_bsl::{GraphLimits, RuntimeShapes, SerializedValueGraph, Value};

pub(super) fn attach(principal: Principal, bindings: Option<&[(String, String)]>) -> Principal {
    match bindings.and_then(snapshot) {
        Some(graph) => principal.with_data_separation(graph),
        None => principal,
    }
}

fn snapshot(bindings: &[(String, String)]) -> Option<SerializedValueGraph> {
    let mut shapes = RuntimeShapes::seeded(vec![], vec![], None);
    let mut entries = Vec::with_capacity(bindings.len());
    for (name, value) in bindings {
        let id = shapes.names.intern(name);
        // Имена структуры регистронезависимы. Нельзя потерять привязку при
        // преобразовании неоднозначного списка в словарь.
        if entries.iter().any(|(existing, _)| *existing == id) {
            return None;
        }
        entries.push((id, Value::Str(value.as_str().into())));
    }
    let structure = Value::new_dictionary_structure(entries);
    // При превышении бюджета снимок остаётся неизвестным: чтение свойства
    // выдаст явную ошибку, а не пустую структуру или частичные данные.
    SerializedValueGraph::capture(&[structure], &shapes, &GraphLimits::default()).ok()
}

#[cfg(test)]
#[path = "user_separation_tests.rs"]
mod tests;
