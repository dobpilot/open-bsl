//! Табличный документ BSL: MXL, XLSX, шаблоны и PDF-раскладка.
//!
//! Писатель PDF приходит из `bsl_pdf::writer`, парсер XML — из
//! `bsl_xml::core`.

mod document;
mod pdf_layout;
mod template;
mod xlsx;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDescriptor,
    RtResult, TypeDescriptor,
};

pub use document::{
    Color, Font, HAlign, Line, LineStyle, Merge, NamedArea, SpreadDocData, SpreadDocumentObject,
    VAlign, apply_params, from_mxl_bytes, is_area, is_spread_document, new_document, output,
    read as spread_read, set_detail, set_value, take_params, to_mxl_bytes, write as spread_write,
    write_file as spread_write_file,
};
pub use template::from_template_xml;
pub use xlsx::to_xlsx_bytes;

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn construct_document(
    context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_document(context.files_rc()?))
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["ТабличныйДокумент", "SpreadsheetDocument"],
    arity: Arity::exact(0),
    call: construct_document,
}];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::document::objects::AREA_TYPE,
    &crate::document::objects::DOCUMENT_TYPE,
    &crate::document::objects::DRAWINGS_TYPE,
    &crate::document::objects::DRAWING_TYPE,
    &crate::document::objects::PARAMS_TYPE,
];

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] =
    &[document::objects::API_MEMBERS];

/// Дескриптор статически подключаемого компонента табличного документа.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(
        PACKAGE_NAME,
        PACKAGE_VERSION,
        bsl_rt::ObjectContextNeed::Reduced,
    )
    .with_constructors(CONSTRUCTORS)
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
        assert_eq!(codes, (1..=1).collect::<Vec<_>>());
    }
}
