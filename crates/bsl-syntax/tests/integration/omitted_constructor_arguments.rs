#[test]
fn consecutive_omitted_constructor_arguments_parse() {
    bsl_syntax::parse("А = Новый Тип(1, , , 4);")
        .expect("конструктор допускает несколько пропущенных позиций подряд");
}
