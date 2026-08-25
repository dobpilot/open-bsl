use crate::keywords;
use crate::preproc::{self, Directive, PreprocSymbols};
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
    /// Ошибка в инструкции препроцессора: постоянный текст причины и
    /// смещение `#`.
    Preproc(&'static str, u32),
    /// Ошибка в директиве компиляции: постоянный текст причины и
    /// смещение `&`.
    Annotation(&'static str, u32),
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Число во всех вариантах — смещение в байтах от начала исходника.
        match self {
            LexError::UnexpectedChar(c, at) => {
                write!(f, "неожиданный символ «{c}» (байт {at})")
            }
            LexError::UnterminatedString(at) => {
                write!(f, "незакрытый строковый литерал (байт {at})")
            }
            LexError::UnterminatedDate(at) => write!(f, "незакрытый литерал даты (байт {at})"),
            LexError::ExpectedContinuationBar(at) => {
                write!(f, "продолжение строки обязано начинаться с «|» (байт {at})")
            }
            LexError::BadDateLiteral(at) => write!(f, "некорректный литерал даты (байт {at})"),
            LexError::Preproc(what, at) => {
                write!(f, "инструкция препроцессора: {what} (байт {at})")
            }
            LexError::Annotation(what, at) => {
                write!(f, "директива компиляции: {what} (байт {at})")
            }
        }
    }
}

impl std::error::Error for LexError {}

pub type LexResult<T> = Result<T, LexError>;

/// Открытый `#Если`: помним, взята ли уже какая-то его ветка, и где он
/// начался — для сообщения о незакрытой директиве.
struct OpenIf {
    taken: bool,
    at: usize,
}

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
    symbols: PreprocSymbols,
    open_ifs: Vec<OpenIf>,
    /// Смещения незакрытых `#Область`. Платформа парность отслеживает:
    /// и незакрытая область, и одинокий `#КонецОбласти` — ошибки
    /// компиляции (измерено).
    open_regions: Vec<usize>,
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self::with_symbols(src, PreprocSymbols::new())
    }

    /// Лексер с заданным набором символов условной компиляции.
    pub fn with_symbols(src: &'src str, symbols: PreprocSymbols) -> Self {
        Lexer {
            src,
            pos: 0,
            symbols,
            open_ifs: Vec::new(),
            open_regions: Vec::new(),
        }
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
        self.skip_trivia()?;
        let start = self.pos;
        let c = match self.peek() {
            None => {
                // Незакрытый `#Если` — ошибка компиляции и на платформе.
                if let Some(open) = self.open_ifs.first() {
                    return Err(LexError::Preproc(
                        "не закрыта инструкция «#Если»",
                        open.at as u32,
                    ));
                }
                if let Some(&at) = self.open_regions.first() {
                    return Err(LexError::Preproc("не закрыта область", at as u32));
                }
                return Ok(self.tok(TokenKind::Eof, start));
            }
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
            '~' => {
                self.bump();
                Ok(self.tok(TokenKind::Tilde, start))
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

    /// Пробелы, комментарии и инструкции препроцессора: для потока
    /// токенов всё это одинаково незначащий текст.
    fn skip_trivia(&mut self) -> LexResult<()> {
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
                Some('#') => self.directive()?,
                Some('&') => self.annotation()?,
                _ => break,
            }
        }
        Ok(())
    }

    /// Директива компиляции (`&НаСервере`, `&НаСервереБезКонтекста`,
    /// `&НаКлиентеНаСервереБезКонтекста`).
    ///
    /// ПРИНИМАЕТСЯ И ИГНОРИРУЕТСЯ. Эти директивы описывают границу
    /// «клиент — сервер»: где исполняется метод и едет ли с вызовом
    /// контекст формы. У open-bsl такой границы нет — процесс один, вызов
    /// никуда не передаётся, — поэтому переносить их семантику некуда, а
    /// отвергать значило бы не принимать исходники 1С как есть. Настоящее
    /// разделение встраивающая программа делает символами препроцессора,
    /// см. `docs/bsl-preproc.md`.
    ///
    /// Имя не проверяется по списку СОЗНАТЕЛЬНО. Список директив у
    /// платформы закрытый, но снять его стендом замеров нельзя: он сам
    /// подставляет `&НаСервере` перед каждым объявлением и чужую директиву
    /// туда не поставить. Список по памяти был бы неизмеренным
    /// утверждением, поэтому здесь принимается любое имя — мы мягче
    /// платформы, и это записано как расхождение, а не как совместимость.
    fn annotation(&mut self) -> LexResult<()> {
        let at = self.pos;
        self.bump(); // «&»
        let name_start = self.pos;
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        if self.pos == name_start {
            return Err(LexError::Annotation(
                "после «&» ожидается имя директивы компиляции",
                at as u32,
            ));
        }
        // Формы с аргументом (`&Вместо("Имя")`) — механика расширений
        // конфигурации, которых у open-bsl нет. Отказ здесь такой же
        // сознательный, как у `#Вставка`: молча пропустить `&Вместо`
        // опаснее, чем отказать, потому что смысл такого кода —
        // подменить чужой метод, а не просто выполниться.
        let tail = self.src[self.pos..].trim_start_matches([' ', '\t']);
        if tail.starts_with('(') {
            return Err(LexError::Annotation(
                "директивы компиляции расширений не поддержаны",
                at as u32,
            ));
        }
        Ok(())
    }

    /// Обрабатывает строку директивы в АКТИВНОМ тексте. Здесь строгость
    /// полная: неизвестная инструкция, пропущенное «Тогда», посторонний
    /// хвост и безымянная область — измеренные ошибки компиляции.
    fn directive(&mut self) -> LexResult<()> {
        let at = self.pos;
        let (word, rest, next) = preproc::split_line(self.src, at);
        let fail = |what: &'static str| LexError::Preproc(what, at as u32);
        if word.is_empty() {
            return Err(fail("после «#» ожидается имя инструкции препроцессора"));
        }
        if preproc::is_extension(word) {
            return Err(fail("инструкции расширений конфигурации не поддержаны"));
        }
        let directive =
            preproc::classify(word).ok_or_else(|| fail("неизвестная инструкция препроцессора"))?;
        self.pos = next;
        match directive {
            Directive::Region => {
                preproc::expect_region_name(rest).map_err(fail)?;
                self.open_regions.push(at);
            }
            Directive::EndRegion => {
                preproc::expect_empty_tail(rest).map_err(fail)?;
                if self.open_regions.pop().is_none() {
                    return Err(fail("«#КонецОбласти» без парного «#Область»"));
                }
            }
            Directive::If => {
                let taken = preproc::eval_condition(rest, &self.symbols).map_err(fail)?;
                self.open_ifs.push(OpenIf { taken, at });
                if !taken {
                    self.skip_to_branch(at)?;
                }
            }
            Directive::ElsIf | Directive::Else => {
                // Досюда доходит только АКТИВНАЯ ветка: выключенные
                // пропускает скан. Значит своё этот `#Если` уже отработал,
                // и остаток блока выключен целиком.
                if self.open_ifs.is_empty() {
                    return Err(fail("инструкция без парного «#Если»"));
                }
                if directive == Directive::Else {
                    preproc::expect_empty_tail(rest).map_err(fail)?;
                }
                self.skip_to_endif(at)?;
            }
            Directive::EndIf => {
                preproc::expect_empty_tail(rest).map_err(fail)?;
                if self.open_ifs.pop().is_none() {
                    return Err(fail("«#КонецЕсли» без парного «#Если»"));
                }
            }
        }
        Ok(())
    }

    /// Начало ближайшей следующей строки, у которой первый непробельный
    /// знак — `#`.
    fn next_directive_line(&self) -> Option<usize> {
        let mut i = self.pos;
        loop {
            let nl = self.src[i..].find('\n')?;
            i += nl + 1;
            let tail = self.src[i..].trim_start_matches([' ', '\t', '\r']);
            if tail.starts_with('#') {
                return Some(self.src.len() - tail.len());
            }
        }
    }

    /// Пропускает выключенный текст до ветки этого же `#Если`, которую
    /// надо включить, либо до его `#КонецЕсли`.
    ///
    /// Мёртвый текст НЕ ЛЕКСИРУЕТСЯ и не разбирается: измерено, что
    /// платформа терпит там не-токены. Поэтому строки, начинающиеся с `#`,
    /// но не опознанные как директива, здесь тоже просто пропускаются.
    fn skip_to_branch(&mut self, from: usize) -> LexResult<()> {
        let mut depth = 0usize;
        loop {
            let Some(at) = self.next_directive_line() else {
                return Err(LexError::Preproc(
                    "не закрыта инструкция «#Если»",
                    from as u32,
                ));
            };
            let (word, rest, next) = preproc::split_line(self.src, at);
            self.pos = next;
            match preproc::classify(word) {
                Some(Directive::If) => depth += 1,
                Some(Directive::EndIf) if depth > 0 => depth -= 1,
                Some(Directive::EndIf) => {
                    self.open_ifs.pop();
                    return Ok(());
                }
                Some(Directive::ElsIf) if depth == 0 => {
                    let already = self.open_ifs.last().is_some_and(|o| o.taken);
                    if !already
                        && preproc::eval_condition(rest, &self.symbols)
                            .map_err(|w| LexError::Preproc(w, at as u32))?
                    {
                        if let Some(top) = self.open_ifs.last_mut() {
                            top.taken = true;
                        }
                        return Ok(());
                    }
                }
                Some(Directive::Else) if depth == 0 => {
                    if let Some(top) = self.open_ifs.last_mut()
                        && !top.taken
                    {
                        top.taken = true;
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    /// Пропускает остаток условного блока до его `#КонецЕсли`, не
    /// рассматривая промежуточные ветки.
    fn skip_to_endif(&mut self, from: usize) -> LexResult<()> {
        let mut depth = 0usize;
        loop {
            let Some(at) = self.next_directive_line() else {
                return Err(LexError::Preproc(
                    "не закрыта инструкция «#Если»",
                    from as u32,
                ));
            };
            let (word, _, next) = preproc::split_line(self.src, at);
            self.pos = next;
            match preproc::classify(word) {
                Some(Directive::If) => depth += 1,
                Some(Directive::EndIf) if depth > 0 => depth -= 1,
                Some(Directive::EndIf) => {
                    self.open_ifs.pop();
                    return Ok(());
                }
                _ => {}
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
        assert_eq!(
            toks,
            vec![TokenKind::String("a\"b".to_string()), TokenKind::Eof]
        );
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
            lex_all("<> <= >= < > = + - * / ( ) [ ] , . ; : ~ ?").unwrap(),
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
                TokenKind::Tilde,
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

    // --- Инструкции препроцессора -------------------------------------
    //
    // Каждый тест ниже пришпилен к строке из `docs/bsl-preproc.md`, снятой
    // на 8.3.27.2130. Где поведение выбрано нами, а не платформой, это
    // сказано прямо.

    fn idents(src: &str) -> Vec<String> {
        let mut lexer = Lexer::new(src);
        let mut out = Vec::new();
        loop {
            let tok = lexer.next_token().expect("лексер не должен падать");
            match tok.kind {
                TokenKind::Eof => return out,
                TokenKind::String(v) => out.push(v),
                _ => {}
            }
        }
    }

    #[test]
    fn a_true_condition_keeps_the_branch_and_a_false_one_drops_it() {
        let src = "#Если Сервер Тогда\n\"да\"\n#КонецЕсли\n#Если Клиент Тогда\n\"нет\"\n#КонецЕсли";
        assert_eq!(idents(src), vec!["да".to_string()]);
    }

    #[test]
    fn elsif_and_else_pick_exactly_one_branch() {
        let src =
            "#Если Клиент Тогда\n\"к\"\n#ИначеЕсли Сервер Тогда\n\"с\"\n#Иначе\n\"и\"\n#КонецЕсли";
        assert_eq!(idents(src), vec!["с".to_string()]);
        let src = "#Если Клиент Тогда\n\"к\"\n#ИначеЕсли ВебКлиент Тогда\n\"в\"\n#Иначе\n\"и\"\n#КонецЕсли";
        assert_eq!(idents(src), vec!["и".to_string()]);
    }

    #[test]
    fn conditionals_nest() {
        let src = "#Если Сервер Тогда\n#Если Клиент Тогда\n\"нет\"\n#Иначе\n\"да\"\n#КонецЕсли\n#КонецЕсли";
        assert_eq!(idents(src), vec!["да".to_string()]);
    }

    #[test]
    fn a_directive_can_split_an_expression() {
        // Измерено: платформа даёт «аб», то есть директива режет выражение,
        // а не только оператор.
        let src = "\"а\"\n#Если Сервер Тогда\n+ \"б\"\n#КонецЕсли\n;";
        let toks = lex_all(src).unwrap();
        assert_eq!(
            toks,
            vec![
                TokenKind::String("а".to_string()),
                TokenKind::Plus,
                TokenKind::String("б".to_string()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn a_disabled_branch_need_not_be_lexable() {
        // Измерено: платформа терпит в выключенной ветке не-токены.
        let src = "#Если Клиент Тогда\n@ ` ~ вообще не токены\n#КонецЕсли\n\"после\"";
        assert_eq!(idents(src), vec!["после".to_string()]);
    }

    #[test]
    fn an_unknown_symbol_is_false_and_not_an_error() {
        let src = "#Если ЛютоНеизвестныйСимвол Тогда\n\"нет\"\n#Иначе\n\"да\"\n#КонецЕсли";
        assert_eq!(idents(src), vec!["да".to_string()]);
    }

    #[test]
    fn directive_and_symbol_names_ignore_case_including_cyrillic() {
        let src = "#если сЕрВеР тогда\n\"да\"\n#конецесли";
        assert_eq!(idents(src), vec!["да".to_string()]);
    }

    #[test]
    fn english_directives_and_symbols_work() {
        let src = "#If Server Then\n\"да\"\n#Else\n\"нет\"\n#EndIf";
        assert_eq!(idents(src), vec!["да".to_string()]);
        let src = "#Region Проба\n\"да\"\n#EndRegion";
        assert_eq!(idents(src), vec!["да".to_string()]);
    }

    #[test]
    fn regions_are_skipped_but_need_a_name() {
        let src = "#Область Служебные\n\"да\"\n#КонецОбласти";
        assert_eq!(idents(src), vec!["да".to_string()]);
        assert!(matches!(
            lex_all("#Область\n\"да\"\n#КонецОбласти"),
            Err(LexError::Preproc(..))
        ));
    }

    #[test]
    fn a_comment_may_follow_a_directive_but_other_text_may_not() {
        let src = "#Если Сервер Тогда // хвост\n\"да\"\n#КонецЕсли";
        assert_eq!(idents(src), vec!["да".to_string()]);
        assert!(matches!(
            lex_all("#Если Сервер Тогда\n\"да\"\n#КонецЕсли лишнее"),
            Err(LexError::Preproc(..))
        ));
    }

    #[test]
    fn the_measured_directive_errors_are_errors_here_too() {
        for src in [
            "#Если Сервер\n\"да\"\n#КонецЕсли",         // пропущено «Тогда»
            "#Если Сервер Тогда\n\"да\"",               // не закрыт
            "#КонецЕсли",                               // без парного «#Если»
            "#Вставка\n\"да\"\n#КонецВставки",          // директива расширения
            "#ЧтоТоНеизвестное\n\"да\"",                // не инструкция вовсе
            "#Если ( Сервер Тогда\n\"да\"\n#КонецЕсли", // не закрыта скобка
            "#Если И Сервер Тогда\n\"да\"\n#КонецЕсли", // нет операнда
        ] {
            assert!(
                matches!(lex_all(src), Err(LexError::Preproc(..))),
                "должно быть ошибкой: {src}"
            );
        }
    }

    #[test]
    fn logical_operators_and_parentheses_work() {
        let cases = [
            ("Сервер И НЕ Клиент", true),
            ("Сервер И Клиент", false),
            ("Клиент ИЛИ Сервер", true),
            ("(Клиент ИЛИ ВебКлиент)", false),
            ("НЕ (Клиент И Сервер)", true),
            ("ВнешнееСоединение", true),
        ];
        for (cond, want) in cases {
            let src = format!("#Если {cond} Тогда\n\"да\"\n#КонецЕсли");
            assert_eq!(
                idents(&src) == vec!["да".to_string()],
                want,
                "условие: {cond}"
            );
        }
    }

    #[test]
    fn a_hash_inside_a_string_literal_is_not_a_directive() {
        // Продолжение литерала обязано начинаться с `|`, поэтому строки,
        // начинающейся с `#`, внутри литерала не бывает — но сам знак `#`
        // в тексте литерала законен.
        let toks = lex_all("\"первая\n|#КонецЕсли внутри строки\"").unwrap();
        assert_eq!(
            toks,
            vec![
                TokenKind::String("первая\n#КонецЕсли внутри строки".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn the_host_may_redefine_the_symbol_set() {
        let mut symbols = PreprocSymbols::new();
        symbols.set("Клиент", true);
        symbols.set("Сервер", false);
        let mut lexer = Lexer::with_symbols(
            "#Если Клиент И НЕ Сервер Тогда\n\"да\"\n#КонецЕсли",
            symbols,
        );
        let mut got = None;
        loop {
            let tok = lexer.next_token().unwrap();
            match tok.kind {
                TokenKind::Eof => break,
                TokenKind::String(v) => got = Some(v),
                _ => {}
            }
        }
        assert_eq!(got.as_deref(), Some("да"));
        // Английское написание — тот же символ, а не второй.
        let mut symbols = PreprocSymbols::new();
        symbols.set("Client", true);
        assert!(symbols.is_on("Клиент"));
    }

    #[test]
    fn a_region_name_is_one_identifier() {
        // Измерено: платформа берёт весь хвост строки и проверяет его как
        // ОДИН идентификатор — указатель ошибки стоит в начале имени, а не
        // на точке.
        for bad in [
            "#Область Имя.Точка\n\"да\"\n#КонецОбласти",
            "#Область 1Имя\n\"да\"\n#КонецОбласти",
            "#Область Две Слова\n\"да\"\n#КонецОбласти",
        ] {
            assert!(
                matches!(lex_all(bad), Err(LexError::Preproc(..))),
                "должно быть ошибкой: {bad}"
            );
        }
        for good in [
            "#Область Имя_Второе2\n\"да\"\n#КонецОбласти",
            "#Область LatinName\n\"да\"\n#КонецОбласти",
        ] {
            assert_eq!(
                idents(good),
                vec!["да".to_string()],
                "должно приниматься: {good}"
            );
        }
    }

    #[test]
    fn regions_must_be_balanced() {
        // Обе формы — измеренные ошибки компиляции; у одинокого закрытия
        // платформа говорит «Пропущен оператор препроцессора Область».
        assert!(matches!(
            lex_all("#Область Незакрытая\n\"да\""),
            Err(LexError::Preproc(..))
        ));
        assert!(matches!(
            lex_all("\"да\"\n#КонецОбласти"),
            Err(LexError::Preproc(..))
        ));
        // Вложенные области при этом законны.
        assert_eq!(
            idents("#Область Внешняя\n#Область Внутренняя\n\"да\"\n#КонецОбласти\n#КонецОбласти"),
            vec!["да".to_string()]
        );
    }

    // --- Директивы компиляции ------------------------------------------

    #[test]
    fn compilation_directives_are_accepted_and_produce_no_tokens() {
        // Границы «клиент — сервер» у open-bsl нет, переносить семантику
        // некуда: директива принимается и выбрасывается, как пробел.
        let toks = lex_all("&НаСервереБезКонтекста\n\"да\"").unwrap();
        assert_eq!(
            toks,
            vec![TokenKind::String("да".to_string()), TokenKind::Eof]
        );
        assert_eq!(
            idents("&НаКлиентеНаСервереБезКонтекста\n\"да\""),
            vec!["да".to_string()]
        );
        // Имя по списку не проверяется — см. комментарий у `annotation`.
        assert_eq!(idents("&ЧтоУгодно\n\"да\""), vec!["да".to_string()]);
    }

    #[test]
    fn extension_annotations_and_a_bare_ampersand_are_errors() {
        // `&Вместо("Имя")` подменяет чужой метод — молча пропустить такое
        // опаснее, чем отказать, ровно как с `#Вставка`.
        for src in [
            "&Вместо(\"Чужой\")\n\"да\"",
            "&Перед(\"Чужой\")\n\"да\"",
            "& \n\"да\"",
        ] {
            assert!(
                matches!(lex_all(src), Err(LexError::Annotation(..))),
                "должно быть ошибкой: {src}"
            );
        }
    }
}
