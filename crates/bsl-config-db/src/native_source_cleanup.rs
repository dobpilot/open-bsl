//! Консервативное упрощение уже полностью восстановленного текста.
//!
//! Лексер проекта отделяет код от строк и комментариев. Этот проход не
//! восстанавливает неизвестные команды и не принимает частичный модуль.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use bsl_syntax::{Keyword, Lexer, Token, TokenKind};

struct Edit {
    range: Range<usize>,
    text: String,
}

fn tokens(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer
            .next_token()
            .map_err(|error| format!("image/source: {error}"))?;
        if token.kind == TokenKind::Eof {
            return Ok(tokens);
        }
        tokens.push(token);
    }
}

fn spelling<'a>(source: &'a str, token: &Token) -> &'a str {
    &source[token.span.start as usize..token.span.end as usize]
}

fn key(source: &str, token: &Token) -> String {
    // Полная стандартная свёртка совпадает с медленным путём `folded_eq`.
    spelling(source, token).to_uppercase()
}

fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| edit.range.start);
    let mut output = String::new();
    let mut at = 0;
    for edit in edits {
        output.push_str(&source[at..edit.range.start]);
        output.push_str(&edit.text);
        at = edit.range.end;
    }
    output.push_str(&source[at..]);
    output
}

fn line_range(source: &str, start: usize, end: usize) -> Range<usize> {
    let line_start = source[..start].rfind('\n').map_or(0, |at| at + 1);
    let line_end = source[end..].find('\n').map_or(source.len(), |at| end + at);
    if source[line_start..start].trim().is_empty() && source[end..line_end].trim().is_empty() {
        line_start..(line_end + usize::from(line_end < source.len()))
    } else {
        start..end
    }
}

fn fold_booleans(source: &str) -> Result<String, String> {
    use Keyword::{False, Not, True};
    use TokenKind::{Eq, Keyword as K, LParen, RParen};
    let tokens = tokens(source)?;
    let mut edits = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let rest = &tokens[at..];
        let found = match rest {
            [a, b, c, d, e, ..] if a.kind == LParen && c.kind == Eq && e.kind == RParen => {
                match (&b.kind, &d.kind) {
                    (K(False), K(False)) | (K(True), K(True)) => Some((5, "Истина")),
                    (K(False), K(True)) => Some((5, "Ложь")),
                    _ => None,
                }
            }
            _ => None,
        }
        .or_else(|| match rest {
            [a, b, c, d, ..]
                if a.kind == LParen
                    && b.kind == K(Not)
                    && c.kind == K(True)
                    && d.kind == RParen =>
            {
                Some((4, "Ложь"))
            }
            _ => None,
        });
        if let Some((count, text)) = found {
            let range = tokens[at].span.start as usize..tokens[at + count - 1].span.end as usize;
            if !source[range.clone()].contains("//") {
                edits.push(Edit {
                    range,
                    text: text.into(),
                });
            }
            at += count;
        } else {
            at += 1;
        }
    }
    Ok(apply(source, edits))
}

fn remove_fallthrough(source: &str) -> Result<String, String> {
    let tokens = tokens(source)?;
    let mut refs = HashMap::<String, usize>::new();
    let mut jumps = Vec::new();
    for (at, group) in tokens.windows(4).enumerate() {
        if group[0].kind == TokenKind::Keyword(Keyword::Goto)
            && group[1].kind == TokenKind::Tilde
            && group[2].kind == TokenKind::Ident
            && group[3].kind == TokenKind::Semicolon
        {
            let name = key(source, &group[2]);
            *refs.entry(name.clone()).or_default() += 1;
            jumps.push((at, name));
        }
    }
    let mut edits = Vec::new();
    let mut candidates = HashSet::new();
    for (at, name) in jumps {
        if let Some(next) = tokens.get(at + 4..at + 7)
            && next[0].kind == TokenKind::Tilde
            && next[1].kind == TokenKind::Ident
            && next[2].kind == TokenKind::Colon
            && spelling(source, &next[1]).starts_with("__НативныйБлок_")
            && key(source, &next[1]) == name
            && source[tokens[at + 3].span.end as usize..next[0].span.start as usize]
                .trim()
                .is_empty()
        {
            edits.push(Edit {
                range: line_range(
                    source,
                    tokens[at].span.start as usize,
                    tokens[at + 3].span.end as usize,
                ),
                text: String::new(),
            });
            *refs.get_mut(&name).expect("переход учтён выше") -= 1;
            candidates.insert(name);
        }
    }
    for group in tokens.windows(3) {
        if group[0].kind == TokenKind::Tilde
            && group[1].kind == TokenKind::Ident
            && group[2].kind == TokenKind::Colon
        {
            let name = key(source, &group[1]);
            if candidates.contains(&name) && refs[&name] == 0 {
                edits.push(Edit {
                    range: line_range(
                        source,
                        group[0].span.start as usize,
                        group[2].span.end as usize,
                    ),
                    text: String::new(),
                });
            }
        }
    }
    Ok(apply(source, edits))
}

fn scopes(tokens: &[Token]) -> Vec<Range<usize>> {
    let mut scopes = Vec::new();
    let mut body = None;
    for (at, token) in tokens.iter().enumerate() {
        // Ключевое слово после точки обозначает член объекта, а не границу метода.
        if at > 0 && tokens[at - 1].kind == TokenKind::Dot {
            continue;
        }
        match token.kind {
            TokenKind::Keyword(Keyword::Procedure | Keyword::Function) => {
                let mut depth = 0;
                for (offset, token) in tokens[at + 1..].iter().enumerate() {
                    match token.kind {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => {
                            depth -= 1;
                            if depth == 0 {
                                body = Some(at + offset + 2);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            TokenKind::Keyword(Keyword::EndProcedure | Keyword::EndFunction) => {
                if let Some(start) = body.take() {
                    scopes.push(start..at);
                }
            }
            _ => {}
        }
    }
    scopes
}

fn pure_constant(mut tokens: &[Token]) -> bool {
    while tokens.len() >= 2
        && tokens[0].kind == TokenKind::LParen
        && tokens[tokens.len() - 1].kind == TokenKind::RParen
    {
        tokens = &tokens[1..tokens.len() - 1];
    }
    match tokens {
        [token] => matches!(
            token.kind,
            TokenKind::Number
                | TokenKind::String(_)
                | TokenKind::Date(_)
                | TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::Undefined)
        ),
        [sign, number] if matches!(sign.kind, TokenKind::Plus | TokenKind::Minus) => {
            number.kind == TokenKind::Number
        }
        [not, value] => {
            not.kind == TokenKind::Keyword(Keyword::Not)
                && matches!(
                    value.kind,
                    TokenKind::Keyword(Keyword::True | Keyword::False)
                )
        }
        _ => false,
    }
}

fn remove_unused(source: &str) -> Result<String, String> {
    let tokens = tokens(source)?;
    let scopes = scopes(&tokens);
    let mut module_names = HashSet::new();
    for (at, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Ident && !scopes.iter().any(|scope| scope.contains(&at)) {
            module_names.insert(key(source, token));
        }
    }
    let mut edits = Vec::new();
    for scope in scopes {
        let body = &tokens[scope.clone()];
        let dynamic = body.iter().enumerate().any(|(at, token)| {
            let bare = at == 0 || body[at - 1].kind != TokenKind::Dot;
            bare && (token.kind == TokenKind::Keyword(Keyword::Execute)
                || (token.kind == TokenKind::Ident
                    && ["Вычислить", "Eval"]
                        .iter()
                        .any(|name| open_bsl::folded_eq(spelling(source, token), name))
                    && body
                        .get(at + 1)
                        .is_some_and(|next| next.kind == TokenKind::LParen)))
        });
        if dynamic {
            continue;
        }
        let mut declarations = Vec::new();
        let mut declared = HashSet::new();
        let mut assignments = Vec::new();
        let mut ignored = HashSet::new();
        let mut at = scope.start;
        while at < scope.end {
            if tokens[at].kind == TokenKind::Keyword(Keyword::Var)
                && (at == 0 || tokens[at - 1].kind != TokenKind::Dot)
            {
                let mut end = at + 1;
                while end < scope.end && tokens[end].kind != TokenKind::Semicolon {
                    end += 1;
                }
                if end < scope.end {
                    let names = (at + 1..end)
                        .filter(|&index| tokens[index].kind == TokenKind::Ident)
                        .collect::<Vec<_>>();
                    for &index in &names {
                        declared.insert(key(source, &tokens[index]));
                        ignored.insert(index);
                    }
                    declarations.push((at, end, names));
                    at = end + 1;
                    continue;
                }
            }
            if tokens[at].kind == TokenKind::Ident
                && tokens
                    .get(at + 1)
                    .is_some_and(|next| next.kind == TokenKind::Eq)
            {
                let start = tokens[at].span.start as usize;
                let line = source[..start].rfind('\n').map_or(0, |index| index + 1);
                if source[line..start].trim().is_empty() {
                    let mut end = at + 2;
                    while end < scope.end && tokens[end].kind != TokenKind::Semicolon {
                        end += 1;
                    }
                    if end < scope.end {
                        assignments.push((at, end));
                        ignored.insert(at);
                    }
                }
            }
            at += 1;
        }
        let mut reads = HashSet::new();
        for at in scope.clone() {
            if tokens[at].kind == TokenKind::Ident && !ignored.contains(&at) {
                reads.insert(key(source, &tokens[at]));
            }
        }
        let mut retained_writes = HashSet::new();
        for (start, end) in assignments {
            let name = key(source, &tokens[start]);
            let temporary = spelling(source, &tokens[start])
                .starts_with("__open_bsl_image_result_")
                && !module_names.contains(&name);
            if !reads.contains(&name)
                && (declared.contains(&name) || temporary)
                && pure_constant(&tokens[start + 2..end])
                && !source[tokens[start].span.start as usize..tokens[end].span.end as usize]
                    .contains("//")
            {
                edits.push(Edit {
                    range: line_range(
                        source,
                        tokens[start].span.start as usize,
                        tokens[end].span.end as usize,
                    ),
                    text: String::new(),
                });
            } else {
                retained_writes.insert(name);
            }
        }
        for (start, end, names) in declarations {
            let kept = names
                .iter()
                .filter(|&&at| {
                    let name = key(source, &tokens[at]);
                    reads.contains(&name) || retained_writes.contains(&name)
                })
                .map(|&at| spelling(source, &tokens[at]))
                .collect::<Vec<_>>();
            if kept.len() != names.len()
                && !source[tokens[start].span.start as usize..tokens[end].span.end as usize]
                    .contains("//")
            {
                let range = if kept.is_empty() {
                    line_range(
                        source,
                        tokens[start].span.start as usize,
                        tokens[end].span.end as usize,
                    )
                } else {
                    tokens[start].span.start as usize..tokens[end].span.end as usize
                };
                let text = if kept.is_empty() {
                    String::new()
                } else {
                    format!("Перем {};", kept.join(", "))
                };
                edits.push(Edit { range, text });
            }
        }
    }
    Ok(apply(source, edits))
}

pub(super) fn simplify(source: &str) -> Result<String, String> {
    let mut source = source.to_owned();
    loop {
        let next = fold_booleans(&source)?;
        let next = remove_fallthrough(&next)?;
        let next = remove_unused(&next)?;
        if next == source {
            return Ok(next);
        }
        // Удаление пустого присваивания может открыть следующий переход.
        // Проходы только сворачивают код; повтор до неподвижной точки
        // не меняет границы и сигнатуры методов.
        source = next;
    }
}

#[cfg(test)]
#[path = "native_source_cleanup_tests.rs"]
mod tests;
