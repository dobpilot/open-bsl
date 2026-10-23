//! Измеренное заполнение собственных реквизитов документа и бизнес-процесса.
use super::*;

/// `Отказ` после `ОбработкаПроверкиЗаполнения`. Платформа приводит его как
/// `Булево`, а неприводимое значение (`"abc"`, `Неопределено`, `Null`,
/// массив) считает отсутствием отказа без ошибки. Измерено на справочнике и
/// документе: `measure-catalog-cancel-coercion.platform.txt` и
/// `measure-document-cancel-coercion.platform.txt` в `tests/conformance/measure/1c/`.
pub(super) fn handler_refused(value: &Value) -> bool {
    open_bsl::convert_to_boolean(value).unwrap_or(false)
}

/// Общий протокол вызова обработчика события объекта (`ОбработкаЗаполнения`,
/// `ОбработкаПроверкиЗаполнения`): обработчик ищется в модуле объекта без
/// требования `Экспорт`, получатель и память модуля — экземпляр объекта.
/// `arguments` строит фактические параметры по числу параметров обработчика
/// или возвращает `None` для неподдержанной сигнатуры — тогда поднимается
/// `unsupported`. Возвращает конечные значения параметров; `None` — у объекта
/// нет такого обработчика.
pub(super) fn call_object_handler(
    context: &mut CallContext<'_>,
    module: Option<&str>,
    instance: &open_bsl::ConfigurationInstance,
    this_object: Value,
    handler: &str,
    arguments: impl FnOnce(usize) -> Option<Vec<Value>>,
    unsupported: &'static str,
) -> RtResult<Option<Vec<Value>>> {
    let Some(module) = module else {
        return Ok(None);
    };
    let Some(count) = context.configuration_function_parameter_count(module, handler)? else {
        return Ok(None);
    };
    let arguments = arguments(count).ok_or_else(|| RtError::DynamicError(unsupported.into()))?;
    let (_, parameters) = context.call_configuration_instance_handler(
        module,
        instance,
        handler,
        arguments,
        this_object,
        open_bsl::ConfigurationInstanceEntryMode::Inherit,
    )?;
    Ok(Some(parameters))
}

/// `ОбработкаПроверкиЗаполнения(Отказ, ПроверяемыеРеквизиты)`: обработчик
/// получает `Ложь` и сам массив `paths` — его изменения видит стандартная
/// проверка, присваивание параметру нового массива — нет.
pub(super) fn call_check_filling_handler(
    context: &mut CallContext<'_>,
    module: Option<&str>,
    instance: &open_bsl::ConfigurationInstance,
    this_object: Value,
    paths: &Value,
    unsupported: &'static str,
) -> RtResult<Option<Vec<Value>>> {
    call_object_handler(
        context,
        module,
        instance,
        this_object,
        "ОбработкаПроверкиЗаполнения",
        |count| (count == 2).then(|| vec![Value::Boolean(false), paths.clone()]),
        unsupported,
    )
}

pub(super) fn fill<'a>(
    attributes: impl IntoIterator<Item = (usize, &'a DocumentAttribute)>,
    attribute_values: &RefCell<Vec<Value>>,
    catalog: &Arc<MetadataCatalog>,
    filling_data: &Value,
    context: &mut CallContext<'_>,
    validate_default: impl Fn(&Value) -> RtResult<()>,
) -> RtResult<()> {
    for (index, attribute) in attributes {
        // Платформа сохраняет непустую строку, истину и непустую дату.
        // Ложь остаётся доступна для заполнения при повторном вызове.
        let values = attribute_values.borrow();
        let preserved = if attribute.types().len() > 1 {
            // Измеренный составной реквизит сохраняет даже пустую строку
            // и пустую ссылку: его начальное значение — `Неопределено`.
            !matches!(&values[index], Value::Undefined)
        } else {
            match &values[index] {
                Value::Str(text) => text.len_utf16() != 0,
                Value::Boolean(value) => *value,
                Value::Date(date) => !date.is_empty(),
                value @ Value::Object(_) => value.object_ref().is_some_and(|receiver| {
                    receiver.downcast_ref::<ValueStorageValue>().is_some()
                        || receiver
                            .downcast_ref::<ConfigurationReferenceValue>()
                            .is_some_and(|reference| reference.uuid != [0; 16])
                        || receiver
                            .downcast_ref::<EnumerationReferenceValue>()
                            .is_some_and(|reference| !reference.value_id.is_empty())
                }),
                _ => false,
            }
        };
        drop(values);
        if preserved {
            continue;
        }
        let enabled = attribute.fill_from_filling_value().ok_or_else(|| {
            RtError::DynamicError(format!(
                "настройка заполнения реквизита «{}» неизвестна",
                attribute.name()
            ))
        })?;
        let source = if enabled
            && matches!(&filling_data, Value::Object(value) if matches!(value.as_ref(), BslObject::Structure(_)))
        {
            let field = context.runtime_shapes().names.intern(attribute.name());
            match filling_data.get_field(field) {
                Ok(value) => Some(value),
                Err(RtError::UnknownField(_)) => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        let value = match source {
            Some(value) => value,
            None => {
                let graph = attribute.fill_value().ok_or_else(|| {
                    RtError::DynamicError(format!(
                        "значение заполнения реквизита «{}» неизвестно",
                        attribute.name()
                    ))
                })?;
                let mut values = graph.materialize(context.runtime_shapes())?;
                if values.len() != 1 {
                    return Err(RtError::DynamicError(
                        "снимок значения заполнения должен иметь один корень".into(),
                    ));
                }
                let value = values.pop().expect("проверен один корень");
                validate_default(&value)?;
                value
            }
        };
        attribute_values.borrow_mut()[index] =
            document_object::adjust_attribute_value(value, attribute, catalog, context)?;
    }
    Ok(())
}
