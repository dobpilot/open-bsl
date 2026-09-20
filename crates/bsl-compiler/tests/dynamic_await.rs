use bsl_bytecode::{DynamicKind, DynamicRequest, DynamicScope, LibraryRequirement};

#[test]
fn dynamic_await_is_rejected_with_or_without_registry_and_async_caller() {
    let mut builder = bsl_rt::RuntimeBuilder::new();
    builder.register(bsl_rt::core_library());
    let registry = builder.build().unwrap();
    for registry in [None, Some(&registry)] {
        for caller_is_async in [false, true] {
            for keyword in ["Ждать", "Await"] {
                for kind in [DynamicKind::Eval, DynamicKind::Execute] {
                    let source = match kind {
                        DynamicKind::Eval => format!("{keyword} 23"),
                        DynamicKind::Execute => {
                            format!("Локальная = 11; Локальная = {keyword} 23;")
                        }
                    };
                    let locals = ["Локальная".into()];
                    let requirements = [LibraryRequirement::bsl_rt()];
                    let mut request = DynamicRequest {
                        imports: &[],
                        source: &source,
                        debug_info: false,
                        kind,
                        scope: DynamicScope {
                            module: None,
                            program: DynamicScope::ROOT,
                            chunk: 0,
                        },
                        caller_is_async,
                        locals: &locals,
                        module_vars: &[],
                        functions: &[],
                        names: &[],
                        requirements: &requirements,
                    };
                    let compile = |request: &DynamicRequest<'_>| {
                        bsl_compiler::compile_dynamic_snippet(
                            request,
                            registry,
                            &bsl_syntax::PreprocSymbols::default(),
                            std::num::NonZeroU64::new(1).unwrap(),
                        )
                    };
                    let error = compile(&request)
                        .err()
                        .expect("Ждать отвергается до исполнения");
                    assert!(
                        error.contains("асинхронной процедуре или функции"),
                        "{error}"
                    );
                    request.source = match kind {
                        DynamicKind::Eval => "23",
                        DynamicKind::Execute => "Локальная = 23;",
                    };
                    assert!(!compile(&request).unwrap().chunk.is_async);
                }
            }
        }
    }
}
