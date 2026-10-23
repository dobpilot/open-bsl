use super::decode;

#[test]
fn every_standard_tag_form_matches_independent_icu_vectors() {
    let vectors = include_str!("../../../tests/conformance/measure/1c/native-image-scsu-icu.tsv");
    for line in vectors.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3);
        let input = hex(fields[1]);
        let expected = String::from_utf8(hex(fields[2])).unwrap();
        assert_eq!(decode(&input).unwrap(), expected, "{}", fields[0]);
    }
}

#[test]
fn malformed_sequences_are_rejected_without_replacement_characters() {
    for bytes in [
        &[0x01][..],
        &[0x08][..],
        &[0x0b][..],
        &[0x0b, 0xe1][..],
        &[0x0c][..],
        &[0x0e][..],
        &[0x0e, 0xfe][..],
        &[0x18][..],
        &[0x1f][..],
        &[0x18, 0][..],
        &[0x18, 0xa8][..],
        &[0x18, 0xf8][..],
        &[0x0f, 0][..],
        &[0x0f, 0xe8][..],
        &[0x0f, 0xef][..],
        &[0x0f, 0xef, 0xa8][..],
        &[0x0f, 0xf0][..],
        &[0x0f, 0xf0, 0xfe][..],
        &[0x0f, 0xf1][..],
        &[0x0f, 0xf1, 0xe1][..],
        &[0x0f, 0xf2][..],
        &[0x0e, 0xd8, 0][..],
        &[0x0e, 0xdc, 0][..],
        &[0x0e, 0xd8, 0, b'A'][..],
        &[0x0f, 0xd8, 0, 0, b'A'][..],
    ] {
        let error = decode(bytes).unwrap_err();
        assert!(error.starts_with("image/SCSU: смещение"), "{error}");
    }
}

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
