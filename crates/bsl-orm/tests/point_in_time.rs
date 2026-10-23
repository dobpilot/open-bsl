use bsl_orm::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData};
use open_bsl::{Engine, Value};
use std::{rc::Rc, sync::Arc};

fn state(known_storage: bool) -> open_bsl::State {
    let objects = [
        ("ЗамерДокумента", 53),
        ("АльфаМомента", 56),
        ("ЯмаМомента", 55),
    ]
    .into_iter()
    .map(|(name, number)| {
        let object = MetadataObject::new(name, name, MetadataKind::Document);
        if known_storage {
            object.with_reference_storage_number(number)
        } else {
            object
        }
    })
    .collect();
    let catalog = Arc::new(MetadataCatalog::new("point-native", objects).unwrap());
    let mut builder = Engine::builder().register_library(bsl_orm::library());
    for value_type in catalog.value_types() {
        builder = builder.register_value_type(Arc::clone(value_type));
    }
    builder
        .build()
        .unwrap()
        .state_builder()
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build()
}

#[test]
fn cross_kind_point_comparisons_match_all_native_results() {
    let mut state = state(true);
    let names = ["ЗамерДокумента", "АльфаМомента", "ЯмаМомента"];
    let low = "00000000-0000-0000-0000-000000000001";
    let high = "ffffffff-ffff-ffff-ffff-ffffffffffff";
    let receipt = include_str!(
        "../../../docs/research/fixtures/document-point-in-time/cross-kind.platform.txt"
    );
    let mut checked = 0;
    for (first, second) in [(0, 1), (0, 2), (1, 2)] {
        for (case, first_uuid, second_uuid) in [
            ("same", low, low),
            ("low.high", low, high),
            ("high.low", high, low),
        ] {
            let setup = format!(
                "Первый = Новый МоментВремени(Дата(2024,1,2), Документы.{}.ПолучитьСсылку(Новый УникальныйИдентификатор(\"{first_uuid}\"))); Второй = Новый МоментВремени(Дата(2024,1,2), Документы.{}.ПолучитьСсылку(Новый УникальныйИдентификатор(\"{second_uuid}\")));",
                names[first], names[second]
            );
            for (direction, expression) in [
                ("forward", "Первый.Сравнить(Второй)"),
                ("reverse", "Второй.Compare(Первый)"),
                ("equal", "?(Первый = Второй, 1, 0)"),
            ] {
                let id = format!("kinds.{first}.{second}.{case}.{direction}");
                let expected = receipt
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once('\t')?;
                        (key == id).then(|| value.parse::<i64>().unwrap())
                    })
                    .unwrap();
                assert_eq!(
                    state
                        .exec(&format!("{setup} Возврат {expression};"))
                        .unwrap(),
                    Value::number_from_i64(expected),
                    "{id}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 27);
}

#[test]
fn unknown_cross_kind_storage_order_is_not_inferred_from_names() {
    let mut state = state(false);
    let error = state.exec("Первый = Новый МоментВремени(Дата(2024,1,2), Документы.ЗамерДокумента.ПустаяСсылка()); Второй = Новый МоментВремени(Дата(2024,1,2), Документы.АльфаМомента.ПустаяСсылка()); Возврат Первый.Сравнить(Второй);").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("порядок видов ссылок момента времени неизвестен"),
        "{error}"
    );
}

#[test]
fn same_kind_reference_comparisons_replay_native_scripts_without_storage_numbers() {
    let mut state = state(false);
    for (source, receipt, count) in [
        (
            include_str!("../../../docs/research/fixtures/document-point-in-time/references.bsl"),
            include_str!(
                "../../../docs/research/fixtures/document-point-in-time/references.platform.txt"
            ),
            18,
        ),
        (
            include_str!("../../../docs/research/fixtures/document-point-in-time/group-order.bsl"),
            include_str!(
                "../../../docs/research/fixtures/document-point-in-time/group-order.platform.txt"
            ),
            10,
        ),
    ] {
        // Меняется только приёмник сообщений; вычисления берутся из нативного скрипта.
        let source = source.replace("Сообщить(", "РезультатыПробы.Добавить(");
        let result = state
            .exec(&format!(
                "РезультатыПробы = Новый Массив; {source} Возврат РезультатыПробы;"
            ))
            .unwrap();
        let Value::Object(result) = result else {
            panic!("массив результатов");
        };
        let open_bsl::BslObject::Array(result) = result.as_ref() else {
            panic!("массив результатов");
        };
        let lines = result
            .borrow()
            .iter()
            .map(|value| open_bsl::format_value(value, None).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), count);
        assert_eq!(lines.join("\n"), receipt.trim_end());
    }
}
