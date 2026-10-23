use bsl_rt::{RuntimeBuilder, core_library};
use bsl_sema::{ResolveOptions, SemaError, resolve_program_with};
use bsl_syntax::parse;

#[test]
fn registry_backed_error_names_the_unsupported_constructor() {
    let mut builder = RuntimeBuilder::new();
    builder.register(core_library());
    let registry = builder.build().unwrap();
    let program =
        parse("Функция Тест() Экспорт Возврат Новый НеизвестныйТип(); КонецФункции").unwrap();
    let error = resolve_program_with(
        &program.items,
        &ResolveOptions::new(&registry).configuration_module(),
    )
    .unwrap_err();
    assert!(matches!(&error, SemaError::UnsupportedConstructor(name) if name == "НеизвестныйТип"));
    assert!(error.to_string().contains("НеизвестныйТип"));
}
