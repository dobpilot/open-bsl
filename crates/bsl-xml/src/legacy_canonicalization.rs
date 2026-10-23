//! Старый интерфейс канонизации, который использует исходный код UNF.

use bsl_rt::{
    Arity, BslString, BslValue, CallContext, EnumValue, MethodDescriptor, ObjectMembersDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor, receiver_of,
};
use std::cell::RefCell;

pub(crate) static WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КаноническаяЗаписьXML",
    type_display: "Canonical XML writing",
    type_names: &["XMLCanonicalizingWriter"],
};
pub(crate) static DOM_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КаноническийDOM",
    type_display: "DOM node canonical writing",
    type_names: &["DOMCanonicalization"],
};

#[derive(Debug)]
struct Writer {
    state: RefCell<Option<WriterState>>,
}
#[derive(Debug)]
struct WriterState {
    exclusive: bool,
    comments: bool,
    output: Option<String>,
}
#[derive(Debug)]
struct CanonicalDom;

impl ObjectProtocol for Writer {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }
    fn display(&self) -> String {
        WRITER_TYPE.type_display.to_owned()
    }
}
impl ObjectProtocol for CanonicalDom {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOM_TYPE
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        DOM_METHODS
    }
    fn display(&self) -> String {
        DOM_TYPE.type_display.to_owned()
    }
}

pub(crate) fn construct_writer(
    _context: &mut CallContext<'_>,
    _args: &[BslValue],
) -> RtResult<BslValue> {
    Ok(BslValue::new_object(Writer {
        state: RefCell::new(None),
    }))
}
pub(crate) fn construct_dom(
    _context: &mut CallContext<'_>,
    _args: &[BslValue],
) -> RtResult<BslValue> {
    Ok(BslValue::new_object(CanonicalDom))
}

fn set_string(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = receiver_of::<Writer>(receiver, "УстановитьСтроку")?;
    if writer.state.borrow().is_some() {
        return Err(RtError::DynamicError(
            "канонический писатель уже открыт".into(),
        ));
    }
    use EnumValue::*;
    let (exclusive, comments) = match args.first() {
        None
        | Some(BslValue::Undefined)
        | Some(BslValue::Enum(XmlCanonicalizationTypeCanonical)) => (false, false),
        Some(BslValue::Enum(XmlCanonicalizationTypeCanonicalWithComments)) => (false, true),
        Some(BslValue::Enum(XmlCanonicalizationTypeExclusive)) => (true, false),
        Some(BslValue::Enum(XmlCanonicalizationTypeExclusiveWithComments)) => (true, true),
        _ => {
            return Err(RtError::TypeError {
                expected: "ТипКаноническогоXML версии 1.0",
                op: "УстановитьСтроку",
            });
        }
    };
    *writer.state.borrow_mut() = Some(WriterState {
        exclusive,
        comments,
        output: None,
    });
    Ok(BslValue::Undefined)
}

fn close(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let writer = receiver_of::<Writer>(receiver, "Закрыть")?;
    let output = writer
        .state
        .borrow_mut()
        .take()
        .and_then(|state| state.output)
        .unwrap_or_default();
    Ok(BslValue::Str(BslString::from_str(&output)))
}

fn canonicalize(
    _receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let (node, _) = crate::dom::as_node(&args[0], "Канонизировать")?;
    let object = args[1].object_ref().ok_or(RtError::TypeError {
        expected: "КаноническаяЗаписьXML",
        op: "Канонизировать",
    })?;
    let writer = object.downcast_ref::<Writer>().ok_or(RtError::TypeError {
        expected: "КаноническаяЗаписьXML",
        op: "Канонизировать",
    })?;
    let mut state = writer.state.borrow_mut();
    let state = state
        .as_mut()
        .ok_or_else(|| RtError::DynamicError("канонический писатель не открыт".into()))?;
    if state.output.is_some() {
        return Err(RtError::DynamicError(
            "канонический документ уже записан".into(),
        ));
    }
    let selected = match args.get(2) {
        None | Some(BslValue::Undefined) => None,
        Some(value) => Some(crate::xpath::legacy_canonical_selection(value)?),
    };
    state.output = Some(crate::dom::canonical::render_version(
        &node,
        state.exclusive,
        state.comments,
        "",
        selected.as_deref(),
        false,
    )?);
    Ok(BslValue::Undefined)
}

static WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьСтроку", "SetString"],
        Arity::range(0, 1),
        set_string,
    ),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), close),
];
static DOM_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Канонизировать", "Canonicalize"],
    Arity::range(2, 3),
    canonicalize,
)];
pub(crate) const API_MEMBERS: &[ObjectMembersDescriptor] = &[
    ObjectMembersDescriptor::new(&WRITER_TYPE).with_methods(WRITER_METHODS),
    ObjectMembersDescriptor::new(&DOM_TYPE).with_methods(DOM_METHODS),
];
