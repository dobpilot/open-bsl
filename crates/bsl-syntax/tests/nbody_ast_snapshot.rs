use bsl_syntax::parse;

#[test]
fn nbody_fixture_parses_into_stable_ast() {
    // n-body.bsl раньше был здесь неисполнимым файлом (50 000 000 итераций
    // Advance) — задача 2 ревью разрезала его на n-body-{precision,smoke,
    // perf}.bsl. n-body-precision.bsl — та же грамматика (Function/
    // EndFunction, For/ForEach, New Structure(...), индексация, цепочки
    // полей, унарный минус, деление гигантских констант), парсер не видит
    // разницы в числе итераций Advance.
    let src = include_str!("../../../tests/conformance/fixtures/n-body-precision.bsl");
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
