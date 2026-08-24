//! Тесты дерева DOM, его поверхности и сериализации.

use super::*;

fn document_of(text: &str) -> Rc<DomNode> {
    let mut state = XmlReaderState::over(XmlParser::new(text));
    build(&mut state).expect("документ обязан построиться")
}

/// Корень ВМЕСТЕ с документом: документ возвращается, чтобы вызывающий
/// держал его живым — ровно ту же работу в бою делает сильная ссылка
/// внутри значения (см. [`node_value`]).
fn root_of(text: &str) -> (Rc<DomNode>, Rc<DomNode>) {
    let doc = document_of(text);
    let root = document_element(&doc).expect("у документа обязан быть корень");
    (root, doc)
}

/// Значение узла. Документ берётся из самого узла — в тестах он жив,
/// а там, где проверяется обратное, значение строится вручную.
fn value(node: &Rc<DomNode>) -> BslValue {
    let doc = node
        .owner
        .borrow()
        .upgrade()
        .expect("документ узла обязан быть жив");
    node_value(node, &doc)
}

fn prop(node: &Rc<DomNode>, name: &str) -> BslValue {
    get_property(&value(node), name).expect("свойство обязано читаться")
}

/// Документ вложенности `depth` с текстом в самой глубине.
fn nested(depth: usize) -> String {
    let mut text = String::new();
    for _ in 0..depth {
        text.push_str("<а>");
    }
    text.push('х');
    for _ in 0..depth {
        text.push_str("</а>");
    }
    text
}

fn text_of(v: &BslValue) -> String {
    match v {
        BslValue::Str(s) => s.to_string(),
        other => panic!("ожидалась строка, получено {other:?}"),
    }
}

#[test]
fn dom_builds_the_element_tree() {
    let (root, _doc) = root_of("<а><б>текст</б><в/></а>");
    assert_eq!(root.name, "а");
    assert_eq!(root.children.borrow().len(), 2);
    assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "текст");
    let first = root.children.borrow()[0].clone();
    assert_eq!(first.name, "б");
    assert_eq!(first.kind, DomKind::Element);
}

#[test]
fn dom_navigation_links_parents_and_siblings() {
    let (root, _doc) = root_of("<а><б/><в/></а>");
    let first = root.children.borrow()[0].clone();
    let second = root.children.borrow()[1].clone();
    assert_eq!(text_of(&prop(&first, "ИмяУзла")), "б");
    let next = prop(&first, "СледующийСоседний");
    assert_eq!(next, value(&second));
    let back = prop(&second, "ПредыдущийСоседний");
    assert_eq!(back, value(&first));
    assert_eq!(prop(&first, "ПредыдущийСоседний"), BslValue::Undefined);
    assert_eq!(prop(&second, "СледующийСоседний"), BslValue::Undefined);
    let parent = prop(&first, "РодительскийУзел");
    assert_eq!(parent, value(&root));
}

#[test]
fn dom_attributes_carry_namespaces() {
    let (root, _doc) = root_of(r#"<к:а xmlns:к="urn:к" п="1" к:б="2"/>"#);
    let attrs = root.attrs.borrow().clone();
    assert_eq!(attrs.len(), 3, "объявление xmlns тоже атрибут");
    assert_eq!(attrs[0].name, "xmlns:к");
    assert_eq!(attrs[0].uri, XMLNS_URI);
    assert_eq!(attrs[1].uri, "", "атрибут без префикса — без URI");
    assert_eq!(attrs[2].uri, "urn:к");
    assert_eq!(attrs[2].local_name(), "б");
    assert_eq!(text_of(&prop(&root, "URIПространстваИмен")), "urn:к");
    assert_eq!(text_of(&prop(&root, "Префикс")), "к");
    assert_eq!(text_of(&prop(&root, "ЛокальноеИмя")), "а");
}

#[test]
fn dom_attribute_lookup_ignores_prefixed_names() {
    let (root, doc) = root_of(r#"<к:а xmlns:к="urn:к" п="1" к:б="2"/>"#);
    let el = node_value(&root, &doc);
    let one = |args: &[BslValue]| get_attribute(&el, args).unwrap();
    assert_eq!(one(&[str_value("п")]), str_value("1"));
    // Полное имя одним аргументом не находится — измерено.
    assert_eq!(one(&[str_value("к:б")]), BslValue::Undefined);
    assert_eq!(one(&[str_value("б")]), BslValue::Undefined);
    assert_eq!(one(&[str_value("urn:к"), str_value("б")]), str_value("2"));
    assert_eq!(
        has_attribute(&el, &[str_value("нет")]).unwrap(),
        BslValue::Boolean(false)
    );
    assert_eq!(
        get_attribute_node(&el, &[str_value("нет")]).unwrap(),
        BslValue::Undefined
    );
}

#[test]
fn dom_text_merges_cdata_and_drops_blank_runs() {
    let (root, _doc) = root_of("<а>раз<![CDATA[два]]>три</а>");
    assert_eq!(root.children.borrow().len(), 1);
    assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "раздватри");
    let (blank, _blank_doc) = root_of("<а>   </а>");
    assert!(blank.children.borrow().is_empty());
    let (around, _around_doc) = root_of("<а>  <б/>  хвост  </а>");
    assert_eq!(around.children.borrow().len(), 2);
    assert_eq!(
        text_of(&prop(&around.children.borrow()[1], "ЗначениеУзла")),
        "  хвост  "
    );
}

#[test]
fn dom_keeps_comments_inside_the_root_only() {
    let doc = document_of("<!--до--><а>раз<!--в середине-->два</а><!--после-->");
    let kids = doc.children.borrow().clone();
    assert_eq!(kids.len(), 2, "комментарий до корня в дерево не идёт");
    assert_eq!(kids[0].kind, DomKind::Element);
    assert_eq!(kids[1].kind, DomKind::Comment);
    assert_eq!(kids[1].value.borrow().as_deref(), Some("после"));
    let root = kids[0].clone();
    assert_eq!(root.children.borrow().len(), 3, "комментарий рвёт текст");
    assert_eq!(text_of(&prop(&root, "ТекстовоеСодержимое")), "раздва");
}

#[test]
fn dom_processing_instructions_survive_around_the_root() {
    let doc = document_of("<?пи данные?><а/>");
    let kids = doc.children.borrow().clone();
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].kind, DomKind::ProcessingInstruction);
    assert_eq!(text_of(&prop(&kids[0], "ИмяУзла")), "пи");
    assert_eq!(text_of(&prop(&kids[0], "Цель")), "пи");
    assert_eq!(text_of(&prop(&kids[0], "Данные")), "данные");
    assert_eq!(prop(&kids[0], "ЗначениеУзла"), BslValue::Undefined);
    assert_eq!(prop(&kids[0], "ТекстовоеСодержимое"), BslValue::Undefined);
}

#[test]
fn dom_search_matches_full_and_local_names() {
    let tree = document_of(r#"<к:а xmlns:к="urn:к"><а><к:а/></а></к:а>"#);
    let doc = node_value(&tree, &tree);
    let count = |args: &[BslValue]| {
        let value = get_elements_by_name(&doc, args).unwrap();
        value
            .object_ref()
            .and_then(|object| object.downcast_ref::<DomListObject>())
            .expect("ожидался список")
            .kind
            .len()
    };
    assert_eq!(count(&[str_value("а")]), 3, "локальное имя у всех трёх");
    assert_eq!(count(&[str_value("к:а")]), 2);
    assert_eq!(count(&[str_value("*")]), 3);
    // URI не фильтрует — измерено.
    assert_eq!(count(&[str_value("urn:нет"), str_value("а")]), 3);
    assert_eq!(count(&[str_value("нет")]), 0);
}

#[test]
fn dom_build_continues_from_the_readers_position() {
    let mut state = XmlReaderState::over(XmlParser::new(r#"<а х="1">раз<б/>два</а>"#));
    // Два шага читателя: он стоит на тексте «раз».
    for _ in 0..2 {
        let parser = state.parser.as_mut().unwrap();
        let e = parser.read().unwrap();
        state.current = e;
    }
    let doc = build(&mut state).unwrap();
    let root = document_element(&doc).unwrap();
    assert_eq!(root.name, "а", "предок восстановлен из стека читателя");
    assert_eq!(root.attrs.borrow().len(), 1, "вместе с атрибутами");
    assert_eq!(
        text_of(&prop(&root, "ТекстовоеСодержимое")),
        "два",
        "уже прочитанный текст в дерево не попадает"
    );
}

#[test]
fn dom_exhausted_reader_gives_an_empty_document() {
    let mut state = XmlReaderState::over(XmlParser::new("<а/>"));
    while let Some(e) = state.parser.as_mut().unwrap().read().unwrap() {
        state.current = Some(e);
    }
    let doc = build(&mut state).unwrap();
    assert!(doc.children.borrow().is_empty());
    assert!(document_element(&doc).is_none());
}

#[test]
fn dom_broken_input_is_an_error() {
    let mut state = XmlReaderState::over(XmlParser::new("<а><б></а>"));
    assert!(build(&mut state).is_err());
    let mut empty = XmlReaderState::over(XmlParser::new(""));
    assert!(build(&mut empty).is_err());
    let mut two = XmlReaderState::over(XmlParser::new("<а/><б/>"));
    assert!(build(&mut two).is_err());
    // Читатель без источника — тоже ошибка, а не пустой документ.
    let mut none = XmlReaderState::default();
    assert!(build(&mut none).is_err());
}

#[test]
fn dom_attribute_value_is_a_text_child() {
    let (root, _doc) = root_of(r#"<а п="1"/>"#);
    let attr = root.attrs.borrow()[0].clone();
    assert_eq!(attr.children.borrow().len(), 1);
    assert_eq!(text_of(&prop(&attr, "ЗначениеУзла")), "1");
    assert_eq!(text_of(&prop(&attr, "ТекстовоеСодержимое")), "1");
    assert_eq!(prop(&attr, "РодительскийУзел"), BslValue::Undefined);
    assert_eq!(prop(&attr, "ЭлементВладелец"), value(&root));
    assert_eq!(prop(&attr, "СледующийСоседний"), BslValue::Undefined);
}

#[test]
fn dom_document_members_match_the_platform() {
    let doc = document_of("<а/>");
    assert_eq!(text_of(&prop(&doc, "ИмяУзла")), "#document");
    assert_eq!(prop(&doc, "ЗначениеУзла"), BslValue::Undefined);
    assert_eq!(prop(&doc, "ТекстовоеСодержимое"), BslValue::Undefined);
    assert_eq!(text_of(&prop(&doc, "ВерсияXML")), "1.0");
    assert_eq!(prop(&doc, "РодительскийУзел"), BslValue::Undefined);
    assert_eq!(prop(&doc, "Атрибуты"), BslValue::Undefined);
    let root = document_element(&doc).unwrap();
    assert_eq!(prop(&doc, "ЭлементДокумента"), value(&root));
    assert_eq!(prop(&root, "ДокументВладелец"), value(&doc));
}

#[test]
fn dom_nodes_compare_by_identity() {
    let (root, _doc) = root_of("<а><б/></а>");
    let child = value(&root.children.borrow()[0]);
    assert_eq!(prop(&root, "ПервыйДочерний"), child);
    assert_ne!(prop(&root, "ДочерниеУзлы"), prop(&root, "ДочерниеУзлы"));
    assert_ne!(value(&root), child);
}

/// Сильная ссылка на документ внутри значения тождества не трогает:
/// оно по-прежнему по адресу УЗЛА, значит и хэш обязан совпадать —
/// иначе такой узел терялся бы ключом `Соответствие`.
#[test]
fn dom_node_values_of_one_node_are_equal_and_hash_alike() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(v: &BslValue) -> u64 {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    }

    let (root, doc) = root_of("<а><б/></а>");
    let one = node_value(&root, &doc);
    let two = node_value(&root, &doc);
    assert_eq!(one, two);
    assert_eq!(hash_of(&one), hash_of(&two));
    // И то же самое для узла, добытого навигацией, а не напрямую.
    let via_property = prop(&root, "ПервыйДочерний");
    let via_list = {
        let value = prop(&root, "ДочерниеУзлы");
        let list = value
            .object_ref()
            .and_then(|object| object.downcast_ref::<DomListObject>())
            .expect("ожидался список");
        node_value(&list.kind.get(0).expect("список не пуст"), &list.doc)
    };
    assert_eq!(via_property, via_list);
    assert_eq!(hash_of(&via_property), hash_of(&via_list));
}

/// Узел обязан пережить переменную с документом: в BSL идиома
/// `Возврат Док.ЭлементДокумента` возвращает корень, а документ на
/// выходе из функции умирает. Ссылки вверх по дереву слабые, поэтому
/// живым дерево держит только сильная ссылка внутри значения.
#[test]
fn dom_node_value_outlives_the_document_variable() {
    let root_value = {
        let doc = document_of(r#"<а х="1"><б>т</б></а>"#);
        get_property(&node_value(&doc, &doc), "ЭлементДокумента")
            .expect("ЭлементДокумента обязан читаться")
    };
    // Переменной с документом больше нет — навигация обязана работать.
    assert_eq!(text_of(&get_property(&root_value, "ИмяУзла").unwrap()), "а");
    let parent = get_property(&root_value, "РодительскийУзел").unwrap();
    assert_eq!(
        text_of(&get_property(&parent, "ИмяУзла").unwrap()),
        DOCUMENT_NODE_NAME
    );
    let owner = get_property(&root_value, "ДокументВладелец").unwrap();
    assert_eq!(
        text_of(&get_property(&owner, "ИмяУзла").unwrap()),
        DOCUMENT_NODE_NAME
    );
    // Дети и атрибуты — тоже: у атрибута жив его `ЭлементВладелец`.
    let kids = get_property(&root_value, "ДочерниеУзлы").unwrap();
    assert_eq!(kids.collection_len().unwrap(), 1);
    let attr = get_attribute_node(&root_value, &[str_value("х")]).unwrap();
    assert_eq!(
        get_property(&attr, "ЭлементВладелец").unwrap(),
        root_value,
        "элемент-владелец — тот же узел, что и корень"
    );
}

/// Глубина ровно по пределу обязана и строиться, и обходиться, и
/// РАЗРУШАТЬСЯ: разрушение рекурсивно, так что в debug-сборке этот тест
/// и есть проверка того, что 500 уровней стек держит.
#[test]
fn dom_document_at_the_depth_limit_still_works() {
    let mut state = XmlReaderState::over(XmlParser::new(&nested(MAX_DOM_DEPTH)));
    let doc = build(&mut state).expect("глубина ровно по пределу обязана строиться");
    let root = document_element(&doc).expect("у документа обязан быть корень");
    let mut all = Vec::new();
    root.descendants(&mut all);
    assert_eq!(all.len(), MAX_DOM_DEPTH - 1, "корень себя не включает");
    assert_eq!(root.text_content(), "х");
    // Порядок важен: пока `all` и `root` живы, дерево разрушается по
    // кусочкам, а рекурсивный `Drop` во всю глубину случается только
    // на последней сильной ссылке — на документе.
    drop(all);
    drop(root);
    drop(doc);
}

/// Глубже предела — перехватываемая ошибка, а НЕ переполнение стека
/// процесса: `Попытка` обязана такой документ поймать.
#[test]
fn dom_document_deeper_than_the_limit_is_an_error_not_a_crash() {
    let mut state = XmlReaderState::over(XmlParser::new(&nested(MAX_DOM_DEPTH + 1)));
    let err = build(&mut state).expect_err("глубже предела — ошибка");
    assert!(
        matches!(err, RtError::StackOverflow { .. }),
        "ожидалась StackOverflow, получено {err:?}"
    );
    // Тот же предел действует и на предков, восстановленных из стека
    // читателя: дерево «с глубины» второго пути в обход не даёт.
    let deep = nested(MAX_DOM_DEPTH + 1);
    let mut from_inside = XmlReaderState::over(XmlParser::new(&deep));
    for _ in 0..=MAX_DOM_DEPTH {
        let parser = from_inside.parser.as_mut().unwrap();
        from_inside.current = parser.read().unwrap();
    }
    let err = build(&mut from_inside).expect_err("предки глубже предела — тоже ошибка");
    assert!(matches!(err, RtError::StackOverflow { .. }));
}

// --- Создание, мутация и запись -------------------------------------

/// Пустой документ вместе с его значением: фабрики вызываются через
/// значение, потому что получателем работает `BslValue`.
fn fresh_document() -> BslValue {
    new_document()
}

fn call(
    obj: &BslValue,
    f: fn(&BslValue, &[BslValue]) -> RtResult<BslValue>,
    args: &[BslValue],
) -> BslValue {
    f(obj, args).expect("вызов обязан пройти")
}

/// Вызов с ОДНИМ аргументом-узлом. Отдельный помощник — чтобы не плодить
/// `&[узел.clone()]`, на который clippy справедливо ворчит.
fn call1(
    obj: &BslValue,
    f: fn(&BslValue, &[BslValue]) -> RtResult<BslValue>,
    arg: &BslValue,
) -> BslValue {
    f(obj, std::slice::from_ref(arg)).expect("вызов обязан пройти")
}

fn kids_names(node: &BslValue) -> String {
    let list = get_property(node, "ДочерниеУзлы").expect("дети обязаны читаться");
    let kind = list
        .object_ref()
        .and_then(|object| object.downcast_ref::<DomListObject>())
        .expect("ожидался список")
        .kind
        .clone();
    kind.items()
        .iter()
        .map(|n| n.name.clone())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Записать узел через `ЗаписьDOM` и отдать накопленный текст.
fn serialize(node: &BslValue) -> RtResult<String> {
    let target = crate::xml::new_xml_writer(std::rc::Rc::new(bsl_rt::SystemFileSystem));
    crate::xml::set_string(crate::xml::arg_object(&target)?, &[])?;
    write(&new_writer(), &[node.clone(), target.clone()])?;
    match crate::xml::close_writer(crate::xml::arg_object(&target)?)? {
        BslValue::Str(s) => Ok(s.to_string()),
        other => panic!("ожидалась строка, получено {other:?}"),
    }
}

#[test]
fn dom_created_nodes_carry_the_platform_names() {
    let doc = fresh_document();
    let el = call(&doc, create_element, &[str_value("а")]);
    // ИЗМЕРЕНО: у одноаргументной формы локальное имя и префикс ПУСТЫ
    // даже для имени с двоеточием, а у атрибута локальное имя — всё имя.
    assert_eq!(text_of(&get_property(&el, "ЛокальноеИмя").unwrap()), "");
    let pref = call(&doc, create_element, &[str_value("п:а")]);
    assert_eq!(text_of(&get_property(&pref, "ИмяУзла").unwrap()), "п:а");
    assert_eq!(text_of(&get_property(&pref, "Префикс").unwrap()), "");
    let attr = call(&doc, create_attribute, &[str_value("п:а")]);
    assert_eq!(
        text_of(&get_property(&attr, "ЛокальноеИмя").unwrap()),
        "п:а"
    );
    // Двухаргументная форма имя расщепляет и вешает объявление атрибутом.
    let ns = call(
        &doc,
        create_element,
        &[str_value("urn:х"), str_value("п:а")],
    );
    assert_eq!(text_of(&get_property(&ns, "ЛокальноеИмя").unwrap()), "а");
    assert_eq!(text_of(&get_property(&ns, "Префикс").unwrap()), "п");
    let attrs = get_property(&ns, "Атрибуты").unwrap();
    assert_eq!(attrs.collection_len().unwrap(), 1, "объявление xmlns:п");
    let decl = {
        let list = attrs
            .object_ref()
            .and_then(|object| object.downcast_ref::<DomListObject>())
            .expect("ожидалась коллекция атрибутов");
        node_value(&list.kind.get(0).expect("объявление на месте"), &list.doc)
    };
    assert_eq!(text_of(&get_property(&decl, "ИмяУзла").unwrap()), "xmlns:п");
    assert_eq!(text_of(&get_property(&decl, "Значение").unwrap()), "urn:х");
    assert_eq!(
        text_of(&get_property(&decl, "URIПространстваИмен").unwrap()),
        XMLNS_URI
    );
    // Секция CDATA — свой вид узла с собственным служебным именем.
    let cdata = call(&doc, create_cdata_section, &[str_value("ц")]);
    assert_eq!(
        text_of(&get_property(&cdata, "ИмяУзла").unwrap()),
        CDATA_NODE_NAME
    );
    assert_eq!(cdata.type_name(), "СекцияCDATADOM");
}

#[test]
fn dom_bad_names_and_arities_are_rejected() {
    let doc = fresh_document();
    // ИЗМЕРЕНО: пустое имя, имя с пробелом и имя с дефиса платформа не
    // берёт, а имя с точкой и с двумя двоеточиями берёт.
    assert!(create_element(&doc, &[str_value("")]).is_err());
    assert!(create_element(&doc, &[str_value("а б")]).is_err());
    assert!(create_element(&doc, &[str_value("-а")]).is_err());
    assert!(create_element(&doc, &[str_value("а.б")]).is_ok());
    assert!(create_element(&doc, &[str_value("а:б:в")]).is_ok());
    assert!(create_element(&doc, &[]).is_err());
    assert!(create_element(&doc, &[str_value(""), str_value("п:а")]).is_err());
    assert!(create_element(&doc, &[str_value(""), str_value("а")]).is_ok());
    // `СоздатьАтрибут("а", "1")` — это форма (URI, Имя) с негодным именем.
    assert!(create_attribute(&doc, &[str_value("а"), str_value("1")]).is_err());
    // Инструкция обработки — ровно два аргумента.
    assert!(create_processing_instruction(&doc, &[str_value("пи")]).is_err());
    // Фабрик у элемента нет.
    let el = call(&doc, create_element, &[str_value("а")]);
    assert!(create_element(&el, &[str_value("б")]).is_err());
}

#[test]
fn dom_children_are_inserted_removed_and_replaced() {
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    let first = call(&doc, create_element, &[str_value("а")]);
    let second = call(&doc, create_element, &[str_value("б")]);
    // ИЗМЕРЕНО: `ДобавитьДочерний` отдаёт ТОТ ЖЕ узел.
    assert_eq!(call1(&root, append_child, &first), first);
    call1(&root, append_child, &second);
    assert_eq!(kids_names(&root), "а б");
    // Вставка перед опорным узлом.
    let mid = call(&doc, create_element, &[str_value("м")]);
    call(&root, insert_before, &[mid.clone(), second.clone()]);
    assert_eq!(kids_names(&root), "а м б");
    // Удаление отдаёт удалённый узел и обнуляет ему родителя.
    let gone = call1(&root, remove_child, &mid);
    assert_eq!(gone, mid);
    assert_eq!(
        get_property(&mid, "РодительскийУзел").unwrap(),
        BslValue::Undefined
    );
    assert_eq!(get_property(&mid, "ДокументВладелец").unwrap(), doc);
    assert_eq!(kids_names(&root), "а б");
    // Замена отдаёт СТАРЫЙ узел (измерено).
    let fresh = call(&doc, create_element, &[str_value("н")]);
    let old = call(&root, replace_child, &[fresh, first.clone()]);
    assert_eq!(old, first);
    assert_eq!(kids_names(&root), "н б");
}

#[test]
fn dom_inserting_a_node_with_a_parent_moves_it() {
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    let a = call(&doc, create_element, &[str_value("а")]);
    let b = call(&doc, create_element, &[str_value("б")]);
    call1(&root, append_child, &a);
    call1(&root, append_child, &b);
    // Переезд: у прежнего родителя узла больше нет (измерено).
    call1(&b, append_child, &a);
    assert_eq!(kids_names(&root), "б");
    assert_eq!(kids_names(&b), "а");
    // Замена узлом, который уже ребёнок того же родителя, — тоже переезд.
    let c = call(&doc, create_element, &[str_value("в")]);
    call1(&root, append_child, &c);
    assert_eq!(kids_names(&root), "б в");
    call(&root, replace_child, &[c, b]).type_name();
    assert_eq!(kids_names(&root), "в");
}

#[test]
fn dom_insertion_rules_match_the_platform() {
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    let text = call(&doc, create_text_node, &[str_value("т")]);
    // Текст в ДОКУМЕНТ не идёт, а комментарий идёт (измерено).
    assert!(append_child(&doc, std::slice::from_ref(&text)).is_err());
    let comment = call(&doc, create_comment, &[str_value("к")]);
    assert!(append_child(&doc, &[comment]).is_ok());
    // Второй корневой элемент документ принимает молча.
    let second_root = call(&doc, create_element, &[str_value("второй")]);
    assert!(append_child(&doc, &[second_root]).is_ok());
    // Документ, атрибут, не-узел и предок — ошибка.
    assert!(append_child(&root, std::slice::from_ref(&doc)).is_err());
    let attr = call(&doc, create_attribute, &[str_value("а")]);
    assert!(append_child(&root, std::slice::from_ref(&attr)).is_err());
    assert!(append_child(&root, &[BslValue::Undefined]).is_err());
    assert!(append_child(&root, &[str_value("т")]).is_err());
    let inner = call(&doc, create_element, &[str_value("вн")]);
    call1(&root, append_child, &inner);
    assert!(append_child(&inner, std::slice::from_ref(&root)).is_err());
    // У текста детей не бывает, а у атрибута бывает ТЕКСТ (измерено),
    // но не элемент.
    call1(&root, append_child, &text);
    assert!(append_child(&text, &[call(&doc, create_element, &[str_value("х")])]).is_err());
    assert!(append_child(&attr, &[call(&doc, create_text_node, &[str_value("х")])]).is_ok());
    assert!(append_child(&attr, &[call(&doc, create_element, &[str_value("х")])]).is_err());
    // Узел чужого документа — ошибка.
    let other = fresh_document();
    assert!(
        append_child(
            &root,
            &[call(&other, create_element, &[str_value("чужой")])]
        )
        .is_err()
    );
    // Опорный узел вставки обязан быть ребёнком.
    let orphan = call(&doc, create_element, &[str_value("сирота")]);
    let some = call(&doc, create_element, &[str_value("нов")]);
    assert!(insert_before(&root, &[some.clone(), orphan.clone()]).is_err());
    assert!(insert_before(&root, &[some.clone(), BslValue::Undefined]).is_err());
    assert!(remove_child(&root, std::slice::from_ref(&orphan)).is_err());
    assert!(replace_child(&root, &[some, orphan]).is_err());
}

#[test]
fn dom_attributes_are_set_and_removed() {
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    call(&root, set_attribute, &[str_value("а"), str_value("1")]);
    call(&root, set_attribute, &[str_value("б"), str_value("2")]);
    // Повторная установка меняет значение НА МЕСТЕ (измерено).
    call(&root, set_attribute, &[str_value("а"), str_value("9")]);
    let attrs = get_property(&root, "Атрибуты").unwrap();
    assert_eq!(attrs.collection_len().unwrap(), 2);
    assert_eq!(
        get_attribute(&root, &[str_value("а")]).unwrap(),
        str_value("9")
    );
    let node = get_attribute_node(&root, &[str_value("а")]).unwrap();
    // Значение атрибута живёт в текстовом ребёнке (измерено).
    assert_eq!(
        get_property(&node, "ДочерниеУзлы")
            .unwrap()
            .collection_len()
            .unwrap(),
        1
    );
    // Трёхаргументная форма без префикса URI ТЕРЯЕТ (измерено).
    call(
        &root,
        set_attribute,
        &[str_value("urn:х"), str_value("в"), str_value("3")],
    );
    let plain = get_attribute_node(&root, &[str_value("в")]).unwrap();
    assert_eq!(
        text_of(&get_property(&plain, "URIПространстваИмен").unwrap()),
        ""
    );
    // Нестроковое значение и пустое имя — ошибка.
    assert!(set_attribute(&root, &[str_value("а"), BslValue::Boolean(true)]).is_err());
    assert!(set_attribute(&root, &[str_value(""), str_value("1")]).is_err());
    // `УдалитьАтрибут` берёт и имя, и пару URI с локальным именем;
    // отсутствующий атрибут — не ошибка (измерено).
    call(&root, remove_attribute, &[str_value("нет")]);
    call(&root, remove_attribute, &[str_value("б")]);
    assert_eq!(
        get_property(&root, "Атрибуты")
            .unwrap()
            .collection_len()
            .unwrap(),
        2
    );
    // Узел атрибута: занятый и чужой — ошибка, замещённый возвращается.
    let free = call(&doc, create_attribute, &[str_value("а")]);
    set_property(&free, "Значение", &str_value("7")).unwrap();
    let replaced = call1(&root, set_attribute_node, &free);
    assert_eq!(replaced, node, "замещён прежний узел атрибута");
    assert!(
        set_attribute_node(&root, std::slice::from_ref(&free)).is_err(),
        "уже занят"
    );
    let removed = call1(&root, remove_attribute_node, &free);
    assert_eq!(removed, free);
    assert!(remove_attribute_node(&root, &[free]).is_err(), "уже не наш");
}

#[test]
fn dom_writable_properties_follow_the_platform() {
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    call(
        &root,
        append_child,
        &[call(&doc, create_element, &[str_value("а")])],
    );
    call(
        &root,
        append_child,
        &[call(&doc, create_text_node, &[str_value("т")])],
    );
    // ИЗМЕРЕНО: заменяет ВСЕХ детей одним текстовым узлом.
    set_property(&root, "ТекстовоеСодержимое", &str_value("всё")).unwrap();
    assert_eq!(kids_names(&root), TEXT_NODE_NAME);
    assert_eq!(
        text_of(&get_property(&root, "ТекстовоеСодержимое").unwrap()),
        "всё"
    );
    // Пустая строка оставляет узел вовсе без детей.
    set_property(&root, "ТекстовоеСодержимое", &str_value("")).unwrap();
    assert_eq!(kids_names(&root), "");
    // Имя узла только читается; присваивание элементу `ЗначениеУзла` и
    // документу `ТекстовоеСодержимое` платформа принимает и не делает
    // ничего.
    assert!(set_property(&root, "ИмяУзла", &str_value("другое")).is_err());
    set_property(&root, "ЗначениеУзла", &str_value("х")).unwrap();
    assert_eq!(
        get_property(&root, "ЗначениеУзла").unwrap(),
        BslValue::Undefined
    );
    set_property(&doc, "ТекстовоеСодержимое", &str_value("х")).unwrap();
    assert_eq!(kids_names(&doc), "к");
    // Число значением не годится (измерено).
    let text = call(&doc, create_text_node, &[str_value("т")]);
    assert!(set_property(&text, "Данные", &BslValue::Boolean(true)).is_err());
    set_property(&text, "Данные", &str_value("новый")).unwrap();
    assert_eq!(
        text_of(&get_property(&text, "ЗначениеУзла").unwrap()),
        "новый"
    );
}

#[test]
fn dom_child_and_attribute_collections_are_live() {
    // ИЗМЕРЕНО: список, взятый ДО мутации, показывает уже двоих детей, а
    // снимок поиска остаётся прежним.
    let doc = fresh_document();
    let root = call(&doc, create_element, &[str_value("к")]);
    call1(&doc, append_child, &root);
    call(
        &root,
        append_child,
        &[call(&doc, create_element, &[str_value("а")])],
    );
    let kids = get_property(&root, "ДочерниеУзлы").unwrap();
    let found = get_elements_by_name(&doc, &[str_value("*")]).unwrap();
    let attrs = get_property(&root, "Атрибуты").unwrap();
    call(
        &root,
        append_child,
        &[call(&doc, create_element, &[str_value("б")])],
    );
    call(&root, set_attribute, &[str_value("х"), str_value("1")]);
    assert_eq!(kids.collection_len().unwrap(), 2, "дети — живое окно");
    assert_eq!(attrs.collection_len().unwrap(), 1, "атрибуты — живое окно");
    // Поиск от документа нашёл корень и его дитя — снимок так и остаётся
    // на двух, тогда как свежий запрос видит троих (измерено).
    assert_eq!(found.collection_len().unwrap(), 2, "поиск — снимок");
    assert_eq!(
        get_elements_by_name(&doc, &[str_value("*")])
            .unwrap()
            .collection_len()
            .unwrap(),
        3,
        "свежий поиск видит нового"
    );
}

#[test]
fn dom_serialization_matches_the_platform_shape() {
    let doc = fresh_document();
    // Пустой документ — только объявление, БЕЗ хвостового перевода
    // строки (измерено).
    assert_eq!(serialize(&doc).unwrap(), "<?xml version=\"1.0\"?>");
    let root = call(&doc, create_element, &[str_value("корень")]);
    call1(&doc, append_child, &root);
    call(&root, set_attribute, &[str_value("а"), str_value("1")]);
    call(
        &root,
        append_child,
        &[call(&doc, create_text_node, &[str_value("текст")])],
    );
    let inner = call(&doc, create_element, &[str_value("вн")]);
    call1(&root, append_child, &inner);
    call(
        &inner,
        append_child,
        &[call(&doc, create_comment, &[str_value("к")])],
    );
    assert_eq!(
        serialize(&doc).unwrap(),
        "<?xml version=\"1.0\"?>\n<корень а=\"1\">текст\n\t<вн>\n\t\t<!--к-->\n\t</вн>\n</корень>"
    );
    // Отдельный узел пишется без объявления, атрибут — не пишется вовсе,
    // текстоподобный узел вне элемента — ошибка (измерено все три).
    assert_eq!(serialize(&inner).unwrap(), "<вн>\n\t<!--к-->\n</вн>");
    let attr = get_attribute_node(&root, &[str_value("а")]).unwrap();
    assert_eq!(serialize(&attr).unwrap(), "");
    assert!(serialize(&call(&doc, create_text_node, &[str_value("т")])).is_err());
    assert!(serialize(&call(&doc, create_cdata_section, &[str_value("ц")])).is_err());
    // Комментарий и инструкция обработки в одиночку пишутся, и ведущего
    // перевода строки у них нет.
    assert_eq!(
        serialize(&call(&doc, create_comment, &[str_value("сам")])).unwrap(),
        "<!--сам-->"
    );
    assert_eq!(
        serialize(&call(
            &doc,
            create_processing_instruction,
            &[str_value("пи"), str_value("д")]
        ))
        .unwrap(),
        "<?пи д?>"
    );
    // Второй корень записи не даётся.
    call(
        &doc,
        append_child,
        &[call(&doc, create_element, &[str_value("второй")])],
    );
    assert!(serialize(&doc).is_err());
}

#[test]
fn dom_serialization_places_namespace_declarations() {
    let doc = fresh_document();
    let root = call(
        &doc,
        create_element,
        &[str_value("urn:раз"), str_value("р:корень")],
    );
    call1(&doc, append_child, &root);
    // Объявление для атрибута с URI писатель выдумывает сам, и
    // объявления идут ПЕРЕД обычными атрибутами, отсортированные по имени.
    call(
        &root,
        set_attribute,
        &[str_value("urn:два"), str_value("д:атр"), str_value("з")],
    );
    // Потомку с тем же пространством имён объявление не повторяется.
    call(
        &root,
        append_child,
        &[call(
            &doc,
            create_element,
            &[str_value("urn:раз"), str_value("р:дитя")],
        )],
    );
    assert_eq!(
        serialize(&doc).unwrap(),
        "<?xml version=\"1.0\"?>\n<р:корень xmlns:д=\"urn:два\" xmlns:р=\"urn:раз\" д:атр=\"з\">\n\t<р:дитя/>\n</р:корень>"
    );
    // Отдельно записанный потомок объявление получает заново.
    let child = get_property(&root, "ПоследнийДочерний").unwrap();
    assert_eq!(serialize(&child).unwrap(), "<р:дитя xmlns:р=\"urn:раз\"/>");
    // Объявление-атрибут с ПУСТЫМ URI (какой ставит `УстановитьАтрибут`)
    // при записи — ошибка (измерено).
    let plain = fresh_document();
    let bare = call(&plain, create_element, &[str_value("к")]);
    call1(&plain, append_child, &bare);
    call(
        &bare,
        set_attribute,
        &[str_value("xmlns:к"), str_value("urn:к")],
    );
    assert!(serialize(&plain).is_err());
}

/// Вставка тоже держит предел вложенности: без этого скрипт мог бы
/// собрать мутацией дерево, которое потом уронит процесс на разрушении.
#[test]
fn dom_insertion_respects_the_depth_limit() {
    let doc = fresh_document();
    let mut deepest = doc.clone();
    for _ in 0..MAX_DOM_DEPTH {
        let el = call(&doc, create_element, &[str_value("а")]);
        call1(&deepest, append_child, &el);
        deepest = el;
    }
    // Уровень ровно по пределу — текст ещё влезает, элемент уже нет.
    assert!(append_child(&deepest, &[call(&doc, create_text_node, &[str_value("х")])]).is_ok());
    let over = call(&doc, create_element, &[str_value("лишний")]);
    let err = append_child(&deepest, &[over]).expect_err("глубже предела — ошибка");
    assert!(
        matches!(err, RtError::StackOverflow { .. }),
        "ожидалась StackOverflow, получено {err:?}"
    );
}
