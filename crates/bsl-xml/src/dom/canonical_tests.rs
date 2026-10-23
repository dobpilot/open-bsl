use super::*;
use crate::core::XmlParser;

fn document(text: &str) -> Rc<DomNode> {
    let mut reader = crate::xml::XmlReaderState::over(XmlParser::new(text));
    crate::dom::build(&mut reader).unwrap()
}

#[test]
fn complete_document_matches_all_six_native_algorithms() {
    let doc = document(
        "<?xml version='1.0'?><!--before--><r xmlns='urn:root' xmlns:z='urn:z' xmlns:u='urn:unused' b='2' a='1'><z:e z:b='2' a='1'/>текст<![CDATA[<&>]]><!--inside--></r><!--after-->",
    );
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure.platform.txt"
    );
    let cases = [
        (false, false),
        (false, true),
        (true, false),
        (true, true),
        (false, false),
        (false, true),
    ];
    for ((exclusive, comments), line) in cases.into_iter().zip(oracle.lines().skip(2).take(6)) {
        let expected = line.split_once('\t').unwrap().1.replace("\\n", "\n");
        assert_eq!(
            render_document(&doc, exclusive, comments, "").unwrap(),
            expected
        );
    }
    let expected = oracle
        .lines()
        .find(|line| line.starts_with("canonical.prefixes\t"))
        .unwrap()
        .split_once('\t')
        .unwrap()
        .1;
    assert_eq!(render_document(&doc, true, false, "u").unwrap(), expected);
}

#[test]
fn empty_document_matches_the_separate_native_control() {
    let doc = DomNode::new(DomKind::Document, "#document".into(), String::new(), None);
    assert_eq!(render_document(&doc, false, false, "").unwrap(), "");
}

#[test]
fn complete_document_matches_extended_native_controls() {
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure-extended.platform.txt"
    );
    let controls = [
        ("empty-element", "<r/>"),
        (
            "attribute-escapes",
            "<r a=\"&#9;&#10;&#13;&quot;&amp;&lt;\">&#13;&amp;&lt;&gt;</r>",
        ),
        (
            "namespace-reset",
            "<r xmlns=\"urn:r\" xmlns:p=\"urn:p\"><a xmlns=\"\"><p:b xmlns:p=\"urn:q\"/></a></r>",
        ),
        (
            "processing-instructions",
            "<?before data?><r><?inside data?><!--c--></r><?after data?>",
        ),
        ("relative-namespace", "<r xmlns=\"relative\"/>"),
    ];
    for (name, xml) in controls {
        let doc = document(xml);
        let prefix = format!("canonical.extended.{name}.");
        for line in oracle.lines().filter(|line| line.starts_with(&prefix)) {
            let (id, expected) = line.split_once('\t').unwrap();
            let exclusive = id.contains("Исключающий");
            let comments = id.ends_with("СКомментариями");
            let result = render_document(&doc, exclusive, comments, "");
            if expected == "<rejected>" {
                assert!(result.is_err(), "{id}");
            } else {
                assert_eq!(result.unwrap(), expected.replace("\\n", "\n"), "{id}");
            }
        }
    }
}

#[test]
fn canonical_xpath_uses_original_nodes_without_promoting_attributes() {
    let doc = document("<r xmlns:z='urn:z'><z:e a='1'/></r>");
    let selected = crate::xpath::canonical_selection(&doc, "//*[local-name()='e']")
        .unwrap()
        .unwrap();
    assert_eq!(selected.len(), 1);
    let root = crate::dom::document_element(&doc).unwrap();
    assert!(Rc::ptr_eq(&selected[0], &root.children.borrow()[0]));
    assert_eq!(selected[0].attrs.borrow().len(), 1);
    let selected_with_attribute =
        crate::xpath::canonical_selection(&doc, "//*[local-name()='e'] | //*[local-name()='e']/@*")
            .unwrap()
            .unwrap();
    assert_eq!(selected_with_attribute.len(), 2);
    assert!(
        selected_with_attribute
            .iter()
            .any(|node| Rc::ptr_eq(node, &selected[0].attrs.borrow()[0]))
    );
}

#[test]
fn scalar_or_empty_xpath_preserves_the_whole_document_selection() {
    let doc = document("<r/>");
    for expression in ["", "1", "true()"] {
        assert!(
            crate::xpath::canonical_selection(&doc, expression)
                .unwrap()
                .is_none()
        );
    }
    assert!(
        crate::xpath::canonical_selection(&doc, "//missing")
            .unwrap()
            .unwrap()
            .is_empty()
    );
    assert!(crate::xpath::canonical_selection(&doc, "//*[").is_err());
}

#[test]
fn selected_element_matches_native_output_without_attributes_or_namespaces() {
    let doc = document("<r xmlns='urn:root' xmlns:z='urn:z'><z:e z:b='2' a='1'/></r>");
    let selected = crate::xpath::canonical_selection(&doc, "//*[local-name()='e']")
        .unwrap()
        .unwrap();
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure.platform.txt"
    );
    let expected = oracle
        .lines()
        .find(|line| line.starts_with("canonical.xpath\t"))
        .unwrap()
        .split_once('\t')
        .unwrap()
        .1;
    assert_eq!(
        render_selection(&doc, false, false, "", Some(&selected)).unwrap(),
        expected
    );
    assert_eq!(
        render_selection(&doc, false, false, "", Some(&[])).unwrap(),
        ""
    );
}

#[test]
fn all_nodes_including_namespaces_match_native_canonicalization() {
    let doc = document("<r xmlns:p='urn:p'><p:e a='1'/></r>");
    let selected = crate::xpath::canonical_selection(&doc, "//node() | //@* | //namespace::*")
        .unwrap()
        .unwrap();
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure-extended.platform.txt"
    );
    for line in oracle
        .lines()
        .filter(|line| line.starts_with("canonical.extended.all-nodes."))
    {
        let (id, expected) = line.split_once('\t').unwrap();
        assert_eq!(
            render_selection(
                &doc,
                id.contains("Исключающий"),
                id.ends_with("СКомментариями"),
                "",
                Some(&selected)
            )
            .unwrap(),
            expected,
            "{id}"
        );
    }
}

#[test]
fn canonical_10_and_exclusive_subsets_match_native_xml_attribute_controls() {
    let doc = document(
        "<r xmlns:p='urn:p' xml:lang='ru' xml:id='outer' xml:base='http://example.test/a/'><p:e xml:base='../b/' a='1'/></r>",
    );
    let selected = crate::xpath::canonical_selection(
        &doc,
        "//*[local-name()='e'] | //*[local-name()='e']/@* | //*[local-name()='e']/namespace::*",
    )
    .unwrap()
    .unwrap();
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure-extended.platform.txt"
    );
    for line in oracle.lines().filter(|line| {
        line.starts_with("canonical.extended.subset-xml-attributes.") && !line.contains("XML1_1")
    }) {
        let (id, expected) = line.split_once('\t').unwrap();
        assert_eq!(
            render_selection(
                &doc,
                id.contains("Исключающий"),
                id.ends_with("СКомментариями"),
                "",
                Some(&selected)
            )
            .unwrap(),
            expected,
            "{id}"
        );
    }
}

#[test]
fn canonical_11_subsets_match_native_base_and_id_controls() {
    let doc = document(
        "<r xmlns:p='urn:p' xml:lang='ru' xml:id='outer' xml:base='http://example.test/a/'><p:e xml:base='../b/' a='1'/></r>",
    );
    let selected = crate::xpath::canonical_selection(
        &doc,
        "//*[local-name()='e'] | //*[local-name()='e']/@* | //*[local-name()='e']/namespace::*",
    )
    .unwrap()
    .unwrap();
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure-extended.platform.txt"
    );
    for line in oracle.lines().filter(|line| {
        line.starts_with("canonical.extended.subset-xml-attributes.") && line.contains("XML1_1")
    }) {
        let (id, expected) = line.split_once('\t').unwrap();
        assert_eq!(
            render_version(
                &doc,
                false,
                id.ends_with("СКомментариями"),
                "",
                Some(&selected),
                true
            )
            .unwrap(),
            expected,
            "{id}"
        );
    }
}

#[test]
fn xml_base_resolution_matches_all_native_combinations_and_algorithms() {
    let oracle = include_str!(
        "../../../../docs/research/fixtures/xml-canonicalization/measure-base-resolution.platform.txt"
    );
    let cases = [
        ("parent", "http://example.test/a/", "../b/"),
        ("child", "http://example.test/a/", "b/"),
        ("absolute-path", "http://example.test/a/", "/b/"),
        (
            "absolute-uri",
            "http://example.test/a/",
            "https://other.test/b/",
        ),
        ("query", "http://example.test/a/file?old", "?new"),
        ("fragment", "http://example.test/a/file", "#part"),
        ("empty", "http://example.test/a/", ""),
        ("relative-chain", "a/", "../b/"),
    ];
    for (name, base, reference) in cases {
        let doc = document(&format!(
            "<r xml:base='{base}'><e xml:base='{reference}'/></r>"
        ));
        let selected = crate::xpath::canonical_selection(
            &doc,
            "//*[local-name()='e'] | //*[local-name()='e']/@*",
        )
        .unwrap()
        .unwrap();
        let prefix = format!("canonical.extended.base-{name}.");
        for line in oracle.lines().filter(|line| line.starts_with(&prefix)) {
            let (id, expected) = line.split_once('\t').unwrap();
            assert_eq!(
                render_version(
                    &doc,
                    id.contains("Исключающий"),
                    id.ends_with("СКомментариями"),
                    "",
                    Some(&selected),
                    id.contains("XML1_1")
                )
                .unwrap(),
                expected,
                "{id}"
            );
        }
    }
}

#[test]
fn canonicalization_preserves_dom_identity_order_and_values() {
    let doc = document(
        "<r xmlns:z='urn:z' b='2' a='1' xml:base='http://example.test/a/'><z:e xml:base='../b/' c='3'/><!--inside--></r>",
    );
    let mut pending = vec![doc.clone()];
    let mut before = Vec::new();
    while let Some(node) = pending.pop() {
        let children = node.children.borrow().clone();
        let attributes = node.attrs.borrow().clone();
        pending.extend(children.iter().cloned());
        pending.extend(attributes.iter().cloned());
        before.push((
            node.clone(),
            children,
            attributes,
            node.value.borrow().clone(),
        ));
    }
    let selected = crate::xpath::canonical_selection(
        &doc,
        "//*[local-name()='e'] | //*[local-name()='e']/@* | //*[local-name()='e']/namespace::*",
    )
    .unwrap()
    .unwrap();
    for (exclusive, comments, version_11) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (true, true, false),
        (false, false, true),
        (false, true, true),
    ] {
        render_version(&doc, exclusive, comments, "z", None, version_11).unwrap();
        render_version(&doc, exclusive, comments, "z", Some(&selected), version_11).unwrap();
    }
    for (node, children, attributes, value) in before {
        let same_nodes = |left: &[Rc<DomNode>], right: &[Rc<DomNode>]| {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| Rc::ptr_eq(left, right))
        };
        assert!(same_nodes(&node.children.borrow(), &children));
        assert!(same_nodes(&node.attrs.borrow(), &attributes));
        assert_eq!(*node.value.borrow(), value);
    }
}
