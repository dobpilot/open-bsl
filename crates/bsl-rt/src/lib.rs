//! Рантайм-слой: значения, арифметика/сравнение, коллекции (`Массив`,
//! `Структура`, `Соответствие`, `ТаблицаЗначений`), строки UTF-16
//! (`Str`/`BslString`), даты (`Date`/`BslDate`, эпоха — `0001-01-01`, см.
//! модуль `date`) и типы как значения (`Type`/`TypeId`). `BslValue` растёт
//! по мере готовности остальных слоёв, а не заранее под все типы из брифа.

pub mod background_jobs;
mod bindata;
mod builtin;
mod component;
mod date;
pub mod encoding;
mod enums;
mod env;
mod error;
mod error_info;
mod fill;
mod fixed_array;
pub mod fold;
mod host_error;
mod http;
mod interner;
mod job_dto;
mod locale;
mod map;
mod metadata;
mod object;
mod object_protocol;
pub mod open_questions;
mod promise;
mod runtime_shapes;
mod shape;
mod string;
mod table;
mod temp_storage;
mod type_description;
mod types;
mod tz;
mod user_message;
pub mod uuid;
mod value;
mod value_graph;
mod value_list;
mod vstr;
pub use background_jobs::{BackgroundJobService, JobWaitOutcome};
pub use error::{ComponentError, RtError, RtResult};
pub use host_error::{HostError, HostErrorCode};
pub use job_dto::{JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, UserMessageDto};
pub use temp_storage::{
    GlobalStagingBudget, StagedWrite, StagingBudget, TempMailbox, TempStorageHub,
    TempStorageSession,
};
pub use value::BslValue;
pub use value_graph::{GraphBudget, GraphLimits, SerializedValueGraph};

pub use bsl_number::BslNumber;

/// Cargo-идентичность базового runtime-компонента. Её использует манифест
/// байт-кода; строка берётся из манифеста самого крейта, а не дублируется в
/// компиляторе.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use builtin::{
    BUILTIN_FN_NAMES, BUILTIN_METHOD_NAMES, BuiltinFn, BuiltinMethod, HostEffect, call_builtin_env,
    call_builtin_files, call_builtin_fn, call_builtin_fn_ctx, call_builtin_method,
    call_builtin_method_ctx, call_builtin_method_files, call_builtin_temp_file,
};
pub use component::{
    Arity, ByteStreamFactory, CallContext, CallOutcome, Capability, ComponentCall, ConstructorCode,
    ConstructorDescriptor, ContextKind, ExecutionParts, FunctionCode, FunctionDescriptor,
    FunctionKind, InterpreterServices, LibraryDependency, LibraryDescriptor, LibraryKey,
    LibraryRequirement, MethodCall, MethodDescriptor, ObjectMembersDescriptor, PendingHostCall,
    PropertyDescriptor, PropertyGet, PropertySet, RegistryError, RuntimeBuilder, RuntimeRegistry,
    SuspendingMethodCall, call_method_from_table, core_library, get_property_from_table,
    set_property_from_table,
};
pub use date::{
    BslDate, DEFAULT_PATTERN as DEFAULT_DATE_PATTERN, UNIX_EPOCH_SECONDS,
    format_long as format_date_long, format_pattern as format_date_pattern,
    local_date_from_utc_seconds, pseudo_unix_seconds,
};
pub use enums::{EnumKind, EnumValue, lookup_enum, lookup_member};
pub use env::{
    Clock, DirEntry, FileCreate, FileHandle, FileMetadata, FileOpenOptions, FileSystem,
    FixedTimeZone, HostEnv, MAX_OFFSET_SECONDS, MIN_TRANSITION_GAP_SECONDS, RandomHandle,
    RandomSource, SystemClock, SystemFileSystem, SystemRandom, TimeZone, UserMessageSink,
};
pub use error_info::{detailed_error_description, new_error_info};
pub use fold::folded_eq;
pub use http::{
    ClientIdentity, HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink,
    HttpErrorMapper, HttpPromiseSpawner, HttpResponseMapper, HttpWireRequest, HttpWireResponse,
    NetworkError, NetworkErrorKind, ProxyConfig, ProxyMode, RequestHandle, SecretBytes,
    SecretString, TlsConfig,
};
pub use interner::{NameId, NameInterner, first_folded_duplicate};
pub use locale::{Locale, NBSP};
pub use object::{BslObject, StructureStorage};
pub use object_protocol::{
    ByteStreamProtocol, ObjectDowncast, ObjectProtocol, ObjectRef, TypeDescriptor, receiver_of,
};
pub use promise::{ExecutionToken, PROMISE_TYPE, PromiseId, PromiseValue};
pub use runtime_shapes::RuntimeShapes;
pub use shape::{MAX_SHAPE_TRANSITIONS, Shape, ShapeTable};
pub use string::BslString;
pub use table::ValueTableData;
pub use types::{TypeId, TypeRef};
pub use tz::SystemTimeZone;
// Модель типов XDTO наружу крейта нужна целиком: строит её фабрика,
// которой в этой реализации ещё нет, а до тех пор единственный её
// потребитель — собственные тесты модуля.

/// Предвыделяет ёмкость под НЕДОСТОВЕРНОЕ число из входного формата, не
/// роняя процесс, и возвращает то же число, зажатое в `[0, bound]`.
///
/// `declared` — заявленное входом количество, ЗНАКОВОЕ: счётчики MXL
/// приходят из `Node::number()` как `i64`, и отрицательное при `as usize`
/// стало бы огромным ещё до всякой проверки. `bound` — сколько элементов
/// реально осталось во входе (байтов у `ЗначениеИзСтрокиВнутр`, узлов у
/// MXL). Ограничение входом убирает абсурдные числа, `try_reserve` —
/// оставшиеся: законный, но большой `bound` иначе всё равно завершил бы
/// процесс на аллокации (`Vec::with_capacity` при отказе делает `abort`,
/// а не ловимое `Попыткой` исключение — это и есть воспроизведение 5).
///
/// # Errors
///
/// [`std::collections::TryReserveError`], если аллокация не удалась;
/// вызывающий превращает её в свою ошибку формата.
pub fn reserve_hint<T>(
    vec: &mut Vec<T>,
    declared: i64,
    bound: usize,
) -> Result<usize, std::collections::TryReserveError> {
    let hint = declared.clamp(0, i64::try_from(bound).unwrap_or(i64::MAX)) as usize;
    vec.try_reserve(hint)?;
    Ok(hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    /// Воспроизведение 5: заявленное входом число не превращается в
    /// огромную ёмкость. Отрицательное (счётчик MXL из `i64`) зажимается в
    /// ноль ДО преобразования в `usize`, а превышающее остаток входа — по
    /// границе; в пределах границы проходит как есть.
    #[test]
    fn reserve_hint_clamps_untrusted_counts() {
        let mut a: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut a, -1, 100).unwrap(), 0);
        let mut b: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut b, 1_000_000_000, 8).unwrap(), 8);
        let mut c: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut c, 5, 8).unwrap(), 5);
    }
}
