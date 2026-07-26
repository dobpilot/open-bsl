use crate::keywords::Keyword;

/// Диапазон байтовых смещений в исходном тексте (не в UTF-16 единицах —
/// это позиция в лексере, а не в рантайм-строке BSL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident,
    Keyword(Keyword),
    /// Сырой текст числа (цифры и не более одной точки) — разбирается в
    /// `BslNumber` уровнем выше, лексер не считает.
    Number,
    /// Уже разэкранированное значение строкового литерала.
    String(String),
    /// Цифры внутри `'...'` без кавычек: 8 (дата) или 14 (дата+время) разрядов.
    Date(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Semicolon,
    Colon,
    Question,
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
