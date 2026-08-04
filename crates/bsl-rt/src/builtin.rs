use crate::date::{DateBoundary, DatePart};
use crate::runtime_shapes::RuntimeShapes;
use crate::{BslObject, BslValue, NameId, RtError, RtResult};

/// Встроенные функции, вызываемые по голому имени (`Sqrt(x)`, `Pow(x,y)`,
/// ...). Разрешаются регистронезависимо (`sqrt` == `Sqrt`), без перевода на
/// русский — в реальной 1С у математических функций нет русских синонимов
/// (в отличие от ключевых слов), что и подтверждает сам n-body: `sqrt(...)`
/// написан строчными буквами прямо в "русском" файле.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFn {
    Sqrt,
    Pow,
    Ln,
    Log10,
    Exp,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    /// `Окр(x, ЧислоРазрядов, Режим)` — арность у самой функции в 1С
    /// переменная (второй и третий аргументы необязательны, оба по
    /// умолчанию `0`), но здесь всегда 3:
    /// `bsl-sema::resolver::resolve_call` подставляет недостающие `0`
    /// литералами при резолвинге, а не вводит вариативную арность ради
    /// одной функции. Про то, что режим по умолчанию НЕ ИЗМЕРЕН, — см.
    /// `BslValue::round` и `bsl_number::DEFAULT_ROUND_MODE`.
    Round,
    /// `Цел(x)` — усечение к нулю, не округление (см. `BslValue::trunc`).
    Trunc,
    /// Побочный эффект — печать в stdout. Заглушка на месте настоящего
    /// вывода/UI.
    Message,
    /// `Строка(x)` = `Формат(x, Неопределено)` — форматная строка по
    /// умолчанию. Форматирование живёт в `bsl-format` (более высокий
    /// слой, чем этот крейт), поэтому `call_builtin_fn` ниже для этого
    /// варианта не вызывается в реальном пайплайне — VM перехватывает его
    /// раньше (см. `bsl-vm`); здесь только выделено имя-идентификатор.
    ToString,
    /// `Формат(x, spec)` — с явной форматной строкой.
    Format,
    /// `Число(строка)` — обратный разбор форматированной строки в число.
    ToNumber,

    /// `СтрДлина`/`StrLen` — длина в код-юнитах UTF-16, не в символах.
    StrLen,
    /// `Лев`/`Left(строка, длина)`.
    Left,
    /// `Прав`/`Right(строка, длина)`.
    Right,
    /// `Сред`/`Mid(строка, начало[, длина])` — длину можно опустить, тогда
    /// берётся всё до конца строки (см. `arity_range`, который и делает
    /// третий аргумент необязательным).
    Mid,
    Upper,
    Lower,
    /// `СокрЛП`/`TrimAll` — обрезка пробелов с обеих сторон.
    TrimAll,
    /// `СокрЛ`/`TrimL` — только слева.
    TrimLeft,
    /// `СокрП`/`TrimR` — только справа.
    TrimRight,
    /// `СтрНайти`/`StrFind(строка, подстрока)` -> позиция 1-based в
    /// код-юнитах UTF-16, `0` если не найдено.
    StrFind,
    /// `СтрЗаменить`/`StrReplace(строка, что, чем)`.
    StrReplace,
    /// `СтрРазделить`/`StrSplit(строка, разделитель)` -> `Массив`.
    StrSplit,
    /// `СтрСоединить`/`StrConcat(массив, разделитель)` -> `Строка`.
    StrConcat,
    /// `СтрЧислоСтрок`/`StrLineCount`.
    StrLineCount,
    /// `СтрПолучитьСтроку`/`StrGetLine(строка, номер)`.
    StrGetLine,
    /// `СтрШаблон`/`StrTemplate(шаблон[, З1..З10])` — единственная
    /// по-настоящему вариативная встроенная функция (см. `arity_range`).
    StrTemplate,
    /// `Символ`/`Char(код)`.
    Char,
    /// `КодСимвола`/`CharCode(строка[, позиция])`.
    CharCode,
    /// `ЗначениеЗаполнено`/`ValueIsFilled` — см. `BslValue::is_filled`,
    /// там же про то, какие ветки НЕ ИЗМЕРЕНЫ.
    ValueIsFilled,
    /// `ТипЗнч`/`TypeOf(значение)` -> `Тип`.
    TypeOf,
    /// `Тип`/`Type("ИмяТипа")` -> `Тип`.
    TypeByName,

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])` либо
    /// `Дата("ГГГГММДДЧЧММСС")` — одна встроенная функция с перегрузкой по
    /// типу первого аргумента, как и в самой 1С (см. `BslValue::make_date`).
    MakeDate,
    /// `ТекущаяДата`/`CurrentDate` — см. `BslValue::current_date` про то,
    /// почему момент берётся по UTC, а не по локальной зоне.
    CurrentDate,
    /// Число миллисекунд с начала Unix-эпохи в UTC. Используется в BSL для
    /// измерения длительности выполнения коротких участков кода.
    CurrentUniversalDateInMilliseconds,
    /// `Год`/`Месяц`/`День`/`Час`/`Минута`/`Секунда`/`ДеньНедели` — семь
    /// имён на один вариант с селектором: тела у них отличаются одним
    /// полем разложения.
    DatePartOf(DatePart),
    /// `НачалоДня`/`КонецДня`/`НачалоМесяца`/`КонецМесяца`/`НачалоГода`/
    /// `КонецГода`/`НачалоНедели` — так же одним вариантом с селектором.
    DateBoundaryOf(DateBoundary),
    /// `ДобавитьМесяц(Дата, Количество)`.
    AddMonth,

    /// `ЗаполнитьЗначенияСвойств(Приемник, Источник[, СписокСвойств[,
    /// ИсключаяСвойства]])` — единственная встроенная ФУНКЦИЯ, которой
    /// нужен рантайм-контекст имён (набор полей приёмника резолвится по
    /// `NameId`), поэтому её тело живёт в модуле `fill` и вызывается через
    /// [`call_builtin_fn_ctx`], а не через [`call_builtin_fn`].
    FillPropertyValues,

    /// `ПрочитатьJSON(Чтение[, ВозвращатьСоответствие[,
    /// ИменаСвойствСоЗначениямиДата]])`. Как и
    /// `ЗаполнитьЗначенияСвойств`, требует контекста форм: объект JSON
    /// превращается в `Структура`, а её поля надо интернировать.
    ReadJson,
    /// `ЗаписатьJSON(Запись, Значение)` — тот же контекст нужен, чтобы
    /// прочитать ИМЕНА полей сериализуемой структуры.
    WriteJson,
}

/// Написания встроенных ФУНКЦИЙ: `(имя, вариант)` в каноническом
/// регистре — том, который предлагает автодополнение REPL.
///
/// У математических функций русских синонимов НЕТ — в реальной 1С их тоже
/// нет (в отличие от ключевых слов); это подтверждает и сам n-body, где
/// `sqrt(...)` написан строчными прямо в «русском» файле. Поэтому у части
/// вариантов здесь ровно одна строка, а не две.
///
/// Таблица — единственный источник: `lookup` ищет по ней же, второго
/// списка, который мог бы разъехаться с первым, нет.
pub const BUILTIN_FN_NAMES: &[(&str, BuiltinFn)] = &[
    ("Sqrt", BuiltinFn::Sqrt),
    ("Pow", BuiltinFn::Pow),
    ("Log", BuiltinFn::Ln),
    ("Log10", BuiltinFn::Log10),
    ("Exp", BuiltinFn::Exp),
    ("Sin", BuiltinFn::Sin),
    ("Cos", BuiltinFn::Cos),
    ("Tan", BuiltinFn::Tan),
    ("ASin", BuiltinFn::Asin),
    ("ACos", BuiltinFn::Acos),
    ("ATan", BuiltinFn::Atan),
    ("Окр", BuiltinFn::Round),
    ("Round", BuiltinFn::Round),
    ("Цел", BuiltinFn::Trunc),
    ("Int", BuiltinFn::Trunc),
    ("Сообщить", BuiltinFn::Message),
    ("Message", BuiltinFn::Message),
    ("Строка", BuiltinFn::ToString),
    ("String", BuiltinFn::ToString),
    ("Формат", BuiltinFn::Format),
    ("Format", BuiltinFn::Format),
    ("Число", BuiltinFn::ToNumber),
    ("Number", BuiltinFn::ToNumber),
    ("СтрДлина", BuiltinFn::StrLen),
    ("StrLen", BuiltinFn::StrLen),
    ("Лев", BuiltinFn::Left),
    ("Left", BuiltinFn::Left),
    ("Прав", BuiltinFn::Right),
    ("Right", BuiltinFn::Right),
    ("Сред", BuiltinFn::Mid),
    ("Mid", BuiltinFn::Mid),
    ("ВРег", BuiltinFn::Upper),
    ("Upper", BuiltinFn::Upper),
    ("НРег", BuiltinFn::Lower),
    ("Lower", BuiltinFn::Lower),
    ("СокрЛП", BuiltinFn::TrimAll),
    ("TrimAll", BuiltinFn::TrimAll),
    ("СокрЛ", BuiltinFn::TrimLeft),
    ("TrimL", BuiltinFn::TrimLeft),
    ("СокрП", BuiltinFn::TrimRight),
    ("TrimR", BuiltinFn::TrimRight),
    ("СтрНайти", BuiltinFn::StrFind),
    ("StrFind", BuiltinFn::StrFind),
    ("СтрЗаменить", BuiltinFn::StrReplace),
    ("StrReplace", BuiltinFn::StrReplace),
    ("СтрРазделить", BuiltinFn::StrSplit),
    ("StrSplit", BuiltinFn::StrSplit),
    ("СтрСоединить", BuiltinFn::StrConcat),
    ("StrConcat", BuiltinFn::StrConcat),
    ("СтрЧислоСтрок", BuiltinFn::StrLineCount),
    ("StrLineCount", BuiltinFn::StrLineCount),
    ("СтрПолучитьСтроку", BuiltinFn::StrGetLine),
    ("StrGetLine", BuiltinFn::StrGetLine),
    ("СтрШаблон", BuiltinFn::StrTemplate),
    ("StrTemplate", BuiltinFn::StrTemplate),
    ("Символ", BuiltinFn::Char),
    ("Char", BuiltinFn::Char),
    ("КодСимвола", BuiltinFn::CharCode),
    ("CharCode", BuiltinFn::CharCode),
    ("ЗначениеЗаполнено", BuiltinFn::ValueIsFilled),
    ("ValueIsFilled", BuiltinFn::ValueIsFilled),
    ("ТипЗнч", BuiltinFn::TypeOf),
    ("TypeOf", BuiltinFn::TypeOf),
    ("Тип", BuiltinFn::TypeByName),
    ("Type", BuiltinFn::TypeByName),
    ("Дата", BuiltinFn::MakeDate),
    ("Date", BuiltinFn::MakeDate),
    ("ТекущаяДата", BuiltinFn::CurrentDate),
    ("CurrentDate", BuiltinFn::CurrentDate),
    (
        "ТекущаяУниверсальнаяДатаВМиллисекундах",
        BuiltinFn::CurrentUniversalDateInMilliseconds,
    ),
    (
        "CurrentUniversalDateInMilliseconds",
        BuiltinFn::CurrentUniversalDateInMilliseconds,
    ),
    ("Год", BuiltinFn::DatePartOf(DatePart::Year)),
    ("Year", BuiltinFn::DatePartOf(DatePart::Year)),
    ("Месяц", BuiltinFn::DatePartOf(DatePart::Month)),
    ("Month", BuiltinFn::DatePartOf(DatePart::Month)),
    ("День", BuiltinFn::DatePartOf(DatePart::Day)),
    ("Day", BuiltinFn::DatePartOf(DatePart::Day)),
    ("Час", BuiltinFn::DatePartOf(DatePart::Hour)),
    ("Hour", BuiltinFn::DatePartOf(DatePart::Hour)),
    ("Минута", BuiltinFn::DatePartOf(DatePart::Minute)),
    ("Minute", BuiltinFn::DatePartOf(DatePart::Minute)),
    ("Секунда", BuiltinFn::DatePartOf(DatePart::Second)),
    ("Second", BuiltinFn::DatePartOf(DatePart::Second)),
    ("ДеньНедели", BuiltinFn::DatePartOf(DatePart::Weekday)),
    ("WeekDay", BuiltinFn::DatePartOf(DatePart::Weekday)),
    (
        "НачалоДня",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfDay),
    ),
    (
        "BegOfDay",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfDay),
    ),
    (
        "КонецДня",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfDay),
    ),
    (
        "EndOfDay",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfDay),
    ),
    (
        "НачалоМесяца",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfMonth),
    ),
    (
        "BegOfMonth",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfMonth),
    ),
    (
        "КонецМесяца",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfMonth),
    ),
    (
        "EndOfMonth",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfMonth),
    ),
    (
        "НачалоГода",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfYear),
    ),
    (
        "BegOfYear",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfYear),
    ),
    (
        "КонецГода",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfYear),
    ),
    (
        "EndOfYear",
        BuiltinFn::DateBoundaryOf(DateBoundary::EndOfYear),
    ),
    (
        "НачалоНедели",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfWeek),
    ),
    (
        "BegOfWeek",
        BuiltinFn::DateBoundaryOf(DateBoundary::StartOfWeek),
    ),
    ("ДобавитьМесяц", BuiltinFn::AddMonth),
    ("AddMonth", BuiltinFn::AddMonth),
    ("ЗаполнитьЗначенияСвойств", BuiltinFn::FillPropertyValues),
    ("FillPropertyValues", BuiltinFn::FillPropertyValues),
    ("ПрочитатьJSON", BuiltinFn::ReadJson),
    ("ReadJSON", BuiltinFn::ReadJson),
    ("ЗаписатьJSON", BuiltinFn::WriteJson),
    ("WriteJSON", BuiltinFn::WriteJson),
];

impl BuiltinFn {
    /// Регистронезависимый поиск по [`BUILTIN_FN_NAMES`].
    pub fn lookup(name: &str) -> Option<Self> {
        let upper = name.to_uppercase();
        BUILTIN_FN_NAMES
            .iter()
            .find(|(n, _)| n.to_uppercase() == upper)
            .map(|(_, f)| *f)
    }

    /// `(минимум, максимум)` аргументов. У большинства встроенных они
    /// совпадают — необязательные аргументы есть у меньшинства, и раньше
    /// их не было вовсе (`arity()` возвращала одно число).
    ///
    /// Недостающие до МАКСИМУМА позиции дополняются `Неопределено` на
    /// этапе резолвинга (`bsl-sema::resolver::resolve_call`), так что в
    /// рантайме `call_builtin_fn` всегда видит ровно `max` аргументов и
    /// сам решает, что значит `Неопределено` на этой позиции. Единственное
    /// исключение — `Окр`, у которого резолвер подставляет литеральные
    /// `0`, а не `Неопределено` (см. там же, почему).
    pub fn arity_range(self) -> (usize, usize) {
        match self {
            BuiltinFn::Pow
            | BuiltinFn::Format
            | BuiltinFn::Left
            | BuiltinFn::Right
            | BuiltinFn::StrFind
            | BuiltinFn::StrSplit
            | BuiltinFn::StrConcat
            | BuiltinFn::StrGetLine => (2, 2),
            BuiltinFn::StrReplace => (3, 3),
            BuiltinFn::Round => (3, 3),
            // Длину можно не указывать — до конца строки.
            BuiltinFn::Mid => (2, 3),
            // Позиция по умолчанию — первая.
            BuiltinFn::CharCode => (1, 2),
            // Шаблон плюс до десяти значений.
            BuiltinFn::StrTemplate => (1, 1 + crate::string::MAX_TEMPLATE_ARGS),
            BuiltinFn::AddMonth => (2, 2),
            // `Дата(Год, Месяц, День[, Час, Минута, Секунда])` —
            // минимум три; строковая форма `Дата("...")` — это один
            // аргумент, поэтому нижняя граница всё-таки 1, а какая из двух
            // форм имелась в виду, решает тип первого аргумента в
            // `BslValue::make_date`.
            BuiltinFn::MakeDate => (1, 6),
            BuiltinFn::CurrentDate | BuiltinFn::CurrentUniversalDateInMilliseconds => (0, 0),
            // Оба списка свойств необязательны; недостающие позиции
            // резолвер добьёт `Неопределено`, что и значит «не задан».
            BuiltinFn::FillPropertyValues => (2, 4),
            // Функции восстановления и преобразования (последние три
            // параметра каждой у платформы) не поддержаны: встроенная
            // функция не умеет позвать пользовательскую — см. обзор
            // модуля `json`.
            BuiltinFn::ReadJson => (1, 3),
            BuiltinFn::WriteJson => (2, 2),
            _ => (1, 1),
        }
    }
}

/// Методы объектов, вызываемые как `а.Метод(...)`. `Добавить`/`Удалить`/
/// `Очистить` полиморфны по типу получателя в самой 1С (элемент массива,
/// строка таблицы, колонка, ...) — здесь это один идентификатор на все
/// смыслы, арность и поведение решает рантайм (см. `BslValue::push_element`
/// и соседние методы), а не резолвинг в `bsl-sema`, который не может знать
/// заранее, каким объектом окажется получатель.
///
/// Методы `ТаблицаЗначений` добавлялись волнами: волна 2 — `Найти`,
/// `НайтиСтроки`, `Сортировать`, `Итог`; волна 3 — `Скопировать`,
/// `СкопироватьКолонки`, `ВыгрузитьКолонку`, `ЗагрузитьКолонку`,
/// `Сдвинуть`, `Индекс`, `Свернуть`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMethod {
    Count,
    Add,
    Delete,
    Clear,
    /// `Структура.Вставить(Ключ, Значение)` / `Соответствие.Вставить(Ключ,
    /// Значение)` — тоже полиморфен по получателю, но это ДРУГОЙ метод в
    /// самой 1С, чем `Добавить` (разные имена, `Insert` не синоним `Add`),
    /// поэтому отдельный вариант, а не переиспользование `Add`.
    Insert,
    /// `Соответствие.Получить(Ключ)`.
    Get,
    /// `Структура.Свойство(Ключ[, ЗначениеПоУмолчанию])` — см. doc comment
    /// на `BslValue::structure_property` про отклонение от реальной
    /// сигнатуры (там `Значение` — выходной параметр, здесь — значение по
    /// умолчанию).
    Property,

    // --- ТаблицаЗначений, волна 2 -------------------------------------
    /// `Найти(Значение[, Колонки])`.
    Find,
    /// `НайтиСтроки(СтруктураПоиска)`.
    FindRows,
    /// `Сортировать("Кол1 Возр, Кол2 Убыв")`.
    Sort,
    /// `ЗаполнитьЗначения(Значение[, Колонки])`.
    FillValues,
    /// `Итог("Колонка")`.
    Total,

    // --- ТаблицаЗначений, волна 3 -------------------------------------
    /// `Скопировать([Строки], [Колонки])` -> новая таблица.
    Copy,
    /// `СкопироватьКолонки([Колонки])` -> пустая таблица той же структуры.
    CopyColumns,
    /// `ВыгрузитьКолонку(Колонка)` -> `Массив`.
    UnloadColumn,
    /// `ЗагрузитьКолонку(Массив, Колонка)`.
    LoadColumn,
    /// `Сдвинуть(Строка, Смещение)`.
    Move,
    /// `Индекс(Строка)` -> `Число`.
    IndexOf,
    /// `Свернуть(КолонкиГруппировки[, КолонкиСуммирования])`.
    Collapse,

    /// `ЗаписьТекста.Записать(Текст)` — добавляет текст в буфер файла.
    Write,
    /// `ЗаписьТекста.Закрыть()` — сбрасывает буфер и закрывает файл.
    /// У `ЗаписьJSON` тот же метод ОТДАЁТ накопленный текст, поэтому
    /// поведение выбирается по типу получателя, как у `Добавить`.
    Close,

    // --- JSON ----------------------------------------------------------
    /// `ЧтениеJSON.УстановитьСтроку(Текст)` и
    /// `ЗаписьJSON.УстановитьСтроку([Параметры])` — одно имя на два
    /// объекта, как в платформе.
    SetString,
    /// `ОткрытьФайл(Имя[, Параметры])` у обоих объектов JSON.
    OpenFile,
    /// `ЧтениеJSON.Прочитать()` / `ЧтениеXML.Прочитать()` -> `Булево`.
    /// Один вариант на два объекта: имя у платформы общее, а смысл
    /// («следующее значение» против «следующий узел») выбирается по
    /// получателю — как у `Закрыть` и `УстановитьСтроку`.
    ReadNext,
    /// `ЧтениеJSON.Пропустить()` / `ЧтениеXML.Пропустить()`. Имя именно
    /// такое: `ПропуститьЗначение` платформа не знает — измерено.
    SkipNode,
    WriteStartObject,
    WriteEndObject,
    WriteStartArray,
    WriteEndArray,
    WritePropertyName,
    /// `ЗаписьJSON.ЗаписатьЗначение(Значение)` — отдельный метод от
    /// `Записать` у `ЗаписьТекста`.
    WriteJsonValue,

    // --- XML -----------------------------------------------------------
    // `УстановитьСтроку`, `ОткрытьФайл` и `Закрыть` переиспользуются от
    // JSON: у платформы это те же имена, а смысл и там, и там выбирается
    // по получателю. `Прочитать` — НЕТ: у `ЧтениеJSON` он отдаёт следующее
    // ЗНАЧЕНИЕ, у `ЧтениеXML` — следующий УЗЕЛ, и получатель у них разный,
    // так что разделение проходит по типу объекта в одном варианте.
    /// `ЧтениеXML.ПрочитатьАтрибут()` -> `Булево` — курсор по атрибутам.
    XmlReadAttribute,
    /// `ЧтениеXML.КоличествоАтрибутов()`.
    XmlAttributeCount,
    /// `ЧтениеXML.ИмяАтрибута(Индекс)`.
    XmlAttributeName,
    /// `ЧтениеXML.ЗначениеАтрибута(ИмяЛибоИндекс)`.
    XmlAttributeValue,
    /// `ЧтениеXML.ПерейтиКСодержимому()` -> член `ТипУзлаXML`.
    XmlMoveToContent,
    WriteXmlDeclaration,
    WriteStartElement,
    WriteEndElement,
    WriteXmlAttribute,
    WriteXmlText,
    WriteXmlComment,
    WriteCdataSection,
    WriteXmlProcessingInstruction,
    WriteXmlRaw,

    // --- ТекстовыйДокумент ---------------------------------------------
    // `Прочитать`, `Записать` и `Очистить` переиспользуются: у платформы
    // это те же имена, что у JSON/XML/ЗаписьТекста, а смысл выбирается по
    // получателю.
    SetText,
    GetText,
    LineCount,
    GetLine,
    AddLine,
    InsertLine,
    ReplaceLine,
    DeleteLine,
    GetArea,
    /// `Вывести(Область)` — перехватывается в `bsl-vm`: подстановка
    /// параметров форматирует значения, а форматирование живёт в
    /// `bsl-format`, который зависит от этого крейта, не наоборот.
    OutputArea,

    // --- ТабличныйДокумент ----------------------------------------------
    /// `Область(...)` — ССЫЛКА на прямоугольник документа.
    Region,
    /// `Объединить()` у области ячеек.
    MergeCells,
    /// `Разъединить()` у области ячеек.
    UnmergeCells,
    /// `НачатьГруппуСтрок` / `ЗакончитьГруппуСтрок`.
    BeginRowGroup,
    EndRowGroup,
}

/// Написания МЕТОДОВ объектов — тот же принцип, что и у
/// [`BUILTIN_FN_NAMES`]: единственный источник и для поиска, и для
/// автодополнения после точки.
pub const BUILTIN_METHOD_NAMES: &[(&str, BuiltinMethod)] = &[
    ("Количество", BuiltinMethod::Count),
    ("Count", BuiltinMethod::Count),
    ("Добавить", BuiltinMethod::Add),
    ("Add", BuiltinMethod::Add),
    ("Удалить", BuiltinMethod::Delete),
    ("Delete", BuiltinMethod::Delete),
    ("Очистить", BuiltinMethod::Clear),
    ("Clear", BuiltinMethod::Clear),
    ("Вставить", BuiltinMethod::Insert),
    ("Insert", BuiltinMethod::Insert),
    ("Получить", BuiltinMethod::Get),
    ("Get", BuiltinMethod::Get),
    ("Свойство", BuiltinMethod::Property),
    ("Property", BuiltinMethod::Property),
    ("Найти", BuiltinMethod::Find),
    ("Find", BuiltinMethod::Find),
    ("НайтиСтроки", BuiltinMethod::FindRows),
    ("FindRows", BuiltinMethod::FindRows),
    ("Сортировать", BuiltinMethod::Sort),
    ("Sort", BuiltinMethod::Sort),
    ("ЗаполнитьЗначения", BuiltinMethod::FillValues),
    ("FillValues", BuiltinMethod::FillValues),
    ("Итог", BuiltinMethod::Total),
    ("Total", BuiltinMethod::Total),
    ("Скопировать", BuiltinMethod::Copy),
    ("Copy", BuiltinMethod::Copy),
    ("СкопироватьКолонки", BuiltinMethod::CopyColumns),
    ("CopyColumns", BuiltinMethod::CopyColumns),
    ("ВыгрузитьКолонку", BuiltinMethod::UnloadColumn),
    ("UnloadColumn", BuiltinMethod::UnloadColumn),
    ("ЗагрузитьКолонку", BuiltinMethod::LoadColumn),
    ("LoadColumn", BuiltinMethod::LoadColumn),
    ("Сдвинуть", BuiltinMethod::Move),
    ("Move", BuiltinMethod::Move),
    ("Индекс", BuiltinMethod::IndexOf),
    ("IndexOf", BuiltinMethod::IndexOf),
    ("Свернуть", BuiltinMethod::Collapse),
    ("GroupBy", BuiltinMethod::Collapse),
    ("Записать", BuiltinMethod::Write),
    ("Write", BuiltinMethod::Write),
    ("Закрыть", BuiltinMethod::Close),
    ("Close", BuiltinMethod::Close),
    ("УстановитьСтроку", BuiltinMethod::SetString),
    ("SetString", BuiltinMethod::SetString),
    ("ОткрытьФайл", BuiltinMethod::OpenFile),
    ("OpenFile", BuiltinMethod::OpenFile),
    ("Прочитать", BuiltinMethod::ReadNext),
    ("Read", BuiltinMethod::ReadNext),
    ("Пропустить", BuiltinMethod::SkipNode),
    ("Skip", BuiltinMethod::SkipNode),
    ("ЗаписатьНачалоОбъекта", BuiltinMethod::WriteStartObject),
    ("WriteStartObject", BuiltinMethod::WriteStartObject),
    ("ЗаписатьКонецОбъекта", BuiltinMethod::WriteEndObject),
    ("WriteEndObject", BuiltinMethod::WriteEndObject),
    ("ЗаписатьНачалоМассива", BuiltinMethod::WriteStartArray),
    ("WriteStartArray", BuiltinMethod::WriteStartArray),
    ("ЗаписатьКонецМассива", BuiltinMethod::WriteEndArray),
    ("WriteEndArray", BuiltinMethod::WriteEndArray),
    ("ЗаписатьИмяСвойства", BuiltinMethod::WritePropertyName),
    ("WritePropertyName", BuiltinMethod::WritePropertyName),
    ("ЗаписатьЗначение", BuiltinMethod::WriteJsonValue),
    ("WriteValue", BuiltinMethod::WriteJsonValue),
    ("ПрочитатьАтрибут", BuiltinMethod::XmlReadAttribute),
    ("ReadAttribute", BuiltinMethod::XmlReadAttribute),
    ("КоличествоАтрибутов", BuiltinMethod::XmlAttributeCount),
    ("AttributeCount", BuiltinMethod::XmlAttributeCount),
    ("ИмяАтрибута", BuiltinMethod::XmlAttributeName),
    ("AttributeName", BuiltinMethod::XmlAttributeName),
    ("ЗначениеАтрибута", BuiltinMethod::XmlAttributeValue),
    ("AttributeValue", BuiltinMethod::XmlAttributeValue),
    ("ПерейтиКСодержимому", BuiltinMethod::XmlMoveToContent),
    ("MoveToContent", BuiltinMethod::XmlMoveToContent),
    ("ЗаписатьОбъявлениеXML", BuiltinMethod::WriteXmlDeclaration),
    ("WriteXMLDeclaration", BuiltinMethod::WriteXmlDeclaration),
    ("ЗаписатьНачалоЭлемента", BuiltinMethod::WriteStartElement),
    ("WriteStartElement", BuiltinMethod::WriteStartElement),
    ("ЗаписатьКонецЭлемента", BuiltinMethod::WriteEndElement),
    ("WriteEndElement", BuiltinMethod::WriteEndElement),
    ("ЗаписатьАтрибут", BuiltinMethod::WriteXmlAttribute),
    ("WriteAttribute", BuiltinMethod::WriteXmlAttribute),
    ("ЗаписатьТекст", BuiltinMethod::WriteXmlText),
    ("WriteText", BuiltinMethod::WriteXmlText),
    ("ЗаписатьКомментарий", BuiltinMethod::WriteXmlComment),
    ("WriteComment", BuiltinMethod::WriteXmlComment),
    ("ЗаписатьСекциюCDATA", BuiltinMethod::WriteCdataSection),
    ("WriteCDATASection", BuiltinMethod::WriteCdataSection),
    (
        "ЗаписатьИнструкциюОбработки",
        BuiltinMethod::WriteXmlProcessingInstruction,
    ),
    (
        "WriteProcessingInstruction",
        BuiltinMethod::WriteXmlProcessingInstruction,
    ),
    ("ЗаписатьБезОбработки", BuiltinMethod::WriteXmlRaw),
    ("WriteRaw", BuiltinMethod::WriteXmlRaw),
    ("УстановитьТекст", BuiltinMethod::SetText),
    ("SetText", BuiltinMethod::SetText),
    ("ПолучитьТекст", BuiltinMethod::GetText),
    ("GetText", BuiltinMethod::GetText),
    ("КоличествоСтрок", BuiltinMethod::LineCount),
    ("LineCount", BuiltinMethod::LineCount),
    ("ПолучитьСтроку", BuiltinMethod::GetLine),
    ("GetLine", BuiltinMethod::GetLine),
    ("ДобавитьСтроку", BuiltinMethod::AddLine),
    ("AddLine", BuiltinMethod::AddLine),
    ("ВставитьСтроку", BuiltinMethod::InsertLine),
    ("InsertLine", BuiltinMethod::InsertLine),
    ("ЗаменитьСтроку", BuiltinMethod::ReplaceLine),
    ("ReplaceLine", BuiltinMethod::ReplaceLine),
    ("УдалитьСтроку", BuiltinMethod::DeleteLine),
    ("DeleteLine", BuiltinMethod::DeleteLine),
    ("ПолучитьОбласть", BuiltinMethod::GetArea),
    ("GetArea", BuiltinMethod::GetArea),
    ("Вывести", BuiltinMethod::OutputArea),
    ("Output", BuiltinMethod::OutputArea),
    ("Область", BuiltinMethod::Region),
    ("Area", BuiltinMethod::Region),
    ("Объединить", BuiltinMethod::MergeCells),
    ("Merge", BuiltinMethod::MergeCells),
    ("Разъединить", BuiltinMethod::UnmergeCells),
    ("Unmerge", BuiltinMethod::UnmergeCells),
    ("НачатьГруппуСтрок", BuiltinMethod::BeginRowGroup),
    ("StartRowGroup", BuiltinMethod::BeginRowGroup),
    ("ЗакончитьГруппуСтрок", BuiltinMethod::EndRowGroup),
    ("EndRowGroup", BuiltinMethod::EndRowGroup),
];

impl BuiltinMethod {
    /// Регистронезависимый поиск по [`BUILTIN_METHOD_NAMES`].
    pub fn lookup(name: &str) -> Option<Self> {
        let upper = name.to_uppercase();
        BUILTIN_METHOD_NAMES
            .iter()
            .find(|(n, _)| n.to_uppercase() == upper)
            .map(|(_, m)| *m)
    }
}

pub fn call_builtin_fn(f: BuiltinFn, args: &[BslValue]) -> RtResult<BslValue> {
    match f {
        BuiltinFn::Sqrt => args[0].sqrt(),
        BuiltinFn::Pow => args[0].pow(&args[1]),
        BuiltinFn::Ln => args[0].ln(),
        BuiltinFn::Log10 => args[0].log10(),
        BuiltinFn::Exp => args[0].exp(),
        BuiltinFn::Sin => args[0].sin(),
        BuiltinFn::Cos => args[0].cos(),
        BuiltinFn::Tan => args[0].tan(),
        BuiltinFn::Asin => args[0].asin(),
        BuiltinFn::Acos => args[0].acos(),
        BuiltinFn::Atan => args[0].atan(),
        BuiltinFn::Round => args[0].round(&args[1], &args[2]),
        BuiltinFn::Trunc => args[0].trunc(),
        BuiltinFn::Message => {
            println!("{}", args[0]);
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ToString | BuiltinFn::Format | BuiltinFn::ToNumber => {
            unreachable!(
                "форматозависимые builtin'ы (Строка/Формат/Число) перехватываются в bsl-vm, \
                 у которого есть доступ к bsl-format — сюда попадать не должны"
            )
        }
        BuiltinFn::StrLen => Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
            args[0].str_len()? as i64,
        ))),
        BuiltinFn::Left => args[0].str_left(&args[1]),
        BuiltinFn::Right => args[0].str_right(&args[1]),
        BuiltinFn::Mid => args[0].str_mid(&args[1], &args[2]),
        BuiltinFn::Upper => args[0].str_upper(),
        BuiltinFn::Lower => args[0].str_lower(),
        BuiltinFn::TrimAll => args[0].str_trim_all(),
        BuiltinFn::TrimLeft => args[0].str_trim_left(),
        BuiltinFn::TrimRight => args[0].str_trim_right(),
        BuiltinFn::StrFind => args[0].str_find(&args[1]),
        BuiltinFn::StrReplace => args[0].str_replace(&args[1], &args[2]),
        BuiltinFn::StrSplit => args[0].str_split(&args[1]),
        BuiltinFn::StrConcat => args[0].str_join(&args[1]),
        BuiltinFn::StrLineCount => args[0].str_line_count(),
        BuiltinFn::StrGetLine => args[0].str_get_line(&args[1]),
        BuiltinFn::StrTemplate => args[0].str_template(&args[1..]),
        BuiltinFn::Char => args[0].char_from_code(),
        BuiltinFn::CharCode => args[0].char_code(&args[1]),
        BuiltinFn::ValueIsFilled => Ok(BslValue::Boolean(args[0].is_filled()?)),
        BuiltinFn::TypeOf => args[0].type_of(),
        BuiltinFn::TypeByName => args[0].type_by_name(),
        BuiltinFn::MakeDate => BslValue::make_date(args),
        BuiltinFn::CurrentDate => BslValue::current_date(),
        BuiltinFn::CurrentUniversalDateInMilliseconds => {
            BslValue::current_universal_date_in_milliseconds()
        }
        BuiltinFn::DatePartOf(part) => args[0].date_component(part),
        BuiltinFn::DateBoundaryOf(which) => args[0].date_boundary(which),
        BuiltinFn::AddMonth => args[0].add_month(&args[1]),
        // Перехвачена в `call_builtin_fn_ctx` — без таблицы имён набор
        // полей приёмника не прочитать. Ошибка, а не `unreachable!`: эта
        // функция публична, и ронять процесс на прямом вызове из
        // встраивающего приложения незачем (то же соображение, что и у
        // `RtError::InvalidBytecode`).
        BuiltinFn::ReadJson | BuiltinFn::WriteJson => Err(RtError::InvalidBytecode(
            "функции JSON требуют контекста имён: вызывайте call_builtin_fn_ctx",
        )),
        BuiltinFn::FillPropertyValues => Err(RtError::InvalidBytecode(
            "ЗаполнитьЗначенияСвойств требует контекста имён: вызывайте call_builtin_fn_ctx",
        )),
    }
}

/// Обёртка над [`call_builtin_fn`] с доступом к рантайм-контексту форм —
/// ровно тот же приём, что и [`call_builtin_method_ctx`] для методов
/// структуры, и по той же причине: единственной встроенной функции
/// (`ЗаполнитьЗначенияСвойств`) нужна таблица имён, чтобы сопоставить
/// свойства двух объектов. Всё остальное делегируется как есть.
///
/// # Errors
///
/// Ошибку самой встроенной функции; [`RtError::InvalidBytecode`], если
/// аргументов пришло не столько, сколько требует арность.
pub fn call_builtin_fn_ctx(
    f: BuiltinFn,
    args: &[BslValue],
    rt: &mut RuntimeShapes,
) -> RtResult<BslValue> {
    if f == BuiltinFn::FillPropertyValues {
        let [target, source, list, exclude] = args else {
            return Err(RtError::InvalidBytecode(
                "ЗаполнитьЗначенияСвойств вызвана не с четырьмя аргументами",
            ));
        };
        // Только чтение таблицы имён: набор полей приёмника не растёт
        // (измерено, см. обзор модуля `fill`), а значит и новых `NameId`
        // заводить не нужно.
        crate::fill::fill_property_values(target, source, list, exclude, &rt.names)?;
        return Ok(BslValue::Undefined);
    }
    if f == BuiltinFn::ReadJson {
        let as_map = match args.get(1) {
            None | Some(BslValue::Undefined) => false,
            Some(BslValue::Boolean(b)) => *b,
            Some(_) => {
                return Err(RtError::TypeError {
                    expected: "Булево",
                    op: "ПрочитатьJSON(ВозвращатьСоответствие)",
                })
            }
        };
        // Имена свойств с датами приходят массивом строк.
        let mut date_names: Vec<String> = Vec::new();
        match args.get(2) {
            None | Some(BslValue::Undefined) => {}
            Some(list) => {
                let len = list.collection_len().map_err(|_| RtError::TypeError {
                    expected: "Массив",
                    op: "ПрочитатьJSON(ИменаСвойствСоЗначениямиДата)",
                })?;
                for i in 0..len {
                    let item = list.get_index(
                        &BslValue::Number(bsl_number::BslNumber::from_i64(i as i64)),
                        &rt.names,
                    )?;
                    if let BslValue::Str(s) = item {
                        date_names.push(s.to_string());
                    }
                }
            }
        }
        return crate::json::read_json(&args[0], as_map, &date_names, rt);
    }
    if f == BuiltinFn::WriteJson {
        crate::json::write_json(&args[0], &args[1], rt)?;
        return Ok(BslValue::Undefined);
    }
    call_builtin_fn(f, args)
}

/// Необязательный аргумент метода: отсутствующий читается как
/// `Неопределено` — ровно так же, как резолвер выравнивает арность
/// встроенных ФУНКЦИЙ (см. `BuiltinFn::arity_range`). У методов получатель
/// динамический, поэтому выравнивать приходится здесь.
fn arg(args: &[BslValue], i: usize) -> &BslValue {
    args.get(i).unwrap_or(&BslValue::Undefined)
}

/// Лишние аргументы у метода с переменной арностью. Тихо игнорировать их
/// нельзя: `Свернуть("а", "б", "в")` — почти наверняка опечатка, а не
/// намерение.
fn too_many(obj: &BslValue, method: &'static str, args: &[BslValue], max: usize) -> RtResult<()> {
    if args.len() > max {
        return Err(RtError::MethodNotApplicable {
            method,
            receiver: obj.type_name(),
        });
    }
    Ok(())
}

/// Арность `Count`/`Delete`/`Clear` не зависит от получателя и уже
/// проверена в `bsl-sema`; арность `Add` — зависит (0 для строки таблицы,
/// 1 для элемента массива/колонки), поэтому здесь просто читаем
/// `args.len()` и решаем сами, а не полагаемся на проверку выше по стеку.
pub fn call_builtin_method(
    m: BuiltinMethod,
    obj: &BslValue,
    args: &[BslValue],
) -> RtResult<BslValue> {
    match m {
        BuiltinMethod::Count => {
            let len = obj.collection_len()?;
            Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
                len as i64,
            )))
        }
        BuiltinMethod::Add => match args {
            _ if crate::spreadsheet::is_drawings(obj) => crate::spreadsheet::drawings_add(obj, args),
            [] => obj.table_add_row(),
            [v] => match obj.push_element(v.clone()) {
                Ok(()) => Ok(BslValue::Undefined),
                Err(crate::RtError::MethodNotApplicable { .. }) => {
                    obj.table_add_column(v, &BslValue::Undefined)?;
                    Ok(BslValue::Undefined)
                }
                Err(e) => Err(e),
            },
            [name, value_type] => {
                obj.table_add_column(name, value_type)?;
                Ok(BslValue::Undefined)
            }
            _ => Err(crate::RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: obj.type_name(),
            }),
        },
        BuiltinMethod::Delete => {
            obj.delete_element(&args[0])?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Clear => {
            if crate::spreadsheet::is_spread_document(obj) {
                crate::spreadsheet::clear(obj)?;
            } else if crate::textdoc::is_text_document(obj) {
                crate::textdoc::clear(obj)?;
            } else {
                obj.clear_collection()?;
            }
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Insert => match obj {
            BslValue::Object(o) if matches!(&**o, BslObject::Map(_)) => {
                obj.map_insert(args[0].clone(), args[1].clone())?;
                Ok(BslValue::Undefined)
            }
            // `Структура.Вставить` доходит сюда, только если `obj` НЕ
            // структура — сам структурный случай перехвачен раньше, в
            // `call_builtin_method_ctx` (нужен рантайм-контекст форм).
            _ => Err(RtError::MethodNotApplicable {
                method: "Вставить",
                receiver: obj.type_name(),
            }),
        },
        BuiltinMethod::Get => match obj {
            BslValue::Object(o) if matches!(&**o, BslObject::Map(_)) => obj.map_get(&args[0]),
            _ => Err(RtError::MethodNotApplicable {
                method: "Получить",
                receiver: obj.type_name(),
            }),
        },
        // `Структура.Свойство` перехвачен в `call_builtin_method_ctx` —
        // сюда попадает только вызов на не-структуре.
        BuiltinMethod::Property => Err(RtError::MethodNotApplicable {
            method: "Свойство",
            receiver: obj.type_name(),
        }),
        // Второй аргумент `Найти` необязателен: `args.get(1)` вместо
        // `args[1]`, потому что арность метода не выравнивается резолвером
        // (в отличие от встроенных ФУНКЦИЙ — см. `BuiltinFn::arity_range`:
        // у методов получатель динамический, и арность проверяет рантайм).
        BuiltinMethod::Find => {
            // Арность `Найти` резолвер не проверяет (1 или 2 — список
            // колонок необязателен), поэтому нулевой вызов обязан стать
            // понятной ошибкой, а не паникой на `args[0]` — ровно как у
            // `Свойство` ниже.
            let Some(value) = args.first() else {
                return Err(RtError::MethodNotApplicable {
                    method: "Найти",
                    receiver: obj.type_name(),
                });
            };
            if args.len() > 2 {
                return Err(RtError::MethodNotApplicable {
                    method: "Найти",
                    receiver: obj.type_name(),
                });
            }
            obj.table_find(value, args.get(1).unwrap_or(&BslValue::Undefined))
        }
        // `НайтиСтроки` перехвачен в `call_builtin_method_ctx` — ему нужен
        // интернер имён, чтобы прочитать поля структуры поиска.
        BuiltinMethod::FindRows => Err(RtError::MethodNotApplicable {
            method: "НайтиСтроки",
            receiver: obj.type_name(),
        }),
        BuiltinMethod::Sort => {
            too_many(obj, "Сортировать", args, 2)?;
            let Some(spec) = args.first() else {
                return Err(RtError::MethodNotApplicable {
                    method: "Сортировать",
                    receiver: obj.type_name(),
                });
            };
            obj.table_sort(spec, arg(args, 1))?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::FillValues => {
            too_many(obj, "ЗаполнитьЗначения", args, 2)?;
            let Some(value) = args.first() else {
                return Err(RtError::MethodNotApplicable {
                    method: "ЗаполнитьЗначения",
                    receiver: obj.type_name(),
                });
            };
            obj.table_fill_values(value, arg(args, 1))?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Total => obj.table_total(&args[0]),

        // Волна 3. У `Скопировать`/`СкопироватьКолонки`/`Свернуть` арность
        // переменная, поэтому и здесь `args.get(...)`, а лишние аргументы
        // ловятся тут же: резолвер для них проверку не делает.
        BuiltinMethod::Copy => {
            too_many(obj, "Скопировать", args, 2)?;
            obj.table_copy(arg(args, 0), arg(args, 1))
        }
        BuiltinMethod::CopyColumns => {
            too_many(obj, "СкопироватьКолонки", args, 1)?;
            obj.table_copy_columns(arg(args, 0))
        }
        BuiltinMethod::UnloadColumn => obj.table_unload_column(&args[0]),
        BuiltinMethod::LoadColumn => {
            obj.table_load_column(&args[0], &args[1])?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Move => {
            obj.table_move(&args[0], &args[1])?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::IndexOf => obj.table_index_of(&args[0]),
        BuiltinMethod::Collapse => {
            too_many(obj, "Свернуть", args, 2)?;
            let Some(group) = args.first() else {
                return Err(RtError::MethodNotApplicable {
                    method: "Свернуть",
                    receiver: obj.type_name(),
                });
            };
            obj.table_collapse(group, arg(args, 1))?;
            Ok(BslValue::Undefined)
        }
        // `Записать` у `ЗаписьТекста` — дописать кусок, у
        // `ТекстовыйДокумент` — сохранить файл. Одно имя, разный смысл по
        // получателю, как и у `Закрыть`.
        BuiltinMethod::Write => {
            if crate::spreadsheet::is_spread_document(obj) {
                crate::spreadsheet::write(obj, args)?;
                Ok(BslValue::Undefined)
            } else if crate::textdoc::is_text_document(obj) {
                crate::textdoc::write_file(obj, args)?;
                Ok(BslValue::Undefined)
            } else {
                obj.text_writer_write(&args[0])
            }
        }
        // `Закрыть` полиморфен: у `ЗаписьТекста` он ничего не возвращает,
        // у `ЗаписьJSON` — отдаёт накопленный текст.
        BuiltinMethod::Close => obj.close_object(),

        // Имя метода общее для JSON и XML — ветвление по получателю, а не
        // по имени: у платформы это ровно один метод.
        BuiltinMethod::SetString => {
            if crate::xml::is_xml_reader(obj) || crate::xml::is_xml_writer(obj) {
                crate::xml::set_string(obj, args)?;
            } else {
                crate::json::set_string(obj, args)?;
            }
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::OpenFile => {
            if crate::xml::is_xml_reader(obj) || crate::xml::is_xml_writer(obj) {
                crate::xml::open_file(obj, args)?;
            } else {
                crate::json::open_file(obj, args)?;
            }
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::ReadNext => {
            if crate::spreadsheet::is_spread_document(obj) {
                crate::spreadsheet::read(obj, args)?;
                Ok(BslValue::Undefined)
            } else if crate::textdoc::is_text_document(obj) {
                // У документа `Прочитать(Путь)` — загрузка файла, а не шаг
                // по потоку.
                crate::textdoc::read_file(obj, args)?;
                Ok(BslValue::Undefined)
            } else if crate::xml::is_xml_reader(obj) {
                crate::xml::read(obj)
            } else {
                Ok(BslValue::Boolean(crate::json::read(obj)?))
            }
        }
        BuiltinMethod::SkipNode => {
            if crate::xml::is_xml_reader(obj) {
                crate::xml::skip(obj)?;
            } else {
                crate::json::skip(obj)?;
            }
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::SetText => {
            crate::textdoc::set_text(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::GetText => crate::textdoc::get_text(obj),
        BuiltinMethod::LineCount => crate::textdoc::line_count(obj),
        BuiltinMethod::GetLine => crate::textdoc::get_line(obj, args),
        BuiltinMethod::AddLine => {
            crate::textdoc::add_line(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::InsertLine => {
            crate::textdoc::insert_line(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::ReplaceLine => {
            crate::textdoc::replace_line(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::DeleteLine => {
            crate::textdoc::delete_line(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::GetArea => {
            if crate::spreadsheet::is_spread_document(obj) {
                crate::spreadsheet::get_area(obj, args)
            } else {
                crate::textdoc::get_area(obj, args)
            }
        }
        BuiltinMethod::Region => crate::spreadsheet::region(obj, args),
        BuiltinMethod::MergeCells => {
            crate::spreadsheet::merge_cells(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::UnmergeCells => {
            crate::spreadsheet::unmerge_cells(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::BeginRowGroup => {
            crate::spreadsheet::begin_row_group(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::EndRowGroup => {
            crate::spreadsheet::end_row_group(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::OutputArea => Err(RtError::MethodNotApplicable {
            method: "Вывести",
            receiver: obj.type_name(),
        }),
        BuiltinMethod::XmlReadAttribute => crate::xml::read_attribute(obj),
        BuiltinMethod::XmlAttributeCount => crate::xml::attribute_count(obj),
        BuiltinMethod::XmlAttributeName => crate::xml::attribute_name(obj, args),
        BuiltinMethod::XmlAttributeValue => crate::xml::attribute_value(obj, args),
        BuiltinMethod::XmlMoveToContent => crate::xml::move_to_content(obj),
        BuiltinMethod::WriteXmlDeclaration => {
            crate::xml::write_declaration(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteStartElement => {
            crate::xml::write_start_element(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteEndElement => {
            crate::xml::write_end_element(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteXmlAttribute => {
            crate::xml::write_attribute(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteXmlText => {
            crate::xml::write_text(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteXmlComment => {
            crate::xml::write_comment(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteCdataSection => {
            crate::xml::write_cdata(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteXmlProcessingInstruction => {
            crate::xml::write_processing_instruction(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteXmlRaw => {
            crate::xml::write_raw(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteStartObject => {
            crate::json::write_start_object(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteEndObject => {
            crate::json::write_end_object(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteStartArray => {
            crate::json::write_start_array(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteEndArray => {
            crate::json::write_end_array(obj)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WritePropertyName => {
            crate::json::write_property_name(obj, args)?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::WriteJsonValue => {
            crate::json::write_value(obj, args)?;
            Ok(BslValue::Undefined)
        }
    }
}

fn is_structure(obj: &BslValue) -> bool {
    matches!(obj, BslValue::Object(o) if matches!(&**o, BslObject::Structure(_)))
}

fn key_name(key: &BslValue, rt: &mut RuntimeShapes) -> RtResult<NameId> {
    match key {
        BslValue::Str(s) => Ok(rt.names.intern(&s.to_string())),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op: "Ключ",
        }),
    }
}

/// Обёртка над `call_builtin_method` с доступом к рантайм-контексту форм
/// (`RuntimeShapes`) — нужен только трём методам структуры, которые МЕНЯЮТ
/// её форму (`Вставить`/`Удалить`/`Свойство` — точнее, только первые два
/// меняют, `Свойство` лишь читает, но ключ всё равно нужно интернировать в
/// `NameId` тем же рантайм-интернером) плюс `Очистить` на структуре (сброс
/// формы в пустую). Для всего остального — просто делегирование в
/// контекст-независимую `call_builtin_method`, включая `Соответствие`
/// (`MapData` вообще не участвует в системе форм) и все остальные типы
/// получателей.
pub fn call_builtin_method_ctx(
    m: BuiltinMethod,
    obj: &BslValue,
    args: &[BslValue],
    rt: &mut RuntimeShapes,
) -> RtResult<BslValue> {
    if is_structure(obj) {
        match m {
            BuiltinMethod::Insert => {
                let field = key_name(&args[0], rt)?;
                obj.structure_insert(field, args[1].clone(), &mut rt.shapes)?;
                return Ok(BslValue::Undefined);
            }
            BuiltinMethod::Delete => {
                let field = key_name(&args[0], rt)?;
                obj.structure_delete(field, &mut rt.shapes)?;
                return Ok(BslValue::Undefined);
            }
            BuiltinMethod::Property => {
                // Арность у `Свойство` не проверена в bsl-sema (как и у
                // `Add`) — 1 или 2 аргумента оба валидны, но 0 или >2
                // синтаксически пройдут резолвинг и должны стать понятной
                // `RtError`, а не паникой на `args[0]`.
                let Some(key_arg) = args.first() else {
                    return Err(RtError::MethodNotApplicable {
                        method: "Свойство",
                        receiver: obj.type_name(),
                    });
                };
                if args.len() > 2 {
                    return Err(RtError::MethodNotApplicable {
                        method: "Свойство",
                        receiver: obj.type_name(),
                    });
                }
                let field = key_name(key_arg, rt)?;
                let default = args.get(1).cloned();
                return obj.structure_property(field, default);
            }
            BuiltinMethod::Clear => {
                obj.structure_clear(&mut rt.shapes)?;
                return Ok(BslValue::Undefined);
            }
            _ => {}
        }
    }
    // `НайтиСтроки` и перегрузка `Скопировать(Отбор, ...)` читают ИМЕНА
    // полей структуры (они хранятся `NameId`) и сопоставляют их с именами
    // колонок таблицы.
    if m == BuiltinMethod::FindRows {
        let Some(criteria) = args.first() else {
            return Err(RtError::MethodNotApplicable {
                method: "НайтиСтроки",
                receiver: obj.type_name(),
            });
        };
        return obj.table_find_rows(criteria, &rt.names);
    }
    if m == BuiltinMethod::Copy
        && matches!(
            args.first(),
            Some(BslValue::Object(value))
                if matches!(&**value, BslObject::Structure(_))
        )
    {
        too_many(obj, "Скопировать", args, 2)?;
        return obj.table_copy_by_filter(&args[0], arg(args, 1), &rt.names);
    }
    call_builtin_method(m, obj, args)
}

#[cfg(test)]
mod name_table_tests {
    use super::*;

    /// Таблица имён — источник и для поиска, и для автодополнения REPL;
    /// если она разъедется с реальностью, автодополнение начнёт предлагать
    /// то, чего нет. Здесь проверяется ровно это: каждое имя резолвится в
    /// заявленный вариант, регистр не значим, повторов нет.
    #[test]
    fn every_builtin_name_resolves_to_its_own_variant() {
        for (name, expected) in BUILTIN_FN_NAMES {
            assert_eq!(BuiltinFn::lookup(name), Some(*expected), "{name}");
            assert_eq!(BuiltinFn::lookup(&name.to_uppercase()), Some(*expected));
            assert_eq!(BuiltinFn::lookup(&name.to_lowercase()), Some(*expected));
        }
        for (name, expected) in BUILTIN_METHOD_NAMES {
            assert_eq!(BuiltinMethod::lookup(name), Some(*expected), "{name}");
            assert_eq!(BuiltinMethod::lookup(&name.to_uppercase()), Some(*expected));
            assert_eq!(BuiltinMethod::lookup(&name.to_lowercase()), Some(*expected));
        }
    }

    #[test]
    fn builtin_names_are_unique() {
        for names in [
            BUILTIN_FN_NAMES
                .iter()
                .map(|(n, _)| n.to_uppercase())
                .collect::<Vec<_>>(),
            BUILTIN_METHOD_NAMES
                .iter()
                .map(|(n, _)| n.to_uppercase())
                .collect::<Vec<_>>(),
        ] {
            let mut sorted = names.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), names.len(), "повтор имени: {names:?}");
        }
    }

    /// Обратная сторона: имя, которого в таблице нет, не должно
    /// резолвиться. Иначе `lookup` где-то ловил бы лишнее.
    #[test]
    fn unknown_names_do_not_resolve() {
        for name in ["Опечатка", "СтрНайтиИ", "", "Свернуть2"] {
            assert_eq!(BuiltinFn::lookup(name), None, "{name}");
        }
        assert_eq!(BuiltinMethod::lookup("Опечатка"), None);
    }
}
