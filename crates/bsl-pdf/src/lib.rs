//! Документы PDF BSL: «ДокументPDF», его страницы и вложения.
//!
//! Писательское ядро формата — модуль [`writer`]; поверх него крейт добавляет
//! читатель контейнера и измеренную на 8.3.27 поверхность встроенного
//! языка.

mod document;
pub mod writer;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDependency,
    LibraryDescriptor, RtResult,
};

pub use document::{
    new_pdf_attachments, new_pdf_document, AttachmentObject, AttachmentsObject, DocumentObject,
    PageObject, PagesObject,
};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn construct_document(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_pdf_document())
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

/// Дескриптор статически подключаемого компонента документов PDF.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: PACKAGE_NAME,
        version: PACKAGE_VERSION,
        dependencies: &[LibraryDependency {
            package: bsl_rt::PACKAGE_NAME,
            version: bsl_rt::PACKAGE_VERSION,
        }],
        functions: &[],
        constructors: CONSTRUCTORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_codes_are_static_and_dense() {
        let codes = library()
            .constructors
            .iter()
            .map(|constructor| constructor.code.get())
            .collect::<Vec<_>>();
        assert_eq!(codes, (1..=2).collect::<Vec<_>>());
    }
}
