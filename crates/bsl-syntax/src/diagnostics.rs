use crate::keywords::Keyword;
use crate::lexer::LexError;
use crate::token::{Span, TokenKind};

/// Ошибка фазы разбора: ВИД плюс позиция. `ParseError` — единственная фаза
/// конвейера, которая до сих пор несла ошибку строкой; теперь у неё, как и
/// у соседей (`LexError`, `SemaError`, `CompileError`), типизированный вид.
/// Имя варианта — часть публичного текста диагностики (вся проза проекта
/// русская), а `span` остаётся ДАННЫМИ, как и раньше.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

/// Что именно не так с разбором.
///
/// Варианта про предел глубины ЦЕПОЧКИ здесь нет: `SYNTAX.CHAIN_DEPTH` ещё
/// не измерен, а заводить вид ошибки раньше проверки, которая его
/// порождает, — это обещание в типе. `NestingTooDeep` — про уже
/// существующий предел вложенности выражений и операторов
/// (`SYNTAX.MAX_NESTING`), который проверка действительно ставит.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Ожидалось одно, встретилось другое.
    Expected {
        what: Expectation,
        found: FoundToken,
    },
    /// Слева от «=» — не переменная, не элемент по индексу и не поле.
    BadAssignTarget,
    /// Цель присваивания заключена в скобки — платформа их слева от «=» не
    /// принимает ни в одной форме (измерено, см. `parse_simple_stmt`).
    ParenthesizedTarget,
    /// Превышен предел вложенности выражений и операторов.
    NestingTooDeep { limit: u32 },
}

/// Чего ждал разбор в этой позиции.
#[derive(Debug, Clone, PartialEq)]
pub enum Expectation {
    /// Конкретный токен — по большей части пунктуация и структурные скобки.
    Token(TokenKind),
    /// Конкретное ключевое слово.
    Keyword(Keyword),
    /// Идентификатор.
    Identifier,
    /// Имя члена после точки.
    MemberName,
    /// Начало выражения.
    Expression,
    /// Имя метки после `~`: идентификатор, ключевое слово или целое число.
    LabelName,
}

/// КАТЕГОРИЯ встреченного токена — без его содержимого. Идентификатор и
/// литералы приходят из исходника, и печатать их текст в ошибке нельзя:
/// она уходит в журналы хоста. Ключевые слова и пунктуация — токены самого
/// языка, а не содержимое исходника, поэтому их писать можно.
#[derive(Debug, Clone, PartialEq)]
pub enum FoundToken {
    Keyword(Keyword),
    Identifier,
    NumberLiteral,
    StringLiteral,
    DateLiteral,
    /// Пунктуация или оператор — фиксированный токен языка без содержимого.
    Symbol(TokenKind),
    Eof,
}

impl FoundToken {
    /// Категория токена без утечки содержимого литерала: `String`/`Date`
    /// теряют текст, `Ident` — имя, всё это данные исходника. Пунктуация
    /// попадает в `Symbol`, где содержимого нет по устройству токена.
    pub fn of(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::Ident => FoundToken::Identifier,
            TokenKind::Keyword(keyword) => FoundToken::Keyword(*keyword),
            TokenKind::Number => FoundToken::NumberLiteral,
            TokenKind::String(_) => FoundToken::StringLiteral,
            TokenKind::Date(_) => FoundToken::DateLiteral,
            TokenKind::Eof => FoundToken::Eof,
            other => FoundToken::Symbol(other.clone()),
        }
    }
}

/// Глиф пунктуации или оператора для диагностики. Только для токенов без
/// содержимого; на литерал или идентификатор не зовётся.
fn symbol_text(kind: &TokenKind) -> &'static str {
    match kind {
        TokenKind::LParen => "«(»",
        TokenKind::RParen => "«)»",
        TokenKind::LBracket => "«[»",
        TokenKind::RBracket => "«]»",
        TokenKind::Comma => "«,»",
        TokenKind::Dot => "«.»",
        TokenKind::Semicolon => "«;»",
        TokenKind::Colon => "«:»",
        TokenKind::Tilde => "«~»",
        TokenKind::Question => "«?»",
        TokenKind::Plus => "«+»",
        TokenKind::Minus => "«-»",
        TokenKind::Star => "«*»",
        TokenKind::Slash => "«/»",
        TokenKind::Percent => "«%»",
        TokenKind::Eq => "«=»",
        TokenKind::NotEq => "«<>»",
        TokenKind::Lt => "«<»",
        TokenKind::Gt => "«>»",
        TokenKind::Le => "«<=»",
        TokenKind::Ge => "«>=»",
        // Содержательные токены сюда не приходят: `symbol_text` зовётся
        // только на пунктуации.
        TokenKind::Ident
        | TokenKind::Keyword(_)
        | TokenKind::Number
        | TokenKind::String(_)
        | TokenKind::Date(_)
        | TokenKind::Eof => "токен",
    }
}

impl std::fmt::Display for Expectation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expectation::Token(kind) => f.write_str(symbol_text(kind)),
            Expectation::Keyword(keyword) => write!(f, "ключевое слово «{}»", keyword.spelling()),
            Expectation::Identifier => f.write_str("идентификатор"),
            Expectation::MemberName => f.write_str("имя члена после точки"),
            Expectation::Expression => f.write_str("выражение"),
            Expectation::LabelName => f.write_str("имя метки"),
        }
    }
}

impl std::fmt::Display for FoundToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FoundToken::Keyword(keyword) => write!(f, "ключевое слово «{}»", keyword.spelling()),
            FoundToken::Identifier => f.write_str("идентификатор"),
            FoundToken::NumberLiteral => f.write_str("числовой литерал"),
            FoundToken::StringLiteral => f.write_str("строковый литерал"),
            FoundToken::DateLiteral => f.write_str("литерал даты"),
            FoundToken::Symbol(kind) => f.write_str(symbol_text(kind)),
            FoundToken::Eof => f.write_str("конец текста"),
        }
    }
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::Expected { what, found } => {
                write!(f, "ожидается {what}, встречено {found}")
            }
            ParseErrorKind::BadAssignTarget => {
                f.write_str("слева от «=» ожидались переменная, элемент по индексу или поле")
            }
            ParseErrorKind::ParenthesizedTarget => {
                f.write_str("цель присваивания не может быть заключена в скобки")
            }
            ParseErrorKind::NestingTooDeep { limit } => {
                write!(f, "превышена максимальная глубина вложенности ({limit})")
            }
        }
    }
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
            self.kind, self.span.start, self.span.end
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
