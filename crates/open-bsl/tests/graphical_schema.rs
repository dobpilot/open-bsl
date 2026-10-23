#![cfg(feature = "spreadsheet")]

use open_bsl::Engine;
use std::{cell::RefCell, io::Write, rc::Rc};

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn empty_graphical_schema_matches_the_native_russian_control() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(include_str!(
            "../../../docs/research/fixtures/graphical-schema/empty.bsl"
        ))
        .unwrap();
    let capture = Capture::default();
    engine
        .state_builder()
        .stdout(capture.clone())
        .build()
        .run(&module)
        .unwrap();
    assert_eq!(
        String::from_utf8(capture.0.borrow().clone()).unwrap(),
        include_str!("../../../docs/research/fixtures/graphical-schema/empty.platform.txt")
    );
}

#[test]
fn graphical_schema_constructor_rejects_arguments() {
    let engine = Engine::builder().build().unwrap();
    for name in ["ГрафическаяСхема", "GraphicalSchema"] {
        assert!(
            engine
                .compile(&format!("Возврат Новый {name}(1);"))
                .is_err()
        );
    }
}

#[test]
fn invalid_empty_lookup_preserves_the_native_status_and_assignment_target() {
    let engine = Engine::builder().build().unwrap();
    // У `Engine` без `Session` нет сервиса описания исключений. Убираем только получение
    // диагностического текста; вызовы и проверка сохранённых значений неизменны.
    let source = include_str!("../../../docs/research/fixtures/graphical-schema/find-invalid.bsl")
        .replace(
            "ОшибкаПоискаПробы = ОписаниеОшибки();",
            "ОшибкаПоискаПробы = \"\";",
        );
    let module = engine.compile(&source).unwrap();
    let capture = Capture::default();
    engine
        .state_builder()
        .stdout(capture.clone())
        .build()
        .run(&module)
        .unwrap();
    let output = String::from_utf8(capture.0.borrow().clone()).unwrap();
    // Позиции исходника и формулировки ошибок не являются контрактом этой пробы.
    // Сравниваем все шесть строк результата, сохраняя нативные ошибки в фикстуре.
    let statuses = |text: &str| {
        text.lines()
            .filter(|line| !line.split('\t').next().unwrap().ends_with(".error"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&output),
        statuses(include_str!(
            "../../../docs/research/fixtures/graphical-schema/find-invalid.platform.txt"
        ))
    );
}

#[test]
fn picture_location_matches_the_complete_native_control() {
    let engine = Engine::builder().build().unwrap();
    let module = engine
        .compile(include_str!(
            "../../../docs/research/fixtures/graphical-schema/picture-location.bsl"
        ))
        .unwrap();
    let capture = Capture::default();
    engine
        .state_builder()
        .stdout(capture.clone())
        .build()
        .run(&module)
        .unwrap();
    assert_eq!(
        String::from_utf8(capture.0.borrow().clone()).unwrap(),
        include_str!(
            "../../../docs/research/fixtures/graphical-schema/picture-location.platform.txt"
        )
    );
}

#[test]
fn picture_location_uses_existing_folded_lookup_and_rejects_unknown_members() {
    let engine = Engine::builder().build().unwrap();
    assert_eq!(
        engine.new_state().exec(
            "Возврат положениеКАРТИНКИэлементаГРАФИЧЕСКОЙсхемы.цЕНТР = graphicalschemaitempicturelocation.CENTER;"
        ).unwrap(),
        open_bsl::Value::Boolean(true)
    );
    assert!(
        engine
            .compile("Возврат ПоложениеКартинкиЭлементаГрафическойСхемы.Неверное;")
            .is_err()
    );
}

fn assert_native_control(source: &str, expected: &str) {
    let engine = Engine::builder().build().unwrap();
    let module = engine.compile(source).unwrap();
    let capture = Capture::default();
    engine
        .state_builder()
        .stdout(capture.clone())
        .build()
        .run(&module)
        .unwrap();
    assert_eq!(
        String::from_utf8(capture.0.borrow().clone()).unwrap(),
        expected
    );
}

#[test]
fn all_graphical_shapes_match_the_native_control() {
    assert_native_control(
        include_str!("../../../docs/research/fixtures/graphical-schema/shapes.bsl"),
        include_str!("../../../docs/research/fixtures/graphical-schema/shapes.platform.txt"),
    );
}

#[test]
fn all_graphical_line_and_arrow_enums_match_the_native_control() {
    assert_native_control(
        include_str!("../../../docs/research/fixtures/graphical-schema/line-enums.bsl"),
        include_str!("../../../docs/research/fixtures/graphical-schema/line-enums.platform.txt"),
    );
}

#[test]
fn graphical_shape_and_line_names_keep_folded_lookup_and_closed_member_sets() {
    let engine = Engine::builder().build().unwrap();
    for (name, member) in [
        ("GraphicalSchemaShapes", "Block"),
        ("ConnectorLineType", "Solid"),
        ("ConnectorTextLocation", "FirstSegment"),
        ("ArrowStyle", "Filled"),
    ] {
        assert_eq!(
            engine
                .new_state()
                .exec(&format!(
                    "Возврат {}.{} = {name}.{member};",
                    name.to_lowercase(),
                    member.to_uppercase()
                ))
                .unwrap(),
            open_bsl::Value::Boolean(true)
        );
        assert!(
            engine
                .compile(&format!("Возврат {name}.НетТакогоЧлена;"))
                .is_err()
        );
    }
}
