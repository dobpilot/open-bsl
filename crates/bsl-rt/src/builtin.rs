use crate::date::{DateBoundary, DatePart};
use crate::runtime_shapes::RuntimeShapes;
use crate::{BslObject, BslString, BslValue, NameId, RtError, RtResult};

/// Аргументы командной строки, переданные скрипту после его имени.
/// Процессно-глобальное неизменяемое состояние: командная строка у
/// процесса одна, поэтому `bsl-cli` выставляет её один раз при старте, а
/// REPL и `Выполнить`/`Вычислить` видят те же аргументы без протаскивания
/// через `Program` — компилированная программа про своё окружение знать
/// не должна.
static COMMAND_LINE_ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Выставляет аргументы для [`BuiltinFn::CommandLineArguments`]. Зовётся
/// встраивающим приложением один раз до исполнения; повторный вызов
/// игнорируется — командная строка за время жизни процесса не меняется.
pub fn set_command_line_args(args: Vec<String>) {
    let _ = COMMAND_LINE_ARGS.set(args);
}

fn command_line_args() -> &'static [String] {
    COMMAND_LINE_ARGS.get().map_or(&[], Vec::as_slice)
}

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
    /// `ЗаписатьJSON(Запись, Значение[, Настройки[, ИмяФункции[,
    /// ДополнительныеПараметры[, ВызовКонтекстногоМетода]]]])` — тот же
    /// контекст нужен, чтобы прочитать ИМЕНА полей сериализуемой структуры.
    /// Функции преобразования (три последних параметра) появятся на
    /// этапе 1 плана (`docs/std-library-plan.md`) — непустой аргумент на
    /// этих позициях сейчас даёт понятную ошибку, а не молчаливый пропуск.
    WriteJson,

    /// `ЗаписатьДатуJSON(Дата, Формат[, Вариант])` -> `Строка`. Контекста
    /// имён не требует — сама по себе функция дат, без структур.
    WriteJsonDate,
    /// `ПрочитатьДатуJSON(Строка, Формат)` -> `Дата`.
    ReadJsonDate,
    /// `ЗаписатьЗначениеJSON(Значение)` -> `Строка`. Контекст нужен затем
    /// же, зачем `ЗаписатьJSON`, — сериализовать структуру.
    WriteJsonValue,
    /// `ПрочитатьЗначениеJSON(Строка)` -> значение. Контекст нужен затем
    /// же, зачем `ПрочитатьJSON`.
    ReadJsonValue,

    /// `ЗначениеВСтрокуВнутр(Значение)` — внутренний строковый формат
    /// платформы (см. модуль `vstr`). Контекст имён нужен затем же, зачем
    /// `ЗаписатьJSON`: прочитать имена полей структуры.
    ValueToStringInternal,
    /// `ЗначениеИзСтрокиВнутр(Строка)` — обратный разбор; контекст нужен
    /// на запись: поля структур интернируются, формы растут.
    ValueFromStringInternal,

    /// `ЗначениеВФайл(ИмяФайла, Значение)` — строка `ЗначениеВСтрокуВнутр`
    /// в UTF-8 С BOM и переводами строк CRLF (измерено побайтовым
    /// сравнением с файлами платформы, см. `vstr::value_to_file`).
    ValueToFile,
    /// `ЗначениеИзФайла(ИмяФайла)` — обратное чтение того же файла.
    ValueFromFile,

    /// `РазделитьДвоичныеДанные(Данные, РазмерЧасти)` -> `Массив` частей
    /// (см. [`BslValue::binary_data_split`]).
    SplitBinaryData,
    /// `СоединитьДвоичныеДанные(Массив)` -> склеенные данные (см.
    /// [`BslValue::binary_data_combine`]). Английское написание —
    /// `ConcatBinaryData`: ИЗМЕРЕНО пробой (`CombineBinaryData`,
    /// `MergeBinaryData` и `JoinBinaryData` платформа не знает), а не
    /// угадано по русскому имени.
    ConcatBinaryData,

    /// `АргументыКоманднойСтроки` — массив строк, переданных скрипту после
    /// его имени в командной строке. В 1С такой глобальной функции нет —
    /// это расширение по образцу OneScript, мерить его не на чем; резолвер
    /// принимает и запись без скобок, как у свойства глобального контекста
    /// oscript. В отличие от его `ФиксированногоМассива` возвращается
    /// обычный `Массив`, построенный заново на каждое чтение, — правки не
    /// переживают следующее обращение.
    CommandLineArguments,
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
    ("ЗаписатьДатуJSON", BuiltinFn::WriteJsonDate),
    ("WriteJSONDate", BuiltinFn::WriteJsonDate),
    ("ПрочитатьДатуJSON", BuiltinFn::ReadJsonDate),
    ("ReadJSONDate", BuiltinFn::ReadJsonDate),
    ("ЗаписатьЗначениеJSON", BuiltinFn::WriteJsonValue),
    ("WriteJSONValue", BuiltinFn::WriteJsonValue),
    ("ПрочитатьЗначениеJSON", BuiltinFn::ReadJsonValue),
    ("ReadJSONValue", BuiltinFn::ReadJsonValue),
    ("ЗначениеВСтрокуВнутр", BuiltinFn::ValueToStringInternal),
    ("ValueToStringInternal", BuiltinFn::ValueToStringInternal),
    ("ЗначениеИзСтрокиВнутр", BuiltinFn::ValueFromStringInternal),
    (
        "ValueFromStringInternal",
        BuiltinFn::ValueFromStringInternal,
    ),
    ("ЗначениеВФайл", BuiltinFn::ValueToFile),
    ("ValueToFile", BuiltinFn::ValueToFile),
    ("ЗначениеИзФайла", BuiltinFn::ValueFromFile),
    ("ValueFromFile", BuiltinFn::ValueFromFile),
    ("АргументыКоманднойСтроки", BuiltinFn::CommandLineArguments),
    ("CommandLineArguments", BuiltinFn::CommandLineArguments),
    ("РазделитьДвоичныеДанные", BuiltinFn::SplitBinaryData),
    ("SplitBinaryData", BuiltinFn::SplitBinaryData),
    ("СоединитьДвоичныеДанные", BuiltinFn::ConcatBinaryData),
    ("ConcatBinaryData", BuiltinFn::ConcatBinaryData),
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
            BuiltinFn::CurrentDate
            | BuiltinFn::CurrentUniversalDateInMilliseconds
            | BuiltinFn::CommandLineArguments => (0, 0),
            // Оба списка свойств необязательны; недостающие позиции
            // резолвер добьёт `Неопределено`, что и значит «не задан».
            BuiltinFn::FillPropertyValues => (2, 4),
            // Полные арности платформы поддержаны целиком: `ПрочитатьJSON` —
            // 8 позиций, `ЗаписатьJSON` — 6. Хвостовые позиции обеих — это
            // функция восстановления и функция преобразования со своими
            // параметрами: имя, модуль, дополнительные параметры, а у чтения
            // ещё и `ИменаСвойствДляФункцииВосстановления`. Разбирают их
            // `read_json_builtin`/`write_json_builtin`, а зовут по имени через
            // контекст исполнения, который даёт VM в
            // `call_builtin_with_format` (интерпретатор и JIT проходят одной
            // точкой).
            //
            // Про сами хвостовые позиции ИЗМЕРЕНО две вещи. Функция
            // преобразования зовётся ЛЕНИВО: одно её имя ничего не меняет,
            // пока не встретилось значение, которое сериализовать нечем (см.
            // `bsl_rt::json::write_json`). И `ПрочитатьJSON` отвергает ЯВНОЕ
            // `Неопределено` в четвёртой позиции (`ОжидаемыйФорматДаты`) —
            // «Несоответствие типов (параметр номер '4')», — тогда как
            // ПРОПУСК той же позиции принимает; здесь резолвер добивает
            // пропущенные позиции тем же `Неопределено`, поэтому различие не
            // воспроизводится (см.
            // `bsl_rt::json::optional_date_format_from_arg`).
            BuiltinFn::ReadJson => (1, 8),
            BuiltinFn::WriteJson => (2, 6),
            BuiltinFn::WriteJsonDate => (2, 3),
            BuiltinFn::ReadJsonDate => (2, 2),
            BuiltinFn::WriteJsonValue | BuiltinFn::ReadJsonValue => (1, 1),
            BuiltinFn::ValueToStringInternal | BuiltinFn::ValueFromStringInternal => (1, 1),
            BuiltinFn::ValueToFile => (2, 2),
            BuiltinFn::ValueFromFile => (1, 1),
            // Обе арности строгие: платформа отвергает и
            // `РазделитьДвоичныеДанные` с одним аргументом, и
            // `СоединитьДвоичныеДанные` без аргументов (пробы
            // `BIN.SPLIT.ONEARG`, `BIN.COMBINE.NOARG`).
            BuiltinFn::SplitBinaryData => (2, 2),
            BuiltinFn::ConcatBinaryData => (1, 1),
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

    // --- ДвоичныеДанные --------------------------------------------------
    /// `ДвоичныеДанные.Размер()` — число байтов. Не `Количество()`:
    /// двоичные данные не коллекция, и имя у метода своё.
    ///
    /// У `БуферДвоичныхДанных` `Размер` — наоборот, СВОЙСТВО (см.
    /// `BslValue::get_field_by_name`), и вызов со скобками на нём ошибка:
    /// измерено, что платформа отвергает `Буфер.Размер()`.
    Size,

    // --- БуферДвоичныхДанных ---------------------------------------------
    /// `Установить(Позиция, Значение)` — то же, что `Буфер[Позиция] = ...`.
    /// Пары `Получить`/`Get` у буфера нет своей: он делит `BuiltinMethod::Get`
    /// с `Соответствие`, и получатель разводится в рантайме.
    BufSet,
    /// `ПрочитатьЦелое16/32/64` и парные `ЗаписатьЦелое16/32/64`.
    /// Восьмибитных методов у платформы НЕТ ни в каком написании
    /// (проверено перебором `ПрочитатьЦелое8`, `ReadInt8`, `ПрочитатьБайт`,
    /// `ПолучитьБайт`) — один байт берётся индексом либо `Получить`.
    ReadInt16,
    ReadInt32,
    ReadInt64,
    WriteInt16,
    WriteInt32,
    WriteInt64,
    /// `Разделить(Разделитель)` — раскрой по вхождениям БУФЕРА-разделителя,
    /// а не нарезка на куски заданной длины (измерено: число платформа
    /// отвергает).
    BufSplit,
    /// `Соединить(Другой)` -> НОВЫЙ буфер; получатель не меняется.
    BufConcat,
    /// Побитовые операции с БУФЕРОМ-маской, накладываемой с позиции.
    WriteBitwiseAnd,
    WriteBitwiseOr,
    WriteBitwiseXor,
    WriteBitwiseAndNot,
    /// `Инвертировать([Позиция][, Количество])` — побитовое НЕ по месту.
    Invert,
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
    // Английское написание проверено пробой `BIN.SIZE.EN`: платформа
    // принимает `ДД.Size()`.
    ("Размер", BuiltinMethod::Size),
    ("Size", BuiltinMethod::Size),
    // Английские написания методов буфера ИЗМЕРЕНЫ, а не достроены по
    // образцу: платформа принимает `Set`, `ReadInt16`, `WriteInt16`,
    // `Split`, `Concat`, `WriteBitwiseAnd`, `WriteBitwiseXor`,
    // `WriteBitwiseAndNot`, `Invert`. Русское имя исключающего ИЛИ —
    // `ЗаписатьПобитовоеИсключительноеИли`; `...ИсключающееИли` платформа
    // НЕ знает, и это стоило отдельного захода.
    ("Установить", BuiltinMethod::BufSet),
    ("Set", BuiltinMethod::BufSet),
    ("ПрочитатьЦелое16", BuiltinMethod::ReadInt16),
    ("ReadInt16", BuiltinMethod::ReadInt16),
    ("ПрочитатьЦелое32", BuiltinMethod::ReadInt32),
    ("ReadInt32", BuiltinMethod::ReadInt32),
    ("ПрочитатьЦелое64", BuiltinMethod::ReadInt64),
    ("ReadInt64", BuiltinMethod::ReadInt64),
    ("ЗаписатьЦелое16", BuiltinMethod::WriteInt16),
    ("WriteInt16", BuiltinMethod::WriteInt16),
    ("ЗаписатьЦелое32", BuiltinMethod::WriteInt32),
    ("WriteInt32", BuiltinMethod::WriteInt32),
    ("ЗаписатьЦелое64", BuiltinMethod::WriteInt64),
    ("WriteInt64", BuiltinMethod::WriteInt64),
    ("Разделить", BuiltinMethod::BufSplit),
    ("Split", BuiltinMethod::BufSplit),
    ("Соединить", BuiltinMethod::BufConcat),
    ("Concat", BuiltinMethod::BufConcat),
    ("ЗаписатьПобитовоеИ", BuiltinMethod::WriteBitwiseAnd),
    ("WriteBitwiseAnd", BuiltinMethod::WriteBitwiseAnd),
    ("ЗаписатьПобитовоеИли", BuiltinMethod::WriteBitwiseOr),
    ("WriteBitwiseOr", BuiltinMethod::WriteBitwiseOr),
    (
        "ЗаписатьПобитовоеИсключительноеИли",
        BuiltinMethod::WriteBitwiseXor,
    ),
    ("WriteBitwiseXor", BuiltinMethod::WriteBitwiseXor),
    ("ЗаписатьПобитовоеИНе", BuiltinMethod::WriteBitwiseAndNot),
    ("WriteBitwiseAndNot", BuiltinMethod::WriteBitwiseAndNot),
    ("Инвертировать", BuiltinMethod::Invert),
    ("Invert", BuiltinMethod::Invert),
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
        BuiltinFn::CommandLineArguments => Ok(BslValue::new_array(
            command_line_args()
                .iter()
                .map(|a| BslValue::Str(BslString::from_str(a)))
                .collect(),
        )),
        BuiltinFn::DatePartOf(part) => args[0].date_component(part),
        BuiltinFn::DateBoundaryOf(which) => args[0].date_boundary(which),
        BuiltinFn::AddMonth => args[0].add_month(&args[1]),
        // Перехвачена в `call_builtin_fn_ctx` — без таблицы имён набор
        // полей приёмника не прочитать. Ошибка, а не `unreachable!`: эта
        // функция публична, и ронять процесс на прямом вызове из
        // встраивающего приложения незачем (то же соображение, что и у
        // `RtError::InvalidBytecode`).
        BuiltinFn::ReadJson
        | BuiltinFn::WriteJson
        | BuiltinFn::WriteJsonValue
        | BuiltinFn::ReadJsonValue => Err(RtError::InvalidBytecode(
            "функции JSON требуют контекста имён: вызывайте call_builtin_fn_ctx",
        )),
        BuiltinFn::SplitBinaryData => args[0].binary_data_split(&args[1]),
        BuiltinFn::ConcatBinaryData => args[0].binary_data_combine(),
        BuiltinFn::WriteJsonDate => crate::json::write_json_date(&args[0], &args[1], &args[2]),
        BuiltinFn::ReadJsonDate => crate::json::read_json_date(&args[0], &args[1]),
        BuiltinFn::ValueToStringInternal
        | BuiltinFn::ValueFromStringInternal
        | BuiltinFn::ValueToFile
        | BuiltinFn::ValueFromFile => Err(RtError::InvalidBytecode(
            "функции внутреннего формата требуют контекста имён: вызывайте call_builtin_fn_ctx",
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
/// Имя пользовательской функции и её модуль — общая для `ПрочитатьJSON` и
/// `ЗаписатьJSON` пара позиций.
///
/// ИЗМЕРЕНО на 8.3.27 (модуль формы, функции с `Экспорт`), что колбэк
/// работает ТОЛЬКО когда заданы обе позиции:
///
/// * `МодульФункции...` = `Неопределено` (или аргумент опущен) — платформа
///   функцию не ищет ВООБЩЕ: `ЗаписатьJSON` на несериализуемом значении
///   падает тем же «Значение содержит данные недопустимых типов», что и без
///   имени функции, а `ПрочитатьJSON` молча читает документ, ни разу её не
///   позвав;
/// * имя — пустая строка: то же самое, функция не зовётся;
/// * имя не строкой (число) — «Несоответствие типов (параметр номер '4')»,
///   то есть ошибка типа на входе.
///
/// Модуль здесь — ЛЮБОЕ значение, кроме `Неопределено`: своей системы
/// модулей у интерпретатора нет, функция всегда ищется в модуле
/// исполняемого скрипта. Платформа в этой позиции разбирается: не-объект
/// даёт «Несоответствие типов (параметр номер '5')», а объект без такого
/// метода — «Метод 'X' не найден». Воспроизводить это различие нечем —
/// модуль у нас ровно один, — поэтому проверяется только `Неопределено`,
/// от которого зависит, звать ли функцию вообще.
///
/// # Errors
///
/// [`RtError::TypeError`], если имя задано и не строка.
fn callback_name(
    name_arg: Option<&BslValue>,
    module_arg: Option<&BslValue>,
    op: &'static str,
) -> RtResult<Option<String>> {
    let name = match name_arg {
        None | Some(BslValue::Undefined) => return Ok(None),
        Some(BslValue::Str(s)) => s.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            })
        }
    };
    if name.is_empty() || matches!(module_arg, None | Some(BslValue::Undefined)) {
        return Ok(None);
    }
    Ok(Some(name))
}

/// Массив строк из аргумента-списка имён. Отсутствующий аргумент —
/// пустой список.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент задан и не коллекция: ИЗМЕРЕНО,
/// что платформа отвечает на строку в позиции
/// `ИменаСвойствДляФункцииВосстановления` «Несоответствие типов (параметр
/// номер '8')».
fn name_list_arg(
    arg: Option<&BslValue>,
    rt: &RuntimeShapes,
    op: &'static str,
) -> RtResult<Vec<String>> {
    let Some(list) = arg else {
        return Ok(Vec::new());
    };
    if matches!(list, BslValue::Undefined) {
        return Ok(Vec::new());
    }
    let len = list.collection_len().map_err(|_| RtError::TypeError {
        expected: "Массив",
        op,
    })?;
    let mut names = Vec::with_capacity(len);
    for i in 0..len {
        let item = list.get_index(
            &BslValue::Number(bsl_number::BslNumber::from_i64(i as i64)),
            &rt.names,
        )?;
        if let BslValue::Str(s) = item {
            names.push(s.to_string());
        }
    }
    Ok(names)
}

/// `ПрочитатьJSON` целиком: разбор аргументов плюс, если функция
/// восстановления задана и вызывать её есть чем, её подключение.
///
/// `call` — канал вызова функции модуля по имени (его даёт VM, см.
/// [`crate::json::JsonCallByName`]). `None` — контекста исполнения нет
/// (прямой вызов из встраивающего приложения), и тогда имя функции
/// восстановления остаётся понятной ошибкой, а не тихо игнорируется.
///
/// # Errors
///
/// См. `crate::json::read_json`; [`RtError::Json`], если функция
/// восстановления задана, а исполняющей VM нет.
pub fn read_json_builtin(
    args: &[BslValue],
    rt: &mut RuntimeShapes,
    call: Option<crate::json::JsonCallByName<'_>>,
) -> RtResult<BslValue> {
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
    let date_names = name_list_arg(
        args.get(2),
        rt,
        "ПрочитатьJSON(ИменаСвойствСоЗначениямиДата)",
    )?;
    // Четвёртая позиция платформы — `ОжидаемыйФорматДаты`
    // (`ФорматДатыJSON`), а НЕ колбэк: см. `optional_date_format_from_arg`.
    let date_format = crate::json::optional_date_format_from_arg(
        args.get(3),
        "ПрочитатьJSON(ОжидаемыйФорматДаты)",
    )?;
    // Позиции 5..8 — имя функции восстановления, её модуль, дополнительные
    // параметры и `ИменаСвойствДляФункцииВосстановления`.
    let name = callback_name(
        args.get(4),
        args.get(5),
        "ПрочитатьJSON(ИмяФункцииВосстановления)",
    )?;
    let restore = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(crate::json::JsonRestoreFn {
            name,
            extra: args.get(6).cloned().unwrap_or(BslValue::Undefined),
            property_names: name_list_arg(
                args.get(7),
                rt,
                "ПрочитатьJSON(ИменаСвойствДляФункцииВосстановления)",
            )?,
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ПрочитатьJSON: функция восстановления требует исполняющей VM \
                 (вызывайте через bsl-vm, а не call_builtin_fn_ctx)"
                    .to_string(),
            ))
        }
    };
    crate::json::read_json(&args[0], as_map, &date_names, date_format, restore, rt)
}

/// `ЗаписатьJSON` целиком — то же самое для функции преобразования.
///
/// # Errors
///
/// См. `crate::json::write_json`; [`RtError::Json`], если функция
/// преобразования задана, а исполняющей VM нет.
pub fn write_json_builtin(
    args: &[BslValue],
    rt: &mut RuntimeShapes,
    call: Option<crate::json::JsonCallByName<'_>>,
) -> RtResult<BslValue> {
    let settings = crate::json::serializer_settings_from(args.get(2))?;
    let name = callback_name(
        args.get(3),
        args.get(4),
        "ЗаписатьJSON(ИмяФункцииПреобразования)",
    )?;
    let convert = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(crate::json::JsonConvertFn {
            name,
            extra: args.get(5).cloned().unwrap_or(BslValue::Undefined),
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ЗаписатьJSON: функция преобразования требует исполняющей VM \
                 (вызывайте через bsl-vm, а не call_builtin_fn_ctx)"
                    .to_string(),
            ))
        }
    };
    crate::json::write_json(&args[0], &args[1], &settings, convert, rt)?;
    Ok(BslValue::Undefined)
}

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
        // Разбор аргументов — общий с путём из VM, см. `read_json_builtin`.
        // Здесь исполняющей VM нет, поэтому колбэка нет тоже.
        return read_json_builtin(args, rt, None);
    }
    if f == BuiltinFn::WriteJson {
        return write_json_builtin(args, rt, None);
    }
    if f == BuiltinFn::WriteJsonValue {
        return crate::json::write_json_value(&args[0], rt);
    }
    if f == BuiltinFn::ReadJsonValue {
        return crate::json::read_json_value(&args[0], rt);
    }
    if f == BuiltinFn::ValueToStringInternal {
        let text = crate::vstr::value_to_string_internal(&args[0], rt)?;
        return Ok(BslValue::Str(BslString::from_str(&text)));
    }
    if f == BuiltinFn::ValueFromStringInternal {
        let BslValue::Str(text) = &args[0] else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗначениеИзСтрокиВнутр",
            });
        };
        return crate::vstr::value_from_string_internal(&text.to_string(), rt);
    }
    if f == BuiltinFn::ValueToFile {
        let BslValue::Str(path) = &args[0] else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗначениеВФайл(ИмяФайла)",
            });
        };
        crate::vstr::value_to_file(&path.to_string(), &args[1], rt)?;
        return Ok(BslValue::Undefined);
    }
    if f == BuiltinFn::ValueFromFile {
        let BslValue::Str(path) = &args[0] else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "ЗначениеИзФайла(ИмяФайла)",
            });
        };
        return crate::vstr::value_from_file(&path.to_string(), rt);
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
            _ if crate::spreadsheet::is_drawings(obj) => {
                crate::spreadsheet::drawings_add(obj, args)
            }
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
        // `Получить` полиморфен: у `Соответствие` это чтение по ключу, у
        // буфера — байт по позиции (измерено, что `Буфер.Получить(0)`
        // делает ровно то же, что `Буфер[0]`).
        BuiltinMethod::Get => match obj {
            BslValue::Object(o) if matches!(&**o, BslObject::Map(_)) => obj.map_get(&args[0]),
            _ if crate::binbuf::is_buffer(obj) => crate::binbuf::get_byte(obj, &args[0]),
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
        BuiltinMethod::Size => obj.binary_data_size(),

        // --- БуферДвоичныхДанных ------------------------------------------
        BuiltinMethod::BufSet => match args {
            [pos, value] => crate::binbuf::set_byte(obj, pos, value).map(|()| BslValue::Undefined),
            _ => Err(RtError::MethodNotApplicable {
                method: "Установить",
                receiver: obj.type_name(),
            }),
        },
        BuiltinMethod::ReadInt16 => {
            crate::binbuf::read_int(obj, args, crate::binbuf::IntWidth::W16)
        }
        BuiltinMethod::ReadInt32 => {
            crate::binbuf::read_int(obj, args, crate::binbuf::IntWidth::W32)
        }
        BuiltinMethod::ReadInt64 => {
            crate::binbuf::read_int(obj, args, crate::binbuf::IntWidth::W64)
        }
        BuiltinMethod::WriteInt16 => {
            crate::binbuf::write_int(obj, args, crate::binbuf::IntWidth::W16)
        }
        BuiltinMethod::WriteInt32 => {
            crate::binbuf::write_int(obj, args, crate::binbuf::IntWidth::W32)
        }
        BuiltinMethod::WriteInt64 => {
            crate::binbuf::write_int(obj, args, crate::binbuf::IntWidth::W64)
        }
        BuiltinMethod::BufSplit => crate::binbuf::split(obj, &args[0]),
        BuiltinMethod::BufConcat => crate::binbuf::concat(obj, &args[0]),
        BuiltinMethod::WriteBitwiseAnd => {
            crate::binbuf::bitwise(obj, args, crate::binbuf::BitOp::And)
        }
        BuiltinMethod::WriteBitwiseOr => {
            crate::binbuf::bitwise(obj, args, crate::binbuf::BitOp::Or)
        }
        BuiltinMethod::WriteBitwiseXor => {
            crate::binbuf::bitwise(obj, args, crate::binbuf::BitOp::Xor)
        }
        BuiltinMethod::WriteBitwiseAndNot => {
            crate::binbuf::bitwise(obj, args, crate::binbuf::BitOp::AndNot)
        }
        BuiltinMethod::Invert => crate::binbuf::invert(obj, args),
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
        BslValue::Str(s) => Ok(rt.names.intern_bsl(s)),
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

/// Колбэки `ПрочитатьJSON`/`ЗаписатьJSON`: разбор аргументов и вся
/// семантика вызова — на mock-замыкании, без исполняющей VM. Каждый тест
/// назван по пробе, которой поведение снято с 8.3.27 (см. PROGRESS.md).
#[cfg(test)]
mod json_callback_tests {
    use super::*;

    fn s(text: &str) -> BslValue {
        BslValue::Str(BslString::from_str(text))
    }

    fn n(v: i64) -> BslValue {
        BslValue::Number(bsl_number::BslNumber::from_i64(v))
    }

    fn reader_of(text: &str) -> BslValue {
        let reader = BslValue::new_json_reader();
        crate::json::set_string(&reader, &[s(text)]).unwrap();
        reader
    }

    fn writer() -> BslValue {
        let writer = BslValue::new_json_writer();
        crate::json::set_string(&writer, &[]).unwrap();
        writer
    }

    fn written(w: &BslValue) -> String {
        match crate::json::close_writer(w).unwrap() {
            BslValue::Str(text) => text.to_string(),
            other => panic!("Закрыть() вернул не строку: {other:?}"),
        }
    }

    /// Журнал вызовов mock-функции: имя и аргументы каждого вызова.
    #[derive(Default)]
    struct CallLog(Vec<(String, Vec<BslValue>)>);

    impl CallLog {
        /// Имена первого параметра (`Свойство`) в порядке вызовов:
        /// `None` — пришло `Неопределено`.
        fn properties(&self) -> Vec<Option<String>> {
            self.0
                .iter()
                .map(|(_, args)| match &args[0] {
                    BslValue::Str(p) => Some(p.to_string()),
                    _ => None,
                })
                .collect()
        }
    }

    /// Аргументы `ПрочитатьJSON` до колбэка включительно.
    fn read_args(reader: BslValue, name: &str, module: BslValue) -> [BslValue; 8] {
        [
            reader,
            BslValue::Undefined,
            BslValue::Undefined,
            BslValue::Undefined,
            s(name),
            module,
            s("ДОП"),
            BslValue::Undefined,
        ]
    }

    /// Аргументы `ЗаписатьJSON` до колбэка включительно.
    fn write_args(w: BslValue, value: BslValue, name: &str, module: BslValue) -> [BslValue; 6] {
        [w, value, BslValue::Undefined, s(name), module, s("ДОП")]
    }

    /// Несериализуемое значение — то же, на котором снята вся серия проб.
    fn unserializable() -> BslValue {
        BslValue::new_table()
    }

    /// Поле собранной структуры по имени — через таблицу имён, в которую
    /// разбор его и интернировал.
    fn field(v: &BslValue, rt: &RuntimeShapes, name: &str) -> BslValue {
        let id = rt.names.lookup(name).expect("имя интернировано разбором");
        v.get_field(id).expect("поле должно быть на месте")
    }

    /// ИЗМЕРЕНО (проба F1/F3): без `МодульФункции...` платформа функцию не
    /// ищет ВООБЩЕ — чтение молча проходит мимо неё, а запись падает тем же
    /// «недопустимые типы», что и без имени. Раньше здесь была ошибка
    /// «появятся позже».
    #[test]
    fn json_callback_is_not_used_without_a_module() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut called = false;
        {
            let mut call = |_: &str, _: Vec<BslValue>| {
                called = true;
                Ok((BslValue::Undefined, Vec::new()))
            };
            let args = read_args(reader_of("1"), "ИмяФункции", BslValue::Undefined);
            let v = read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
            assert_eq!(v, n(1));
        }
        assert!(!called, "функция восстановления звалась без модуля");

        let mut called = false;
        {
            let mut call = |_: &str, _: Vec<BslValue>| {
                called = true;
                Ok((BslValue::Undefined, Vec::new()))
            };
            let args = write_args(
                writer(),
                unserializable(),
                "ИмяФункции",
                BslValue::Undefined,
            );
            let e = write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err();
            assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
        }
        assert!(!called, "функция преобразования звалась без модуля");
    }

    /// ИЗМЕРЕНО (проба I6/K6): пустое имя равносильно отсутствию функции.
    #[test]
    fn json_callback_is_not_used_with_an_empty_name() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut called = false;
        let mut call = |_: &str, _: Vec<BslValue>| {
            called = true;
            Ok((BslValue::Undefined, Vec::new()))
        };
        let args = read_args(reader_of("1"), "", BslValue::Boolean(true));
        read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        assert!(!called);
    }

    /// ИЗМЕРЕНО (проба T1): имя не строкой — ошибка типа на входе.
    #[test]
    fn json_callback_name_must_be_a_string() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut call = |_: &str, _: Vec<BslValue>| Ok((BslValue::Undefined, Vec::new()));
        let mut args = write_args(writer(), n(1), "х", BslValue::Boolean(true));
        args[3] = n(42);
        let e = write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
    }

    /// Без исполняющей VM (прямой вызов из встраивающего приложения)
    /// заданный колбэк — понятная ошибка, а не тихий пропуск.
    #[test]
    fn json_callback_without_a_vm_is_a_clear_error() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let args = read_args(reader_of("1"), "ИмяФункции", BslValue::Boolean(true));
        let e = read_json_builtin(&args, &mut rt, None).unwrap_err();
        assert!(matches!(e, RtError::Json(_)), "{e:?}");

        let args = write_args(writer(), n(1), "ИмяФункции", BslValue::Boolean(true));
        let e = write_json_builtin(&args, &mut rt, None).unwrap_err();
        assert!(matches!(e, RtError::Json(_)), "{e:?}");
    }

    /// ИЗМЕРЕНО (пробы H4/H5): функция преобразования ЛЕНИВА — на значении,
    /// которое платформа сериализует сама (число, дата), её не зовут ни разу.
    #[test]
    fn json_convert_function_is_lazy() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut log = CallLog::default();
        let w = writer();
        {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args));
                Ok((s("<преобразовано>"), vec![BslValue::Boolean(false); 4]))
            };
            let args = write_args(w.clone(), n(1), "Преобразовать", BslValue::Boolean(true));
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert_eq!(written(&w), "1");
        assert!(log.0.is_empty(), "функция звалась зря: {:?}", log.0);
    }

    /// ИЗМЕРЕНО (пробы H1/H2/H3/H7): `Свойство` — имя свойства для члена
    /// структуры и соответствия, `Неопределено` для элемента массива и для
    /// верхнего уровня; остальные три параметра — значение, дополнительные
    /// параметры и `Отказ = Ложь`.
    #[test]
    fn json_convert_function_gets_the_measured_arguments() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut log = CallLog::default();
        let w = writer();
        {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args));
                Ok((s("<з>"), vec![BslValue::Boolean(false); 4]))
            };
            let value = BslValue::new_array(vec![unserializable()]);
            let args = write_args(w.clone(), value, "Преобразовать", BslValue::Boolean(true));
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert_eq!(written(&w), "[\n\"<з>\"\n]");
        assert_eq!(log.0.len(), 1);
        let (name, args) = &log.0[0];
        assert_eq!(name, "Преобразовать");
        assert_eq!(args.len(), 4, "ровно четыре параметра");
        assert_eq!(args[0], BslValue::Undefined, "элемент массива — без имени");
        assert_eq!(args[2], s("ДОП"));
        assert_eq!(args[3], BslValue::Boolean(false), "Отказ приходит Ложь");
    }

    /// ИЗМЕРЕНО (проба I1/M2/M3): `Отказ` убирает значение из документа
    /// целиком — свойство исчезает, элемент массива исчезает, а на верхнем
    /// уровне не пишется вообще ничего.
    #[test]
    fn json_convert_refusal_drops_the_value() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let refuse = |_: &str, _: Vec<BslValue>| {
            Ok((
                s("<в документ попасть не должно>"),
                vec![
                    BslValue::Undefined,
                    BslValue::Undefined,
                    BslValue::Undefined,
                    BslValue::Boolean(true),
                ],
            ))
        };

        let w = writer();
        {
            let mut call = refuse;
            let value = BslValue::new_array(vec![n(1), unserializable(), n(3)]);
            let args = write_args(w.clone(), value, "Отказная", BslValue::Boolean(true));
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert_eq!(written(&w), "[\n1,\n3\n]");

        let w = writer();
        {
            let mut call = refuse;
            let args = write_args(
                w.clone(),
                unserializable(),
                "Отказная",
                BslValue::Boolean(true),
            );
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert_eq!(written(&w), "", "на верхнем уровне не пишется ничего");
    }

    /// ИЗМЕРЕНО (пробы U/S/V, четырнадцать значений): `Отказ` читается по
    /// обычным правилам условия языка, а неприводимое значение отказом не
    /// считается.
    #[test]
    fn json_convert_refusal_follows_the_condition_rules() {
        let refuses = [BslValue::Boolean(true), n(1), n(-1), s("да")];
        let keeps = [
            BslValue::Boolean(false),
            n(0),
            s(""),
            s("   "),
            s("абв"),
            BslValue::Undefined,
            BslValue::Null,
            BslValue::new_array(Vec::new()),
        ];
        for value in refuses {
            let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
            let w = writer();
            {
                let mut call = |_: &str, _: Vec<BslValue>| {
                    Ok((
                        s("<з>"),
                        vec![
                            BslValue::Undefined,
                            BslValue::Undefined,
                            BslValue::Undefined,
                            value.clone(),
                        ],
                    ))
                };
                let args = write_args(
                    w.clone(),
                    unserializable(),
                    "Отказная",
                    BslValue::Boolean(true),
                );
                write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
            }
            assert_eq!(written(&w), "", "отказом обязано быть: {value:?}");
        }
        for value in keeps {
            let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
            let w = writer();
            {
                let mut call = |_: &str, _: Vec<BslValue>| {
                    Ok((
                        s("<з>"),
                        vec![
                            BslValue::Undefined,
                            BslValue::Undefined,
                            BslValue::Undefined,
                            value.clone(),
                        ],
                    ))
                };
                let args = write_args(
                    w.clone(),
                    unserializable(),
                    "Отказная",
                    BslValue::Boolean(true),
                );
                write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
            }
            assert_eq!(written(&w), "\"<з>\"", "отказом быть НЕ должно: {value:?}");
        }
    }

    /// ИЗМЕРЕНО (проба I2): вернувшееся снова несериализуемое значение
    /// повторного вызова НЕ вызывает — обычная ошибка типа после одного
    /// вызова.
    #[test]
    fn json_convert_result_is_not_converted_again() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut calls = 0;
        let w = writer();
        let e = {
            let mut call = |_: &str, _: Vec<BslValue>| {
                calls += 1;
                Ok((unserializable(), vec![BslValue::Boolean(false); 4]))
            };
            let args = write_args(
                w.clone(),
                unserializable(),
                "Преобразовать",
                BslValue::Boolean(true),
            );
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err()
        };
        assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
        assert_eq!(calls, 1, "вызов обязан быть ровно один");
    }

    /// ИЗМЕРЕНО (проба I3): а вот КОНТЕЙНЕР, вернувшийся из функции,
    /// обходится как обычно — на несериализуемом внутри него функция
    /// зовётся снова.
    #[test]
    fn json_convert_result_container_is_walked_normally() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut calls = 0;
        let w = writer();
        {
            let mut call = |_: &str, _: Vec<BslValue>| {
                calls += 1;
                let returned = if calls > 2 {
                    s("<хватит>")
                } else {
                    BslValue::new_array(vec![unserializable()])
                };
                Ok((returned, vec![BslValue::Boolean(false); 4]))
            };
            let args = write_args(
                w.clone(),
                unserializable(),
                "Преобразовать",
                BslValue::Boolean(true),
            );
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert_eq!(calls, 3);
        assert_eq!(written(&w), "[\n[\n\"<хватит>\"\n]\n]");
    }

    /// Ошибка изнутри функции преобразования не глотается — ИЗМЕРЕНО
    /// (проба L1), платформа выпускает её наружу из `ЗаписатьJSON`.
    #[test]
    fn json_convert_error_is_not_swallowed() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let w = writer();
        let e = {
            let mut call =
                |_: &str, _: Vec<BslValue>| Err(RtError::Raised(s("изнутри преобразования")));
            let args = write_args(
                w.clone(),
                unserializable(),
                "Преобразовать",
                BslValue::Boolean(true),
            );
            write_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err()
        };
        assert!(matches!(e, RtError::Raised(_)), "{e:?}");
    }

    /// ИЗМЕРЕНО (проба J1): функция восстановления зовётся для КАЖДОГО
    /// значения документа в ОБРАТНОМ порядке (дети раньше родителя), с
    /// именем свойства или `Неопределено` для элемента массива и корня.
    /// Порядок снят на этом же документе.
    #[test]
    fn json_restore_function_visits_every_value_bottom_up() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut log = CallLog::default();
        let doc = r#"{"чис":1,"стр":"т","лог":true,"нул":null,"#.to_string()
            + r#""об":{"вчис":2,"вмас":[7,8]},"#
            + r#""мас":[3,"ф",false,null,{"мчис":4},[5,6]]}"#;
        {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args.clone()));
                Ok((args[1].clone(), Vec::new()))
            };
            let args = read_args(reader_of(&doc), "Восстановить", BslValue::Boolean(true));
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        let expected: Vec<Option<String>> = [
            Some("чис"),
            Some("стр"),
            Some("лог"),
            Some("нул"),
            Some("вчис"),
            None,
            None,
            Some("вмас"),
            Some("об"),
            None,
            None,
            None,
            None,
            Some("мчис"),
            None,
            None,
            None,
            None,
            Some("мас"),
            None,
        ]
        .iter()
        .map(|p| p.map(str::to_string))
        .collect();
        assert_eq!(log.properties(), expected);
        assert!(
            log.0.iter().all(|(_, a)| a.len() == 3 && a[2] == s("ДОП")),
            "ровно три параметра, третий — дополнительные"
        );
    }

    /// ИЗМЕРЕНО (пробы J3/O3): скаляр на верхнем уровне тоже получает
    /// вызов, с `Свойство = Неопределено`, и результат заменяет значение.
    #[test]
    fn json_restore_function_replaces_a_top_level_scalar() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut log = CallLog::default();
        let v = {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args));
                Ok((s("<заменено>"), Vec::new()))
            };
            let args = read_args(reader_of("42"), "Восстановить", BslValue::Boolean(true));
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap()
        };
        assert_eq!(v, s("<заменено>"));
        assert_eq!(log.properties(), vec![None]);
    }

    /// ИЗМЕРЕНО (пробы K1/K2/Q1): непустой список имён сужает вызовы до
    /// перечисленных свойств НА ЛЮБОЙ ГЛУБИНЕ и отменяет вызов на корне;
    /// пустой список — то же, что его отсутствие; сравнение
    /// РЕГИСТРОЗАВИСИМОЕ.
    #[test]
    fn json_restore_property_filter_is_case_sensitive_and_skips_the_root() {
        let doc = r#"{"а":1,"б":2,"в":{"б":3,"г":4}}"#;
        for (filter, expected) in [
            (
                vec![s("б")],
                vec![Some("б".to_string()), Some("б".to_string())],
            ),
            (vec![s("Б")], Vec::new()),
            (
                Vec::new(),
                vec![
                    Some("а".to_string()),
                    Some("б".to_string()),
                    Some("б".to_string()),
                    Some("г".to_string()),
                    Some("в".to_string()),
                    None,
                ],
            ),
        ] {
            let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
            let mut log = CallLog::default();
            {
                let mut call = |name: &str, args: Vec<BslValue>| {
                    log.0.push((name.to_string(), args.clone()));
                    Ok((args[1].clone(), Vec::new()))
                };
                let mut args = read_args(reader_of(doc), "Восстановить", BslValue::Boolean(true));
                args[7] = BslValue::new_array(filter.clone());
                read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
            }
            assert_eq!(log.properties(), expected, "фильтр {filter:?}");
        }
    }

    /// ИЗМЕРЕНО (пробы P1/P2/P3): функция восстановления имеет приоритет
    /// над `ИменаСвойствСоЗначениямиДата` — свойство, которое ей достаётся,
    /// приходит СЫРОЙ строкой и датой не становится; свойство, до неё не
    /// дошедшее, разбирается в дату как раньше.
    #[test]
    fn json_restore_function_wins_over_the_date_property_list() {
        let doc = r#"{"создано":"2014-05-10T13:14:15","прочее":1}"#;
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());

        // Без функции восстановления — дата.
        let mut args = read_args(reader_of(doc), "", BslValue::Undefined);
        args[2] = BslValue::new_array(vec![s("создано")]);
        let v = read_json_builtin(&args, &mut rt, None).unwrap();
        assert!(
            matches!(field(&v, &rt, "создано"), BslValue::Date(_)),
            "без функции обязана быть Дата"
        );

        // Функция сужена до «прочее» — «создано» по-прежнему дата.
        let mut log = CallLog::default();
        let v = {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args.clone()));
                Ok((args[1].clone(), Vec::new()))
            };
            let mut args = read_args(reader_of(doc), "Восстановить", BslValue::Boolean(true));
            args[2] = BslValue::new_array(vec![s("создано")]);
            args[7] = BslValue::new_array(vec![s("прочее")]);
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap()
        };
        assert!(matches!(field(&v, &rt, "создано"), BslValue::Date(_)));
        assert_eq!(log.properties(), vec![Some("прочее".to_string())]);

        // Функция достаётся «создано» — приходит и остаётся строкой.
        let mut log = CallLog::default();
        let v = {
            let mut call = |name: &str, args: Vec<BslValue>| {
                log.0.push((name.to_string(), args.clone()));
                Ok((args[1].clone(), Vec::new()))
            };
            let mut args = read_args(reader_of(doc), "Восстановить", BslValue::Boolean(true));
            args[2] = BslValue::new_array(vec![s("создано")]);
            args[7] = BslValue::new_array(vec![s("создано")]);
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap()
        };
        assert_eq!(field(&v, &rt, "создано"), s("2014-05-10T13:14:15"));
        assert_eq!(
            log.0[0].1[1],
            s("2014-05-10T13:14:15"),
            "в функцию — сырая строка"
        );
    }

    /// Ошибка изнутри функции восстановления не глотается (проба L2).
    #[test]
    fn json_restore_error_is_not_swallowed() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let e = {
            let mut call =
                |_: &str, _: Vec<BslValue>| Err(RtError::Raised(s("изнутри восстановления")));
            let args = read_args(
                reader_of("{\"а\":1}"),
                "Восстановить",
                BslValue::Boolean(true),
            );
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err()
        };
        assert!(matches!(e, RtError::Raised(_)), "{e:?}");
    }

    /// ИЗМЕРЕНО (проба T2): список имён не массивом — ошибка типа.
    #[test]
    fn json_restore_property_filter_must_be_an_array() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let mut call = |_: &str, _: Vec<BslValue>| Ok((BslValue::Undefined, Vec::new()));
        let mut args = read_args(
            reader_of("{\"а\":1}"),
            "Восстановить",
            BslValue::Boolean(true),
        );
        args[7] = s("нестрока");
        let e = read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
    }

    /// Повторный вход в тот же `ЧтениеJSON`/`ЗаписьJSON` изнутри колбэка —
    /// перехватываемая ошибка, а НЕ паника `RefCell`. Платформа тоже
    /// отвечает ошибкой («Недопустимое состояние потока чтения JSON»,
    /// «Неверный порядок записи JSON»); текст у нас свой.
    #[test]
    fn reentering_the_same_json_object_from_a_callback_is_an_error_not_a_panic() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let reader = reader_of(r#"{"а":1}"#);
        let inner = reader.clone();
        let mut nested: Option<RtError> = None;
        {
            let mut call = |_: &str, args: Vec<BslValue>| {
                let mut inner_rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
                let probe = [inner.clone()];
                if let Err(e) = read_json_builtin(&probe, &mut inner_rt, None) {
                    nested = Some(e);
                }
                Ok((args[1].clone(), Vec::new()))
            };
            let args = read_args(reader.clone(), "Восстановить", BslValue::Boolean(true));
            read_json_builtin(&args, &mut rt, Some(&mut call)).unwrap();
        }
        assert!(
            matches!(nested, Some(RtError::TypeError { .. })),
            "{nested:?}"
        );
    }
}
