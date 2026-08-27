//! Неизменяемый снимок информации об ошибке одного BSL-исполнения.
//!
//! Состав свойств — по выписке синтакс-помощника 8.3.27:
//! `ДополнительнаяИнформация`, `ИмяМодуля`, `ИсходнаяСтрока`, `Код`,
//! `НомерСтроки`, `Описание`, `Причина`; метод
//! `ЯвляетсяОшибкойКатегории(Категория, УчитыватьПричины = Истина)`.
//! Диагностическая модель open-bsl беднее платформенной: незаполненные
//! поля отдаются пустыми значениями своих типов, а категоризация ошибок
//! не выведена из замеров — истинен только член `ВсеОшибки`.

use crate::{
    Arity, BslString, BslValue, CallContext, EnumValue, MethodDescriptor, ObjectProtocol,
    PropertyDescriptor, RtError, RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static ERROR_INFO_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИнформацияОбОшибке",
    type_display: "Error information",
    type_names: &["ErrorInfo"],
};

#[derive(Debug)]
struct ErrorInfoObject {
    /// Краткий текст — `Описание`.
    description: BslString,
    /// Полный диагностический текст — `ПодробноеПредставлениеОшибки`.
    detail: BslString,
    module_name: BslString,
    line_number: i64,
    source_line: BslString,
    code: BslString,
    /// Вложенная `ИнформацияОбОшибке` либо `Неопределено`.
    cause: BslValue,
}

impl ObjectProtocol for ErrorInfoObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &ERROR_INFO_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        ERROR_INFO_PROPERTIES
    }

    fn get_property(&self, name: &str, ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
        crate::get_property_from_table(ERROR_INFO_PROPERTIES, "ИнформацияОбОшибке", self, name, ctx)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        ERROR_INFO_METHODS
    }
}

macro_rules! str_getter {
    ($get:ident, $field:ident, $name:literal) => {
        fn $get(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
            let error = receiver_of::<ErrorInfoObject>(receiver, $name)?;
            Ok(BslValue::Str(error.$field.clone()))
        }
    };
}

str_getter!(get_description, description, "Описание");
str_getter!(get_module_name, module_name, "ИмяМодуля");
str_getter!(get_source_line, source_line, "ИсходнаяСтрока");
str_getter!(get_code, code, "Код");

fn get_line_number(
    receiver: &dyn ObjectProtocol,
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let error = receiver_of::<ErrorInfoObject>(receiver, "НомерСтроки")?;
    Ok(BslValue::number_from_i64(error.line_number))
}

fn get_cause(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    let error = receiver_of::<ErrorInfoObject>(receiver, "Причина")?;
    Ok(error.cause.clone())
}

fn get_additional(receiver: &dyn ObjectProtocol, _ctx: &mut CallContext<'_>) -> RtResult<BslValue> {
    receiver_of::<ErrorInfoObject>(receiver, "ДополнительнаяИнформация")?;
    Ok(BslValue::Undefined)
}

static ERROR_INFO_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Описание", "Description"],
        get: get_description,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИмяМодуля", "ModuleName"],
        get: get_module_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["НомерСтроки", "LineNumber"],
        get: get_line_number,
        set: None,
    },
    PropertyDescriptor {
        names: &["ИсходнаяСтрока", "SourceLine"],
        get: get_source_line,
        set: None,
    },
    PropertyDescriptor {
        names: &["Код", "Code"],
        get: get_code,
        set: None,
    },
    PropertyDescriptor {
        names: &["Причина", "Cause"],
        get: get_cause,
        set: None,
    },
    PropertyDescriptor {
        names: &["ДополнительнаяИнформация", "AdditionalInformation"],
        get: get_additional,
        set: None,
    },
];

fn is_error_of_category(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<ErrorInfoObject>(receiver, "ЯвляетсяОшибкойКатегории")?;
    let Some(BslValue::Enum(category)) = args.first() else {
        return Err(RtError::TypeError {
            expected: "КатегорияОшибки",
            op: "ЯвляетсяОшибкойКатегории",
        });
    };
    // «Для категории ВсеОшибки всегда возвращает Истина» — выписка
    // синтакс-помощника. Категоризация конкретных ошибок open-bsl не
    // выведена из замеров, поэтому остальные категории отвечают Ложь.
    Ok(BslValue::Boolean(matches!(
        category,
        EnumValue::ErrorCategoryAllErrors
    )))
}

static ERROR_INFO_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ЯвляетсяОшибкойКатегории", "IsErrorOfCategory"],
    Arity::range(1, 2),
    is_error_of_category,
)];

/// Создаёт снимок уже очищенного пользовательского текста ошибки: краткое
/// и полное представления совпадают, координат нет.
#[must_use]
pub fn new_error_info(detail: impl Into<BslString>) -> BslValue {
    let detail = detail.into();
    BslValue::new_object(ErrorInfoObject {
        description: detail.clone(),
        detail,
        module_name: BslString::from_str(""),
        line_number: 0,
        source_line: BslString::from_str(""),
        code: BslString::from_str(""),
        cause: BslValue::Undefined,
    })
}

/// Снимок с полными координатами — мост из `JobErrorDto` фонового задания.
#[must_use]
pub fn new_error_info_detailed(
    description: &str,
    detail: &str,
    module_name: &str,
    line_number: Option<u32>,
    cause: BslValue,
) -> BslValue {
    BslValue::new_object(ErrorInfoObject {
        description: BslString::from_str(description),
        detail: BslString::from_str(detail),
        module_name: BslString::from_str(module_name),
        line_number: line_number.map_or(0, i64::from),
        source_line: BslString::from_str(""),
        code: BslString::from_str(""),
        cause,
    })
}

/// Возвращает подробное представление сохранённой ошибки.
///
/// # Errors
///
/// [`RtError::TypeError`], если передано не `ИнформацияОбОшибке`.
pub fn detailed_error_description(value: &BslValue) -> RtResult<BslValue> {
    let error = value
        .object_ref()
        .and_then(|object| object.downcast_ref::<ErrorInfoObject>())
        .ok_or(RtError::TypeError {
            expected: "ИнформацияОбОшибке",
            op: "ПодробноеПредставлениеОшибки",
        })?;
    Ok(BslValue::Str(error.detail.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_is_an_immutable_snapshot() {
        let info = new_error_info(BslString::from_str("текст"));
        let detail = detailed_error_description(&info).expect("это ИнформацияОбОшибке");
        assert_eq!(detail, BslValue::Str(BslString::from_str("текст")));
    }

    #[test]
    fn detail_rejects_another_value_type() {
        assert!(detailed_error_description(&BslValue::Undefined).is_err());
    }
}
