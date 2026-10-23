//! Материализация приобретённых макетов через существующие компоненты.
use crate::{
    CallContext, ConfigurationArtifact, ConfigurationArtifactKind, RtError, RtResult, Value,
};

/// Создаёт собственное значение сеанса из неизменяемого содержимого макета.
///
/// # Errors
/// Возвращает ошибку неподдержанного вида, недоступного компонента или декодера.
pub fn configuration_template_value(
    artifact: &ConfigurationArtifact,
    context: &mut CallContext<'_>,
) -> RtResult<Value> {
    if artifact.kind != ConfigurationArtifactKind::Template {
        return Err(RtError::DynamicError("ресурс не является макетом".into()));
    }
    let _ = &context;
    let representation = artifact.representation.as_ref();
    match representation {
        #[cfg(feature = "spreadsheet")]
        "SpreadsheetDocument" => {
            let bytes = &artifact.bytes;
            let data = if bytes.starts_with(b"MOXCEL") {
                bsl_spreadsheet::from_mxl_bytes(bytes)?
            } else {
                let text =
                    bsl_config::decode_configuration_text(bytes).map_err(RtError::DynamicError)?;
                bsl_spreadsheet::from_template_xml(&text)?
            };
            Ok(bsl_spreadsheet::from_document_data(
                data,
                context.files_rc()?,
            ))
        }
        #[cfg(feature = "textdoc")]
        "TextDocument" => {
            let text = bsl_config::decode_configuration_text(&artifact.bytes)
                .map_err(RtError::DynamicError)?;
            let value = bsl_textdoc::new_text_document(context.files_rc()?);
            value
                .object_ref()
                .ok_or(RtError::NotAnObject)?
                .call_method(
                    "УстановитьТекст",
                    &[Value::Str(text.trim_start_matches('\u{feff}').into())],
                    context,
                )?;
            Ok(value)
        }
        "BinaryData" => Ok(Value::binary_data_of(artifact.bytes.as_ref())),
        _ => Err(RtError::DynamicError(format!(
            "вид макета «{representation}» пока не поддерживается: {}",
            artifact.description.name
        ))),
    }
}
