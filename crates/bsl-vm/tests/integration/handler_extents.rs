use bsl_bytecode::{Program, image, parse_program, write_program};
use bsl_rt::{BslValue, RtError};

fn compile(source: &str) -> Program {
    let parsed = bsl_syntax::parse(source).unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    bsl_compiler::compile_program(&resolved).unwrap()
}

#[test]
fn explicit_handler_extents_survive_text_round_trip_and_nested_restoration() {
    let program = compile(
        "Попытка ВызватьИсключение \"outer\"; Исключение
         Попытка ВызватьИсключение \"inner\"; Исключение
         Внутренняя = ОписаниеОшибки(); КонецПопытки;
         Внешняя = ОписаниеОшибки(); КонецПопытки;
         Возврат Внутренняя = \"inner\" И Внешняя = \"outer\" И ОписаниеОшибки() = \"\";",
    );
    let text = write_program(&program, None).unwrap();
    let restored = parse_program(&text).unwrap();
    assert_eq!(
        program.chunks[0].exception_ranges,
        restored.chunks[0].exception_ranges
    );
    assert_eq!(
        bsl_vm::run_program(&restored).unwrap(),
        BslValue::Boolean(true)
    );
}

#[test]
fn invalid_handler_extents_are_rejected_by_the_reader_and_vm() {
    for end in [0, usize::MAX] {
        let mut program = compile("Попытка ВызватьИсключение \"x\"; Исключение КонецПопытки;");
        program.chunks[0].exception_ranges[0].handler_end_pc = end;
        assert!(matches!(
            image::verify(&program),
            Err(RtError::InvalidBytecode(_))
        ));
        assert!(matches!(
            bsl_vm::run_program(&program),
            Err(RtError::InvalidBytecode(_))
        ));
        let text = write_program(&program, None).unwrap();
        assert!(parse_program(&text).is_err());
    }
}

#[test]
fn an_empty_compiled_handler_has_equal_entry_and_end() {
    let program = compile("Попытка ВызватьИсключение \"x\"; Исключение КонецПопытки;");
    let range = &program.chunks[0].exception_ranges[0];
    assert_eq!(range.handler_pc, range.handler_end_pc);
    image::verify(&program).unwrap();
    assert_eq!(bsl_vm::run_program(&program).unwrap(), BslValue::Undefined);
}
