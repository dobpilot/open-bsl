//! Запись экземпляров XDTO в XML.

use super::*;

// --- запись экземпляров в XML --------------------------------------------

/// Что пишется одним элементом.
pub(crate) enum Slot<'a> {
    /// Экземпляр объекта — со своей моделью: свойства читаются по ней.
    Object(&'a Rc<XdtoObjectData>),
    /// Значение простого типа: тип, чьей лексической формой его писать, и
    /// само значение.
    Simple {
        model: &'a Rc<XdtoModel>,
        type_index: usize,
        value: &'a BslValue,
    },
    /// `xsi:nil="true"` — свойство заполнено, а значения нет.
    Nil,
}

/// Тип, который уходит в `xsi:type`: пара «модель и номер типа в ней».
/// Модель здесь своя у каждого значения, потому что вложенный экземпляр
/// вправе прийти из другой фабрики.
pub(crate) type Annotation<'a> = Option<(&'a Rc<XdtoModel>, usize)>;

/// Объявления пространств имён, действующие на текущем элементе.
#[derive(Default)]
pub(crate) struct NsFrame {
    /// Объявление по умолчанию, если элемент его сделал.
    pub(crate) default: Option<String>,
    /// `(префикс, URI)` в порядке объявления.
    pub(crate) prefixes: Vec<(String, String)>,
}

impl NsFrame {
    pub(crate) fn prefix_of(&self, uri: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .find(|(_, u)| u == uri)
            .map(|(p, _)| p.as_str())
    }
}

/// Стек объявлений: у каждого записанного элемента свой уровень.
///
/// Область НЕ наследуется от того, что писали в `ЗаписьXML` руками до
/// вызова: измерено, что `ЗаписатьXML` внутри открытого чужого элемента
/// объявляет всё заново, и два вызова подряд в один писатель повторяют
/// объявления оба раза.
#[derive(Default)]
pub(crate) struct NsScope {
    pub(crate) frames: Vec<NsFrame>,
}

/// К чему привязан URI в области видимости.
pub(crate) enum NsUse {
    /// К объявлению по умолчанию — имя пишется без префикса.
    Default,
    /// К префиксу.
    Prefix(String),
}

impl NsScope {
    /// Действующее объявление по умолчанию; вне всяких объявлений это
    /// пустой URI.
    pub(crate) fn default_uri(&self) -> &str {
        self.frames
            .iter()
            .rev()
            .find_map(|f| f.default.as_deref())
            .unwrap_or("")
    }

    pub(crate) fn prefix_of(&self, uri: &str) -> Option<&str> {
        self.frames.iter().rev().find_map(|f| f.prefix_of(uri))
    }

    /// Привязан ли URI хоть чем-нибудь.
    pub(crate) fn binds(&self, uri: &str) -> bool {
        self.default_uri() == uri || self.prefix_of(uri).is_some()
    }

    /// Чем пользоваться для URI, если объявления текущего элемента —
    /// `frame`.
    ///
    /// Побеждает САМОЕ ВНУТРЕННЕЕ объявление, а внутри одного элемента —
    /// умолчательное перед префиксным. Правило снято с четырёх измеренных
    /// случаев сразу: элемент, объявивший умолчание и заодно префикс для
    /// того же URI, пишется без префикса (`<к xmlns="urn:test"
    /// xmlns:d1p1="urn:test" d1p1:qa="де">`), а элемент, которому
    /// умолчание досталось от родителя и который объявил префикс сам, —
    /// уже с префиксом (`<d2p1:nested xmlns:d2p1="urn:test"
    /// d2p1:qi="х">`), и его дети наследуют этот выбор (`<d2p1:in>`).
    pub(crate) fn resolve(&self, frame: &NsFrame, uri: &str) -> NsUse {
        if frame.default.as_deref() == Some(uri) {
            return NsUse::Default;
        }
        if let Some(p) = frame.prefix_of(uri) {
            return NsUse::Prefix(p.to_string());
        }
        for f in self.frames.iter().rev() {
            if f.default.as_deref() == Some(uri) {
                return NsUse::Default;
            }
            if let Some(p) = f.prefix_of(uri) {
                return NsUse::Prefix(p.to_string());
            }
        }
        NsUse::Default
    }
}

/// Имя, которым элемент или тип пишется в текущей области.
pub(crate) fn qualified(scope: &NsScope, frame: &NsFrame, uri: &str, local: &str) -> String {
    match scope.resolve(frame, uri) {
        NsUse::Default => local.to_string(),
        NsUse::Prefix(p) => format!("{p}:{local}"),
    }
}

/// Имя АТРИБУТА: умолчательное объявление на атрибуты не
/// распространяется, поэтому пространству имён обязателен префикс. Он
/// всегда есть — его завёл вызывающий, увидев атрибут с непустым URI
/// (измерено: `<к xmlns="urn:test" xmlns:d1p1="urn:test" … d1p1:qa="де">`
/// — элемент по умолчанию, атрибут того же пространства по префиксу).
///
/// # Errors
///
/// [`RtError::Xdto`], если префикса всё-таки нет: это значило бы, что
/// объявления и имена разошлись.
pub(crate) fn attribute_qname(
    scope: &NsScope,
    frame: &NsFrame,
    uri: &str,
    local: &str,
) -> RtResult<String> {
    if uri.is_empty() {
        return Ok(local.to_string());
    }
    match frame.prefix_of(uri).or_else(|| scope.prefix_of(uri)) {
        Some(prefix) => Ok(format!("{prefix}:{local}")),
        None => Err(RtError::Xdto(format!(
            "для атрибута «{local}» нет префикса пространства имён «{uri}»"
        ))),
    }
}

/// `ФабрикаXDTO.ЗаписатьXML(ЗаписьXML, Значение[, ИмяЭлемента[, УРИ]])`.
///
/// Аргументов не больше четырёх: пятый платформа отвергает (измерено —
/// справочные «ТипXML» и «ФормаXML» пятым и шестым она не берёт). Имя по
/// умолчанию — ИМЯ ТИПА значения, а не имя объявленного в схеме элемента:
/// `ЗаписатьXML(Зп, Объект)` для типа `RootType` даёт `<RootType …>`, а у
/// анонимного типа имя пусто и вызов без имени — ошибка (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика;
/// [`RtError::Xdto`], если аргументы не те, имя элемента пусто или содержит
/// двоеточие, у значения нет лексической формы либо экземпляр вложен глубже
/// `MAX_XDTO_DEPTH` (в том числе когда он ссылается сам на себя);
/// [`RtError::Xml`], если писатель уже закрыт.
pub fn factory_write_xml(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    factory_model(obj, "ЗаписатьXML")?;
    let (writer, value, name, uri) = match args {
        [writer, value] => (writer, value, None, None),
        [writer, value, name] => (writer, value, Some(name), None),
        [writer, value, name, uri] => (writer, value, Some(name), Some(uri)),
        _ => return Err(not_applicable(obj, "ЗаписатьXML")),
    };
    if !crate::xml::is_xml_writer(writer) {
        return Err(RtError::Xdto("ЗаписатьXML пишет в «ЗаписьXML»".to_string()));
    }
    let name = optional_text(name, "ЗаписатьXML")?;
    // Имя с двоеточием платформа отвергает целиком (измерено — проба `зп
    // имя с двоеточием`: `ЗаписатьXML(Зпис, ОбП, "t:мой")` даёт ошибку).
    // Записать его как есть значило бы выпустить документ с необъявленным
    // префиксом. Проверяется РОВНО двоеточие: остальные требования к имени
    // XML не измерены, и расширять запрет без замера нечем.
    if let Some(given) = &name
        && given.contains(':')
    {
        return Err(RtError::Xdto(format!(
            "имя элемента «{given}» содержит двоеточие, а префикс в имени \
                 ЗаписатьXML не объявляет"
        )));
    }
    let uri = optional_text(uri, "ЗаписатьXML")?;
    let (model, type_index, slot) = match repr_of(value) {
        Some(XdtoRepr::Object(data)) => (&data.model, data.type_index, Slot::Object(data)),
        Some(XdtoRepr::Value(model, data)) => (
            model,
            data.type_index,
            Slot::Simple {
                model,
                type_index: data.type_index,
                value: &data.value,
            },
        ),
        _ => return Err(bad_write_value(value)),
    };
    let type_data = model.type_at(type_index)?;
    let name = match name {
        Some(name) => name,
        None => type_data.name.clone(),
    };
    if name.is_empty() {
        return Err(RtError::Xdto(
            "у типа значения нет имени (тип анонимный), поэтому имя элемента \
             обязательно"
                .to_string(),
        ));
    }
    let uri = uri.unwrap_or_else(|| type_data.ns.clone());
    crate::xml::with_writer(crate::xml::arg_object(writer)?, |w| {
        let mut scope = NsScope::default();
        write_node(w, &mut scope, &name, &uri, None, &slot, 0)
    })
}

pub(crate) fn bad_write_value(value: &BslValue) -> RtError {
    RtError::Xdto(format!(
        "ЗаписатьXML пишет «ОбъектXDTO» или «ЗначениеXDTO», а не «{}»",
        value.type_name()
    ))
}

/// Необязательный строковый аргумент.
pub(crate) fn optional_text(
    arg: Option<&BslValue>,
    method: &'static str,
) -> RtResult<Option<String>> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Str(s)) => Ok(Some(s.to_string())),
        Some(other) => Err(RtError::Xdto(format!(
            "{method}: имя элемента и URI — строки, а не «{}»",
            other.type_name()
        ))),
    }
}

/// Записать один элемент вместе с содержимым.
///
/// `annotate` — тип, которым значение отличается от объявленного: он
/// уходит в `xsi:type`. У свойства ПРОСТОГО типа его не бывает никогда
/// (измерено: число в свойстве `xs:int` пишется просто `<num>42</num>`), а
/// у свойства типа объекта — как только фактический тип другой, включая
/// свойство типа `anyType`, где тип берётся у самого значения BSL.
///
/// `depth` — глубина спуска ПО ЭКЗЕМПЛЯРУ, не по документу: она считается
/// от начала записи, тогда как `w.depth()` знает и про элементы, открытые
/// вызывающим кодом до `ЗаписатьXML`.
pub(crate) fn write_node(
    w: &mut crate::core::XmlWriter,
    scope: &mut NsScope,
    name: &str,
    uri: &str,
    annotate: Annotation<'_>,
    slot: &Slot,
    depth: usize,
) -> RtResult<()> {
    // Спуск рекурсивный, а экземпляр — граф, а не дерево: свойство типа
    // объекта (в том числе `anyType`) вправе указывать на самого владельца.
    // Без предела такой цикл и честная цепочка на тысячи уровней одинаково
    // роняют стек процесса вместо перехватываемой ошибки, поэтому предел
    // тот же, что у разбора, и отдельного обнаружения циклов не заводится.
    if depth > MAX_XDTO_DEPTH {
        return Err(RtError::Xdto(format!(
            "экземпляр вложен глубже предела записи XDTO ({MAX_XDTO_DEPTH})"
        )));
    }
    let xml_depth = w.depth() + 1;
    let mut frame = NsFrame::default();
    let mut generated = 0usize;

    // Атрибуты собираются ДО начала тега: их пространства имён могут
    // потребовать префиксов, а те объявляются на этом же элементе.
    let attributes = attributes_of(slot)?;
    for (attr_uri, _, _) in &attributes {
        if attr_uri.is_empty() || scope.prefix_of(attr_uri).is_some() {
            continue;
        }
        if frame.prefix_of(attr_uri).is_none() {
            // Первый порождённый префикс элемента измерен на трёх
            // глубинах — `d1p1`, `d2p1`, `d3p1`. `НЕ ИЗМЕРЕНО(XDTO.IO.PREFIX)`:
            // как нумеруется ВТОРОЙ префикс на том же элементе. Чтобы
            // это увидеть, нужен тип с двумя квалифицированными
            // атрибутами из РАЗНЫХ пространств имён, а фабрика строится
            // из одного файла XSD, и межсхемной ссылки взять неоткуда.
            generated += 1;
            frame
                .prefixes
                .push((format!("d{xml_depth}p{generated}"), attr_uri.clone()));
        }
    }
    // Умолчательное объявление делается только там, где URI ещё не привязан
    // ничем: иначе элемент пользуется уже действующим объявлением.
    if !scope.binds(uri) {
        frame.default = Some(uri.to_string());
    }
    // `xs` нужен `xsi:type` встроенных типов, `xsi` — самим `xsi:type` и
    // `xsi:nil`. Платформа объявляет оба на элементе, с которого начинает
    // запись, и не объявляет `xs`, если пространство XML Schema и так
    // умолчательное (измерено).
    if !scope.binds(XSD_NS) && frame.default.as_deref() != Some(XSD_NS) {
        frame.prefixes.push(("xs".to_string(), XSD_NS.to_string()));
    }
    if !scope.binds(XSI_NS) && frame.default.as_deref() != Some(XSI_NS) {
        frame.prefixes.push(("xsi".to_string(), XSI_NS.to_string()));
    }

    let element_name = qualified(scope, &frame, uri, name);
    let type_name = match annotate {
        Some((model, index)) => {
            let data = model.type_at(index)?;
            Some(qualified(scope, &frame, &data.ns, &data.name))
        }
        None => None,
    };

    w.write_start_element(&element_name)?;
    if let Some(default) = &frame.default {
        w.write_attribute("xmlns", default)?;
    }
    for (prefix, prefix_uri) in &frame.prefixes {
        w.write_attribute(&format!("xmlns:{prefix}"), prefix_uri)?;
    }
    if matches!(slot, Slot::Nil) {
        w.write_attribute(&attribute_qname(scope, &frame, XSI_NS, "nil")?, "true")?;
    }
    if let Some(type_name) = type_name {
        w.write_attribute(&attribute_qname(scope, &frame, XSI_NS, "type")?, &type_name)?;
    }
    for (attr_uri, attr_name, lexical) in &attributes {
        let written = attribute_qname(scope, &frame, attr_uri, attr_name)?;
        w.write_attribute(&written, lexical)?;
    }

    scope.frames.push(frame);
    let result = write_content(w, scope, slot, depth);
    scope.frames.pop();
    result?;
    w.write_end_element()
}

/// Атрибуты элемента: только у экземпляра объекта и только заполненные, в
/// порядке модели типа.
pub(crate) fn attributes_of(slot: &Slot) -> RtResult<Vec<(String, String, String)>> {
    let Slot::Object(data) = slot else {
        return Ok(Vec::new());
    };
    let model = &data.model;
    let mut out = Vec::new();
    for &prop in &model.type_at(data.type_index)?.properties {
        let info = model.property_at(prop)?;
        if info.form != EnumValue::XmlFormAttribute {
            continue;
        }
        for value in occurrences_of(data, prop) {
            out.push((
                info.ns.clone(),
                info.name.clone(),
                lexical_for_write(model, info.type_index, &value)?,
            ));
        }
    }
    Ok(out)
}

/// Значения свойства в порядке заполнения.
pub(crate) fn occurrences_of(data: &Rc<XdtoObjectData>, prop: usize) -> Vec<BslValue> {
    data.entries
        .borrow()
        .iter()
        .filter(|e| e.prop == prop)
        .map(|e| e.value.clone())
        .collect()
}

/// Содержимое элемента: текст простого значения либо свойства объекта.
pub(crate) fn write_content(
    w: &mut crate::core::XmlWriter,
    scope: &mut NsScope,
    slot: &Slot,
    depth: usize,
) -> RtResult<()> {
    match slot {
        Slot::Nil => Ok(()),
        Slot::Simple {
            model,
            type_index,
            value,
        } => {
            let lexical = lexical_for_write(model, *type_index, value)?;
            // Пустая лексическая форма — это `<имя/>`, а не `<имя></имя>`
            // (измерено на пустой строке).
            if lexical.is_empty() {
                return Ok(());
            }
            w.write_text(&lexical)
        }
        Slot::Object(data) => write_properties(w, scope, data, depth),
    }
}

/// Свойства экземпляра: сначала текст простого содержимого, потом
/// элементы.
///
/// Порядок элементов выбирает [`XdtoTypeData::sequenced`], и ничто иное:
/// у НЕ последовательного типа он модельный (измерено: записанные как
/// `num`, потом `name`, свойства вышли как `name`, потом `num`), у
/// ПОСЛЕДОВАТЕЛЬНОГО — порядок заполнения (измерено на `xs:choice`: `cb`,
/// `ca`, `cb` вышли ровно так).
///
/// Развилка выглядит узкой, пока не вспомнить, что маска `xs:any` или
/// `xs:anyAttribute` делает тип ОТКРЫТЫМ, а открытый — последовательным.
/// Отсюда весь EnterpriseData: его 348 масок означают, что реальный обмен
/// пишется в порядке ЗАПОЛНЕНИЯ, а не схемы. Измерено на четырёх типах
/// одной формы, различающихся только масками
/// (`XDTO.WRITE_ORDER.CLOSED` — `[a][b]`, `XDTO.WRITE_ORDER.OPEN` —
/// `[b][a]` при одном и том же заполнении `b`, потом `a`), и подтверждено
/// на настоящей схеме EnterpriseData 1.0.1 (`XDTO.WRITE_ORDER.EDATA`).
///
/// Упорядочивает платформа именно при ЗАПИСИ, а не при установке:
/// `Последовательность()` открытого типа показывает порядок заполнения
/// (`XDTO.WRITE_ORDER.OPEN_SEQ` — `[b=бэ][a=а]`), поэтому [`set_single`]
/// трогать было не нужно. Повторное присваивание своё место сохраняет
/// (`XDTO.WRITE_ORDER.REASSIGN`), а вхождения множественного свойства
/// перемежаются с одиночными по месту `Добавить`
/// (`XDTO.WRITE_ORDER.MULTI` — `[c][c][a]`).
pub(crate) fn write_properties(
    w: &mut crate::core::XmlWriter,
    scope: &mut NsScope,
    data: &Rc<XdtoObjectData>,
    depth: usize,
) -> RtResult<()> {
    let model = &data.model;
    let type_data = model.type_at(data.type_index)?;
    let props = type_data.properties.clone();
    let sequenced = type_data.sequenced();
    for &prop in &props {
        if model.property_at(prop)?.form != EnumValue::XmlFormText {
            continue;
        }
        let info = model.property_at(prop)?;
        for value in occurrences_of(data, prop) {
            let lexical = lexical_for_write(model, info.type_index, &value)?;
            if !lexical.is_empty() {
                w.write_text(&lexical)?;
            }
        }
    }
    let plan: Vec<(usize, BslValue)> = if sequenced {
        let entries = data.entries.borrow();
        let mut out = Vec::with_capacity(entries.len());
        for entry in entries.iter() {
            if model.property_at(entry.prop)?.form == EnumValue::XmlFormElement {
                out.push((entry.prop, entry.value.clone()));
            }
        }
        out
    } else {
        let mut out = Vec::new();
        for &prop in &props {
            if model.property_at(prop)?.form != EnumValue::XmlFormElement {
                continue;
            }
            for value in occurrences_of(data, prop) {
                out.push((prop, value));
            }
        }
        out
    };
    for (prop, value) in plan {
        let info = model.property_at(prop)?;
        let (slot, annotate) = slot_for(model, info.type_index, &value)?;
        write_node(w, scope, &info.name, &info.ns, annotate, &slot, depth + 1)?;
    }
    Ok(())
}

/// Как записывать значение, лежащее в свойстве объявленного типа.
pub(crate) fn slot_for<'a>(
    model: &'a Rc<XdtoModel>,
    declared: usize,
    value: &'a BslValue,
) -> RtResult<(Slot<'a>, Annotation<'a>)> {
    if matches!(value, BslValue::Undefined) {
        return Ok((Slot::Nil, None));
    }
    if model.type_at(declared)?.is_value() {
        return Ok((
            Slot::Simple {
                model,
                type_index: declared,
                value,
            },
            None,
        ));
    }
    // Свойство типа ОБЪЕКТА: фактический тип значения виден в самом
    // значении, и если он не объявленный — платформа помечает элемент
    // `xsi:type` (измерено и на наследнике, и на каждом типе BSL в
    // свойстве `anyType`).
    let annotate = |index: usize| {
        if index == declared {
            None
        } else {
            Some((model, index))
        }
    };
    if let Some(o) = repr_of(value) {
        match o {
            XdtoRepr::Object(inner) => {
                let same_model = Rc::ptr_eq(&inner.model, model);
                let mark = if same_model && inner.type_index == declared {
                    None
                } else {
                    Some((&inner.model, inner.type_index))
                };
                return Ok((Slot::Object(inner), mark));
            }
            XdtoRepr::Value(value_model, data) => {
                return Ok((
                    Slot::Simple {
                        model: value_model,
                        type_index: data.type_index,
                        value: &data.value,
                    },
                    if Rc::ptr_eq(value_model, model) {
                        annotate(data.type_index)
                    } else {
                        Some((value_model, data.type_index))
                    },
                ));
            }
            _ => {}
        }
    }
    let index = builtin_index_for(model, value).ok_or_else(|| {
        RtError::Xdto(format!(
            "у значения «{}» нет лексической формы для записи в XML",
            value.type_name()
        ))
    })?;
    Ok((
        Slot::Simple {
            model,
            type_index: index,
            value,
        },
        annotate(index),
    ))
}

/// Встроенный тип XML Schema, которым платформа помечает значение BSL в
/// свойстве типа `anyType` (измерено поимённо: строка -> `xs:string`,
/// число -> `xs:decimal`, булево -> `xs:boolean`, дата -> `xs:dateTime`,
/// двоичные данные -> `xs:base64Binary`).
pub(crate) fn builtin_index_for(model: &Rc<XdtoModel>, value: &BslValue) -> Option<usize> {
    let name = match value {
        BslValue::Str(_) => "string",
        BslValue::Number(_) => "decimal",
        BslValue::Boolean(_) => "boolean",
        BslValue::Date(_) => "dateTime",
        BslValue::Object(o) => match &**o {
            BslObject::BinaryData(_) => "base64Binary",
            _ => return None,
        },
        _ => return None,
    };
    model.find(XSD_NS, name)
}

/// Лексическая форма значения для записи — обратная к
/// [`value_from_lexical`].
///
/// # Errors
///
/// [`RtError::Xdto`], если у значения такой формы нет (`Неопределено`,
/// `Null`, посторонний объект) либо модель повреждена.
pub(crate) fn lexical_for_write(
    model: &Rc<XdtoModel>,
    type_index: usize,
    value: &BslValue,
) -> RtResult<String> {
    let builtin = model.builtin_of(type_index);
    if let Some(expanded) = value
        .object_ref()
        .and_then(|object| object.downcast_ref::<crate::xsd::ExpandedNameObject>())
    {
        return Ok(expanded.name.local.clone());
    }
    if let BslValue::Object(o) = value {
        match &**o {
            BslObject::BinaryData(bytes) => {
                // `hexBinary` платформа пишет ЗАГЛАВНЫМИ и одной строкой, а
                // `base64Binary` — с переносами по 64 символа, разделёнными
                // CR LF (измерено на 48 и 49 байтах: 64 символа ровно
                // переноса не получают, 68 — получают).
                return Ok(match builtin {
                    Some(BuiltinBsl::Hex) => encode_hex(bytes),
                    _ => encode_base64(bytes),
                });
            }
            // Значение СПИСОЧНОГО простого типа — это массив (у платформы
            // фиксированный, см. «Двоичные лексические формы» в шапке
            // модуля).
            BslObject::Array(items) => {
                return join_list(model, type_index, &items.borrow());
            }
            _ => {}
        }
    }
    match lexical_of_value(value, builtin) {
        Some(lexical) => Ok(lexical),
        None => Err(RtError::Xdto(format!(
            "у значения «{}» нет лексической формы типа «{}»",
            value.type_name(),
            type_display(model.type_at(type_index)?)
        ))),
    }
}

/// Лексическая форма СПИСОЧНОГО простого типа: формы элементов через
/// пробел (измерено: свойство типа `xs:list` пишется как `1 2 3`).
pub(crate) fn join_list(
    model: &Rc<XdtoModel>,
    type_index: usize,
    items: &[BslValue],
) -> RtResult<String> {
    let item_type = list_item_of(model, type_index).unwrap_or(type_index);
    let mut out = String::new();
    for item in items {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&lexical_for_write(model, item_type, item)?);
    }
    Ok(out)
}

/// Тип элемента списочного простого типа — по цепочке базовых, как это
/// делает [`XdtoModel::builtin_of`].
pub(crate) fn list_item_of(model: &XdtoModel, index: usize) -> Option<usize> {
    let mut cur = index;
    for _ in 0..=model.types.len() {
        match model.types.get(cur)?.shape.as_ref()? {
            ValueShape::List(item) => return *item,
            ValueShape::Builtin(_) | ValueShape::Union(_) => return None,
            ValueShape::Atomic => cur = model.types.get(cur)?.base?,
        }
    }
    None
}

/// `hexBinary`: заглавные шестнадцатеричные цифры без переносов.
pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

/// Ширина строки base64 при записи — измерено на 48 и 49 байтах.
pub(crate) const BASE64_LINE: usize = 64;

/// `base64Binary`: стандартный алфавит с дополнением, строками по
/// [`BASE64_LINE`] символов через CR LF.
pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut chars = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let (b0, b1, b2) = (
            u32::from(chunk[0]),
            chunk.get(1).map_or(0, |b| u32::from(*b)),
            chunk.get(2).map_or(0, |b| u32::from(*b)),
        );
        let bits = (b0 << 16) | (b1 << 8) | b2;
        chars.push(ALPHABET[(bits >> 18) as usize & 63] as char);
        chars.push(ALPHABET[(bits >> 12) as usize & 63] as char);
        chars.push(if chunk.len() > 1 {
            ALPHABET[(bits >> 6) as usize & 63] as char
        } else {
            '='
        });
        chars.push(if chunk.len() > 2 {
            ALPHABET[bits as usize & 63] as char
        } else {
            '='
        });
    }
    let mut out = String::with_capacity(chars.len() + chars.len() / BASE64_LINE * 2);
    for (i, chunk) in chars
        .as_bytes()
        .chunks(BASE64_LINE)
        .map(String::from_utf8_lossy)
        .enumerate()
    {
        if i > 0 {
            out.push_str("\r\n");
        }
        out.push_str(&chunk);
    }
    out
}
