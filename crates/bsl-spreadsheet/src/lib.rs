//! Табличный документ BSL: MXL, XLSX, шаблоны и PDF-раскладка.
//!
//! Писатель PDF приходит из `bsl_pdf::writer`, парсер XML — из
//! `bsl_xml::core`.

mod document;
mod graphical_schema;
mod pdf_layout;
mod template;
mod xlsx;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, LibraryDescriptor,
    RtResult, TypeDescriptor,
};

pub use document::{
    Color, Font, HAlign, Line, LineStyle, Merge, NamedArea, SpreadDocData, SpreadDocumentObject,
    VAlign, apply_params, from_document_data, from_mxl_bytes, is_area, is_spread_document,
    new_document, output, read as spread_read, set_detail, set_value, take_params, to_mxl_bytes,
    write as spread_write, write_file as spread_write_file,
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

// Декларация позволяет компилировать исходные менеджеры конфигурации.
// Выполнение явно сообщает об отсутствующей объектной модели диаграммы.
// Представление типа измерено на 1С 8.3.27.2342; оно отличается от имени.
static GEOGRAPHICAL_SCHEMA_TYPE: TypeDescriptor = TypeDescriptor {
    package: PACKAGE_NAME,
    name: "ГеографическаяСхема",
    type_display: "Поле географической схемы",
    type_names: &["GeographicalSchema"],
};

// Имена и представление типа измерены на 1С 8.3.27.2342.
static CHART_TYPE: TypeDescriptor = TypeDescriptor {
    package: PACKAGE_NAME,
    name: "Диаграмма",
    type_display: "Chart",
    type_names: &["Chart"],
};

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ТабличныйДокумент", "SpreadsheetDocument"],
        arity: Arity::exact(0),
        call: Some(construct_document),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["ГеографическаяСхема", "GeographicalSchema"],
        arity: Arity::exact(0),
        call: None,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ГрафическаяСхема", "GraphicalSchema"],
        arity: Arity::exact(0),
        call: Some(graphical_schema::construct),
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["Диаграмма", "Chart"],
        arity: Arity::exact(0),
        call: None,
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[
    &crate::document::objects::AREA_TYPE,
    &crate::document::objects::DOCUMENT_TYPE,
    &crate::document::objects::DRAWINGS_TYPE,
    &crate::document::objects::DRAWING_TYPE,
    &crate::document::objects::PARAMS_TYPE,
    &GEOGRAPHICAL_SCHEMA_TYPE,
    &graphical_schema::SCHEMA_TYPE,
    &graphical_schema::ITEMS_TYPE,
    &CHART_TYPE,
];
const CONSTRUCTOR_TYPES: &[(ConstructorCode, &TypeDescriptor)] = &[
    (
        ConstructorCode::new(1),
        &crate::document::objects::DOCUMENT_TYPE,
    ),
    (ConstructorCode::new(2), &GEOGRAPHICAL_SCHEMA_TYPE),
    (ConstructorCode::new(3), &graphical_schema::SCHEMA_TYPE),
    (ConstructorCode::new(4), &CHART_TYPE),
];

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] = &[
    document::objects::API_MEMBERS,
    graphical_schema::API_MEMBERS,
];

/// Дескриптор статически подключаемого компонента табличного документа.
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
        assert_eq!(codes, (1..=4).collect::<Vec<_>>());
    }
}
