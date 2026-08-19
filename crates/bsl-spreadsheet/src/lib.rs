//! Табличный документ BSL: MXL, XLSX, шаблоны и PDF-раскладка.
//!
//! Форматные ядра — писатель PDF и парсер XML — пока живут в `bsl-rt`
//! (`bsl_rt::pdf`, `bsl_rt::xml`) и переезжают следом.

mod document;
mod pdf_layout;
mod template;
mod xlsx;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDependency,
    LibraryDescriptor, RtResult,
};

pub use document::{
    apply_params, from_mxl_bytes, is_area, is_spread_document, new_document, output,
    read as spread_read, set_detail, set_value, take_params, to_mxl_bytes, write as spread_write,
    write_file as spread_write_file, Color, Font, HAlign, Line, LineStyle, Merge, NamedArea,
    SpreadDocData, SpreadDocumentObject, VAlign,
};
pub use template::from_template_xml;
pub use xlsx::to_xlsx_bytes;

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn construct_document(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_document())
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["ТабличныйДокумент", "SpreadsheetDocument"],
    arity: Arity::exact(0),
    call: construct_document,
}];

/// Дескриптор статически подключаемого компонента табличного документа.
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
        assert_eq!(codes, (1..=1).collect::<Vec<_>>());
    }
}
