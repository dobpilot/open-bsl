use open_bsl::decode_base64;

#[test]
fn acquired_hash_decoding_needs_only_the_facade() {
    assert_eq!(decode_base64("YWJj").unwrap(), b"abc");
    assert!(decode_base64("not base64!").is_none());
}
