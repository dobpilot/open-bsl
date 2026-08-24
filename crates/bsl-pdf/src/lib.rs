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

// Оба конструктора БЕЗ аргументов: измерено, что и путь, и
// `ДвоичныеДанные` в `Новый ДокументPDF` платформа отвергает, а `Новый
// ВложениеPDF` не существует вовсе — вложение появляется через `Добавить`.
const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ДокументPDF", "PDFDocument"],
        arity: Arity::exact(0),
        call: construct_document,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["КоллекцияВложенийPDF", "PDFAttachmentCollection"],
        arity: Arity::exact(0),
        call: construct_attachments,
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

/// Дескриптор статически подключаемого компонента документов PDF.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(
        PACKAGE_NAME,
        PACKAGE_VERSION,
        bsl_rt::ObjectJitPolicy::NativeContextCompatible,
    )
    .with_constructors(CONSTRUCTORS)
    .with_types(TYPES)
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
        assert_eq!(codes, (1..=2).collect::<Vec<_>>());
    }
}
