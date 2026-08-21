//! Мутация дерева: вставка, замена, удаление.

use super::*;

// --- Мутация дерева -----------------------------------------------------

/// Может ли узел вида `child` лежать в детях узла вида `parent`. Таблица
/// целиком ИЗМЕРЕНА (см. заголовок модуля): текст и секция CDATA в документ
/// не идут, атрибут и документ не идут никуда, а у текстоподобных узлов и
/// инструкции обработки детей не бывает.
pub(crate) fn may_contain(parent: DomKind, child: DomKind) -> bool {
    match parent {
        DomKind::Document => matches!(
            child,
            DomKind::Element | DomKind::Comment | DomKind::ProcessingInstruction
        ),
        DomKind::Element => matches!(
            child,
            DomKind::Element
                | DomKind::Text
                | DomKind::CdataSection
                | DomKind::Comment
                | DomKind::ProcessingInstruction
                | DomKind::EntityReference
        ),
        // У атрибута дети — текст: ИЗМЕРЕНО, что дописанный текстовый узел
        // платформа принимает и дописывает им значение атрибута.
        DomKind::Attribute => child == DomKind::Text,
        DomKind::Text | DomKind::CdataSection | DomKind::Comment => false,
        DomKind::ProcessingInstruction => false,
        // Детей у ссылки на сущность здесь нет: платформа сообщает о них
        // (`ДочерниеУзлы.Количество()` — 1), но взять первого не даёт —
        // `ПервыйДочерний` отвечает ошибкой. См. заголовок модуля.
        DomKind::EntityReference => false,
    }
}

/// Узел из аргумента: не узел DOM — ошибка, ровно как на платформе, где
/// `ДобавитьДочерний(Неопределено)` и `ДобавитьДочерний("т")` отвергаются.
pub(crate) fn need_node(arg: Option<&BslValue>, op: &'static str) -> RtResult<Rc<DomNode>> {
    let Some(v) = arg else {
        return Err(RtError::TypeError {
            expected: "узел DOM",
            op,
        });
    };
    match v
        .object_ref()
        .and_then(|object| object.downcast_ref::<DomNodeObject>())
    {
        Some(node) => Ok(node.node.clone()),
        _ => Err(RtError::TypeError {
            expected: "узел DOM",
            op,
        }),
    }
}

/// Один и тот же ли документ владеет обоими узлами. ИЗМЕРЕНО, что узел
/// чужого документа платформа не принимает — ни разобранный, ни созданный
/// чужой фабрикой.
pub(crate) fn same_document(a: &Rc<DomNode>, b: &Rc<DomNode>) -> bool {
    match (a.owner.borrow().upgrade(), b.owner.borrow().upgrade()) {
        (Some(x), Some(y)) => Rc::ptr_eq(&x, &y),
        _ => false,
    }
}

/// Является ли `candidate` самим `node` или его предком. Проверка не ради
/// совместимости, а ради целостности: вставка узла в своего потомка
/// замкнула бы цикл сильных ссылок, и дерево не освободилось бы никогда.
/// Платформа такую вставку тоже отвергает (измерено).
pub(crate) fn is_self_or_ancestor(candidate: &Rc<DomNode>, node: &Rc<DomNode>) -> bool {
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        if Rc::ptr_eq(&n, candidate) {
            return true;
        }
        cur = n.parent.borrow().upgrade();
    }
    false
}

/// Уровень узла в ЭЛЕМЕНТАХ: у документа 0, у корневого элемента 1 — та же
/// мера, которой считает предел построение (см. [`MAX_DOM_DEPTH`]).
pub(crate) fn element_depth(node: &Rc<DomNode>) -> usize {
    let mut depth = 0;
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        if n.kind == DomKind::Element {
            depth += 1;
        }
        cur = n.parent.borrow().upgrade();
    }
    depth
}

/// Высота поддерева в тех же элементных уровнях. Рекурсия здесь безопасна:
/// поддерево уже подчиняется [`MAX_DOM_DEPTH`], иначе его не удалось бы
/// собрать.
pub(crate) fn element_height(node: &Rc<DomNode>) -> usize {
    let mut deepest = 0;
    for c in node.children.borrow().iter() {
        deepest = deepest.max(element_height(c));
    }
    deepest + usize::from(node.kind == DomKind::Element)
}

/// Все проверки вставки, общие для добавления, вставки перед и замены.
pub(crate) fn check_insert(
    parent: &Rc<DomNode>,
    child: &Rc<DomNode>,
    op: &'static str,
) -> RtResult<()> {
    if !may_contain(parent.kind, child.kind) {
        return Err(RtError::TypeError {
            expected: "узел, допустимый в этом родителе",
            op,
        });
    }
    if !same_document(parent, child) {
        return Err(RtError::TypeError {
            expected: "узел того же документа",
            op,
        });
    }
    if is_self_or_ancestor(child, parent) {
        return Err(RtError::TypeError {
            expected: "узел, не являющийся предком приёмника",
            op,
        });
    }
    if element_depth(parent) + element_height(child) > MAX_DOM_DEPTH {
        return Err(too_deep());
    }
    Ok(())
}

/// Отцепить узел от нынешнего родителя. Узел, у которого родитель уже есть,
/// при вставке ПЕРЕЕЗЖАЕТ (измерено), поэтому это не ошибка, а шаг вставки.
pub(crate) fn detach(node: &Rc<DomNode>) {
    let parent = node.parent.borrow().upgrade();
    if let Some(p) = parent {
        let mut kids = p.children.borrow_mut();
        if let Some(i) = kids.iter().position(|c| Rc::ptr_eq(c, node)) {
            kids.remove(i);
        }
    }
    *node.parent.borrow_mut() = Weak::new();
}

/// Ребёнок ли `child` у `parent` — с ошибкой, если нет. Так платформа
/// отвечает и на удаление, и на замену, и на опорный узел вставки
/// (измерено все три).
pub(crate) fn child_index(
    parent: &Rc<DomNode>,
    child: &Rc<DomNode>,
    op: &'static str,
) -> RtResult<usize> {
    DomNode::index_in(child, parent).ok_or(RtError::TypeError {
        expected: "узел из детей этого родителя",
        op,
    })
}

/// `ДобавитьДочерний(Узел)` -> тот же узел (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не узел DOM, вид узла в этом
/// родителе недопустим, узел из другого документа или является предком
/// приёмника; [`RtError::StackOverflow`] при превышении предела вложенности.
pub fn append_child(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "ДобавитьДочерний";
    let (parent, doc) = as_node(obj, op)?;
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент — узел DOM",
            op,
        });
    }
    let child = need_node(args.first(), op)?;
    check_insert(&parent, &child, op)?;
    detach(&child);
    DomNode::append(&parent, &child, &doc);
    Ok(node_value(&child, &doc))
}

/// `ВставитьПеред(Новый, Опорный)` -> вставленный узел.
///
/// Опорный узел ОБЯЗАТЕЛЕН: `Неопределено` вместо него платформа отвергает,
/// а не трактует как «добавить в конец» (измерено).
///
/// # Errors
///
/// Как у [`append_child`], плюс если опорный узел не ребёнок получателя.
pub fn insert_before(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "ВставитьПеред";
    let (parent, doc) = as_node(obj, op)?;
    if args.len() != 2 {
        return Err(RtError::TypeError {
            expected: "ровно два аргумента — новый и опорный узлы",
            op,
        });
    }
    let fresh = need_node(args.first(), op)?;
    let anchor = need_node(args.get(1), op)?;
    child_index(&parent, &anchor, op)?;
    check_insert(&parent, &fresh, op)?;
    // Порядок важен: отцепить нужно ДО поиска места, иначе номер опорного
    // узла сдвинется, если вставляемый лежал перед ним у того же родителя.
    detach(&fresh);
    let at = child_index(&parent, &anchor, op)?;
    *fresh.parent.borrow_mut() = Rc::downgrade(&parent);
    *fresh.owner.borrow_mut() = Rc::downgrade(&doc);
    parent.children.borrow_mut().insert(at, fresh.clone());
    Ok(node_value(&fresh, &doc))
}

/// `УдалитьДочерний(Узел)` -> удалённый узел.
///
/// ИЗМЕРЕНО: родитель у него становится `Неопределено`, а документ-владелец
/// остаётся тем же.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не узел DOM или не ребёнок
/// получателя.
pub fn remove_child(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "УдалитьДочерний";
    let (parent, doc) = as_node(obj, op)?;
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент — узел DOM",
            op,
        });
    }
    let child = need_node(args.first(), op)?;
    let at = child_index(&parent, &child, op)?;
    parent.children.borrow_mut().remove(at);
    *child.parent.borrow_mut() = Weak::new();
    Ok(node_value(&child, &doc))
}

/// `ЗаменитьДочерний(Новый, Старый)` -> СТАРЫЙ узел (измерено).
///
/// # Errors
///
/// Как у [`append_child`], плюс если старый узел не ребёнок получателя.
pub fn replace_child(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "ЗаменитьДочерний";
    let (parent, doc) = as_node(obj, op)?;
    if args.len() != 2 {
        return Err(RtError::TypeError {
            expected: "ровно два аргумента — новый и старый узлы",
            op,
        });
    }
    let fresh = need_node(args.first(), op)?;
    let old = need_node(args.get(1), op)?;
    child_index(&parent, &old, op)?;
    check_insert(&parent, &fresh, op)?;
    // Как и у вставки: сначала отцепить, потом искать место старого.
    detach(&fresh);
    let at = child_index(&parent, &old, op)?;
    *fresh.parent.borrow_mut() = Rc::downgrade(&parent);
    *fresh.owner.borrow_mut() = Rc::downgrade(&doc);
    parent.children.borrow_mut()[at] = fresh;
    *old.parent.borrow_mut() = Weak::new();
    Ok(node_value(&old, &doc))
}
