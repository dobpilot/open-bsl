use crate::lexer::LexError;
use crate::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    Lex(LexError),
    Parse(ParseError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Позиция — смещения В БАЙТАХ исходника: строки и колонки здесь
        // взять неоткуда, диагностика текста не хранит. Печатать её всё
        // равно обязательно — без неё сообщение о разборе бесполезно.
        write!(
            f,
            "{} (байты {}..{})",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Diagnostic::Lex(error) => write!(f, "{error}"),
            Diagnostic::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Diagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Diagnostic::Lex(error) => Some(error),
            Diagnostic::Parse(error) => Some(error),
        }
    }
}
