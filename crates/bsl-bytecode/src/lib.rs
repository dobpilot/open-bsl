//! Байт-код, компилятор и текстовый формат программ BSL.
//!
//! [`compile_program`] преобразует семантическое представление в [`Program`].
//! [`write_program`] и [`parse_program`] задают единый текстовый формат для
//! `bsl-cli --emit-bytecode` и `bsl-cli --run-bytecode`.

pub mod bundle;
mod chunk;
mod compiler;
mod instr;
mod text;

pub use bsl_rt::LibraryRequirement;
pub use chunk::{Chunk, ExceptionRange, Program, PropCacheSlot};
pub use compiler::{
    CompileError, compile_program, compile_snippet, compile_snippet_with_requirements,
};
pub use instr::{ArgMode, Instr};
pub use text::{FORMAT_VERSION, OPCODES, TextError, parse_program, write_program};
