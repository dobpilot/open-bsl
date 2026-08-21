//! Тесты модели, MXL и поверхности — над общими фикстурами.

use super::*;

/// Пустой документ платформы — 605 байт; проверяем, что скелет совпал
/// хотя бы по длине и по обрамлению, а точное совпадение ловит фикстура
/// с эталонным файлом.
#[test]
fn empty_document_has_a_header_and_no_trailing_newline() {
    let bytes = to_mxl_bytes(&SpreadDocData::new());
    assert!(bytes.starts_with(MXL_HEADER));
    assert_eq!(&bytes[13..16], BOM);
    assert_eq!(bytes.last(), Some(&b'}'));
}

#[test]
fn a_quote_in_text_is_doubled() {
    assert_eq!(quoted("ка\"вычка"), "\"ка\"\"вычка\"");
}

#[test]
fn height_never_shrinks_after_clearing_a_cell() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(2, 4, "X");
    assert_eq!((doc.height(), doc.width()), (3, 5));
    doc.set_cell_text(2, 4, "");
    assert_eq!((doc.height(), doc.width()), (3, 5));
}

/// `Записать` разбирает второй аргумент по перечислению, и член, в
/// который мы писать не умеем, обязан дать ошибку, а не тихий MXL.
/// Фикстура этого проверить не может: платформа такие форматы УМЕЕТ, и
/// строка вышла бы разной не из-за ошибки, а из-за объёма.
#[test]
fn write_refuses_a_file_type_it_cannot_produce() {
    let doc = new_document();
    let path = std::env::temp_dir().join(format!("open-bsl-spread-{}.bin", std::process::id()));
    let path = BslValue::Str(BslString::from_str(&path.to_string_lossy()));
    for (kind, ok) in [
        (bsl_rt::EnumValue::SpreadFileMxl, true),
        (bsl_rt::EnumValue::SpreadFileTxt, true),
        (bsl_rt::EnumValue::SpreadFileXlsx, true),
        (bsl_rt::EnumValue::SpreadFilePdf, true),
        (bsl_rt::EnumValue::JsonBoolean, false),
    ] {
        let args = [path.clone(), BslValue::Enum(kind)];
        assert_eq!(write(&doc, &args).is_ok(), ok, "{kind:?}");
    }
    // Не член перечисления вовсе — тоже ошибка.
    assert!(write(&doc, &[path.clone(), BslValue::Boolean(true)]).is_err());
    if let BslValue::Str(s) = &path {
        std::fs::remove_file(s.to_string()).ok();
    }
}

/// Поля страницы и ориентация — свойства ДОКУМЕНТА: умолчания
/// измерены (10 мм и «Портрет»), строка приводится к числу, а
/// не-число отвергается.
#[test]
fn page_properties_round_trip_through_bsl() {
    let doc = new_document();
    assert_eq!(
        get_property(&doc, "ПолеСлева").unwrap().to_string(),
        "10",
        "умолчание поля"
    );
    assert!(matches!(
        get_property(&doc, "ОриентацияСтраницы").unwrap(),
        BslValue::Enum(bsl_rt::EnumValue::PageOrientationPortrait)
    ));

    set_property(&doc, "ПолеСлева", int_value(30)).unwrap();
    set_property(
        &doc,
        "BottomMargin",
        BslValue::Str(BslString::from_str("12,7")),
    )
    .unwrap();
    set_property(
        &doc,
        "ОриентацияСтраницы",
        BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape),
    )
    .unwrap();
    assert_eq!(get_property(&doc, "LeftMargin").unwrap().to_string(), "30");
    // `Display` у `BslValue` отладочный, с точкой; запятую пользователь
    // видит через `bsl_format` — это проверяет фикстура `pdf-write`.
    assert_eq!(get_property(&doc, "ПолеСнизу").unwrap().to_string(), "12.7");
    assert!(matches!(
        get_property(&doc, "PageOrientation").unwrap(),
        BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape)
    ));

    assert!(
        set_property(
            &doc,
            "ПолеСлева",
            BslValue::Str(BslString::from_str("не число"))
        )
        .is_err()
    );
    assert!(set_property(&doc, "ОриентацияСтраницы", int_value(1)).is_err());
}

/// Поля переживают круг через MXL: идентификаторы пар 6..9 измерены
/// (сверху, слева, снизу, справа) и лежат в сотых долях миллиметра.
#[test]
fn margins_survive_the_mxl_round_trip() {
    let mut doc = SpreadDocData::new();
    doc.set_cell_text(0, 0, "A");
    doc.margins.top = 33.0;
    doc.margins.left = 31.0;
    doc.margins.bottom = 34.0;
    doc.margins.right = 32.5;
    let bytes = to_mxl_bytes(&doc);
    let text = String::from_utf8_lossy(&bytes).to_string();
    assert!(text.contains("{\"N\",3300}"), "{text}");
    assert!(text.contains("{\"N\",3250}"), "{text}");
    let back = from_mxl_bytes(&bytes).unwrap();
    assert_eq!(back.margins, doc.margins);
    // Умолчательный документ пишется теми же 1000, что и раньше, —
    // иначе разъехался бы весь корпус MXL.
    let plain = to_mxl_bytes(&SpreadDocData::new());
    assert_eq!(
        String::from_utf8_lossy(&plain)
            .matches("{\"N\",1000}")
            .count(),
        6
    );
}

/// Тот же круг, но с ФАЙЛОМ ПЛАТФОРМЫ. `tests/conformance/pdf/
/// probe-margins.mxl` записан 8.3.27 при полях 31, 32, 33 и 34 мм
/// (съёмка `capture-platform-pdf-layout.bsl`, её вывод помнит длину:
/// «probe-margins.mxl: Да, 1 085»), и порядок идентификаторов пар —
/// 6 сверху, 7 слева, 8 снизу, 9 справа — прочитан именно из него.
/// Проба закоммичена рядом со скриптом, чтобы это утверждение
/// проверялось, а не пересказывалось.
#[test]
fn margins_come_back_from_the_mxl_written_by_the_platform() {
    let bytes = std::fs::read("../../tests/conformance/pdf/probe-margins.mxl")
        .expect("проба съёмки лежит рядом со скриптом");
    let doc = from_mxl_bytes(&bytes).expect("файл платформы читается");
    assert_eq!(
        doc.margins,
        crate::pdf_layout::PageMargins {
            left: 31.0,
            right: 32.0,
            top: 33.0,
            bottom: 34.0,
        }
    );
}

#[test]
fn output_shifts_rows_down() {
    let mut a = SpreadDocData::new();
    a.set_cell_text(0, 0, "A");
    let mut b = SpreadDocData::new();
    b.set_cell_text(0, 0, "B");
    a.append(&b);
    assert_eq!(a.cell_text(0, 0), "A");
    assert_eq!(a.cell_text(1, 0), "B");
    assert_eq!(a.height(), 2);
}
