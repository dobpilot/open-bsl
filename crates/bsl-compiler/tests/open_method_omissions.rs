use bsl_bytecode::{parse_program, write_program};

#[test]
fn a_core_receivers_open_call_retains_default_modes_through_optimization_and_text() {
    let source = "Процедура Начать(Путь) Запись = Новый ЗаписьТекста; Запись.Открыть(Путь,,,Истина); КонецПроцедуры";
    let parsed = bsl_syntax::parse(source).unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let program = bsl_compiler::compile_program_with(&resolved, optimizations).unwrap();
        let text = write_program(&program, None).unwrap();
        assert!(text.contains("default default value"), "{text}");
        assert!(
            program
                .chunks
                .iter()
                .flat_map(|chunk| &chunk.instrs)
                .any(|instr| matches!(instr, bsl_bytecode::Instr::CallObjectProcedure { .. }))
        );
        let restored = parse_program(&text).unwrap();
        bsl_bytecode::image::verify(&restored).unwrap();
        assert_eq!(write_program(&restored, None).unwrap(), text);
    }
}
