//! Присваивание свойств узла.

use super::*;

// --- Присваивание свойств ------------------------------------------------

/// `Узел.Свойство = Значение`.
///
/// Пишутся `Значение`, `ЗначениеУзла`, `Данные` и `ТекстовоеСодержимое`;
/// `ИмяУзла` только читается (измерено). Присваивание `ЗначениеУзла`
/// элементу и `ТекстовоеСодержимое` документу платформа принимает и ничего
/// не делает — здесь так же.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства у узла этого вида нет;
/// [`RtError::TypeError`] при нестроковом значении и при попытке записать
/// `ИмяУзла`.
pub fn set_property(obj: &BslValue, name: &str, val: &BslValue) -> RtResult<()> {
    let (node, doc) = as_node(obj, "свойство узла DOM")?;
    let unknown = || RtError::UnknownColumn(name.to_string());
    let is = |ru: &str, en: &str| folded_eq(name, ru) || folded_eq(name, en);
    let text = |op: &'static str| match val {
        BslValue::Str(s) => Ok(s.to_string()),
        // ИЗМЕРЕНО: `А.Значение = 5` платформа отвергает — число здесь не
        // приводится к строке.
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    };

    if is("ИмяУзла", "NodeName") {
        return Err(RtError::TypeError {
            expected: "Свойство, доступное для записи",
            op: "ИмяУзла",
        });
    }
    if is("Значение", "Value") {
        if node.kind != DomKind::Attribute {
            return Err(unknown());
        }
        DomNode::set_text_children(&node, &text("Значение")?, &doc);
        return Ok(());
    }
    if is("ЗначениеУзла", "NodeValue") {
        return match node.kind {
            DomKind::Attribute => {
                DomNode::set_text_children(&node, &text("ЗначениеУзла")?, &doc);
                Ok(())
            }
            DomKind::Text | DomKind::CdataSection | DomKind::Comment => {
                *node.value.borrow_mut() = Some(text("ЗначениеУзла")?);
                Ok(())
            }
            // Элемент, документ и инструкция обработки: значения у них нет,
            // и присваивание платформа молча проглатывает (измерено на
            // элементе).
            DomKind::Element
            | DomKind::Document
            | DomKind::ProcessingInstruction
            | DomKind::EntityReference => Ok(()),
        };
    }
    if is("Данные", "Data") {
        return match node.kind {
            DomKind::Text
            | DomKind::CdataSection
            | DomKind::Comment
            | DomKind::ProcessingInstruction => {
                *node.value.borrow_mut() = Some(text("Данные")?);
                Ok(())
            }
            _ => Err(unknown()),
        };
    }
    if is("ТекстовоеСодержимое", "TextContent") {
        return match node.kind {
            // ИЗМЕРЕНО: у элемента заменяет ВСЕХ детей одним текстовым
            // узлом, а пустая строка оставляет его вовсе без детей.
            DomKind::Element | DomKind::Attribute => {
                DomNode::set_text_children(&node, &text("ТекстовоеСодержимое")?, &doc);
                Ok(())
            }
            DomKind::Text | DomKind::CdataSection | DomKind::Comment | DomKind::EntityReference => {
                *node.value.borrow_mut() = Some(text("ТекстовоеСодержимое")?);
                Ok(())
            }
            // У документа читается `Неопределено`, и присваивание ничего не
            // делает (измерено).
            DomKind::Document | DomKind::ProcessingInstruction => Ok(()),
        };
    }
    Err(unknown())
}
