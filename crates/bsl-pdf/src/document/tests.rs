//! Тесты чтения PDF и поверхности «ДокументPDF» — над общими фикстурами.

use super::*;
use crate::writer::{PaintMode, PdfDocument, PdfFont};
use std::path::PathBuf;

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// --- чтение контейнера --------------------------------------------

/// Собрать классический файл из готовых тел объектов: заголовок,
/// объекты подряд, таблица `xref` и трейлер. Ровно то, что делает
/// [`PdfDocument::write`], только без содержимого страниц — здесь
/// проверяется КОНТЕЙНЕР.
fn build_classic(objects: &[(u32, Vec<u8>)], trailer_extra: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1;
    let mut offsets = vec![0usize; size as usize];
    for (number, body) in objects {
        offsets[*number as usize] = out.len();
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root 1 0 R{trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

fn empty_content() -> Vec<u8> {
    b"<< /Length 0 >>\nstream\n\nendstream".to_vec()
}

/// Однолистовой файл с заданным словарём страницы.
fn one_page(page: &str) -> Vec<u8> {
    build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
            (3, page.as_bytes().to_vec()),
            (4, empty_content()),
        ],
        "",
    )
}

/// Собственный вывод писателя обязан читаться собственным читателем:
/// число страниц и их размеры сходятся с тем, что было задано.
#[test]
fn pdf_reader_reads_the_writer_output() {
    let mut doc = PdfDocument::new();
    let first = doc.add_page(595.32, 841.92).unwrap();
    doc.text(first, 40.0, 800.0, PdfFont::Courier, 12.0, "Накладная № 7")
        .unwrap();
    let second = doc.add_page(841.92, 595.32).unwrap();
    doc.rect(second, 10.0, 10.0, 100.0, 50.0, PaintMode::Stroke)
        .unwrap();
    let bytes = doc.write().unwrap();

    let file = PdfFile::parse(&bytes).expect("свой же вывод обязан читаться");
    assert_eq!(file.page_count(), 2);
    assert_eq!(file.page(0).unwrap().width_mm(), 210);
    assert_eq!(file.page(0).unwrap().height_mm(), 297);
    assert_eq!(file.page(1).unwrap().width_mm(), 297);
    assert_eq!(file.page(1).unwrap().height_mm(), 210);
    assert_eq!(file.page(0).unwrap().rotate(), 0);
}

/// ВСЕ закоммиченные файлы, снятые с платформы, обязаны читаться, и
/// число страниц в каждом — совпасть с тем, ради чего файл снимали
/// (происхождение всех — `tests/conformance/pdf/capture-platform-pdf*.bsl`).
#[test]
fn pdf_reader_reads_every_committed_platform_file() {
    let expected: &[(&str, usize)] = &[
        // Три файла задачи о вложениях: снятый с платформы и два
        // собранных НАШИМ писателем поверх чужой основы (см.
        // `make-open-bsl-attachments.bsl`).
        ("attach-platform.pdf", 1),
        ("attach-open-bsl.pdf", 1),
        ("attach-open-bsl-xrefstream.pdf", 2),
        ("platform-simple.pdf", 1),
        ("probe-align.pdf", 1),
        ("probe-big.pdf", 4),
        ("probe-border.pdf", 1),
        ("probe-color.pdf", 1),
        ("probe-colwidth.pdf", 1),
        ("probe-empty.pdf", 1),
        ("probe-fit.pdf", 1),
        ("probe-font.pdf", 1),
        ("probe-grid.pdf", 1),
        ("probe-landscape.pdf", 1),
        ("probe-line.pdf", 1),
        ("probe-margins.pdf", 1),
        ("probe-merge.pdf", 1),
        ("probe-number.pdf", 1),
        ("probe-numeric.pdf", 1),
        ("probe-pages.pdf", 2),
        ("probe-rowheight.pdf", 1),
        ("probe-wide.pdf", 2),
    ];
    let dir = PathBuf::from("../../tests/conformance/pdf");
    let mut checked = 0usize;
    for (name, pages) in expected {
        let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| {
            panic!("файл {name} обязан лежать в дереве: {e}");
        });
        let file = PdfFile::parse(&bytes)
            .unwrap_or_else(|e| panic!("платформенный файл {name} обязан читаться: {e:?}"));
        assert_eq!(file.page_count(), *pages, "число страниц в {name}");
        for i in 0..file.page_count() {
            let page = file.page(i).unwrap();
            assert!(
                page.width_mm() > 0 && page.height_mm() > 0,
                "страница {i} файла {name} без размера"
            );
        }
        checked += 1;
    }
    // Ни один файл каталога не должен остаться неучтённым: новый
    // снимок с платформы обязан попасть в таблицу выше.
    let on_disk = std::fs::read_dir(&dir)
        .expect("каталог со снимками обязан существовать")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "pdf"))
        .count();
    assert_eq!(checked, on_disk, "в каталоге появился неучтённый PDF");
}

/// Альбомный лист платформы: `probe-landscape.pdf` снят с документа,
/// у которого `ОриентацияСтраницы = Ландшафт`.
#[test]
fn pdf_reader_sees_the_platform_landscape_page() {
    let bytes = std::fs::read("../../tests/conformance/pdf/probe-landscape.pdf").unwrap();
    let file = PdfFile::parse(&bytes).unwrap();
    let page = file.page(0).unwrap();
    assert_eq!((page.width_mm(), page.height_mm()), (297, 210));
}

/// Размеры отдаются в миллиметрах с округлением к ближайшему, начало
/// рамки не важно (измерено, см. обзор модуля).
#[test]
fn pdf_reader_converts_points_to_millimetres() {
    let cases: &[(&str, i64, i64)] = &[
        ("[ 0 0 100 200 ]", 35, 71),
        ("[ 10 20 110 220 ]", 35, 71),
        ("[ 0 0 100.62 200 ]", 35, 71),
        ("[ 0 0 100.64 200 ]", 36, 71),
        ("[ 0 0 595.32 841.92 ]", 210, 297),
    ];
    for (rect, width, height) in cases {
        let pdf = one_page(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
        ));
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!(
            (page.width_mm(), page.height_mm()),
            (*width, *height),
            "рамка {rect}"
        );
    }
}

/// Рамка без площади — ноль на ноль по обеим сторонам сразу, а страница
/// без рамки вовсе — US Letter (измерено).
#[test]
fn pdf_reader_gives_zero_for_degenerate_boxes() {
    for rect in ["[ 595.32 0 0 841.92 ]", "[ 0 0 0 0 ]", "[ 0 0 -100 -200 ]"] {
        let pdf = one_page(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {rect} /Contents 4 0 R >>"
        ));
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        assert_eq!((page.width_mm(), page.height_mm()), (0, 0), "рамка {rect}");
    }
    let pdf = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
    let file = PdfFile::parse(&pdf).unwrap();
    let page = file.page(0).unwrap();
    assert_eq!((page.width_mm(), page.height_mm()), (216, 279));
}

/// `/CropBox` задаёт видимую область и ПЕРЕСЕКАЕТСЯ с `/MediaBox`;
/// обе рамки наследуются от узла `/Pages`, а `/Rotate` — нет
/// (измерено).
#[test]
fn pdf_reader_inherits_boxes_but_not_rotation() {
    let pdf = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R 4 0 R 5 0 R ] /Count 3 \
                   /MediaBox [ 0 0 595.32 841.92 ] /CropBox [ 50 60 545.32 781.92 ] \
                   /Rotate 90 >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>".to_vec(),
            ),
            (
                4,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                   /CropBox [ -100 -100 700 900 ] /Contents 6 0 R >>"
                    .to_vec(),
            ),
            (
                5,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
                   /Rotate 180 /Contents 6 0 R >>"
                    .to_vec(),
            ),
            (6, empty_content()),
        ],
        "",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_eq!(file.page_count(), 3);
    // Унаследованный `/CropBox`.
    let first = file.page(0).unwrap();
    assert_eq!((first.width_mm(), first.height_mm()), (175, 255));
    // Наследуется, но `/Rotate` узла на страницу НЕ переходит.
    assert_eq!(first.rotate(), 0);
    // `/CropBox` шире `/MediaBox` — берётся пересечение.
    let second = file.page(1).unwrap();
    assert_eq!((second.width_mm(), second.height_mm()), (210, 297));
    // Собственный `/Rotate` страницы виден.
    assert_eq!(file.page(2).unwrap().rotate(), 180);
}

/// `/Rotate` приводится к кратному прямому углу — все десять значений,
/// снятых с платформы.
#[test]
fn pdf_reader_normalises_rotation_like_the_platform() {
    let cases: &[(&str, i64)] = &[
        ("0", 0),
        ("90", 90),
        ("180", 180),
        ("270", 270),
        ("-90", 270),
        ("450", 90),
        ("44", 0),
        ("45", 90),
        ("46", 90),
        ("135", 180),
        ("315", 0),
        ("350", 0),
    ];
    for (raw, expected) in cases {
        let pdf = one_page(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] \
             /Rotate {raw} /Contents 4 0 R >>"
        ));
        let file = PdfFile::parse(&pdf).unwrap();
        assert_eq!(file.page(0).unwrap().rotate(), *expected, "/Rotate {raw}");
    }
}

/// Инкрементальное обновление: вторая таблица `xref` с `/Prev`
/// перекрывает объект 2 и добавляет объект 5.
#[test]
fn pdf_reader_follows_the_prev_chain() {
    let base = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ],
        "",
    );
    let previous = find(&base, b"xref\n0 ").expect("в базовом файле есть таблица");
    let mut out = base.clone();
    let five = out.len();
    out.extend_from_slice(
        b"5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] \
          /Contents 4 0 R >>\nendobj\n",
    );
    let two = out.len();
    out.extend_from_slice(
        b"2 0 obj\n<< /Type /Pages /Kids [ 3 0 R 5 0 R ] /Count 2 \
          /MediaBox [ 0 0 595.32 841.92 ] >>\nendobj\n",
    );
    let xref = out.len();
    out.extend_from_slice(
        format!(
            "xref\n0 1\n0000000000 65535 f \n2 1\n{two:010} 00000 n \n5 1\n{five:010} 00000 n \n\
             trailer\n<< /Size 6 /Root 1 0 R /Prev {previous} >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );

    let file = PdfFile::parse(&out).expect("инкрементальное обновление обязано читаться");
    assert_eq!(file.page_count(), 2);
    assert_eq!(file.page(0).unwrap().width_mm(), 210);
    assert_eq!(file.page(1).unwrap().width_mm(), 297);
    // Базовый файл в одиночку по-прежнему даёт одну страницу — значит
    // прочиталась именно цепочка, а не хвост.
    assert_eq!(PdfFile::parse(&base).unwrap().page_count(), 1);
}

/// Файл PDF 1.5: `xref`-поток с предиктором PNG «Up» и объектный поток,
/// в котором лежат каталог, узел `/Pages` и обе страницы.
///
/// Собран байтами прямо здесь по разделам 7.5.7 и 7.5.8 спецификации:
/// ни qpdf, ни pikepdf на машине нет, а снимать такой файл с платформы
/// нечем — её писатель кладёт классическую таблицу.
#[test]
fn pdf_reader_reads_xref_stream_with_object_stream() {
    let inner: &[(u32, &str)] = &[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [ 3 0 R 4 0 R ] /Count 2 /MediaBox [ 0 0 595.32 841.92 ] >>",
        ),
        (3, "<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>"),
        (
            4,
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 841.92 595.32 ] /Contents 6 0 R >>",
        ),
    ];
    let mut head = String::new();
    let mut body = String::new();
    for (number, text) in inner {
        head.push_str(&format!("{number} {} ", body.len()));
        body.push_str(text);
        body.push(' ');
    }
    let first = head.len();
    let objstm_plain = format!("{head}{body}").into_bytes();
    let objstm = zlib_compress(&objstm_plain);

    let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let at_five = out.len();
    out.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
            inner.len(),
            objstm.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&objstm);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let at_six = out.len();
    out.extend_from_slice(b"6 0 obj\n");
    out.extend_from_slice(&empty_content());
    out.extend_from_slice(b"\nendobj\n");
    let at_xref = out.len();

    // Записи шириной [1 4 2]; строки предсказаны фильтром PNG «Up»
    // (тип 2), то есть каждая строка — разность с предыдущей.
    let mut rows: Vec<[u8; 7]> = Vec::new();
    let mut push = |kind: u8, second: u32, third: u16| {
        let mut row = [0u8; 7];
        row[0] = kind;
        row[1..5].copy_from_slice(&second.to_be_bytes());
        row[5..7].copy_from_slice(&third.to_be_bytes());
        rows.push(row);
    };
    push(0, 0, 65535);
    for index in 0..inner.len() {
        push(2, 5, index as u16);
    }
    push(1, at_five as u32, 0);
    push(1, at_six as u32, 0);
    push(1, at_xref as u32, 0);
    let mut predicted = Vec::new();
    let mut previous = [0u8; 7];
    for row in &rows {
        predicted.push(2u8);
        for i in 0..7 {
            predicted.push(row[i].wrapping_sub(previous[i]));
        }
        previous = *row;
    }
    let xref_stream = zlib_compress(&predicted);
    out.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /W [ 1 4 2 ] /Root 1 0 R /Filter /FlateDecode \
             /DecodeParms << /Predictor 12 /Columns 7 >> /Length {} >>\nstream\n",
            xref_stream.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&xref_stream);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());

    let file = PdfFile::parse(&out).expect("xref-поток с ObjStm обязан читаться");
    assert_eq!(file.page_count(), 2);
    assert_eq!(file.page(0).unwrap().width_mm(), 210);
    assert_eq!(file.page(1).unwrap().width_mm(), 297);
}

/// Зашифрованный файл — ЧЕСТНЫЙ ОТКАЗ, а не попытка прочитать мусор:
/// расшифровки здесь нет.
#[test]
fn pdf_reader_refuses_an_encrypted_file() {
    let pdf = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
            (
                5,
                b"<< /Filter /Standard /V 1 /R 2 /O <00> /U <00> /P -1 >>".to_vec(),
            ),
        ],
        " /Encrypt 5 0 R",
    );
    let error = PdfFile::parse(&pdf).expect_err("зашифрованный файл читаться не должен");
    let RtError::Pdf(text) = error else {
        panic!("ожидалась ошибка PDF");
    };
    assert!(text.contains("зашифрован"), "текст ошибки: {text}");
}

/// Неподдержанный фильтр отвергается ПО ИМЕНИ — чтобы из текста было
/// видно, чего не хватает.
#[test]
fn pdf_reader_names_the_unsupported_filter() {
    let inner = b"<< /Type /Catalog /Pages 2 0 R >>";
    let pdf = build_classic(
        &[
            (
                1,
                format!(
                    "<< /Type /ObjStm /N 1 /First 4 /Filter /LZWDecode /Length {} >>\nstream\n{}\nendstream",
                    inner.len() + 4,
                    format_args!("1 0 {}", String::from_utf8_lossy(inner))
                )
                .into_bytes(),
            ),
            (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
        ],
        "",
    );
    // Каталог здесь — сам объект 1, то есть словарь потока; разбор
    // упрётся в фильтр только если добраться до потока, поэтому файл
    // собран так, чтобы `/Root` вёл именно в него.
    let error = PdfFile::parse(&pdf);
    // Каталог-поток — вырожденный случай; важно, что разбор кончается
    // ошибкой, а не паникой.
    assert!(error.is_err());

    // А вот прямая проверка фильтра: поток страницы с /LZWDecode.
    let mut reader = Reader::new(&pdf).unwrap();
    let dict = vec![(
        "Filter".to_string(),
        PdfValue::Name("LZWDecode".to_string()),
    )];
    let error = reader
        .decode_stream(&dict, b"whatever")
        .expect_err("LZW не поддержан");
    let RtError::Pdf(text) = error else {
        panic!("ожидалась ошибка PDF");
    };
    assert!(text.contains("LZWDecode"), "текст ошибки: {text}");
}

/// Битые входы: разбор обязан кончиться ошибкой, а не паникой и не
/// зависанием.
#[test]
fn pdf_reader_rejects_broken_input_without_hanging() {
    let good =
        one_page("<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("пусто", Vec::new()),
        ("не PDF", b"just some bytes".to_vec()),
        (
            "заголовок без содержимого",
            b"%PDF-1.4\nnonsense\n".to_vec(),
        ),
        ("обрезан на половине", good[..good.len() / 2].to_vec()),
        ("обрезан на четверти", good[..good.len() / 4].to_vec()),
        ("startxref в никуда", {
            let mut bytes = good.clone();
            let at = find(&bytes, b"startxref\n").unwrap();
            bytes.truncate(at);
            bytes.extend_from_slice(b"startxref\n999999999\n%%EOF\n");
            bytes
        }),
        (
            "трейлер без /Root",
            build_classic(
                &[(1, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec())],
                "",
            )
            .split(|b| *b == b'\n')
            .map(|line| {
                if line.starts_with(b"<< /Size") {
                    b"<< /Size 2 >>".to_vec()
                } else {
                    line.to_vec()
                }
            })
            .collect::<Vec<_>>()
            .join(&b'\n'),
        ),
        (
            "цикл в дереве страниц",
            build_classic(
                &[
                    (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                    (2, b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>".to_vec()),
                    (3, b"<< /Type /Pages /Kids [ 2 0 R ] /Count 1 >>".to_vec()),
                ],
                "",
            ),
        ),
        (
            "циклическая косвенная ссылка",
            build_classic(
                &[
                    (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
                    (2, b"2 0 R".to_vec()),
                ],
                "",
            ),
        ),
        (
            "словарь без закрытия",
            build_classic(
                &[
                    (1, b"<< /Type /Catalog /Pages 2 0 R".to_vec()),
                    (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
                ],
                "",
            ),
        ),
    ];
    for (what, bytes) in cases {
        match PdfFile::parse(&bytes) {
            Err(RtError::Pdf(text)) => {
                assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
            }
            Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
            Ok(file) => panic!("«{what}» разобрался в {} страниц", file.page_count()),
        }
    }
}

/// Мусор на входе не должен уводить разбор в панику ни на одном
/// префиксе живого файла: грубая, но действенная проверка на то, что
/// каждый обрыв обработан.
#[test]
fn pdf_reader_survives_every_prefix_of_a_good_file() {
    let good =
        one_page("<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 595.32 841.92 ] /Contents 4 0 R >>");
    for cut in 0..good.len() {
        // Результат не важен: важно, что вызов вернулся.
        let _ = PdfFile::parse(&good[..cut]);
    }
    // И то же самое с одним испорченным байтом в каждой позиции.
    for at in 0..good.len() {
        let mut bytes = good.clone();
        bytes[at] = b'#';
        let _ = PdfFile::parse(&bytes);
    }
}

/// Числа, которые проба кладёт в словари вместо правильных. `None` —
/// вычисленное значение, то есть законный файл.
#[derive(Debug, Default, Clone, Copy)]
struct Boundary<'a> {
    /// `/N` объектного потока.
    n: Option<&'a str>,
    /// `/First` объектного потока.
    first: Option<&'a str>,
    /// `/Length` объектного потока.
    objstm_length: Option<&'a str>,
    /// `/Size` xref-потока.
    size: Option<&'a str>,
    /// Целиком ключ `/Index [ ... ]`: по умолчанию его нет вовсе и
    /// действует умолчание `[0 /Size]` (раздел 7.5.8.2).
    index: Option<&'a str>,
    /// Целиком массив `/W`.
    widths: Option<&'a str>,
    /// Целиком словарь `/DecodeParms` xref-потока.
    parms: Option<&'a str>,
    /// `/Length` xref-потока.
    xref_length: Option<&'a str>,
}

/// Собрать файл PDF 1.5 «объектный поток плюс xref-поток» байтами по
/// разделам 7.5.7 и 7.5.8 спецификации, подставив в числовые ключи то,
/// что просит проба. Без подстановок файл законный и читается — значит
/// в пробе ломает именно подставленное число, а не сборка.
fn build_boundary_pdf(boundary: Boundary<'_>) -> Vec<u8> {
    let pick = |over: Option<&str>, computed: String| -> String {
        over.map(str::to_string).unwrap_or(computed)
    };
    // Каталог и узел `/Pages` лежат внутри объектного потока 1,
    // xref-поток — объект 4.
    let inner: &[(u32, &str)] = &[
        (2, "<< /Type /Catalog /Pages 3 0 R >>"),
        (3, "<< /Type /Pages /Kids [ ] /Count 0 >>"),
    ];
    let mut head = String::new();
    let mut body = String::new();
    for (number, text) in inner {
        head.push_str(&format!("{number} {} ", body.len()));
        body.push_str(text);
        body.push(' ');
    }
    let first = head.len();
    let objstm = zlib_compress(format!("{head}{body}").as_bytes());

    let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let at_objstm = out.len();
    out.extend_from_slice(
        format!(
            "1 0 obj\n<< /Type /ObjStm /N {} /First {} /Filter /FlateDecode /Length {} >>\nstream\n",
            pick(boundary.n, inner.len().to_string()),
            pick(boundary.first, first.to_string()),
            pick(boundary.objstm_length, objstm.len().to_string()),
        )
        .as_bytes(),
    );
    out.extend_from_slice(&objstm);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let at_xref = out.len();

    // Записи шириной [1 4 2], строки предсказаны фильтром PNG «Up».
    let mut rows: Vec<[u8; 7]> = Vec::new();
    let mut push = |kind: u8, second: u32, third: u16| {
        let mut row = [0u8; 7];
        row[0] = kind;
        row[1..5].copy_from_slice(&second.to_be_bytes());
        row[5..7].copy_from_slice(&third.to_be_bytes());
        rows.push(row);
    };
    push(0, 0, 65535);
    push(1, at_objstm as u32, 0);
    push(2, 1, 0);
    push(2, 1, 1);
    push(1, at_xref as u32, 0);
    let mut predicted = Vec::new();
    let mut previous = [0u8; 7];
    for row in &rows {
        predicted.push(2u8);
        for i in 0..7 {
            predicted.push(row[i].wrapping_sub(previous[i]));
        }
        previous = *row;
    }
    let xref_stream = zlib_compress(&predicted);
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size {} {} /W {} /Root 2 0 R /Filter /FlateDecode \
             /DecodeParms {} /Length {} >>\nstream\n",
            pick(boundary.size, rows.len().to_string()),
            boundary.index.unwrap_or(""),
            pick(boundary.widths, "[ 1 4 2 ]".to_string()),
            pick(boundary.parms, "<< /Predictor 12 /Columns 7 >>".to_string()),
            pick(boundary.xref_length, xref_stream.len().to_string()),
        )
        .as_bytes(),
    );
    out.extend_from_slice(&xref_stream);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
    out
}

/// Граничные числа ПРЯМО ИЗ ФАЙЛА в каждом числовом ключе разбора:
/// разбор обязан вернуться с `RtError`, а не переполнить арифметику и
/// не выделить память по объявленному в файле размеру. Проверки
/// префиксов и однобайтовых порч этого не ловят — двадцатизначное
/// число из хорошего файла так не получить, поэтому словари здесь
/// собираются байтами.
///
/// Случаи, помеченные как обязанные упасть, — ровно те, где иначе
/// сложение уходит за `i64` или строка предиктора выделяется по
/// объявленным гигабайтам; там `Ok` означал бы, что защита снята.
#[test]
fn pdf_reader_rejects_boundary_numbers_in_dictionaries() {
    assert_eq!(
        PdfFile::parse(&build_boundary_pdf(Boundary::default()))
            .expect("базовый файл пробы обязан читаться")
            .page_count(),
        0
    );

    let cases: &[(&str, Boundary, bool)] = &[
        (
            "первый номер /Index у i64::MAX",
            Boundary {
                index: Some("/Index [ 9223372036854775807 2 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "первый номер /Index на единицу ниже i64::MAX",
            Boundary {
                index: Some("/Index [ 9223372036854775806 2 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "первый номер /Index за u32",
            Boundary {
                index: Some("/Index [ 4294967296 2 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "первый номер /Index на 10^11",
            Boundary {
                index: Some("/Index [ 100000000000 1 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "число записей /Index у i64::MAX",
            Boundary {
                index: Some("/Index [ 0 9223372036854775807 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "ширина поля /W у i64::MAX",
            Boundary {
                widths: Some("[ 9223372036854775807 4 2 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "лишнее поле /W на 10^11",
            Boundary {
                widths: Some("[ 1 4 2 100000000000 ]"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Size у i64::MAX",
            Boundary {
                size: Some("9223372036854775807"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Size на 10^11",
            Boundary {
                size: Some("100000000000"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/N объектного потока у i64::MAX",
            Boundary {
                n: Some("9223372036854775807"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/N объектного потока за u32",
            Boundary {
                n: Some("4294967296"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/First объектного потока у i64::MAX",
            Boundary {
                first: Some("9223372036854775807"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/First объектного потока на 10^11",
            Boundary {
                first: Some("100000000000"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Colors предиктора у i64::MAX",
            Boundary {
                parms: Some(
                    "<< /Predictor 12 /Colors 9223372036854775807 /BitsPerComponent 1 /Columns 1 >>",
                ),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Columns предиктора на 10^11",
            Boundary {
                parms: Some("<< /Predictor 12 /Columns 100000000000 >>"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Columns предиктора у i64::MAX",
            Boundary {
                parms: Some("<< /Predictor 12 /Columns 9223372036854775807 >>"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/BitsPerComponent предиктора у i64::MAX",
            Boundary {
                parms: Some("<< /Predictor 12 /BitsPerComponent 9223372036854775807 /Columns 7 >>"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/BitsPerComponent предиктора за u32",
            Boundary {
                parms: Some("<< /Predictor 12 /BitsPerComponent 4294967296 /Columns 7 >>"),
                ..Boundary::default()
            },
            true,
        ),
        (
            "/Colors предиктора за u32",
            Boundary {
                parms: Some("<< /Predictor 12 /Colors 4294967296 /Columns 7 >>"),
                ..Boundary::default()
            },
            true,
        ),
        // Номер предиктора больше десяти — это по-прежнему строки PNG
        // (раздел 7.4.4.4), поэтому такой файл имеет право прочитаться.
        (
            "/Predictor у i64::MAX",
            Boundary {
                parms: Some("<< /Predictor 9223372036854775807 /Columns 7 >>"),
                ..Boundary::default()
            },
            false,
        ),
        // Соврамший `/Length` спасает поиск `endstream`, и это
        // измеренное поведение платформы, а не недосмотр.
        (
            "/Length объектного потока у i64::MAX",
            Boundary {
                objstm_length: Some("9223372036854775807"),
                ..Boundary::default()
            },
            false,
        ),
        (
            "/Length xref-потока на 10^11",
            Boundary {
                xref_length: Some("100000000000"),
                ..Boundary::default()
            },
            false,
        ),
    ];

    for (what, boundary, must_fail) in cases {
        match PdfFile::parse(&build_boundary_pdf(*boundary)) {
            Err(RtError::Pdf(text)) => {
                assert!(!text.is_empty(), "у ошибки «{what}» пустой текст");
            }
            Err(other) => panic!("«{what}»: ожидалась ошибка PDF, получено {other:?}"),
            Ok(file) => assert!(
                !must_fail,
                "«{what}» разобрался в {} страниц",
                file.page_count()
            ),
        }
    }
}

/// Пустое дерево страниц — законный файл с нулём страниц (измерено).
#[test]
fn pdf_reader_accepts_an_empty_page_tree() {
    let pdf = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (2, b"<< /Type /Pages /Kids [ ] /Count 0 >>".to_vec()),
        ],
        "",
    );
    let file = PdfFile::parse(&pdf).expect("пустое дерево — законный файл");
    assert_eq!(file.page_count(), 0);
}

/// `/Length` косвенной ссылкой — так пишет сама платформа
/// (`platform-simple.pdf`), и так же обязан читаться собранный здесь
/// файл; соврамший `/Length` не должен рушить разбор.
#[test]
fn pdf_reader_takes_length_from_an_indirect_object_and_survives_a_wrong_one() {
    let pdf = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, b"<< /Length 5 0 R >>\nstream\nq Q\nendstream".to_vec()),
            (5, b"3".to_vec()),
        ],
        "",
    );
    assert_eq!(PdfFile::parse(&pdf).unwrap().page_count(), 1);

    // Подмена РАВНОЙ ДЛИНЫ: «/Length 5 0 R» и «/Length 99999» — по
    // тринадцать байтов, иначе поехали бы смещения в таблице xref и
    // проба мерила бы не то.
    let mut lying = pdf.clone();
    let at = find(&lying, b"/Length 5 0 R").expect("поток с косвенной длиной");
    lying[at..at + 13].copy_from_slice(b"/Length 99999");
    assert_eq!(PdfFile::parse(&lying).unwrap().page_count(), 1);
}

/// Синтаксис значений: имена с `#`, строки со скобками и escape'ами,
/// шестнадцатеричные строки, логические значения и `null`.
#[test]
fn pdf_reader_parses_the_value_syntax() {
    let source =
        "[ /A#20B (в (скобках) \\) и \\101) <48656C6C6F> <4a4> true false null 1 2 R -3.5 ]"
            .as_bytes();
    let value = Lexer::new(source, 0).parse_value(0).unwrap();
    let PdfValue::Array(items) = value else {
        panic!("ожидался массив");
    };
    assert_eq!(items[0], PdfValue::Name("A B".to_string()));
    assert_eq!(
        items[1],
        PdfValue::Str("в (скобках) ) и A".as_bytes().to_vec())
    );
    assert_eq!(items[2], PdfValue::Str(b"Hello".to_vec()));
    // Нечётный хвост шестнадцатеричной строки дополняется нулём.
    assert_eq!(items[3], PdfValue::Str(vec![0x4A, 0x40]));
    assert_eq!(items[4], PdfValue::Bool(true));
    assert_eq!(items[5], PdfValue::Bool(false));
    assert_eq!(items[6], PdfValue::Null);
    assert_eq!(items[7], PdfValue::Ref(1));
    assert_eq!(items[8], PdfValue::Real(-3.5));
    assert_eq!(items.len(), 9);
}

/// КРУГ ЗАМЫКАЕТСЯ на файле самой платформы: `probe-margins.pdf` она
/// записала из документа с полями 30, 10, 25 и 5 мм
/// (`capture-platform-pdf-layout.bsl`, строки 241..244), и читатель
/// обязан вернуть ровно их.
#[test]
fn pdf_reader_reads_back_the_platform_margins() {
    let bytes = std::fs::read("../../tests/conformance/pdf/probe-margins.pdf").unwrap();
    let file = PdfFile::parse(&bytes).unwrap();
    let page = file.page(0).unwrap();
    assert_eq!(page.margin(PdfMargin::Left), 30);
    assert_eq!(page.margin(PdfMargin::Right), 10);
    assert_eq!(page.margin(PdfMargin::Top), 25);
    assert_eq!(page.margin(PdfMargin::Bottom), 5);
    // А умолчание полей табличного документа — 10 мм со всех сторон.
    let empty = std::fs::read("../../tests/conformance/pdf/probe-empty.pdf").unwrap();
    let empty = PdfFile::parse(&empty).unwrap();
    let page = empty.page(0).unwrap();
    assert_eq!(
        [
            page.margin(PdfMargin::Left),
            page.margin(PdfMargin::Right),
            page.margin(PdfMargin::Top),
            page.margin(PdfMargin::Bottom),
        ],
        [10, 10, 10, 10]
    );
}

/// Правило полей целиком, все одиннадцать снятых страниц.
#[test]
fn pdf_reader_takes_margins_from_the_trim_box_only() {
    let a4 = "/MediaBox [ 0 0 595.32 841.92 ]";
    let shift = "/MediaBox [ 10 20 605.32 861.92 ]";
    let cases: &[(&str, [i64; 4])] = &[
        // Обе рамки объявлены и совпадают — как пишет платформа.
        (
            "/TrimBox [ 85.04 70.87 566.97 827.75 ] /BleedBox [ 85.04 70.87 566.97 827.75 ]",
            [30, 10, 25, 5],
        ),
        ("/TrimBox [ 85.04 70.87 566.97 827.75 ]", [30, 10, 25, 5]),
        // Один `/BleedBox` полей не даёт, как и один `/ArtBox`.
        ("/BleedBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
        ("/ArtBox [ 85.04 70.87 566.97 827.75 ]", [0, 0, 0, 0]),
        // Рамки разные — побеждает `/TrimBox`.
        (
            "/TrimBox [ 56.7 56.7 538.62 785.22 ] /BleedBox [ 14.17 14.17 581.15 827.75 ]",
            [20, 20, 20, 20],
        ),
        // `/CropBox` двигает ДАЛЬНИЕ края, но не ближние.
        (
            "/CropBox [ 50 60 545.32 781.92 ] /TrimBox [ 85.04 70.87 495.32 731.92 ]",
            [30, 18, 25, 18],
        ),
        // Поля не поджимаются к нулю.
        ("/TrimBox [ -50 -50 700 900 ]", [-18, -37, -18, -20]),
        ("/TrimBox [ 0 0 595.32 841.92 ]", [0, 0, 0, 0]),
    ];
    for (extra, expected) in cases {
        let pdf = one_page(&format!(
            "<< /Type /Page /Parent 2 0 R {a4} {extra} /Contents 4 0 R >>"
        ));
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        let got = [
            page.margin(PdfMargin::Left),
            page.margin(PdfMargin::Right),
            page.margin(PdfMargin::Top),
            page.margin(PdfMargin::Bottom),
        ];
        assert_eq!(got, *expected, "страница с {extra}");
    }

    // Смещённое начало `/MediaBox`: левое и верхнее поля — АБСОЛЮТНЫЕ
    // координаты угла `/TrimBox`, а не отступы от рамки.
    let shifted: &[(&str, [i64; 4])] = &[
        ("/TrimBox [ 95.04 90.87 576.97 847.75 ]", [34, 10, 32, 5]),
        (
            "/CropBox [ 60 80 555.32 801.92 ] /TrimBox [ 95.04 90.87 505.32 751.92 ]",
            [34, 18, 32, 18],
        ),
    ];
    for (extra, expected) in shifted {
        let pdf = one_page(&format!(
            "<< /Type /Page /Parent 2 0 R {shift} {extra} /Contents 4 0 R >>"
        ));
        let file = PdfFile::parse(&pdf).unwrap();
        let page = file.page(0).unwrap();
        let got = [
            page.margin(PdfMargin::Left),
            page.margin(PdfMargin::Right),
            page.margin(PdfMargin::Top),
            page.margin(PdfMargin::Bottom),
        ];
        assert_eq!(got, *expected, "смещённая страница с {extra}");
    }

    // `/TrimBox` НЕ наследуется от узла `/Pages` — как и `/Rotate`.
    let pdf = build_classic(
        &[
            (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
            (
                2,
                b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] \
                   /TrimBox [ 85.04 70.87 566.97 827.75 ] >>"
                    .to_vec(),
            ),
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
            ),
            (4, empty_content()),
        ],
        "",
    );
    let file = PdfFile::parse(&pdf).unwrap();
    assert_eq!(file.page(0).unwrap().margin(PdfMargin::Left), 0);
}

/// Все пять фильтров строк PNG, снимаемые предиктором, — и отказ с
/// НОМЕРОМ на предикторе TIFF и на неизвестном типе строки.
#[test]
fn pdf_reader_undoes_every_png_row_filter() {
    // Три строки по четыре байта, шаг предсказания — один байт.
    let rows: [[u8; 4]; 3] = [[10, 20, 30, 40], [11, 22, 33, 44], [200, 100, 50, 25]];
    for kind in 0u8..=4 {
        let mut encoded = Vec::new();
        let mut previous = [0u8; 4];
        for row in &rows {
            encoded.push(kind);
            for i in 0..4 {
                let left = if i >= 1 { row[i - 1] } else { 0 };
                let up = previous[i];
                let up_left = if i >= 1 { previous[i - 1] } else { 0 };
                let predictor = match kind {
                    0 => 0,
                    1 => left,
                    2 => up,
                    3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                    _ => paeth(left, up, up_left),
                };
                encoded.push(row[i].wrapping_sub(predictor));
            }
            previous = *row;
        }
        let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
        let mut reader = Reader::new(&empty).unwrap();
        let parms = vec![
            ("Predictor".to_string(), PdfValue::Integer(12)),
            ("Columns".to_string(), PdfValue::Integer(4)),
        ];
        let decoded = reader.apply_predictor(&parms, encoded).unwrap();
        let expected: Vec<u8> = rows.iter().flatten().copied().collect();
        assert_eq!(decoded, expected, "фильтр строки {kind}");
    }

    let empty = one_page("<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>");
    let mut reader = Reader::new(&empty).unwrap();
    let tiff = vec![("Predictor".to_string(), PdfValue::Integer(2))];
    let error = reader
        .apply_predictor(&tiff, vec![0; 4])
        .expect_err("предиктор TIFF не поддержан");
    let RtError::Pdf(text) = error else {
        panic!("ожидалась ошибка PDF");
    };
    assert!(
        text.contains('2'),
        "номер предиктора обязан быть в тексте: {text}"
    );

    let png = vec![
        ("Predictor".to_string(), PdfValue::Integer(12)),
        ("Columns".to_string(), PdfValue::Integer(4)),
    ];
    let error = reader
        .apply_predictor(&png, vec![9, 0, 0, 0, 0])
        .expect_err("тип строки 9 у PNG не определён");
    let RtError::Pdf(text) = error else {
        panic!("ожидалась ошибка PDF");
    };
    assert!(
        text.contains('9'),
        "номер типа строки обязан быть в тексте: {text}"
    );
}

// -----------------------------------------------------------------
// Вложения
// -----------------------------------------------------------------

/// Однолистовой файл с деревом имён вложений: `spec` — тело
/// `/EmbeddedFiles`, `objects` — всё остальное, что ему нужно.
fn with_attachments(tree: &str, objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut all = vec![
        (
            1,
            format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                .into_bytes(),
        ),
        (
            2,
            b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>".to_vec(),
        ),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        ),
        (4, empty_content()),
    ];
    all.extend_from_slice(objects);
    all.sort_by_key(|(number, _)| *number);
    build_classic(&all, "")
}

/// Пара объектов «поток встроенного файла + файловая спецификация»
/// под номерами `number` и `number + 1`.
fn filespec_pair(number: u32, head: &str, data: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let mut stream = format!(
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    stream.extend_from_slice(data);
    stream.extend_from_slice(b"\nendstream");
    vec![
        (number, stream),
        (
            number + 1,
            format!("<< /Type /Filespec {head} /EF << /F {number} 0 R >> >>").into_bytes(),
        ),
    ]
}

/// Файл с xref-ПОТОКОМ вместо классической таблицы: объекты пишутся
/// подряд, а таблица — поток `/XRef` шириной `[1 4 2]` без предиктора.
fn build_xref_stream(objects: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let size = objects.iter().map(|(n, _)| *n).max().unwrap_or(0) + 2;
    let mut offsets = vec![0usize; size as usize];
    for (number, body) in objects {
        offsets[*number as usize] = out.len();
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let at_xref = out.len();
    let table_number = size - 1;
    offsets[table_number as usize] = at_xref;
    let mut rows = Vec::new();
    for (index, offset) in offsets.iter().enumerate() {
        if index == 0 {
            rows.push(0u8);
            rows.extend_from_slice(&0u32.to_be_bytes());
            rows.extend_from_slice(&65535u16.to_be_bytes());
            continue;
        }
        rows.push(1u8);
        rows.extend_from_slice(&(*offset as u32).to_be_bytes());
        rows.extend_from_slice(&0u16.to_be_bytes());
    }
    let packed = zlib_compress(&rows);
    out.extend_from_slice(
        format!(
            "{table_number} 0 obj\n<< /Type /XRef /Size {size} /W [ 1 4 2 ] /Root 1 0 R \
             /Filter /FlateDecode /Length {} >>\nstream\n",
            packed.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&packed);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out.extend_from_slice(format!("startxref\n{at_xref}\n%%EOF\n").as_bytes());
    out
}

fn names_tree(entries: &[(&str, u32)]) -> String {
    let mut out = String::from("<< /Names [");
    for (name, number) in entries {
        out.push_str(&format!(" ({name}) {number} 0 R"));
    }
    out.push_str(" ] >>");
    out
}

/// Круг «добавить — записать — прочитать своим же читателем» на ОБОИХ
/// видах таблицы перекрёстных ссылок: инкрементальное обновление
/// дописывается и поверх классической `xref`, и поверх xref-потока, а
/// страницы исходного файла при этом остаются на месте.
#[test]
fn pdf_attachments_round_trip_over_both_xref_kinds() {
    let mut objects = filespec_pair(10, "/F (первое.txt)", "было".as_bytes());
    let tree = names_tree(&[("первое.txt", 11)]);
    let classic = with_attachments(&tree, &objects);

    objects.sort_by_key(|(number, _)| *number);
    let mut stream_objects = vec![
        (
            1,
            format!("<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles {tree} >> >>")
                .into_bytes(),
        ),
        (
            2,
            b"<< /Type /Pages /Kids [ 3 0 R ] /Count 1 /MediaBox [ 0 0 595.32 841.92 ] >>".to_vec(),
        ),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        ),
        (4, empty_content()),
    ];
    stream_objects.extend(objects);
    stream_objects.sort_by_key(|(number, _)| *number);
    let with_stream = build_xref_stream(&stream_objects);

    for (kind, base) in [("классическая xref", classic), ("xref-поток", with_stream)]
    {
        let file = PdfFile::parse(&base).unwrap_or_else(|e| panic!("{kind}: {e:?}"));
        assert_eq!(file.page_count(), 1, "{kind}: страница");
        assert_eq!(file.attachments().len(), 1, "{kind}: вложение из основы");

        let mut attachments = file.attachments().to_vec();
        attachments.push(PdfAttachment::new(
            "второе.bin".to_string(),
            "application/octet-stream".to_string(),
            PdfRelation::Data,
            vec![0, 1, 2, 250, 251, 252],
        ));
        let updated = file
            .write_with_attachments(&attachments)
            .unwrap_or_else(|e| panic!("{kind}: запись {e:?}"));
        // Исходные байты остаются началом файла: обновление ТОЛЬКО
        // дописывает.
        assert!(updated.starts_with(&base), "{kind}: основа переписана");

        let back = PdfFile::parse(&updated).unwrap_or_else(|e| panic!("{kind}: перечтение {e:?}"));
        assert_eq!(back.page_count(), 1, "{kind}: страница после записи");
        let names: Vec<&str> = back.attachments().iter().map(|a| a.name()).collect();
        // Порядок — по имени: так пишется дерево имён.
        assert_eq!(names, ["второе.bin", "первое.txt"], "{kind}: имена");
        assert_eq!(
            back.attachments()[0].data(),
            &[0, 1, 2, 250, 251, 252],
            "{kind}"
        );
        assert_eq!(
            back.attachments()[0].relation(),
            PdfRelation::Data,
            "{kind}"
        );
        assert_eq!(
            back.attachments()[0].content_type(),
            "application/octet-stream",
            "{kind}"
        );
        assert_eq!(back.attachments()[1].data(), "было".as_bytes(), "{kind}");

        // И ещё круг: обновление поверх обновления.
        let mut again = back.attachments().to_vec();
        again.retain(|item| item.name() != "первое.txt");
        let twice = back
            .write_with_attachments(&again)
            .expect("второе обновление");
        let last = PdfFile::parse(&twice).expect("перечтение второго обновления");
        assert_eq!(last.attachments().len(), 1, "{kind}: после удаления");
        assert_eq!(last.attachments()[0].name(), "второе.bin", "{kind}");
        assert_eq!(last.page_count(), 1, "{kind}: страница цела");
    }
}

/// Дерево имён с промежуточными узлами `/Kids` обходится целиком, а
/// `/Limits` при этом не читается вовсе — нам нужны все записи.
#[test]
fn pdf_attachments_walk_a_name_tree_with_kids() {
    let mut objects = Vec::new();
    objects.extend(filespec_pair(10, "/F (a.txt)", b"A"));
    objects.extend(filespec_pair(12, "/F (b.txt)", b"B"));
    objects.extend(filespec_pair(14, "/F (c.txt)", b"C"));
    objects.push((
        30,
        b"<< /Limits [ (a.txt) (b.txt) ] /Names [ (a.txt) 11 0 R (b.txt) 13 0 R ] >>".to_vec(),
    ));
    objects.push((
        31,
        b"<< /Limits [ (c.txt) (c.txt) ] /Names [ (c.txt) 15 0 R ] >>".to_vec(),
    ));
    objects.push((32, b"<< /Kids [ 30 0 R 31 0 R ] >>".to_vec()));
    let bytes = with_attachments("32 0 R", &objects);

    let file = PdfFile::parse(&bytes).expect("дерево с /Kids обязано читаться");
    let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
    assert_eq!(names, ["a.txt", "b.txt", "c.txt"]);
    assert_eq!(file.attachments()[2].data(), b"C");
}

/// Битое дерево имён — `RtError` с внятным текстом, а не паника и не
/// молчаливый пропуск. Вход враждебный: цикл, лишняя глубина,
/// нечётный `/Names`, не тот тип узла.
#[test]
fn pdf_attachments_reject_a_broken_name_tree() {
    let cycle = with_attachments("32 0 R", &[(32, b"<< /Kids [ 32 0 R ] >>".to_vec())]);
    let not_a_dict = with_attachments("32 0 R", &[(32, b"[ 1 2 3 ]".to_vec())]);
    let odd = with_attachments(
        "32 0 R",
        &[(32, "<< /Names [ (одно) ] >>".as_bytes().to_vec())],
    );
    let kids_not_array = with_attachments("32 0 R", &[(32, b"<< /Kids 7 >>".to_vec())]);
    let names_not_array = with_attachments("32 0 R", &[(32, b"<< /Names 7 >>".to_vec())]);

    // Глубина: цепочка `/Kids` длиннее предела.
    let mut deep: Vec<(u32, Vec<u8>)> = Vec::new();
    let depth = MAX_DEPTH as u32 + 5;
    for i in 0..depth {
        deep.push((
            100 + i,
            format!("<< /Kids [ {} 0 R ] >>", 100 + i + 1).into_bytes(),
        ));
    }
    deep.push((100 + depth, b"<< /Names [ ] >>".to_vec()));
    let too_deep = with_attachments("100 0 R", &deep);

    for (what, bytes) in [
        ("цикл", cycle),
        ("узел не словарь", not_a_dict),
        ("нечётный /Names", odd),
        ("/Kids не массив", kids_not_array),
        ("/Names не массив", names_not_array),
        ("слишком глубоко", too_deep),
    ] {
        let err =
            PdfFile::parse(&bytes).expect_err(&format!("{what}: разбор обязан кончиться ошибкой"));
        let RtError::Pdf(text) = err else {
            panic!("{what}: ожидался RtError::Pdf, получено {err:?}");
        };
        assert!(!text.is_empty(), "{what}: пустой текст ошибки");
    }
}

/// Записи без данных платформа молча пропускает — и мы тоже: без
/// `/EF`, с висящей ссылкой в нём и без имени вовсе.
#[test]
fn pdf_attachments_skip_entries_without_data_or_name() {
    let mut objects = Vec::new();
    objects.push((11, b"<< /Type /Filespec /F (noef.txt) >>".to_vec()));
    objects.push((
        13,
        b"<< /Type /Filespec /F (dangling.txt) /EF << /F 99 0 R >> >>".to_vec(),
    ));
    objects.extend(filespec_pair(
        14,
        "/Type /Filespec",
        "безымянное".as_bytes(),
    ));
    objects.extend(filespec_pair(16, "/F (живое.txt)", "живое".as_bytes()));
    let tree = names_tree(&[("noef", 11), ("dangling", 13), ("noname", 15), ("ok", 17)]);
    let bytes = with_attachments(&tree, &objects);

    let file = PdfFile::parse(&bytes).expect("файл с битыми записями обязан читаться");
    let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
    assert_eq!(names, ["живое.txt"]);
}

/// Имя берётся из `/UF`, а без него из `/F`; байты со знаком порядка
/// FE FF читаются как UTF-16BE, остальные — как UTF-8. Одноимённые
/// записи схлопываются, и побеждает ПОСЛЕДНЯЯ.
#[test]
fn pdf_attachments_decode_names_and_collapse_duplicates() {
    let utf16: String = "юникод.txt"
        .encode_utf16()
        .map(|unit| {
            let [hi, lo] = unit.to_be_bytes();
            format!("\\{hi:03o}\\{lo:03o}")
        })
        .collect();
    let mut objects = Vec::new();
    objects.extend(filespec_pair(10, "/F (только-f.txt)", b"1"));
    objects.extend(filespec_pair(12, "/F (не-это.txt) /UF (это.txt)", b"2"));
    objects.extend(filespec_pair(14, &format!("/UF (\\376\\377{utf16})"), b"3"));
    objects.extend(filespec_pair(16, "/F (дубль.txt)", "первое".as_bytes()));
    objects.extend(filespec_pair(18, "/F (дубль.txt)", "второе".as_bytes()));
    let tree = names_tree(&[("a", 11), ("b", 13), ("c", 15), ("d", 17), ("e", 19)]);
    let bytes = with_attachments(&tree, &objects);

    let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
    let names: Vec<&str> = file.attachments().iter().map(|a| a.name()).collect();
    assert_eq!(
        names,
        ["только-f.txt", "это.txt", "юникод.txt", "дубль.txt"]
    );
    assert_eq!(file.attachments()[3].data(), "второе".as_bytes());
}

/// Поток встроенного файла с фильтром, которого мы не умеем, не
/// уносит документ: вложение остаётся, содержимое — сырые байты.
#[test]
fn pdf_attachments_survive_an_unsupported_filter() {
    let objects = vec![
        (
            10,
            b"<< /Type /EmbeddedFile /Filter /LZWDecode /Length 3 >>\nstream\n\x80\x0b\x60\nendstream"
                .to_vec(),
        ),
        (
            11,
            b"<< /Type /Filespec /F (lzw.txt) /EF << /F 10 0 R >> >>".to_vec(),
        ),
    ];
    let bytes = with_attachments(&names_tree(&[("lzw", 11)]), &objects);

    let file = PdfFile::parse(&bytes).expect("неизвестный фильтр не должен ронять документ");
    assert_eq!(file.page_count(), 1);
    assert_eq!(file.attachments().len(), 1);
    assert_eq!(file.attachments()[0].data(), b"\x80\x0b\x60");
}

/// Связь читается из `/AFRelationship`, а неизвестное имя — это
/// `НеУстановлено` (измерено на `/AFRelationship /Nonsense`).
#[test]
fn pdf_attachments_read_the_relationship() {
    let mut objects = Vec::new();
    objects.extend(filespec_pair(
        10,
        "/F (s.txt) /AFRelationship /Source",
        b"1",
    ));
    objects.extend(filespec_pair(12, "/F (d.txt) /AFRelationship /Data", b"2"));
    objects.extend(filespec_pair(
        14,
        "/F (a.txt) /AFRelationship /Alternative",
        b"3",
    ));
    objects.extend(filespec_pair(
        16,
        "/F (u.txt) /AFRelationship /Supplement",
        b"4",
    ));
    objects.extend(filespec_pair(
        18,
        "/F (n.txt) /AFRelationship /Nonsense",
        b"5",
    ));
    objects.extend(filespec_pair(20, "/F (empty.txt)", b"6"));
    let tree = names_tree(&[
        ("a", 11),
        ("b", 13),
        ("c", 15),
        ("d", 17),
        ("e", 19),
        ("f", 21),
    ]);
    let bytes = with_attachments(&tree, &objects);

    let file = PdfFile::parse(&bytes).expect("файл обязан читаться");
    let relations: Vec<PdfRelation> = file.attachments().iter().map(|a| a.relation()).collect();
    assert_eq!(
        relations,
        [
            PdfRelation::Source,
            PdfRelation::Data,
            PdfRelation::Alternative,
            PdfRelation::Supplement,
            PdfRelation::Unspecified,
            PdfRelation::Unspecified,
        ]
    );
}

/// Имена в записанном дереве экранируются РОВНО тремя знаками и
/// уходят сырыми байтами: `/F` в UTF-8, `/UF` в UTF-16BE. Иначе
/// платформа, которая не снимает восьмеричные экраны, прочитала бы
/// вместо имени его запись.
#[test]
fn pdf_attachments_are_written_with_raw_name_bytes() {
    let base = with_attachments("32 0 R", &[(32, b"<< /Names [ ] >>".to_vec())]);
    let file = PdfFile::parse(&base).expect("основа обязана читаться");
    let written = file
        .write_with_attachments(&[PdfAttachment::new(
            "имя (со скобкой).txt".to_string(),
            "text/plain".to_string(),
            PdfRelation::Unspecified,
            b"data".to_vec(),
        )])
        .expect("запись");

    assert!(
        find(&written, "имя \\(со скобкой\\).txt".as_bytes()).is_some(),
        "имя обязано быть записано сырыми байтами UTF-8 со скобочным экранированием"
    );
    assert!(
        find(&written, &[0xFE, 0xFF, 0x04, 0x38, 0x04, 0x3C, 0x04, 0x4F]).is_some(),
        "/UF обязан быть UTF-16BE с меткой порядка байтов"
    );
    assert!(
        find(&written, b"\\376\\377").is_none(),
        "восьмеричных экранов в имени быть не должно"
    );
    let back = PdfFile::parse(&written).expect("перечтение");
    assert_eq!(back.attachments()[0].name(), "имя (со скобкой).txt");
}

/// Шестнадцатеричная строка в фикстуре `pdf-attachments.bsl` — это
/// РОВНО байты снятого с платформы `attach-platform.pdf`.
///
/// Копия нужна оснастке (конформанс-раннер запускает фикстуру из
/// каталога крейта, платформенный — из корня репозитория, и одному
/// относительному пути с обоими не сойтись), но копия, которая молча
/// разъехалась с оригиналом, — это фикстура, проверяющая не то.
#[test]
fn pdf_attachments_fixture_hex_matches_the_captured_file() {
    let fixture = std::fs::read_to_string("../../tests/conformance/fixtures/pdf-attachments.bsl")
        .expect("фикстура вложений обязана лежать в дереве");
    let mut hex = String::new();
    for line in fixture.lines() {
        if !line.starts_with("ШестнПлатформа") {
            continue;
        }
        let mut parts = line.split('"');
        parts.next();
        for (index, part) in parts.enumerate() {
            if index % 2 == 0 {
                hex.push_str(part);
            }
        }
    }
    assert!(!hex.is_empty(), "в фикстуре не нашлось строки с байтами");
    let bytes: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16)
                .expect("в строке фикстуры обязаны быть только шестнадцатеричные цифры")
        })
        .collect();
    let captured = std::fs::read("../../tests/conformance/pdf/attach-platform.pdf")
        .expect("снимок платформы обязан лежать в дереве");
    assert_eq!(
        bytes, captured,
        "байты в фикстуре разошлись со снимком attach-platform.pdf"
    );
}
