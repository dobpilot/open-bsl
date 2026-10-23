use super::*;

#[test]
fn native_html_document_type_is_the_first_child_before_the_root() {
    let value = new_document(&[]).unwrap();
    let document = need_node(Some(&value), "ДокументHTML").unwrap();
    let children = document.children.borrow();
    assert_eq!(children.len(), 2);
    let definition = node_value(&children[0], &document);
    assert_eq!(
        get_property(&definition, "ИмяУзла").unwrap(),
        BslValue::Str("html".into())
    );
    assert_eq!(
        get_property(&definition, "ТипУзла").unwrap(),
        BslValue::Enum(EnumValue::DomDocumentType)
    );
    assert_eq!(
        get_property(&definition, "РодительскийУзел").unwrap(),
        value
    );
    let reference = definition.object_ref().unwrap();
    let object = reference.downcast_ref::<DomNodeObject>().unwrap();
    assert!(std::ptr::eq(
        object.type_descriptor(),
        &DOCUMENT_TYPE_DEFINITION_TYPE
    ));
    assert_eq!(children[1].kind, DomKind::Element);
    assert_eq!(children[1].name, "html");
}

#[test]
fn html_writer_lifecycle_keeps_the_measured_close_reset_and_repeat_behavior() {
    let mut shapes = bsl_rt::RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
    let mut context = CallContext::minimal(&mut shapes, |value, _| Ok(format!("{value:?}")));
    let writer = HtmlWriterObject::default();
    assert_eq!(
        writer_close(&writer, &[], &mut context).unwrap(),
        BslValue::Undefined
    );
    writer_set_string(&writer, &[], &mut context).unwrap();
    assert_eq!(
        writer_close(&writer, &[], &mut context).unwrap(),
        BslValue::Str("".into())
    );
    assert_eq!(
        writer_close(&writer, &[], &mut context).unwrap(),
        BslValue::Undefined
    );

    let writer_value = new_writer();
    let writer_reference = writer_value.object_ref().unwrap();
    let writer = writer_reference.downcast_ref::<HtmlWriterObject>().unwrap();
    let document_value = new_document(&[]).unwrap();
    let document = need_node(Some(&document_value), "ДокументHTML").unwrap();
    writer_set_string(writer, &[], &mut context).unwrap();
    write_to_string(&writer_value, &document).unwrap();
    let first = writer.output.borrow().clone();
    assert!(write_to_string(&writer_value, &document).is_err());
    assert_eq!(*writer.output.borrow(), first);
    writer_set_string(writer, &[], &mut context).unwrap();
    assert_eq!(
        writer_close(writer, &[], &mut context).unwrap(),
        BslValue::Str("".into())
    );
}

#[test]
fn native_timelist_html_output_matches_the_saved_platform_receipt() {
    let document = new_document(&[]).unwrap();
    let reference = document.object_ref().unwrap();
    let object = reference.downcast_ref::<DomNodeObject>().unwrap();
    let body = create_element(&document, &[BslValue::Str("body".into())]).unwrap();
    set_body(object, body.clone()).unwrap();
    let text = create_text_node(
        &document,
        &[BslValue::Str("TimelistRecognizeResult".into())],
    )
    .unwrap();
    append_child(&body, &[text]).unwrap();
    let writer = new_writer();
    let writer_reference = writer.object_ref().unwrap();
    let writer_object = writer_reference.downcast_ref::<HtmlWriterObject>().unwrap();
    *writer_object.output.borrow_mut() = Some(String::new());
    write_to_string(&writer, &object.node).unwrap();
    let actual = writer_object.output.borrow().clone().unwrap();
    let receipt =
        include_str!("../../../../docs/research/fixtures/html-document/write.platform.txt");
    assert_eq!(actual.chars().count(), 111);
    assert!(receipt.lines().any(|line| {
        line.split_once('\t')
            .is_some_and(|(_, value)| value == actual)
    }));
}

#[test]
fn native_title_escaping_and_empty_elements_match_the_platform() {
    let document = new_document(&[BslValue::Str("Заголовок <&>".into())]).unwrap();
    let reference = document.object_ref().unwrap();
    let object = reference.downcast_ref::<DomNodeObject>().unwrap();
    let body = create_element(&document, &[BslValue::Str("body".into())]).unwrap();
    set_body(object, body.clone()).unwrap();
    let text = create_text_node(&document, &[BslValue::Str("<&>\"".into())]).unwrap();
    append_child(&body, &[text]).unwrap();
    for tag in ["br", "div"] {
        let child = create_element(&document, &[BslValue::Str(tag.into())]).unwrap();
        append_child(&body, &[child]).unwrap();
    }
    let actual = super::super::serialize::html_string(&object.node).unwrap();
    let receipt =
        include_str!("../../../../docs/research/fixtures/html-document/structure.platform.txt");
    assert!(receipt.lines().any(|line| {
        line.split_once('\t')
            .is_some_and(|(_, value)| value == actual)
    }));
}

#[test]
fn unsupported_html_nodes_do_not_leave_partial_output_in_the_writer() {
    let document = new_document(&[]).unwrap();
    let reference = document.object_ref().unwrap();
    let object = reference.downcast_ref::<DomNodeObject>().unwrap();
    // Неизмеренные виды узлов пока отклоняются целиком. Их можно получить
    // через общую модель DOM, но нельзя молча потерять при записи HTML.
    let comment = create_comment(&document, &[BslValue::Str("контроль".into())]).unwrap();
    append_child(&document, &[comment]).unwrap();
    let writer = new_writer();
    let writer_reference = writer.object_ref().unwrap();
    let writer_object = writer_reference.downcast_ref::<HtmlWriterObject>().unwrap();
    *writer_object.output.borrow_mut() = Some(String::new());
    assert!(write_to_string(&writer, &object.node).is_err());
    assert_eq!(writer_object.output.borrow().as_deref(), Some(""));
}

#[test]
fn native_empty_document_and_body_replacement_keep_node_identity() {
    let value = new_document(&[]).unwrap();
    let reference = value.object_ref().unwrap();
    let document = reference.downcast_ref::<DomNodeObject>().unwrap();
    assert!(std::ptr::eq(document.type_descriptor(), &DOCUMENT_TYPE));
    assert_eq!(document.node.children.borrow().len(), 2);
    assert_eq!(
        document.node.children.borrow()[0].kind,
        DomKind::DocumentType
    );
    let root = document_element(&document.node).unwrap();
    assert_eq!(root.name, "html");
    assert!(root.children.borrow().is_empty());
    assert_eq!(get_body(document).unwrap(), BslValue::Undefined);
    let first = create_element(&value, &[BslValue::Str("body".into())]).unwrap();
    set_body(document, first.clone()).unwrap();
    assert_eq!(get_body(document).unwrap(), first);
    let second = create_element(&value, &[BslValue::Str("body".into())]).unwrap();
    set_body(document, second.clone()).unwrap();
    assert_eq!(get_body(document).unwrap(), second);
    let first = need_node(Some(&first), "Тело").unwrap();
    assert!(first.parent.borrow().upgrade().is_none());
    let text =
        create_text_node(&value, &[BslValue::Str("TimelistRecognizeResult".into())]).unwrap();
    append_child(&second, &[text]).unwrap();
    assert_eq!(
        need_node(Some(&second), "Тело")
            .unwrap()
            .children
            .borrow()
            .len(),
        1
    );
}

#[test]
fn title_initialization_uses_the_same_owned_dom_nodes() {
    let value = new_document(&[BslValue::Str("Контроль".into())]).unwrap();
    let (document, _) = as_node(&value, "ДокументHTML").unwrap();
    let root = document_element(&document).unwrap();
    let head = root.children.borrow()[0].clone();
    let title = head.children.borrow()[0].clone();
    let text = title.children.borrow()[0].clone();
    assert_eq!(head.name, "head");
    assert_eq!(title.name, "title");
    assert_eq!(text.value.borrow().as_deref(), Some("Контроль"));
    assert!(Rc::ptr_eq(
        &text.owner.borrow().upgrade().unwrap(),
        &document
    ));
}

#[test]
fn an_empty_title_keeps_the_native_empty_root() {
    let value = new_document(&[BslValue::Str("".into())]).unwrap();
    let (document, _) = as_node(&value, "ДокументHTML").unwrap();
    let root = document_element(&document).unwrap();
    assert!(root.children.borrow().is_empty());
}

#[test]
fn xml_documents_keep_their_existing_identity_and_property_surface() {
    let value = super::super::new_document();
    let reference = value.object_ref().unwrap();
    let document = reference.downcast_ref::<DomNodeObject>().unwrap();
    assert!(std::ptr::eq(document.type_descriptor(), &DOM_DOCUMENT_TYPE));
    assert!(
        !document
            .property_table()
            .iter()
            .any(|property| property.names.contains(&"Тело"))
    );
    assert!(document.node.children.borrow().is_empty());
}
