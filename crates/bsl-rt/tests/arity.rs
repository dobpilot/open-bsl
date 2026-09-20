use bsl_rt::Arity;

#[test]
fn disjoint_counts_do_not_accept_the_gaps() {
    const NOTIFICATION: Arity = Arity::one_of(&[0, 2, 3, 5]);
    assert_eq!(NOTIFICATION.min(), 0);
    assert_eq!(NOTIFICATION.max(), 5);
    for count in 0..=u8::MAX {
        assert_eq!(NOTIFICATION.accepts(count), [0, 2, 3, 5].contains(&count));
    }
    assert_eq!(NOTIFICATION.to_string(), "0, 2, 3, 5");
}

#[test]
fn ranges_and_exact_counts_keep_their_contract() {
    for count in 0..=u8::MAX {
        assert_eq!(Arity::exact(3).accepts(count), count == 3);
        assert_eq!(Arity::range(2, 5).accepts(count), (2..=5).contains(&count));
        assert!(Arity::range(0, u8::MAX).accepts(count));
    }
    assert_eq!(Arity::exact(3).to_string(), "3");
    assert_eq!(Arity::range(2, 5).to_string(), "2…5");
    assert_eq!(Arity::one_of(&[255]).to_string(), "255");
}

#[test]
fn malformed_count_sets_are_rejected() {
    for counts in [&[][..], &[2, 2], &[3, 1]] {
        assert!(std::panic::catch_unwind(|| Arity::one_of(counts)).is_err());
    }
}

#[test]
fn method_dispatch_rejects_disallowed_counts() {
    fn call(
        _: &dyn bsl_rt::ObjectProtocol,
        _: &[bsl_rt::BslValue],
        _: &mut bsl_rt::CallContext<'_>,
    ) -> bsl_rt::RtResult<bsl_rt::BslValue> {
        unreachable!("проверка арности не вызывает обработчик")
    }
    let descriptor = bsl_rt::MethodDescriptor::new(&["Проба"], Arity::one_of(&[0, 2, 3, 5]), call);
    for count in 0..=6 {
        assert_eq!(
            descriptor.check_arity(count, "Тест").is_ok(),
            [0, 2, 3, 5].contains(&count)
        );
    }
}
