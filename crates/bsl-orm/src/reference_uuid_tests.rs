use super::*;
use open_bsl::{HostEnv, RandomSource};

struct FixedRandom;
impl RandomSource for FixedRandom {
    fn fill(&mut self, bytes: &mut [u8; 16]) {
        *bytes = [0; 16];
    }
}

fn timestamp(uuid: [u8; 16]) -> u64 {
    u64::from(u32::from_be_bytes(uuid[..4].try_into().unwrap()))
        | (u64::from(u16::from_be_bytes(uuid[4..6].try_into().unwrap())) << 32)
        | (u64::from(u16::from_be_bytes(uuid[6..8].try_into().unwrap()) & 0x0fff) << 48)
}

#[test]
fn generation_uses_time_and_preserves_session_node_on_stopped_and_rewound_clocks() {
    let random = HostEnv::process().with_random(FixedRandom).random();
    let mut generator = ReferenceUuids::default();
    let first = generator.next(0, &random).unwrap();
    assert_eq!(timestamp(first), 122_192_928_000_000_000);
    assert_eq!(first[6] >> 4, 1);
    assert_eq!(first[8] >> 6, 2);
    assert_eq!(first[10] & 1, 1);
    let stopped = generator.next(0, &random).unwrap();
    let rewound = generator.next(-1000, &random).unwrap();
    assert_eq!(timestamp(stopped), timestamp(first) + 1);
    assert_eq!(timestamp(rewound), timestamp(first) + 2);
    assert_eq!(&first[8..], &stopped[8..]);
    assert_eq!(&first[8..], &rewound[8..]);
    assert_eq!(
        timestamp(generator.next(1000, &random).unwrap()),
        timestamp(first) + 10_000_000
    );
}

#[test]
fn out_of_range_clock_is_rejected_without_poisoning_generator() {
    let random = HostEnv::process().random();
    let mut generator = ReferenceUuids::default();
    assert!(generator.next(i64::MAX, &random).is_err());
    assert!(generator.next(i64::MIN, &random).is_err());
    assert!(generator.next(0, &random).is_ok());
}

struct FixedClock;
impl open_bsl::Clock for FixedClock {
    fn unix_millis(&mut self) -> i64 {
        0
    }
}

#[test]
fn reference_generation_uses_the_injected_clock_through_static_and_dynamic_calls() {
    use crate::{MetadataCatalog, MetadataKind, MetadataObject, Principal, SessionData};
    use open_bsl::{BslObject, Engine, Value};
    use std::{rc::Rc, sync::Arc};
    let catalog = Arc::new(
        MetadataCatalog::new(
            "uuid-test",
            vec![MetadataObject::new(
                "catalog",
                "Проба",
                MetadataKind::Catalog,
            )],
        )
        .unwrap(),
    );
    let mut builder = Engine::builder().register_library(crate::library());
    for ty in catalog.value_types() {
        builder = builder.register_value_type(Arc::clone(ty));
    }
    let engine = builder.build().unwrap();
    let mut state = engine
        .state_builder()
        .clock(FixedClock)
        .random(FixedRandom)
        .host_data(Rc::new(SessionData::new(
            catalog,
            Principal::new("user", "Тест", vec![]),
        )))
        .build();
    let first = state
        .exec("Возврат Справочники.Проба.ПолучитьСсылку().УникальныйИдентификатор();")
        .unwrap();
    let second = state
        .exec(
            r#"Возврат Вычислить("Справочники.Проба.ПолучитьСсылку().УникальныйИдентификатор()");"#,
        )
        .unwrap();
    let third = state
        .exec("Возврат Новый УникальныйИдентификатор();")
        .unwrap();
    let uuid = |value: Value| {
        let Value::Object(object) = value else {
            panic!("UUID");
        };
        let BslObject::Uuid(uuid) = object.as_ref() else {
            panic!("UUID");
        };
        *uuid
    };
    let first = uuid(first);
    let second = uuid(second);
    assert_eq!(timestamp(first), 122_192_928_000_000_000);
    assert_eq!(timestamp(second), timestamp(first) + 1);
    assert_eq!(first[6] >> 4, 1);
    assert_eq!(&first[8..], &second[8..]);
    assert_eq!(uuid(third)[6] >> 4, 4);
}
