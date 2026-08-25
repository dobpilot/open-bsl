//! Проба замкнутости публичного фасада: всё здесь написано ЧЕРЕЗ `open_bsl`
//! и ничего — через внутренние крейты. Крейт собирается только тогда, когда
//! автор компонента, разбор ошибки по фазам и разбор значения по видам
//! обходятся именами из фасада; иначе сборка падает. Зависимости в
//! `Cargo.toml` намеренно ограничены одним `open-bsl` (см. C.2).

use std::cell::Cell;
use std::rc::Rc;

use open_bsl::{
    Arity, ByteStreamProtocol, CallContext, Capability, CompileError, ConstructorCode,
    ConstructorDescriptor, ContextKind, Diagnostic, Engine, Error, Expectation, FoundToken,
    LexError, LibraryDependency, LibraryDescriptor, LibraryRequirement, MethodDescriptor,
    ObjectContextNeed, ObjectProtocol, ObjectRef, ParseError, ParseErrorKind, PreprocSymbols,
    PropertyDescriptor, RandomHandle, RegistryError, RtError, RtResult, RuntimeRegistry,
    RuntimeShapes, SemaError, Span, TextError, TypeDescriptor, Value, format_value,
};

// --- Свой компонент целиком, только через фасад --------------------------

#[derive(Debug)]
struct Meter {
    value: Rc<Cell<i64>>,
}

static METER_TYPE: TypeDescriptor = TypeDescriptor::new("consumer-host", "Счётчик");

fn meter_step(
    receiver: &dyn ObjectProtocol,
    _arguments: &[Value],
    _context: &mut CallContext<'_>,
) -> RtResult<Value> {
    let meter = receiver
        .downcast_ref::<Meter>()
        .ok_or(RtError::NotAnObject)?;
    meter.value.set(meter.value.get() + 1);
    Ok(Value::Undefined)
}

const METER_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Шаг", "Step"],
    Arity::exact(0),
    meter_step,
)];

fn meter_value(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<Value> {
    let meter = receiver
        .downcast_ref::<Meter>()
        .ok_or(RtError::NotAnObject)?;
    Ok(Value::number_from_i64(meter.value.get()))
}

const METER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["Значение", "Value"],
    get: meter_value,
    set: None,
}];

impl ObjectProtocol for Meter {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &METER_TYPE
    }
    fn property_table(&self) -> &'static [PropertyDescriptor] {
        METER_PROPERTIES
    }
    fn method_table(&self) -> &'static [MethodDescriptor] {
        METER_METHODS
    }
}

fn construct_meter(_context: &mut CallContext<'_>, _arguments: &[Value]) -> RtResult<Value> {
    Ok(Value::new_object(Meter {
        value: Rc::new(Cell::new(0)),
    }))
}

const METER_TYPES: &[&TypeDescriptor] = &[&METER_TYPE];

const METER_CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["Счётчик", "Meter"],
    arity: Arity::exact(0),
    call: construct_meter,
}];

/// Дескриптор компонента — с ОБЪЯВЛЕННОЙ зависимостью, чтобы через фасад
/// был достижим и [`LibraryDependency`].
pub fn meter_library() -> LibraryDescriptor {
    LibraryDescriptor::new("consumer-host", "1.0.0", ObjectContextNeed::Reduced)
        .with_dependencies(NO_DEPENDENCIES)
        .with_constructors(METER_CONSTRUCTORS)
        .with_types(METER_TYPES)
}

const NO_DEPENDENCIES: &[LibraryDependency] = &[];

// Эти типы раскрываются полями `RtError::CapabilityMissing` и результатом
// `CallContext::random`; фасад обязан дать хосту имена для их разбора.
const _: Option<Capability> = None;
const _: Option<ContextKind> = None;
const _: Option<RandomHandle> = None;

// --- Разбор ошибки по фазам, только через фасад --------------------------

/// Хост обязан уметь написать `match` по фазе ошибки и заглянуть внутрь
/// каждой. `Error` объявлен `#[non_exhaustive]`, поэтому ветка `_` тут не
/// лазейка, а контракт: новые фазы не ломают сборку хоста.
pub fn describe_error(error: &Error) -> String {
    match error {
        Error::Parse(diagnostic) => describe_diagnostic(diagnostic),
        Error::Semantic(semantic) => describe_semantic(semantic),
        Error::Compile(compile) => describe_compile(compile),
        Error::Registry(registry) => describe_registry(registry),
        Error::Runtime(runtime) => describe_runtime(runtime),
        Error::Bytecode(bytecode) => describe_bytecode(bytecode),
        _ => "неизвестная фаза".to_string(),
    }
}

fn describe_diagnostic(diagnostic: &Diagnostic) -> String {
    match diagnostic {
        Diagnostic::Lex(lex) => describe_lex(lex),
        Diagnostic::Parse(parse) => describe_parse(parse),
    }
}

fn describe_lex(lex: &LexError) -> String {
    format!("лексика: {lex}")
}

/// Разбор синтаксической ошибки по ВИДУ (не по тексту) — ради этого вид и
/// типизирован. Позиция берётся полем `span`, а не из текста.
fn describe_parse(parse: &ParseError) -> String {
    let span: Span = parse.span;
    let kind = match &parse.kind {
        ParseErrorKind::Expected { what, found } => {
            format!(
                "ожидание {} вместо {}",
                name_expectation(what),
                name_found(found)
            )
        }
        ParseErrorKind::BadAssignTarget => "негодная цель присваивания".to_string(),
        ParseErrorKind::ParenthesizedTarget => "цель в скобках".to_string(),
        ParseErrorKind::NestingTooDeep { limit } => format!("вложенность глубже {limit}"),
    };
    format!("разбор [{}..{}]: {kind}", span.start, span.end)
}

fn name_expectation(what: &Expectation) -> String {
    match what {
        Expectation::Token(_) => "токен".to_string(),
        Expectation::Keyword(_) => "ключевое слово".to_string(),
        Expectation::Identifier => "идентификатор".to_string(),
        Expectation::MemberName => "имя члена".to_string(),
        Expectation::Expression => "выражение".to_string(),
        Expectation::LabelName => "имя метки".to_string(),
    }
}

fn name_found(found: &FoundToken) -> String {
    match found {
        FoundToken::Keyword(_) => "ключевое слово".to_string(),
        FoundToken::Identifier => "идентификатор".to_string(),
        FoundToken::NumberLiteral => "число".to_string(),
        FoundToken::StringLiteral => "строку".to_string(),
        FoundToken::DateLiteral => "дату".to_string(),
        FoundToken::Symbol(_) => "символ".to_string(),
        FoundToken::Eof => "конец текста".to_string(),
    }
}

fn describe_semantic(semantic: &SemaError) -> String {
    format!("семантика: {semantic}")
}

fn describe_compile(compile: &CompileError) -> String {
    format!("компиляция: {compile}")
}

fn describe_registry(registry: &RegistryError) -> String {
    format!("компоненты: {registry}")
}

fn describe_runtime(runtime: &RtError) -> String {
    format!("исполнение: {runtime}")
}

fn describe_bytecode(bytecode: &TextError) -> String {
    format!("байт-код: {bytecode}")
}

// --- Разбор значения по видам, только через фасад ------------------------

/// `Value` НЕ `#[non_exhaustive]`: сопоставление с вариантом — рабочий
/// способ разбора значения, и здесь оно исчерпывающее, без ветки `_`.
pub fn classify_value(value: &Value) -> &'static str {
    match value {
        Value::Undefined => "неопределено",
        Value::Null => "null",
        Value::Boolean(_) => "булево",
        Value::Number(_) => "число",
        Value::Str(_) => "строка",
        Value::Date(_) => "дата",
        Value::Type(_) => "тип",
        Value::Enum(_) => "член перечисления",
        Value::EnumType(_) => "перечисление",
        Value::Object(_) => "объект",
    }
}

/// Значения РАЗНЫХ видов, собранные через фасад: строку хост кладёт тем же
/// `into()`, что число и свой объект.
pub fn build_values() -> Vec<Value> {
    vec![
        Value::Undefined,
        Value::Null,
        Value::Boolean(true),
        Value::number_from_i64(42),
        Value::Str("привет".into()),
        Value::new_array(vec![Value::number_from_i64(1)]),
        Value::new_object(Meter {
            value: Rc::new(Cell::new(0)),
        }),
    ]
}

/// Пользовательский вывод рендерится через `format_value`, как предписывает
/// README, — тоже из фасада.
pub fn render(value: &Value) -> String {
    format_value(value, None).unwrap_or_else(|error| format!("<{error}>"))
}

// --- Прочие имена контракта, встречающиеся в сигнатурах ------------------

/// Ссылки на типы, которые автор компонента или host видит в сигнатурах
/// трейтов и сервисов: доказываем их достижимость через фасад, называя их.
pub fn contract_types_are_reachable(
    _registry: &RuntimeRegistry,
    _shapes: &RuntimeShapes,
    _symbols: &PreprocSymbols,
    _requirements: &[LibraryRequirement],
    _stream: Option<&dyn ByteStreamProtocol>,
    _object: Option<ObjectRef>,
) {
}

// --- Сквозной прогон: компонент действительно работает -------------------

/// Не только компилируется, но и исполняется — чтобы проба не выродилась в
/// «собралось, а работает ли — неизвестно».
pub fn run_meter() -> Result<String, Error> {
    let engine = Engine::builder()
        .register_library(meter_library())
        .build()?;
    let module = engine.compile("с = Новый Счётчик();\nс.Шаг();\nс.Шаг();\nВозврат с.Значение;")?;
    Ok(engine.new_state().run(&module)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_component_runs_through_the_facade_alone() {
        assert_eq!(run_meter().unwrap(), "2");
    }

    #[test]
    fn a_syntax_error_is_matchable_by_kind() {
        let engine = Engine::builder().build().unwrap();
        let Err(error) = engine.compile("х = ;") else {
            panic!("разбор обязан упасть");
        };
        let text = describe_error(&error);
        assert!(text.starts_with("разбор ["), "{text}");
    }

    #[test]
    fn every_value_kind_classifies() {
        for value in build_values() {
            assert!(!classify_value(&value).is_empty());
            let _ = render(&value);
        }
    }
}
