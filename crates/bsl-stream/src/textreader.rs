//! Последовательное чтение всего оставшегося текста из байтового потока.

use std::cell::Cell;

use bsl_rt::{
    Arity, BslString, BslValue, CallContext, MethodDescriptor, ObjectProtocol, RtError, RtResult,
    TypeDescriptor, encoding::Encoding, receiver_of,
};

pub(crate) static TEXT_READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЧтениеТекста",
    type_display: "TextReader",
    type_names: &["TextReader"],
};

#[derive(Debug)]
struct TextReaderObject {
    stream: BslValue,
    encoding: Encoding,
    closed: Cell<bool>,
}

fn encoding(value: &BslValue) -> RtResult<Encoding> {
    const OP: &str = "Новый ЧтениеТекста";
    // ИЗМЕРЕНО на 8.3.27: явное `Неопределено` выбирает однобайтовое
    // системное умолчание, которое в oracle-сеансе совпадает с Latin-1.
    Encoding::from_bsl_value(value, Some(Encoding::Latin1), OP)
}

/// Строит `ЧтениеТекста(Поток, Кодировка)`.
///
/// # Errors
///
/// Возвращает ошибку, если первый аргумент не предоставляет байтовый поток
/// либо кодировка неизвестна.
pub fn new_text_reader(stream: &BslValue, encoding_value: &BslValue) -> RtResult<BslValue> {
    if stream.byte_stream().is_none() {
        return Err(RtError::TypeError {
            expected: "Поток",
            op: "Новый ЧтениеТекста",
        });
    }
    Ok(BslValue::new_object(TextReaderObject {
        stream: stream.clone(),
        encoding: encoding(encoding_value)?,
        closed: Cell::new(false),
    }))
}

fn read_text(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    const OP: &str = "ЧтениеТекста.Прочитать";
    let reader = receiver_of::<TextReaderObject>(receiver, "Прочитать")?;
    if reader.closed.get() {
        return Err(RtError::IoError(format!("{OP}: объект закрыт")));
    }
    let stream = reader.stream.byte_stream().ok_or(RtError::TypeError {
        expected: "Поток",
        op: OP,
    })?;
    let position = stream.position(OP)?;
    let len = stream.len(OP)?;
    if position >= len {
        return Ok(BslValue::Undefined);
    }
    let count = usize::try_from(len - position).map_err(|_| RtError::TypeError {
        expected: "Остаток потока, который умещается в памяти",
        op: OP,
    })?;
    let bytes = stream.read_bytes(count, OP)?;
    let text = if position == 0 {
        reader.encoding.decode(&bytes)?
    } else {
        reader.encoding.decode_without_bom(&bytes)?
    };
    Ok(BslValue::Str(BslString::from_utf8_string(text)))
}

fn close_reader(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let reader = receiver_of::<TextReaderObject>(receiver, "Закрыть")?;
    reader.closed.set(true);
    Ok(BslValue::Undefined)
}

const METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Прочитать", "Read"], Arity::exact(0), read_text),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), close_reader),
];

impl ObjectProtocol for TextReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TEXT_READER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::EnumValue;

    fn with_context<T>(f: impl FnOnce(&mut CallContext<'_>) -> T) -> T {
        let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut context = CallContext::native(&mut shapes, |_value, _spec| {
            unreachable!("методы ЧтениеТекста не форматируют значения")
        });
        f(&mut context)
    }

    #[test]
    fn reader_consumes_utf8_then_returns_undefined_and_closes_only_itself() {
        let buffer = BslValue::binary_buffer_of("Аб".as_bytes().to_vec());
        let stream = crate::new_memory_stream(&buffer).unwrap();
        let reader =
            new_text_reader(&stream, &BslValue::Enum(EnumValue::TextEncodingUtf8)).unwrap();
        let object = reader.object_ref().unwrap();
        with_context(|context| {
            assert_eq!(
                object.call_method("Прочитать", &[], context).unwrap(),
                BslValue::Str(BslString::from_str("Аб"))
            );
            assert_eq!(
                object.call_method("Прочитать", &[], context).unwrap(),
                BslValue::Undefined
            );
            object.call_method("Закрыть", &[], context).unwrap();
            assert!(object.call_method("Прочитать", &[], context).is_err());
        });
        assert_eq!(stream.byte_stream().unwrap().len("test").unwrap(), 4);
    }

    #[test]
    fn reader_decodes_string_encoding_and_removes_initial_bom() {
        let buffer = BslValue::binary_buffer_of(vec![0xEF, 0xBB, 0xBF, b'A']);
        let stream = crate::new_memory_stream(&buffer).unwrap();
        let reader =
            new_text_reader(&stream, &BslValue::Str(BslString::from_str("UTF-8"))).unwrap();
        with_context(|context| {
            assert_eq!(
                reader
                    .object_ref()
                    .unwrap()
                    .call_method("Read", &[], context)
                    .unwrap(),
                BslValue::Str(BslString::from_str("A"))
            );
        });
    }
}
