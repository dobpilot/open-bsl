use bsl_rt::{BslString, BslValue, BuiltinFn, call_builtin_fn};

#[test]
fn original_base64_accepts_the_measured_extension_hash_string_only() {
    let argument = BslValue::Str(BslString::from_str("m8U/yTkcx2G67LLpVzg36smnDVs="));
    assert_eq!(
        call_builtin_fn(BuiltinFn::Base64String, std::slice::from_ref(&argument)).unwrap(),
        BslValue::Str(BslString::from_str(""))
    );
    assert!(call_builtin_fn(BuiltinFn::GetBase64StringFromBinaryData, &[argument]).is_err());
}
