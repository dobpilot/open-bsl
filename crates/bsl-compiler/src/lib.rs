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

mod compiler;
mod dynamic;

pub use compiler::{
    CompileError, SnippetUnit, compile_configuration, compile_program, compile_snippet,
    compile_snippet_with_requirements,
};
pub use dynamic::compile_dynamic_snippet;
