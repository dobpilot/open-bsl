use bsl_syntax::parse;

#[test]
fn nbody_fixture_parses_into_stable_ast() {
    let src = include_str!("../../../tests/conformance/fixtures/n-body.bsl");
    let program = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    // Снапшот всего дерева фиксирует форму парсера разом на всём наборе
    // конструкций языка, которые встречаются в n-body: Function/EndFunction,
    // For/ForEach, New Structure(...), индексация, цепочки полей,
    // унарный минус, деление констант вида -103622044471123109/1000000000000000000.
    insta::assert_debug_snapshot!(program);
}

#[test]
fn nbody_pow_variant_parses_into_stable_ast() {
    let src = include_str!("../../../tests/conformance/fixtures/n-body-pow-variant.bsl");
    let program = parse(src).unwrap_or_else(|e| panic!("parse error: {e:?}"));
    insta::assert_debug_snapshot!(program);
}
