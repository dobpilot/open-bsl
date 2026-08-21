//! Атрибуты элемента.

use super::*;

// --- Атрибуты элемента ---------------------------------------------------

/// Пара «URI, локальное имя», по которой атрибут ищется среди своих.
pub(crate) fn attr_key(uri: &str, local: &str, list: &[Rc<DomNode>]) -> Option<usize> {
    list.iter()
        .position(|a| a.uri == uri && a.local_name() == local)
}

/// `УстановитьАтрибут(Имя, Значение)` / `(URI, Имя, Значение)`.
///
/// На платформе это ПРОЦЕДУРА; здесь, как и все встроенные методы без
/// результата, отдаёт `Неопределено` (см. заголовок модуля).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не элемент;
/// [`RtError::TypeError`] при негодном имени, нестроковом значении или
/// неверной арности.
pub fn set_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "УстановитьАтрибут";
    let (el, doc) = as_element(obj, op)?;
    let (head, value) = match args.len() {
        2 => (&args[..1], need_str(args.get(1), op)?),
        3 => (&args[..2], need_str(args.get(2), op)?),
        _ => {
            return Err(RtError::TypeError {
                expected: "два или три аргумента",
                op,
            });
        }
    };
    let (name, local, prefix, uri) = named_parts(DomKind::Attribute, head, op)?;
    // ИЗМЕРЕНО: у трёхаргументной формы с именем БЕЗ префикса URI теряется —
    // пространство имён по умолчанию на атрибуты не распространяется.
    let (local, uri) = if prefix.is_empty() {
        (name.clone(), String::new())
    } else {
        (local, uri)
    };
    let existing = attr_key(&uri, &local, &el.attrs.borrow());
    match existing {
        // ИЗМЕРЕНО: повторная установка меняет значение НА МЕСТЕ — атрибут
        // не уезжает в конец коллекции.
        Some(i) => {
            let attr = el.attrs.borrow()[i].clone();
            DomNode::set_text_children(&attr, &value, &doc);
        }
        None => {
            let attr = DomNode::raw(DomKind::Attribute, name, local, prefix, uri, None);
            *attr.parent.borrow_mut() = Rc::downgrade(&el);
            *attr.owner.borrow_mut() = Rc::downgrade(&doc);
            DomNode::set_text_children(&attr, &value, &doc);
            el.attrs.borrow_mut().push(attr);
        }
    }
    Ok(BslValue::Undefined)
}

/// `УдалитьАтрибут(Имя)` / `(URI, ЛокальноеИмя)`.
///
/// Атрибут, которого нет, — не ошибка (измерено). Как и
/// [`set_attribute`], на платформе это процедура.
///
/// # Errors
///
/// Как у [`get_attribute`].
pub fn remove_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "УдалитьАтрибут";
    let (el, _) = as_element(obj, op)?;
    if let Some(attr) = find_attribute(&el, args, op)? {
        let mut attrs = el.attrs.borrow_mut();
        if let Some(i) = attrs.iter().position(|a| Rc::ptr_eq(a, &attr)) {
            attrs.remove(i);
        }
        drop(attrs);
        *attr.parent.borrow_mut() = Weak::new();
    }
    Ok(BslValue::Undefined)
}

/// `УстановитьУзелАтрибута(Атрибут)` -> ЗАМЕЩЁННЫЙ атрибут либо
/// `Неопределено` (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не элемент;
/// [`RtError::TypeError`], если аргумент не атрибут, атрибут из другого
/// документа или уже висит на элементе.
pub fn set_attribute_node(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "УстановитьУзелАтрибута";
    let (el, doc) = as_element(obj, op)?;
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент — атрибут DOM",
            op,
        });
    }
    let attr = need_node(args.first(), op)?;
    if attr.kind != DomKind::Attribute {
        return Err(RtError::TypeError {
            expected: "атрибут DOM",
            op,
        });
    }
    if !same_document(&el, &attr) {
        return Err(RtError::TypeError {
            expected: "атрибут того же документа",
            op,
        });
    }
    // ИЗМЕРЕНО: атрибут, у которого элемент-владелец уже есть, платформа не
    // перевешивает, а отвергает — в отличие от детей, которые переезжают.
    if attr.parent.borrow().upgrade().is_some() {
        return Err(RtError::TypeError {
            expected: "атрибут без элемента-владельца",
            op,
        });
    }
    let replaced = attr_key(&attr.uri, attr.local_name(), &el.attrs.borrow());
    *attr.parent.borrow_mut() = Rc::downgrade(&el);
    *attr.owner.borrow_mut() = Rc::downgrade(&doc);
    match replaced {
        Some(i) => {
            let old = el.attrs.borrow()[i].clone();
            el.attrs.borrow_mut()[i] = attr;
            *old.parent.borrow_mut() = Weak::new();
            Ok(node_value(&old, &doc))
        }
        None => {
            el.attrs.borrow_mut().push(attr);
            Ok(BslValue::Undefined)
        }
    }
}

/// `УдалитьУзелАтрибута(Атрибут)` -> удалённый атрибут (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не атрибут этого элемента.
pub fn remove_attribute_node(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "УдалитьУзелАтрибута";
    let (el, doc) = as_element(obj, op)?;
    if args.len() != 1 {
        return Err(RtError::TypeError {
            expected: "ровно один аргумент — атрибут DOM",
            op,
        });
    }
    let attr = need_node(args.first(), op)?;
    let mut attrs = el.attrs.borrow_mut();
    let Some(i) = attrs.iter().position(|a| Rc::ptr_eq(a, &attr)) else {
        return Err(RtError::TypeError {
            expected: "атрибут этого элемента",
            op,
        });
    };
    attrs.remove(i);
    drop(attrs);
    *attr.parent.borrow_mut() = Weak::new();
    Ok(node_value(&attr, &doc))
}
