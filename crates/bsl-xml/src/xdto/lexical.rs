//! Лексические формы значений простых типов.

use super::*;

// --- лексические формы ---------------------------------------------------

/// Значение BSL из лексической формы по типу модели, БЕЗ проверки фасетов.
///
/// Так разбираются значения самих фасетов и то, что читает
/// `СериализаторXDTO` (измерено: его чтение фасеты не проверяет), — то есть
/// места, где проверять нечего или не по чему. Всё, что приходит от
/// программы, из документа или из умолчания схемы, идёт через проверяющий
/// разбор `value_from_lexical_checked`.
///
/// # Errors
///
/// [`RtError::Xdto`], если тип — объектный, если лексическая форма не
/// разбирается в его отображении либо если ни один член объединения её не
/// принял.
pub fn value_from_lexical(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
) -> RtResult<BslValue> {
    // Глубины хватает на любую честную схему: цепочка «производный тип ->
    // базовый» не длиннее числа типов, а список и объединение добавляют
    // по шагу на уровень вложенности. Ограничение здесь не оптимизация, а
    // страховка: `<xs:restriction base="t:A"/>` внутри самого `A` даёт
    // КОЛЬЦО, и без счётчика разбор ушёл бы в переполнение стека.
    value_from_lexical_at(model, type_index, lexical, model.types.len() + 8, false)
}

/// То же с ПРОВЕРКОЙ ПО ФАСЕТАМ — путь всех измеренных точек, где платформа
/// значение проверяет: `ФабрикаXDTO.Создать`, запись свойства, наполнение
/// списка и чтение документа с типом.
///
/// # Errors
///
/// То же, что у [`value_from_lexical`], плюс [`RtError::Xdto`] на любом
/// нарушенном фасете и на фасете образца (см. [`check_pattern`]).
pub(crate) fn value_from_lexical_checked(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
) -> RtResult<BslValue> {
    value_from_lexical_at(model, type_index, lexical, model.types.len() + 8, true)
}

/// Разбор лексической формы; при `checked` — заодно проверка по фасетам.
///
/// Фасеты НАСЛЕДУЮТСЯ, и цепочку обходит сам разбор: производный тип
/// спускается к базовому, а на обратном ходе каждый уровень проверяет свои
/// фасеты — те самые, что платформа показывает членом `Фасеты`. Отдельного
/// обхода требует только встроенная часть цепочки (`int` -> `long` ->
/// `integer` -> `decimal`): в неё разбор не спускается, лексическая форма
/// встроенного типа разбирается на месте.
pub(crate) fn value_from_lexical_at(
    model: &XdtoModel,
    type_index: usize,
    lexical: &str,
    depth: usize,
    checked: bool,
) -> RtResult<BslValue> {
    let data = model.type_at(type_index)?;
    let Some(depth) = depth.checked_sub(1) else {
        return Err(RtError::Xdto(format!(
            "цепочка типов вокруг «{}» замкнута сама на себя",
            data.name
        )));
    };
    let Some(shape) = data.shape.as_ref() else {
        return Err(RtError::Xdto(format!(
            "тип объекта «{}» не строится из лексической формы",
            data.name
        )));
    };
    let value = match shape {
        ValueShape::Builtin(bsl) => {
            let value = builtin_from_lexical(*bsl, lexical, model.zone())?;
            if checked {
                check_facet_chain(model, data.base, lexical, &value)?;
            }
            value
        }
        ValueShape::Atomic => {
            let base = data.base.ok_or_else(|| {
                RtError::Xdto(format!("у типа значения «{}» нет базового типа", data.name))
            })?;
            value_from_lexical_at(model, base, lexical, depth, checked)?
        }
        // Список: лексическая форма делится пробельными символами.
        // Платформа отдаёт `ФиксированныйМассив` (измерено), а здесь это
        // обычный `Массив` — своего неизменяемого вида в этой реализации
        // нет, и `vstr.rs` читает фиксированный массив тем же обычным
        // (терять данные хуже, чем терять неизменяемость).
        ValueShape::List(item) => {
            let item = item.ok_or_else(|| {
                RtError::Xdto(format!(
                    "у списочного типа «{}» нет типа элемента",
                    data.name
                ))
            })?;
            let mut items = Vec::new();
            for part in lexical.split_whitespace() {
                items.push(value_from_lexical_at(model, item, part, depth, checked)?);
            }
            BslValue::new_array(items)
        }
        // Объединение: первый член, который принял форму (измерено на
        // `union memberTypes="xs:int xs:string"`). При проверке «принял» —
        // это ещё и фасеты члена: у объединения `t:Col xs:int` лексическая
        // форма «5» разбирается СТРОКОЙ первым членом, но перечисление
        // `red|green` её отвергает, и платформа отдаёт число (измерено).
        ValueShape::Union(members) => {
            let mut found = None;
            for member in members {
                if let Ok(v) = value_from_lexical_at(model, *member, lexical, depth, checked) {
                    found = Some(v);
                    break;
                }
            }
            found.ok_or_else(|| {
                RtError::Xdto(format!(
                    "лексическую форму «{lexical}» не принял ни один член объединения «{}»",
                    data.name
                ))
            })?
        }
    };
    if checked {
        check_facets(model, type_index, lexical, &value)?;
    }
    Ok(value)
}
