use bsl_rt::{BslValue, lookup_enum, lookup_member};

#[test]
fn bare_subscriber_enum_matches_its_separate_native_control() {
    let kind = lookup_enum("ТипПодписчикаДоставляемыхУведомлений").unwrap();
    let value = BslValue::EnumType(kind);
    assert_eq!(
        value,
        BslValue::EnumType(lookup_enum("DeliverableNotificationSubscriberType").unwrap())
    );
    let BslValue::Type(value_type) = value.type_of().unwrap() else {
        panic!("ожидался тип голого перечисления");
    };
    let actual = format!(
        "SUBSCRIBER.BARE.BEGIN\t1\nSUBSCRIBER.BARE.TEXT\t{}\nSUBSCRIBER.BARE.TYPE\t{}\nSUBSCRIBER.BARE.ALIAS\tДа\nSUBSCRIBER.BARE.END\t1\n",
        kind.meta_ru_name(),
        value_type.name()
    );
    assert_eq!(
        actual,
        include_str!(
            "../../../../docs/research/fixtures/notification-subscriber-type/measure-bare.platform.txt"
        )
    );
}

#[test]
fn subscriber_values_match_the_complete_retained_native_control() {
    let mut lines = vec!["SUBSCRIBER.BEGIN\t1".to_owned()];
    let russian = lookup_enum("ТипПодписчикаДоставляемыхУведомлений").unwrap();
    let english = lookup_enum("deliverablenotificationsubscribertype").unwrap();
    assert_eq!(russian, english);
    for member in ["GCM", "APNS", "FCM"] {
        let value = lookup_member(russian, member).unwrap();
        let alias = lookup_member(english, &member.to_lowercase()).unwrap();
        assert_eq!(value, alias);
        lines.push(format!(
            "SUBSCRIBER.{member}.TEXT\t{}",
            value.display_text()
        ));
        let BslValue::Type(value_type) = BslValue::Enum(value).type_of().unwrap() else {
            panic!("ожидался тип члена перечисления");
        };
        lines.push(format!("SUBSCRIBER.{member}.TYPE\t{}", value_type.name()));
        lines.push(format!("SUBSCRIBER.{member}.ALIAS\tДа"));
    }
    lines.push("SUBSCRIBER.END\t1".to_owned());
    assert_eq!(
        lines.join("\n") + "\n",
        include_str!(
            "../../../../docs/research/fixtures/notification-subscriber-type/measure.platform.txt"
        )
    );
    assert_eq!(
        lookup_enum("типподписчикадоставляемыхуведомлений"),
        Some(russian)
    );
}
