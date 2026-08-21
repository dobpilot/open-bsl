//! Ошибка публичного конвейера фасада.

use std::fmt;

/// Ошибка одной из фаз публичного конвейера.
#[derive(Debug)]
pub enum Error {
    Parse(bsl_syntax::Diagnostic),
    Semantic(bsl_sema::SemaError),
    Compile(bsl_bytecode::CompileError),
    Registry(bsl_rt::RegistryError),
    Runtime(bsl_rt::RtError),
    Bytecode(bsl_bytecode::TextError),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Каждая фаза печатается своим `Display`, а не `Debug`. Половина
        // из них печаталась отладочным представлением, и наружу выходило
        // «ошибка синтаксиса: Parse(ParseError { message: "...", span:
        // Span { start: 3, end: 4 } })» — читаемое разработчиком этого
        // крейта и никем больше. Позиция при этом не потерялась: её несёт
        // `Display` самой диагностики.
        match self {
            Self::Parse(error) => write!(formatter, "ошибка синтаксиса: {error}"),
            Self::Semantic(error) => write!(formatter, "ошибка семантики: {error}"),
            Self::Compile(error) => write!(formatter, "ошибка компиляции: {error}"),
            Self::Registry(error) => write!(formatter, "ошибка компонентов: {error}"),
            Self::Runtime(error) => write!(formatter, "ошибка исполнения: {error}"),
            Self::Bytecode(error) => write!(formatter, "ошибка байт-кода: {error}"),
        }
    }
}

impl std::error::Error for Error {
    /// Вложенная ошибка фазы — чтобы host-приложение могло дойти до неё
    /// цепочкой `source()`, не разбирая текст и не сопоставляясь с
    /// вариантами этого перечисления.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Semantic(error) => Some(error),
            Self::Compile(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Bytecode(error) => Some(error),
        }
    }
}

impl From<bsl_syntax::Diagnostic> for Error {
    fn from(error: bsl_syntax::Diagnostic) -> Self {
        Self::Parse(error)
    }
}

impl From<bsl_sema::SemaError> for Error {
    fn from(error: bsl_sema::SemaError) -> Self {
        Self::Semantic(error)
    }
}

impl From<bsl_bytecode::CompileError> for Error {
    fn from(error: bsl_bytecode::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<bsl_rt::RegistryError> for Error {
    fn from(error: bsl_rt::RegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<bsl_rt::RtError> for Error {
    fn from(error: bsl_rt::RtError) -> Self {
        Self::Runtime(error)
    }
}

impl From<bsl_bytecode::TextError> for Error {
    fn from(error: bsl_bytecode::TextError) -> Self {
        Self::Bytecode(error)
    }
}
