use super::*;

#[test]
fn linux_authorities_keep_native_identity_and_verified_transport() {
    if !cfg!(target_os = "linux") {
        assert!(new_linux_roots().is_err());
        return;
    }
    let roots = new_linux_roots().unwrap();
    assert_eq!(
        roots.object_ref().unwrap().type_descriptor().type_display,
        "СертификатыУдостоверяющихЦентровLinux"
    );
    assert!(
        roots
            .object_ref()
            .unwrap()
            .downcast_ref::<SystemRootsObject>()
            .is_none()
    );
    let connection = new_secure_connection(&[BslValue::Undefined, roots]).unwrap();
    let connection = connection
        .object_ref()
        .unwrap()
        .downcast_ref::<SecureConnectionObject>()
        .unwrap();
    assert!(matches!(connection.tls, TlsConfig::SystemRoots));
}

#[test]
fn linux_authority_constructor_has_both_native_names_and_no_parameters() {
    let constructor = crate::CONSTRUCTORS
        .iter()
        .find(|constructor| {
            constructor
                .names
                .contains(&"LinuxCertificationAuthorityCertificates")
        })
        .unwrap();
    assert_eq!(
        constructor.names,
        &[
            "СертификатыУдостоверяющихЦентровLinux",
            "LinuxCertificationAuthorityCertificates"
        ]
    );
    assert_eq!(constructor.arity, Arity::exact(0));
}
