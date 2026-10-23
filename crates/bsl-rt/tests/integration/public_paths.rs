//! Прежние пути и сигнатуры runtime при разделении внутренних модулей.

use bsl_rt::{BslValue, ComponentError, NameId, NameInterner, RtError, RtResult};
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[test]
fn legacy_http_spawner_keeps_its_implementation_and_rejects_file_requests() {
    struct Legacy;
    impl bsl_rt::HttpPromiseSpawner for Legacy {
        fn spawn_http(
            &mut self,
            _: std::sync::Arc<dyn bsl_rt::HttpClient>,
            _: bsl_rt::HttpWireRequest,
            _: bsl_rt::HttpResponseMapper,
            _: bsl_rt::HttpErrorMapper,
        ) -> RtResult<BslValue> {
            panic!("файловый запрос не должен обращаться к HTTP")
        }
    }
    let mut legacy = Legacy;
    let spawner: &mut dyn bsl_rt::HostPromiseSpawner = &mut legacy;
    assert!(matches!(
        spawner.ready_file_promise(Ok(BslValue::Undefined)),
        Err(RtError::IoError(_))
    ));
    let files = std::rc::Rc::new(bsl_rt::SystemFileSystem);
    let zone = std::rc::Rc::new(bsl_rt::FixedTimeZone::new(0).unwrap());
    assert!(matches!(
        spawner.spawn_file_operation(
            Ok(bsl_rt::FileOperationRequest::TemporaryDirectory), files, zone,
        ),
        Err(RtError::IoError(message)) if message.contains("файловые обещания")
    ));
}

#[test]
fn simple_mask_is_available_through_the_runtime_facade() {
    let matches: fn(&str, &str) -> bool = bsl_rt::simple_mask_matches;
    assert!(matches("?.txt", "я.txt"));
    assert!(!matches("[ab]", "a"));
}

#[test]
fn directory_noop_preparation_is_available_through_the_runtime_facade() {
    let prepare: fn(&[BslValue]) -> Option<BslValue> = bsl_rt::prepare_create_directory_noop;
    for path in [
        ".", "./", "..", "../", "./.", "././", ".//", "../.", ".././", "..//", "./..", "./../",
        "./../.", "../../", "/./", "/../",
    ] {
        let value = BslValue::Str(bsl_rt::BslString::from_str(path));
        assert_eq!(prepare(std::slice::from_ref(&value)), Some(value), "{path}");
    }
    let path = BslValue::Str(bsl_rt::BslString::from_str("private/leaf"));
    assert_eq!(prepare(&[path]), None);
}

#[test]
fn value_and_error_keep_their_root_paths_and_traits() {
    fn value_traits<T: Clone + Debug + Display + Eq + Hash>() {}
    fn error_traits<T: Clone + Debug + Display + PartialEq + std::error::Error>() {}
    value_traits::<BslValue>();
    error_traits::<RtError>();
    let value: RtResult<BslValue> = Ok(BslValue::Undefined);
    assert_eq!(value, Ok(BslValue::Undefined));
    let error = ComponentError {
        package: "public-paths",
        kind: "проверка",
        message: "ошибка".to_string(),
    };
    assert_eq!(error.clone(), error);
    assert!(matches!(
        RtError::UnknownField(NameId::from_index(0)),
        RtError::UnknownField(_)
    ));
}

#[test]
fn domain_methods_keep_their_inherent_signatures() {
    let _: fn(&BslValue, &BslValue) -> RtResult<BslValue> = BslValue::add;
    let _: fn(&BslValue) -> RtResult<bool> = BslValue::as_condition;
    let _: fn(&BslValue) -> RtResult<usize> = BslValue::str_len;
    let _: fn(&[BslValue]) -> RtResult<BslValue> = BslValue::make_date;
    let _: fn(Vec<BslValue>) -> BslValue = BslValue::new_array;
    let _: fn(&BslValue, &BslValue, &NameInterner) -> RtResult<BslValue> = BslValue::get_index;
    let _: fn(&BslValue, &BslValue, BslValue) -> RtResult<()> = BslValue::set_index;
    let _: fn() -> BslValue = BslValue::new_table;
    let _: fn(&BslValue, &BslValue) -> RtResult<BslValue> = BslValue::table_total;
    let _: fn(&BslValue) -> RtResult<BslValue> = BslValue::text_writer_close;
    let _: fn(&BslValue, &BslValue) -> RtResult<BslValue> = BslValue::binary_data_split;
    let _: fn(&RtError) -> bool = RtError::is_bsl_exception;
    assert_eq!(
        BslValue::binary_data_of(vec![1, 2]).binary_data_bytes(),
        Some(&[1, 2][..])
    );
}

// Снимок исходной сборки 408f4d5 на x86-64 Linux, rustc 1.97.1.
// Это защита текущего переноса, а не обещание стабильного Rust ABI.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[test]
fn value_and_error_keep_the_baseline_layout() {
    assert_eq!(std::mem::size_of::<BslValue>(), 24);
    assert_eq!(std::mem::align_of::<BslValue>(), 8);
    assert_eq!(std::mem::size_of::<RtError>(), 48);
    assert_eq!(std::mem::align_of::<RtError>(), 8);
}
