//! Каноническая запись существующего DOM без изменения дерева.
//!
//! Обычная `ЗаписьDOM` сохраняет форматирование XML-писателя. Здесь нужны
//! измеренные правила канонизации: отдельные закрывающие теги, сортировка
//! пространств имён и атрибутов, отсутствие декларации и особое экранирование.

use super::{DomKind, DomNode};
use bsl_rt::{RtError, RtResult};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::rc::Rc;

#[cfg(test)]
pub(crate) fn render_document(
    document: &Rc<DomNode>,
    exclusive: bool,
    comments: bool,
    prefixes: &str,
) -> RtResult<String> {
    render_selection(document, exclusive, comments, prefixes, None)
}

#[cfg(test)]
pub(crate) fn render_selection(
    document: &Rc<DomNode>,
    exclusive: bool,
    comments: bool,
    prefixes: &str,
    nodes: Option<&[Rc<DomNode>]>,
) -> RtResult<String> {
    render_version(document, exclusive, comments, prefixes, nodes, false)
}

pub(crate) fn render_version(
    document: &Rc<DomNode>,
    exclusive: bool,
    comments: bool,
    prefixes: &str,
    nodes: Option<&[Rc<DomNode>]>,
    version_11: bool,
) -> RtResult<String> {
    let writer = CanonicalWriter {
        version_11,
        exclusive,
        comments,
        included: prefixes
            .split_whitespace()
            .map(|p| if p == "#default" { "" } else { p }.to_owned())
            .collect(),
        selected: nodes.map(|nodes| nodes.iter().map(Rc::as_ptr).collect()),
        namespaces: nodes
            .unwrap_or_default()
            .iter()
            .filter(|node| node.uri == super::XMLNS_URI)
            .filter_map(|node| {
                node.parent
                    .borrow()
                    .upgrade()
                    .map(|owner| (Rc::as_ptr(&owner), node.local.clone()))
            })
            .collect(),
    };
    let mut output = String::new();
    if document.kind != DomKind::Document {
        let mut inherited = BTreeMap::new();
        let mut current = document.parent.borrow().upgrade();
        while let Some(parent) = current {
            for attr in parent
                .attrs
                .borrow()
                .iter()
                .filter(|attr| attr.uri == super::XMLNS_URI)
            {
                let prefix = if attr.name == "xmlns" {
                    ""
                } else {
                    &attr.local
                };
                inherited
                    .entry(prefix.to_owned())
                    .or_insert_with(|| attr.attr_value());
            }
            current = parent.parent.borrow().upgrade();
        }
        // Отдельный legacy-элемент переносит используемые объявления
        // предков; собственные объявления обрабатывает общий обход.
        inherited.retain(|prefix, _| {
            prefix == &document.prefix
                || document
                    .attrs
                    .borrow()
                    .iter()
                    .any(|attr| !attr.prefix.is_empty() && &attr.prefix == prefix)
        });
        writer.write_node(document, &inherited, &BTreeMap::new(), &mut output)?;
        return Ok(output);
    }
    let mut after_root = false;
    for node in document.children.borrow().iter() {
        let outside = writer.contains(node)
            && (node.kind == DomKind::ProcessingInstruction
                || (comments && node.kind == DomKind::Comment));
        if outside && after_root {
            output.push('\n');
        }
        writer.write_node(node, &BTreeMap::new(), &BTreeMap::new(), &mut output)?;
        if outside && !after_root {
            output.push('\n');
        }
        if node.kind == DomKind::Element {
            after_root = true;
        }
    }
    Ok(output)
}

struct CanonicalWriter {
    version_11: bool,
    exclusive: bool,
    comments: bool,
    included: BTreeSet<String>,
    selected: Option<HashSet<*const DomNode>>,
    namespaces: HashSet<(*const DomNode, String)>,
}

impl CanonicalWriter {
    fn contains(&self, node: &Rc<DomNode>) -> bool {
        self.selected
            .as_ref()
            .is_none_or(|selected| selected.contains(&Rc::as_ptr(node)))
    }
    fn effective_base(&self, node: &Rc<DomNode>) -> Option<String> {
        let mut values = Vec::new();
        let mut current = Some(node.clone());
        while let Some(element) = current {
            if !Rc::ptr_eq(&element, node) && self.contains(&element) {
                break;
            }
            if let Some(attr) = element
                .attrs
                .borrow()
                .iter()
                .find(|attr| attr.prefix == "xml" && attr.local == "base")
            {
                values.push(attr.attr_value());
            }
            current = element.parent.borrow().upgrade();
        }
        let mut values = values.into_iter().rev();
        let mut base = values.next()?;
        for reference in values {
            base = resolve_base(&base, &reference);
        }
        Some(base)
    }

    fn write_node(
        &self,
        node: &Rc<DomNode>,
        inherited: &BTreeMap<String, String>,
        rendered: &BTreeMap<String, String>,
        output: &mut String,
    ) -> RtResult<()> {
        let selected = self.contains(node);
        if !selected
            && !matches!(
                node.kind,
                DomKind::Element | DomKind::Document | DomKind::EntityReference
            )
        {
            return Ok(());
        }
        match node.kind {
            DomKind::Element => {
                let mut scope = inherited.clone();
                let attributes = node.attrs.borrow();
                for attr in attributes
                    .iter()
                    .filter(|attr| attr.uri == super::XMLNS_URI)
                {
                    let prefix = if attr.name == "xmlns" {
                        ""
                    } else {
                        &attr.local
                    };
                    let uri = attr.attr_value();
                    if !uri.is_empty() && !uri.contains(':') {
                        return Err(RtError::DynamicError(
                            "относительный URI пространства имён при канонизации XML".into(),
                        ));
                    }
                    scope.insert(prefix.to_owned(), uri);
                }
                let mut next_rendered = rendered.clone();
                let mut visible: BTreeSet<&str> =
                    self.included.iter().map(String::as_str).collect();
                visible.insert(&node.prefix);
                for attr in attributes
                    .iter()
                    .filter(|attr| attr.uri != super::XMLNS_URI && !attr.prefix.is_empty())
                {
                    visible.insert(&attr.prefix);
                }
                if !selected {
                    for child in node.children.borrow().iter() {
                        self.write_node(child, &scope, rendered, output)?;
                    }
                    return Ok(());
                }
                output.push('<');
                output.push_str(&node.name);
                for (prefix, uri) in &scope {
                    if (self.selected.is_some()
                        && !self
                            .namespaces
                            .contains(&(Rc::as_ptr(node), prefix.clone())))
                        || prefix == "xml"
                        || (self.exclusive && !visible.contains(prefix.as_str()))
                    {
                        continue;
                    }
                    if rendered.get(prefix).map(String::as_str).unwrap_or("") == uri {
                        continue;
                    }
                    output.push_str(" xmlns");
                    if !prefix.is_empty() {
                        output.push(':');
                        output.push_str(prefix);
                    }
                    output.push_str("=\"");
                    escape(uri, true, output);
                    output.push('"');
                    next_rendered.insert(prefix.clone(), uri.clone());
                }
                let mut ordinary: Vec<_> = attributes
                    .iter()
                    .filter(|attr| attr.uri != super::XMLNS_URI && self.contains(attr))
                    .cloned()
                    .collect();
                // Включающая канонизация наследует XML-атрибуты пропущенных
                // предков; исключающая оставляет только выбранные атрибуты.
                if self.selected.is_some() && !self.exclusive {
                    let mut names: BTreeSet<_> =
                        attributes.iter().map(|attr| attr.name.clone()).collect();
                    let mut ancestor = node.parent.borrow().upgrade();
                    while let Some(parent) = ancestor {
                        if self.contains(&parent) {
                            break;
                        }
                        for attr in parent
                            .attrs
                            .borrow()
                            .iter()
                            .filter(|attr| attr.prefix == "xml")
                        {
                            if !(self.version_11 && attr.local == "id")
                                && names.insert(attr.name.clone())
                            {
                                ordinary.push(attr.clone());
                            }
                        }
                        ancestor = parent.parent.borrow().upgrade();
                    }
                }
                ordinary.sort_by(|left, right| {
                    (attribute_uri(left), &left.local).cmp(&(attribute_uri(right), &right.local))
                });
                for attr in ordinary {
                    output.push(' ');
                    output.push_str(&attr.name);
                    output.push_str("=\"");
                    let value = if self.version_11
                        && !self.exclusive
                        && self.selected.is_some()
                        && attr.prefix == "xml"
                        && attr.local == "base"
                    {
                        self.effective_base(node)
                            .unwrap_or_else(|| attr.attr_value())
                    } else {
                        attr.attr_value()
                    };
                    escape(&value, true, output);
                    output.push('"');
                }
                output.push('>');
                for child in node.children.borrow().iter() {
                    self.write_node(child, &scope, &next_rendered, output)?;
                }
                output.push_str("</");
                output.push_str(&node.name);
                output.push('>');
            }
            DomKind::Text | DomKind::CdataSection => {
                escape(node.value.borrow().as_deref().unwrap_or(""), false, output)
            }
            DomKind::Comment if self.comments => {
                output.push_str("<!--");
                output.push_str(
                    &node
                        .value
                        .borrow()
                        .as_deref()
                        .unwrap_or("")
                        .replace('\r', "&#xD;"),
                );
                output.push_str("-->");
            }
            DomKind::ProcessingInstruction => {
                output.push_str("<?");
                output.push_str(&node.name);
                let value = node.value.borrow();
                if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                    output.push(' ');
                    output.push_str(&value.replace('\r', "&#xD;"));
                }
                output.push_str("?>");
            }
            DomKind::Document | DomKind::EntityReference => {
                for child in node.children.borrow().iter() {
                    self.write_node(child, inherited, rendered, output)?;
                }
            }
            DomKind::Comment | DomKind::DocumentType | DomKind::Attribute => {}
        }
        Ok(())
    }
}

// Зарезервированный префикс xml не требует явного объявления в DOM.
fn attribute_uri(node: &DomNode) -> &str {
    if node.prefix == "xml" {
        "http://www.w3.org/XML/1998/namespace"
    } else {
        &node.uri
    }
}

fn resolve_base(base: &str, reference: &str) -> String {
    if reference
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|part| part.contains(':'))
    {
        return reference.to_owned();
    }
    let base_without_fragment = base.split('#').next().unwrap_or(base);
    if reference.is_empty() {
        return base_without_fragment.to_owned();
    }
    if reference.starts_with('#') {
        return format!("{base_without_fragment}{reference}");
    }
    let base_path = base_without_fragment
        .split('?')
        .next()
        .unwrap_or(base_without_fragment);
    if reference.starts_with('?') {
        return format!("{base_path}{reference}");
    }
    let origin_end = base_path.find("://").map(|at| {
        base_path[at + 3..]
            .find('/')
            .map_or(base_path.len(), |slash| at + 3 + slash)
    });
    let origin_end = origin_end.or_else(|| base_path.find(':').map(|at| at + 1));
    let origin = origin_end.map_or("", |at| &base_path[..at]);
    if reference.starts_with("//") {
        return base_path.split_once(':').map_or_else(
            || reference.to_owned(),
            |(scheme, _)| format!("{scheme}:{reference}"),
        );
    }
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let path = origin_end.map_or(base_path, |at| &base_path[at..]);
        let directory = path
            .rfind('/')
            .map_or(if origin.contains("://") { "/" } else { "" }, |at| {
                &path[..=at]
            });
        format!("{directory}{reference}")
    };
    let suffix_at = joined.find(['?', '#']).unwrap_or(joined.len());
    let (path, suffix) = joined.split_at(suffix_at);
    let absolute = path.starts_with('/');
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.last().is_some_and(|segment| *segment != "..") {
                    segments.pop();
                } else if !absolute {
                    segments.push("..");
                }
            }
            other => segments.push(other),
        }
    }
    let mut normalized = if absolute {
        "/".to_owned()
    } else {
        String::new()
    };
    normalized.push_str(&segments.join("/"));
    if (path.ends_with('/') || path.ends_with("/.") || path.ends_with("/.."))
        && !normalized.ends_with('/')
    {
        normalized.push('/');
    }
    format!("{origin}{normalized}{suffix}")
}

fn escape(text: &str, attribute: bool, output: &mut String) {
    for character in text.chars() {
        output.push_str(match character {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' if !attribute => "&gt;",
            '"' if attribute => "&quot;",
            '\t' if attribute => "&#x9;",
            '\n' if attribute => "&#xA;",
            '\r' => "&#xD;",
            other => {
                output.push(other);
                continue;
            }
        });
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod tests;
