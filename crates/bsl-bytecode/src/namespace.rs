//! Холодное связывание форм программы с пространством имён исполнения.

use crate::Program;
use bsl_rt::{RtError, RuntimeShapes, Shape};
use std::rc::Rc;

/// Строит формы исполнения, сохраняя локальные операнды исходной программы.
/// Имена методов остаются в таблице программы; глобальные номера нужны только
/// полям структур. Написание полей сохраняется при совпадении без учёта регистра.
///
/// # Errors
/// Возвращает ошибку для имени поля за пределами таблицы исходной программы.
pub fn bind_program_shapes(
    program: &Program,
    runtime: &mut RuntimeShapes,
) -> Result<Vec<Rc<Shape>>, RtError> {
    program
        .shapes
        .iter()
        .map(|shape| {
            let fields = shape
                .names
                .iter()
                .map(|id| {
                    let name = program
                        .names
                        .get(id.index())
                        .ok_or(RtError::InvalidBytecode(
                            "имя поля формы вне таблицы имён программы",
                        ))?;
                    Ok(runtime.names.intern(name))
                })
                .collect::<Result<Vec<_>, RtError>>()?;
            let spellings = shape
                .names
                .iter()
                .zip(&fields)
                .filter_map(|(old, new)| {
                    let spelling = &program.names[old.index()];
                    if runtime.names.name(*new) == Some(spelling.as_str()) {
                        None
                    } else {
                        Some((*new, spelling.clone()))
                    }
                })
                .collect();
            Ok(runtime.shapes.intern_spelled(&fields, spellings))
        })
        .collect()
}
