use bsl_bytecode::{Instr, image};

fn compile(source: &str) -> bsl_bytecode::Program {
    let parsed = bsl_syntax::parse(source).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("разрешение имён");
    bsl_compiler::compile_program(&resolved).expect("компиляция")
}

#[test]
fn duplicate_literal_uses_the_runtime_constructor_and_has_no_invalid_shape() {
    let program = compile("С = Новый Структура(\"А,а\", 1, 2);");
    image::verify(&program).expect("образ с повторным полем корректен");
    assert!(
        program.chunks[0]
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::NewDynamicStructure { .. }))
    );
}

#[test]
fn distinct_literal_keeps_the_shaped_constructor() {
    let program = compile("С = Новый Структура(\"А,Б\", 1, 2);");
    image::verify(&program).expect("обычная форма корректна");
    assert!(
        program.chunks[0]
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::NewStructure { .. }))
    );
    assert!(
        !program.chunks[0]
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::NewDynamicStructure { .. }))
    );
}
