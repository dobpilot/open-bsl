//! Байт-код, компилятор и текстовый формат программ BSL.
//!
//! [`compile_program`] преобразует семантическое представление в [`Program`].
//! [`write_program`] и [`parse_program`] задают единый текстовый формат для
//! `bsl-cli --emit-bytecode` и `bsl-cli --run-bytecode`.
//! [`dynamic`] описывает границу между VM и хостом для
//! `Выполнить`/`Вычислить`: VM их исполняет, а компилирует хост.

pub mod bundle;
mod chunk;
mod compiler;
pub mod dynamic;
mod instr;
mod text;

pub use bsl_rt::LibraryRequirement;
pub use chunk::{Chunk, ExceptionRange, MethodCacheSlot, Program, PropCacheSlot};
pub use compiler::{
    CompileError, compile_program, compile_snippet, compile_snippet_with_requirements,
};
pub use dynamic::{
    DynamicCompiler, DynamicKind, DynamicRequest, DynamicScope, DynamicSignature, DynamicUnit,
    compile_dynamic_snippet,
};
pub use instr::{ArgMode, Instr};
pub use text::{FORMAT_VERSION, OPCODES, TextError, parse_program, write_program};
