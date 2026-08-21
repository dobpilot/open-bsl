//! Чтение XML в экземпляры XDTO.

use super::*;

// --- чтение XML в экземпляры ---------------------------------------------

/// Пространство имён экземпляров XML Schema: `xsi:nil` и `xsi:type`.
pub(crate) const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Предел вложенности экземпляра — общий для разбора документа и для
/// обратной записи. Оба спуска рекурсивные, и без предела документ на
/// тысячи уровней (а при записи ещё и цикл в графе экземпляров) уронил бы
/// стек процесса вместо честной ошибки.
// НЕ ИЗМЕРЕНО(XDTO.MAX_DEPTH) — как глубоко 8.3.27 позволяет вкладывать
// экземпляр в `ПрочитатьXML`/`ЗаписатьXML` фабрики и что платформа делает
// с циклическим графом экземпляров при записи; ни растущий, ни циклический
// зонд намеренно не ставятся: если платформа на них падает, они уносят
// весь сеанс замеров. Замер даёт нижнюю границу: 400 уровней обязаны
// записываться.
pub(crate) const MAX_XDTO_DEPTH: usize = 500;

/// Голова начального тега — ровно то, что несёт событие `ЧтениеXML`.
pub(crate) struct ElementHead {
    pub(crate) name: String,
    pub(crate) uri: String,
    pub(crate) attrs: Rc<Vec<crate::core::XmlAttr>>,
}

/// Что вышло из одного элемента.
pub(crate) enum ReadOut {
    /// `xsi:nil="true"`: свойство заполнено, а значения нет (измерено —
    /// `Установлено` даёт «Да», само свойство — `Неопределено`).
    Nil,
    /// Значение простого типа вместе с лексической формой, из которой оно
    /// получено: наверху из пары строится `ЗначениеXDTO`, а в свойстве
    /// остаётся одно значение (измерено: `О.num` после чтения — `Число`,
    /// а `ПрочитатьXML(Чт, xs:int)` — `ЗначениеXDTO`).
    Simple(BslValue, String),
    /// Экземпляр объекта.
    Object(BslValue),
    /// Текст элемента типа `anyType` — платформа кладёт его СТРОКОЙ
    /// (измерено: `<notype>5</notype>` в свойстве типа `anyType` читается
    /// как строка «5», а с `xsi:type="xs:decimal"` — как число).
    Open(String),
}

/// `ФабрикаXDTO.ПрочитатьXML(ЧтениеXML, Тип)`.
///
/// Тип обязателен, хотя платформа принимает и вызов с одним аргументом:
/// без типа она читает документ в ОТКРЫТОЕ содержимое `anyType`, где
/// свойства заводятся по ходу разбора, а значения остаются строками
/// (измерено, см. заголовок модуля). Открытого содержимого в этой
/// реализации нет, и молчаливая подмена его разбором по схеме означала бы
/// другой результат, поэтому вызов без типа отвечает отказом.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика;
/// [`RtError::Xdto`], если аргументы не те, документ не соответствует типу
/// или в нём нет элементов; [`RtError::Xml`] на битой разметке.
pub fn factory_read_xml(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    factory_model(obj, "ПрочитатьXML")?;
    let (reader, type_arg) = match args {
        [reader] => (reader, None),
        [reader, ty] => (reader, Some(ty)),
        _ => return Err(not_applicable(obj, "ПрочитатьXML")),
    };
    if !crate::xml::is_xml_reader(reader) {
        return Err(RtError::Xdto(
            "ПрочитатьXML читает из «ЧтениеXML»".to_string(),
        ));
    }
    let Some(type_arg) = type_arg else {
        return Err(RtError::Xdto(
            "ПрочитатьXML без типа читает документ в открытое содержимое \
             «anyType», а оно здесь не поддержано: передайте тип из \
             «ФабрикаXDTO.Тип»"
                .to_string(),
        ));
    };
    let (model, type_index) = match repr_of(type_arg) {
        Some(XdtoRepr::Type(model, i)) => (model.clone(), *i),
        _ => return Err(bad_read_type(type_arg)),
    };
    crate::xml::with_reader(crate::xml::arg_object(reader)?, |state| {
        read_document(state, &model, type_index)
    })
}

pub(crate) fn bad_read_type(arg: &BslValue) -> RtError {
    RtError::Xdto(format!(
        "второй аргумент «ПрочитатьXML» — тип XDTO, а не «{}»",
        arg.type_name()
    ))
}

/// Разбор одного элемента источника в значение указанного типа.
///
/// Читатель встаёт на СЛЕДУЮЩИЙ за прочитанным элементом узел — измерено:
/// после чтения вложенного элемента `ТипУзла` показывает начало соседа, а
/// после чтения корня документа читатель исчерпан («Ничего», `Прочитать()`
/// — «Нет»). Поэтому два элемента подряд читаются двумя вызовами без
/// `Прочитать()` между ними.
pub(crate) fn read_document(
    state: &mut crate::xml::XmlReaderState,
    model: &Rc<XdtoModel>,
    type_index: usize,
) -> RtResult<BslValue> {
    let out = read_one(state, |parser, head| {
        read_element(parser, model, type_index, head, 0)
    })?;
    match out {
        ReadOut::Nil => Ok(BslValue::Undefined),
        ReadOut::Object(value) => Ok(value),
        ReadOut::Simple(value, lexical) => Ok(data_value(
            model,
            &Rc::new(XdtoValueData {
                value,
                lexical,
                type_index,
            }),
        )),
        ReadOut::Open(_) => Err(RtError::Xdto(
            "чтение в тип «anyType» — это открытое содержимое, оно здесь не \
             поддержано"
                .to_string(),
        )),
    }
}

/// Обвязка одного чтения, общая у фабрики и у сериализатора: довести
/// читателя до первого начального тега, разобрать ровно один элемент и
/// оставить читателя на СЛЕДУЮЩЕМ за ним узле.
///
/// Позиция измерена и на той, и на другой стороне: и свежий читатель, и
/// читатель после `Прочитать()`, и после `ПерейтиКСодержимому()` дают один
/// и тот же результат, а после разбора корня читатель исчерпан
/// («Ничего», `Прочитать()` — «Нет»). Отсюда и два соседних элемента,
/// читаемых двумя вызовами без `Прочитать()` между ними.
///
/// # Errors
///
/// [`RtError::Xdto`], если источник не задан или в нём не осталось
/// элементов; ошибка самого разбора приходит из `read`.
pub(crate) fn read_one<T>(
    state: &mut crate::xml::XmlReaderState,
    read: impl FnOnce(&mut crate::core::XmlParser, &ElementHead) -> RtResult<T>,
) -> RtResult<T> {
    let mut current = state.current.take();
    let parser = state
        .parser
        .as_mut()
        .ok_or_else(|| RtError::Xdto("источник для ЧтениеXML не задан".to_string()))?;
    let head = loop {
        match current {
            Some(crate::core::XmlEvent::ElementStart { name, uri, attrs }) => {
                break ElementHead { name, uri, attrs };
            }
            _ => match parser.read()? {
                Some(event) => current = Some(event),
                None => {
                    return Err(RtError::Xdto(
                        "ПрочитатьXML: в источнике не осталось элементов".to_string(),
                    ));
                }
            },
        }
    };
    let out = read(parser, &head)?;
    let next = parser.read()?;
    state.depth = state
        .parser
        .as_ref()
        .map_or(0, crate::core::XmlParser::depth);
    state.current = next;
    state.attr_cursor = None;
    Ok(out)
}

/// Разбор элемента, начальный тег которого уже прочитан: разбор идёт до
/// парного закрывающего тега включительно.
pub(crate) fn read_element(
    parser: &mut crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    type_index: usize,
    head: &ElementHead,
    depth: usize,
) -> RtResult<ReadOut> {
    if depth > MAX_XDTO_DEPTH {
        return Err(RtError::Xdto(format!(
            "документ вложен глубже предела разбора XDTO ({MAX_XDTO_DEPTH})"
        )));
    }
    if reads_as_nil(parser, head) {
        skip_element(parser)?;
        return Ok(ReadOut::Nil);
    }
    let data = model.type_at(type_index)?;
    if is_any_type(data) {
        return read_open(parser, head);
    }
    if data.is_value() {
        return read_simple(parser, model, type_index, head);
    }
    read_object(parser, model, type_index, head, depth)
}

/// `xsi:nil="true"` у элемента. Цифровая запись `1` — та же, что у
/// `xs:boolean`, где обе формы измерены.
pub(crate) fn reads_as_nil(parser: &crate::core::XmlParser, head: &ElementHead) -> bool {
    head.attrs.iter().any(|a| {
        attribute_uri(parser, &a.name) == XSI_NS
            && crate::core::local_of(&a.name) == "nil"
            && (a.value == "true" || a.value == "1")
    })
}

/// URI атрибута: у атрибута без префикса пространства имён нет НИКОГДА —
/// умолчательное объявление на атрибуты не распространяется.
pub(crate) fn attribute_uri(parser: &crate::core::XmlParser, name: &str) -> String {
    let prefix = crate::core::prefix_of(name);
    if prefix.is_empty() {
        String::new()
    } else {
        parser.namespace_of(prefix)
    }
}

/// Объявление пространства имён, а не атрибут данных.
pub(crate) fn is_ns_declaration(name: &str) -> bool {
    name == "xmlns" || crate::core::prefix_of(name) == "xmlns"
}

/// Проглотить остаток текущего элемента вместе с его закрывающим тегом.
pub(crate) fn skip_element(parser: &mut crate::core::XmlParser) -> RtResult<()> {
    let target = parser.depth().saturating_sub(1);
    while let Some(event) = parser.read()? {
        if matches!(event, crate::core::XmlEvent::ElementEnd { .. }) && parser.depth() == target {
            return Ok(());
        }
    }
    Err(RtError::Xdto(
        "документ оборвался внутри элемента".to_string(),
    ))
}

/// Собрать текст элемента. Вложенный элемент — ошибка: у простого типа
/// содержимого быть не может.
pub(crate) fn read_text_only(
    parser: &mut crate::core::XmlParser,
    head: &ElementHead,
) -> RtResult<String> {
    let target = parser.depth().saturating_sub(1);
    let mut text = String::new();
    loop {
        let Some(event) = parser.read()? else {
            return Err(RtError::Xdto(format!(
                "документ оборвался внутри элемента «{}»",
                head.name
            )));
        };
        match event {
            crate::core::XmlEvent::ElementEnd { .. } if parser.depth() == target => {
                return Ok(text);
            }
            crate::core::XmlEvent::Text(t) => text.push_str(&t),
            crate::core::XmlEvent::ElementStart { name, .. } => {
                return Err(RtError::Xdto(format!(
                    "элемент «{}» простого типа не может содержать элемент «{name}»",
                    head.name
                )));
            }
            // Инструкции обработки и комментарии значения не несут;
            // чужого закрывающего тега разборщик не отдаёт.
            crate::core::XmlEvent::ElementEnd { .. }
            | crate::core::XmlEvent::ProcessingInstruction { .. }
            | crate::core::XmlEvent::Comment(_) => {}
            // Ссылку на сущность разборщик отдаёт узлом, а не текстом:
            // подставить её значение здесь нечем, а молча потерять —
            // испортить прочитанное.
            crate::core::XmlEvent::EntityReference { name } => {
                return Err(RtError::Xdto(format!(
                    "ссылка на сущность «&{name};» при чтении XDTO не поддерживается"
                )));
            }
        }
    }
}

/// Элемент типа ЗНАЧЕНИЯ. Посторонний атрибут платформа отвергает
/// (измерено: `<число а="1">42</число>` при чтении с типом `xs:int` —
/// ошибка), поэтому пропускаются только объявления пространств имён и
/// служебные `xsi:*`.
pub(crate) fn read_simple(
    parser: &mut crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    type_index: usize,
    head: &ElementHead,
) -> RtResult<ReadOut> {
    for attr in head.attrs.iter() {
        if is_ns_declaration(&attr.name) || attribute_uri(parser, &attr.name) == XSI_NS {
            continue;
        }
        return Err(RtError::Xdto(format!(
            "у элемента «{}» простого типа не может быть атрибута «{}»",
            head.name, attr.name
        )));
    }
    let text = read_text_only(parser, head)?;
    // Чтение с типом ПРОВЕРЯЕТ документ по фасетам так же, как запись
    // проверяет присваивание (измерено: элемент со значением вне
    // перечисления, вне длины, вне диапазона и с лишними разрядами
    // платформа отвергает).
    let value = value_from_lexical_checked(model, type_index, &text)?;
    Ok(ReadOut::Simple(value, text))
}

/// Элемент типа `anyType`. Поддержан ровно измеренный случай — текст,
/// который платформа отдаёт строкой; всё остальное (атрибуты и вложенные
/// элементы) — это уже открытое содержимое.
pub(crate) fn read_open(
    parser: &mut crate::core::XmlParser,
    head: &ElementHead,
) -> RtResult<ReadOut> {
    for attr in head.attrs.iter() {
        if is_ns_declaration(&attr.name) || attribute_uri(parser, &attr.name) == XSI_NS {
            continue;
        }
        return Err(open_content(&head.name));
    }
    let target = parser.depth().saturating_sub(1);
    let mut text = String::new();
    loop {
        let Some(event) = parser.read()? else {
            return Err(RtError::Xdto(format!(
                "документ оборвался внутри элемента «{}»",
                head.name
            )));
        };
        match event {
            crate::core::XmlEvent::ElementEnd { .. } if parser.depth() == target => {
                return Ok(ReadOut::Open(text));
            }
            crate::core::XmlEvent::Text(t) => text.push_str(&t),
            crate::core::XmlEvent::ElementStart { .. } => return Err(open_content(&head.name)),
            crate::core::XmlEvent::ElementEnd { .. }
            | crate::core::XmlEvent::ProcessingInstruction { .. }
            | crate::core::XmlEvent::Comment(_) => {}
            // Ссылку на сущность разборщик отдаёт узлом, а не текстом:
            // подставить её значение здесь нечем, а молча потерять —
            // испортить прочитанное.
            crate::core::XmlEvent::EntityReference { name } => {
                return Err(RtError::Xdto(format!(
                    "ссылка на сущность «&{name};» при чтении XDTO не поддерживается"
                )));
            }
        }
    }
}

pub(crate) fn open_content(name: &str) -> RtError {
    RtError::Xdto(format!(
        "содержимое элемента «{name}» типа «anyType» — открытое, а открытое \
         содержимое здесь не поддержано"
    ))
}

/// Элемент типа ОБЪЕКТА: атрибуты, содержимое, проверка порядка и
/// обязательных свойств — всё так, как это делает платформа.
pub(crate) fn read_object(
    parser: &mut crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    type_index: usize,
    head: &ElementHead,
    depth: usize,
) -> RtResult<ReadOut> {
    let instance = Rc::new(XdtoObjectData {
        model: model.clone(),
        type_index,
        owner: RefCell::new(Weak::new()),
        entries: RefCell::new(Vec::new()),
    });
    let props = model.type_at(type_index)?.properties.clone();
    let ordered = model.type_at(type_index)?.ordered;

    for attr in head.attrs.iter() {
        if is_ns_declaration(&attr.name) {
            continue;
        }
        let uri = attribute_uri(parser, &attr.name);
        if uri == XSI_NS {
            continue;
        }
        let local = crate::core::local_of(&attr.name);
        let Some(k) = find_property(model, &props, local, &uri, EnumValue::XmlFormAttribute)?
        else {
            return Err(RtError::Xdto(format!(
                "у типа «{}» нет свойства-атрибута «{}»",
                type_display(model.type_at(type_index)?),
                attr.name
            )));
        };
        let prop = props[k];
        let target = model.property_at(prop)?.type_index;
        let value = value_from_lexical_checked(model, target, &attr.value)?;
        push_entry(&instance, prop, value);
    }

    // Текстовое свойство `__content` бывает только у типа с простым
    // содержимым, и текст в нём — это ВЕСЬ текст элемента.
    let content_prop = find_property(model, &props, CONTENT_PROPERTY, "", EnumValue::XmlFormText)?
        .map(|k| props[k]);
    let mut content = String::new();
    // Позиция последнего совпавшего свойства-элемента: по ней проверяется
    // порядок у УПОРЯДОЧЕННОГО типа (измерено: `<num>` перед `<name>` в
    // типе-последовательности — ошибка, как и второй `<tag>` после
    // `<nested>`).
    let mut matched: Option<usize> = None;
    let end_depth = parser.depth().saturating_sub(1);
    loop {
        let Some(event) = parser.read()? else {
            return Err(RtError::Xdto(format!(
                "документ оборвался внутри элемента «{}»",
                head.name
            )));
        };
        match event {
            crate::core::XmlEvent::ElementEnd { .. } if parser.depth() == end_depth => break,
            crate::core::XmlEvent::Text(text) => {
                if content_prop.is_some() {
                    content.push_str(&text);
                } else if !text.trim().is_empty() {
                    // Пробельный текст между дочерними элементами
                    // платформа пропускает, значащий — отвергает
                    // (измерено на обоих).
                    return Err(RtError::Xdto(format!(
                        "элемент «{}» составного типа не может содержать текст",
                        head.name
                    )));
                }
            }
            crate::core::XmlEvent::ElementStart { name, uri, attrs } => {
                let child = ElementHead { name, uri, attrs };
                let local = crate::core::local_of(&child.name);
                let Some(k) =
                    find_property(model, &props, local, &child.uri, EnumValue::XmlFormElement)?
                else {
                    return Err(RtError::Xdto(format!(
                        "у типа «{}» нет свойства-элемента «{}»",
                        type_display(model.type_at(type_index)?),
                        child.name
                    )));
                };
                let prop = props[k];
                if ordered {
                    check_order(model, &instance, &props, matched, k)?;
                }
                matched = Some(k);
                if !is_multiple(model.property_at(prop)?) && occupied(&instance, prop) {
                    return Err(RtError::Xdto(format!(
                        "свойство «{}» одиночное, а элемент «{}» встретился второй раз",
                        model.property_at(prop)?.name,
                        child.name
                    )));
                }
                let child_type = child_type_of(parser, model, prop, &child)?;
                let out = read_element(parser, model, child_type, &child, depth + 1)?;
                let value = match out {
                    ReadOut::Nil => BslValue::Undefined,
                    ReadOut::Simple(value, _) => value,
                    ReadOut::Open(text) => str_value(&text),
                    ReadOut::Object(value) => {
                        if let Some(XdtoRepr::Object(inner)) = repr_of(&value) {
                            *inner.owner.borrow_mut() = Rc::downgrade(&instance);
                        }
                        value
                    }
                };
                push_entry(&instance, prop, value);
            }
            crate::core::XmlEvent::ElementEnd { .. }
            | crate::core::XmlEvent::ProcessingInstruction { .. }
            | crate::core::XmlEvent::Comment(_) => {}
            // Ссылку на сущность разборщик отдаёт узлом, а не текстом:
            // подставить её значение здесь нечем, а молча потерять —
            // испортить прочитанное.
            crate::core::XmlEvent::EntityReference { name } => {
                return Err(RtError::Xdto(format!(
                    "ссылка на сущность «&{name};» при чтении XDTO не поддерживается"
                )));
            }
        }
    }

    if let Some(prop) = content_prop {
        let target = model.property_at(prop)?.type_index;
        let value = value_from_lexical_checked(model, target, &content)?;
        push_entry(&instance, prop, value);
    }
    for &prop in &props {
        let data = model.property_at(prop)?;
        if data.lower.unwrap_or(0) >= 1 && !occupied(&instance, prop) {
            return Err(RtError::Xdto(format!(
                "в элементе «{}» нет обязательного свойства «{}»",
                head.name, data.name
            )));
        }
    }
    Ok(ReadOut::Object(instance_value(&instance)))
}

/// Номер свойства в СПЛЮЩЕННОМ списке типа по имени, пространству имён и
/// форме. Сравнение имён точное: имена XML регистр различают, и свёртка
/// здесь дала бы совпадение там, где документ его не даёт.
pub(crate) fn find_property(
    model: &Rc<XdtoModel>,
    props: &[usize],
    name: &str,
    uri: &str,
    form: EnumValue,
) -> RtResult<Option<usize>> {
    for (k, &prop) in props.iter().enumerate() {
        let data = model.property_at(prop)?;
        if data.form == form && data.name == name && data.ns == uri {
            return Ok(Some(k));
        }
    }
    Ok(None)
}

/// Проверка порядка у упорядоченного типа: назад ходить нельзя, а всё
/// пропущенное по дороге обязано быть необязательным.
pub(crate) fn check_order(
    model: &Rc<XdtoModel>,
    instance: &Rc<XdtoObjectData>,
    props: &[usize],
    matched: Option<usize>,
    k: usize,
) -> RtResult<()> {
    if let Some(prev) = matched {
        if k < prev {
            return Err(RtError::Xdto(format!(
                "свойство «{}» стоит в типе раньше уже прочитанного «{}»",
                model.property_at(props[k])?.name,
                model.property_at(props[prev])?.name
            )));
        }
        if k == prev {
            return Ok(());
        }
    }
    let from = matched.map_or(0, |prev| prev + 1);
    for &prop in props.iter().take(k).skip(from) {
        let data = model.property_at(prop)?;
        if data.form == EnumValue::XmlFormElement
            && data.lower.unwrap_or(0) >= 1
            && !occupied(instance, prop)
        {
            return Err(RtError::Xdto(format!(
                "не заполнено обязательное свойство «{}»",
                data.name
            )));
        }
    }
    Ok(())
}

/// Тип дочернего элемента: объявленный, а при `xsi:type` — названный им,
/// если он и правда наследник объявленного. Неизвестное имя платформа
/// ИГНОРИРУЕТ и читает объявленным типом (измерено).
pub(crate) fn child_type_of(
    parser: &crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    prop: usize,
    head: &ElementHead,
) -> RtResult<usize> {
    let declared = model.property_at(prop)?.type_index;
    let Some(qname) = head.attrs.iter().find(|a| {
        attribute_uri(parser, &a.name) == XSI_NS && crate::core::local_of(&a.name) == "type"
    }) else {
        return Ok(declared);
    };
    // Имя типа — это QName в СОДЕРЖИМОМ атрибута, и запись без префикса
    // разрешается умолчательным объявлением (измерено обе записи:
    // `t:InnerExt` при префиксном объявлении и `InnerExt` при
    // умолчательном дают один и тот же тип).
    let prefix = crate::core::prefix_of(&qname.value);
    let uri = parser.namespace_of(prefix);
    let local = crate::core::local_of(&qname.value);
    match model.find(&uri, local) {
        Some(actual) if derives_from(model, actual, declared) => Ok(actual),
        _ => Ok(declared),
    }
}

/// Заполнение свойства при разборе. Приведение здесь не нужно и вредно:
/// значение уже получено из лексической формы ИМЕННО того типа, которым
/// элемент читался, а `xsi:type` мог сделать этот тип уже объявленного.
pub(crate) fn push_entry(instance: &Rc<XdtoObjectData>, prop: usize, value: BslValue) {
    instance
        .entries
        .borrow_mut()
        .push(XdtoEntry { prop, value });
}

/// Заполнено ли свойство хотя бы раз.
pub(crate) fn occupied(instance: &Rc<XdtoObjectData>, prop: usize) -> bool {
    instance.entries.borrow().iter().any(|e| e.prop == prop)
}
