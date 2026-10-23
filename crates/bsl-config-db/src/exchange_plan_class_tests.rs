use super::*;

const NATIVE_CLASS: &str = "857c4a91-e5f4-4fac-86ec-787626f1c108";

#[test]
fn native_demo_exchange_plan_class_reaches_the_metadata_projection() {
    let evidence = include_str!(
        "../../../docs/research/fixtures/demo-exchange-plan-class-1016/native-root-db-names.log"
    );
    let classes = evidence
        .lines()
        .filter_map(|line| line.strip_prefix("exchange.root.class="))
        .map(|line| line.split_once(';').unwrap().0)
        .collect::<Vec<_>>();
    assert_eq!(classes.len(), 12);
    for class in classes {
        assert_eq!(class, NATIVE_CLASS);
        assert_eq!(metadata_kind(class), Some(MetadataKind::ExchangePlan));
    }
}

#[test]
fn native_exchange_plan_class_retains_both_existing_module_slots() {
    let slots = module_slots(NATIVE_CLASS);
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].kind, ModuleKind::Manager);
    assert_eq!(slots[0].suffix, "3");
    assert_eq!(slots[0].prefix, "ПланОбмена");
    assert_eq!(slots[1].kind, ModuleKind::Object);
    assert_eq!(slots[1].suffix, "2");
}

#[test]
fn misspelled_exchange_plan_class_is_not_a_platform_alias() {
    let typo = "857c4a91-e5f4-4fac-86ec-787626f1e108";
    assert_eq!(metadata_kind(typo), None);
    assert!(module_slots(typo).is_empty());
}
