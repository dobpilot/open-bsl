//! Объект канонической записи DOM через возможности текущего сеанса.

use bsl_rt::{
    Arity, BslString, BslValue, CallContext, EnumValue, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor,
};

pub(crate) static TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПреобразованиеККаноническомуXML",
    type_display: "Canonicalizing DOM document XML writer",
    type_names: &["ConversionToCanonicalXML"],
};

#[derive(Debug)]
struct Canonicalization;

impl ObjectProtocol for Canonicalization {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
    fn display(&self) -> String {
        TYPE.type_display.to_owned()
    }
}

pub(crate) fn construct(_context: &mut CallContext<'_>, _args: &[BslValue]) -> RtResult<BslValue> {
    Ok(BslValue::new_object(Canonicalization))
}

fn text(args: &[BslValue], algorithm_index: usize) -> RtResult<String> {
    let (doc, _) = crate::dom::as_node(&args[0], "КанонизироватьВСтроку")?;
    if doc.kind != crate::dom::DomKind::Document {
        return Err(RtError::TypeError {
            expected: "ДокументDOM",
            op: "Канонизация XML",
        });
    }
    let algorithm = args.get(algorithm_index).unwrap_or(&BslValue::Undefined);
    use EnumValue::*;
    let (exclusive, comments, version_11) = match algorithm {
        BslValue::Undefined | BslValue::Enum(XmlCanonicalizationTypeCanonical11) => {
            (false, false, true)
        }
        BslValue::Enum(XmlCanonicalizationTypeCanonical) => (false, false, false),
        BslValue::Enum(XmlCanonicalizationTypeCanonicalWithComments) => (false, true, false),
        BslValue::Enum(XmlCanonicalizationTypeExclusive) => (true, false, false),
        BslValue::Enum(XmlCanonicalizationTypeExclusiveWithComments) => (true, true, false),
        BslValue::Enum(XmlCanonicalizationTypeCanonical11WithComments) => (false, true, true),
        _ => {
            return Err(RtError::TypeError {
                expected: "ТипКаноническогоXML",
                op: "Канонизация XML",
            });
        }
    };
    let selection = match args.get(algorithm_index + 1) {
        None | Some(BslValue::Undefined) => None,
        Some(BslValue::Str(filter)) => {
            crate::xpath::canonical_selection(&doc, &filter.to_string())?
        }
        Some(BslValue::Number(_)) => None,
        Some(value) => crate::xpath::canonical_result_selection(value)?,
    };
    let prefixes = match args.get(algorithm_index + 2) {
        Some(BslValue::Str(prefixes)) => prefixes.to_string(),
        _ => String::new(),
    };
    crate::dom::canonical::render_version(
        &doc,
        exclusive,
        comments,
        &prefixes,
        selection.as_deref(),
        version_11,
    )
}

fn to_string(
    _receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::Str(BslString::from_str(&text(args, 1)?)))
}

fn to_file(
    _receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let path = match &args[1] {
        BslValue::Str(path) => path.to_string(),
        BslValue::Number(_) => context.format_value(&args[1], None)?,
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "КанонизироватьВФайл",
            });
        }
    };
    let output = text(args, 2)?;
    let files = context.files()?;
    let path = bsl_rt::prepare_file_operation_path(&path, files)
        .map_err(|error| RtError::IoError(error.to_string()))?;
    files
        .write(&path, output.as_bytes())
        .map_err(|error| RtError::IoError(error.to_string()))?;
    Ok(BslValue::Undefined)
}

fn to_stream(
    _receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let output = text(args, 2)?;
    let object = args[1].object_ref().ok_or(RtError::TypeError {
        expected: "Поток",
        op: "КанонизироватьВПоток",
    })?;
    let stream = object.byte_stream().ok_or(RtError::TypeError {
        expected: "Поток",
        op: "КанонизироватьВПоток",
    })?;
    stream.write_all(output.as_bytes(), "КанонизироватьВПоток")?;
    Ok(BslValue::Undefined)
}

static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["КанонизироватьВСтроку", "CanonicalizeToString"],
        Arity::range(1, 4),
        to_string,
    ),
    MethodDescriptor::new(
        &["КанонизироватьВФайл", "CanonicalizeToFile"],
        Arity::range(2, 5),
        to_file,
    ),
    MethodDescriptor::new(
        &["КанонизироватьВПоток", "CanonicalizeToStream"],
        Arity::range(2, 5),
        to_stream,
    ),
];
pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] =
    &[ObjectMembersDescriptor::new(&TYPE).with_methods(METHODS)];
