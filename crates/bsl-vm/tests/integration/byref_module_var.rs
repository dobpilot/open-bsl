//! Модульная переменная, переданная в параметр без `Знач`, мутируется по
//! ссылке — и из тела модуля, и изнутри процедуры. ИЗМЕРЕНО на 8.3.27
//! (`CALL.BYREF.MODULEVAR`): платформа отвечает `изменено|изменено`. Прежде
//! второй случай уходил копией (`ArgMode::Value`) и возвращал исходное
//! значение — `RExpr::ModuleVar` не ловился образцом `RExpr::Local`.

use bsl_bytecode::{ArgMode, Program};
use bsl_compiler::compile_program;

fn compile(src: &str) -> Program {
    let parsed = bsl_syntax::parse(src).expect("разбор");
    let resolved = bsl_sema::resolve_program(&parsed.items).expect("резолвинг");
    compile_program(&resolved).expect("кодоген")
}

#[test]
fn a_module_var_passed_by_ref_from_a_procedure_is_mutated() {
    let program = compile(
        "Перем М;\n\
         Процедура Подменить(п)\n  п = 42;\nКонецПроцедуры\n\
         Процедура ИзПроцедуры()\n  Подменить(М);\nКонецПроцедуры\n\
         М = 0;\nИзПроцедуры();\nВозврат М;\n",
    );
    // Внутри процедуры вызов передаёт модульную переменную режимом
    // ByRefModuleVar, а не копией: без этого `Подменить` писал бы в свой
    // временный слот, и `М` оставалась бы нулём.
    let has_bymodvar = program.chunks.iter().any(|chunk| {
        chunk
            .call_arg_modes
            .iter()
            .flatten()
            .any(|mode| matches!(mode, ArgMode::ByRefModuleVar(_)))
    });
    assert!(
        has_bymodvar,
        "вызов из процедуры обязан дать ByRefModuleVar"
    );
    assert_eq!(
        bsl_vm::run_program(&program).unwrap().to_string(),
        "42",
        "модульная переменная мутирована по ссылке из процедуры"
    );
}
