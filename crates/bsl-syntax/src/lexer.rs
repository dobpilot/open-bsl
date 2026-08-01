use crate::keywords;
use crate::token::{Span, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    UnexpectedChar(char, u32),
    UnterminatedString(u32),
    UnterminatedDate(u32),
    /// Многострочный литерал: следующая строка не начинается с `|`.
    ExpectedContinuationBar(u32),
    /// Внутри `'...'` встретился не-цифровой символ, либо длина не 8/14.
    BadDateLiteral(u32),
}

pub type LexResult<T> = Result<T, LexError>;

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Lexer { src, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn tok(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span {
                start: start as u32,
                end: self.pos as u32,
            },
        }
    }

    /// Следующий токен. `Eof` возвращается многократно после конца входа.
    ///
    /// # Errors
    ///
    /// Возвращает [`LexError`], если вход содержит недопустимый символ, незакрытый строковый
    /// литерал или некорректный литерал даты.
    pub fn next_token(&mut self) -> LexResult<Token> {
        self.skip_trivia();
        let start = self.pos;
        let c = match self.peek() {
            None => return Ok(self.tok(TokenKind::Eof, start)),
            Some(c) => c,
        };

        if is_ident_start(c) {
            return Ok(self.lex_ident_or_keyword(start));
        }
        if c.is_ascii_digit() {
            return Ok(self.lex_number(start));
        }
        match c {
            '"' => self.lex_string(start),
            '\'' => self.lex_date(start),
            '(' => {
                self.bump();
                Ok(self.tok(TokenKind::LParen, start))
            }
            ')' => {
                self.bump();
                Ok(self.tok(TokenKind::RParen, start))
            }
            '[' => {
                self.bump();
                Ok(self.tok(TokenKind::LBracket, start))
            }
            ']' => {
                self.bump();
                Ok(self.tok(TokenKind::RBracket, start))
            }
            ',' => {
                self.bump();
                Ok(self.tok(TokenKind::Comma, start))
            }
            '.' => {
                self.bump();
                Ok(self.tok(TokenKind::Dot, start))
            }
            ';' => {
                self.bump();
                Ok(self.tok(TokenKind::Semicolon, start))
            }
            ':' => {
                self.bump();
                Ok(self.tok(TokenKind::Colon, start))
            }
            '?' => {
                self.bump();
                Ok(self.tok(TokenKind::Question, start))
            }
            '+' => {
                self.bump();
                Ok(self.tok(TokenKind::Plus, start))
            }
            '-' => {
                self.bump();
                Ok(self.tok(TokenKind::Minus, start))
            }
            '*' => {
                self.bump();
                Ok(self.tok(TokenKind::Star, start))
            }
            '/' => {
                self.bump();
                Ok(self.tok(TokenKind::Slash, start))
            }
            '%' => {
                self.bump();
                Ok(self.tok(TokenKind::Percent, start))
            }
            '=' => {
                self.bump();
                Ok(self.tok(TokenKind::Eq, start))
            }
            '<' => {
                self.bump();
                match self.peek() {
                    Some('>') => {
                        self.bump();
                        Ok(self.tok(TokenKind::NotEq, start))
                    }
                    Some('=') => {
                        self.bump();
                        Ok(self.tok(TokenKind::Le, start))
                    }
                    _ => Ok(self.tok(TokenKind::Lt, start)),
                }
            }
            '>' => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    Ok(self.tok(TokenKind::Ge, start))
                } else {
                    Ok(self.tok(TokenKind::Gt, start))
                }
            }
            other => {
                self.bump();
                Err(LexError::UnexpectedChar(other, start as u32))
            }
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek2() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Token {
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.src[start..self.pos];
        match keywords::lookup(text) {
            Some(kw) => self.tok(TokenKind::Keyword(kw), start),
            None => self.tok(TokenKind::Ident, start),
        }
    }

    /// Целая или дробная часть, без экспоненты — в BSL нет литералов вида
    /// `1e10`, поэтому реальный код пишет константы дробями.
    fn lex_number(&mut self, start: usize) -> Token {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') && matches!(self.peek2(), Some(c) if c.is_ascii_digit()) {
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        self.tok(TokenKind::Number, start)
    }

    /// `"` ... `"`, где `""` — экранированная кавычка. Перенос строки внутри
    /// литерала обязан продолжаться строкой, начинающейся (после пробелов) с
    /// `|`; сам символ `|` и ведущие пробелы в значение не входят.
    fn lex_string(&mut self, start: usize) -> LexResult<Token> {
        self.bump(); // открывающая "
        let mut value = String::new();
        loop {
            match self.peek() {
                None => return Err(LexError::UnterminatedString(start as u32)),
                Some('"') => {
                    self.bump();
                    if self.peek() == Some('"') {
                        self.bump();
                        value.push('"');
                        continue;
                    }
                    return Ok(Token {
                        kind: TokenKind::String(value),
                        span: Span {
                            start: start as u32,
                            end: self.pos as u32,
                        },
                    });
                }
                Some('\n') => {
                    self.bump();
                    value.push('\n');
                    while matches!(self.peek(), Some(' ') | Some('\t')) {
                        self.bump();
                    }
                    if self.peek() != Some('|') {
                        return Err(LexError::ExpectedContinuationBar(self.pos as u32));
                    }
                    self.bump(); // '|'
                }
                Some(c) => {
                    self.bump();
                    value.push(c);
                }
            }
        }
    }

    /// `'YYYYMMDD'` или `'YYYYMMDDhhmmss'` — только цифры, 8 либо 14 разрядов.
    fn lex_date(&mut self, start: usize) -> LexResult<Token> {
        self.bump(); // открывающая '
        let mut digits = String::new();
        loop {
            match self.peek() {
                None => return Err(LexError::UnterminatedDate(start as u32)),
                Some('\'') => {
                    self.bump();
                    break;
                }
                Some(c) if c.is_ascii_digit() => {
                    self.bump();
                    digits.push(c);
                }
                Some(_) => return Err(LexError::BadDateLiteral(start as u32)),
            }
        }
        if digits.len() != 8 && digits.len() != 14 {
            return Err(LexError::BadDateLiteral(start as u32));
        }
        Ok(Token {
            kind: TokenKind::Date(digits),
            span: Span {
                start: start as u32,
                end: self.pos as u32,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::Keyword;

    fn lex_all(src: &str) -> LexResult<Vec<TokenKind>> {
        let mut lexer = Lexer::new(src);
        let mut out = Vec::new();
        loop {
            let tok = lexer.next_token()?;
            let done = tok.kind == TokenKind::Eof;
            out.push(tok.kind);
            if done {
                return Ok(out);
            }
        }
    }

    #[test]
    fn bilingual_keywords_resolve_to_same_variant() {
        assert_eq!(
            lex_all("Если Тогда КонецЕсли").unwrap(),
            lex_all("If Then EndIf").unwrap()
        );
        assert_eq!(
            lex_all("если").unwrap(),
            vec![TokenKind::Keyword(Keyword::If), TokenKind::Eof]
        );
    }

    #[test]
    fn identifiers_case_insensitive_lookup_but_plain_ident_otherwise() {
        assert_eq!(
            lex_all("ПеременнаяX").unwrap(),
            vec![TokenKind::Ident, TokenKind::Eof]
        );
    }

    #[test]
    fn numbers_no_exponent_notation() {
        assert_eq!(
            lex_all("1000000000000000019").unwrap(),
            vec![TokenKind::Number, TokenKind::Eof]
        );
        assert_eq!(
            lex_all("3.14").unwrap(),
            vec![TokenKind::Number, TokenKind::Eof]
        );
        // Точка без следующей цифры — это Dot, не часть числа (метод/поле).
        assert_eq!(
            lex_all("bodies.Count").unwrap(),
            vec![
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn string_literal_with_escaped_quote() {
        let toks = lex_all(r#""a""b""#).unwrap();
        assert_eq!(toks, vec![TokenKind::String("a\"b".to_string()), TokenKind::Eof]);
    }

    #[test]
    fn multiline_string_requires_continuation_bar() {
        let toks = lex_all("\"Строка1\n|Строка2\"").unwrap();
        assert_eq!(
            toks,
            vec![
                TokenKind::String("Строка1\nСтрока2".to_string()),
                TokenKind::Eof
            ]
        );

        let err = lex_all("\"Строка1\nСтрока2\"").unwrap_err();
        assert!(matches!(err, LexError::ExpectedContinuationBar(_)));
    }

    #[test]
    fn date_literal_requires_8_or_14_digits() {
        assert_eq!(
            lex_all("'20200101'").unwrap(),
            vec![TokenKind::Date("20200101".to_string()), TokenKind::Eof]
        );
        assert_eq!(
            lex_all("'20200101120000'").unwrap(),
            vec![
                TokenKind::Date("20200101120000".to_string()),
                TokenKind::Eof
            ]
        );
        assert!(matches!(
            lex_all("'2020'").unwrap_err(),
            LexError::BadDateLiteral(_)
        ));
    }

    #[test]
    fn line_comments_are_skipped() {
        assert_eq!(
            lex_all("1 // комментарий до конца строки\n2").unwrap(),
            vec![TokenKind::Number, TokenKind::Number, TokenKind::Eof]
        );
    }

    #[test]
    fn operators_and_punctuation() {
        assert_eq!(
            lex_all("<> <= >= < > = + - * / ( ) [ ] , . ; : ?").unwrap(),
            vec![
                TokenKind::NotEq,
                TokenKind::Le,
                TokenKind::Ge,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::Eq,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Semicolon,
                TokenKind::Colon,
                TokenKind::Question,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unexpected_char_is_an_error() {
        assert!(matches!(
            lex_all("@").unwrap_err(),
            LexError::UnexpectedChar('@', 0)
        ));
    }
}
