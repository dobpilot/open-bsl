//! Одиночные серверные пробы; особое имя a* не является oracle чистой маски.

#[test]
fn file_masks_match_the_isolated_platform_cases() {
    let names = [
        "a", "Z", "1", "я", "漢", "😀", ".hidden", "a*", "a?", "a\\b", "[", "[x]", "-", "]",
    ];
    let masks = [
        "?",
        "??",
        "???",
        "????",
        "[a-z]",
        "[!a-z]",
        "[^a-z]",
        "[[:alpha:]]",
        "[а-я]",
        "[",
        "\\?",
        "a\\*",
        "a*",
        "*.*",
        "*",
        "[[]x]",
        "[a-]",
        "[z-a]",
        "[[]",
        "[]]",
    ];
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-masks-single.platform.txt"
    );
    let mut checked = 0;
    for line in oracle.lines() {
        let Some((id, value)) = line.split_once('\t') else {
            continue;
        };
        let Some(id) = id.strip_prefix('s') else {
            continue;
        };
        let (name_index, mask_index) = id.split_once('.').unwrap();
        let name_index: usize = name_index.parse().unwrap();
        let mask_index: usize = mask_index.parse::<usize>().unwrap() - 1;
        if name_index == 7 {
            continue;
        }
        let count = value.split('|').next().unwrap();
        assert!(count == "0" || count == "1");
        assert_eq!(
            bsl_rt::file_mask_matches(masks[mask_index], names[name_index]),
            count == "1",
            "{id}"
        );
        checked += 1;
    }
    assert_eq!(checked, 260);
}

#[test]
fn named_classes_and_escapes_match_the_platform_oracle() {
    let cases = [
        ("class.alnum.letter", "A", "[[:alnum:]]"),
        ("class.alnum.digit", "7", "[[:alnum:]]"),
        ("class.alnum.punct", ".", "[[:alnum:]]"),
        ("class.digit.yes", "7", "[[:digit:]]"),
        ("class.digit.no", "A", "[[:digit:]]"),
        ("class.xdigit.lower", "f", "[[:xdigit:]]"),
        ("class.xdigit.upper", "F", "[[:xdigit:]]"),
        ("class.xdigit.no", "g", "[[:xdigit:]]"),
        ("class.lower.yes", "a", "[[:lower:]]"),
        ("class.lower.no", "A", "[[:lower:]]"),
        ("class.upper.yes", "A", "[[:upper:]]"),
        ("class.upper.no", "a", "[[:upper:]]"),
        ("class.blank.space", " ", "[[:blank:]]"),
        ("class.blank.tab", "\t", "[[:blank:]]"),
        ("class.blank.newline", "\n", "[[:blank:]]"),
        ("class.space.space", " ", "[[:space:]]"),
        ("class.space.tab", "\t", "[[:space:]]"),
        ("class.space.newline", "\n", "[[:space:]]"),
        ("class.cntrl.yes", "\u{1}", "[[:cntrl:]]"),
        ("class.cntrl.no", "A", "[[:cntrl:]]"),
        ("class.print.space", " ", "[[:print:]]"),
        ("class.print.letter", "A", "[[:print:]]"),
        ("class.print.control", "\u{1}", "[[:print:]]"),
        ("class.graph.space", " ", "[[:graph:]]"),
        ("class.graph.letter", "A", "[[:graph:]]"),
        ("class.graph.punct", ".", "[[:graph:]]"),
        ("class.punct.yes", ".", "[[:punct:]]"),
        ("class.punct.no", "A", "[[:punct:]]"),
        ("class.unknown", "A", "[[:unknown:]]"),
        ("class.negated_digit.yes", "A", "[![:digit:]]"),
        ("class.negated_digit.no", "7", "[![:digit:]]"),
        ("escape.question", "?", "\\?"),
        ("escape.open", "[", "\\["),
        ("escape.literal", "A", "\\A"),
        ("escape.class.close", "]", "[\\]]"),
        ("escape.class.hyphen", "-", "[\\-]"),
        ("broken.named_close", "A", "[[:alpha]"),
        ("broken.named_outer", "A", "[[:alpha:]"),
        ("broken.escape_end", "A", "\\"),
        ("broken.empty", "]", "[]"),
    ];
    let oracle = include_str!(
        "../../../tests/conformance/measure/filesystem/file-mask-extensions.platform.txt"
    );

    for (id, core, core_mask) in cases {
        let line = oracle
            .lines()
            .find(|line| line.starts_with(id) && line.as_bytes().get(id.len()) == Some(&b'\t'))
            .unwrap_or_else(|| panic!("в oracle отсутствует {id}"));
        let expected = line.split_once('\t').unwrap().1;
        assert!(expected == "0" || expected == "1", "{id}: {expected}");
        assert_eq!(
            bsl_rt::file_mask_matches(&format!("x{core_mask}y"), &format!("x{core}y")),
            expected == "1",
            "{id}"
        );
    }
}
