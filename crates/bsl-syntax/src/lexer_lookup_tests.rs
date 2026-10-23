use super::*;

#[test]
fn identifier_fast_paths_equal_the_previous_unicode_predicates_for_every_scalar() {
    for code in 0..=0x10ffff {
        if let Some(c) = char::from_u32(code) {
            assert_eq!(is_ident_start(c), c.is_alphabetic() || c == '_', "{code:x}");
            assert_eq!(
                is_ident_continue(c),
                c.is_alphanumeric() || c == '_',
                "{code:x}"
            );
        }
    }
}

#[test]
fn repeated_words_keep_their_spelling_spans_and_keyword_classification() {
    let source = "Если Если IF if ıf ıf Größe Größe ёж ёж";
    let mut lexer = Lexer::new(source);
    for word in source.split_whitespace() {
        let token = lexer.next_token().unwrap();
        let expected = keywords::lookup(word).map_or(TokenKind::Ident, TokenKind::Keyword);
        assert_eq!(token.kind, expected);
        let span = token.span;
        assert_eq!(&source[span.start as usize..span.end as usize], word);
    }
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}
