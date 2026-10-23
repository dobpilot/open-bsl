use super::{bytes_resource, text_resource};
use crate::{LoadOptions, Resources, native_image};

const OWNER: &str = "22222222-2222-2222-2222-222222222222";
const INITIALIZER: &[u8] =
    include_bytes!("../../../tests/conformance/measure/1c/native-image-module-initializer.image");

#[test]
fn exported_placeholders_do_not_initialize_or_modify_arguments() {
    let source = native_image::exported_interface(INITIALIZER).unwrap();
    assert_eq!(
        source,
        "Функция ПрочитатьПробы() Экспорт\nВозврат Истина;\nКонецФункции\nПроцедура УстановитьПробы(Знач НовоеЗначениеПробы) Экспорт\n\nКонецПроцедуры\n"
    );
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, _) = instance.call(&mut state, "ПрочитатьПробы", vec![]).unwrap();
    assert!(matches!(value, open_bsl::Value::Boolean(true)));
    let (value, args) = instance
        .call(
            &mut state,
            "УстановитьПробы",
            vec![open_bsl::Value::Boolean(false)],
        )
        .unwrap();
    assert!(matches!(value, open_bsl::Value::Undefined));
    assert!(matches!(args.as_slice(), [open_bsl::Value::Boolean(false)]));
}

#[test]
fn private_and_external_methods_are_omitted_and_reference_parameters_remain() {
    let image = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-ordinary-shared-context.image"
    );
    let source = native_image::exported_interface(image).unwrap();
    assert_eq!(
        source,
        "Функция Проба(Аргумент) Экспорт\nВозврат Истина;\nКонецФункции\n"
    );
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, args) = instance
        .call(&mut state, "Проба", vec![open_bsl::Value::Boolean(false)])
        .unwrap();
    assert!(matches!(value, open_bsl::Value::Boolean(true)));
    assert!(matches!(args.as_slice(), [open_bsl::Value::Boolean(false)]));
    assert!(
        instance
            .call(&mut state, "Изменить", vec![open_bsl::Value::Undefined])
            .is_err()
    );
}

#[test]
fn boolean_defaults_and_required_parameters_keep_their_signatures() {
    let image =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-boolean-defaults.image");
    let source = native_image::exported_interface(image).unwrap();
    assert!(source.contains("Знач Аргумент = Истина"));
    assert!(source.contains("Знач Аргумент = Ложь"));
    assert!(source.contains("БезУмолчания(Знач Аргумент) Экспорт"));
    bsl_syntax::parse(&source).unwrap();
}

#[test]
fn unsupported_bodies_need_not_be_decoded_but_exported_headers_remain_strict() {
    let original = std::str::from_utf8(INITIALIZER).unwrap();
    let unknown = original.replacen("{2,0}", "{9999,0}", 1);
    assert_eq!(
        native_image::exported_interface(unknown.as_bytes()).unwrap(),
        native_image::exported_interface(INITIALIZER).unwrap()
    );
    assert!(native_image::decompile(unknown.as_bytes()).is_err());
    for (from, to) in [
        (
            "{\"НовоеЗначениеПробы\",13,-1}",
            "{\"НовоеЗначениеПробы\",99,-1}",
        ),
        ("{\"DefPrm\",1,{\"\"}}", "{\"DefPrm\",1,{\"Unknown\"}}"),
        ("\"ПрочитатьПробы\",17", "\"ПрочитатьПробы\",31"),
    ] {
        assert!(original.contains(from));
        assert!(
            native_image::exported_interface(original.replacen(from, to, 1).as_bytes()).is_err()
        );
    }
    assert!(native_image::exported_interface(b"broken").is_err());
}

#[test]
fn resource_loading_uses_default_placeholders_and_explicit_complete_recovery() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-constant.Module.bin");
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    for enabled in [false, true] {
        let resources = Resources::with_options(
            std::slice::from_ref(&resource),
            LoadOptions {
                decompile_protected_modules: enabled,
            },
        )
        .unwrap();
        let source = resources.module_source(OWNER, "0").unwrap().unwrap();
        let engine = open_bsl::Engine::builder().build().unwrap();
        let module = engine.compile(&source).unwrap();
        let mut state = engine.new_state();
        let instance = state.instantiate(&module).unwrap();
        let (value, _) = instance.call(&mut state, "Проба", vec![]).unwrap();
        let actual = open_bsl::format_value(&value, None).unwrap();
        let native = include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-constant.platform.txt"
        )
        .trim()
        .strip_prefix("image.native.constant=")
        .unwrap();
        assert_eq!(actual, if enabled { native } else { "Да" });
    }
    assert!(!LoadOptions::default().decompile_protected_modules);
}

#[test]
fn plain_and_container_text_are_preserved_in_both_modes() {
    let text = "Функция Проба() Экспорт Возврат 777; КонецФункции";
    let resources = [
        text_resource(&format!("{OWNER}.0"), text),
        bytes_resource(
            &format!("{OWNER}.1"),
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-source-priority.Module.bin"
            ),
        ),
    ];
    for enabled in [false, true] {
        let resources = Resources::with_options(
            &resources,
            LoadOptions {
                decompile_protected_modules: enabled,
            },
        )
        .unwrap();
        assert_eq!(resources.module_source(OWNER, "0").unwrap().unwrap(), text);
        assert_eq!(
            resources.module_source(OWNER, "1").unwrap().unwrap(),
            "Возврат 777;"
        );
    }
}

#[test]
fn snapshot_loading_passes_the_selected_policy_to_the_session_module() {
    let root = "11111111-1111-1111-1111-111111111111";
    let resources = [
        text_resource("root", &format!("{{1,{{{root}}}}}")),
        text_resource(
            root,
            &format!(
                "{{2,{{{root}}},1,{{{},{{1,{{68,{{0,{{3,{{1,0,{OWNER}}},\"Конфигурация\"}}}}}},0}}}}}}",
                crate::CONFIGURATION
            ),
        ),
        bytes_resource(
            &format!("{OWNER}.7"),
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
            ),
        ),
    ];
    for enabled in [false, true] {
        let loaded = crate::load_base_graph(
            &resources,
            LoadOptions {
                decompile_protected_modules: enabled,
            },
        )
        .unwrap();
        assert_eq!(loaded.snapshot.modules().len(), 1);
        let source = loaded.snapshot.modules()[0].source();
        let expected = if enabled {
            native_image::decompile(INITIALIZER)
        } else {
            native_image::exported_interface(INITIALIZER)
        }
        .unwrap();
        assert_eq!(source, expected);
    }
}

#[test]
fn string_defaults_remain_bare_literals_in_exported_headers() {
    let original = std::str::from_utf8(INITIALIZER).unwrap();
    let image = original.replace("{\"DefPrm\",1,{\"\"}}", "{\"DefPrm\",1,{\"S\",\"\"}}");
    let source = native_image::exported_interface(image.as_bytes()).unwrap();
    assert!(source.contains("Знач НовоеЗначениеПробы = \"\"") && !source.contains("= (\"\")"));
    bsl_syntax::parse(&source).unwrap();
}

#[test]
fn extension_module_replacement_uses_the_same_protected_source_policy() {
    let resource = bytes_resource(
        &format!("{}.3", super::LOCAL_OBJECT),
        include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
        ),
    );
    for enabled in [false, true] {
        let resources = Resources::with_options(
            std::slice::from_ref(&resource),
            LoadOptions {
                decompile_protected_modules: enabled,
            },
        )
        .unwrap();
        let mut modules = Vec::new();
        crate::apply_extension_object(
            &mut modules,
            &super::base_objects_for_test(),
            &resources,
            super::CATALOG_CLASS,
            &super::extension_object(1, super::BASE_OBJECT),
            "Тестовое",
        )
        .unwrap();
        assert_eq!(modules.len(), 1);
        let expected = if enabled {
            native_image::decompile(INITIALIZER)
        } else {
            native_image::exported_interface(INITIALIZER)
        }
        .unwrap();
        assert_eq!(modules[0].source(), expected);
        assert_eq!(modules[0].owner(), Some(super::BASE_OBJECT));
    }
}

#[test]
fn measured_async_exports_use_the_explicit_synchronous_placeholder_policy() {
    let image =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-method-flag2.image");
    let source = native_image::exported_interface(image).unwrap();
    assert!(!source.contains("Асинх"));
    assert!(source.contains("Процедура ЭкспортнаяПроцедураПробы(АргументПробы) Экспорт"));
    assert!(source.contains("Функция ЭкспортнаяФункцияПробы(АргументПробы) Экспорт"));
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    for (name, function) in [
        ("ЭкспортнаяПроцедураПробы", false),
        ("ЭкспортнаяФункцияПробы", true),
    ] {
        let (value, args) = instance
            .call(&mut state, name, vec![open_bsl::Value::Boolean(false)])
            .unwrap();
        if function {
            assert!(matches!(value, open_bsl::Value::Boolean(true)));
        } else {
            assert!(matches!(value, open_bsl::Value::Undefined));
        }
        assert!(matches!(args.as_slice(), [open_bsl::Value::Boolean(false)]));
    }
}

#[test]
fn opaque_exports_use_only_the_approved_placeholder_signature() {
    let original = std::str::from_utf8(INITIALIZER).unwrap();
    let header = "{\"УстановитьПробы\",16,1,7,{\"Var\",1,{\"НовоеЗначениеПробы\",13,-1}},{\"DefPrm\",1,{\"\"}}}";
    assert!(original.contains(header));
    let opaque =
        "{\"УстановитьПробы\",16,2,7,{\"Var\",2,{\"Первый\",74306,0},{\"Второй\",65822,0}}}";
    let image = original.replace(header, opaque);
    let source = native_image::exported_interface(image.as_bytes()).unwrap();
    assert!(source.contains("Процедура УстановитьПробы(Первый, Второй) Экспорт"));
    assert!(native_image::decompile(image.as_bytes()).is_err());
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, args) = instance
        .call(
            &mut state,
            "УстановитьПробы",
            vec![
                open_bsl::Value::Boolean(false),
                open_bsl::Value::Boolean(true),
            ],
        )
        .unwrap();
    assert!(matches!(value, open_bsl::Value::Undefined));
    assert!(matches!(
        args.as_slice(),
        [
            open_bsl::Value::Boolean(false),
            open_bsl::Value::Boolean(true)
        ]
    ));
    assert!(
        instance
            .call(
                &mut state,
                "УстановитьПробы",
                vec![open_bsl::Value::Boolean(false)]
            )
            .is_err()
    );
    for (from, to) in [
        ("\"Второй\",65822,0", "\"пЕРВЫЙ\",65822,0"),
        ("\"Первый\",74306,0", "\"1Первый\",74306,0"),
        ("\"Первый\",74306,0", "\"Первый\",74306,-1"),
        ("\"Первый\",74306,0", "\"Первый\",5,0"),
        ("16,2,7", "16,3,7"),
    ] {
        assert!(native_image::exported_interface(image.replace(from, to).as_bytes()).is_err());
    }
}

#[test]
fn opaque_placeholder_arity_follows_the_header_for_one_and_three_parameters() {
    let original = std::str::from_utf8(INITIALIZER).unwrap();
    let header = "{\"УстановитьПробы\",16,1,7,{\"Var\",1,{\"НовоеЗначениеПробы\",13,-1}},{\"DefPrm\",1,{\"\"}}}";
    for (count, variables, expected) in [
        (1, "{\"Первый\",7669,0}", "Первый"),
        (
            3,
            "{\"Первый\",74306,0},{\"Второй\",65822,0},{\"Третий\",7669,0}",
            "Первый, Второй, Третий",
        ),
    ] {
        let opaque =
            format!("{{\"УстановитьПробы\",16,{count},7,{{\"Var\",{count},{variables}}}}}");
        let image = original.replace(header, &opaque);
        let source = native_image::exported_interface(image.as_bytes()).unwrap();
        assert!(source.contains(&format!("Процедура УстановитьПробы({expected}) Экспорт")));
        assert!(native_image::decompile(image.as_bytes()).is_err());
        bsl_syntax::parse(&source).unwrap();
    }
}
