//! Устойчивый фасад для встраивания интерпретатора BSL в Rust-приложение.
//!
//! [`Engine`] собирает неизменяемый набор статических компонентов и
//! компилирует [`Module`]. [`State`] владеет сервисами конкретной сессии,
//! включая независимые потоки вывода.

mod dynamic;
mod engine;
mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod jobs;
mod state;

pub use bsl_rt::BslValue as Value;
pub use bsl_rt::{
    ApplicationCompletionSink, ApplicationErrorMapper, ApplicationExit, ApplicationLauncher,
    ApplicationRequest, ApplicationResponseMapper, ApplicationResult, ApplicationTarget, Arity,
    BslDate, BslNumber, BslObject, BslString, ByteStreamProtocol, CallContext, Capability, Clock,
    ComponentError, ConstructorCode, ConstructorDescriptor, ContextKind, DirEntry, EnumKind,
    EnumValue, FileCreate, FileHandle, FileMetadata, FileMetadataQuery, FileMetadataUpdate,
    FileOpenOptions, FileOperationRequest, FileSearchRequest, FileSystem, FixedTimeZone,
    FunctionCode, FunctionDescriptor, FunctionKind, GlobalStagingBudget, GraphLimits, HostEnv,
    HostError, HostErrorCode, JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto,
    LibraryDependency, LibraryDescriptor, LibraryRequirement, MethodCall, MethodDescriptor,
    NetworkError, NetworkErrorKind, ObjectMembersDescriptor, ObjectProtocol, ObjectRef,
    OpenedTemporaryFile, PropertyDescriptor, PropertyGet, PropertySet, ProxyConfig, ProxyMode,
    RandomHandle, RandomSource, RegistryError, RequestHandle, RtError, RtResult, RuntimeRegistry,
    RuntimeShapes, SecretString, SerializedValueGraph, StagingBudget, SystemApplicationLauncher,
    SystemClock, SystemRandom, SystemTimeZone, TimeZone, TlsConfig, TransferableTemporaryFile,
    TypeDescriptor, TypeRef, UserMessageDto, UserMessageSink, call_method_from_table, folded_eq,
    get_property_from_table, set_property_from_table,
};
pub use bsl_rt::{
    FileNotificationOperation, HostPromiseSpawner, HttpClient, HttpClientConfig, HttpClientFactory,
    HttpCompletionSink, HttpErrorMapper, HttpPromiseSpawner, HttpResponseMapper, HttpWireRequest,
    HttpWireResponse, NotificationDescription,
};
pub use bsl_rt::{TemporaryFileCleanup, TemporaryFileRegistry, TemporaryFileResource};
pub use bsl_vm::{DebugAction, DebugHook, DebugPosition, DebugValues, ROOT_MODULE};

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
pub use engine::{Engine, EngineBuilder, Module, ModuleGraphRecipe, ModuleRecipe};
pub use error::Error;
pub use state::{Execution, ExecutionPoll, State, StateBuilder};
