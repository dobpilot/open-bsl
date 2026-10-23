//! Общее дерево поверх существующего событийного XML-парсера.
use bsl_config::ConfigurationProperty;
use bsl_xml::core::{XmlEvent, XmlParser};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct Node {
    pub name: String,
    pub uri: String,
    pub text: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
fn key(name: &str) -> String {
    let name = local(name);
    if name == "valueType" {
        return "type".into();
    }
    if name == "MainAttribute" {
        return "main".into();
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

impl Node {
    pub fn child(&self, name: &str) -> Option<&Self> {
        self.children
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    pub fn value(&self, name: &str) -> Option<&str> {
        self.child(name)
            .map(|node| node.text.trim())
            .or_else(|| self.attr(name))
    }
    pub fn find_uuid(&self, id: &str) -> Option<&Self> {
        if self
            .attr("uuid")
            .is_some_and(|known| known.eq_ignore_ascii_case(id))
        {
            return Some(self);
        }
        self.children.iter().find_map(|node| node.find_uuid(id))
    }
    pub fn property(&self) -> ConfigurationProperty {
        let mut children: Vec<_> = if let Some(properties) = self.child("Properties") {
            let mut children: Vec<_> = properties.children.iter().map(Self::property).collect();
            if let Some(members) = self.child("ChildObjects") {
                children.extend(members.children.iter().map(Self::property));
            }
            children
        } else {
            self.children.iter().map(Self::property).collect()
        };
        for (name, value) in &self.attrs {
            if name.starts_with("xmlns") {
                continue;
            }
            let name = key(name);
            let canonical_value: Arc<str> = if name == "uuid" {
                value.to_ascii_lowercase().into()
            } else {
                value.as_str().into()
            };
            if !children.iter().any(|node| node.name.as_ref() == name) {
                children.push(ConfigurationProperty {
                    name: name.into(),
                    value: canonical_value,
                    attributes: Arc::from([]),
                    children: Arc::from([]),
                });
            }
        }
        let property_name = key(&self.name);
        // Нативная модель EDT использует `precision`/`scale` вместо
        // `Digits`/`FractionDigits` XML. Соответствие задано API EDT 2025.2.
        if property_name == "numberQualifiers" {
            for child in &mut children {
                match child.name.as_ref() {
                    "precision" => child.name = "digits".into(),
                    "scale" => child.name = "fractionDigits".into(),
                    "nonNegative" => {
                        child.name = "allowedSign".into();
                        child.value = match child.value.trim() {
                            "true" => "Nonnegative".into(),
                            "false" => "Any".into(),
                            _ => Arc::clone(&child.value),
                        };
                    }
                    _ => {}
                }
            }
        } else if property_name == "stringQualifiers" {
            for child in &mut children {
                if child.name.as_ref() == "fixed" {
                    child.name = "allowedLength".into();
                    child.value = match child.value.trim() {
                        "true" => "Fixed".into(),
                        "false" => "Variable".into(),
                        _ => Arc::clone(&child.value),
                    };
                }
            }
        }
        // XML содержит обёртку коллекции, а EDT — повторяемые элементы
        // с именем коллекции. Обёртка группируется только её родителем.
        let children = if matches!(
            property_name.as_str(),
            "attributes" | "tabularSections" | "commands" | "dimensions" | "resources"
        ) && !children.iter().any(|p| p.name.as_ref() == "name")
        {
            children
        } else {
            group(children)
        };
        ConfigurationProperty {
            name: property_name.into(),
            value: self.text.as_str().into(),
            attributes: self
                .attrs
                .iter()
                .filter(|(key, _)| !key.starts_with("xmlns"))
                .map(|(name, value)| (key(name).into(), value.as_str().into()))
                .collect(),
            children: children.into(),
        }
    }
    pub fn description(&self) -> ConfigurationProperty {
        if let Some(properties) = self.child("Properties") {
            let mut result = properties.property();
            result.name = Arc::from("description");
            if let Some(children) = self.child("ChildObjects") {
                let mut all = result.children.to_vec();
                all.extend(children.children.iter().map(Self::property));
                result.children = group(all).into();
            }
            result
        } else {
            let mut result = self.property();
            result.name = Arc::from("description");
            result
        }
    }
}

pub(super) fn parse(text: &str) -> Result<Node, String> {
    let mut parser = XmlParser::new(text);
    let mut stack = Vec::<Node>::new();
    let mut root = None;
    while let Some(event) = parser.read().map_err(|error| error.to_string())? {
        match event {
            XmlEvent::ElementStart { name, uri, attrs } => stack.push(Node {
                name: local(&name).into(),
                uri,
                text: String::new(),
                attrs: attrs
                    .iter()
                    .map(|attr| (local(&attr.name).into(), attr.value.clone()))
                    .collect(),
                children: Vec::new(),
            }),
            XmlEvent::Text(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&text);
                }
            }
            XmlEvent::ElementEnd { .. } => {
                let node = stack.pop().ok_or("закрывающий элемент без начала")?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            XmlEvent::EntityReference { name } => {
                return Err(format!("неразвёрнутая сущность описания «{name}»"));
            }
            _ => {}
        }
    }
    root.ok_or_else(|| "пустое описание XML".into())
}

fn group(children: Vec<ConfigurationProperty>) -> Vec<ConfigurationProperty> {
    let mut result = Vec::<ConfigurationProperty>::new();
    for mut child in children {
        let collection = match child.name.as_ref() {
            "attribute" | "attributes" => Some(("attributes", "attribute")),
            "tabularSection" | "tabularSections" => Some(("tabularSections", "tabularSection")),
            "command" | "commands" => Some(("commands", "command")),
            "dimension" | "dimensions" => Some(("dimensions", "dimension")),
            "resource" | "resources" => Some(("resources", "resource")),
            _ => None,
        };
        let Some((collection, item)) = collection else {
            result.push(child);
            continue;
        };
        let items = if child.name.as_ref() == collection && child.child("name").is_none() {
            child.children.to_vec()
        } else {
            child.name = item.into();
            vec![child]
        };
        if let Some(existing) = result
            .iter_mut()
            .find(|node| node.name.as_ref() == collection)
        {
            let mut all = existing.children.to_vec();
            all.extend(items);
            existing.children = all.into();
        } else {
            result.push(ConfigurationProperty {
                name: collection.into(),
                value: "".into(),
                attributes: Arc::from([]),
                children: items.into(),
            });
        }
    }
    result
}
