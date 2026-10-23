use bsl_compiler::{BuildOptions, compile_configuration};

#[test]
fn compiled_configuration_keeps_one_allocation_for_its_equal_name_tables() {
    let resolved = ["ПолеПервого", "ПолеВторого"].map(|field| {
        let parsed = bsl_syntax::parse(&format!(
            "Функция Прочитать(Объект) Экспорт Возврат Объект.{field}; КонецФункции"
        ))
        .unwrap();
        bsl_sema::resolve_program(&parsed.items).unwrap()
    });
    let modules = vec![
        ("Первый".into(), &resolved[0]),
        ("Второй".into(), &resolved[1]),
    ];
    let (catalog, _) = compile_configuration(&modules, None, BuildOptions::default()).unwrap();
    let first = &catalog.modules[0].program.names;
    let second = &catalog.modules[1].program.names;
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(String::as_str).collect::<Vec<_>>(),
        ["ПолеПервого", "ПолеВторого"]
    );
    assert!(
        std::ptr::eq(first.as_ptr(), second.as_ptr()),
        "равные таблицы каталога должны разделять выделение, а не копировать строки"
    );
}

#[test]
fn parsed_configuration_shares_equal_tables_and_preserves_distinct_tables() {
    let parsed =
        bsl_syntax::parse("Функция Прочитать(Объект) Экспорт Возврат Объект.Поле; КонецФункции")
            .unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    let modules = vec![("Первый".into(), &resolved), ("Второй".into(), &resolved)];
    let (catalog, entry) =
        compile_configuration(&modules, Some(&resolved), BuildOptions::default()).unwrap();
    let entry = entry.map(|program| bsl_bytecode::EntryProgram {
        id: bsl_bytecode::EntryId::new(1),
        program,
    });
    let mut image = bsl_bytecode::BytecodeImage::Configuration { catalog, entry };
    let text = bsl_bytecode::write_image(&image, None).unwrap();
    let restored = bsl_bytecode::parse_image(&text).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } = &restored else {
        panic!("ожидался каталог");
    };
    assert!(std::sync::Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &catalog.modules[1].program.names
    ));
    assert!(std::sync::Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &entry.as_ref().unwrap().program.names
    ));
    bsl_bytecode::image::verify_configuration(catalog, entry.as_ref()).unwrap();
    assert_eq!(bsl_bytecode::write_image(&restored, None).unwrap(), text);

    let bsl_bytecode::BytecodeImage::Configuration { catalog, .. } = &mut image else {
        unreachable!();
    };
    catalog.modules[1].program.names = vec!["ДругоеПоле".to_string()].into();
    let text = bsl_bytecode::write_image(&image, None).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, .. } =
        bsl_bytecode::parse_image(&text).unwrap()
    else {
        unreachable!();
    };
    assert_eq!(catalog.modules[1].program.names[0], "ДругоеПоле");
    assert!(!std::sync::Arc::ptr_eq(
        &catalog.modules[0].program.names,
        &catalog.modules[1].program.names
    ));
}

#[test]
fn entry_shares_the_compilation_table_but_later_appends_do_not_mutate_it() {
    let parsed = bsl_syntax::parse("Функция Ф(О) Экспорт Возврат О.Поле; КонецФункции").unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    let modules = vec![("Модуль".into(), &resolved)];
    let (catalog, entry) =
        compile_configuration(&modules, Some(&resolved), BuildOptions::default()).unwrap();
    let entry = entry.unwrap();
    let base = &catalog.modules[0].program;
    assert!(std::sync::Arc::ptr_eq(&base.names, &entry.names));
    let parsed = bsl_syntax::parse("Функция Ф(О) Возврат О.НовоеПоле; КонецФункции").unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    let appended = bsl_compiler::compile_entry_program(
        &resolved,
        &base.names,
        &base.shapes,
        BuildOptions::default(),
    )
    .unwrap();
    assert_eq!(base.names.as_ref(), ["Поле"]);
    assert_eq!(entry.names.as_ref(), ["Поле"]);
    assert_eq!(appended.names.as_ref(), ["Поле", "НовоеПоле"]);
}

#[test]
fn decoding_does_not_replace_a_table_that_fails_verification() {
    let parsed = bsl_syntax::parse("Функция Ф(О) Экспорт Возврат О.Поле; КонецФункции").unwrap();
    let resolved = bsl_sema::resolve_program(&parsed.items).unwrap();
    let modules = vec![("Первый".into(), &resolved), ("Второй".into(), &resolved)];
    let (mut catalog, _) = compile_configuration(&modules, None, BuildOptions::default()).unwrap();
    catalog.modules[1].program.names = Default::default();
    let before = bsl_bytecode::image::verify_configuration(&catalog, None).unwrap_err();
    let image = bsl_bytecode::BytecodeImage::Configuration {
        catalog,
        entry: None,
    };
    let text = bsl_bytecode::write_image(&image, None).unwrap();
    let bsl_bytecode::BytecodeImage::Configuration { catalog, entry } =
        bsl_bytecode::parse_image(&text).unwrap()
    else {
        unreachable!();
    };
    assert!(catalog.modules[1].program.names.is_empty());
    let after = bsl_bytecode::image::verify_configuration(&catalog, entry.as_ref()).unwrap_err();
    assert_eq!(before.to_string(), after.to_string());
}
