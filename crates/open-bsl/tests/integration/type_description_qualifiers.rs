use open_bsl::{BslObject, Engine, Value};

fn replay(source: &str, expected: &str) {
    let engine = Engine::builder().build().unwrap();
    let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
    let result = engine
        .new_state()
        .exec(&format!(
            "РезультатыПробы = Новый Массив;\n{source}\nВозврат РезультатыПробы;"
        ))
        .unwrap();
    let Value::Object(object) = result else {
        panic!("массив результатов")
    };
    let BslObject::Array(lines) = object.as_ref() else {
        panic!("массив результатов")
    };
    let actual = lines
        .borrow()
        .iter()
        .map(|value| {
            let Value::Str(line) = value else {
                panic!("строка результата")
            };
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(actual, expected);
}

#[test]
fn primitive_type_description_qualifiers_match_the_native_receipt() {
    replay(
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-values.bsl"
        ),
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-values.platform.txt"
        ),
    );
}

#[test]
fn fixed_string_qualifiers_and_adjustment_match_the_native_receipt() {
    replay(
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-fixed.bsl"
        ),
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-fixed.platform.txt"
        ),
    );
}

#[test]
fn qualifier_aliases_and_number_assignment_match_the_native_receipt() {
    replay(
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-aliases.bsl"
        ),
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-aliases.platform.txt"
        ),
    );
}

#[test]
fn string_length_policy_encoding_matches_the_native_receipt() {
    replay(
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-encoding.bsl"
        ),
        include_str!(
            "../../../../docs/research/fixtures/type-description-qualifiers/measure-encoding.platform.txt"
        ),
    );
}
