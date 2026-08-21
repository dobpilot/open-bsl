//! JSON: потоковое чтение и запись.
//!
//! Один разборщик на двоих: `ЧтениеJSON` отдаёт его события наружу по
//! одному, а `ПрочитатьJSON` собирает из тех же событий готовое значение.
//! Второй реализации разбора в проекте быть не должно — иначе потоковый и
//! целиком-режим разъедутся на первом же краевом случае.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё, что ниже, снято пробами (`tests/conformance/measure/measure-json.bsl`),
//! а не выведено из спецификации JSON — платформа местами от неё отходит:
//!
//! * экранируются только `"`, `\`, ПС и ВК; прямая косая НЕ экранируется,
//!   остальные символы до `0x20` уходят как `\uXXXX` ЗАГЛАВНЫМИ шестнадцатеричными
//!   (табуляция уходит как `\u0009`, а НЕ как `\t`);
//! * не-ASCII (кириллица) пишется как есть;
//! * `ЗаписатьЗначение` принимает ТОЛЬКО строку, число и булево: `Null`,
//!   `Неопределено` и `Дата` дают ошибку типа;
//! * числа пишутся точной десятичной записью с точкой (`1/3` уходит всеми
//!   27 знаками), без разделителей групп;
//! * разборщик СНИСХОДИТЕЛЕН к битому вводу: пропущенное значение,
//!   висячая запятая, отсутствующее двоеточие и незакрытый объект
//!   принимаются молча, ошибку даёт только мусор на месте самого значения.

mod bridge;
mod dates;
mod objects;
mod parse;
mod write;

pub use bridge::*;
pub use dates::*;
pub use objects::*;
pub use parse::*;
pub use write::*;

use bsl_rt::{
    Arity, ConstructorCode, ConstructorDescriptor, FunctionCode, FunctionDescriptor, FunctionKind,
    LibraryDescriptor, RtError,
};

/// Ошибка разбора. Текст платформы мы не воспроизводим (он привязан к её
/// номерам строк), поэтому своё сообщение.
fn bad(what: &str) -> RtError {
    RtError::Json(format!("некорректный JSON: {what}"))
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПрочитатьJSON", "ReadJSON"],
        arity: Arity::range(1, 8),
        kind: FunctionKind::Function,
        call: component_read_json,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ЗаписатьJSON", "WriteJSON"],
        arity: Arity::range(2, 6),
        kind: FunctionKind::Procedure,
        call: component_write_json,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ЗаписатьДатуJSON", "WriteJSONDate"],
        arity: Arity::range(2, 3),
        kind: FunctionKind::Function,
        call: component_write_json_date,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПрочитатьДатуJSON", "ReadJSONDate"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: component_read_json_date,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["ЗаписатьЗначениеJSON", "WriteJSONValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: component_write_json_value,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПрочитатьЗначениеJSON", "ReadJSONValue"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: component_read_json_value,
    },
];

const CONSTRUCTORS: &[ConstructorDescriptor] = &[
    ConstructorDescriptor {
        code: ConstructorCode::new(1),
        names: &["ЧтениеJSON", "JSONReader"],
        arity: Arity::exact(0),
        call: construct_reader,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(2),
        names: &["ЗаписьJSON", "JSONWriter"],
        arity: Arity::exact(0),
        call: construct_writer,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(3),
        names: &["ПараметрыЗаписиJSON", "JSONWriterSettings"],
        arity: Arity::range(0, 2),
        call: construct_writer_settings,
    },
    ConstructorDescriptor {
        code: ConstructorCode::new(4),
        names: &["НастройкиСериализацииJSON", "JSONSerializerSettings"],
        arity: Arity::exact(0),
        call: construct_serializer_settings,
    },
];

/// Дескриптор статически подключаемого JSON-компонента.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        // Ядро в зависимостях не объявляется: реестр включает его в
        // требования любой программы (`RuntimeRegistry::requirements_for`).
        dependencies: &[],
        functions: FUNCTIONS,
        constructors: CONSTRUCTORS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_component_codes_are_stable() {
        let descriptor = library();
        assert_eq!(
            descriptor
                .functions
                .iter()
                .map(|function| function.code.get())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert_eq!(
            descriptor
                .constructors
                .iter()
                .map(|constructor| constructor.code.get())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }
}
