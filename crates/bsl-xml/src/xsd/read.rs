//! Чтение модели изнутри крейта.

use super::*;

// --- чтение модели изнутри крейта ----------------------------------------

/// Объявление элемента или атрибута — ровно те поля, которые нужны
/// строителю модели типов XDTO.
pub struct DeclView<'a> {
    pub name: &'a str,
    pub ns: &'a str,
    pub type_name: Option<&'a XName>,
    pub anonymous_type: Option<usize>,
    /// `default`/`fixed` как написано; пусто, если ограничения нет.
    pub lexical: &'a str,
    pub has_constraint: bool,
}

/// Использование атрибута: обязательность и значение по умолчанию живут
/// здесь, а имя и тип — в объявлении внутри.
pub struct AttributeUseView<'a> {
    pub declaration: usize,
    pub required: bool,
    pub lexical: &'a str,
    pub has_constraint: bool,
}

/// Читающая поверхность разобранной схемы для других модулей крейта.
///
/// Наружу крейта модель схемы видна только значениями BSL, и снаружи так и
/// должно быть. Но строить поверх `get_property` модель типов XDTO значило
/// бы разбирать собственный вывод обратно, поэтому здесь заведены прямые —
/// и только читающие — доступы. Ни один из них не меняет схему, поэтому
/// инварианты разбора (см. заголовок модуля) остаются в силе.
impl XsSchemaData {
    /// Целевое пространство имён схемы.
    pub fn target_namespace(&self) -> &str {
        self.target_ns()
    }

    pub fn kind_of(&self, i: usize) -> XsKind {
        self.node(i).kind
    }

    pub fn name_of(&self, i: usize) -> &str {
        &self.node(i).name
    }

    /// Номера ИМЕНОВАННЫХ глобальных определений типов, как их отдаёт
    /// `ОпределенияТипов`: отсортированные по имени, без дублей.
    pub fn global_types(&self) -> &[usize] {
        match &self.nodes[0].data {
            XsData::Schema(d) => &d.types,
            _ => &[],
        }
    }

    /// Имя базового типа простого типа — лексически, как написано в
    /// `xs:restriction base`.
    pub fn simple_base_of(&self, i: usize) -> Option<&XName> {
        match &self.node(i).data {
            XsData::SimpleType(d) => d.base_name.as_ref(),
            _ => None,
        }
    }

    /// Вариант простого типа (`Атомарная`/`Список`/`Объединение`) вместе с
    /// именем типа элемента списка и именами членов объединения.
    pub fn simple_variety_of(&self, i: usize) -> Option<(EnumValue, Option<&XName>, &[XName])> {
        match &self.node(i).data {
            XsData::SimpleType(d) => Some((
                d.variety?,
                d.item_type_name.as_ref(),
                d.member_type_names.as_slice(),
            )),
            _ => None,
        }
    }

    /// Фасеты простого типа — вид и лексическая запись значения, в порядке
    /// документа.
    pub fn facets_of(&self, i: usize) -> Vec<(FacetKind, &str)> {
        let XsData::SimpleType(d) = &self.node(i).data else {
            return Vec::new();
        };
        d.facets
            .iter()
            .filter_map(|f| match (self.node(*f).kind, &self.node(*f).data) {
                (XsKind::Facet(kind), XsData::Facet(data)) => Some((kind, data.lexical.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Имя базового типа составного типа (`extension`/`restriction`).
    pub fn complex_base_of(&self, i: usize) -> Option<&XName> {
        match &self.node(i).data {
            XsData::ComplexType(d) => d.base_name.as_ref(),
            _ => None,
        }
    }

    /// `mixed`/`abstract` составного типа; неуказанный атрибут — «нет».
    pub fn complex_flags_of(&self, i: usize) -> (bool, bool) {
        match &self.node(i).data {
            XsData::ComplexType(d) => (d.mixed.unwrap_or(false), d.is_abstract.unwrap_or(false)),
            _ => (false, false),
        }
    }

    /// Есть ли в объявлении составного типа маска `xs:any` или
    /// `xs:anyAttribute`. Наследование сюда не входит: это СОБСТВЕННОЕ
    /// объявление типа, включая тело `xs:extension`/`xs:restriction`.
    pub fn complex_has_wildcard(&self, i: usize) -> bool {
        match &self.node(i).data {
            XsData::ComplexType(d) => d.has_wildcard,
            _ => false,
        }
    }

    /// Фрагмент содержимого составного типа, если он есть.
    pub fn complex_content_of(&self, i: usize) -> Option<usize> {
        match &self.node(i).data {
            XsData::ComplexType(d) => d.content,
            _ => None,
        }
    }

    /// Использования атрибутов составного типа — в порядке документа.
    pub fn complex_attribute_uses_of(&self, i: usize) -> &[usize] {
        match &self.node(i).data {
            XsData::ComplexType(d) => &d.attributes,
            _ => &[],
        }
    }

    /// Фрагмент: терм и границы вхождения так, как они написаны.
    pub fn particle_of(&self, i: usize) -> Option<(usize, Option<u32>, Option<u32>)> {
        match &self.node(i).data {
            XsData::Particle {
                term,
                min_occurs,
                max_occurs,
            } => Some((*term, *min_occurs, *max_occurs)),
            _ => None,
        }
    }

    /// Группа модели: вид композитора и её фрагменты.
    pub fn model_group_of(&self, i: usize) -> Option<(EnumValue, &[usize])> {
        match &self.node(i).data {
            XsData::ModelGroup {
                compositor,
                particles,
            } => Some((*compositor, particles.as_slice())),
            _ => None,
        }
    }

    /// Объявление элемента или атрибута.
    pub fn decl_of(&self, i: usize) -> Option<DeclView<'_>> {
        let node = self.node(i);
        let (XsData::Element(d) | XsData::Attribute(d)) = &node.data else {
            return None;
        };
        Some(DeclView {
            name: &node.name,
            ns: &node.ns,
            type_name: d.type_name.as_ref(),
            anonymous_type: d.anonymous_type,
            lexical: &d.lexical,
            has_constraint: d.constraint.is_some(),
        })
    }

    /// Использование атрибута.
    pub fn attribute_use_of(&self, i: usize) -> Option<AttributeUseView<'_>> {
        match &self.node(i).data {
            XsData::AttributeUse(d) => Some(AttributeUseView {
                declaration: d.declaration,
                required: d.required,
                lexical: &d.lexical,
                has_constraint: d.constraint.is_some(),
            }),
            _ => None,
        }
    }
}

/// Схема из текста XSD — тем же путём, каким её строит BSL-код: дерево
/// строит `crate::dom`, а разбирает уже готовое дерево этот модуль.
/// Второго разборщика схем в проекте нет, поэтому этой же дорогой ходит
/// `СоздатьФабрикуXDTO`, прочитав файл (см. модуль `xdto` компонента `bsl-xml`).
///
/// # Errors
///
/// Всё, чем отвечает [`create_schema`], плюс ошибка разбора XML.
pub fn schema_of_text(text: &str) -> RtResult<Rc<XsSchemaData>> {
    let mut state = crate::xml::XmlReaderState::over(crate::core::XmlParser::new(text));
    let doc = crate::dom::build_tree(&mut state)?;
    let value = crate::dom::node_value(&doc, &doc);
    match as_component(&create_schema(&new_builder(), &[value])?) {
        Some((schema, 0)) => Ok(schema),
        _ => Err(RtError::Xsd("корень дерева — не схема".to_string())),
    }
}
