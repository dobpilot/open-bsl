//! Состояние параметров одного сеанса конфигурации 1С.
//!
//! Контейнер воспроизводит измеренную ленивую инициализацию через
//! `УстановкаПараметровСеанса`. Он принадлежит одному `open_bsl::State` и
//! обращается к модулю сеанса через фасадный мост существующей VM.

#![warn(missing_docs)]

mod messages;
mod planner_item;

mod client_application;
mod client_settings;
mod error_processing;
mod event_log;
mod factory;
mod globals;
mod infobase_sessions;
mod job_schedule;
mod library_tables;
mod parameters;
mod safe_mode;
mod session;
mod session_runtime;
mod system_information;

use client_application::*;
use client_settings::*;
use error_processing::*;
use event_log::*;
use globals::*;
use infobase_sessions::*;
use job_schedule::*;
use library_tables::*;
use parameters::*;
use safe_mode::*;
use system_information::*;

pub use client_application::{ClientConnectionSpeed, ClientDisplayInformation};
pub use client_settings::{
    ClientSettingsAdapter, ClientSettingsDeleteRequest, ClientSettingsDescription,
    ClientSettingsDescriptionRequest, ClientSettingsEntry, ClientSettingsListRequest,
    ClientSettingsLoadRequest, ClientSettingsSaveRequest, ClientSettingsSelectRequest,
    ClientSettingsStorage, HomePageFormsData,
};
pub use error_processing::{ErrorProcessingSettingsData, ErrorReportingMode};
pub use event_log::{
    EventLogEntry, EventLogEventUseData, EventLogLevel, EventLogTransactionMode, EventLogWriter,
};
pub use factory::{PrincipalCandidate, PrincipalSelectionError, SessionFactory, select_principal};
pub use infobase_sessions::InfoBaseConnection;
pub use parameters::{ParameterError, SessionParameters};
pub use session::{Session, SessionError};
pub use session_runtime::{ExternalResourcesMode, SessionKind, SessionResources, SessionRuntime};

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use bsl_config::{ConfigurationCodeSnapshot, ModuleKind};
use bsl_orm::{DataAccess, MetadataCatalog, MetadataKind, Principal, SessionData};
use open_bsl::{
    Arity, BslDate, BslObject, CallContext, ConstructorCode, ConstructorDescriptor, Engine,
    EnumValue, Error as OpenBslError, FunctionCode, FunctionDescriptor, FunctionKind,
    GlobalDescriptor, LibraryDescriptor, MethodDescriptor, ModuleGraphRecipe, ModuleInstance,
    ModuleRecipe, ObjectMembersDescriptor, ObjectProtocol, PropertyDescriptor, RtError, RtResult,
    State, TypeDescriptor, Value, background_job_type_descriptor, brief_error_description,
    detailed_error_description, fixed_array_from_values, folded_eq, value_list_from_values,
};

fn session_data<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionData> {
    ctx.host_data()?
        .downcast_ref::<SessionData>()
        .ok_or_else(|| RtError::DynamicError("данные сеанса не переданы".into()))
}

fn runtime<'a>(ctx: &'a CallContext<'_>) -> RtResult<&'a SessionRuntime> {
    session_data(ctx)?
        .extension::<SessionRuntime>()
        .ok_or_else(|| RtError::DynamicError("runtime сеанса не передан".into()))
}

/// Возвращает runtime-библиотеку параметров сеанса.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LIBRARY
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "factory_dynamic_common_modules_tests.rs"]
mod factory_dynamic_common_modules_tests;

#[cfg(test)]
#[path = "factory_separation_parameters_tests.rs"]
mod factory_separation_parameters_tests;

#[cfg(test)]
#[path = "settings_call_context_tests.rs"]
mod settings_call_context_tests;

#[cfg(test)]
#[path = "factory_kind_tests.rs"]
mod factory_kind_tests;

#[cfg(test)]
#[path = "session_initialization_privilege_tests.rs"]
mod session_initialization_privilege_tests;

#[cfg(test)]
#[path = "linked_query_parameter_tests.rs"]
mod linked_query_parameter_tests;

#[path = "source_context.rs"]
mod source_context;

#[cfg(test)]
#[path = "initialization_index_tests.rs"]
mod initialization_index_tests;

#[cfg(test)]
#[path = "file_source_session_tests.rs"]
mod file_source_session_tests;
