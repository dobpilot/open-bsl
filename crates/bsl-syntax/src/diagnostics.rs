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
