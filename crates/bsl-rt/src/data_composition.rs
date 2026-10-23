//! Минимальные неизменяемые значения системы компоновки данных, достигнутые
//! при запуске модулей сеанса. Движок компоновки этот модуль не реализует.

use crate::{
    BslString, BslValue, CallContext, EnumKind, EnumValue, ObjectProtocol, ObjectRef, RtError,
    RtResult, TypeDescriptor,
};

pub(crate) static DATA_COMPOSITION_FIELD_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПолеКомпоновкиДанных",
    type_display: "Поле компоновки данных",
    type_names: &["DataCompositionField"],
};

pub(crate) static DATA_COMPOSITION_PARAMETER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПараметрКомпоновкиДанных",
    type_display: "Параметр компоновки данных",
    type_names: &["DataCompositionParameter"],
};

pub(crate) static STANDARD_BEGINNING_DATE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СтандартнаяДатаНачала",
    type_display: "Стандартная дата начала",
    type_names: &["StandardBeginningDate"],
};

#[derive(Debug)]
struct DataCompositionField {
    path: BslString,
}

impl ObjectProtocol for DataCompositionField {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_FIELD_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.path.units().is_empty())
    }

    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        other
            .downcast_ref::<Self>()
            .map(|other| crate::folded_eq(&self.path.to_string(), &other.path.to_string()))
    }

    fn value_hash_key(&self) -> Option<u64> {
        Some(crate::fold::folded_hash(&self.path.to_string()))
    }

    fn display(&self) -> String {
        self.path.to_string()
    }
}

pub(crate) fn construct_field(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let [BslValue::Str(path)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Новый ПолеКомпоновкиДанных",
        });
    };
    Ok(BslValue::new_object(DataCompositionField {
        path: path.clone(),
    }))
}

#[derive(Debug)]
struct DataCompositionParameter {
    name: BslString,
}

impl ObjectProtocol for DataCompositionParameter {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_COMPOSITION_PARAMETER_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.name.units().is_empty())
    }

    fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
        other
            .downcast_ref::<Self>()
            .map(|other| crate::folded_eq(&self.name.to_string(), &other.name.to_string()))
    }

    fn value_hash_key(&self) -> Option<u64> {
        Some(crate::fold::folded_hash(&self.name.to_string()))
    }

    fn display(&self) -> String {
        self.name.to_string()
    }
}

pub(crate) fn construct_parameter(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let [BslValue::Str(name)] = arguments else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "Новый ПараметрКомпоновкиДанных",
        });
    };
    Ok(BslValue::new_object(DataCompositionParameter {
        name: name.clone(),
    }))
}

struct StandardBeginningDate {
    variant: EnumValue,
}

impl std::fmt::Debug for StandardBeginningDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("СтандартнаяДатаНачала")
            .field("вариант", &self.variant)
            .finish()
    }
}

impl ObjectProtocol for StandardBeginningDate {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &STANDARD_BEGINNING_DATE_TYPE
    }
}

pub(crate) fn construct_standard_beginning_date(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let [BslValue::Enum(variant)] = arguments else {
        return Err(RtError::TypeError {
            expected: "ВариантСтандартнойДатыНачала",
            op: "Новый СтандартнаяДатаНачала",
        });
    };
    if variant.kind() != EnumKind::StandardBeginningDateVariant {
        return Err(RtError::TypeError {
            expected: "ВариантСтандартнойДатыНачала",
            op: "Новый СтандартнаяДатаНачала",
        });
    }
    Ok(BslValue::new_object(StandardBeginningDate {
        variant: *variant,
    }))
}

#[cfg(test)]
#[path = "data_composition_tests.rs"]
mod tests;
