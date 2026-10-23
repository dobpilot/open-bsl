use super::*;

fn unicode_path(extra: &[u8]) -> &[u8] {
    let mut at = 0;
    while at + 4 <= extra.len() {
        let id = u16::from_le_bytes([extra[at], extra[at + 1]]);
        let len = usize::from(u16::from_le_bytes([extra[at + 2], extra[at + 3]]));
        if id == 0x7075 {
            return &extra[at..at + 4 + len];
        }
        at += 4 + len;
    }
    panic!("нет Unicode Path");
}

fn headers(bytes: &[u8]) -> [(u16, Vec<u8>, Vec<u8>); 2] {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let file = archive.by_index(0).unwrap();
    [
        (file.header_start() as usize, 6, 26, 30),
        (file.central_header_start() as usize, 8, 28, 46),
    ]
    .map(|(at, flags, name, size)| {
        let n = usize::from(u16::from_le_bytes([bytes[at + name], bytes[at + name + 1]]));
        let x = usize::from(u16::from_le_bytes([
            bytes[at + name + 2],
            bytes[at + name + 3],
        ]));
        (
            u16::from_le_bytes([bytes[at + flags], bytes[at + flags + 1]]),
            bytes[at + size..at + size + n].to_vec(),
            unicode_path(&bytes[at + size + n..at + size + n + x]).to_vec(),
        )
    })
}

#[test]
fn os_utf8_headers_match_the_native_linux_archive() {
    let oracle = include_str!(
        "../../../../docs/research/fixtures/zip-os-utf8-encoding/measure-archive.platform.txt"
    );
    let encoded = oracle
        .lines()
        .find(|line| line.starts_with("zip.archive.КодировкаОСДополнительноUTF8\t"))
        .unwrap()
        .split_once('\t')
        .unwrap()
        .1;
    let native = bsl_rt::encoding::decode_base64(encoded).unwrap();
    let expected = headers(&native);
    let entry = PendingEntry {
        name: String::from_utf8(expected[0].1.clone()).unwrap(),
        data: b"probe".to_vec(),
        method: WriteMethod::Stored,
        time: 0,
        date: 0,
        directory: false,
    };
    let actual = build_archive_with_encoding(&[entry], "", true).unwrap();
    assert_eq!(headers(&actual), expected);
}

#[test]
#[cfg(target_os = "linux")]
fn opening_the_writer_keeps_and_resets_the_requested_name_encoding() {
    let mut args = vec![BslValue::Undefined; 7];
    args[6] = BslValue::Enum(EnumValue::ZipNamesOsWithUtf8);
    let value = new_archive_writer(true, &args, Rc::new(bsl_rt::SystemFileSystem)).unwrap();
    let writer: &WriterObject = value.object_ref().unwrap().downcast_ref().unwrap();
    assert!(writer.os_utf8.get());
    writer.state.borrow_mut().entries.push(PendingEntry {
        name: "имя.txt".into(),
        data: b"probe".to_vec(),
        method: WriteMethod::Stored,
        time: 0,
        date: 0,
        directory: false,
    });
    let binary = writer_binary_data(writer).unwrap();
    for (flags, name, extra) in headers(binary.binary_data_bytes().unwrap()) {
        assert_eq!(flags & 0x0800, 0);
        assert_eq!(name, "имя.txt".as_bytes());
        assert!(!unicode_path(&extra).is_empty());
    }
    let directory =
        std::env::temp_dir().join(format!("open-bsl-zip-os-utf8-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("archive.zip");
    args[0] = BslValue::Str(BslString::from_str(path.to_str().unwrap()));
    writer_open(writer, &args).unwrap();
    writer_write(writer, &bsl_rt::SystemFileSystem).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    for (flags, _, extra) in headers(&bytes) {
        assert_eq!(flags & 0x0800, 0);
        assert!(!unicode_path(&extra).is_empty());
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
    writer_open(
        writer,
        &[BslValue::Str(BslString::from_str("mode-reset.zip"))],
    )
    .unwrap();
    assert!(!writer.os_utf8.get());
}
