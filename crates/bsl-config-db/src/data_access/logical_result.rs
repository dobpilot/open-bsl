//! Сборка логического результата по окончательной проекции компилятора SDK.

use open_bsl::{RtError, RtResult, Value};
use open_sdbl::query::{LogicalResultColumn, LogicalResultValue, TypeValue};
use open_sdbl_db::Cell;

pub(super) fn materialize_row(
    row: Vec<Cell>,
    projection: &[LogicalResultColumn],
    convert: &mut dyn FnMut(usize, Cell) -> RtResult<Value>,
) -> RtResult<Vec<Value>> {
    for logical in projection {
        match &logical.value {
            LogicalResultValue::PointInTime { date, reference }
                if *date < row.len() && *reference < row.len() && date != reference => {}
            LogicalResultValue::Scalar { column } if *column < row.len() => {}
            LogicalResultValue::Composite {
                discriminator,
                members,
            } if *discriminator < row.len() && !members.is_empty() => {
                for (index, member) in members.iter().enumerate() {
                    if member.column.is_some_and(|column| column >= row.len())
                        || members[..index]
                            .iter()
                            .any(|prior| prior.type_tag == member.type_tag)
                    {
                        return Err(error(
                            "неверный индекс или неоднозначный тег логической проекции",
                        ));
                    }
                }
            }
            _ => {
                return Err(error(
                    "неполный или неподдержанный рецепт логического значения",
                ));
            }
        }
    }
    let mut cells = row.into_iter().map(Some).collect::<Vec<_>>();
    let mut take = |index: usize| {
        cells
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| error("индекс логической ячейки неверен или используется повторно"))
    };
    projection
        .iter()
        .map(|logical| match &logical.value {
            LogicalResultValue::PointInTime { date, reference } => {
                let Value::Date(date_value) = convert(*date, take(*date)?)? else {
                    return Err(error("дата логического момента времени имеет неверный тип"));
                };
                let reference = convert(*reference, take(*reference)?)?;
                open_bsl::new_point_in_time(date_value, reference)
            }
            LogicalResultValue::Scalar { column } => convert(*column, take(*column)?),
            LogicalResultValue::Composite {
                discriminator,
                members,
            } => {
                let tag = match take(*discriminator)? {
                    Cell::Bytes(bytes) if bytes.len() == 1 => bytes[0],
                    // Внешнее соединение SQL сохраняет прежнее преобразование NULL.
                    Cell::Null => return Ok(Value::Null),
                    _ => return Err(error("признак составного типа не является одним байтом")),
                };
                let mut active = members.iter().filter(|member| member.type_tag == tag);
                let member = active
                    .next()
                    .ok_or_else(|| error("неизвестный признак составного типа"))?;
                if active.next().is_some() {
                    return Err(error(
                        "признак составного типа задаёт несколько активных членов",
                    ));
                }
                match member.column {
                    Some(column) => convert(column, take(column)?),
                    None if tag == TypeValue::Null.tag() => Ok(Value::Null),
                    None if tag == TypeValue::Undefined.tag() => Ok(Value::Undefined),
                    None => Err(error("активный член составного типа не содержит ячейки")),
                }
            }
            _ => Err(error("неподдержанный рецепт логического значения")),
        })
        .collect()
}

fn error(message: &str) -> RtError {
    RtError::DynamicError(message.into())
}

#[cfg(test)]
#[path = "logical_result_tests.rs"]
mod tests;
