use open_bsl::{Engine, ModuleGraphRecipe, ModuleRecipe, ModuleVisibility};
use std::{cell::RefCell, io, io::Write, rc::Rc};

const PARENT: &str = "ЗамерыТранзитивногоРодителя";
const CHILD: &str = "ЗамерыТранзитивногоДочернего";
const PARENT_SOURCE: &str = include_str!(
    "../../../../tests/conformance/measure/1c/unknown-common-method-parent-module.bsl"
);
const CHILD_SOURCE: &str =
    include_str!("../../../../tests/conformance/measure/1c/unknown-common-method-child-module.bsl");

#[derive(Clone)]
struct Output(Rc<RefCell<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn recipe(visibility: ModuleVisibility, parent_source: &str) -> ModuleGraphRecipe {
    ModuleGraphRecipe {
        modules: vec![
            ModuleRecipe {
                name: CHILD.into(),
                source: CHILD_SOURCE.into(),
                imports: vec![],
                visibility: ModuleVisibility::Named,
            },
            ModuleRecipe {
                name: PARENT.into(),
                source: parent_source.into(),
                imports: vec![],
                visibility,
            },
        ],
        eager_init: false,
    }
}

#[test]
fn missing_imported_methods_match_native_entry_failure_without_body_or_arguments() {
    for (rooted, visibility) in [
        (false, ModuleVisibility::Named),
        (true, ModuleVisibility::Named),
        (false, ModuleVisibility::Global),
        (true, ModuleVisibility::Global),
    ] {
        let mut builder = Engine::builder()
            .defer_common_module_name_errors(true)
            .configuration(recipe(visibility, PARENT_SOURCE));
        if rooted {
            builder = builder.configuration_roots([PARENT]);
        }
        let engine = builder.build().unwrap();
        let output = Output(Rc::new(RefCell::new(Vec::new())));
        let mut state = engine.state_builder().stdout(output.clone()).build();
        let source = include_str!(
            "../../../../tests/conformance/measure/1c/measure-module-unknown-common-method.bsl"
        );
        let source = if visibility == ModuleVisibility::Global {
            source.replace(&format!("{PARENT}."), "")
        } else {
            source.into()
        };
        let module = engine.compile_entry(&source).unwrap();
        state.run(&module).unwrap();
        let actual = String::from_utf8(output.0.borrow().clone()).unwrap();
        let expected = include_str!(
            "../../../../tests/conformance/measure/1c/measure-module-unknown-common-method.platform.txt"
        );
        assert_eq!(actual.trim(), expected.trim());
    }
}

#[test]
fn strict_default_and_non_common_modules_keep_their_diagnostics() {
    let error = Engine::builder()
        .configuration(recipe(ModuleVisibility::Named, PARENT_SOURCE))
        .build()
        .err()
        .expect("ожидалась ошибка связывания")
        .to_string();
    assert!(error.contains("НетТакогоМетодаПробы"), "{error}");
    for visibility in [ModuleVisibility::Private, ModuleVisibility::Explicit] {
        let error = Engine::builder()
            .defer_common_module_name_errors(true)
            .configuration(recipe(visibility, PARENT_SOURCE))
            .build()
            .err()
            .expect("ожидалась ошибка связывания")
            .to_string();
        assert!(error.contains("НетТакогоМетодаПробы"), "{error}");
    }
}
