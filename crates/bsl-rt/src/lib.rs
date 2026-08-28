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
mod value_graph;
mod value_list;
mod vstr;
pub use background_jobs::{BackgroundJobService, JobWaitOutcome};
pub use host_error::{HostError, HostErrorCode};
pub use job_dto::{JobErrorDto, JobId, JobKeyDto, JobSnapshotDto, JobStateDto, UserMessageDto};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::rc::Rc;
pub use temp_storage::{
    GlobalStagingBudget, StagedWrite, StagingBudget, TempMailbox, TempStorageHub,
    TempStorageSession,
};
pub use value_graph::{GraphBudget, GraphLimits, SerializedValueGraph};

pub use bsl_number::BslNumber;
use bsl_number::NumError;

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
    LibraryRequirement, MethodCall, MethodDescriptor, ObjectContextNeed, PendingHostCall,
    PropertyDescriptor, PropertyGet, PropertySet, RegistryError, RuntimeBuilder, RuntimeRegistry,
    SuspendingMethodCall, call_method_from_table, core_library, get_property_from_table,
    set_property_from_table,
};
pub use date::{
    BslDate, DEFAULT_PATTERN as DEFAULT_DATE_PATTERN, UNIX_EPOCH_SECONDS,
    format_long as format_date_long, format_pattern as format_date_pattern,
    local_date_from_utc_seconds, pseudo_unix_seconds,
};
use date::{DateBoundary, DatePart};
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
use map::MapData;
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

#[derive(Debug, Clone)]
pub enum BslValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(BslNumber),
    Str(BslString),
    /// Момент времени с разрешением 1 секунда. Отсчёт — от `0001-01-01`,
    /// НЕ от Unix-эпохи: пустая дата (`'00010101'`) обязана быть нулём, а
    /// не отрицательным числом, иначе `ЗначениеЗаполнено` и сравнения с
    /// пустой датой пришлось бы писать через отдельную константу. Подробно
    /// — в модуле `date`.
    Date(BslDate),
    /// Тип как ЗНАЧЕНИЕ (`ТипЗнч(х)`, `Тип("Массив")`) — не тег этого
    /// перечисления, а полноценное значение, которое можно сравнить,
    /// положить в переменную и напечатать (`Строка(ТипЗнч(1))` -> `Число`).
    /// `TypeRef` — `Copy` в размер указателя, поэтому вариант ничего не
    /// добавляет к размеру `BslValue`.
    Type(TypeRef),
    /// Член платформенного перечисления (`ТипЗначенияJSON.Строка`). Как и
    /// `Type`, это ЗНАЧЕНИЕ, а не тег: его можно сравнить, положить в
    /// переменную и напечатать. `Copy` в один байт — размер `BslValue` не
    /// растёт (см. модуль `enums`).
    Enum(EnumValue),
    /// Голое имя системного перечисления как ВЫРАЖЕНИЕ (`Вычислить("ВариантЗаписиДатыJSON")`,
    /// без `.Член`) — ИЗМЕРЕНО (`JSON.DATE_VARIANT_EN_NAMES`, «Т+»): платформа
    /// принимает такое выражение, а не отвергает его как обращение к
    /// неопределённой переменной. Что именно возвращает `Строка()`/`ТипЗнч()`
    /// от этого значения — `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`; здесь это
    /// самостоятельный вариант (не `Type`/`Enum` — семантически ни то, ни
    /// другое: не тип и не конкретный член), `Copy` в один байт, как и они.
    EnumType(EnumKind),
    Object(Rc<BslObject>),
}

/// Ошибка, о которой сообщил компонент: пакет, категория и текст.
///
/// Отдельная структура за `Box` в [`RtError::Component`] — см. там же,
/// почему поля не лежат в самом варианте.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentError {
    /// Cargo-имя пакета компонента.
    pub package: &'static str,
    /// Категория ошибки в терминах самого компонента — то, что host
    /// назвал бы «видом»: «формат», «доступ», «предел».
    pub kind: &'static str,
    pub message: String,
}

impl ComponentError {
    /// Ошибка компонента как [`RtError`] — форма, в которой её ждёт VM.
    #[must_use]
    pub fn raise(package: &'static str, kind: &'static str, message: impl Into<String>) -> RtError {
        RtError::Component(Box::new(ComponentError {
            package,
            kind,
            message: message.into(),
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RtError {
    Num(NumError),
    /// Операция получила значение не того типа — например, `Если 1 Тогда`:
    /// в BSL условия строго булевы, никакой truthiness.
    TypeError {
        expected: &'static str,
        op: &'static str,
    },
    /// Индексация значения, которое не индексируется (не `Массив`).
    NotIndexable,
    IndexOutOfBounds {
        index: i64,
        len: usize,
    },
    /// Индекс — не целое неотрицательное число.
    BadIndex,
    /// Доступ к полю значения, у которого полей нет (не `Структура`).
    NotAnObject,
    /// Обращение к полю, которого нет в форме структуры.
    UnknownField(NameId),
    /// `ВызватьИсключение <значение>;` — значение, с которым бросили.
    Raised(BslValue),
    /// Явный ресурсный лимит host-а: бюджет снимка значений, admission
    /// фонового задания, staging временного хранилища. Ловимая ошибка:
    /// платформенные лимиты BSL-код перехватывает обычной «Попыткой».
    ResourceLimit(String),
    /// Типизированная ошибка host-границы фоновых заданий: для BSL —
    /// такое же ловимое исключение одного класса, как `ResourceLimit`,
    /// Rust-встраивание различает причину по [`HostErrorCode`].
    Host(Box<HostError>),
    /// Кооперативная отмена исполнения. НЕ ловится «Попыткой» — ИЗМЕРЕНО
    /// (`JOB.CANCEL.CATCH`): после отмены фонового задания ветка
    /// `Исключение` и код после неё не выполнялись. Разматывание проходит
    /// мимо обработчиков и доводит отмену до драйвера.
    Canceled,
    /// Обращение к `СтрокаТаблицы`, чья строка уже удалена (`row_id` не
    /// не резолвится обратным индексом) — не тихое чтение чужих данных.
    RowInvalidated,
    /// Обращение к несуществующей колонке `ТаблицыЗначений`/`СтрокиТаблицы`.
    UnknownColumn(String),
    /// Разбор или запись JSON: битый входной текст либо нарушение
    /// структуры документа при записи (`ЗаписатьКонецОбъекта` без
    /// открытого объекта). Отдельно от [`RtError::DynamicError`], у
    /// которого смысл другой — ошибка ЧУЖОГО слоя, пришедшая готовым
    /// текстом.
    Json(String),
    /// Разбор или запись XML: битая разметка либо нарушение структуры
    /// документа при записи (атрибут после текста). Отдельно от
    /// [`RtError::Json`] — чтобы по типу ошибки было видно, чей это слой.
    Xml(String),
    /// Объектная модель XML-схемы: конструкция XSD, которой эта модель не
    /// знает, битая ссылка или неверный аргумент разбора. Отдельно от
    /// [`RtError::Xml`] по той же причине, по какой тот отделён от JSON:
    /// разбор схемы — свой слой поверх готового дерева DOM, и по типу
    /// ошибки должно быть видно, чей это отказ.
    Xsd(String),
    /// Модель типов XDTO поверх модели схемы: ссылка на тип, которого в
    /// схеме нет, цикл наследования, лексическая форма, не разбирающаяся
    /// в своём типе. Отдельно от [`RtError::Xsd`] по той же причине, по
    /// какой тот отделён от [`RtError::Xml`]: разрешённая модель типов —
    /// слой поверх лексической модели схемы, и отказы у них разные.
    Xdto(String),
    /// Выражение XPath над деревом DOM: негодный синтаксис, неизвестная
    /// функция или ось, неразыменованный префикс, неподходящий контекстный
    /// узел. Отдельно от [`RtError::Xml`] по той же причине, по какой
    /// отделены разбор схемы и модель типов: вычислитель выражений — свой
    /// слой поверх готового дерева, и его отказы не спутать с разбором
    /// разметки.
    XPath(String),
    /// `ТекстовыйДокумент`: области макета и его параметры. Отдельно от
    /// [`RtError::Xml`] — слой другой, и по типу ошибки это должно быть
    /// видно.
    TextDoc(String),
    /// `ТабличныйДокумент`: адресация областей, запись и чтение файлов.
    /// Отдельно от [`RtError::TextDoc`] по той же причине — это другой
    /// слой с другим форматом файла.
    Spread(String),
    /// Регулярное выражение: шаблон не разбирается — незакрытая группа или
    /// класс, перевёрнутый диапазон, квантор без атома, неподдержанная
    /// конструкция диалекта. Отдельно от соседей по той же причине, по
    /// какой отделены они: разбор шаблона — свой слой, и по типу ошибки
    /// должно быть видно, чей это отказ. Ошибка ТОЛЬКО разбора: сам поиск
    /// либо находит, либо нет.
    Regex(String),
    /// Внутренний строковый формат значений (`ЗначениеВСтрокуВнутр` /
    /// `ЗначениеИзСтрокиВнутр`): текст не является форматом либо несёт
    /// вид объекта, которого в этой реализации нет. Отдельно от соседей
    /// по той же причине, что и они: свой слой — свой тип ошибки.
    Vstr(String),
    /// Архив: поток deflate не разбирается — обрезан, зарезервированный тип
    /// блока, неверный код Хаффмана, ссылка за начало распакованных данных,
    /// превышен заявленный размер. Отдельно от [`RtError::IoError`]
    /// намеренно: тот означает отказ файловой ОПЕРАЦИИ (открыть, прочитать,
    /// закрыть), а здесь байты прочитаны успешно и не годится их
    /// СОДЕРЖИМОЕ — по типу ошибки это должно быть видно так же, как у XML,
    /// XSD и XDTO.
    Zip(String),
    /// Писатель PDF: испорченный вход его API — координата NaN, цвет вне
    /// диапазона, документ без страниц, управляющий знак в тексте.
    /// Отдельно от [`RtError::Spread`] намеренно: `pdf` — слой формата
    /// файла, а не табличного документа, и пользоваться им будет не
    /// только он.
    Pdf(String),
    /// Имя из `СписокСвойств` в `ЗаполнитьЗначенияСвойств`, которого нет у
    /// источника или у приёмника. Отдельно от [`RtError::UnknownField`] и
    /// [`RtError::UnknownColumn`], потому что имя тут пришло СТРОКОЙ из
    /// списка (интернировать его незачем — оно может не быть полем ничего)
    /// и одинаково относится к обеим сторонам, а не к конкретному
    /// носителю.
    UnknownProperty(String),
    /// Запись в свойство, у которого есть только чтение.
    PropertyReadOnly {
        property: String,
        receiver: &'static str,
    },
    /// `Тип("ОпечаткаВИмени")` — такого типа в реестре нет.
    UnknownType(String),
    /// Дата вышла за `0001-01-01 .. 9999-12-31` — при построении
    /// (`Дата(0, 1, 1)`), при сдвиге (`Дата + огромное число`) или при
    /// `ДобавитьМесяц`. Заворачивать в другой конец диапазона нельзя:
    /// молчаливое `9999-12-31 + 1 сутки = 0001-01-01` дало бы неверные
    /// сравнения там, где ожидалась ошибка.
    DateOutOfRange {
        op: &'static str,
    },
    /// Метод объекта существует, но не для этого типа получателя, либо
    /// вызван не с тем числом аргументов для этого типа (некоторые методы,
    /// например `Добавить`, полиморфны: означают разное в зависимости от
    /// типа получателя, и арность из-за этого проверяется в рантайме, не
    /// на этапе резолвинга).
    MethodNotApplicable {
        method: &'static str,
        receiver: &'static str,
    },
    /// Имя метода отсутствует у фактического типа получателя. В отличие
    /// от [`RtError::MethodNotApplicable`], имя не принадлежит закрытой
    /// таблице ядра и поэтому хранится как строка программы.
    UnknownMethod {
        method: String,
        receiver: &'static str,
    },
    /// Ошибка лексера/парсера/резолвинга/компиляции строки, переданной в
    /// `Выполнить`/`Вычислить` — текст уже отформатирован тем слоем, что
    /// её обнаружил (`bsl-syntax`/`bsl-sema`/`bsl-bytecode`), сюда попадает
    /// как есть: `bsl-rt` не знает про эти крейты (обратная зависимость).
    DynamicError(String),
    /// Инструкция сослалась на то, чего в её `Program`/`Chunk` нет: номер
    /// регистра за границей стека кадра, номер чанка/константы/формы/имени
    /// за границей таблицы, недостача аргументов у builtin'а. Корректный
    /// кодоген такого не порождает — но VM исполняет и байт-код, собранный
    /// в рантайме (`Выполнить`/`Вычислить`, REPL `bsl-cli`), и предъявленный
    /// извне через публичный `run_program`/`run_repl_chunk_with_registry`,
    /// поэтому это ошибка, а не паника: уронить процесс на кривом входе —
    /// не вариант.
    InvalidBytecode(&'static str),
    /// Ошибка открытия, записи или закрытия файла.
    IoError(String),
    /// Байт-код требует отсутствующий/несовместимый runtime-компонент либо
    /// неизвестный код функции этого компонента. Ошибка СВЯЗЫВАНИЯ: она
    /// возникает до первой инструкции и говорит о сборке, а не о данных.
    Link(String),
    /// Ошибка, о которой сообщил САМ компонент.
    ///
    /// Транспортная форма для того, у кого нет и не должно быть варианта в
    /// этом перечислении: сторонний компонент, подключённый хостом через
    /// `register_library`, называет свой пакет и категорию, а подробный
    /// тип ошибки хранит у себя и переводит сюда на входе в VM.
    ///
    /// Варианты ядра ниже (`Json`, `Xml`, `Zip`, ...) сознательно остаются
    /// как есть: у официальных компонентов текст ошибки — часть измеренной
    /// совместимости, ловимая `Попыткой`, и переводить его в общий вид
    /// значило бы переписать двести мест ради единообразия, потеряв
    /// типизацию там, где она уже есть.
    ///
    /// Поля за `Box`: `RtError` едет в каждом `RtResult`, в том числе по
    /// рекурсивным путям разбора, и его ширина — это глубина стека. Три
    /// поля здесь подняли бы размер с 48 байт до 56, и предел вложенности
    /// JSON перестал бы срабатывать раньше переполнения стека — тест
    /// `too_deep_json_document_is_an_error_not_a_crash` это и показал.
    Component(Box<ComponentError>),
    /// Превышена глубина стека: слишком глубокая рекурсия вызовов BSL,
    /// вложенность `Выполнить`/`Вычислить` или вложенность данных при
    /// сериализации. `what` уточняет, какой именно предел задет. Это
    /// перехватываемая `Попыткой` ошибка, а не паника: одно из мест, где
    /// вход пользователя (сколь угодно глубокая рекурсия или циклическая
    /// структура) не имеет права ронять процесс.
    StackOverflow {
        what: &'static str,
    },
    /// Возможность прогона (`stdout`, зона, файловая система, источник случайности,
    /// вызов функции модуля) спрошена на пути, который её не несёт. Одна форма отказа
    /// вместо молчаливого стока JIT-шимов, локального `missing_zone` в
    /// bsl-json и `InvalidBytecode` о зоне: расхождение путей исполнения —
    /// это отсутствие возможности, а не повреждённый образ. Ловится
    /// `Попыткой`, как и прочие рантайм-условия.
    CapabilityMissing {
        capability: crate::component::Capability,
        path: crate::component::ContextKind,
    },
}

impl RtError {
    /// Ловится ли ошибка оператором `Попытка`. Повреждённый ОБРАЗ программы
    /// (`InvalidBytecode`) — не пользовательское исключение: если его
    /// поймать, битый байт-код уйдёт наружу с признаком успеха, а это ровно
    /// тот класс «недостоверному входу доверяют», который периметр образа и
    /// закрывает. Всё остальное приходит из пользовательских данных или
    /// чужого слоя (`Link`, `StackOverflow`, `DynamicError`, ошибки
    /// форматов) и ловится штатно.
    ///
    /// `match` исчерпывающий и без `_` НАРОЧНО: новый вариант `RtError` не
    /// соберётся, пока его не отнесут к ловимым или нет.
    #[must_use]
    pub fn is_bsl_exception(&self) -> bool {
        match self {
            RtError::InvalidBytecode(_) => false,
            RtError::Canceled => false,
            RtError::ResourceLimit(_) => true,
            // Ошибки host-границы фоновых заданий — один класс ловимого
            // исключения (план, «Переносимый host-контракт»).
            RtError::Host(_) => true,
            RtError::Num(_)
            | RtError::TypeError { .. }
            | RtError::NotIndexable
            | RtError::IndexOutOfBounds { .. }
            | RtError::BadIndex
            | RtError::NotAnObject
            | RtError::UnknownField(_)
            | RtError::Raised(_)
            | RtError::RowInvalidated
            | RtError::UnknownColumn(_)
            | RtError::Json(_)
            | RtError::Xml(_)
            | RtError::Xsd(_)
            | RtError::Xdto(_)
            | RtError::XPath(_)
            | RtError::TextDoc(_)
            | RtError::Spread(_)
            | RtError::Regex(_)
            | RtError::Vstr(_)
            | RtError::Zip(_)
            | RtError::Pdf(_)
            | RtError::UnknownProperty(_)
            | RtError::PropertyReadOnly { .. }
            | RtError::UnknownType(_)
            | RtError::DateOutOfRange { .. }
            | RtError::MethodNotApplicable { .. }
            | RtError::UnknownMethod { .. }
            | RtError::DynamicError(_)
            | RtError::IoError(_)
            | RtError::Link(_)
            | RtError::Component(_)
            | RtError::StackOverflow { .. }
            | RtError::CapabilityMissing { .. } => true,
        }
    }
}

impl From<NumError> for RtError {
    fn from(e: NumError) -> Self {
        RtError::Num(e)
    }
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RtError::Num(e) => write!(f, "{e}"),
            RtError::ResourceLimit(what) => write!(f, "превышен ресурсный лимит: {what}"),
            RtError::Host(error) => write!(f, "{error}"),
            RtError::Canceled => write!(f, "выполнение отменено"),
            RtError::TypeError { expected, op } => {
                write!(f, "ожидался тип «{expected}» для операции «{op}»")
            }
            RtError::NotIndexable => write!(f, "значение не поддерживает индексацию"),
            RtError::IndexOutOfBounds { index, len } => {
                write!(f, "индекс {index} вне границ (длина {len})")
            }
            RtError::BadIndex => write!(f, "индекс должен быть целым неотрицательным числом"),
            RtError::NotAnObject => write!(f, "значение не поддерживает доступ к полям"),
            RtError::UnknownField(_) => write!(f, "поле не найдено в структуре"),
            RtError::Raised(v) => write!(f, "{v}"),
            RtError::RowInvalidated => write!(f, "строка таблицы значений больше не существует"),
            RtError::UnknownColumn(name) => write!(f, "колонка «{name}» не найдена"),
            RtError::UnknownProperty(name) => write!(f, "свойство «{name}» не найдено"),
            RtError::PropertyReadOnly { property, receiver } => write!(
                f,
                "свойство «{property}» объекта «{receiver}» доступно только для чтения"
            ),
            RtError::Json(msg) => write!(f, "{msg}"),
            RtError::Xml(msg) => write!(f, "{msg}"),
            RtError::Xsd(msg) => write!(f, "{msg}"),
            RtError::Xdto(msg) => write!(f, "{msg}"),
            RtError::XPath(msg) => write!(f, "{msg}"),
            RtError::TextDoc(msg) => write!(f, "{msg}"),
            RtError::Spread(msg) => write!(f, "{msg}"),
            RtError::Regex(msg) => write!(f, "{msg}"),
            RtError::Vstr(msg) => write!(f, "{msg}"),
            RtError::Zip(msg) => write!(f, "{msg}"),
            RtError::Pdf(msg) => write!(f, "{msg}"),
            RtError::UnknownType(name) => write!(f, "тип «{name}» не определён"),
            RtError::DateOutOfRange { op } => write!(
                f,
                "результат «{op}» вне диапазона дат (0001-01-01 .. 9999-12-31)"
            ),
            RtError::MethodNotApplicable { method, receiver } => {
                write!(f, "метод «{method}» не применим к «{receiver}»")
            }
            RtError::UnknownMethod { method, receiver } => {
                write!(f, "метод «{method}» не найден у «{receiver}»")
            }
            RtError::DynamicError(msg) => write!(f, "{msg}"),
            RtError::InvalidBytecode(what) => write!(f, "некорректный байт-код: {what}"),
            RtError::IoError(msg) => write!(f, "ошибка файлового ввода-вывода: {msg}"),
            RtError::Link(msg) => write!(f, "ошибка runtime-компонента: {msg}"),
            RtError::Component(error) => {
                write!(f, "{}: {}: {}", error.package, error.kind, error.message)
            }
            RtError::StackOverflow { what } => {
                write!(f, "превышена глубина стека: {what}")
            }
            RtError::CapabilityMissing { capability, path } => {
                let cap = match capability {
                    crate::component::Capability::Stdout => "вывод",
                    crate::component::Capability::Stderr => "поток ошибок",
                    crate::component::Capability::Zone => "часовой пояс",
                    crate::component::Capability::FileSystem => "файловая система",
                    crate::component::Capability::FunctionCaller => "вызов функции модуля",
                    crate::component::Capability::Random => "источник случайности",
                    crate::component::Capability::Network => "сеть",
                    crate::component::Capability::HostPromises => "host-обещания",
                    crate::component::Capability::BackgroundJobs => "фоновые задания",
                    crate::component::Capability::TempStorage => "временное хранилище",
                };
                let path = match path {
                    crate::component::ContextKind::Full => "полного контекста",
                    crate::component::ContextKind::Reduced => "сокращённого контекста",
                };
                write!(f, "возможность «{cap}» недоступна на этом пути ({path})")
            }
        }
    }
}

impl std::error::Error for RtError {}

pub type RtResult<T> = Result<T, RtError>;

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

/// `TypeId` перечисления, к которому принадлежит член, — используется и
/// для `ТипЗнч()` конкретного члена (`BslValue::Enum`), и для голого имени
/// перечисления как выражения (`BslValue::EnumType`, см. doc comment на
/// самом варианте).
fn enum_kind_type_id(kind: EnumKind) -> TypeId {
    match kind {
        EnumKind::JsonValueType => TypeId::JsonValueType,
        EnumKind::JsonLineBreak => TypeId::JsonLineBreak,
        EnumKind::JsonEscapeCharacters => TypeId::JsonEscapeCharacters,
        EnumKind::JsonDateFormat => TypeId::JsonDateFormat,
        EnumKind::JsonDateWritingVariant => TypeId::JsonDateWritingVariant,
        EnumKind::XmlNodeType => TypeId::XmlNodeType,
        EnumKind::DomNodeType => TypeId::DomNodeType,
        EnumKind::SpreadFileType => TypeId::SpreadFileType,
        EnumKind::DrawingKind => TypeId::DrawingKind,
        EnumKind::PageOrientation => TypeId::PageOrientation,
        EnumKind::TextEncoding => TypeId::TextEncoding,
        EnumKind::StringEncodingMethod => TypeId::StringEncodingMethod,
        EnumKind::SortDirection => TypeId::SortDirection,
        EnumKind::BackgroundJobState => TypeId::BackgroundJobState,
        EnumKind::MessageStatus => TypeId::MessageStatus,
        EnumKind::ErrorCategory => TypeId::ErrorCategory,
        EnumKind::DateFractions => TypeId::DateFractions,
        EnumKind::HashFunction => TypeId::HashFunction,
        EnumKind::ByteOrder => TypeId::ByteOrder,
        EnumKind::FileOpenMode => TypeId::FileOpenMode,
        EnumKind::FileAccess => TypeId::FileAccess,
        EnumKind::StreamPosition => TypeId::StreamPosition,
        EnumKind::XsComponentType => TypeId::XsComponentType,
        EnumKind::XsForm => TypeId::XsForm,
        EnumKind::XsSimpleTypeVariety => TypeId::XsSimpleTypeVariety,
        EnumKind::XsModelGroupKind => TypeId::XsModelGroupKind,
        EnumKind::XsDerivationMethod => TypeId::XsDerivationMethod,
        EnumKind::XsValueConstraint => TypeId::XsValueConstraint,
        EnumKind::XsWhitespaceHandling => TypeId::XsWhitespaceHandling,
        EnumKind::XmlForm => TypeId::XmlForm,
        EnumKind::XdtoFacetKind => TypeId::XdtoFacetKind,
        EnumKind::DomXPathResultType => TypeId::DomXPathResultType,
        EnumKind::SearchDirection => TypeId::SearchDirection,
        EnumKind::ZipRestorePathsMode => TypeId::ZipRestorePathsMode,
        EnumKind::PdfAttachmentRelation => TypeId::PdfAttachmentRelation,
        EnumKind::ArchiveFileType => TypeId::ArchiveFileType,
        EnumKind::ZipCompressionMethod => TypeId::ZipCompressionMethod,
        EnumKind::ZipCompressionLevel => TypeId::ZipCompressionLevel,
        EnumKind::ZipStorePathMode => TypeId::ZipStorePathMode,
        EnumKind::ZipSubDirProcessingMode => TypeId::ZipSubDirProcessingMode,
        EnumKind::ZipEncryptionMethod => TypeId::ZipEncryptionMethod,
        EnumKind::ZipFileNamesEncoding => TypeId::ZipFileNamesEncoding,
        EnumKind::ByteOrderMarkUse => TypeId::ByteOrderMarkUse,
    }
}

fn optional_positive_usize(value: &BslValue, default: usize, op: &'static str) -> RtResult<usize> {
    if matches!(value, BslValue::Undefined) {
        return Ok(default);
    }
    let BslValue::Number(number) = value else {
        return Err(RtError::TypeError {
            expected: "Положительное целое число",
            op,
        });
    };
    number
        .to_i64_exact()
        .and_then(|number| usize::try_from(number).ok())
        .filter(|number| *number != 0)
        .ok_or(RtError::TypeError {
            expected: "Положительное целое число",
            op,
        })
}

impl BslValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            BslValue::Undefined => "Неопределено",
            BslValue::Null => "Null",
            BslValue::Boolean(_) => "Булево",
            BslValue::Number(_) => "Число",
            BslValue::Str(_) => "Строка",
            BslValue::Date(_) => "Дата",
            BslValue::Type(_) => "Тип",
            BslValue::Enum(e) => e.enum_name(),
            // ИЗМЕРЕНО (проба `JSON.ENUM.BARE_NAME`): голое имя
            // перечисления печатается МЕТАТИПОМ — `Перечисление` плюс
            // русское написание, слитно.
            BslValue::EnumType(k) => k.meta_ru_name(),
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.type_descriptor().name,
                BslObject::Array(_) => "Массив",
                BslObject::Structure(_) => "Структура",
                // Служебное имя ЭТОЙ реализации: в 1С такое значение всегда
                // имеет настоящий тип (ссылка, список значений, ...), но
                // без базы и без реверса разметки материализовать его нечем.
                BslObject::VstrOpaque(_) => "НепрозрачноеЗначение",
                BslObject::ValueTable(_) => "ТаблицаЗначений",
                BslObject::TableColumns(_) => "КоллекцияКолонокТаблицыЗначений",
                BslObject::TableColumn(..) => "КолонкаТаблицыЗначений",
                BslObject::TableRow(_, _) => "СтрокаТаблицыЗначений",
                BslObject::TypeDescription(_) => "ОписаниеТипов",
                BslObject::ValueComparison => "СравнениеЗначений",
                BslObject::Map(_) => "Соответствие",
                BslObject::KeyValuePair(_, _) => "КлючИЗначение",
                BslObject::TextWriter(_) => "ЗаписьТекста",
                // У двоичных данных имя ЗНАЧЕНИЯ платформой не наблюдаемо:
                // `Строка(ДД)` отдаёт дамп байтов, а не имя (измерено,
                // проба `BIN.STR`). Эта строка живёт только в
                // диагностике самой реализации — в тексте `RtError`, — и
                // написана слитно по образцу соседей.
                BslObject::BinaryData(_) => "ДвоичныеДанные",
                // А вот у БУФЕРА имя значения наблюдаемо и измерено:
                // `Строка(Буфер)` печатает именно «БуферДвоичныхДанных»
                // (слитно), в отличие от имени типа «Буфер двоичных
                // данных» в `types.rs` и в отличие от соседа сверху,
                // который печатается дампом байтов.
                BslObject::BinaryBuffer(_) => "БуферДвоичныхДанных",
                // Имя ЗНАЧЕНИЯ здесь не наблюдаемо: `Строка(УИД)` печатает
                // саму каноническую форму, а не имя (фикстура `uuid`).
                // Строка ниже живёт в диагностике `RtError`.
                BslObject::Uuid(_) => "УникальныйИдентификатор",
            },
        }
    }

    fn as_number(&self, op: &'static str) -> RtResult<&BslNumber> {
        match self {
            BslValue::Number(n) => Ok(n),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op,
            }),
        }
    }

    /// `+` между двумя строками — конкатенация (реальная 1С считает это
    /// перегрузкой того же оператора, не отдельной функцией). Любая другая
    /// комбинация типов идёт по числовому пути и получает его же ошибку
    /// типа, если не подходит.
    pub fn add(&self, other: &Self) -> RtResult<Self> {
        if let (BslValue::Str(a), BslValue::Str(b)) = (self, other) {
            return Ok(BslValue::Str(a.concat(b)));
        }
        // `Дата + Число` — сдвиг на N СЕКУНД (не дней: разрешение типа —
        // секунда, и `Дата - Дата` симметрично отдаёт секунды).
        if let BslValue::Date(d) = self {
            let secs = Self::whole_seconds(other, "+")?;
            return Self::shifted(*d, secs, "+");
        }
        Ok(BslValue::Number(
            self.as_number("+")?.add(other.as_number("+")?)?,
        ))
    }

    /// `%` — остаток от деления. Дат это не касается: `Дата % Число`
    /// платформе неизвестно так же, как и нам, и общая числовая ветка ниже
    /// отвергнет её сама.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если операнд не число, либо
    /// [`RtError::Num`] при делении на ноль.
    pub fn rem(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("%")?.rem(other.as_number("%")?)?,
        ))
    }

    pub fn sub(&self, other: &Self) -> RtResult<Self> {
        if let BslValue::Date(a) = self {
            return match other {
                // `Дата - Дата` -> Число секунд между ними.
                BslValue::Date(b) => Ok(BslValue::Number(BslNumber::from_i64(a.diff_seconds(*b)))),
                // `Дата - Число` -> Дата.
                _ => {
                    let secs = Self::whole_seconds(other, "-")?;
                    Self::shifted(*a, -secs, "-")
                }
            };
        }
        Ok(BslValue::Number(
            self.as_number("-")?.sub(other.as_number("-")?)?,
        ))
    }

    /// Слагаемое к дате обязано быть ЦЕЛЫМ числом секунд: у типа нет
    /// разрешения мельче секунды, и тихо отбрасывать дробную часть значит
    /// делать `Дата + 0.4` неотличимым от `Дата + 0`.
    fn whole_seconds(v: &Self, op: &'static str) -> RtResult<i64> {
        v.as_number(op)?.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Число (целое количество секунд)",
            op,
        })
    }

    /// Выход за границы `0001-01-01 .. 9999-12-31` — ошибка, а не тихое
    /// заворачивание в другой конец диапазона.
    fn shifted(d: BslDate, secs: i64, op: &'static str) -> RtResult<Self> {
        d.shift_seconds(secs)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op })
    }

    pub fn mul(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("*")?.mul(other.as_number("*")?)?,
        ))
    }

    pub fn div(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("/")?.div(other.as_number("/")?)?,
        ))
    }

    /// Специализированный шаг числового `Для`, сохраняющий проверку типов,
    /// если тело цикла переприсвоило переменную-счётчик.
    #[inline]
    pub fn increment_numeric_for_and_le(&mut self, bound: &Self) -> RtResult<bool> {
        let bound = bound.as_number("Для")?;
        match self {
            BslValue::Number(counter) => Ok(counter.increment_and_le(bound)?),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op: "Для",
            }),
        }
    }

    pub fn neg(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("унарный -")?.neg()))
    }

    /// `Не` приводит операнд по тем же правилам, что и любое другое
    /// условие: `Не 1` даёт «Нет», а `Не "Ложь"` — «Да» (измерено,
    /// `COND.NOT_NUMBER` и `COND.NOT_STRING`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если операнд к условию не приводится.
    pub fn not(&self) -> RtResult<Self> {
        Ok(BslValue::Boolean(!self.as_condition()?))
    }

    // --- Трансцендентные функции (через f64 в bsl-number) ------------------

    pub fn sqrt(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sqrt")?.sqrt()?))
    }

    pub fn pow(&self, exp: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("Pow")?.pow(exp.as_number("Pow")?)?,
        ))
    }

    pub fn ln(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log")?.ln()?))
    }

    pub fn log10(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log10")?.log10()?))
    }

    pub fn exp(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Exp")?.exp()?))
    }

    /// `Окр(Число, ЧислоРазрядов, Режим)` — decimal, НЕ через `f64`:
    /// `Окр(2.675, 2)` обязан дать `2.68`, а не `2.67` (ближайший `f64` к
    /// `2.675` чуть меньше самого числа).
    ///
    /// Все три аргумента здесь всегда есть: недостающие подставляет
    /// `bsl-sema::resolver::resolve_call` литеральным `0` (см. там же, почему
    /// не вариативная арность).
    ///
    /// Кодировка режимов ИЗМЕРЕНА на платформе: `0` — половина к нулю,
    /// `1` — половина от нуля, опущенный аргумент — как `1`, а НЕ как `0`.
    ///
    /// Неизвестный код (`Окр(2.5, 0, 7)`) платформа не считает ошибкой и
    /// округляет по умолчанию — измерено, поэтому здесь тоже не ошибка.
    /// Раньше тут стояло исключение, и это было расхождение.
    pub fn round(&self, digits: &Self, mode: &Self) -> RtResult<Self> {
        let n = self.as_number("Окр")?;
        // Опущенное число разрядов — ноль (`Окр(2.5)` округляет до целого).
        let scale = match digits {
            BslValue::Undefined => 0,
            other => Self::round_arg_as_i32(other)?,
        };
        // Опущенный третий аргумент приходит `Неопределено` (см.
        // `bsl-sema::resolver::resolve_call`): подставлять вместо него `0`
        // нельзя — это ДРУГОЙ режим.
        let mode = match mode {
            BslValue::Undefined => bsl_number::DEFAULT_ROUND_MODE,
            other => match Self::round_arg_as_i32(other)? {
                0 => bsl_number::RoundMode::HalfDown,
                1 => bsl_number::RoundMode::HalfUp,
                _ => bsl_number::DEFAULT_ROUND_MODE,
            },
        };
        Ok(BslValue::Number(match mode {
            bsl_number::RoundMode::HalfUp => n.round_to_scale(scale),
            bsl_number::RoundMode::HalfDown => n.round_to_scale_half_down(scale),
        }))
    }

    fn round_arg_as_i32(v: &Self) -> RtResult<i32> {
        v.as_number("Окр")?
            .to_i64_exact()
            .and_then(|s| i32::try_from(s).ok())
            .ok_or(RtError::TypeError {
                expected: "Число (целое)",
                op: "Окр",
            })
    }

    /// `Цел(Число)` — отбрасывание дробной части К НУЛЮ (не half-up, в
    /// отличие от `Окр` выше): `Цел(2.9) = 2`, `Цел(-2.9) = -2`.
    pub fn trunc(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Цел")?.trunc_to_scale(0)))
    }

    pub fn sin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sin")?.sin()?))
    }

    pub fn cos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Cos")?.cos()?))
    }

    pub fn tan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Tan")?.tan()?))
    }

    pub fn asin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ASin")?.asin()?))
    }

    pub fn acos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ACos")?.acos()?))
    }

    pub fn atan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ATan")?.atan()?))
    }

    /// `И`/`ИЛИ` в 1С короткозамкнутые, поэтому у `BslValue` больше нет
    /// `and`/`or`: комбинирование живёт в потоке управления кодогена
    /// (`Instr::JumpIfFalse`/`JumpIfTrue` в `bsl-bytecode::compiler`), а не
    /// здесь, — правый операнд физически не вычисляется, если левый уже
    /// решил результат.
    /// Условие приводится к булеву, и это ИЗМЕРЕНО, а не выведено: до
    /// замеров здесь стояла строгая булевость с комментарием «никакой
    /// truthiness», и она оказалась неверной. Платформа принимает
    ///
    /// * булево — как есть;
    /// * ЧИСЛО: ноль — ложь, любое другое (в том числе отрицательное и
    ///   дробное) — истина (замеры `TERNARY.CONDITION_ZERO`,
    ///   `TERNARY.CONDITION_NEGATIVE`, `COND.IF_NUMBER_ONE`);
    /// * СТРОКУ — но только словами, и это не «непустая строка истинна»:
    ///   «абв», «0» и «1» отвергаются наравне с пустой (замеры
    ///   `TERNARY.CONDITION_WORD_OTHER`, `..._STRING_ZERO`, `..._STRING_ONE`).
    ///
    /// Всё остальное — `Неопределено`, `Null`, дата, коллекция — ошибка.
    /// Правило одно на все условия языка: `Если`, `Пока`, `И`, `ИЛИ`, `Не`
    /// и `?()` идут сюда же, и на платформе они тоже ведут себя одинаково
    /// (замеры `COND.*`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если значение к условию не приводится.
    pub fn as_condition(&self) -> RtResult<bool> {
        match self {
            BslValue::Boolean(b) => Ok(*b),
            BslValue::Number(n) => Ok(!n.is_zero()),
            BslValue::Str(s) => condition_word(&s.to_string()).ok_or(RtError::TypeError {
                expected: "Булево",
                op: "Условие",
            }),
            _ => Err(RtError::TypeError {
                expected: "Булево",
                op: "Условие",
            }),
        }
    }

    /// Сравнение по значению для `=`/`<>`. Разнотипные значения просто не
    /// равны — это не ошибка (в отличие от `<`/`>`/... между разными
    /// типами). `Массив`/`Структура` — ссылочные типы: равенство — это
    /// тождество объекта (`Rc::ptr_eq`), а не структурное сравнение
    /// содержимого.
    ///
    /// ОДНО исключение, и оно измерено: булево сравнивается с числом
    /// ЧИСЛЕННО, `Истина` как единица, `Ложь` как ноль. Именно численно, а
    /// не «по истинности»: `Истина = 2` — ЛОЖЬ (замер `EQ.BOOL.TRUE_TWO`).
    /// Строка при этом не приводится никак: `"5" = 5` ложно.
    ///
    /// Поэтому же эта функция БОЛЬШЕ НЕ совпадает с [`PartialEq`]: там
    /// отношение строже, и так и должно быть. Платформа различает эти два
    /// отношения — измерено на самом наблюдаемом их следствии: `Истина` и
    /// `1` остаются РАЗНЫМИ ключами `Соответствие` (замеры
    /// `EQ.MAP.BOOL_KEY_BY_NUMBER` и `EQ.MAP.BOTH_KEYS`), и `Найти` булево
    /// по единице тоже не находит. Ослабить заодно `PartialEq` было бы
    /// нельзя ещё и технически: с ним согласован `Hash`, и соответствие
    /// сломалось бы молча.
    pub fn eq_value(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Boolean(b), BslValue::Number(n))
            | (BslValue::Number(n), BslValue::Boolean(b)) => {
                *n == BslNumber::from_i64(i64::from(*b))
            }
            _ => self == other,
        }
    }

    /// Сравнение строк — упорядочивание код-юнитов UTF-16, без учёта
    /// локали (настоящая коллация для `Сортировать` в `ТаблицаЗначений` —
    /// отдельная, ещё не сделанная задача).
    pub fn compare(&self, other: &Self, op: &'static str) -> RtResult<Ordering> {
        match (self, other) {
            (BslValue::Number(a), BslValue::Number(b)) => Ok(a.cmp(b)),
            (BslValue::Str(a), BslValue::Str(b)) => Ok(a.cmp(b)),
            // Даты сравниваются по значению (моменту времени) — секунды от
            // общей эпохи, поэтому это обычное сравнение `i64`.
            (BslValue::Date(a), BslValue::Date(b)) => Ok(a.cmp(b)),
            _ => Err(RtError::TypeError {
                expected: "Число, Строка или Дата",
                op,
            }),
        }
    }

    fn as_str(&self, op: &'static str) -> RtResult<&BslString> {
        match self {
            BslValue::Str(s) => Ok(s),
            _ => Err(RtError::TypeError {
                expected: "Строка",
                op,
            }),
        }
    }

    fn as_usize(&self, op: &'static str) -> RtResult<usize> {
        let n = self.as_number(op)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    // --- Строки ---------------------------------------------------------

    pub fn str_len(&self) -> RtResult<usize> {
        Ok(self.as_str("СтрДлина")?.len_utf16())
    }

    pub fn str_left(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(
            self.as_str("Лев")?.left(len.as_usize("Лев")?),
        ))
    }

    pub fn str_right(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(
            self.as_str("Прав")?.right(len.as_usize("Прав")?),
        ))
    }

    /// `Сред(Строка, Начало[, Длина])` — `Неопределено` на месте длины
    /// (аргумент опущен, см. `BuiltinFn::arity_range`) значит "до конца
    /// строки".
    pub fn str_mid(&self, start: &Self, len: &Self) -> RtResult<Self> {
        let s = self.as_str("Сред")?;
        let start = start.as_usize("Сред")?;
        let len = match len {
            BslValue::Undefined => s.len_utf16(),
            other => other.as_usize("Сред")?,
        };
        Ok(BslValue::Str(s.substring(start, len)))
    }

    fn string_for_case(&self, op: &'static str) -> RtResult<BslString> {
        match self {
            BslValue::Str(value) => Ok(value.clone()),
            // ИЗМЕРЕНО(STRING.LOWER.UUID.VALUE,
            // STRING.UPPER.UUID.VALUE): функции регистра принимают UUID и
            // сначала используют его каноническую строковую форму.
            BslValue::Object(object) => match &**object {
                BslObject::Uuid(bytes) => Ok(BslString::from_utf8_string(uuid::format(bytes))),
                _ => Err(RtError::TypeError {
                    expected: "Строка или УникальныйИдентификатор",
                    op,
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Строка или УникальныйИдентификатор",
                op,
            }),
        }
    }

    pub fn str_upper(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.string_for_case("ВРег")?.to_uppercase()))
    }

    pub fn str_lower(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.string_for_case("НРег")?.to_lowercase()))
    }

    pub fn str_trim_all(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛП")?.trim()))
    }

    pub fn str_trim_left(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛ")?.trim_start()))
    }

    pub fn str_trim_right(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрП")?.trim_end()))
    }

    /// `СтрНайти(Строка, Подстрока)` — позиция в КОД-ЮНИТАХ UTF-16, 1-based,
    /// `0` если не найдено. Те же единицы, что считает `СтрДлина`, чтобы
    /// результат можно было передать в `Сред`/`Лев` без пересчёта.
    pub fn str_find(
        &self,
        needle: &Self,
        direction: &Self,
        start: &Self,
        occurrence: &Self,
    ) -> RtResult<Self> {
        let text = self.as_str("СтрНайти")?;
        let needle = needle.as_str("СтрНайти")?;
        if needle.units().is_empty() || needle.units().len() > text.units().len() {
            return Ok(BslValue::number_from_i64(0));
        }
        let from_end = match direction {
            BslValue::Undefined | BslValue::Enum(EnumValue::SearchFromBegin) => false,
            BslValue::Enum(EnumValue::SearchFromEnd) => true,
            _ => {
                return Err(RtError::TypeError {
                    expected: "НаправлениеПоиска",
                    op: "СтрНайти",
                });
            }
        };
        let default_start = if from_end { text.units().len() } else { 1 };
        let start = optional_positive_usize(start, default_start, "СтрНайти")?;
        let occurrence = optional_positive_usize(occurrence, 1, "СтрНайти")?;
        if start > text.units().len() {
            return Ok(BslValue::number_from_i64(0));
        }

        let last = text.units().len() - needle.units().len();
        let matches = |position: &usize| {
            text.units()[*position..*position + needle.units().len()] == *needle.units()
        };
        let position = if from_end {
            (0..=last)
                .rev()
                .filter(|position| *position < start)
                .filter(matches)
                .nth(occurrence - 1)
        } else {
            (start - 1..=last).filter(matches).nth(occurrence - 1)
        };
        Ok(BslValue::number_from_i64(
            position.map_or(0, |position| position as i64 + 1),
        ))
    }

    pub fn str_starts_with(&self, prefix: &Self) -> RtResult<Self> {
        let text = self.as_str("СтрНачинаетсяС")?;
        let prefix = prefix.as_str("СтрНачинаетсяС")?;
        if prefix.units().is_empty() {
            return Err(RtError::TypeError {
                expected: "Непустая строка",
                op: "СтрНачинаетсяС",
            });
        }
        Ok(BslValue::Boolean(text.units().starts_with(prefix.units())))
    }

    pub fn str_ends_with(&self, suffix: &Self) -> RtResult<Self> {
        let text = self.as_str("СтрЗаканчиваетсяНа")?;
        let suffix = suffix.as_str("СтрЗаканчиваетсяНа")?;
        if suffix.units().is_empty() {
            return Err(RtError::TypeError {
                expected: "Непустая строка",
                op: "СтрЗаканчиваетсяНа",
            });
        }
        Ok(BslValue::Boolean(text.units().ends_with(suffix.units())))
    }

    pub fn str_replace(&self, from: &Self, to: &Self) -> RtResult<Self> {
        let uuid_text;
        let text = match self {
            BslValue::Str(text) => text,
            BslValue::Object(object) => match &**object {
                BslObject::Uuid(bytes) => {
                    uuid_text = BslString::from_utf8_string(uuid::format(bytes));
                    &uuid_text
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка или УникальныйИдентификатор",
                        op: "СтрЗаменить",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Строка или УникальныйИдентификатор",
                    op: "СтрЗаменить",
                });
            }
        };
        Ok(BslValue::Str(text.replace(
            from.as_str("СтрЗаменить")?,
            to.as_str("СтрЗаменить")?,
        )))
    }

    /// `СтрРазделить(Строка, Разделитель[, ВключатьПустые])` -> `Массив`.
    pub fn str_split(&self, sep: &Self, include_empty: &Self) -> RtResult<Self> {
        let include_empty = match include_empty {
            BslValue::Undefined => true,
            BslValue::Boolean(value) => *value,
            _ => {
                return Err(RtError::TypeError {
                    expected: "Булево",
                    op: "СтрРазделить",
                });
            }
        };
        let parts = self
            .as_str("СтрРазделить")?
            .split(sep.as_str("СтрРазделить")?)
            .into_iter()
            .filter(|part| include_empty || !part.units().is_empty());
        Ok(BslValue::new_array(parts.map(BslValue::Str).collect()))
    }

    /// `СтрСоединить(Массив, Разделитель)`. Не-строковые элементы массива
    /// приводятся к строке через `Display` — у 1С `СтрСоединить` тоже
    /// принимает массив любых значений, а не только строк. ВНИМАНИЕ:
    /// приведение здесь идёт МИМО `bsl-format` (этот крейт ниже него
    /// слоем), поэтому число получает каноническую форму, а не
    /// локализованную с NBSP-группировкой — см. `Display for BslNumber`.
    /// Это осознанное расхождение уровня слоёв, а не забытая локализация:
    /// массив строк (обычный случай) оно не затрагивает вовсе.
    pub fn str_join(&self, sep: &Self) -> RtResult<Self> {
        let sep = sep.as_str("СтрСоединить")?;
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let parts: Vec<BslString> = items
                        .borrow()
                        .iter()
                        .map(|v| match v {
                            BslValue::Str(s) => s.clone(),
                            other => BslString::from_str(&other.to_string()),
                        })
                        .collect();
                    Ok(BslValue::Str(BslString::join(&parts, sep)))
                }
                _ => Err(RtError::TypeError {
                    expected: "Массив",
                    op: "СтрСоединить",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Массив",
                op: "СтрСоединить",
            }),
        }
    }

    pub fn str_line_count(&self) -> RtResult<Self> {
        let n = self.as_str("СтрЧислоСтрок")?.line_count();
        Ok(BslValue::Number(BslNumber::from_i64(n as i64)))
    }

    pub fn str_get_line(&self, n: &Self) -> RtResult<Self> {
        let s = self.as_str("СтрПолучитьСтроку")?;
        let n = n.as_usize("СтрПолучитьСтроку")?;
        Ok(BslValue::Str(s.line_at(n)))
    }

    /// `СтрШаблон(Шаблон, З1, ..., З10)` — значения уже дополнены до
    /// `MAX_TEMPLATE_ARGS` штук `Неопределено` резолвером (см.
    /// `BuiltinFn::arity_range`); хвостовые `Неопределено` отбрасываются
    /// здесь, чтобы `%3` при двух реально переданных значениях дал пусто, а
    /// не строковое представление `Неопределено` (оно, впрочем, тоже
    /// пустое — но полагаться на это совпадение не нужно).
    pub fn str_template(&self, values: &[Self]) -> RtResult<Self> {
        let tmpl = self.as_str("СтрШаблон")?;
        let end = values
            .iter()
            .rposition(|v| !matches!(v, BslValue::Undefined))
            .map(|i| i + 1)
            .unwrap_or(0);
        let vals: Vec<BslString> = values[..end]
            .iter()
            .map(|v| match v {
                BslValue::Str(s) => s.clone(),
                other => BslString::from_str(&other.to_string()),
            })
            .collect();
        Ok(BslValue::Str(tmpl.template(&vals)))
    }

    /// `Символ(Код)`.
    /// ИЗМЕРЕНО на 8.3.27: `Символ(128512)` возвращает ПУСТУЮ строку, а не
    /// суррогатную пару и не ошибку — платформа за пределы BMP не выходит.
    /// `Символ(65535)` при этом даёт строку длины 1, а `Символ(65)` — «A».
    pub fn char_from_code(&self) -> RtResult<Self> {
        let code = self.as_number("Символ")?;
        let code = code.to_i64_exact().and_then(|c| u32::try_from(c).ok());
        let text = match code {
            Some(c) if c <= 0xFFFF => BslString::from_char_code(c),
            // Астральный код — пустая строка, как на платформе.
            Some(_) => Some(BslString::from_str("")),
            None => None,
        };
        Ok(BslValue::Str(text.ok_or(RtError::TypeError {
            expected: "Код символа (целое в диапазоне Unicode)",
            op: "Символ",
        })?))
    }

    /// `КодСимвола(Строка[, Позиция])` — позиция по умолчанию `1`.
    pub fn char_code(&self, pos: &Self) -> RtResult<Self> {
        let s = self.as_str("КодСимвола")?;
        let pos = match pos {
            BslValue::Undefined => 1,
            other => other.as_usize("КодСимвола")?,
        };
        // ИЗМЕРЕНО: `КодСимвола("")` даёт -1, а не ошибку. Позиция за
        // границей непустой строки замером не покрыта — трактуем так же,
        // потому что «за границей» тут ровно тот же случай.
        let code = s.char_code_at(pos).map_or(-1, |c| c as i64);
        Ok(BslValue::Number(BslNumber::from_i64(code)))
    }

    // --- Даты -------------------------------------------------------------

    fn as_date(&self, op: &'static str) -> RtResult<BslDate> {
        match self {
            BslValue::Date(d) => Ok(*d),
            _ => Err(RtError::TypeError {
                expected: "Дата",
                op,
            }),
        }
    }

    /// Компонента `Дата(...)` — целое число, помещающееся в календарный
    /// диапазон. Нецелое (`Дата(2024.5, 1, 1)`) — ошибка, а не усечение.
    fn date_part(v: &Self, op: &'static str) -> RtResult<i64> {
        v.as_number(op)?.to_i64_exact().ok_or(RtError::TypeError {
            expected: "Число (целое)",
            op,
        })
    }

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])` — шестиместная
    /// форма, и `Дата("ГГГГММДД[ЧЧММСС]")` — строковая.
    ///
    /// Обе формы живут в одной функции, потому что в 1С это одна и та же
    /// встроенная функция с перегрузкой по типу первого аргумента, а не две
    /// разных. Опущенные `Час`/`Минута`/`Секунда` приходят сюда как
    /// `Неопределено` (см. `BuiltinFn::arity_range`) и означают ноль.
    pub fn make_date(args: &[BslValue]) -> RtResult<Self> {
        // Строковая форма: `Дата("20240115103000")`. Остальные позиции при
        // ней обязаны быть пустыми — `Дата("20240115", 2, 3)` бессмысленно.
        if let BslValue::Str(s) = &args[0] {
            if args[1..].iter().any(|a| !matches!(a, BslValue::Undefined)) {
                return Err(RtError::TypeError {
                    expected: "Дата(«ГГГГММДДЧЧММСС») без остальных аргументов",
                    op: "Дата",
                });
            }
            return BslDate::parse_digits(&s.to_string())
                .map(BslValue::Date)
                .ok_or(RtError::DateOutOfRange { op: "Дата" });
        }

        let year = Self::date_part(&args[0], "Дата")?;
        let month = Self::date_part(&args[1], "Дата")?;
        let day = Self::date_part(&args[2], "Дата")?;
        // Час/минута/секунда необязательны — опущенные значат ноль.
        let mut time = [0i64; 3];
        for (i, slot) in time.iter_mut().enumerate() {
            *slot = match &args[3 + i] {
                BslValue::Undefined => 0,
                other => Self::date_part(other, "Дата")?,
            };
        }
        let fits = |v: i64| u32::try_from(v).ok();
        let built = (|| {
            BslDate::from_civil(
                year,
                fits(month)?,
                fits(day)?,
                fits(time[0])?,
                fits(time[1])?,
                fits(time[2])?,
            )
        })();
        built
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange { op: "Дата" })
    }

    /// `ТекущаяДата()`.
    ///
    /// ОТКЛОНЕНИЕ, о котором надо знать: возвращается момент по UTC, а не
    /// по локальной зоне машины. Дата в 1С — наивный локальный момент без
    /// зоны (см. модуль `date`), а в `std` нет способа узнать смещение
    /// локальной зоны; тащить ради этого `chrono`/`libc` значит тащить
    /// целую модель времени с зонами, которой в типе всё равно нет.
    /// Наблюдаемо это только как сдвиг на смещение зоны, и только у
    /// `ТекущаяДата` — все остальные функции работают с датами, которые им
    /// дали.
    pub fn current_date(env: &mut HostEnv) -> RtResult<Self> {
        let secs = env.unix_millis().div_euclid(1000);
        BslDate::from_seconds(secs + date::UNIX_EPOCH_SECONDS)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяДата"
            })
    }

    /// `ТекущаяУниверсальнаяДата()` — текущий момент UTC как наивная дата
    /// BSL. Источник времени принадлежит прогону и потому подменяется тем
    /// же `Clock`, что и две соседние функции времени.
    pub fn current_universal_date(env: &mut HostEnv) -> RtResult<Self> {
        let secs = env.unix_millis().div_euclid(1000);
        BslDate::from_seconds(secs + date::UNIX_EPOCH_SECONDS)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяУниверсальнаяДата",
            })
    }

    /// `ТекущаяУниверсальнаяДатаВМиллисекундах()` — целое число
    /// миллисекунд от Unix-эпохи в UTC.
    pub fn current_universal_date_in_milliseconds(env: &mut HostEnv) -> RtResult<Self> {
        let millis = env.unix_millis();
        // Отсчёт — от эпохи дат BSL (0001-01-01 UTC), не от 1970-го:
        // ИЗМЕРЕНО на 8.3.27, платформа печатает ~63.9e12.
        let millis = millis
            .checked_add(crate::date::UNIX_EPOCH_SECONDS * 1000)
            .ok_or(RtError::DateOutOfRange {
                op: "ТекущаяУниверсальнаяДатаВМиллисекундах",
            })?;
        Ok(BslValue::Number(BslNumber::from_i64(millis)))
    }

    /// `Год`/`Месяц`/`День`/`Час`/`Минута`/`Секунда`/`ДеньНедели` — все
    /// возвращают `Число`, поэтому одна функция с селектором вместо шести
    /// почти одинаковых.
    pub fn date_component(&self, part: DatePart) -> RtResult<Self> {
        let d = self.as_date(part.op())?;
        let c = d.to_civil();
        let n = match part {
            DatePart::Year => c.year,
            DatePart::Month => c.month as i64,
            DatePart::Day => c.day as i64,
            DatePart::Hour => c.hour as i64,
            DatePart::Minute => c.minute as i64,
            DatePart::Second => c.second as i64,
            DatePart::Weekday => d.weekday() as i64,
        };
        Ok(BslValue::Number(BslNumber::from_i64(n)))
    }

    /// `НачалоДня`/`КонецДня`/`НачалоМесяца`/... — тоже один селектор:
    /// все семь границ отличаются только тем, что округляют.
    pub fn date_boundary(&self, which: DateBoundary) -> RtResult<Self> {
        let d = self.as_date(which.op())?;
        Ok(BslValue::Date(match which {
            DateBoundary::StartOfDay => d.start_of_day(),
            DateBoundary::EndOfDay => d.end_of_day(),
            DateBoundary::StartOfMonth => d.start_of_month(),
            DateBoundary::EndOfMonth => d.end_of_month(),
            DateBoundary::StartOfYear => d.start_of_year(),
            DateBoundary::EndOfYear => d.end_of_year(),
            DateBoundary::StartOfWeek => d.start_of_week(),
        }))
    }

    /// `ДобавитьМесяц(Дата, Количество)` — про зажатие дня см.
    /// `BslDate::add_months` (там же пометка `НЕ ИЗМЕРЕНО(DATE.ADD_MONTH_CLAMP)`).
    pub fn add_month(&self, count: &Self) -> RtResult<Self> {
        let d = self.as_date("ДобавитьМесяц")?;
        let n = Self::date_part(count, "ДобавитьМесяц")?;
        d.add_months(n)
            .map(BslValue::Date)
            .ok_or(RtError::DateOutOfRange {
                op: "ДобавитьМесяц",
            })
    }

    // --- Проверки значения и типа ----------------------------------------

    /// `ЗначениеЗаполнено(Значение)`.
    ///
    /// ИЗМЕРЕНО (из брифа): `Ложь` для `Неопределено`, `Null`, пустой
    /// строки, нуля и пустой даты.
    ///
    /// Всё остальное — три открытых вопроса,
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BOOLEAN)`,
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BLANK_STRING)` и
    /// `НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.EMPTY_COLLECTION)`, каждый со своей веткой
    /// ниже. Ошибиться здесь дорого: это главный потребитель
    /// короткозамкнутых `И`/`ИЛИ` (`Если ЗначениеЗаполнено(Х) И Х.Поле = 1`),
    /// поэтому спорные ветки помечены поимённо, а не «примерно так».
    pub fn is_filled(&self) -> RtResult<bool> {
        Ok(match self {
            BslValue::Undefined | BslValue::Null => false,
            // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BOOLEAN): считается ли `Ложь`
            // незаполненным. Взято
            // «булево заполнено всегда» — иначе `ЗначениеЗаполнено(Флаг)`
            // нельзя было бы отличить от «флага нет вовсе», а именно ради
            // этого различия функция и существует.
            BslValue::Boolean(_) => true,
            BslValue::Number(n) => !n.is_zero(),
            // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.BLANK_STRING): пусты ли строки из
            // одних пробелов. Взято «да,
            // пусты» (сравнение после `СокрЛП`) — это поведение, которого
            // ждут от защитной проверки введённого пользователем текста.
            BslValue::Str(s) => s.trim().len_utf16() > 0,
            // ИЗМЕРЕНО (из брифа): пустая дата не заполнена. Ровно ради
            // этой строчки эпоха и сдвинута на `0001-01-01` — «пусто» это
            // просто ноль, без отдельной константы.
            BslValue::Date(d) => !d.is_empty(),
            // Тип — всегда значение, «пустого типа» не существует.
            BslValue::Type(_) => true,
            // Член перечисления — тоже всегда значение.
            BslValue::Enum(_) => true,
            // Голое имя перечисления — тем же рассуждением: значение есть,
            // «пустого» варианта нет (не измерено отдельно).
            BslValue::EnumType(_) => true,
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.is_filled()?,
                // Непрозрачное значение — всегда «что-то»: судить о его
                // заполненности, не материализуя вид, нечем.
                BslObject::VstrOpaque(_) => true,
                // НЕ ИЗМЕРЕНО(TYPE.IS_FILLED.EMPTY_COLLECTION): пустая
                // коллекция. Взято «пустая — не
                // заполнена» (по числу элементов), потому что для
                // коллекций это единственное содержательное прочтение;
                // альтернатива («объект есть — значит заполнено») делает
                // проверку тождественно истинной и бесполезной.
                BslObject::Array(_)
                | BslObject::Structure(_)
                | BslObject::Map(_)
                | BslObject::ValueTable(_)
                | BslObject::TableColumns(_) => self.collection_len()? > 0,
                // ИЗМЕРЕНО (пробы `BIN.IS_FILLED`/`BIN.EMPTY`): двоичные
                // данные считаются заполненными по ДЛИНЕ — 13 байт «Да»,
                // ноль байт «Нет». Это тот же критерий, что у коллекций,
                // а не «объект есть — значит заполнен».
                BslObject::BinaryData(bytes) => !bytes.is_empty(),
                // У буфера тот же критерий — ПО РАЗМЕРУ, не по содержимому:
                // измерено, что четырёхбайтовый НУЛЕВОЙ буфер считается
                // заполненным («Да»), а буфер нулевого размера — нет.
                BslObject::BinaryBuffer(d) => !d.borrow().is_empty(),
                // ИЗМЕРЕНО (фикстура `uuid`): нулевой идентификатор не
                // заполнен, любой другой — заполнен.
                BslObject::Uuid(b) => *b != [0; 16],
                // У строки таблицы и пары ключ-значение «длины» нет: сам
                // факт существования объекта и есть заполненность.
                BslObject::TableRow(..)
                | BslObject::TableColumn(..)
                | BslObject::TypeDescription(..)
                | BslObject::ValueComparison
                | BslObject::KeyValuePair(..)
                | BslObject::TextWriter(..) => true,
            },
        })
    }

    /// `ТипЗнч(Значение)` -> `Тип`.
    pub fn type_of(&self) -> RtResult<Self> {
        let id = match self {
            BslValue::Undefined => TypeId::Undefined,
            BslValue::Null => TypeId::Null,
            BslValue::Boolean(_) => TypeId::Boolean,
            BslValue::Number(_) => TypeId::Number,
            BslValue::Str(_) => TypeId::String,
            BslValue::Date(_) => TypeId::Date,
            BslValue::Type(_) => TypeId::Type,
            // Тип члена перечисления — само перечисление.
            BslValue::Enum(e) => enum_kind_type_id(e.kind()),
            // ИЗМЕРЕНО (проба `JSON.ENUM.BARE_NAME`): `ТипЗнч()` голого
            // имени перечисления — отдельный МЕТАТИП, печатающийся как
            // `Перечисление<Имя>`, а не тип членов.
            BslValue::EnumType(k) => TypeId::EnumMeta(*k),
            BslValue::Object(o) => match &**o {
                // Тип объекта компонента — его дескриптор, и только он:
                // ни у официального типа, ни у host-типа больше нет
                // строки в закрытом реестре `TypeId` ядра.
                BslObject::Extension(object) => {
                    return Ok(BslValue::Type(TypeRef::Object(object.type_descriptor())));
                }
                BslObject::VstrOpaque(_) => TypeId::VstrOpaque,
                BslObject::Array(_) => TypeId::Array,
                BslObject::Structure(_) => TypeId::Structure,
                BslObject::Map(_) => TypeId::Map,
                BslObject::ValueTable(_) => TypeId::ValueTable,
                BslObject::TableColumns(_) => TypeId::ValueTableColumns,
                BslObject::TableColumn(..) => TypeId::ValueTableColumn,
                BslObject::TableRow(..) => TypeId::ValueTableRow,
                BslObject::TypeDescription(_) => TypeId::TypeDescription,
                BslObject::ValueComparison => TypeId::ValueComparison,
                BslObject::KeyValuePair(..) => TypeId::KeyAndValue,
                BslObject::TextWriter(..) => {
                    return Err(RtError::TypeError {
                        expected: "Зарегистрированный тип",
                        op: "ТипЗнч",
                    });
                }
                BslObject::BinaryData(..) => TypeId::BinaryData,
                BslObject::BinaryBuffer(..) => TypeId::BinaryDataBuffer,
                BslObject::Uuid(..) => TypeId::Uuid,
            },
        };
        Ok(BslValue::Type(TypeRef::Native(id)))
    }

    /// `Тип("ИмяТипа")` -> `Тип`. Неизвестное имя — ошибка (в 1С тоже
    /// исключение, а не `Неопределено`: опечатка в имени типа обязана
    /// падать сразу, а не молча делать сравнение вечно ложным).
    pub fn type_by_name(&self) -> RtResult<Self> {
        let name = self.as_str("Тип")?.to_string();
        // Имя ищется в таблице ядра: типы компонентов пока опознаются
        // через свои `TypeId` там же (см. `TypeRef`).
        TypeId::lookup(&name)
            .map(|id| BslValue::Type(TypeRef::Native(id)))
            .ok_or(RtError::UnknownType(name))
    }

    // --- Коллекции ----------------------------------------------------

    /// Создаёт целое число BSL без прямой зависимости компонента от
    /// внутреннего числового крейта runtime.
    pub fn number_from_i64(value: i64) -> Self {
        Self::Number(BslNumber::from_i64(value))
    }

    /// Заворачивает реализацию статически подключённого компонента в
    /// ссылочное значение BSL.
    pub fn new_object(object: impl ObjectProtocol + 'static) -> Self {
        BslValue::Object(Rc::new(BslObject::Extension(ObjectRef::new(object))))
    }

    /// Создаёт непрозрачное обещание, принадлежащее конкретному запуску.
    #[must_use]
    pub fn new_promise(execution_token: ExecutionToken, promise_id: PromiseId) -> Self {
        Self::new_object(PromiseValue::new(execution_token, promise_id))
    }

    /// Идентификаторы обещания либо `None` для значения другого типа.
    #[must_use]
    pub fn promise_identity(&self) -> Option<(ExecutionToken, PromiseId)> {
        let promise = self.object_ref()?.downcast_ref::<PromiseValue>()?;
        Some((promise.execution_token(), promise.promise_id()))
    }

    /// Возвращает расширяемый объект, не раскрывая legacy-представление.
    pub fn object_ref(&self) -> Option<&ObjectRef> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::Extension(object) => Some(object),
                _ => None,
            },
            _ => None,
        }
    }

    /// Возвращает байтовую потоковую возможность внешнего объекта.
    pub fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        self.object_ref()?.byte_stream()
    }

    /// Байты `ДвоичныеДанные` без копирования.
    pub fn binary_data_bytes(&self) -> Option<&[u8]> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryData(bytes) => Some(bytes),
                _ => None,
            },
            _ => None,
        }
    }

    /// Создаёт `БуферДвоичныхДанных` с готовыми байтами и малым порядком.
    pub fn binary_buffer_of(bytes: Vec<u8>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryBuffer(Rc::new(
            std::cell::RefCell::new(bindata::BinBufData::new(bytes, bindata::ByteOrder::Little)),
        ))))
    }

    /// Размер буфера либо `None` для значения другого типа.
    pub fn binary_buffer_len(&self) -> Option<usize> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().len()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Снимок байтов буфера.
    pub fn binary_buffer_bytes(&self) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().to_vec()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Копирует ограниченный отрезок буфера; позиция за концом даёт пустой
    /// отрезок, как чтение потока.
    pub fn binary_buffer_slice(&self, offset: u64, count: usize) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().with_bytes(|bytes| {
                    let Ok(start) = usize::try_from(offset) else {
                        return Vec::new();
                    };
                    let start = start.min(bytes.len());
                    let end = start.saturating_add(count).min(bytes.len());
                    bytes[start..end].to_vec()
                })),
                _ => None,
            },
            _ => None,
        }
    }

    /// Записывает отрезок в существующий буфер без изменения его размера.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для не-буфера или ошибку границ, если отрезок
    /// не помещается в буфер.
    pub fn binary_buffer_write(&self, offset: usize, bytes: &[u8]) -> RtResult<()> {
        let buffer = match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => buffer,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "БуферДвоичныхДанных",
                        op: "Записать",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: "Записать",
                });
            }
        };
        let end = offset.checked_add(bytes.len()).ok_or(RtError::BadIndex)?;
        if end > buffer.borrow().len() {
            return Err(RtError::IndexOutOfBounds {
                index: i64::try_from(end).unwrap_or(i64::MAX),
                len: buffer.borrow().len(),
            });
        }
        buffer.borrow().with_bytes_mut(|target| {
            target[offset..end].copy_from_slice(bytes);
        });
        Ok(())
    }

    pub fn new_array(items: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Array(std::cell::RefCell::new(items))))
    }

    pub fn new_structure(shape: Rc<Shape>, slots: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Structure(std::cell::RefCell::new(
            StructureStorage::new(shape, slots),
        ))))
    }

    pub fn new_table() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueTable(ValueTableData::new())))
    }

    /// `Новый ОписаниеТипов("Тип1, Тип2")`.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если аргумент не строка либо содержит имя
    /// незарегистрированного типа.
    /// Имена ищутся там же, где их ищет `Тип("Имя")`: сперва в таблице
    /// ядра, потом среди типов компонентов этого прогона. Без второго
    /// шага `Новый ОписаниеТипов("ЧтениеJSON")` перестал бы работать,
    /// когда компонентные типы ушли из закрытого реестра `TypeId`.
    pub fn new_type_description(names: &BslValue, rt: &RuntimeShapes) -> RtResult<Self> {
        let names = names.as_str("Новый ОписаниеТипов")?.to_string();
        let mut types: Vec<TypeRef> = Vec::new();
        for name in names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let ty = rt
                .resolve_type(name)
                .ok_or_else(|| RtError::UnknownType(name.to_string()))?;
            if !types.contains(&ty) {
                types.push(ty);
            }
        }
        Ok(BslValue::Object(Rc::new(BslObject::TypeDescription(types))))
    }

    pub fn new_value_comparison() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueComparison))
    }

    pub fn new_map() -> Self {
        BslValue::Object(Rc::new(BslObject::Map(std::cell::RefCell::new(
            MapData::new(),
        ))))
    }

    /// `Закрыть()` — полиморфен по получателю: `ЗаписьТекста` сбрасывает
    /// буфер и ничего не возвращает, `ЗаписьJSON` отдаёт накопленный
    /// текст.
    ///
    /// Разведение делает сам объект: общая диспетчеризация метода не знает его
    /// конкретный тип заранее.
    ///
    /// # Errors
    ///
    /// Ошибку ввода-вывода либо неприменимость метода к получателю.
    pub fn close_object(&self) -> RtResult<BslValue> {
        self.text_writer_close()
    }

    /// Создаёт объект `ЗаписьТекста` и открывает файл для буферизованной
    /// записи UTF-8 С МЕТКОЙ ПОРЯДКА БАЙТОВ.
    ///
    /// `ЗаписьТекста` над файловой системой прогона. Файл открывается ЗДЕСЬ,
    /// в конструкторе, и объект дальше держит только `FileHandle` — поэтому
    /// файловая система нужна ему лишь на время построения (BORROW), и VM
    /// передаёт её ссылкой из окружения (`host.env()?.files()`), как уже
    /// делает для `Новый ДвоичныеДанные`.
    ///
    /// BOM — ИЗМЕРЕНО на 8.3.27: файл, созданный `Новый ЗаписьТекста(Путь)`
    /// без прочих аргументов, начинается с `EF BB BF`. Отключается он
    /// шестым аргументом конструктора, которого здесь пока нет. Существующий
    /// файл обрезается до нулевой длины.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файл невозможно создать.
    pub fn new_text_writer_with_files(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ЗаписьТекста")?.to_string();
        // `File::create` = открыть-или-создать с обрезанием.
        let handle = files
            .open(
                &path,
                crate::FileOpenOptions::write(crate::FileCreate::OpenOrCreate).truncate(true),
            )
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        let mut buffered = std::io::BufWriter::new(handle);
        std::io::Write::write_all(&mut buffered, &[0xef, 0xbb, 0xbf])
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::Object(Rc::new(BslObject::TextWriter(
            std::cell::RefCell::new(Some(buffered)),
        ))))
    }

    /// Записывает строку в буфер объекта `ЗаписьТекста`.
    ///
    /// UTF-16-представление [`BslString`] кодируется непосредственно в
    /// UTF-8 без промежуточного [`String`], а перевод строки разворачивается
    /// в CRLF — см. [`BslString::write_utf8_crlf`].
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для нестрокового аргумента, ошибку
    /// применимости для другого объекта либо [`RtError::IoError`] при
    /// записи в закрытый файл или ошибке файловой системы.
    pub fn text_writer_write(&self, text: &BslValue) -> RtResult<Self> {
        let text = text.as_str("Записать")?;
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    let mut writer = writer.borrow_mut();
                    text.write_utf8_crlf(
                        writer
                            .as_mut()
                            .ok_or_else(|| RtError::IoError("файл уже закрыт".to_string()))?,
                    )
                    .map_err(|e| RtError::IoError(e.to_string()))?;
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Записать",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Записать",
                receiver: self.type_name(),
            }),
        }
    }

    /// Сбрасывает буфер и закрывает `ЗаписьТекста`.
    ///
    /// Повторный вызов безопасен и возвращает `Неопределено`.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку применимости для другого объекта либо
    /// [`RtError::IoError`], если буфер не удалось сбросить.
    pub fn text_writer_close(&self) -> RtResult<Self> {
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    let mut slot = writer.borrow_mut();
                    // Сброс НА МЕСТЕ, а писатель снимается только ПОСЛЕ
                    // успеха: прежде `take()` забирал буфер ДО `flush()`, и на
                    // отказе `?` уносил ошибку наружу с уже опустевшим слотом
                    // — накопленный текст терялся, а повторный `Закрыть()`
                    // находил `None` и врал успехом при незаписанном тексте.
                    // Теперь на отказе слот цел, и повторный `Закрыть()`
                    // пробует снова.
                    if let Some(writer) = slot.as_mut() {
                        // Сброс буфера в дескриптор, затем ЯВНОЕ закрытие
                        // дескриптора: оба на `?` оставляют слот целым, так
                        // что повторный `Закрыть()` пробует снова (закон
                        // `close` из ABI-G0). `BufWriter` на `Drop` молча
                        // проглотил бы отказ — здесь он наблюдаем.
                        writer
                            .flush()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                        writer
                            .get_mut()
                            .close()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                    }
                    *slot = None;
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Закрыть",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Закрыть",
                receiver: self.type_name(),
            }),
        }
    }

    /// Двоичные данные из готовых байтов — общий конструктор для чтения
    /// файла, `РазделитьДвоичныеДанные` и `СоединитьДвоичныеДанные`.
    pub fn binary_data_of(bytes: impl Into<Rc<[u8]>>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryData(bytes.into())))
    }

    /// Байты значения `ДвоичныеДанные`; для любого другого значения —
    /// ошибка типа с указанием операции, которая его потребовала.
    fn as_binary_data(&self, op: &'static str) -> RtResult<&Rc<[u8]>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::BinaryData(bytes) => Ok(bytes),
                _ => Err(RtError::TypeError {
                    expected: "ДвоичныеДанные",
                    op,
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op,
            }),
        }
    }

    /// `Новый ДвоичныеДанные(ИмяФайла)` — файл читается ЦЕЛИКОМ в память
    /// сразу, как и на платформе (размер известен немедленно, а `Размер()`
    /// после удаления файла продолжает отвечать).
    ///
    /// Конструктор без аргументов, с числом вместо имени файла и с двумя
    /// аргументами платформа отвергает (пробы `BIN.NEW.NOARG`,
    /// `BIN.NEW.NUMARG`, `BIN.NEW.TWOARGS`) — ровно один строковый
    /// аргумент, и это проверяет резолвер.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файла нет, он недоступен или это каталог (пробы
    /// `BIN.NEW.MISSING`, `BIN.NEW.DIR` — платформа в обоих случаях
    /// бросает исключение).
    pub(crate) fn new_binary_data(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ДвоичныеДанные")?.to_string();
        let bytes = files
            .read(&path)
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::binary_data_of(bytes))
    }

    /// `Новый БуферДвоичныхДанных(Размер[, ПорядокБайтов])`.
    ///
    /// Размер обязателен и фиксирует буфер навсегда — роста у него нет;
    /// байты нулевые, порядок по умолчанию `LittleEndian` (измерено).
    /// Пропущенный второй аргумент приходит сюда как
    /// [`BslValue::Undefined`].
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если размер не целое неотрицательное число
    /// (платформа отвергает `-1`, `2.5` и строку `"4"`, а `0` принимает),
    /// если порядок байтов не член `ПорядокБайтов`, а также если буфер
    /// такого размера не удалось разместить в памяти: отказом это лучше,
    /// чем падением процесса на числе из пользовательского текста.
    pub fn new_binary_buffer(size: &BslValue, order: &BslValue) -> RtResult<Self> {
        bindata::new_binary_buffer(size, order)
    }

    /// `Новый УникальныйИдентификатор([СтрокаЛибоУИД])`. Без аргумента —
    /// случайный идентификатор версии 4, со строкой — разбор канонической
    /// формы `8-4-4-4-12` (регистр цифр безразличен), с другим
    /// идентификатором — равная копия (обе формы измерены фикстурой
    /// `uuid`).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если аргумент не строка, не идентификатор и
    /// не `Неопределено`, либо строка не в канонической форме.
    pub(crate) fn new_uuid(arg: &BslValue, random: &RandomHandle) -> RtResult<Self> {
        let bytes = match arg {
            BslValue::Undefined => {
                let mut bytes = [0u8; 16];
                random.fill(&mut bytes);
                uuid::v4_from_bytes(bytes)
            }
            BslValue::Str(s) => uuid::parse(&s.to_string())?,
            // Конструктор от другого идентификатора платформа принимает и
            // отдаёт равное значение (измерено фикстурой `uuid`).
            BslValue::Object(o) => match &**o {
                BslObject::Uuid(b) => *b,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: "Новый УникальныйИдентификатор",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op: "Новый УникальныйИдентификатор",
                });
            }
        };
        Ok(BslValue::Object(Rc::new(BslObject::Uuid(bytes))))
    }

    /// `ДвоичныеДанные.Размер()` — число байтов.
    ///
    /// # Errors
    ///
    /// [`RtError::MethodNotApplicable`], если получатель — не двоичные
    /// данные.
    pub fn binary_data_size(&self) -> RtResult<Self> {
        let bytes = self
            .as_binary_data("Размер")
            .map_err(|_| RtError::MethodNotApplicable {
                method: "Размер",
                receiver: self.type_name(),
            })?;
        Ok(BslValue::Number(BslNumber::from_i64(bytes.len() as i64)))
    }

    /// `РазделитьДвоичныеДанные(Данные, РазмерЧасти)` -> `Массив` частей.
    ///
    /// ИЗМЕРЕНО на 8.3.27 (пробы `BIN.SPLIT.*`) на 13 байтах: по 5 — три
    /// части 5, 5, 3; по 3 — пять частей 3, 3, 3, 3, 1; по 100 и по
    /// 10 000 000 000 — одна часть целиком; по 10 — две части 10 и 3.
    /// То есть хвост КОРОЧЕ, а не дополняется, и размер части больше
    /// целого — не ошибка.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если первый аргумент не двоичные данные
    /// (проба `BIN.SPLIT.BADARG`) либо размер части не целое положительное
    /// число, влезающее в 64 бита без знака: ноль, отрицательное, дробное
    /// и даже числовая СТРОКА `"5"` платформой отвергнуты (пробы
    /// `BIN.SPLIT.ZERO`, `.NEGATIVE`, `.FRACTIONAL`, `.STRSIZE`), а
    /// верхняя граница снята фикстурой `binary-data` с точностью до
    /// единицы: `2^64-1` принимается, `2^64` — уже ошибка.
    pub fn binary_data_split(&self, part_size: &BslValue) -> RtResult<Self> {
        const OP: &str = "РазделитьДвоичныеДанные";
        let bad_size = || RtError::TypeError {
            expected: "Целое положительное число не больше 2^64-1",
            op: OP,
        };
        let bytes = self.as_binary_data(OP)?;
        let size = part_size.as_number(OP).map_err(|_| bad_size())?;
        if !size.is_integer()
            || size.is_negative()
            || size.is_zero()
            || *size > binary_split_max_part()
        {
            return Err(bad_size());
        }
        // Размер части шире `usize` ошибкой НЕ является, пока он в
        // пределах `2^64-1`: платформа на 10^10 и на `2^64-1` одинаково
        // отдаёт одну часть целиком, и насыщение до `usize::MAX` даёт
        // ровно это.
        let size = size
            .to_i64_exact()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(usize::MAX);
        // ПУСТЫЕ данные — краевой случай, где `chunks` расходится с
        // платформой: он не даёт ни одной части, а платформа отдаёт массив
        // из ОДНОЙ пустой части (измерено фикстурой `binary-data`, строка
        // «разбиение пустых»). Пустое на входе — пустое на выходе, но
        // обёрнутое.
        if bytes.is_empty() {
            return Ok(BslValue::new_array(vec![BslValue::binary_data_of(
                Vec::new(),
            )]));
        }
        Ok(BslValue::new_array(
            bytes
                .chunks(size)
                .map(BslValue::binary_data_of)
                .collect::<Vec<_>>(),
        ))
    }

    /// `СоединитьДвоичныеДанные(Массив)` -> склеенные данные в порядке
    /// массива (ИЗМЕРЕНО, проба `BIN.COMBINE.ORDER`: три элемента по 13
    /// байт дают 39 байт, и дамп идёт в порядке массива).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если аргумент не массив (проба
    /// `BIN.COMBINE.NOTARRAY`) или его элемент — не двоичные данные:
    /// платформа отвергает и строку, и `Неопределено` (пробы
    /// `BIN.COMBINE.BADELEM`, `BIN.COMBINE.UNDEF`). Пустой массив
    /// ошибкой НЕ является — он даёт пустые двоичные данные (проба
    /// `BIN.COMBINE.EMPTY`).
    pub fn binary_data_combine(&self) -> RtResult<Self> {
        const OP: &str = "СоединитьДвоичныеДанные";
        let items = match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => items,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: OP,
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: OP,
                });
            }
        };
        let items = items.borrow();
        let mut out = Vec::new();
        for item in items.iter() {
            out.extend_from_slice(item.as_binary_data(OP)?);
        }
        Ok(BslValue::binary_data_of(out))
    }

    /// Индекс должен быть целым неотрицательным числом — `1С` использует
    /// `Число` для индексов, отдельного целочисленного типа нет.
    fn index_as_usize(idx: &BslValue) -> RtResult<usize> {
        let n = idx.as_number("[]").map_err(|_| RtError::BadIndex)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    /// `names` нужен единственной ветке — `Структура`: её поля хранятся
    /// идентификаторами (`NameId`), а `Для Каждого` обязан отдать ключ
    /// пользовательскому коду СТРОКОЙ (`КлючИЗначение.Ключ`). Тащить сюда
    /// интернер целиком дешевле, чем держать в каждой структуре ещё и
    /// строковые имена рядом с формой.
    pub fn get_index(&self, idx: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.get_index(idx),
                BslObject::Array(v) => {
                    let v = v.borrow();
                    let i = Self::index_as_usize(idx)?;
                    v.get(i).cloned().ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: v.len(),
                    })
                }
                BslObject::ValueTable(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let row_id = {
                        let d = data.borrow();
                        d.row_id_at(i).ok_or(RtError::IndexOutOfBounds {
                            index: i as i64,
                            len: d.row_count(),
                        })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(
                        data.clone(),
                        row_id,
                    ))))
                }
                BslObject::TableColumns(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let name = {
                        let d = data.borrow();
                        d.column_names
                            .get(i)
                            .cloned()
                            .ok_or(RtError::IndexOutOfBounds {
                                index: i as i64,
                                len: d.column_names.len(),
                            })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableColumn(
                        data.clone(),
                        name,
                    ))))
                }
                // `СтрокаТаблицы[Ключ]` — значение ячейки: строковый ключ —
                // имя колонки (тот же путь, что `Строка.Имя` через
                // `get_field_by_name`), числовой — её номер.
                BslObject::TableRow(..) => match idx {
                    BslValue::Str(name) => self.get_field_by_name(&name.to_string()),
                    _ => {
                        let i = Self::index_as_usize(idx)?;
                        let name = match &**o {
                            BslObject::TableRow(data, _) => {
                                let d = data.borrow();
                                d.column_names
                                    .get(i)
                                    .cloned()
                                    .ok_or(RtError::IndexOutOfBounds {
                                        index: i as i64,
                                        len: d.column_names.len(),
                                    })?
                            }
                            _ => unreachable!("вариант проверен объемлющим match"),
                        };
                        self.get_field_by_name(&name)
                    }
                },
                // Строковый индекс — ключ, как в BSL. Числовой пока остаётся
                // ПОЗИЦИОННЫМ: `Для Каждого` компилируется в
                // общий для всех коллекций протокол `CollectionLen` + рост
                // числового индекса `0..len` через эту же функцию (см.
                // `bsl-bytecode::compiler::RStmt::ForEach`) — компилятор не
                // знает на этапе компиляции, что `idx` в рантайме окажется
                // `Соответствие`, и не может эмитить для него другой путь.
                // Доступ ПО КЛЮЧУ у `Соответствие` поэтому сознательно НЕ
                // здесь, а в `.Получить(Ключ)` (`map_get`) — если бы `[]`
                // тоже читал по ключу, `м[0]` было бы неразрешимо
                // неоднозначно между "0-я по счёту пара" и "значение по
                // ключу 0" для карты с целочисленными ключами.
                BslObject::Map(data) if matches!(idx, BslValue::Str(_)) => {
                    Ok(data.borrow().get(idx).unwrap_or(BslValue::Undefined))
                }
                BslObject::Map(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let data = data.borrow();
                    let (k, v) = data.entry_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: data.len(),
                    })?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(k, v))))
                }
                // `Для Каждого КиЗ Из Структура` — тот же протокол, что и у
                // `Соответствие` (`CollectionLen` + позиционный обход), и та
                // же пара `Ключ`/`Значение` на выходе. Порядок — вставки, в
                // обоих режимах хранения (`StructureStorage::entry_at`).
                BslObject::Structure(s) if matches!(idx, BslValue::Str(_)) => {
                    let BslValue::Str(name) = idx else {
                        unreachable!("вариант проверен защитой match")
                    };
                    let text = name.to_string();
                    let field = names
                        .lookup(&text)
                        .ok_or_else(|| RtError::UnknownColumn(text.clone()))?;
                    s.borrow()
                        .get(field)
                        .ok_or_else(|| RtError::UnknownColumn(text))
                }
                BslObject::Structure(s) => {
                    let i = Self::index_as_usize(idx)?;
                    let s = s.borrow();
                    let (n, v) = s.entry_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len: s.len(),
                    })?;
                    let key = names.name(n).ok_or(RtError::UnknownField(n))?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(
                        BslValue::Str(BslString::from_str(key)),
                        v,
                    ))))
                }
                // `Буфер[Позиция]` -> `Число` 0..255. Индекс здесь свой, не
                // общий `index_as_usize`: у буфера дробная позиция не
                // ошибка, а отбрасывается к нулю (измерено).
                BslObject::BinaryBuffer(_) => bindata::get_byte(self, idx),
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn set_index(&self, idx: &BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.set_index(idx, val),
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    let slot = v.get_mut(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    })?;
                    *slot = val;
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().insert(idx.clone(), val);
                    Ok(())
                }
                // Буфер меняется по числовой позиции; соответствие выше —
                // по значению ключа.
                BslObject::BinaryBuffer(_) => bindata::set_byte(self, idx, &val),
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    /// Длина коллекции — используется и `Для Каждого` (компилируется в
    /// индексный цикл поверх этой длины), и `Количество()`.
    pub fn collection_len(&self) -> RtResult<usize> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => object.collection_len(),
                BslObject::Array(v) => Ok(v.borrow().len()),
                BslObject::Structure(s) => Ok(s.borrow().len()),
                BslObject::ValueTable(data) => Ok(data.borrow().row_count()),
                BslObject::TableColumns(data) => Ok(data.borrow().column_names.len()),
                BslObject::TableColumn(..)
                | BslObject::TypeDescription(_)
                | BslObject::ValueComparison => Err(RtError::NotIndexable),
                BslObject::TableRow(..) => Err(RtError::NotIndexable),
                BslObject::Map(data) => Ok(data.borrow().len()),
                BslObject::KeyValuePair(..) => Err(RtError::NotIndexable),
                BslObject::VstrOpaque(_) => Err(RtError::NotIndexable),
                // Число байтов отдаёт `Размер()`, а `Количество()` у этого
                // типа нет вовсе — как нет и обхода `Для Каждого`:
                // двоичные данные не коллекция, доступа к отдельному байту
                // здесь не заведено (он появится с `БуферДвоичныхДанных`).
                BslObject::BinaryData(..) => Err(RtError::NotIndexable),
                // Число байтов буфера отдаёт СВОЙСТВО `Размер`, а
                // `Количество()` платформа на нём отвергает — измерено.
                // (`Для Каждого` по буферу она при этом принимает; обход
                // здесь не заведён, потому что в задачу этого типа он не
                // входит, и своего эталона у него ещё нет.)
                BslObject::BinaryBuffer(..) => Err(RtError::NotIndexable),
                BslObject::Uuid(..) => Err(RtError::NotIndexable),
                BslObject::TextWriter(..) => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn get_field(&self, name: NameId) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => s.borrow().get(name).ok_or(RtError::UnknownField(name)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field(&self, name: NameId, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    if s.borrow_mut().set(name, val) {
                        Ok(())
                    } else {
                        Err(RtError::UnknownField(name))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Рантайм-мутация формы структуры ---------------------------------
    //
    // `Вставить`/`Удалить`/`Свойство` (двухаргументная форма) — в отличие
    // от `get_field`/`set_field` выше, которые лишь ЧИТАЮТ уже готовую
    // форму, эти три меняют её: `ShapeTable` здесь больше не голая таблица
    // компиляции, а рантайм-контекст (`RuntimeShapes`, см. одноимённый
    // модуль), поэтому и подписи ниже берут `&mut ShapeTable`, а не
    // работают в изоляции. Инлайн-кэш `GetProp`/`SetProp` (`Rc::ptr_eq` на
    // `s.shape`) сам заметит смену формы после любой из них — ничего
    // специально инвалидировать не нужно.

    /// `Структура.Вставить(Ключ, Значение)`. Поле уже есть — просто новое
    /// значение на том же слоте, форма не меняется (у 1С `Вставить`
    /// повторного поля — это не ошибка и не дубликат, а перезапись).
    pub fn structure_insert(
        &self,
        field: NameId,
        val: BslValue,
        shapes: &mut ShapeTable,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().insert(field, val, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Удалить(Ключ)`. Поля нет — no-op (симметрично
    /// `MapData::remove`, см. его doc comment): убрать то, чего и так нет,
    /// не повод падать.
    pub fn structure_delete(&self, field: NameId, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().remove(field, shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Свойство(Ключ)` / `Структура.Свойство(Ключ,
    /// ЗначениеПоУмолчанию)`.
    ///
    /// Одноместная форма возвращает `Булево` наличия поля, как платформа.
    /// ОТКЛОНЕНИЕ остаётся только у двухместной формы: настоящий второй
    /// параметр — выходной ПО ССЫЛКЕ, но `CallMethod` пока не несёт
    /// `ArgMode::ByRefLocal`. До появления такого ABI он трактуется как
    /// значение по умолчанию безопасного геттера.
    pub fn structure_property(
        &self,
        field: NameId,
        default: Option<BslValue>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let value = s.borrow().get(field);
                    match default {
                        None => Ok(BslValue::Boolean(value.is_some())),
                        Some(default) => Ok(value.unwrap_or(default)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Очистить()` — сбрасывает набор полей целиком (форма
    /// становится пустой), не только значения на месте: у 1С `Очистить()`
    /// на структуре убирает и сами поля, следующий `Свойство`/`.Х` их уже
    /// не найдёт.
    pub fn structure_clear(&self, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    s.borrow_mut().clear(shapes);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Вставить(Ключ, Значение)`.
    pub fn map_insert(&self, key: BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => {
                    data.borrow_mut().insert(key, val);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Получить(Ключ)` — `Неопределено`, если ключа нет, не
    /// ошибка (соответствует `MapData::get`/реальной 1С).
    pub fn map_get(&self, key: &BslValue) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => Ok(data.borrow().get(key).unwrap_or(BslValue::Undefined)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Снимок пар `Соответствие` в порядке вставки. Нужен компонентам,
    /// которые переводят коллекцию в нейтральный DTO до внешнего эффекта.
    pub fn map_entries(&self) -> RtResult<Vec<(BslValue, BslValue)>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::Map(data) => {
                    let data = data.borrow();
                    Ok((0..data.len())
                        .filter_map(|index| data.entry_at(index))
                        .collect())
                }
                _ => Err(RtError::TypeError {
                    expected: "Соответствие",
                    op: "получение пар соответствия",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Соответствие",
                op: "получение пар соответствия",
            }),
        }
    }

    /// Инлайн-кэш для `GetProp` (см. брифовский план оптимизаций: «слот
    /// хранит (`shape_ptr`, `slot_idx`)»). `cache` — одна ячейка на конкретную
    /// инструкцию в чанке (см. `Chunk::prop_cache` в `bsl-bytecode`),
    /// живёт между исполнениями этой инструкции. Промах — обычный поиск
    /// по `Shape::index` плюс запись в кэш; форма меняется редко (обычно
    /// вообще никогда для данной инструкции — иначе откуда там структура
    /// другой формы), так что кэш почти всегда мономорфный.
    ///
    /// Держим `Rc<Shape>` целиком, а не голый указатель: так кэш не может
    /// протухнуть на чужой адрес, если форма где-то освободится — он сам
    /// продлевает ей жизнь, пока висит в кэше.
    ///
    /// Словарная структура (`StructureStorage::Dictionary`) формы не имеет
    /// вообще, поэтому ВСЕГДА промахивается мимо кэша и идёт в `HashMap` —
    /// и, что важнее, НЕ ТРОГАЕТ ячейку кэша. Если бы словарный объект
    /// затирал её (хоть чем — своим отсутствием формы, `None`), то шейповые
    /// объекты на том же сайте вызова теряли бы быстрый путь после каждого
    /// прохода словарного, то есть навсегда в смешанном цикле.
    pub fn get_field_cached(
        &self,
        name: NameId,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &*s.borrow() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref()
                            && Rc::ptr_eq(cached_shape, shape)
                        {
                            return Ok(slots[*slot as usize].clone());
                        }
                        match shape.index.get(&name) {
                            Some(&slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                Ok(slots[slot as usize].clone())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    StructureStorage::Dictionary { values, .. } => values
                        .get(&name)
                        .cloned()
                        .ok_or(RtError::UnknownField(name)),
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `SetProp` — см. `get_field_cached`.
    pub fn set_field_cached(
        &self,
        name: NameId,
        val: BslValue,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => match &mut *s.borrow_mut() {
                    StructureStorage::Shaped { shape, slots } => {
                        if let Some((cached_shape, slot)) = cache.borrow().as_ref()
                            && Rc::ptr_eq(cached_shape, shape)
                        {
                            slots[*slot as usize] = val;
                            return Ok(());
                        }
                        match shape.index.get(&name).copied() {
                            Some(slot) => {
                                *cache.borrow_mut() = Some((shape.clone(), slot));
                                slots[slot as usize] = val;
                                Ok(())
                            }
                            None => Err(RtError::UnknownField(name)),
                        }
                    }
                    // Кэш не трогаем — см. `get_field_cached`.
                    StructureStorage::Dictionary { values, .. } => match values.get_mut(&name) {
                        Some(slot) => {
                            *slot = val;
                            Ok(())
                        }
                        None => Err(RtError::UnknownField(name)),
                    },
                },
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Резолвинг поля/псевдо-свойства по ИМЕНИ (не `NameId`) — нужен для
    /// объектов, чьи "поля" известны только в рантайме: колонки
    /// `СтрокиТаблицыЗначений` заводятся через `.Колонки.Добавить(имя)`, а
    /// не как статичная форма структуры, поэтому по ним нельзя
    /// интернировать `NameId` на этапе компиляции. `Структура` в эту
    /// функцию не заходит — у неё есть более быстрый путь через
    /// `get_field`/`NameId`, здесь она просто не находится.
    ///
    /// Имена сравниваются через [`folded_eq`], а не `eq_ignore_ascii_case`:
    /// последняя не сворачивает кириллицу, и `КЗ.значение` не совпадало с
    /// `Значение` — при том что в языке имена регистронезависимы. Путь
    /// холодный (у `Структуры` свой), а `folded_eq` начинает с побайтового
    /// равенства, так что каноничное написание не платит ничего.
    pub fn get_field_by_name(&self, name: &str) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    if folded_eq(name, "Колонки") || folded_eq(name, "Columns") {
                        Ok(BslValue::Object(Rc::new(BslObject::TableColumns(
                            data.clone(),
                        ))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::TableRow(data, row_id) => {
                    let data = data.borrow();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.get_cell(*row_id, col).ok_or(RtError::RowInvalidated)
                }
                BslObject::TableColumn(data, column_name) => {
                    let column = data
                        .borrow()
                        .column_index(column_name)
                        .ok_or_else(|| RtError::UnknownColumn(column_name.clone()))?;
                    if folded_eq(name, "Имя") || folded_eq(name, "Name") {
                        Ok(BslValue::Str(BslString::from_str(column_name)))
                    } else if folded_eq(name, "ТипЗначения") || folded_eq(name, "ValueType")
                    {
                        let types: Vec<TypeRef> = data
                            .borrow()
                            .column_types
                            .get(column)
                            .cloned()
                            .flatten()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|t| t.id)
                            .collect();
                        Ok(BslValue::Object(Rc::new(BslObject::TypeDescription(types))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                // У буфера `Размер` и `ПорядокБайтов` — именно СВОЙСТВА:
                // `Б.Размер()` со скобками платформа отвергает (измерено),
                // поэтому оба живут здесь, а не в таблице методов.
                BslObject::BinaryBuffer(_) => {
                    if folded_eq(name, "Размер") || folded_eq(name, "Size") {
                        bindata::size(self)
                    } else if folded_eq(name, "ПорядокБайтов") || folded_eq(name, "ByteOrder")
                    {
                        bindata::get_order(self)
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::KeyValuePair(k, v) => {
                    if folded_eq(name, "Ключ") || folded_eq(name, "Key") {
                        Ok(k.clone())
                    } else if folded_eq(name, "Значение") || folded_eq(name, "Value") {
                        Ok(v.clone())
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field_by_name(&self, name: &str, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                // Пишется только `ПорядокБайтов`: `Размер` доступен лишь на
                // чтение, присваивание в него платформа отвергает
                // (измерено — прежний размер при этом уцелел).
                BslObject::BinaryBuffer(_) => {
                    if folded_eq(name, "ПорядокБайтов") || folded_eq(name, "ByteOrder")
                    {
                        bindata::set_order(self, val)
                    } else if folded_eq(name, "Размер") || folded_eq(name, "Size") {
                        Err(RtError::TypeError {
                            expected: "Свойство, доступное для записи",
                            op: "Размер",
                        })
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                // Узлы DOM: пишутся значение, данные и текстовое
                BslObject::TableRow(data, row_id) => {
                    let mut data = data.borrow_mut();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.set_cell(*row_id, col, val)
                        .ok_or(RtError::RowInvalidated)
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Методы, полиморфные по типу получателя --------------------------
    //
    // `Добавить`/`Удалить`/`Очистить` в реальной 1С означают разное в
    // зависимости от типа получателя (элемент массива, строка таблицы,
    // колонка, ...) — то же имя метода, разное поведение и разная арность.
    // Резолвинг имени в `bsl-sema` не может знать заранее, каким объектом
    // оказится `obj` в рантайме (BSL — динамически типизированный), поэтому
    // диспетчеризация и проверка арности — здесь, в рантайме, а не на этапе
    // компиляции.

    /// `Массив.Добавить(значение)`.
    pub fn push_element(&self, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().push(val);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `ТаблицаЗначений.Добавить()` -> новая строка.
    pub fn table_add_row(&self) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    let row_id = data.borrow_mut().add_row()?;
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(
                        data.clone(),
                        row_id,
                    ))))
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Таблица.Колонки.Добавить(Имя[, ТипЗначения])`.
    pub fn table_add_column(&self, name: &BslValue, value_type: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableColumns(data) => {
                    let name = name.as_str("Колонки.Добавить")?.to_string();
                    let value_types = match value_type {
                        BslValue::Undefined => None,
                        BslValue::Object(value) => match &**value {
                            BslObject::TypeDescription(types) => Some(types.clone()),
                            _ => {
                                return Err(RtError::TypeError {
                                    expected: "ОписаниеТипов",
                                    op: "Колонки.Добавить",
                                });
                            }
                        },
                        _ => {
                            return Err(RtError::TypeError {
                                expected: "ОписаниеТипов",
                                op: "Колонки.Добавить",
                            });
                        }
                    };
                    data.borrow_mut().add_typed_column(&name, value_types);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Удалить(индекс)` / `ТаблицаЗначений.Удалить(индекс)`.
    pub fn delete_element(&self, idx: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    if i >= len {
                        return Err(RtError::IndexOutOfBounds {
                            index: i as i64,
                            len,
                        });
                    }
                    v.remove(i);
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    let mut d = data.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = d.row_count();
                    d.delete_row_at(i).ok_or(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    })
                }
                // `Соответствие.Удалить(Ключ)` — по значению ключа, не по
                // позиции (в отличие от Array/ValueTable выше): в этом
                // случае `idx` в имени параметра функции вводит в
                // заблуждение, но сигнатура (`&BslValue`) уже общая для
                // всех получателей, менять её ради одного случая не стоит.
                BslObject::Map(data) => {
                    data.borrow_mut().remove(idx);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Удалить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Удалить",
                receiver: self.type_name(),
            }),
        }
    }

    // --- ТаблицаЗначений, волна 2 ----------------------------------------

    /// Общий доступ к данным таблицы для методов волны 2 — все они
    /// применимы только к самой `ТаблицаЗначений`, не к строке и не к
    /// коллекции колонок.
    fn as_table(&self, method: &'static str) -> RtResult<&Rc<std::cell::RefCell<ValueTableData>>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => Ok(data),
                _ => Err(RtError::MethodNotApplicable {
                    method,
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: self.type_name(),
            }),
        }
    }

    /// Разбор списка колонок `"Кол1, Кол2"` в индексы. Пустая строка (или
    /// отсутствующий аргумент) — пустой список, что для `Найти` значит
    /// «искать во всех колонках».
    fn column_indices(data: &ValueTableData, spec: &str) -> RtResult<Vec<usize>> {
        spec.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|name| {
                data.column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))
            })
            .collect()
    }

    /// `Найти(Значение[, Колонки])` -> `СтрокаТаблицыЗначений` либо
    /// `Неопределено`, если ничего не нашлось (не ошибка — это штатный
    /// способ проверить наличие).
    pub fn table_find(&self, value: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Найти")?;
        let cols = match columns {
            BslValue::Undefined => Vec::new(),
            other => {
                let spec = other.as_str("Найти")?.to_string();
                Self::column_indices(&data.borrow(), &spec)?
            }
        };
        let found = data.borrow().find(value, &cols);
        Ok(match found {
            Some(row_id) => BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))),
            None => BslValue::Undefined,
        })
    }

    /// `НайтиСтроки(СтруктураПоиска)` -> `Массив` строк таблицы.
    ///
    /// Имена полей структуры — это имена колонок, поэтому нужен интернер:
    /// поля хранятся `NameId`, а колонки — строками (они заведены в
    /// рантайме через `.Колонки.Добавить`, см. `get_field_by_name`).
    pub fn table_find_rows(&self, criteria: &BslValue, names: &NameInterner) -> RtResult<BslValue> {
        let data = self.as_table("НайтиСтроки")?;
        let BslValue::Object(o) = criteria else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };
        let BslObject::Structure(s) = &**o else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "НайтиСтроки",
            });
        };

        let pairs = {
            let s = s.borrow();
            let d = data.borrow();
            let mut pairs = Vec::with_capacity(s.len());
            for i in 0..s.len() {
                let (field, want) = s.entry_at(i).ok_or(RtError::NotAnObject)?;
                let name = names.name(field).ok_or(RtError::UnknownField(field))?;
                let col = d
                    .column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                pairs.push((col, want));
            }
            pairs
        };

        let ids = data.borrow().find_rows(&pairs);
        Ok(BslValue::new_array(
            ids.into_iter()
                .map(|id| BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), id))))
                .collect(),
        ))
    }

    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`. Живые объекты
    /// `СтрокаТаблицыЗначений` переживают сортировку — см.
    /// `ValueTableData::sort`.
    pub fn table_sort(&self, spec: &BslValue, comparison: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сортировать")?;
        if !matches!(comparison, BslValue::Undefined)
            && !matches!(
                comparison,
                BslValue::Object(value) if matches!(&**value, BslObject::ValueComparison)
            )
        {
            return Err(RtError::TypeError {
                expected: "СравнениеЗначений",
                op: "Сортировать",
            });
        }
        let spec = spec.as_str("Сортировать")?.to_string();
        let keys = {
            let d = data.borrow();
            table::parse_sort_spec(&spec, |name| d.column_index(name))
                .map_err(RtError::UnknownColumn)?
        };
        data.borrow_mut().sort(&keys);
        Ok(())
    }

    /// `ЗаполнитьЗначения(Значение[, Колонки])`.
    pub fn table_fill_values(&self, value: &BslValue, columns: &BslValue) -> RtResult<()> {
        let data = self.as_table("ЗаполнитьЗначения")?;
        let cols = match columns {
            BslValue::Undefined => Vec::new(),
            other => {
                let spec = other.as_str("ЗаполнитьЗначения")?.to_string();
                Self::column_indices(&data.borrow(), &spec)?
            }
        };
        data.borrow_mut().fill_values(value, &cols);
        Ok(())
    }

    /// `Итог("Колонка")` -> `Число`. Про нечисловые значения см.
    /// `ValueTableData::total` (`НЕ ИЗМЕРЕНО(TABLE.TOTAL.NON_NUMERIC)`).
    pub fn table_total(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Итог")?;
        let name = column.as_str("Итог")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::Number(d.total(col)?))
    }

    // --- ТаблицаЗначений, волна 3 ----------------------------------------

    /// Строка ЭТОЙ таблицы: разворачивает объект `СтрокаТаблицыЗначений` в
    /// текущую позицию. Строка чужой таблицы — ошибка метода, а не «не
    /// найдено»: спутать таблицы легко, и молчаливый `-1` в ответ прятал бы
    /// эту ошибку до самого конца.
    fn row_position(
        data: &Rc<std::cell::RefCell<ValueTableData>>,
        row: &BslValue,
        method: &'static str,
    ) -> RtResult<usize> {
        let BslValue::Object(o) = row else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        let BslObject::TableRow(owner, row_id) = &**o else {
            return Err(RtError::TypeError {
                expected: "СтрокаТаблицыЗначений",
                op: method,
            });
        };
        if !Rc::ptr_eq(owner, data) {
            return Err(RtError::MethodNotApplicable {
                method,
                receiver: "СтрокаТаблицыЗначений другой таблицы",
            });
        }
        data.borrow().pos_of(*row_id).ok_or(RtError::RowInvalidated)
    }

    /// Список колонок `"Кол1, Кол2"` -> индексы; `Неопределено` или пустая
    /// строка -> ВСЕ колонки в их порядке. Разворачивать «все» здесь, а не
    /// в `ValueTableData`, нарочно: слой данных не должен знать, что пустой
    /// список для `Скопировать` значит «все», а для `Найти` — «любая».
    fn columns_or_all(
        data: &ValueTableData,
        spec: &BslValue,
        method: &'static str,
    ) -> RtResult<Vec<usize>> {
        let all = || (0..data.column_names.len()).collect::<Vec<usize>>();
        match spec {
            BslValue::Undefined => Ok(all()),
            other => {
                let spec = other.as_str(method)?.to_string();
                if spec.trim().is_empty() {
                    return Ok(all());
                }
                Self::column_indices(data, &spec)
            }
        }
    }

    /// `Скопировать([Строки], [Колонки])` -> новая `ТаблицаЗначений`.
    ///
    /// `Строки` — `Массив` строк ЭТОЙ таблицы (порядок массива и есть
    /// порядок строк копии) либо `Неопределено` — тогда все строки в
    /// текущем порядке.
    pub fn table_copy(&self, rows: &BslValue, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Скопировать")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "Скопировать")?;
        let positions: Vec<usize> = match rows {
            BslValue::Undefined => (0..data.borrow().row_count()).collect(),
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let items = items.borrow();
                    let mut out = Vec::with_capacity(items.len());
                    for row in items.iter() {
                        out.push(Self::row_position(data, row, "Скопировать")?);
                    }
                    out
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: "Скопировать",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: "Скопировать",
                });
            }
        };
        let copy = data.borrow().copy_of(&positions, &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// Перегрузка `Скопировать(Отбор, Колонки)`, где `Отбор` — структура
    /// с именами колонок и требуемыми значениями.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при неверном типе отбора или неизвестной колонке.
    pub fn table_copy_by_filter(
        &self,
        criteria: &BslValue,
        columns: &BslValue,
        names: &NameInterner,
    ) -> RtResult<BslValue> {
        let data = self.as_table("Скопировать")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "Скопировать")?;
        let BslValue::Object(criteria_object) = criteria else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Скопировать",
            });
        };
        let BslObject::Structure(criteria_data) = &**criteria_object else {
            return Err(RtError::TypeError {
                expected: "Структура",
                op: "Скопировать",
            });
        };

        let pairs = {
            let criteria_data = criteria_data.borrow();
            let table_data = data.borrow();
            let mut pairs = Vec::with_capacity(criteria_data.len());
            for i in 0..criteria_data.len() {
                let (field, value) = criteria_data.entry_at(i).ok_or(RtError::NotAnObject)?;
                let name = names.name(field).ok_or(RtError::UnknownField(field))?;
                let col = table_data
                    .column_index(name)
                    .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                pairs.push((col, value));
            }
            pairs
        };
        let positions: Vec<usize> = {
            let table_data = data.borrow();
            table_data
                .find_rows(&pairs)
                .into_iter()
                .filter_map(|row_id| table_data.pos_of(row_id))
                .collect()
        };
        let copy = data.borrow().copy_of(&positions, &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `СкопироватьКолонки([Колонки])` -> пустая таблица той же структуры.
    /// Это `Скопировать` без единой строки, а не отдельный алгоритм.
    pub fn table_copy_columns(&self, columns: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("СкопироватьКолонки")?;
        let cols = Self::columns_or_all(&data.borrow(), columns, "СкопироватьКолонки")?;
        let copy = data.borrow().copy_of(&[], &cols);
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(Rc::new(
            std::cell::RefCell::new(copy),
        )))))
    }

    /// `ВыгрузитьКолонку(Колонка)` -> `Массив` значений в текущем порядке
    /// строк.
    pub fn table_unload_column(&self, column: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("ВыгрузитьКолонку")?;
        let name = column.as_str("ВыгрузитьКолонку")?.to_string();
        let d = data.borrow();
        let col = d
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        Ok(BslValue::new_array(d.unload_column(col)))
    }

    /// `ЗагрузитьКолонку(Массив, Колонка)`. Про несовпадение длин — см.
    /// `ValueTableData::load_column`
    /// (`НЕ ИЗМЕРЕНО(TABLE.LOAD_COLUMN.LENGTH_MISMATCH)`).
    pub fn table_load_column(&self, values: &BslValue, column: &BslValue) -> RtResult<()> {
        let data = self.as_table("ЗагрузитьКолонку")?;
        let name = column.as_str("ЗагрузитьКолонку")?.to_string();
        let BslValue::Object(o) = values else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let BslObject::Array(items) = &**o else {
            return Err(RtError::TypeError {
                expected: "Массив",
                op: "ЗагрузитьКолонку",
            });
        };
        let col = data
            .borrow()
            .column_index(&name)
            .ok_or_else(|| RtError::UnknownColumn(name.clone()))?;
        let values = items.borrow().clone();
        data.borrow_mut().load_column(col, &values);
        Ok(())
    }

    /// `Сдвинуть(Строка, Смещение)` — `Строка` — это либо объект строки, либо
    /// её индекс. Целевая позиция вне таблицы — `IndexOutOfBounds`, а не
    /// зажатие в границы.
    ///
    /// `НЕ ИЗМЕРЕНО(TABLE.MOVE.OUT_OF_RANGE)`: падает ли платформа или молча
    /// зажимает. Взята ошибка: `Сдвинуть(ПерваяСтрока, -1)`, тихо ничего не
    /// сделавший, — та же категория беды, что и `Сортировать("Опечатка")`,
    /// молча ничего не отсортировавшая.
    pub fn table_move(&self, row: &BslValue, offset: &BslValue) -> RtResult<()> {
        let data = self.as_table("Сдвинуть")?;
        let from = match row {
            BslValue::Number(_) => {
                let i = Self::index_as_usize(row)?;
                let len = data.borrow().row_count();
                if i >= len {
                    return Err(RtError::IndexOutOfBounds {
                        index: i as i64,
                        len,
                    });
                }
                i
            }
            other => Self::row_position(data, other, "Сдвинуть")?,
        };
        let BslValue::Number(n) = offset else {
            return Err(RtError::TypeError {
                expected: "Число",
                op: "Сдвинуть",
            });
        };
        let offset = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        let len = data.borrow().row_count();
        data.borrow_mut()
            .move_row(from, offset)
            .map(|_| ())
            .ok_or(RtError::IndexOutOfBounds {
                index: from as i64 + offset,
                len,
            })
    }

    /// `Индекс(Строка)` -> `Число`, позиция строки (с нуля, как у
    /// `Получить`/`Удалить`).
    pub fn table_index_of(&self, row: &BslValue) -> RtResult<BslValue> {
        let data = self.as_table("Индекс")?;
        let pos = Self::row_position(data, row, "Индекс")?;
        Ok(BslValue::Number(BslNumber::from_i64(pos as i64)))
    }

    /// `Свернуть(КолонкиГруппировки[, КолонкиСуммирования])` — группировка
    /// на месте. Три неизмеренных решения (судьба прочих колонок, порядок
    /// строк, нечисловые значения) описаны у `ValueTableData::collapse`.
    pub fn table_collapse(&self, group: &BslValue, sum: &BslValue) -> RtResult<()> {
        let data = self.as_table("Свернуть")?;
        let (group_cols, sum_cols) = {
            let d = data.borrow();
            let group_cols = Self::column_indices(&d, &group.as_str("Свернуть")?.to_string())?;
            let sum_cols = match sum {
                BslValue::Undefined => Vec::new(),
                other => Self::column_indices(&d, &other.as_str("Свернуть")?.to_string())?,
            };
            (group_cols, sum_cols)
        };
        data.borrow_mut().collapse(&group_cols, &sum_cols)?;
        Ok(())
    }

    /// `Массив.Очистить()` / `ТаблицаЗначений.Очистить()` /
    /// `Соответствие.Очистить()`.
    pub fn clear_collection(&self) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().clear();
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Очистить",
                receiver: self.type_name(),
            }),
        }
    }
}

/// Ручная реализация вместо `derive`: `Массив`/`Структура` — ссылочные
/// типы, `=` для них — тождество объекта (`Rc::ptr_eq`), а не структурное
/// сравнение содержимого (в отличие от `Число`/`Строка`/`Булево`).
/// Строка как условие: набор слов ИЗМЕРЕН перебором на 8.3.27, а не взят
/// из документации. Принимаются ровно шесть написаний, регистр не важен, а
/// пробелы по краям обрезаются («" Истина "» проходит). «yes», «no» и «Y`»
/// платформа НЕ принимает — то есть это не «любое разумное слово», а
/// закрытый список; `None` здесь и означает отказ.
fn condition_word(s: &str) -> Option<bool> {
    let w = s.trim().to_uppercase();
    match w.as_str() {
        "ИСТИНА" | "ДА" | "TRUE" => Some(true),
        "ЛОЖЬ" | "НЕТ" | "FALSE" => Some(false),
        _ => None,
    }
}

impl PartialEq for BslValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Undefined, BslValue::Undefined) => true,
            (BslValue::Null, BslValue::Null) => true,
            (BslValue::Boolean(a), BslValue::Boolean(b)) => a == b,
            (BslValue::Number(a), BslValue::Number(b)) => a == b,
            (BslValue::Str(a), BslValue::Str(b)) => a == b,
            // Дата — тип ЗНАЧЕНИЯ, как число и строка: равенство по
            // моменту времени, а не по тождеству объекта.
            (BslValue::Date(a), BslValue::Date(b)) => a == b,
            // Тип — значение, а не объект: два `ТипЗнч(...)` от разных
            // значений одного типа равны (`ТипЗнч(1) = ТипЗнч(2)`), иначе
            // проверка типа была бы бесполезна.
            (BslValue::Type(a), BslValue::Type(b)) => a == b,
            // Члены перечисления сравниваются как значения — на этом
            // держится весь потоковый разбор JSON (`Если Т =
            // ТипЗначенияJSON.ИмяСвойства Тогда`).
            (BslValue::Enum(a), BslValue::Enum(b)) => a == b,
            // Голое имя перечисления — тоже значение, тем же рассуждением.
            (BslValue::EnumType(a), BslValue::EnumType(b)) => a == b,
            // Один и тот же объект равен себе при любом виде — для
            // непрозрачных значений это быстрый путь: чтение интернирует
            // повторяющиеся ссылки в один объект, и сравнение текстов до
            // них просто не доходит.
            (BslValue::Object(a), BslValue::Object(b)) if Rc::ptr_eq(a, b) => true,
            (BslValue::Object(a), BslValue::Object(b)) => match (&**a, &**b) {
                // Внешние объекты равны по объявленному ключу «то же
                // состояние, то же место»: обёртку строит каждое обращение
                // к коллекции, и тождества обёрток для измеренных равенств
                // мало. Без ключа — только тождество (быстрый путь выше).
                (BslObject::Extension(x), BslObject::Extension(y)) => {
                    if !std::ptr::eq(x.type_descriptor(), y.type_descriptor()) {
                        return false;
                    }
                    if let Some(equal) = x.value_eq(y) {
                        return equal;
                    }
                    match (x.identity_key(), y.identity_key()) {
                        (Some(kx), Some(ky)) => kx == ky,
                        _ => false,
                    }
                }
                // Непрозрачные значения внутреннего формата равны ПО
                // ТЕКСТУ: две ссылки на один объект базы, прочитанные из
                // разных строк, равны — так ведёт себя платформа
                // (измерено, проба `REF.CAT.RT`).
                (BslObject::VstrOpaque(x), BslObject::VstrOpaque(y)) => x == y,
                // Двоичные данные — тоже значение, а не ссылка: ИЗМЕРЕНО
                // (пробы `BIN.EQ`/`BIN.EQ.DIFF`), что два `Новый
                // ДвоичныеДанные` от ОДНОГО файла равны, а от разных — нет.
                (BslObject::BinaryData(x), BslObject::BinaryData(y)) => x == y,
                // УИД — значение: два идентификатора с одними байтами
                // равны, откуда бы они ни пришли.
                (BslObject::Uuid(x), BslObject::Uuid(y)) => x == y,
                // Узлы DOM — ССЫЛКИ на место в дереве: обёртка каждый раз
                _ => false,
            },
            _ => false,
        }
    }
}

/// `Eq` — все варианты `PartialEq::eq` выше рефлексивны (десятичные числа
/// без NaN-подобных значений, указатели сравниваются с самими собой),
/// значит `Eq` держится честно, не только формально для `HashMap`.
impl Eq for BslValue {}

/// Нужен ключу `Соответствие` (`HashMap<BslValue, BslValue>` в `map.rs`).
/// Согласован с `PartialEq` выше пункт-в-пункт: `Число` хэширует через
/// `BslNumber` (см. его `impl Hash` — нормализация уже сделала хэш
/// независимым от масштаба представления, `1.0` и `1.00` дают одно и то же
/// (`m`, `scale`)), `Строка` — по содержимому (`BslString` производит
/// `Hash` от `Rc<[u16]>`, тоже по значению, не по адресу), `Массив`/
/// `Структура`/... — по адресу `Rc`, ровно как их `==` через `Rc::ptr_eq`.
impl Hash for BslValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BslValue::Undefined | BslValue::Null => {}
            BslValue::Boolean(b) => b.hash(state),
            BslValue::Number(n) => n.hash(state),
            BslValue::Str(s) => s.hash(state),
            BslValue::Date(d) => d.hash(state),
            BslValue::Type(t) => t.hash(state),
            BslValue::Enum(e) => e.hash(state),
            BslValue::EnumType(k) => k.hash(state),
            // Непрозрачное значение хэширует текст, двоичные данные —
            // байты: оба согласованы со своим равенством ПО СОДЕРЖИМОМУ в
            // `PartialEq` выше, иначе ключ `Соответствие` терялся бы.
            BslValue::Object(o) => match &**o {
                BslObject::VstrOpaque(text) => text.hash(state),
                BslObject::BinaryData(bytes) => bytes.hash(state),
                // УИД — значение: хэшируем байты, ровно как `PartialEq` их
                // сравнивает (воспроизведение 3 — иначе два равных УИД дают
                // два ключа `Соответствия`). Совпадение с хэшем `BinaryData`
                // из тех же байтов законно: это коллизия, равными их
                // `PartialEq` не делает (кросс-тип уходит в `_ => false`).
                BslObject::Uuid(bytes) => bytes.hash(state),
                // Внешний объект хэширует то же, чем равняется, и в ТОМ ЖЕ
                // порядке правил, что `PartialEq` выше: сперва содержимое
                // (`value_eq` у типов-значений хэширует представление), затем
                // ключ места (`identity_key`), и только при чистом тождестве —
                // адрес обёртки. Обратный порядок ломал бы ключ `Соответствия`
                // для типа, реализующего ОБА метода: он равнялся бы по
                // `value_eq`, а хэшировался по чужому `identity_key`.
                BslObject::Extension(object) => {
                    std::ptr::from_ref(object.type_descriptor()).hash(state);
                    if object.value_eq(object).is_some() {
                        object.display().hash(state);
                    } else if let Some(key) = object.identity_key() {
                        key.hash(state);
                    } else {
                        Rc::as_ptr(o).hash(state);
                    }
                }
                _ => Rc::as_ptr(o).hash(state),
            },
        }
    }
}

/// Наибольший размер части, который принимает `РазделитьДвоичныеДанные`.
///
/// ИЗМЕРЕНО фикстурой `binary-data` с точностью до единицы: `2^64-1`
/// платформа принимает (и отдаёт одну часть целиком), `2^64` — уже
/// ошибка. То есть счётчик у неё 64-битный БЕЗ знака, а не `i64`: `2^63`
/// тоже проходит.
fn binary_split_max_part() -> BslNumber {
    BslNumber::from_i128(u64::MAX as i128)
}

/// Сколько байтов попадает в строковое представление `ДвоичныеДанные`.
///
/// ИЗМЕРЕНО (проба `BIN.STR.LONG`): у значения в 303 байта `Строка()`
/// печатает ровно 256 пар, за которыми СРАЗУ, без разделяющего пробела,
/// идёт многоточие. Ровно на границе (255/256/257 байт) поведение
/// закреплено фикстурой `binary-data`.
const BINARY_DATA_DISPLAY_LIMIT: usize = 256;

/// Строковое представление `ДвоичныеДанные` — не имя типа, а САМИ БАЙТЫ:
/// шестнадцатеричные пары в ВЕРХНЕМ регистре через пробел, не более
/// [`BINARY_DATA_DISPLAY_LIMIT`] штук, с многоточием у длинного значения
/// (измерено, пробы `BIN.STR`, `BIN.STR.LONG`, `BIN.EMPTY`: у пустых
/// данных представление — пустая строка).
fn binary_data_display(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let shown = bytes.len().min(BINARY_DATA_DISPLAY_LIMIT);
    let mut out = String::with_capacity(shown * 3 + 3);
    for (i, b) in bytes[..shown].iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    if bytes.len() > shown {
        out.push_str("...");
    }
    out
}

impl fmt::Display for BslValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BslValue::Undefined => write!(f, ""),
            BslValue::Null => write!(f, "Null"),
            BslValue::Boolean(true) => write!(f, "Да"),
            BslValue::Boolean(false) => write!(f, "Нет"),
            BslValue::Number(n) => write!(f, "{n}"),
            BslValue::Str(s) => write!(f, "{s}"),
            // Формат по умолчанию (`date::DEFAULT_PATTERN`) — НЕ ИЗМЕРЕН,
            // см. там же.
            BslValue::Date(d) => write!(f, "{d}"),
            // Локализованное имя типа: `Строка(ТипЗнч(Новый Массив))` даёт
            // то же `Массив`, что и `Строка(Новый Массив)`.
            BslValue::Type(t) => write!(f, "{t}"),
            BslValue::Enum(e) => write!(f, "{}", e.display_text()),
            // `НЕ ИЗМЕРЕНО(JSON.ENUM.BARE_NAME)`: по умолчанию — то же имя,
            // что стоит слева от точки в исходном тексте (симметрично
            // `type_name`).
            BslValue::EnumType(k) => write!(f, "{}", k.meta_ru_name()),
            BslValue::Object(o) => match &**o {
                BslObject::Extension(object) => write!(f, "{}", object.display()),
                BslObject::VstrOpaque(_) => write!(f, "НепрозрачноеЗначение"),
                BslObject::Array(_) => write!(f, "Массив"),
                BslObject::Structure(_) => write!(f, "Структура"),
                BslObject::ValueTable(_) => write!(f, "ТаблицаЗначений"),
                BslObject::TableColumns(_) => write!(f, "КоллекцияКолонокТаблицыЗначений"),
                BslObject::TableColumn(..) => write!(f, "КолонкаТаблицыЗначений"),
                BslObject::TableRow(_, _) => write!(f, "СтрокаТаблицыЗначений"),
                BslObject::TypeDescription(_) => write!(f, "ОписаниеТипов"),
                BslObject::ValueComparison => write!(f, "СравнениеЗначений"),
                BslObject::Map(_) => write!(f, "Соответствие"),
                BslObject::KeyValuePair(_, _) => write!(f, "КлючИЗначение"),
                BslObject::TextWriter(_) => write!(f, "ЗаписьТекста"),
                // УИД печатается своей канонической формой, а не именем
                // типа: `Строка(УИД)` — это и есть его строка (фикстура
                // `uuid`, эталон с платформы).
                BslObject::Uuid(b) => write!(f, "{}", uuid::format(b)),
                // Единственный объект, который печатается СОДЕРЖИМЫМ, а не
                // именем: см. `binary_data_display`.
                BslObject::BinaryData(bytes) => write!(f, "{}", binary_data_display(bytes)),
                // Буфер, в отличие от двоичных данных, печатается ИМЕНЕМ, а
                // не содержимым (измерено): дампа байтов у него нет.
                BslObject::BinaryBuffer(_) => write!(f, "БуферДвоичныхДанных"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
