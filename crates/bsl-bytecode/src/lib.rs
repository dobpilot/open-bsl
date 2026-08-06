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

pub use chunk::{Chunk, ExceptionRange, Program, PropCacheSlot};
pub use compiler::{compile_program, compile_snippet, CompileError};
pub use instr::{ArgMode, Instr};
pub use text::{parse_program, write_program, TextError, FORMAT_VERSION, OPCODES};
