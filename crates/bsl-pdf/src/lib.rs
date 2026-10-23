//! Документы PDF BSL: «ДокументPDF», его страницы и вложения.
//!
//! Писательское ядро формата — модуль [`writer`]; поверх него крейт добавляет
//! читатель контейнера и измеренную на 8.3.27 поверхность встроенного
//! языка.

mod document;
pub mod writer;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDescriptor,
    RtResult, TypeDescriptor,
};

pub use document::{
    AttachmentObject, AttachmentsObject, DocumentObject, PageObject, PagesObject,
    new_pdf_attachments, new_pdf_document,
};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn construct_document(
    context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_pdf_document(context.files_rc()?))
}

fn construct_attachments(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_pdf_attachments())
}

// Конструкторы документа и коллекции без аргументов: измерено, что и путь, и
// `ДвоичныеДанные` в `Новый ДокументPDF` платформа отвергает, а `Новый
// ВложениеPDF` не существует вовсе — вложение появляется через `Добавить`.
const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ДокументPDF", "PDFDocument"],
        arity: Arity::exact(0),
        call: Some(construct_document),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["КоллекцияВложенийPDF", "PDFAttachmentCollection"],
        arity: Arity::exact(0),
        call: Some(construct_attachments),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &[
            "ОписаниеОтображаемогоОбъектаPDF",
            "PDFRepresentationObjectDescription",
        ],
        arity: Arity::exact(0),
        // Без свойств отображаемого объекта описание PDF не может участвовать
        // в построении документа.
        call: None,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["ЗаписьPDF", "PDFWriter"],
        arity: Arity::range(0, 1),
        // Низкоуровневый писатель не реализует методы и жизненный цикл
        // платформенного объекта `ЗаписьPDF`.
        call: None,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(5),
        names: &["ЧтениеPDF", "PDFReader"],
        arity: Arity::range(0, 1),
        // Читатель контейнера документа не реализует жизненный цикл `ЧтениеPDF`.
        // Без этой реализации успешное создание обещало бы недоступное чтение.
        call: None,
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::document::surface::ATTACHMENTS_TYPE,
    &crate::document::surface::ATTACHMENT_TYPE,
    &crate::document::surface::DOCUMENT_TYPE,
    &crate::document::surface::PAGES_TYPE,
    &crate::document::surface::PAGE_TYPE,
];
const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (
        ConstructorCode::new(1),
        &crate::document::surface::DOCUMENT_TYPE,
    ),
    (
        ConstructorCode::new(2),
        &crate::document::surface::ATTACHMENTS_TYPE,
    ),
];

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] =
    &[document::objects::API_MEMBERS];

/// Дескриптор статически подключаемого компонента документов PDF.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(PACKAGE_NAME, PACKAGE_VERSION)
        .with_constructors(CONSTRUCTORS)
        .with_constructor_types(CONSTRUCTOR_TYPES)
        .with_types(TYPES)
        .with_object_member_groups(OBJECT_MEMBER_GROUPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_codes_are_static_and_dense() {
        let codes = library()
            .constructors()
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(codes, (1..=5).collect::<Vec<_>>());
    }
}
