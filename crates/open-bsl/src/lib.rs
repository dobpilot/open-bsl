//! Устойчивый фасад для встраивания интерпретатора BSL в Rust-приложение.
//!
//! [`Engine`] собирает неизменяемый набор статических компонентов и
//! компилирует [`Module`]. [`State`] владеет сервисами конкретной сессии,
//! включая независимые потоки вывода.

mod dynamic;
mod engine;
mod error;
mod instance;
#[cfg(not(target_arch = "wasm32"))]
pub mod jobs;
mod state;

pub use instance::ModuleInstance;

pub use bsl_rt::BslValue as Value;
pub use bsl_rt::ConfigurationInstance;
pub use bsl_rt::SecurityModeControl;
pub use bsl_rt::encoding::decode_base64;
pub use bsl_rt::local_date_from_utc_seconds;
pub use bsl_rt::uuid;
pub use bsl_rt::{
    ApplicationCompletionSink, ApplicationErrorMapper, ApplicationExit, ApplicationLauncher,
    ApplicationRequest, ApplicationResponseMapper, ApplicationResult, ApplicationTarget, Arity,
    BackgroundJobService, BslDate, BslNumber, BslObject, BslString, ByteStreamProtocol,
    CallContext, Capability, Clock, ClockHandle, ColumnType, ComponentError,
    ConfigurationHandlerEntryMode, ConfigurationInstanceEntryMode, ConstructorCode,
    ConstructorDescriptor, ContextKind, DirEntry, EnumKind, EnumValue, FileCreate, FileHandle,
    FileMetadata, FileMetadataQuery, FileMetadataUpdate, FileOpenOptions, FileOperationRequest,
    FileSearchRequest, FileSystem, FixedTimeZone, FunctionCall, FunctionCode, FunctionDescriptor,
    FunctionKind, GlobalDescriptor, GlobalStagingBudget, GraphLimits, HostEnv, HostError,
    HostErrorCode, JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, JobWaitOutcome,
    LibraryDependency, LibraryDescriptor, LibraryRequirement, MethodCall, MethodDescriptor,
    NetworkError, NetworkErrorKind, ObjectMembersDescriptor, ObjectProtocol, ObjectRef,
    OpenedTemporaryFile, PICTURE_TYPE, POINT_IN_TIME_TYPE, PendingUserMessages, PointInTimeValue,
    PropertyDescriptor, PropertyGet, PropertySet, ProxyConfig, ProxyMode, RandomHandle,
    RandomSource, ReferenceOrderKey, RegistryError, RequestHandle, RtError, RtResult,
    RuntimeRegistry, RuntimeShapes, SecretString, SerializedValueGraph,
    ServerTlsCertificateRevocationCheckMode, StagingBudget, SystemApplicationLauncher, SystemClock,
    SystemRandom, SystemTimeZone, TimeZone, TlsConfig, TransferableTemporaryFile, TypeDescriptor,
    TypeId, TypeRef, UserMessageDto, UserMessageSink, ValueMaterializer, ValueTypeDescriptor,
    adjust_to_types, background_job_type_descriptor, brief_error_description,
    call_method_from_table, convert_to_boolean, detailed_error_description,
    fixed_array_from_values, fixed_map_entries, fixed_map_from_map, folded_eq,
    get_property_from_table, lookup_enum, lookup_member, new_point_in_time,
    set_property_from_table, value_list_from_values,
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
pub use bsl_config::{
    BoundConfigurationSource, ConfigurationArtifact, ConfigurationArtifactKind,
    ConfigurationArtifacts, ConfigurationDescription, ConfigurationIndex,
    ConfigurationModuleDeclaration, ConfigurationProperty, ConfigurationSource,
    ModuleDeclarationSource, ModuleExecution, ModuleImport, ModuleKind, SnapshotSource,
    decode_configuration_text,
};
pub use bsl_format::{NumberFormat, format_value};
pub use bsl_rt::Locale;
pub use bsl_rt::value_from_string_internal;
pub use bsl_rt::{NameId, NameInterner};

/// Разбирает число существующим форматным парсером через замкнутый фасад.
///
/// # Errors
///
/// Возвращает `RtError::Num`, если строка не является допустимым числом.
pub fn parse_number(value: &str, format: &NumberFormat) -> RtResult<BslNumber> {
    bsl_format::parse_number(value, format).map_err(RtError::Num)
}
pub use bsl_sema::SemaError;
pub use bsl_syntax::{
    Diagnostic, Expectation, FoundToken, LexError, ParseError, ParseErrorKind, PreprocSymbols, Span,
};
pub use dynamic::DynamicCode;
pub use engine::{
    Engine, EngineBuilder, Module, ModuleContextSource, ModuleGraphRecipe, ModuleRecipe,
    ModuleVisibility,
};
pub use error::Error;
pub use state::{Execution, ExecutionPoll, State, StateBuilder};

pub use bsl_rt::value_from_string_internal_with_resolver;

#[cfg(feature = "configuration-files")]
pub use bsl_config_edt::EdtConfigurationSource;
#[cfg(feature = "configuration-files")]
pub use bsl_config_xml::{ConfigurationFileFormat, XmlConfigurationSource};

mod configuration_artifacts;
pub use configuration_artifacts::configuration_template_value;
