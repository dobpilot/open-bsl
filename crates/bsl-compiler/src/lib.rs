//! Кодоген BSL: семантическое представление `bsl-sema` — в байт-код.
//!
//! Крейт стоит между фронтендом и представлением: он зависит от
//! `bsl-syntax`, `bsl-sema` и `bsl-bytecode`, а `bsl-vm` — только от
//! последнего. Именно поэтому кодоген вынесен сюда: исполняющему слою
//! незачем видеть ни лексер, ни резолвер, и после разделения он их и не
//! видит.
//!
//! [`compile_program`] делает модуль целиком, [`compile_dynamic_snippet`] —
//! фрагмент `Выполнить`/`Вычислить` по описанию, которое VM передаёт хосту
//! через `bsl_bytecode::DynamicRequest`.

pub mod cfg;
mod compiler;
mod dynamic;

pub use compiler::{
    CompileError, Optimizations, SnippetUnit, compile_configuration, compile_entry_program,
    compile_program, compile_program_with, compile_snippet, compile_snippet_with_requirements,
};
pub use dynamic::compile_dynamic_snippet;
