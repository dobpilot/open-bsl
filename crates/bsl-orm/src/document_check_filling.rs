//! Проверка полей документа по приобретённым настройкам и решению его обработчика.

use super::*;

fn unknown() -> RtError {
    RtError::DynamicError("настройки проверки заполнения документа не приобретены полностью".into())
}

pub(super) fn check_filling(
    receiver: &dyn ObjectProtocol,
    _: &[Value],
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let object = object(receiver)?;
    object.require_selected()?;
    let metadata = object.metadata()?;
    let standard = metadata
        .document_standard_fill_checking()
        .ok_or_else(unknown)?;
    let attributes = metadata.document_attributes().ok_or_else(unknown)?;
    let sections = metadata.document_tabular_sections().ok_or_else(unknown)?;
    let mut paths = Vec::new();
    for setting in standard {
        if setting.fill_checking == FillChecking::ShowError {
            paths.push(Value::Str(setting.name.as_str().into()));
        }
    }
    for attribute in attributes {
        if attribute.fill_checking().ok_or_else(unknown)? == FillChecking::ShowError {
            paths.push(Value::Str(attribute.name().into()));
        }
    }
    for section in sections {
        if section.fill_checking().ok_or_else(unknown)? == FillChecking::ShowError {
            paths.push(Value::Str(section.name().into()));
        }
        for attribute in section.attributes().ok_or_else(unknown)? {
            if attribute.fill_checking().ok_or_else(unknown)? == FillChecking::ShowError {
                paths.push(Value::Str(
                    format!("{}.{}", section.name(), attribute.name())
                        .as_str()
                        .into(),
                ));
            }
        }
    }
    let paths = Value::new_array(paths);
    let mut refused = false;
    if let Some(module) = metadata.object_module()
        && let Some(count) =
            context.configuration_function_parameter_count(module, "ОбработкаПроверкиЗаполнения")?
    {
        if count != 2 {
            return Err(RtError::DynamicError(
                "эта сигнатура ОбработкаПроверкиЗаполнения не поддерживается".into(),
            ));
        }
        let (_, parameters) = context.call_configuration_instance_handler(
            module,
            &object.0.instance,
            "ОбработкаПроверкиЗаполнения",
            vec![Value::Boolean(false), paths.clone()],
            Value::new_object(object.clone()),
        )?;
        refused = attribute_filling::handler_refused(&parameters[0]);
    }
    // Сохраняем исходный массив: его очистка влияет на проверку,
    // присваивание нового массива параметру обработчика — нет.
    let Value::Object(array) = paths else {
        unreachable!("создан массив")
    };
    let BslObject::Array(paths) = array.as_ref() else {
        unreachable!("создан массив")
    };
    let paths = paths.borrow().clone();
    for path in paths {
        let Value::Str(path) = path else {
            return Err(RtError::DynamicError(
                "проверка нестрокового пути документа не поддерживается".into(),
            ));
        };
        let path = path.to_string();
        let (section_name, column) = path
            .split_once('.')
            .map_or((path.as_str(), None), |(s, c)| (s, Some(c)));
        if let Some(index) = sections
            .iter()
            .position(|s| folded_eq(s.name(), section_name))
        {
            let section = &sections[index];
            let value = object.0.sections.borrow()[index].clone();
            let collection = value.object_ref().ok_or(RtError::NotAnObject)?;
            if let Some(column) = column {
                let attribute = section
                    .attributes()
                    .ok_or_else(unknown)?
                    .iter()
                    .find(|a| folded_eq(a.name(), column))
                    .ok_or_else(|| {
                        RtError::DynamicError(
                            "этот путь столбца документа не поддерживается".into(),
                        )
                    })?;
                for row_index in 0..collection.collection_len()? {
                    let row = collection
                        .collection_item(&Value::Number(BslNumber::from_i64(row_index as i64)))?;
                    let field = row
                        .object_ref()
                        .ok_or(RtError::NotAnObject)?
                        .get_property(attribute.name(), context)?;
                    if !field.is_filled()? {
                        refused = true;
                        message(
                            &format!("{}[{row_index}].{}", section.name(), attribute.name()),
                            format!(
                                "\"{}\" is required in row {} of the \"{}\" list.",
                                attribute.name(),
                                row_index + 1,
                                section.name()
                            ),
                            format!(
                                "Не заполнена колонка \"{}\" в строке {} списка \"{}\"",
                                attribute.name(),
                                row_index + 1,
                                section.name()
                            ),
                            context,
                        )?;
                    }
                }
            } else if collection.collection_len()? == 0 {
                refused = true;
                message(
                    section.name(),
                    format!("No rows entered into the list \"{}\"", section.name()),
                    format!("Не введено ни одной строки в список \"{}\"", section.name()),
                    context,
                )?;
            }
        } else {
            let field = if folded_eq(&path, "Date") {
                object.0.date.borrow().clone()
            } else if attributes.iter().any(|a| folded_eq(a.name(), &path)) {
                object.get_property(&path, context)?
            } else {
                return Err(RtError::DynamicError(format!(
                    "проверка пути документа «{path}» пока не поддерживается"
                )));
            };
            if !field.is_filled()? {
                refused = true;
                let caption_ru = if folded_eq(&path, "Date") {
                    "Дата"
                } else {
                    &path
                };
                message(
                    &path,
                    format!("\"{path}\" is a required field."),
                    format!("Поле \"{caption_ru}\" не заполнено"),
                    context,
                )?;
            }
        }
    }
    Ok(Value::Boolean(!refused))
}

fn message(
    field: &str,
    english: String,
    russian: String,
    context: &mut CallContext<'_>,
) -> RtResult<()> {
    let language = session(context)?
        .system_language()
        .ok_or_else(|| RtError::DynamicError("язык проверки документа не предоставлен".into()))?;
    let text = if language.starts_with("en") {
        english
    } else if language.starts_with("ru") {
        russian
    } else {
        return Err(RtError::DynamicError(
            "сообщения проверки документа для этого языка пока не измерены".into(),
        ));
    };
    if let Some(sink) = context.message_sink().map(Rc::clone) {
        let mut message = open_bsl::UserMessageDto::from_text(text);
        message.field = field.into();
        if let Some(left) = sink.message_bytes_left()
            && message.byte_size() > left
        {
            return Err(open_bsl::HostError::new(
                open_bsl::HostErrorCode::ResourceLimit,
                "исчерпан бюджет сообщений задания",
            )
            .raise());
        }
        sink.enqueue(&message).map_err(open_bsl::HostError::raise)?;
        if sink.also_write_to_stdout() {
            writeln!(context.stdout()?, "{}", message.text)
                .map_err(|error| RtError::IoError(error.to_string()))?;
        }
    } else {
        writeln!(context.stdout()?, "{text}")
            .map_err(|error| RtError::IoError(error.to_string()))?;
    }
    Ok(())
}
