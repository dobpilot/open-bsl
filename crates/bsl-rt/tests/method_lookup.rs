use bsl_rt::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectProtocol, RtResult,
    find_method_from_table,
};

fn must_not_run(
    _: &dyn ObjectProtocol,
    _: &[BslValue],
    _: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    panic!("поиск метода не должен вызывать обработчик")
}

static METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["Изменить", "Mutate"],
        Arity::one_of(&[0, 2, 3, 5]),
        must_not_run,
    ),
    MethodDescriptor::new(
        &["Другой", "Other", "Straße"],
        Arity::exact(1),
        must_not_run,
    ),
];

#[test]
fn lookup_returns_the_original_descriptor_without_calling_it() {
    for name in ["Изменить", "ИЗМЕНИТЬ", "изменить", "Mutate", "mUtAtE"] {
        let found = find_method_from_table(METHODS, name).unwrap();
        assert!(std::ptr::eq(found, &METHODS[0]));
        assert!(!found.arity().accepts(1));
    }
    assert!(std::ptr::eq(
        find_method_from_table(METHODS, "other").unwrap(),
        &METHODS[1]
    ));
    assert!(std::ptr::eq(
        find_method_from_table(METHODS, "STRASSE").unwrap(),
        &METHODS[1]
    ));
}

#[test]
fn lookup_does_not_trim_names_or_invent_missing_methods() {
    for name in ["", "Изменить ", " Изменить", "Mutate()", "Unknown"] {
        assert!(find_method_from_table(METHODS, name).is_none());
    }
    assert!(find_method_from_table(&[], "Изменить").is_none());
}

#[derive(Debug)]
struct StaticReceiver;

impl ObjectProtocol for StaticReceiver {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        panic!("поиск не требует описания типа")
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        METHODS
    }
}

#[derive(Debug)]
struct DynamicReceiver;

impl ObjectProtocol for DynamicReceiver {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        panic!("поиск не требует описания типа")
    }

    fn has_method(&self, name: &str) -> bool {
        ["Экспортный", "Exported"]
            .iter()
            .any(|candidate| bsl_rt::folded_eq(candidate, name))
    }

    fn call_method(&self, _: &str, _: &[BslValue], _: &mut CallContext<'_>) -> RtResult<BslValue> {
        panic!("проверка наличия не вызывает динамический метод")
    }
}

#[derive(Debug)]
struct LegacyReceiver;

impl ObjectProtocol for LegacyReceiver {
    fn type_descriptor(&self) -> &'static bsl_rt::TypeDescriptor {
        panic!("поиск не требует описания типа")
    }

    fn call_method(&self, _: &str, _: &[BslValue], _: &mut CallContext<'_>) -> RtResult<BslValue> {
        panic!("наличие строкового входа не объявляет доступность метода")
    }
}

#[test]
fn static_receiver_uses_the_shared_lookup_without_calling_handlers() {
    let receiver = bsl_rt::ObjectRef::new(StaticReceiver);
    for name in ["изменить", "mUtAtE", "STRASSE"] {
        assert!(receiver.has_method(name));
    }
    for name in ["", "Unknown", " Изменить", "Mutate "] {
        assert!(!receiver.has_method(name));
    }
}

#[test]
fn dynamic_receiver_explicitly_declares_methods_without_dispatch() {
    let receiver = bsl_rt::ObjectRef::new(DynamicReceiver);
    assert!(receiver.method_table().is_empty());
    for name in ["ЭКСПОРТНЫЙ", "exported"] {
        assert!(receiver.has_method(name));
    }
    for name in ["Неэкспортный", " Exported", "Exported ", ""] {
        assert!(!receiver.has_method(name));
    }
    let legacy = bsl_rt::ObjectRef::new(LegacyReceiver);
    assert!(!legacy.has_method("Exported"));
}
