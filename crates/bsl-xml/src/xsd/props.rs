//! Свойства компонент схемы.

use super::*;

// --- свойства ------------------------------------------------------------

/// Свойство компоненты схемы, коллекции компонент, набора схем или
/// расширенного имени.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у этого вида
/// компоненты нет.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    let Some(object) = obj.object_ref() else {
        return Err(unknown());
    };
    if let Some(expanded) = object.downcast_ref::<ExpandedNameObject>() {
        let n = &expanded.name;
        if is("ЛокальноеИмя", "LocalName") {
            return Ok(str_value(&n.local));
        }
        if is("URIПространстваИмен", "NamespaceURI") {
            return Ok(str_value(&n.uri));
        }
        return Err(unknown());
    }
    if let Some(component) = object.downcast_ref::<ComponentObject>() {
        return component_property(&component.schema, component.index, name);
    }
    Err(unknown())
}

pub(crate) fn component_property(
    schema: &Rc<XsSchemaData>,
    index: usize,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);
    let node = schema.node(index);

    // Общее у всех компонент.
    if is("ТипКомпоненты", "ComponentType") {
        return Ok(BslValue::Enum(node.kind.component_type()));
    }
    if is("Схема", "Schema") {
        return Ok(component_value(schema, 0));
    }
    if is("Контейнер", "Container") {
        return Ok(match node.parent {
            Some(p) => component_value(schema, p),
            None => BslValue::Undefined,
        });
    }
    if is("Компоненты", "Components") {
        return Ok(list_value(schema, XsListKind::Fixed(node.children.clone())));
    }
    if is("ЭлементDOM", "DOMElement") {
        return Ok(match (&node.dom, &schema.dom_doc) {
            (Some(n), Some(doc)) => crate::dom::node_value(n, doc),
            _ => BslValue::Undefined,
        });
    }
    if is("Аннотация", "Annotation") {
        return Ok(annotation_of(schema, index));
    }

    match &node.data {
        XsData::Schema(d) => schema_property(schema, node, d, name),
        XsData::Element(d) | XsData::Attribute(d) => declaration_property(
            schema,
            node,
            d,
            name,
            matches!(node.data, XsData::Element(_)),
        ),
        XsData::SimpleType(d) => simple_type_property(schema, node, d, name),
        XsData::ComplexType(d) => complex_type_property(schema, node, d, name),
        XsData::Particle {
            term,
            min_occurs,
            max_occurs,
        } => {
            if is("Часть", "Term") {
                return Ok(component_value(schema, *term));
            }
            // Границы вхождения — тоже ЛЕКСИЧЕСКИЕ: платформа отдаёт то,
            // что написано в самой частице, и `Неопределено`, когда
            // атрибута нет. Измерено на одной и той же схеме: у
            // безатрибутного `<xs:sequence>` обе границы пусты (проба `час
            // МинимальноВходит`), а у `<xs:element minOccurs="0"
            // maxOccurs="unbounded">` — 0 и строка `unbounded` (проба
            // `час0 …`).
            if is("МинимальноВходит", "MinOccurs") {
                return Ok(occurs_value(*min_occurs, false));
            }
            if is("МаксимальноВходит", "MaxOccurs") {
                return Ok(occurs_value(*max_occurs, true));
            }
            Err(unknown())
        }
        XsData::ModelGroup {
            compositor,
            particles,
        } => {
            if is("ВидГруппы", "GroupKind") {
                return Ok(BslValue::Enum(*compositor));
            }
            if is("Фрагменты", "Particles") {
                return Ok(list_value(schema, XsListKind::Plain(particles.clone())));
            }
            Err(unknown())
        }
        XsData::AttributeUse(d) => {
            if is("ОбъявлениеАтрибута", "AttributeDeclaration") {
                return Ok(component_value(schema, d.declaration));
            }
            if is("Обязательный", "Required") {
                return Ok(BslValue::Boolean(d.required));
            }
            // `Использование` платформа не заполняет (измерено даже при
            // `use="required"`), а обязательность отдаёт `Обязательный`.
            if is("Использование", "Use") {
                return Ok(BslValue::Undefined);
            }
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&d.lexical));
            }
            if is("Ограничение", "Constraint") {
                return Ok(opt_enum(d.constraint));
            }
            if is("Значение", "Value") {
                return Ok(typed_value(&d.lexical, d.constraint));
            }
            Err(unknown())
        }
        XsData::Facet(d) => {
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&d.lexical));
            }
            if is("Значение", "Value") {
                return Ok(facet_value(node.kind, &d.lexical));
            }
            if is("Фиксированный", "Fixed") {
                // Только у числовых фасетов — см. `FacetKind::is_numeric`.
                return match node.kind {
                    XsKind::Facet(f) if f.is_numeric() => Ok(opt_bool(d.fixed)),
                    _ => Err(unknown()),
                };
            }
            Err(unknown())
        }
        XsData::Annotation {
            documentation,
            appinfo,
        } => {
            if is("Документация", "Documentation") {
                return Ok(list_value(schema, XsListKind::Fixed(documentation.clone())));
            }
            if is("ИнформацияДляПриложения", "AppInfo") {
                return Ok(list_value(schema, XsListKind::Fixed(appinfo.clone())));
            }
            Err(unknown())
        }
        XsData::Documentation { lang, source } => {
            if is("Язык", "Language") {
                return Ok(str_value(lang));
            }
            if is("Источник", "Source") {
                return Ok(str_value(source));
            }
            Err(unknown())
        }
    }
}

/// Аннотация компоненты — первая среди её детей.
pub(crate) fn annotation_of(schema: &Rc<XsSchemaData>, index: usize) -> BslValue {
    schema
        .node(index)
        .children
        .iter()
        .find(|c| schema.node(**c).kind == XsKind::Annotation)
        .map_or(BslValue::Undefined, |c| component_value(schema, *c))
}

pub(crate) fn schema_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &SchemaData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    if is("ПространствоИмен", "TargetNamespace") {
        return Ok(str_value(&node.ns));
    }
    if is("Версия", "Version") {
        return Ok(str_value(&d.version));
    }
    if is("РасположениеСхемы", "SchemaLocation") {
        return Ok(str_value(&d.location));
    }
    if is("ФормаЭлементовПоУмолчанию", "ElementFormDefault") {
        return Ok(opt_enum(d.element_form));
    }
    if is("ФормаАтрибутовПоУмолчанию", "AttributeFormDefault") {
        return Ok(opt_enum(d.attribute_form));
    }
    // Блокировку и завершённость платформа отдаёт пустыми и там, где они
    // записаны; заводить для них поле незачем.
    if is("БлокировкаПоУмолчанию", "BlockDefault") || is("ЗавершенностьПоУмолчанию", "FinalDefault")
    {
        return Ok(BslValue::Undefined);
    }
    if is("ОбъявленияЭлементов", "ElementDeclarations") {
        return Ok(list_value(schema, XsListKind::Named(d.elements.clone())));
    }
    if is("ОбъявленияАтрибутов", "AttributeDeclarations") {
        return Ok(list_value(schema, XsListKind::Named(d.attributes.clone())));
    }
    if is("ОпределенияТипов", "TypeDefinitions") {
        return Ok(list_value(schema, XsListKind::Named(d.types.clone())));
    }
    // Групп, нотаций и ограничений идентичности эта модель не строит —
    // конструкции за её границей отвергаются разбором, — но коллекции
    // существуют и пусты.
    if is("ОпределенияГруппМоделей", "ModelGroupDefinitions")
        || is("ОпределенияГруппАтрибутов", "AttributeGroupDefinitions")
        || is("ОбъявленияНотаций", "NotationDeclarations")
        || is(
            "ОпределенияОграниченийИдентичности",
            "IdentityConstraintDefinitions",
        )
    {
        return Ok(list_value(schema, XsListKind::Named(Vec::new())));
    }
    if is("Директивы", "Directives") {
        return Ok(list_value(schema, XsListKind::Plain(Vec::new())));
    }
    Err(unknown())
}

pub(crate) fn declaration_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &DeclData,
    name: &str,
    element: bool,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяТипа", "TypeName") {
        return Ok(opt_name(d.type_name.as_ref()));
    }
    if is("АнонимноеОпределениеТипа", "AnonymousTypeDefinition") {
        return Ok(match d.anonymous_type {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    if is("Ссылка", "Reference") {
        return Ok(opt_name(d.reference.as_ref()));
    }
    if is("Форма", "Form") {
        return Ok(opt_enum(d.form));
    }
    if is("ЛексическоеЗначение", "LexicalValue") {
        return Ok(str_value(&d.lexical));
    }
    if is("Ограничение", "Constraint") {
        return Ok(opt_enum(d.constraint));
    }
    if is("Значение", "Value") {
        return Ok(typed_value(&d.lexical, d.constraint));
    }
    if is("ЭтоГлобальноеОбъявление", "IsGlobalDeclaration") {
        return Ok(BslValue::Boolean(d.global));
    }
    if is("Блокировка", "Block") || is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    if element && is("Абстрактный", "Abstract") {
        return Ok(opt_bool(d.is_abstract));
    }
    Err(unknown())
}

pub(crate) fn simple_type_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &SimpleTypeData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяБазовогоТипа", "BaseTypeName") {
        return Ok(opt_name(d.base_name.as_ref()));
    }
    if is("ОпределениеБазовогоТипа", "BaseTypeDefinition") {
        return Ok(BslValue::Undefined);
    }
    if is("Вариант", "Variety") {
        return Ok(opt_enum(d.variety));
    }
    if is("Фасеты", "Facets") {
        return Ok(list_value(schema, XsListKind::Plain(d.facets.clone())));
    }
    if is("ИмяТипаЭлемента", "ItemTypeName") {
        return Ok(opt_name(d.item_type_name.as_ref()));
    }
    if is("ОпределениеТипаЭлемента", "ItemTypeDefinition") {
        return Ok(match d.item_type {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    if is("ИменаТиповОбъединения", "MemberTypeNames") {
        return Ok(BslValue::new_object(NameListObject {
            names: Rc::new(d.member_type_names.clone()),
        }));
    }
    if is("ОпределенияТиповОбъединения", "MemberTypeDefinitions") {
        return Ok(list_value(schema, XsListKind::Plain(Vec::new())));
    }
    if is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    Err(unknown())
}

pub(crate) fn complex_type_property(
    schema: &Rc<XsSchemaData>,
    node: &XsNode,
    d: &ComplexTypeData,
    name: &str,
) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);

    if is("Имя", "Name") {
        return Ok(str_value(&node.name));
    }
    if is("URIПространстваИмен", "NamespaceURI") {
        return Ok(str_value(&node.ns));
    }
    if is("ИмяБазовогоТипа", "BaseTypeName") {
        return Ok(opt_name(d.base_name.as_ref()));
    }
    if is("ОпределениеБазовогоТипа", "BaseTypeDefinition") {
        return Ok(BslValue::Undefined);
    }
    if is("МетодНаследования", "DerivationMethod") {
        return Ok(opt_enum(d.derivation));
    }
    if is("Содержимое", "Content") {
        return Ok(match d.content {
            Some(i) => component_value(schema, i),
            None => BslValue::Undefined,
        });
    }
    // Модель содержимого и маска атрибутов — вычисляемые свойства, которых
    // платформа на этом этапе не заполняет.
    if is("МодельСодержимого", "ContentModel") || is("МаскаАтрибутов", "AttributeWildcard")
    {
        return Ok(BslValue::Undefined);
    }
    if is("Атрибуты", "Attributes") {
        return Ok(list_value(schema, XsListKind::Plain(d.attributes.clone())));
    }
    if is("Смешанный", "Mixed") {
        return Ok(opt_bool(d.mixed));
    }
    if is("Абстрактный", "Abstract") {
        return Ok(opt_bool(d.is_abstract));
    }
    if is("Блокировка", "Block") || is("Завершенность", "Final") {
        return Ok(BslValue::Undefined);
    }
    Err(unknown())
}

/// `Значение` объявления: платформа угадывает тип по САМОЙ записи
/// `default`/`fixed`, а объявленный `type` при этом не смотрит вовсе.
///
/// Измерено на 8.3.27 одной пробой из 28 объявлений (строка
/// `XSD.TYPED_VALUE` в `tests/conformance/measure/platform.tsv`). Порядок
/// ровно такой:
///
/// 1. `true`/`false` без учёта регистра и `1`/`0` целиком дают `Булево`, и
///    объявленный тип тут ни при чём: `xs:string` со значением `1` — это
///    `Булево` (`Да`), а `xs:int` со значением `0` — `Булево` (`Нет`);
/// 2. иначе берётся наибольший ЧИСЛОВОЙ ПРЕФИКС записи, и если в нём есть
///    хоть одна цифра — выходит `Число`: `xs:date` с `2021-07-05` даёт
///    `2021`, `xs:time` с `08:30:00` — `8`, `xs:double` с `1.5e3` — `1,5`
///    (показатель степени не разбирается), `xs:hexBinary` с `0A0B` — `0`,
///    `xs:string` с `1,5` — `1` (запятая разделителем не считается);
/// 3. всё остальное остаётся строкой, как записано: `AQI=` при
///    `xs:base64Binary`, `urn:x` при `xs:anyURI`.
///
/// Из-за этого `type_name` здесь и не нужен — параметра у функции нет.
pub(crate) fn typed_value(lexical: &str, constraint: Option<EnumValue>) -> BslValue {
    if constraint.is_none() {
        // Значения нет вовсе: `ЛексическоеЗначение` пусто, а `Значение` —
        // `Неопределено` (измерено на объявлении без `default`/`fixed`).
        return BslValue::Undefined;
    }
    // Пробелы по краям платформа игнорирует: `xs:string` со значением
    // « 7 » (пробелы сохраняет сама схема) отдан числом `7`.
    let text = lexical.trim();
    if folded_eq(text, "true") || text == "1" {
        return BslValue::Boolean(true);
    }
    if folded_eq(text, "false") || text == "0" {
        return BslValue::Boolean(false);
    }
    match numeric_prefix(text) {
        Some(prefix) => match BslNumber::parse_canonical(prefix) {
            Ok(n) => BslValue::Number(n),
            // Префикс состоит из знака, цифр и не более чем одной точки,
            // поэтому разбор отказывает только на переполнении разрядности;
            // такое значение остаётся текстом, как записано.
            Err(_) => str_value(lexical),
        },
        None => str_value(lexical),
    }
}

/// Наибольший числовой префикс записи: знак, цифры и не более одной точки.
///
/// `None`, когда цифр в префиксе нет вовсе, — тогда значение остаётся
/// строкой. Измеренные границы: `+3` -> `3`, `.5` -> `0,5`, `1.2.3` -> `1,2`
/// (вторая точка обрывает префикс), `1,5` -> `1` (запятая обрывает),
/// `2021-07-05` -> `2021`, `1.5e3` -> `1,5`.
pub(crate) fn numeric_prefix(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let mut digits = 0;
    let mut dot = false;
    while end < bytes.len() {
        match bytes[end] {
            b'0'..=b'9' => digits += 1,
            b'.' if !dot => dot = true,
            _ => break,
        }
        end += 1;
    }
    if digits == 0 {
        return None;
    }
    Some(&text[..end])
}

/// `Значение` фасета: у строковых фасетов это сама строка, у `whiteSpace` —
/// член перечисления, у числовых — число. Расхождение с платформой описано
/// в заголовке модуля: там числовые фасеты отдают неинициализированное
/// поле.
pub(crate) fn facet_value(kind: XsKind, lexical: &str) -> BslValue {
    let XsKind::Facet(facet) = kind else {
        return str_value(lexical);
    };
    match facet {
        FacetKind::WhiteSpace => match lexical {
            "preserve" => BslValue::Enum(EnumValue::XsWhitespacePreserve),
            "replace" => BslValue::Enum(EnumValue::XsWhitespaceReplace),
            "collapse" => BslValue::Enum(EnumValue::XsWhitespaceCollapse),
            _ => str_value(lexical),
        },
        f if f.is_numeric() => match BslNumber::parse_canonical(lexical) {
            Ok(n) => BslValue::Number(n),
            Err(_) => str_value(lexical),
        },
        _ => str_value(lexical),
    }
}
