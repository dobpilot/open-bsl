use bsl_syntax::{Keyword, Lexer, TokenKind};

fn tokenize(src: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(src);
    let mut out = Vec::new();
    loop {
        let tok = lexer
            .next_token()
            .unwrap_or_else(|e| panic!("lex error: {e:?}"));
        let done = tok.kind == TokenKind::Eof;
        out.push(tok.kind);
        if done {
            return out;
        }
    }
}

#[test]
fn nbody_fixture_tokenizes_without_errors() {
    // См. nbody_ast_snapshot.rs: n-body.bsl разрезан на три варианта задачей
    // 2 ревью, n-body-precision.bsl несёт ту же грамматику.
    let src = include_str!("../../../tests/conformance/fixtures/n-body-precision.bsl");
    let toks = tokenize(src);
    assert!(
        toks.len() > 500,
        "expected a substantial token stream, got {}",
        toks.len()
    );

    let has = |kw: Keyword| toks.contains(&TokenKind::Keyword(kw));
    assert!(has(Keyword::Function));
    assert!(has(Keyword::EndFunction));
    assert!(has(Keyword::For));
    assert!(has(Keyword::To));
    assert!(has(Keyword::Do));
    assert!(has(Keyword::EndDo));
    assert!(has(Keyword::Return));
    assert!(has(Keyword::New));
}

#[test]
fn nbody_pow_variant_tokenizes_without_errors() {
    let src = include_str!("../../../tests/conformance/fixtures/n-body-pow-variant.bsl");
    let toks = tokenize(src);
    assert!(toks.len() > 500);
}
