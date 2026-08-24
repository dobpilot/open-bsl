//! Представление байт-кода BSL и его текстовый формат.
//!
//! Здесь только то, что нужно ОБЕИМ сторонам исполнения: [`Instr`],
//! [`Chunk`] и [`Program`], разметка VLIW-бандлов ([`bundle`]) и единый
//! текстовый формат [`write_program`]/[`parse_program`] для
//! `bsl-cli --emit-bytecode` и `bsl-cli --run-bytecode`. Кодогена здесь
//! нет: он живёт в `bsl-compiler`, который зависит от этого крейта, — так
//! `bsl-vm` видит представление, не видя фронтенда.
//!
//! [`dynamic`] описывает границу между VM и хостом для
//! `Выполнить`/`Вычислить`: VM их исполняет, а компилирует хост.

pub mod bundle;
mod chunk;
pub mod dynamic;
pub mod image;
mod instr;
mod text;

pub use bsl_rt::LibraryRequirement;
pub use chunk::{Chunk, ExceptionRange, MethodCacheSlot, Program, PropCacheSlot, SnippetUnit};
pub use dynamic::{
    DynamicCompiler, DynamicKind, DynamicRequest, DynamicScope, DynamicSignature, DynamicUnit,
};
pub use instr::{ArgMode, Instr};
pub use text::{FORMAT_VERSION, OPCODES, TextError, parse_program, write_program};
