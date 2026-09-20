use bsl_bytecode::{parse_program, write_program};

#[test]
fn a_variable_passed_to_an_unknown_method_is_not_a_proven_core_receiver() {
    let source = "Процедура Тест(Получатель) А = Новый Массив; Получатель.Заменить(А); А.Очистить(1, 2); КонецПроцедуры";
    let parsed = bsl_syntax::parse(source).unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    let program = bsl_compiler::compile_program(&resolved).unwrap();
    assert_eq!(
        program.chunks[1]
            .instrs
            .iter()
            .filter(|instr| matches!(
                instr,
                bsl_bytecode::Instr::CallObjectMethod { .. }
                    | bsl_bytecode::Instr::CallObjectProcedure { .. }
            ))
            .count(),
        2
    );
}

#[test]
fn open_calls_preserve_reference_candidates_and_omitted_positions() {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    let registry = builder.build().unwrap();
    let source = "Перем Модульная; Процедура Тест(Объект, Локальная) Объект.Обработать(Локальная, Модульная, , Неопределено); КонецПроцедуры";
    let parsed = bsl_syntax::parse(source).unwrap();
    let resolved = bsl_sema::resolve_program_with_registry(&parsed.items, &registry).unwrap();
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let program = bsl_compiler::compile_program_with(&resolved, optimizations).unwrap();
        let text = write_program(&program, None).unwrap();
        assert!(text.contains("byref:1 bymodvar:0 default value"), "{text}");
        let restored = parse_program(&text).unwrap();
        bsl_bytecode::image::verify(&restored).unwrap();
        assert_eq!(write_program(&restored, None).unwrap(), text);
    }
}

#[test]
fn structure_property_preserves_an_indexed_output_target() {
    let source = "С = Новый Структура(\"А\", 1); М = Новый Массив; М.Добавить(99); Р = С.Свойство(\"А\", М[0]);";
    let parsed = bsl_syntax::parse(source).unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    for optimizations in [
        bsl_compiler::Optimizations::default(),
        bsl_compiler::Optimizations::all(),
    ] {
        let program = bsl_compiler::compile_program_with(&resolved, optimizations).unwrap();
        let text = write_program(&program, None).unwrap();
        assert!(text.contains("byindex:"), "{text}");
        let restored = parse_program(&text).unwrap();
        bsl_bytecode::image::verify(&restored).unwrap();
        assert_eq!(write_program(&restored, None).unwrap(), text);
    }
}
