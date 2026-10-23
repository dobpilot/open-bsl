use bsl_compiler::{BuildOptions, compile_configuration};

#[test]
fn property_only_names_do_not_exhaust_configuration_method_indices() {
    let mut source = String::from("Функция Прочитать(О) Экспорт\n");
    for index in 0..65_536 {
        if index == 0 {
            source.push_str("Р = О.пЕрВоЕ;\n");
        } else {
            source.push_str(&format!("Р = О.Поле{index};\n"));
        }
    }
    source.push_str("Возврат Р; КонецФункции");
    let parsed = bsl_syntax::parse(&source).unwrap();
    let fields = bsl_sema::resolve_program(&parsed.items).unwrap();
    let parsed = bsl_syntax::parse(
        "Функция Вызвать(О) Экспорт Возврат О.НовыйМетод(О.ПЕРВОЕ()); КонецФункции",
    )
    .unwrap();
    let method = bsl_sema::resolve_program(&parsed.items).unwrap();
    let modules = [("Свойства".into(), &fields), ("Вызов".into(), &method)];
    let (catalog, _) = compile_configuration(&modules, None, BuildOptions::default()).unwrap();
    let first = &catalog.modules[0].program;
    let second = &catalog.modules[1].program;
    assert_eq!(first.names.len(), 65_537);
    assert!(std::sync::Arc::ptr_eq(&first.names, &second.names));
    let called_names = second
        .chunks
        .iter()
        .flat_map(|chunk| &chunk.instrs)
        .filter_map(|instruction| match instruction {
            bsl_bytecode::Instr::CallObjectMethod { method, .. } => {
                Some(second.names[usize::from(*method)].as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(called_names, ["пЕрВоЕ", "НовыйМетод"]);
    bsl_bytecode::image::verify_configuration(&catalog, None).unwrap();
    let image = bsl_bytecode::BytecodeImage::Configuration {
        catalog,
        entry: None,
    };
    let text = bsl_bytecode::write_image(&image, None).unwrap();
    let restored = bsl_bytecode::parse_image(&text).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } = &restored else {
        panic!("ожидался каталог");
    };
    bsl_bytecode::image::verify_configuration(catalog, entry.as_ref()).unwrap();
    assert_eq!(bsl_bytecode::write_image(&restored, None).unwrap(), text);
}

#[test]
fn ordinary_configuration_keeps_first_spelling_and_existing_name_order() {
    let resolved = [
        "Функция Прочитать(О) Экспорт Возврат О.пЕрВоЕ; КонецФункции",
        "Функция Вызвать(О) Экспорт Возврат О.ПЕРВОЕ(); КонецФункции",
    ]
    .map(|source| {
        let parsed = bsl_syntax::parse(source).unwrap();
        bsl_sema::resolve_program(&parsed.items).unwrap()
    });
    let modules = [
        ("Первый".into(), &resolved[0]),
        ("Второй".into(), &resolved[1]),
    ];
    let (catalog, _) = compile_configuration(&modules, None, BuildOptions::default()).unwrap();
    assert_eq!(&*catalog.modules[0].program.names, &["пЕрВоЕ"]);
    assert!(std::sync::Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &catalog.modules[1].program.names,
    ));
    bsl_bytecode::image::verify_configuration(&catalog, None).unwrap();
}
