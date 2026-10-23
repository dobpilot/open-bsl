//! Глобальный менеджер проверки текстового представления XML.

use bsl_rt::{
    Arity, BslNumber, BslString, BslValue, CallContext, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor, receiver_of,
};

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "МенеджерОбработкиСтрокиXML",
    type_display: "Менеджер обработки строки XML",
    type_names: &["XMLStringProcessingManager"],
};

#[derive(Debug)]
struct XmlStringProcessingManager;

impl ObjectProtocol for XmlStringProcessingManager {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }

    fn display(&self) -> String {
        "МенеджерОбработкиСтрокиXML".to_owned()
    }
}

fn find_disallowed(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    receiver_of::<XmlStringProcessingManager>(receiver, "НайтиНедопустимыеСимволыXML")?;
    let BslValue::Str(text) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "НайтиНедопустимыеСимволыXML",
        });
    };
    let start = match arguments.get(1) {
        None => 1,
        Some(BslValue::Number(number)) => number
            .to_i64_exact()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value != 0)
            .ok_or(RtError::TypeError {
                expected: "Положительное целое число",
                op: "НайтиНедопустимыеСимволыXML",
            })?,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Положительное целое число",
                op: "НайтиНедопустимыеСимволыXML",
            });
        }
    };
    let position = first_disallowed_xml_character(text, start).unwrap_or(0);
    Ok(BslValue::Number(BslNumber::from_i64(
        i64::try_from(position).expect("позиция строки помещается в i64"),
    )))
}

static METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["НайтиНедопустимыеСимволыXML", "FindDisallowedXMLCharacters"],
    Arity::range(1, 2),
    find_disallowed,
)];

pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&TYPE).with_methods(METHODS)];

pub(crate) fn new_manager() -> BslValue {
    BslValue::new_object(XmlStringProcessingManager)
}

pub(crate) fn has_disallowed_xml_characters(text: &BslString) -> bool {
    first_disallowed_xml_character(text, 1).is_some()
}

/// Возвращает позицию первого недопустимого символа в кодовых единицах
/// UTF-16. Позиции, как в BSL, начинаются с единицы.
fn first_disallowed_xml_character(text: &BslString, start: usize) -> Option<usize> {
    let units = text.units();
    let mut position = start;
    let suffix = units.get(start.saturating_sub(1)..).unwrap_or_default();
    for decoded in char::decode_utf16(suffix.iter().copied()) {
        let width = decoded
            .as_ref()
            .map_or(1, |character| character.len_utf16());
        match decoded {
            Ok(character) if is_allowed_xml_character(character) => position += width,
            _ => return Some(position),
        }
    }
    None
}

fn is_allowed_xml_character(character: char) -> bool {
    matches!(
        character as u32,
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}
