//! Лексический и синтаксический анализ BSL.
//!
//! Крейт преобразует исходный текст в AST и сообщает об ошибках с точными
//! байтовыми диапазонами в исходнике. Основная точка входа — [`parse`].

mod ast;
mod diagnostics;
mod keywords;
mod lexer;
mod lines;
mod parser;
mod preproc;
mod token;

pub use ast::*;
pub use diagnostics::{Diagnostic, Expectation, FoundToken, ParseError, ParseErrorKind};
pub use keywords::{Keyword, SPELLINGS as KEYWORD_SPELLINGS, lookup as lookup_keyword};
pub use lexer::{LexError, LexResult, Lexer};
pub use lines::LineIndex;
pub use parser::{parse, parse_with_symbols};
pub use preproc::PreprocSymbols;
pub use token::{Span, Token, TokenKind};
