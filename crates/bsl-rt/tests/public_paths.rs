//! Прежние пути и сигнатуры runtime при разделении внутренних модулей.

use bsl_rt::{BslValue, ComponentError, NameId, NameInterner, RtError, RtResult};
use std::fmt::{Debug, Display};
use std::hash::Hash;

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
