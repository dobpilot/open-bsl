use super::*;

#[test]
fn sha512_replays_native_empty_incremental_and_repeated_snapshots() {
    let mut hasher = Hasher::new(EnumValue::HashSha512).unwrap();
    assert_eq!(
        hex(&hasher.snapshot()),
        "CF83E1357EEFB8BDF1542850D66D8007D620E4050B5715DC83F4A921D36CE9CE47D0D13C5D85F2B0FF8318D2877EEC2F63B931BD47417A81A538327AF927DA3E"
    );
    hasher.update(b"a");
    let first = hasher.snapshot();
    assert_eq!(first, hasher.snapshot());
    hasher.update(b"bc");
    assert_eq!(
        hex(&hasher.snapshot()),
        "DDAF35A193617ABACC417349AE20413112E6FA4E89A97EA20A9EEEE64B55D39A2192992A274FC1A836BA3C23A3FEEBBD454D4423643CE80E2A9AC94FA54CA49F"
    );
    let mut whole = Hasher::new(EnumValue::HashSha512).unwrap();
    whole.update(b"abc");
    assert_eq!(whole.snapshot(), hasher.snapshot());
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}
