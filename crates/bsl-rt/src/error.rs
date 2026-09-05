//! Ошибки runtime и их классификация для обработки исключений BSL.

use crate::{BslValue, HostError, NameId};
use bsl_number::NumError;
use std::fmt;

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
    /// Rust-встраивание различает причину по [`HostErrorCode`](crate::HostErrorCode).
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
