//! Устойчивый фасад для встраивания интерпретатора BSL в Rust-приложение.
//!
//! [`Engine`] собирает неизменяемый набор статических компонентов и
//! компилирует [`Module`]. [`State`] владеет сервисами конкретной сессии,
//! включая независимые потоки вывода.

mod dynamic;
mod engine;
mod error;
mod state;

pub use bsl_rt::BslValue as Value;
pub use bsl_rt::{
    Arity, BslDate, BslNumber, BslObject, BslString, ByteStreamProtocol, CallContext, Capability,
    Clock, ComponentError, ConstructorCode, ConstructorDescriptor, ContextKind, DirEntry, EnumKind,
    EnumValue, FileCreate, FileHandle, FileMetadata, FileOpenOptions, FileSystem, FixedTimeZone,
    FunctionCode, FunctionDescriptor, FunctionKind, HostEnv, LibraryDependency, LibraryDescriptor,
    LibraryRequirement, MethodCall, MethodDescriptor, NetworkError, NetworkErrorKind,
    ObjectContextNeed, ObjectProtocol, ObjectRef, PropertyDescriptor, PropertyGet, PropertySet,
    ProxyConfig, ProxyMode, RandomHandle, RandomSource, RegistryError, RequestHandle, RtError,
    RtResult, RuntimeRegistry, RuntimeShapes, SecretString, SystemClock, SystemRandom,
    SystemTimeZone, TimeZone, TlsConfig, TypeDescriptor, TypeRef, call_method_from_table,
    folded_eq, get_property_from_table, set_property_from_table,
};
pub use bsl_rt::{
    HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink, HttpErrorMapper,
    HttpPromiseSpawner, HttpResponseMapper, HttpWireRequest, HttpWireResponse,
};

// Замыкание поверхности: типы, до которых достаёт публичная сигнатура
// фасада, обязаны быть достижимы через него, а не только упомянуты.
// `Error` раскрывает диагностику каждой фазы (см. `error.rs`), `Diagnostic`
// — лексическую и синтаксическую ошибку с их видами, а `Value` — типы
// своих вариантов. Без этих имён хост получил бы ошибку или значение и не
// смог бы написать по нему `match`.
pub use bsl_bytecode::TextError;
pub use bsl_compiler::CompileError;
pub use bsl_format::format_value;
pub use bsl_sema::SemaError;
pub use bsl_syntax::{
    Diagnostic, Expectation, FoundToken, LexError, ParseError, ParseErrorKind, PreprocSymbols, Span,
};
pub use dynamic::DynamicCode;
pub use engine::{Engine, EngineBuilder, Module};
pub use error::Error;
pub use state::{Execution, ExecutionPoll, State, StateBuilder};
