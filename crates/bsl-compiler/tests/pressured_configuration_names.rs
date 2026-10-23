use bsl_compiler::{BuildOptions, Optimizations, compile_configuration};

#[test]
fn retry_preserves_names_and_shapes_before_after_and_in_another_module() {
    let first = include_str!(
        "../../../docs/research/fixtures/buh-register-pressure-1016/configuration-module.bsl"
    );
    let second = "Функция Прочитать(О) Экспорт Возврат О.ПОЛЕ; КонецФункции Функция Создать() Экспорт Возврат Новый Структура(\"Поле,Вторая\",3,4); КонецФункции";
    let resolved = [first, second].map(|source| {
        let parsed = bsl_syntax::parse(source).unwrap();
        bsl_sema::resolve_program(&parsed.items).unwrap()
    });
    let modules = [
        ("Первый".into(), &resolved[0]),
        ("Второй".into(), &resolved[1]),
    ];
    let (retry, _) = compile_configuration(&modules, None, BuildOptions::default()).unwrap();
    let (explicit, _) = compile_configuration(
        &modules,
        None,
        BuildOptions {
            optimizations: Optimizations {
                ssa_regalloc: true,
                ..Optimizations::default()
            },
            ..BuildOptions::default()
        },
    )
    .unwrap();
    for (left, right) in retry.modules.iter().zip(&explicit.modules) {
        assert_eq!(left.program.names, right.program.names);
        assert_eq!(
            left.program
                .shapes
                .iter()
                .map(|s| (&s.names, s.depth))
                .collect::<Vec<_>>(),
            right
                .program
                .shapes
                .iter()
                .map(|s| (&s.names, s.depth))
                .collect::<Vec<_>>()
        );
        assert!(left.program.names.iter().any(|name| name == "Поле"));
        assert!(!left.program.names.iter().any(|name| name == "ПОЛЕ"));
    }
    bsl_bytecode::image::verify_configuration(&retry, None).unwrap();
    let image = bsl_bytecode::BytecodeImage::Configuration {
        catalog: retry,
        entry: None,
    };
    let text = bsl_bytecode::write_image(&image, None).unwrap();
    let parsed = bsl_bytecode::parse_image(&text).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } = &parsed else {
        panic!("ожидался каталог");
    };
    bsl_bytecode::image::verify_configuration(catalog, entry.as_ref()).unwrap();
    assert_eq!(bsl_bytecode::write_image(&parsed, None).unwrap(), text);
}
