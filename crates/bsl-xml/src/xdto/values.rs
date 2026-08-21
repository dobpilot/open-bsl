//! Значения BSL внутри экземпляров XDTO.

use super::*;

// --- значения BSL --------------------------------------------------------

pub(crate) fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

pub(crate) fn number_value(n: i64) -> BslValue {
    BslValue::Number(BslNumber::from_i64(n))
}

/// `ТипЗначенияXDTO`/`ТипОбъектаXDTO` по номеру в модели.
pub fn type_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    shell_value(XdtoRepr::Type(model.clone(), index))
}

pub(crate) fn property_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    shell_value(XdtoRepr::Property(model.clone(), index))
}

/// `ЗначениеXDTO` из готовой пары «значение, лексическая форма».
pub(crate) fn data_value(model: &Rc<XdtoModel>, data: &Rc<XdtoValueData>) -> BslValue {
    shell_value(XdtoRepr::Value(model.clone(), data.clone()))
}

/// Границы наружу: `unbounded` — это `-1` (измерено).
pub(crate) fn bound_value(bound: Option<u32>) -> BslValue {
    match bound {
        Some(n) => number_value(i64::from(n)),
        None => number_value(-1),
    }
}

/// Как печатает `Строка()` от типа: `{URI}Имя`, а у безымянного
/// (анонимного) типа — пустая строка (измерено).
pub(crate) fn type_display(data: &XdtoTypeData) -> String {
    if data.name.is_empty() {
        return String::new();
    }
    XName {
        uri: data.ns.clone(),
        local: data.name.clone(),
    }
    .display_text()
}

/// Строковое представление значения модели типов.
pub fn display_text(obj: &XdtoRepr) -> Option<String> {
    Some(match obj {
        XdtoRepr::Type(model, i) => match model.types.get(*i) {
            Some(data) => type_display(data),
            None => String::new(),
        },
        // Свойство печатается ИМЕНЕМ (измерено: `Строка(Свв)` -> `name`).
        XdtoRepr::Property(model, i) => match model.properties.get(*i) {
            Some(data) => data.name.clone(),
            None => String::new(),
        },
        // Фабрика, экземпляр, его список и последовательность печатаются
        // именем своего типа — измерено все четыре: `Строка(Фаб)` ->
        // `ФабрикаXDTO`, `Строка(Объект)` -> `ОбъектXDTO`,
        // `Строка(О.code)` -> `СписокXDTO` (и пустого, и непустого),
        // `Строка(О.Последовательность())` -> `ПоследовательностьXDTO`.
        XdtoRepr::Properties(..)
        | XdtoRepr::Facets(..)
        | XdtoRepr::Facet(..)
        | XdtoRepr::Value(..)
        | XdtoRepr::Factory(_)
        | XdtoRepr::Serializer(_)
        | XdtoRepr::Object(..)
        | XdtoRepr::List(..)
        | XdtoRepr::Sequence(..) => type_name_of(obj)?.to_string(),
    })
}

/// Имя типа значения — то, чем зовут тип в коде.
pub fn type_name_of(obj: &XdtoRepr) -> Option<&'static str> {
    Some(match obj {
        XdtoRepr::Type(model, i) => match model.types.get(*i) {
            Some(data) if data.is_value() => "ТипЗначенияXDTO",
            Some(_) => "ТипОбъектаXDTO",
            None => return None,
        },
        XdtoRepr::Property(..) => "СвойствоXDTO",
        XdtoRepr::Properties(..) => "КоллекцияСвойствXDTO",
        XdtoRepr::Facets(..) => "КоллекцияФасетовXDTO",
        XdtoRepr::Facet(..) => "ФасетXDTO",
        XdtoRepr::Value(..) => "ЗначениеXDTO",
        XdtoRepr::Factory(_) => "ФабрикаXDTO",
        // Измерено: `Строка(Сер)` -> `СериализаторXDTO`, как у фабрики.
        XdtoRepr::Serializer(_) => "СериализаторXDTO",
        XdtoRepr::Object(..) => "ОбъектXDTO",
        XdtoRepr::List(..) => "СписокXDTO",
        XdtoRepr::Sequence(..) => "ПоследовательностьXDTO",
    })
}
