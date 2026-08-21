//! Сериализация дерева в XML.

use super::*;

// --- Сериализация --------------------------------------------------------

/// Объявление ли это пространства имён.
pub(crate) fn is_xmlns_attr(name: &str) -> bool {
    name == "xmlns" || prefix_of(name) == "xmlns"
}

/// Обход дерева, пишущий через `XmlWriter`.
///
/// Второго сериализатора нет: всё форматирование — отступ, поведение
/// закрывающего тега после текста, экранирование — приходит из уже
/// измеренного [`crate::core::XmlWriter`].
pub(crate) struct DomSerializer<'a> {
    pub(crate) w: &'a mut XmlWriter,
    /// На каждый открытый элемент — объявленные им пары «префикс, URI».
    /// Ищется с конца: внутреннее объявление того же префикса перекрывает
    /// внешнее.
    pub(crate) scopes: Vec<Vec<(String, String)>>,
}

impl DomSerializer<'_> {
    /// Связан ли префикс с этим URI в текущей области видимости.
    pub(crate) fn bound(&self, prefix: &str, uri: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            for (p, u) in scope.iter().rev() {
                if p == prefix {
                    return u == uri;
                }
            }
        }
        false
    }

    /// Объявления, которые надо напечатать у этого элемента.
    ///
    /// Собираются из двух источников (оба ИЗМЕРЕНЫ): из объявлений-атрибутов
    /// самого элемента и из URI, которые нужны элементу и его атрибутам.
    /// Лишнее не печатается: объявление, которое уже действует в области
    /// видимости с тем же URI, писатель опускает — ИЗМЕРЕНО, что элемент,
    /// созданный формой `(URI, Имя)` внутри родителя с тем же объявлением,
    /// выходит как `<к:цена/>`, хотя атрибут-объявление у него свой. Порядок
    /// — по имени атрибута: `xmlns` раньше `xmlns:а` раньше `xmlns:я`.
    pub(crate) fn declarations(&self, el: &Rc<DomNode>) -> RtResult<Vec<(String, String)>> {
        let mut decls: Vec<(String, String)> = Vec::new();
        let add = |prefix: &str, uri: &str, decls: &mut Vec<(String, String)>| {
            if !decls.iter().any(|(p, _)| p == prefix) {
                decls.push((prefix.to_string(), uri.to_string()));
            }
        };
        for a in el.attrs.borrow().iter() {
            if !is_xmlns_attr(&a.name) {
                continue;
            }
            // ИЗМЕРЕНО: объявление с пустым URI (какое получается из
            // `УстановитьАтрибут("xmlns:к", "urn:к")`) при записи — ошибка.
            if a.uri != XMLNS_URI {
                return Err(bad(
                    "объявление пространства имён без URI пространства имён объявлений",
                ));
            }
            let prefix = if a.name == "xmlns" {
                ""
            } else {
                local_of(&a.name)
            };
            let uri = a.attr_value();
            if self.bound(prefix, &uri) {
                continue;
            }
            add(prefix, &uri, &mut decls);
        }
        let mut needed: Vec<(String, String)> = vec![(el.prefix.clone(), el.uri.clone())];
        for a in el.attrs.borrow().iter() {
            // Атрибут без префикса пространства имён не имеет (измерено),
            // поэтому объявления не требует.
            if !is_xmlns_attr(&a.name) && !a.prefix.is_empty() {
                needed.push((a.prefix.clone(), a.uri.clone()));
            }
        }
        for (prefix, uri) in needed {
            if uri.is_empty() || self.bound(&prefix, &uri) {
                continue;
            }
            add(&prefix, &uri, &mut decls);
        }
        decls.sort_by_key(|(prefix, _)| decl_name(prefix));
        Ok(decls)
    }

    pub(crate) fn write_element(&mut self, el: &Rc<DomNode>) -> RtResult<()> {
        let decls = self.declarations(el)?;
        self.w.write_start_element(&el.name)?;
        for (prefix, uri) in &decls {
            self.w.write_attribute(&decl_name(prefix), uri)?;
        }
        for a in el.attrs.borrow().iter() {
            if !is_xmlns_attr(&a.name) {
                self.w.write_attribute(&a.name, &a.attr_value())?;
            }
        }
        self.scopes.push(decls);
        let kids = el.children.borrow().clone();
        for c in kids.iter() {
            self.write_node(c)?;
        }
        self.scopes.pop();
        self.w.write_end_element()
    }

    pub(crate) fn write_node(&mut self, node: &Rc<DomNode>) -> RtResult<()> {
        let value = node.value.borrow().clone().unwrap_or_default();
        match node.kind {
            DomKind::Document => {
                let kids = node.children.borrow().clone();
                for c in kids.iter() {
                    self.write_node(c)?;
                }
                Ok(())
            }
            DomKind::Element => self.write_element(node),
            DomKind::Text => self.w.write_text(&value),
            DomKind::CdataSection => {
                // ИЗМЕРЕНО: секцию CDATA вне элемента платформа отвергает —
                // ровно как текст, о котором судит сам писатель.
                if !self.w.in_element() {
                    return Err(bad("секция CDATA вне элемента"));
                }
                self.w.write_cdata(&value)
            }
            DomKind::Comment => self.w.write_comment(&value),
            DomKind::ProcessingInstruction => {
                self.w.write_processing_instruction(&node.name, &value)
            }
            // ИЗМЕРЕНО: ссылка возвращается в текст такой же ссылкой —
            // дерево от `<к>раз&е;два</к>` писатель отдаёт байт в байт.
            DomKind::EntityReference => self.w.write_entity_reference(&node.name),
            // ИЗМЕРЕНО: атрибут не пишется вовсе — результат пустой, и это
            // не ошибка.
            DomKind::Attribute => Ok(()),
        }
    }
}

/// Имя атрибута-объявления для префикса.
pub(crate) fn decl_name(prefix: &str) -> String {
    if prefix.is_empty() {
        "xmlns".to_string()
    } else {
        format!("xmlns:{prefix}")
    }
}

/// `ЗаписьDOM.Записать(Узел, ЗаписьXML)`.
///
/// ДОКУМЕНТ пишется вместе с объявлением XML, и объявление берётся из
/// настроек ПИСАТЕЛЯ, а не из `ВерсияXML` документа (измерено: документ
/// версии 1.1 всё равно даёт `version="1.0"`). Любой другой узел пишется без
/// объявления.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЗаписьDOM` или
/// второй аргумент не `ЗаписьXML`; [`RtError::TypeError`] при неверной
/// арности или если первый аргумент не узел DOM; [`RtError::Xml`], если
/// приёмник писателя не задан, дерево не укладывается в XML (второй корень,
/// текстоподобный узел вне элемента) или несёт объявление пространства имён
/// с пустым URI.
pub fn write(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let op = "ЗаписьDOM.Записать";
    if !is_dom_writer(obj) {
        return Err(RtError::MethodNotApplicable {
            method: "Записать",
            receiver: obj.type_name(),
        });
    }
    if args.len() != 2 {
        return Err(RtError::TypeError {
            expected: "ровно два аргумента — узел DOM и ЗаписьXML",
            op,
        });
    }
    let node = need_node(args.first(), op)?;
    let target = &args[1];
    crate::xml::with_writer(crate::xml::arg_object(target)?, |w| {
        let mut ser = DomSerializer {
            w,
            scopes: Vec::new(),
        };
        if node.kind == DomKind::Document {
            ser.w.write_declaration()?;
        }
        ser.write_node(&node)
    })?;
    Ok(BslValue::Undefined)
}
