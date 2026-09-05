//! Рантайм-слой: значения, арифметика/сравнение, коллекции (`Массив`,
//! `Структура`, `Соответствие`, `ТаблицаЗначений`), строки UTF-16
//! (`Str`/`BslString`), даты (`Date`/`BslDate`, эпоха — `0001-01-01`, см.
//! модуль `date`) и типы как значения (`Type`/`TypeId`). `BslValue` растёт
//! по мере готовности остальных слоёв, а не заранее под все типы из брифа.

pub mod background_jobs;
mod bindata;
mod builtin;
mod component;
mod date;
pub mod encoding;
mod enums;
mod env;
mod error;
mod error_info;
mod fill;
mod fixed_array;
pub mod fold;
mod host_error;
mod http;
mod interner;
mod job_dto;
mod locale;
mod map;
mod metadata;
mod object;
mod object_protocol;
pub mod open_questions;
mod promise;
mod runtime_shapes;
mod shape;
mod string;
mod table;
mod temp_storage;
mod type_description;
mod types;
mod tz;
mod user_message;
pub mod uuid;
mod value;
mod value_graph;
mod value_list;
mod vstr;
pub use background_jobs::{BackgroundJobService, JobWaitOutcome};
pub use error::{ComponentError, RtError, RtResult};
pub use host_error::{HostError, HostErrorCode};
pub use job_dto::{JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, UserMessageDto};
pub use temp_storage::{
    GlobalStagingBudget, StagedWrite, StagingBudget, TempMailbox, TempStorageHub,
    TempStorageSession,
};
pub use value::BslValue;
pub use value_graph::{GraphBudget, GraphLimits, SerializedValueGraph};

pub use bsl_number::BslNumber;

/// Cargo-идентичность базового runtime-компонента. Её использует манифест
/// байт-кода; строка берётся из манифеста самого крейта, а не дублируется в
/// компиляторе.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use builtin::{
    BUILTIN_FN_NAMES, BUILTIN_METHOD_NAMES, BuiltinFn, BuiltinMethod, HostEffect, call_builtin_env,
    call_builtin_files, call_builtin_fn, call_builtin_fn_ctx, call_builtin_method,
    call_builtin_method_ctx, call_builtin_method_files, call_builtin_temp_file,
};
pub use component::{
    Arity, ByteStreamFactory, CallContext, CallOutcome, Capability, ComponentCall, ConstructorCode,
    ConstructorDescriptor, ContextKind, ExecutionParts, FunctionCode, FunctionDescriptor,
    FunctionKind, InterpreterServices, LibraryDependency, LibraryDescriptor, LibraryKey,
    LibraryRequirement, MethodCall, MethodDescriptor, ObjectContextNeed, ObjectMembersDescriptor,
    PendingHostCall, PropertyDescriptor, PropertyGet, PropertySet, RegistryError, RuntimeBuilder,
    RuntimeRegistry, SuspendingMethodCall, call_method_from_table, core_library,
    get_property_from_table, set_property_from_table,
};
pub use date::{
    BslDate, DEFAULT_PATTERN as DEFAULT_DATE_PATTERN, UNIX_EPOCH_SECONDS,
    format_long as format_date_long, format_pattern as format_date_pattern,
    local_date_from_utc_seconds, pseudo_unix_seconds,
};
pub use enums::{EnumKind, EnumValue, lookup_enum, lookup_member};
pub use env::{
    Clock, DirEntry, FileCreate, FileHandle, FileMetadata, FileOpenOptions, FileSystem,
    FixedTimeZone, HostEnv, MAX_OFFSET_SECONDS, MIN_TRANSITION_GAP_SECONDS, RandomHandle,
    RandomSource, SystemClock, SystemFileSystem, SystemRandom, TimeZone, UserMessageSink,
};
pub use error_info::{detailed_error_description, new_error_info};
pub use fold::folded_eq;
pub use http::{
    ClientIdentity, HttpClient, HttpClientConfig, HttpClientFactory, HttpCompletionSink,
    HttpErrorMapper, HttpPromiseSpawner, HttpResponseMapper, HttpWireRequest, HttpWireResponse,
    NetworkError, NetworkErrorKind, ProxyConfig, ProxyMode, RequestHandle, SecretBytes,
    SecretString, TlsConfig,
};
pub use interner::{NameId, NameInterner, first_folded_duplicate};
pub use locale::{Locale, NBSP};
pub use object::{BslObject, StructureStorage};
pub use object_protocol::{
    ByteStreamProtocol, ObjectDowncast, ObjectProtocol, ObjectRef, TypeDescriptor, receiver_of,
};
pub use promise::{ExecutionToken, PROMISE_TYPE, PromiseId, PromiseValue};
pub use runtime_shapes::RuntimeShapes;
pub use shape::{MAX_SHAPE_TRANSITIONS, Shape, ShapeTable};
pub use string::BslString;
pub use table::ValueTableData;
pub use types::{TypeId, TypeRef};
pub use tz::SystemTimeZone;
// Модель типов XDTO наружу крейта нужна целиком: строит её фабрика,
// которой в этой реализации ещё нет, а до тех пор единственный её
// потребитель — собственные тесты модуля.

/// Предвыделяет ёмкость под НЕДОСТОВЕРНОЕ число из входного формата, не
/// роняя процесс, и возвращает то же число, зажатое в `[0, bound]`.
///
/// `declared` — заявленное входом количество, ЗНАКОВОЕ: счётчики MXL
/// приходят из `Node::number()` как `i64`, и отрицательное при `as usize`
/// стало бы огромным ещё до всякой проверки. `bound` — сколько элементов
/// реально осталось во входе (байтов у `ЗначениеИзСтрокиВнутр`, узлов у
/// MXL). Ограничение входом убирает абсурдные числа, `try_reserve` —
/// оставшиеся: законный, но большой `bound` иначе всё равно завершил бы
/// процесс на аллокации (`Vec::with_capacity` при отказе делает `abort`,
/// а не ловимое `Попыткой` исключение — это и есть воспроизведение 5).
///
/// # Errors
///
/// [`std::collections::TryReserveError`], если аллокация не удалась;
/// вызывающий превращает её в свою ошибку формата.
pub fn reserve_hint<T>(
    vec: &mut Vec<T>,
    declared: i64,
    bound: usize,
) -> Result<usize, std::collections::TryReserveError> {
    let hint = declared.clamp(0, i64::try_from(bound).unwrap_or(i64::MAX)) as usize;
    vec.try_reserve(hint)?;
    Ok(hint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};
    use std::rc::Rc;

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    /// Имена членов в языке регистронезависимы, и кириллица здесь не
    /// исключение: `eq_ignore_ascii_case` сворачивала только ASCII, из-за
    /// чего `КЗ.значение` строчными не находило `Значение`. Корпус
    /// фикстур этого не ловил: имя приходит сюда в написании ПЕРВОГО
    /// вхождения в программе, а фикстуры пишут его канонично.
    #[test]
    fn native_property_names_fold_cyrillic_in_both_directions() {
        let pair = BslValue::Object(Rc::new(BslObject::KeyValuePair(
            BslValue::Str(BslString::from_str("к")),
            BslValue::Str(BslString::from_str("з")),
        )));
        for name in ["Значение", "значение", "ЗНАЧЕНИЕ", "Value", "value"] {
            assert_eq!(
                pair.get_field_by_name(name).expect(name),
                BslValue::Str(BslString::from_str("з")),
                "член «{name}»"
            );
        }
        for name in ["Ключ", "ключ", "КЛЮЧ"] {
            assert_eq!(
                pair.get_field_by_name(name).expect(name),
                BslValue::Str(BslString::from_str("к")),
                "член «{name}»"
            );
        }
        assert!(pair.get_field_by_name("НетТакого").is_err());
    }

    /// Условие приводится, и правило измерено, а не выведено. Здесь стоял
    /// обратный тест — `Если 1 Тогда` считалось ошибкой, — и платформа с
    /// ним разошлась: единица там истина, ноль ложь. Замеры `COND.*` и
    /// `TERNARY.CONDITION_*`.
    #[test]
    fn condition_converts_numbers_and_boolean_words() {
        assert!(num("1").as_condition().expect("единица — истина"));
        assert!(!num("0").as_condition().expect("ноль — ложь"));
        assert!(num("-1").as_condition().expect("минус единица — истина"));
        assert!(num("0.5").as_condition().expect("дробь — истина"));

        // Строка принимается ТОЛЬКО словами, регистр не важен, пробелы по
        // краям обрезаются.
        for (word, want) in [
            ("Истина", true),
            ("истина", true),
            ("ИСТИНА", true),
            (" Истина ", true),
            ("Да", true),
            ("True", true),
            ("Ложь", false),
            ("нет", false),
            ("False", false),
        ] {
            let v = BslValue::Str(BslString::from_str(word));
            assert_eq!(v.as_condition().expect(word), want, "{word}");
        }

        // «Непустая строка истинна» — НЕ то правило: цифры и мусор
        // отвергаются наравне с пустой строкой.
        for word in ["абв", "", "0", "1", "yes", "no"] {
            let v = BslValue::Str(BslString::from_str(word));
            assert!(v.as_condition().is_err(), "{word} обязано быть ошибкой");
        }

        // Всё, что не булево, не число и не строка, — ошибка.
        for v in [BslValue::Undefined, BslValue::Null] {
            assert!(v.as_condition().is_err());
        }
    }

    #[test]
    fn equality_by_value_across_representations() {
        assert!(num("1.0").eq_value(&num("1.00")));
    }

    /// `ЗаписьТекста.Закрыть()` не теряет буфер при отказе сброса. Файл,
    /// открытый ТОЛЬКО НА ЧТЕНИЕ, заворачивается в `BufWriter`: маленькая
    /// запись остаётся в памяти, а `flush` на закрытии падает. Прежде
    /// `take()` снимал писатель ДО `flush`, и второй `Закрыть()` находил
    /// `None` и врал успехом при незаписанном тексте.
    #[test]
    fn text_writer_close_keeps_the_buffer_when_flush_fails() {
        use std::io::Write as _;
        let path = std::env::temp_dir().join("open-bsl-text-writer-close-fail.txt");
        std::fs::File::create(&path).expect("создать файл");
        let read_only = std::fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .expect("открыть на чтение");
        let mut buffered = std::io::BufWriter::new(Box::new(read_only) as Box<dyn FileHandle>);
        buffered
            .write_all("незаписанный текст".as_bytes())
            .expect("в буфер");
        let writer = BslValue::Object(Rc::new(BslObject::TextWriter(std::cell::RefCell::new(
            Some(buffered),
        ))));
        assert!(
            matches!(writer.text_writer_close(), Err(RtError::IoError(_))),
            "сброс в файл только на чтение обязан упасть"
        );
        assert!(
            matches!(writer.text_writer_close(), Err(RtError::IoError(_))),
            "повторный Закрыть() снова падает — буфер не потерян"
        );

        let ok_path = std::env::temp_dir().join("open-bsl-text-writer-close-ok.txt");
        let ok_file = std::fs::File::create(&ok_path).expect("создать файл");
        let writer_ok = BslValue::Object(Rc::new(BslObject::TextWriter(std::cell::RefCell::new(
            Some(std::io::BufWriter::new(
                Box::new(ok_file) as Box<dyn FileHandle>
            )),
        ))));
        assert!(writer_ok.text_writer_close().is_ok(), "исправный закрылся");
        assert!(
            writer_ok.text_writer_close().is_ok(),
            "повторный Закрыть() идемпотентен"
        );
    }

    /// Воспроизведение 5: заявленное входом число не превращается в
    /// огромную ёмкость. Отрицательное (счётчик MXL из `i64`) зажимается в
    /// ноль ДО преобразования в `usize`, а превышающее остаток входа — по
    /// границе; в пределах границы проходит как есть.
    #[test]
    fn reserve_hint_clamps_untrusted_counts() {
        let mut a: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut a, -1, 100).unwrap(), 0);
        let mut b: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut b, 1_000_000_000, 8).unwrap(), 8);
        let mut c: Vec<u8> = Vec::new();
        assert_eq!(reserve_hint(&mut c, 5, 8).unwrap(), 5);
    }

    /// Воспроизведение 3: два `УникальныйИдентификатор` из одних байтов
    /// равны по `PartialEq`, значит ОБЯЗАНЫ давать равный хэш — иначе
    /// `Соответствие` держит их двумя ключами. На прежнем дереве `Uuid`
    /// проваливался в `_ => Rc::as_ptr`, и хэши расходились.
    #[test]
    fn equal_value_objects_hash_equal() {
        use std::collections::hash_map::DefaultHasher;
        fn hash_of(value: &BslValue) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }
        let bytes = [
            0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x66, 0x66, 0x77, 0x77,
            0x88, 0x88,
        ];
        let a = BslValue::Object(Rc::new(BslObject::Uuid(bytes)));
        let b = BslValue::Object(Rc::new(BslObject::Uuid(bytes)));
        assert_eq!(a, b, "равные УИД равны по значению");
        assert_eq!(hash_of(&a), hash_of(&b), "равные значения — равный хэш");
    }

    /// Хэш согласован с `PartialEq` по ПОРЯДКУ правил. Тип, реализующий и
    /// `value_eq`, и `identity_key`, равняется по `value_eq` — это его
    /// `PartialEq`, — поэтому и хэшироваться обязан по нему: два объекта с
    /// одними байтами, но разными местами равны и дают один хэш. До
    /// выравнивания `Hash` брал `identity_key` первым и расходился, ломая
    /// ключ `Соответствия` для любого будущего типа с обоими методами.
    #[test]
    fn hash_follows_partial_eq_when_value_eq_and_identity_key_both_exist() {
        use std::collections::hash_map::DefaultHasher;

        #[derive(Debug)]
        struct BytesAtPlace {
            bytes: Vec<u8>,
            place: (usize, usize),
        }
        static BOTH: TypeDescriptor = TypeDescriptor::new("test", "БайтыСМестом");
        impl ObjectProtocol for BytesAtPlace {
            fn type_descriptor(&self) -> &'static TypeDescriptor {
                &BOTH
            }
            fn identity_key(&self) -> Option<(usize, usize)> {
                Some(self.place)
            }
            fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
                other
                    .downcast_ref::<BytesAtPlace>()
                    .map(|o| o.bytes == self.bytes)
            }
            fn display(&self) -> String {
                format!("байты:{:?}", self.bytes)
            }
        }

        fn hash_of(value: &BslValue) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }
        // Одни байты, РАЗНЫЕ места: равны по value_eq, различны по identity_key.
        let a = BslValue::new_object(BytesAtPlace {
            bytes: vec![1, 2, 3],
            place: (10, 0),
        });
        let b = BslValue::new_object(BytesAtPlace {
            bytes: vec![1, 2, 3],
            place: (20, 0),
        });
        assert_eq!(a, b, "равны по value_eq");
        assert_eq!(
            hash_of(&a),
            hash_of(&b),
            "хэш обязан следовать value_eq, а не identity_key"
        );
    }

    /// Закон равенства и хэша по ВИДАМ объектов.
    ///
    /// `PartialEq` и `Hash` для `BslValue` — две РАЗДЕЛЬНЫЕ реализации, и их
    /// рассогласование теряет ключ `Соответствия` (воспроизведение 3). Тест —
    /// предохранитель против такого рассогласования, и он устроен так, чтобы
    /// новый вид объекта нельзя было забыть:
    ///
    /// * `kind_of` перечисляет КАЖДЫЙ вариант `BslObject` матчем без `_`,
    ///   поэтому новый вариант не соберётся, пока автор не решит, идёт он по
    ///   общему пути тождества (`None`) или по собственному закону (`Some`);
    /// * `samples` — исчерпывающий матч по `Kind`, поэтому вид, объявленный
    ///   собственным, обязан принести и образцы, а значит пройти закон целиком.
    ///
    /// Виды с общим путём (`None`) представлены ОДНИМ образцом намеренно: у
    /// них в `PartialEq`/`Hash` не по ветке на вариант, а одна общая —
    /// `Rc::ptr_eq` и адрес обёртки, — и двенадцатикратный её повтор проверял
    /// бы один и тот же код.
    ///
    /// `Extension` — не один вид, а ТРИ: внешний объект выбирает между
    /// `value_eq`, `identity_key` и чистым тождеством обёртки, и у каждого
    /// выбора своя ветка в обеих реализациях (у `value_eq` хэшируется
    /// `display()`, у `identity_key` — ключ, иначе адрес). Классификатор
    /// повторяет ровно этот порядок правил.
    #[test]
    fn the_value_equality_and_hash_law_holds_across_object_kinds() {
        use std::collections::hash_map::DefaultHasher;
        fn hash_of(value: &BslValue) -> u64 {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        /// Виды, у которых равенство СВОЁ, — по одному на ветку
        /// `PartialEq`/`Hash`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Kind {
            BinaryData,
            Uuid,
            VstrOpaque,
            /// Внешний объект с `value_eq`: равен по содержимому.
            ExtensionByValue,
            /// Внешний объект без `value_eq`, но с `identity_key`: равен по
            /// объявленному месту.
            ExtensionByKey,
            /// Внешний объект без обоих: равен только сам себе.
            ExtensionBare,
        }

        /// Все виды со своим законом. Единственное место теста, которое
        /// поддерживается руками; `samples` ниже не даст забыть образцы, а
        /// проверка round-trip в конце — перепутать вид.
        const ALL_KINDS: [Kind; 6] = [
            Kind::BinaryData,
            Kind::Uuid,
            Kind::VstrOpaque,
            Kind::ExtensionByValue,
            Kind::ExtensionByKey,
            Kind::ExtensionBare,
        ];

        // --- Три внешних типа: по одному на ветку дисптача -----------------

        static BY_VALUE: TypeDescriptor = TypeDescriptor::new("test", "ВнешнийПоЗначению");
        static BY_KEY: TypeDescriptor = TypeDescriptor::new("test", "ВнешнийПоМесту");
        static BARE: TypeDescriptor = TypeDescriptor::new("test", "ВнешнийБезЗакона");

        #[derive(Debug)]
        struct ByValue(Vec<u8>);
        impl ObjectProtocol for ByValue {
            fn type_descriptor(&self) -> &'static TypeDescriptor {
                &BY_VALUE
            }
            fn value_eq(&self, other: &ObjectRef) -> Option<bool> {
                other.downcast_ref::<ByValue>().map(|o| o.0 == self.0)
            }
            // `Hash` для этой ветки берёт `display()`, поэтому равные по
            // `value_eq` объекты ОБЯЗАНЫ печататься одинаково — иначе ключ
            // `Соответствия` разъедется. Тест это и проверяет.
            fn display(&self) -> String {
                format!("по-значению:{:?}", self.0)
            }
        }

        #[derive(Debug)]
        struct ByKey {
            place: (usize, usize),
            /// Содержимое РАЗНОЕ у равных по месту: доказывает, что сравнение
            /// идёт по ключу, а не случайно по содержимому.
            tag: u8,
        }
        impl ObjectProtocol for ByKey {
            fn type_descriptor(&self) -> &'static TypeDescriptor {
                &BY_KEY
            }
            fn identity_key(&self) -> Option<(usize, usize)> {
                Some(self.place)
            }
            fn display(&self) -> String {
                format!("по-месту:{:?}:{}", self.place, self.tag)
            }
        }

        #[derive(Debug)]
        struct Bare(u8);
        impl ObjectProtocol for Bare {
            fn type_descriptor(&self) -> &'static TypeDescriptor {
                &BARE
            }
            fn display(&self) -> String {
                format!("без-закона:{}", self.0)
            }
        }

        /// Вид объекта — исчерпывающе по вариантам `BslObject`. `None` —
        /// общий путь: равенство и хэш по адресу обёртки.
        fn kind_of(object: &BslObject) -> Option<Kind> {
            match object {
                BslObject::BinaryData(_) => Some(Kind::BinaryData),
                BslObject::Uuid(_) => Some(Kind::Uuid),
                BslObject::VstrOpaque(_) => Some(Kind::VstrOpaque),
                // Порядок правил — тот же, что в `PartialEq`/`Hash`.
                BslObject::Extension(external) => Some(if external.value_eq(external).is_some() {
                    Kind::ExtensionByValue
                } else if external.identity_key().is_some() {
                    Kind::ExtensionByKey
                } else {
                    Kind::ExtensionBare
                }),
                BslObject::Array(_)
                | BslObject::Structure(_)
                | BslObject::ValueTable(_)
                | BslObject::TableColumns(_)
                | BslObject::TableColumn(..)
                | BslObject::TableRow(..)
                | BslObject::TypeDescription(_)
                | BslObject::ValueComparison
                | BslObject::Map(_)
                | BslObject::KeyValuePair(..)
                | BslObject::TextWriter(_)
                | BslObject::BinaryBuffer(_) => None,
            }
        }

        /// Образцы вида: свежая обёртка РАВНОГО содержимого на каждый вызов
        /// `equal` (разные `Rc` — иначе быстрый путь `Rc::ptr_eq` подменил бы
        /// проверку) и одна заведомо НЕравная.
        struct Samples {
            /// Образец РАВНОГО содержимого. Аргумент — «метка», которая по
            /// закону вида НЕ ДОЛЖНА влиять на равенство: у вида по месту в
            /// ней лежит различающееся содержимое (равенство обязано идти по
            /// ключу вопреки ему), у прочих видов она не используется.
            equal: fn(u8) -> BslValue,
            different: fn() -> BslValue,
        }

        /// Исчерпывающий матч: новый вид со своим законом обязан принести
        /// образцы, иначе тест не соберётся.
        fn samples(kind: Kind) -> Samples {
            match kind {
                Kind::BinaryData => Samples {
                    equal: |_| {
                        BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(&[1u8, 2, 3][..]))))
                    },
                    different: || {
                        BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(&[9u8][..]))))
                    },
                },
                Kind::Uuid => Samples {
                    equal: |_| BslValue::Object(Rc::new(BslObject::Uuid([0x11; 16]))),
                    different: || BslValue::Object(Rc::new(BslObject::Uuid([0x22; 16]))),
                },
                Kind::VstrOpaque => Samples {
                    equal: |_| BslValue::Object(Rc::new(BslObject::VstrOpaque("реф".to_string()))),
                    different: || {
                        BslValue::Object(Rc::new(BslObject::VstrOpaque("другой".to_string())))
                    },
                },
                Kind::ExtensionByValue => Samples {
                    equal: |_| BslValue::new_object(ByValue(vec![1, 2, 3])),
                    different: || BslValue::new_object(ByValue(vec![9])),
                },
                Kind::ExtensionByKey => Samples {
                    // Одно место, РАЗНЫЕ метки: равенство обязано идти по
                    // ключу места ВОПРЕКИ различному содержимому — метка и
                    // есть то содержимое, которое не должно ни на что влиять.
                    equal: |tag| {
                        BslValue::new_object(ByKey {
                            place: (10, 0),
                            tag,
                        })
                    },
                    different: || {
                        BslValue::new_object(ByKey {
                            place: (20, 0),
                            tag: 1,
                        })
                    },
                },
                Kind::ExtensionBare => Samples {
                    equal: |_| BslValue::new_object(Bare(1)),
                    different: || BslValue::new_object(Bare(2)),
                },
            }
        }

        fn object_of(value: &BslValue) -> &BslObject {
            match value {
                BslValue::Object(object) => object,
                other => panic!("ожидался объект, получено {other:?}"),
            }
        }

        for kind in ALL_KINDS {
            let Samples { equal, different } = samples(kind);
            // Три РАЗНЫЕ метки: для вида по месту это три разных содержимого
            // при одном ключе, и равенство обязано их не различать.
            let (a, b, c, other) = (equal(1), equal(2), equal(3), different());

            // Round-trip: образец действительно того вида, за который выдан, —
            // иначе закон проверялся бы не на той ветке дисптача.
            assert_eq!(kind_of(object_of(&a)), Some(kind), "вид образца {kind:?}");

            // Рефлексивность — общая для всех видов.
            assert_eq!(a, a, "{kind:?}: рефлексивность");
            assert_eq!(hash_of(&a), hash_of(&a), "{kind:?}: хэш устойчив");

            // Разное содержимое не равно ни при каком виде.
            assert_ne!(a, other, "{kind:?}: разное содержимое не равно");

            if kind == Kind::ExtensionBare {
                // Чистое тождество: две обёртки одного содержимого НЕ равны.
                assert_ne!(a, b, "{kind:?}: равны только сами себе");
                continue;
            }

            // Симметрия и транзитивность на трёх свежих обёртках.
            assert_eq!(a, b, "{kind:?}: равное содержимое равно");
            assert_eq!(b, a, "{kind:?}: симметрия");
            assert!(b == c && a == c, "{kind:?}: транзитивность");
            // Согласованность двух реализаций — то, ради чего тест написан.
            assert_eq!(hash_of(&a), hash_of(&b), "{kind:?}: равные — равный хэш");
        }

        // Виды с ОБЩИМ путём тождества: одна ветка `Rc::ptr_eq` на все, здесь
        // проверяется она, а не каждый вариант по отдельности.
        let shared = BslValue::new_array(vec![BslValue::number_from_i64(1)]);
        let twin = BslValue::new_array(vec![BslValue::number_from_i64(1)]);
        assert_eq!(kind_of(object_of(&shared)), None, "общий путь тождества");
        assert_eq!(shared, shared, "объект равен себе");
        assert_eq!(shared, shared.clone(), "клон той же обёртки равен");
        assert_ne!(shared, twin, "разные обёртки одного содержимого не равны");

        // Ответ `value_eq` УСТОЙЧИВ: тот же ответ при повторном спросе, иначе
        // и равенство, и хэш зависели бы от момента вызова.
        let stable = samples(Kind::ExtensionByValue);
        let (x, y) = ((stable.equal)(1), (stable.equal)(2));
        assert_eq!(x == y, x == y, "устойчивость ответа value_eq");
        assert_eq!(hash_of(&x), hash_of(&y), "устойчивость хэша");
    }

    #[test]
    fn display_matches_measured_platform_strings() {
        assert_eq!(BslValue::Boolean(true).to_string(), "Да");
        assert_eq!(BslValue::Boolean(false).to_string(), "Нет");
        assert_eq!(BslValue::Undefined.to_string(), "");
    }

    #[test]
    fn array_index_get_set_roundtrip() {
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        assert_eq!(
            arr.get_index(&num("1"), &NameInterner::new()).unwrap(),
            num("2")
        );
        arr.set_index(&num("1"), num("99")).unwrap();
        assert_eq!(
            arr.get_index(&num("1"), &NameInterner::new()).unwrap(),
            num("99")
        );
        assert_eq!(arr.collection_len().unwrap(), 3);
    }

    #[test]
    fn array_out_of_bounds_is_an_error() {
        let arr = BslValue::new_array(vec![num("1")]);
        assert!(matches!(
            arr.get_index(&num("5"), &NameInterner::new()).unwrap_err(),
            RtError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn arrays_and_structures_are_reference_types() {
        // b = a делает b тем же объектом, что и a: мутация через одну
        // переменную видна через другую (Rc, не глубокое копирование).
        let a = BslValue::new_array(vec![num("1")]);
        let b = a.clone();
        b.set_index(&num("0"), num("42")).unwrap();
        assert_eq!(
            a.get_index(&num("0"), &NameInterner::new()).unwrap(),
            num("42")
        );
        assert!(a.eq_value(&b));

        let c = BslValue::new_array(vec![num("42")]);
        assert!(
            !a.eq_value(&c),
            "структурно равные, но разные объекты — не равны"
        );
    }

    #[test]
    fn structure_field_get_set_by_interned_name() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let y = names.intern("y");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x, y]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1"), num("2")]);
        assert_eq!(s.get_field(x).unwrap(), num("1"));
        s.set_field(y, num("99")).unwrap();
        assert_eq!(s.get_field(y).unwrap(), num("99"));
    }

    #[test]
    fn unknown_field_is_an_error() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let z = names.intern("z");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1")]);
        assert!(matches!(
            s.get_field(z).unwrap_err(),
            RtError::UnknownField(_)
        ));
    }

    // --- Словарный режим структуры ------------------------------------
    //
    // Порог `MAX_SHAPE_TRANSITIONS` — единственная защита от того, что
    // `Вставить` с динамическим именем в цикле навсегда интернирует форму
    // на каждой итерации (см. doc comment на константе). Тесты ниже
    // фиксируют и сам переход, и то, ради чего он затеян: таблица форм
    // после него перестаёт расти.

    /// Пустая структура плюс рантайм-контекст форм — общая затравка для
    /// тестов деградации.
    fn fresh_structure() -> (BslValue, RuntimeShapes) {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let empty = rt.shapes.empty();
        (BslValue::new_structure(empty, Vec::new()), rt)
    }

    fn is_dictionary(v: &BslValue) -> bool {
        match v {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    matches!(&*s.borrow(), StructureStorage::Dictionary { .. })
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// `Вставить("Поле<i>", i)` — ровно тот путь, на котором форма уходит
    /// вглубь: каждый ключ новый, так что каждый переход заводил бы форму.
    fn insert_generated_fields(s: &BslValue, rt: &mut RuntimeShapes, count: u32) {
        for i in 0..count {
            let f = rt.names.intern(&format!("Поле{i}"));
            s.structure_insert(f, num(&i.to_string()), &mut rt.shapes)
                .unwrap();
        }
    }

    #[test]
    fn structure_degrades_to_dictionary_after_threshold_inserts() {
        let (s, mut rt) = fresh_structure();

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS);
        assert!(
            !is_dictionary(&s),
            "ровно {MAX_SHAPE_TRANSITIONS} переходов должны укладываться в порог"
        );

        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 1);
        assert!(
            is_dictionary(&s),
            "переход за порог обязан деградировать объект"
        );
    }

    #[test]
    fn dictionary_structure_field_get_set_roundtrip() {
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, MAX_SHAPE_TRANSITIONS + 5);
        assert!(is_dictionary(&s));

        // Поля, заведённые ДО деградации (перенесённые из слотов), и после
        // неё — читаются и пишутся одинаково.
        let first = rt.names.intern("Поле0");
        let last = rt
            .names
            .intern(&format!("Поле{}", MAX_SHAPE_TRANSITIONS + 4));
        assert_eq!(s.get_field(first).unwrap(), num("0"));
        assert_eq!(
            s.get_field(last).unwrap(),
            num(&(MAX_SHAPE_TRANSITIONS + 4).to_string())
        );

        s.set_field(first, num("777")).unwrap();
        s.set_field(last, num("888")).unwrap();
        assert_eq!(s.get_field(first).unwrap(), num("777"));
        assert_eq!(s.get_field(last).unwrap(), num("888"));

        let missing = rt.names.intern("НетТакогоПоля");
        assert!(matches!(
            s.get_field(missing).unwrap_err(),
            RtError::UnknownField(_)
        ));
        assert!(matches!(
            s.set_field(missing, num("1")).unwrap_err(),
            RtError::UnknownField(_)
        ));
    }

    #[test]
    fn dictionary_structure_delete_keeps_order_of_the_rest() {
        let total = MAX_SHAPE_TRANSITIONS + 3;
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, total);
        assert!(is_dictionary(&s));

        let victim = rt.names.intern("Поле1");
        s.structure_delete(victim, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);

        // Ожидаемый порядок — исходный без удалённого: `order` теряет ровно
        // один элемент, остальные не переставляются.
        let expected: Vec<String> = (0..total)
            .filter(|i| *i != 1)
            .map(|i| format!("Поле{i}"))
            .collect();
        let actual: Vec<String> = (0..expected.len())
            .map(
                |i| match s.get_index(&num(&i.to_string()), &rt.names).unwrap() {
                    BslValue::Object(o) => match &*o {
                        BslObject::KeyValuePair(k, _) => k.to_string(),
                        other => panic!("ожидался КлючИЗначение, получено {other:?}"),
                    },
                    other => panic!("ожидался объект, получено {other:?}"),
                },
            )
            .collect();
        assert_eq!(actual, expected);

        // Удаление отсутствующего — no-op и в словарном режиме тоже.
        let missing = rt.names.intern("НетТакогоПоля");
        s.structure_delete(missing, &mut rt.shapes).unwrap();
        assert_eq!(s.collection_len().unwrap(), total as usize - 1);
    }

    #[test]
    fn shape_table_stops_growing_after_degradation() {
        // Суть всей задачи: без порога здесь было бы 10_000 бессмертных
        // форм со списками имён нарастающей длины — квадратичная память.
        let (s, mut rt) = fresh_structure();
        insert_generated_fields(&s, &mut rt, 10_000);

        assert!(is_dictionary(&s));
        assert_eq!(s.collection_len().unwrap(), 10_000);
        // Пустая форма + по одной на каждый разрешённый переход, и ни одной
        // сверх того.
        assert_eq!(rt.shapes.len(), MAX_SHAPE_TRANSITIONS as usize + 1);
    }

    #[test]
    fn dictionary_structure_does_not_poison_inline_cache_for_shaped_objects() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // `Поле0` — первое имя и у `shaped`, и у сгенерированной серии, так
        // что оба объекта проходят через одну и ту же форму `[Поле0]`.
        let x = rt.names.intern("Поле0");

        let shaped = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        shaped
            .structure_insert(x, num("1"), &mut rt.shapes)
            .unwrap();

        let dict = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        insert_generated_fields(&dict, &mut rt, MAX_SHAPE_TRANSITIONS + 2);
        assert!(is_dictionary(&dict));
        assert!(!is_dictionary(&shaped));
        assert_eq!(dict.get_field(x).unwrap(), num("0"));

        // Один и тот же сайт вызова (одна ячейка кэша) видит оба объекта.
        let cache: std::cell::RefCell<Option<(Rc<Shape>, u32)>> = std::cell::RefCell::new(None);

        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("1"));
        let filled = cache.borrow().clone();
        assert!(filled.is_some(), "шейповый объект обязан заполнить кэш");

        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("0"));
        let after_dict = cache.borrow().clone();
        match (&filled, &after_dict) {
            (Some((a, ai)), Some((b, bi))) => {
                assert!(Rc::ptr_eq(a, b), "словарный объект затёр форму в кэше");
                assert_eq!(ai, bi, "словарный объект затёр слот в кэше");
            }
            _ => panic!("словарный объект обнулил ячейку кэша"),
        }

        // И быстрый путь для шейпового объекта по-прежнему работает.
        shaped.set_field_cached(x, num("42"), &cache).unwrap();
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
        // Запись через тот же сайт в словарный объект тоже не портит кэш.
        dict.set_field_cached(x, num("43"), &cache).unwrap();
        assert_eq!(dict.get_field_cached(x, &cache).unwrap(), num("43"));
        let after_dict_set = cache.borrow().clone();
        match (&filled, &after_dict_set) {
            (Some((a, _)), Some((b, _))) => assert!(Rc::ptr_eq(a, b)),
            _ => panic!("словарный объект обнулил ячейку кэша при записи"),
        }
        assert_eq!(shaped.get_field_cached(x, &cache).unwrap(), num("42"));
    }

    #[test]
    fn display_matches_measured_platform_strings_for_collections() {
        // Строка(Новый Массив) -> "Массив" (измерено на платформе).
        assert_eq!(BslValue::new_array(vec![]).to_string(), "Массив");
    }

    #[test]
    fn builtin_math_functions_lookup_and_call() {
        assert_eq!(BuiltinFn::lookup("sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("Sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("СООБЩИТЬ"), Some(BuiltinFn::Message));
        assert_eq!(
            BuiltinFn::lookup("ТекущаяУниверсальнаяДатаВМиллисекундах"),
            Some(BuiltinFn::CurrentUniversalDateInMilliseconds)
        );
        assert_eq!(
            BuiltinFn::CurrentUniversalDateInMilliseconds.arity_range(),
            (0, 0)
        );
        assert_eq!(BuiltinFn::lookup("НетТакойФункции"), None);
        assert_eq!(BuiltinFn::Pow.arity_range(), (2, 2));
        assert_eq!(BuiltinFn::Sqrt.arity_range(), (1, 1));
        // Необязательный аргумент — диапазон, а не одно число.
        assert_eq!(BuiltinFn::Mid.arity_range(), (2, 3));
        assert_eq!(BuiltinFn::StrTemplate.arity_range(), (1, 11));

        let v = call_builtin_fn(BuiltinFn::Sqrt, &[num("2")]).unwrap();
        assert_eq!(v, num("1.4142135623731"));
    }

    #[test]
    fn builtin_method_count_on_array() {
        assert_eq!(BuiltinMethod::lookup("count"), Some(BuiltinMethod::Count));
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        let v = call_builtin_method(BuiltinMethod::Count, &arr, &[]).unwrap();
        assert_eq!(v, num("3"));
    }

    #[test]
    fn builtin_method_upper_bound_on_array() {
        assert_eq!(
            BuiltinMethod::lookup("UBound"),
            Some(BuiltinMethod::UpperBound)
        );
        let empty = BslValue::new_array(Vec::new());
        assert_eq!(
            call_builtin_method(BuiltinMethod::UpperBound, &empty, &[]).unwrap(),
            num("-1")
        );
        let filled = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        assert_eq!(
            call_builtin_method(BuiltinMethod::UpperBound, &filled, &[]).unwrap(),
            num("2")
        );
    }

    /// Двоичные данные из байтов — минуя файл: разбиение и склейка сами по
    /// себе к файловой системе отношения не имеют, а фикстура
    /// `binary-data` проверяет их вместе с конструктором.
    fn bin(bytes: &[u8]) -> BslValue {
        BslValue::binary_data_of(bytes)
    }

    /// Размеры частей разбиения — то, что видно из BSL через `Размер()`.
    fn part_sizes(parts: &BslValue) -> Vec<usize> {
        let BslValue::Object(o) = parts else {
            panic!("разбиение обязано отдать массив, отдало {parts:?}");
        };
        let BslObject::Array(items) = &**o else {
            panic!("разбиение обязано отдать массив, отдало {parts:?}");
        };
        items
            .borrow()
            .iter()
            .map(
                |part| match part.binary_data_size().expect("у части есть размер") {
                    BslValue::Number(n) => n.to_i64_exact().expect("размер целый") as usize,
                    other => panic!("Размер() вернул не число: {other:?}"),
                },
            )
            .collect()
    }

    #[test]
    fn binary_data_split_exact_multiple() {
        let parts = bin(b"0123456789ab").binary_data_split(&num("4")).unwrap();
        assert_eq!(part_sizes(&parts), vec![4, 4, 4]);
    }

    #[test]
    fn binary_data_split_short_tail() {
        let parts = bin(b"0123456789").binary_data_split(&num("4")).unwrap();
        assert_eq!(part_sizes(&parts), vec![4, 4, 2]);
    }

    #[test]
    fn binary_data_split_part_larger_than_whole() {
        let parts = bin(b"012").binary_data_split(&num("100")).unwrap();
        assert_eq!(part_sizes(&parts), vec![3]);
        // Размер части шире `usize`, но в пределах `2^64-1`, — не ошибка:
        // та же одна часть (измерено, см. `binary_split_max_part`).
        let parts = bin(b"012")
            .binary_data_split(&num("18446744073709551615"))
            .unwrap();
        assert_eq!(part_sizes(&parts), vec![3]);
        // На единицу больше — уже ошибка, ровно как у платформы.
        assert!(
            bin(b"012")
                .binary_data_split(&num("18446744073709551616"))
                .is_err()
        );
    }

    /// Пустые данные дают массив из ОДНОЙ пустой части, а не пустой массив
    /// (измерено фикстурой `binary-data`).
    #[test]
    fn binary_data_split_empty_yields_one_empty_part() {
        let parts = bin(b"").binary_data_split(&num("5")).unwrap();
        assert_eq!(part_sizes(&parts), vec![0]);
    }

    #[test]
    fn binary_data_split_rejects_non_positive_and_fractional_sizes() {
        for bad in ["0", "-1", "2.5"] {
            assert!(
                bin(b"0123").binary_data_split(&num(bad)).is_err(),
                "размер части {bad} обязан быть ошибкой"
            );
        }
        // Числовая строка тоже отвергается — платформа её не приводит.
        assert!(
            bin(b"0123")
                .binary_data_split(&BslValue::Str(BslString::from_str("5")))
                .is_err()
        );
        // Разбивать не двоичные данные нечего.
        assert!(
            BslValue::Str(BslString::from_str("абв"))
                .binary_data_split(&num("2"))
                .is_err()
        );
    }

    #[test]
    fn binary_data_combine_empty_array() {
        let joined = BslValue::new_array(vec![]).binary_data_combine().unwrap();
        assert_eq!(joined, bin(b""));
        assert!(!joined.is_filled().expect("пустые данные не заполнены"));
        assert_eq!(joined.to_string(), "");
    }

    #[test]
    fn binary_data_combine_concatenates_in_array_order() {
        let joined = BslValue::new_array(vec![bin(b"ab"), bin(b""), bin(b"cd")])
            .binary_data_combine()
            .unwrap();
        assert_eq!(joined, bin(b"abcd"));
    }

    #[test]
    fn binary_data_combine_rejects_a_non_binary_element() {
        let bad = BslValue::new_array(vec![bin(b"ab"), BslValue::Str(BslString::from_str("вг"))]);
        assert!(bad.binary_data_combine().is_err());
        let bad = BslValue::new_array(vec![bin(b"ab"), BslValue::Undefined]);
        assert!(bad.binary_data_combine().is_err());
        // Аргумент вообще не массив.
        assert!(bin(b"ab").binary_data_combine().is_err());
    }

    /// Строковое представление — байты, а не имя типа: пары в верхнем
    /// регистре через пробел, не длиннее 256 байт, с многоточием после
    /// обрезания (измерено, фикстура `binary-data` плюс проба
    /// `BIN.STR.LONG`).
    #[test]
    fn binary_data_display_is_a_hex_dump_capped_at_256_bytes() {
        assert_eq!(bin(&[0xef, 0xbb, 0xbf, 0x30]).to_string(), "EF BB BF 30");
        assert_eq!(bin(b"").to_string(), "");

        let at_limit = bin(&[0x41; 256]).to_string();
        assert_eq!(at_limit.len(), 256 * 3 - 1);
        assert!(!at_limit.ends_with("..."), "на границе многоточия ещё нет");

        let over_limit = bin(&[0x41; 257]).to_string();
        assert_eq!(over_limit.len(), 256 * 3 - 1 + 3);
        assert!(over_limit.ends_with("41..."), "многоточие без пробела");
    }

    /// Равенство и хэш идут ПО СОДЕРЖИМОМУ (измерено): иначе одинаковые
    /// данные из двух файлов оказались бы разными ключами `Соответствие`.
    #[test]
    fn binary_data_compares_and_hashes_by_content() {
        assert_eq!(bin("абв".as_bytes()), bin("абв".as_bytes()));
        assert_ne!(bin("абв".as_bytes()), bin("абг".as_bytes()));

        let mut map = crate::map::MapData::default();
        map.insert(bin("ключ".as_bytes()), num("1"));
        assert_eq!(map.get(&bin("ключ".as_bytes())), Some(num("1")));
    }

    #[test]
    fn uuid_is_accepted_by_case_conversion_functions() {
        let uuid = BslValue::Object(Rc::new(BslObject::Uuid([
            0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56,
            0x78, 0x90,
        ])));
        assert_eq!(
            uuid.str_lower().unwrap().to_string(),
            "abcdef12-3456-7890-abcd-ef1234567890"
        );
        assert_eq!(
            uuid.str_upper().unwrap().to_string(),
            "ABCDEF12-3456-7890-ABCD-EF1234567890"
        );
    }
}
