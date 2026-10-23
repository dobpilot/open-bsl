use bsl_compiler::{BuildOptions, CompileError, compile_configuration, compile_program};

fn oversized_method() -> bsl_sema::ResolvedProgram {
    let mut source = String::from("Функция Перегрузка()\n");
    for index in 0..256 {
        source.push_str(&format!("Локальная{index} = {index};\n"));
    }
    source.push_str("Результат = 0;\n");
    for index in 0..256 {
        source.push_str(&format!("Результат = Результат + Локальная{index};\n"));
    }
    source.push_str("Возврат Результат;\nКонецФункции\n");
    let parsed = bsl_syntax::parse(&source).expect("разбор");
    bsl_sema::resolve_program(&parsed.items).expect("разрешение имён")
}

#[test]
fn configuration_compile_error_names_module_and_method() {
    let resolved = oversized_method();
    let error = compile_configuration(
        &[("МодульПроба".to_string(), &resolved)],
        None,
        BuildOptions::default(),
    )
    .expect_err("лимит кадра");
    assert_eq!(
        error.to_string(),
        "модуль «МодульПроба»: метод «Перегрузка»: слишком много локальных переменных в кадре"
    );
    assert!(matches!(
        error,
        CompileError::InModule(context)
            if matches!(&context.1, CompileError::InMethod(context)
                if context.1 == CompileError::TooManyLocals)
    ));
}

#[test]
fn standalone_compile_error_keeps_its_original_shape() {
    let resolved = oversized_method();
    assert!(matches!(
        compile_program(&resolved),
        Err(CompileError::TooManyLocals)
    ));
}
