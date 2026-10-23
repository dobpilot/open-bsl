use super::{Resources, bytes_resource};

const OWNER: &str = "22222222-2222-2222-2222-222222222222";
const CONTAINER: &[u8] =
    include_bytes!("../../../tests/conformance/measure/1c/native-image-constant.Module.bin");

#[test]
fn a_local_label_container_executes_both_complete_method_bodies() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-goto-labels-both.Module.bin"
    );
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let mut actual = Vec::new();
    for (name, id) in [("ПерваяПробы", "first"), ("ВтораяПробы", "second")] {
        let (value, parameters) = instance.call(&mut state, name, vec![]).unwrap();
        assert!(parameters.is_empty());
        actual.push(format!(
            "image.native.goto_labels.both.{id}={}",
            open_bsl::format_value(&value, None).unwrap()
        ));
    }
    assert_eq!(actual.join("\n"), include_str!("../../../tests/conformance/measure/1c/measure-native-image-goto-labels-both.platform.txt").trim());
}

#[test]
fn a_native_proven_unreachable_container_preserves_every_exported_method() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-unreachable-probe.Module.bin"
    );
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let mut actual = Vec::new();
    for (name, id) in [
        ("ИстиннаяПробы", "true"),
        ("ЛожнаяПробы", "false"),
        ("СледующаяПробы", "next"),
        ("ПропускПробы", "jump"),
    ] {
        let (value, parameters) = instance.call(&mut state, name, vec![]).unwrap();
        assert!(parameters.is_empty());
        actual.push(format!(
            "image.native.unreachable.{id}={}",
            open_bsl::format_value(&value, None).unwrap()
        ));
    }
    assert_eq!(actual.join("\n"), include_str!("../../../tests/conformance/measure/1c/measure-native-image-unreachable-probe.platform.txt").trim());
}

#[test]
fn a_true_predicate_jump_container_preserves_complete_source_and_reference_calls() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-jump-true.Module.bin");
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    assert_eq!(
        source,
        crate::native_image::decompile(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-jump-true.image"
        ))
        .unwrap()
    );
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let receipt = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-jump-true.platform.txt"
    );
    for (argument, id) in [(-5, "negative"), (7, "positive")] {
        let (_, parameters) = instance
            .call(
                &mut state,
                "ПроцедураПробы",
                vec![open_bsl::Value::Number(open_bsl::BslNumber::from_i64(
                    argument,
                ))],
            )
            .unwrap();
        let actual = format!(
            "image.native.jump_true.reference.{id}={}",
            open_bsl::format_value(&parameters[0], None).unwrap()
        );
        assert!(receipt.lines().any(|line| line == actual), "{actual}");
    }
}

#[test]
fn an_initializer_container_preserves_the_complete_recovered_module() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-module-initializer.Module.bin"
    );
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    assert_eq!(
        source,
        crate::native_image::decompile(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-module-initializer.image"
        ))
        .unwrap()
    );
}

#[test]
fn a_scsu_compiler_container_loads_the_same_complete_source_as_utf8() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-scsu.Module.bin");
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    let expected = crate::native_image::decompile(include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-compiled.image"
    ))
    .unwrap();
    assert_eq!(source, expected);
}

#[test]
fn a_malformed_scsu_container_reports_its_module_without_partial_source() {
    let image = include_bytes!("../../../tests/conformance/measure/1c/native-image-scsu.image");
    let mut bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-scsu.Module.bin")
            .to_vec();
    let start = bytes
        .windows(image.len())
        .position(|part| part == image)
        .unwrap();
    bytes[start + 3] = 0x0c;
    let resource = bytes_resource(&format!("{OWNER}.0"), &bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let error = resources.module_source(OWNER, "0").unwrap_err().to_string();
    assert!(error.contains(OWNER), "{error}");
    assert!(error.contains("image/SCSU: смещение 3"), "{error}");
}

#[test]
fn image_only_resource_loads_and_executes_through_the_facade() {
    let resource = bytes_resource(&format!("{OWNER}.0"), CONTAINER);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, _) = instance.call(&mut state, "Проба", vec![]).unwrap();
    let actual = format!(
        "image.native.constant={}",
        open_bsl::format_value(&value, None).unwrap()
    );
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-constant.platform.txt"
    );
    assert_eq!(actual, expected.trim());
}

#[test]
fn image_only_undefined_resource_loads_and_returns_the_native_value() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-undefined.Module.bin");
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let (value, parameters) = instance.call(&mut state, "Проба", vec![]).unwrap();
    assert!(matches!(value, open_bsl::Value::Undefined));
    assert!(parameters.is_empty());
}

#[test]
fn the_complete_native_compiler_container_loads_through_the_existing_resource_path() {
    let bytes =
        include_bytes!("../../../tests/conformance/measure/1c/native-image-compiled.Module.bin");
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    assert_eq!(
        source,
        crate::native_image::decompile(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled.image"
        ))
        .unwrap()
    );
    let engine = open_bsl::Engine::builder().build().unwrap();
    let module = engine.compile(&source).unwrap();
    let mut state = engine.new_state();
    let instance = state.instantiate(&module).unwrap();
    let mut actual = Vec::new();
    for (name, id) in [
        ("Циклы", "image.loops"),
        ("КороткоеВычисление", "image.short_circuit"),
        ("ИсключениеПробы", "image.exception"),
    ] {
        let (value, parameters) = instance.call(&mut state, name, vec![]).unwrap();
        assert!(parameters.is_empty());
        actual.push(format!(
            "{id}={}",
            open_bsl::format_value(&value, None).unwrap()
        ));
    }
    let expected = include_str!(
        "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
    )
    .lines()
    .filter(|line| {
        ["image.loops=", "image.short_circuit=", "image.exception="]
            .iter()
            .any(|prefix| line.starts_with(prefix))
    })
    .collect::<Vec<_>>()
    .join("\n");
    assert_eq!(actual.join("\n"), expected);
    assert!(
        instance
            .call(&mut state, "СчитатьВызов", vec![open_bsl::Value::Undefined])
            .is_err()
    );
}

#[test]
fn source_text_wins_even_when_an_image_body_is_unsupported() {
    let bytes = include_bytes!(
        "../../../tests/conformance/measure/1c/native-image-source-priority.Module.bin"
    );
    let resource = bytes_resource(&format!("{OWNER}.0"), bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let source = resources.module_source(OWNER, "0").unwrap().unwrap();
    assert_eq!(source, "Возврат 777;");
}

#[test]
fn unsupported_image_diagnostic_identifies_the_module_and_instruction() {
    let mut bytes = CONTAINER.to_vec();
    let start = bytes.windows(5).position(|part| part == b"{4,0}").unwrap();
    bytes[start + 1] = b'9';
    let resource = bytes_resource(&format!("{OWNER}.0"), &bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let error = resources.module_source(OWNER, "0").unwrap_err().to_string();
    assert!(error.contains(OWNER), "{error}");
    assert!(error.contains("image/Cmd[1]"), "{error}");
}

#[test]
fn an_unclosed_image_is_not_treated_as_an_absent_module() {
    let image = include_bytes!("../../../tests/conformance/measure/1c/native-image-constant.image");
    let mut bytes = CONTAINER.to_vec();
    let start = bytes
        .windows(image.len())
        .position(|part| part == image)
        .unwrap();
    bytes[start + image.len() - 1] = b' ';
    let resource = bytes_resource(&format!("{OWNER}.0"), &bytes);
    let resources = Resources::new(std::slice::from_ref(&resource)).unwrap();
    let error = resources.module_source(OWNER, "0").unwrap_err().to_string();
    assert!(error.contains(OWNER), "{error}");
    assert!(error.contains("image"), "{error}");
}

#[test]
fn selected_native_compiler_resource_becomes_an_executable_configuration_snapshot() {
    let root_id = "11111111-1111-1111-1111-111111111111";
    let configuration_id = "fb558588-a8fb-40be-91c0-b140be0c648c";
    let module_id = "f0d54940-82ea-4392-a3b5-2af94c9a7e00";
    let mut resources = vec![
        super::text_resource("root", &format!("{{2,{root_id},}}")),
        super::text_resource(
            root_id,
            &format!(
                "{{2,{{{root_id}}},1,{{9cd510cd-abfc-11d4-9434-004095e12fc7,{{1,{{68,{{0,{{3,{{1,0,{configuration_id}}},\"Конфигурация\"}}}}}},1,{{0fe48980-252d-11d6-a3c7-0050bae0a776,1,{module_id}}}}}}}}}"
            ),
        ),
        super::text_resource(&format!("{configuration_id}.7"), ""),
        super::text_resource(
            module_id,
            &format!(
                "{{1,{{12,{{2,{{1,0,{module_id}}},\"ЗамерыТранзитивногоРодителя\",{{1,\"ru\",\"ЗамерыТранзитивногоРодителя\"}},\"\",0,0,33333333-3333-3333-3333-333333333333}},0,1,0,0,0,0,0,1}},0}}"
            ),
        ),
        bytes_resource(
            &format!("{module_id}.0"),
            include_bytes!(
                "../../../tests/conformance/measure/1c/native-image-compiled.Module.bin"
            ),
        ),
        bytes_resource(&format!("{OWNER}.0"), b"unsupported unselected module"),
    ];
    super::retain_factory_resources(&mut resources).unwrap();
    assert!(
        !resources
            .iter()
            .any(|resource| resource.file_name == format!("{OWNER}.0"))
    );
    let snapshot = super::load_base(&resources).unwrap();
    assert_eq!(snapshot.identity(), configuration_id);
    let common = snapshot
        .modules()
        .iter()
        .find(|module| module.kind() == bsl_config::ModuleKind::Common)
        .unwrap();
    assert_eq!(common.id(), format!("{module_id}.0"));
    assert!(common.execution().server());
    assert_eq!(
        common.source(),
        crate::native_image::decompile(include_bytes!(
            "../../../tests/conformance/measure/1c/native-image-compiled.image"
        ))
        .unwrap()
    );
    let engine = open_bsl::Engine::builder()
        .configuration(open_bsl::ModuleGraphRecipe {
            modules: vec![open_bsl::ModuleRecipe {
                name: common.name().into(),
                source: common.source().into(),
                imports: vec![],
                visibility: open_bsl::ModuleVisibility::Named,
            }],
            eager_init: false,
        })
        .build()
        .unwrap();
    let output =
        crate::native_test_output::Output(std::rc::Rc::new(std::cell::RefCell::new(Vec::new())));
    let mut state = engine.state_builder().stdout(output.clone()).build();
    let entry = engine
        .compile_entry(include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.bsl"
        ))
        .unwrap();
    state.run(&entry).unwrap();
    let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
    assert_eq!(
        actual.trim(),
        include_str!(
            "../../../tests/conformance/measure/1c/measure-native-image-compiled.platform.txt"
        )
        .trim()
    );
}
