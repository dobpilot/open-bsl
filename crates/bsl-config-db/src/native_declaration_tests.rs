use super::*;

#[test]
fn protected_declarations_do_not_recover_instruction_bodies() {
    let original =
        include_str!("../../../tests/conformance/measure/1c/native-image-local-call.image");
    let unsupported = original.replacen("{18,1}", "{999999,1}", 1);
    assert_ne!(unsupported, original);
    assert!(decompile(unsupported.as_bytes()).is_err());
    assert_eq!(
        declarations(original.as_bytes()).unwrap(),
        declarations(unsupported.as_bytes()).unwrap()
    );
}

#[test]
fn private_methods_keep_the_export_position_without_becoming_exports() {
    // Тест меняет только порядок записей, а не толкует новые нативные флаги.
    let image = r#"{1,{"Cmd",0,4294967295},{"Const",0},{"Var",0},{"Proc",3,{"Частный",0,0,0},{"Внешний",32,0,0},{"Публичный",17,0,0}}}"#;
    let declarations = declarations(image.as_bytes()).unwrap();
    assert!(declarations.starts_with("Процедура Частный()\n"));
    assert!(!declarations.contains("Внешний"));
    assert!(declarations.contains("Функция Публичный() Экспорт"));
    assert!(
        !exported_interface(image.as_bytes())
            .unwrap()
            .contains("Частный")
    );
}

#[test]
fn native_export_positions_match_the_full_decompiled_source() {
    for image in [
        include_bytes!("../../../tests/conformance/measure/1c/native-image-local-call.image")
            .as_slice(),
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled-arithmetic.image"
        )
        .as_slice(),
        include_bytes!("../../../docs/research/fixtures/native-local-async-eval/image").as_slice(),
    ] {
        let complete = bsl_syntax::scan_declarations_with_symbols(
            &decompile(image).unwrap(),
            &bsl_syntax::PreprocSymbols::default(),
        )
        .unwrap();
        let projected = bsl_syntax::scan_declarations_with_symbols(
            &declarations(image).unwrap(),
            &bsl_syntax::PreprocSymbols::default(),
        )
        .unwrap();
        let exports = |declarations: &bsl_syntax::ModuleDeclarations| {
            declarations
                .functions
                .iter()
                .enumerate()
                .filter(|(_, method)| method.export)
                .map(|(position, method)| {
                    (
                        position,
                        method.name.clone(),
                        method.is_procedure,
                        method
                            .params
                            .iter()
                            .map(|param| (param.by_val, param.default.is_some()))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(exports(&complete), exports(&projected));
    }
}
