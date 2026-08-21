//! Устойчивый фасад для встраивания интерпретатора BSL в Rust-приложение.
//!
//! [`Engine`] собирает неизменяемый набор статических компонентов и
//! компилирует [`Module`]. [`State`] владеет сервисами конкретной сессии,
//! включая независимые потоки вывода.

mod engine;
mod error;
mod state;

pub use bsl_rt::BslValue as Value;
pub use bsl_rt::{
    Arity, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode, FunctionDescriptor,
    FunctionKind, LibraryDependency, LibraryDescriptor, MethodCall, MethodCode, MethodDescriptor,
    ObjectProtocol, RtError, RtResult, TypeDescriptor, call_method_from_table,
};

pub use engine::{Engine, EngineBuilder, Module};
pub use error::Error;
pub use state::{HostServices, State, StateBuilder};
