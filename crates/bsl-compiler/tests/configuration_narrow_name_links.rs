use bsl_compiler::{BuildOptions, CompileError, compile_configuration};
use bsl_rt::{LibraryKey, LibraryRequirement, TypeDescriptor};
use bsl_sema::{RExpr, RStmt, RStmtKind, ResolvedLink, ResolvedProgram};

fn resolve(source: &str) -> ResolvedProgram {
    let parsed = bsl_syntax::parse(source).unwrap();
    bsl_sema::resolve_program(&parsed.items).unwrap()
}

fn properties(count: usize) -> ResolvedProgram {
    let mut source = String::from("Функция Читать(О) Экспорт\n");
    for index in 0..count {
        source.push_str(&format!("Р=О.Поле{index};\n"));
    }
    source.push_str("Возврат Р; КонецФункции");
    resolve(&source)
}

#[test]
fn linked_component_type_and_method_both_fit_the_shared_narrow_prefix() {
    static TYPE: TypeDescriptor = TypeDescriptor::new("test-component", "ТипОбъекта");
    let fields = properties(65_536);
    let mut method = resolve("Функция Вызвать(О) Экспорт Возврат О; КонецФункции");
    method
        .requirements
        .push(LibraryRequirement::new("test-component", "1"));
    method.links.push(ResolvedLink::ObjectMethod {
        library: LibraryKey::new("test-component"),
        type_name: TYPE.name,
        method: "Получить".into(),
    });
    method.functions[0].body = vec![RStmt {
        line: 1,
        kind: RStmtKind::Return(Some(RExpr::CallMethod {
            obj: Box::new(RExpr::Local(0)),
            method: "Получить".into(),
            open: true,
            component_type: Some(&TYPE),
            component_link: Some(0),
            args: Vec::new(),
        })),
    }];
    let (catalog, _) = compile_configuration(
        &[("Свойства".into(), &fields), ("Компонент".into(), &method)],
        None,
        BuildOptions::default(),
    )
    .unwrap();
    let program = &catalog.modules[1].program;
    let bsl_bytecode::LinkEntry::ObjectMethod {
        type_name, method, ..
    } = program.links[0]
    else {
        panic!("ожидалась связь компонентного метода");
    };
    assert_eq!(program.names[usize::from(type_name)], "ТипОбъекта");
    assert_eq!(program.names[usize::from(method)], "Получить");
    assert!(usize::from(type_name) < 2);
    assert!(usize::from(method) < 2);
    assert!(std::sync::Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &program.names
    ));
    bsl_bytecode::image::verify_configuration(&catalog, None).unwrap();
}

#[test]
fn genuinely_more_than_65536_narrow_names_return_a_contextual_error() {
    let resolved = [0, 32_769].map(|start| {
        let mut source = String::from("Процедура Вызвать(О) Экспорт\n");
        for index in start..start + 32_769 {
            source.push_str(&format!("О.Метод{index}();\n"));
        }
        source.push_str("КонецПроцедуры");
        resolve(&source)
    });
    let error = compile_configuration(
        &[
            ("Первый".into(), &resolved[0]),
            ("Второй".into(), &resolved[1]),
        ],
        None,
        BuildOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(&error, CompileError::InModule(context)
        if context.0 == "Второй" && matches!(context.1, CompileError::TooManyNames)));
}
