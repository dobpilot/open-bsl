use crate::date::{DateBoundary, DatePart};
use crate::env::HostEnv;
use crate::runtime_shapes::RuntimeShapes;
use crate::{BslObject, BslString, BslValue, NameId, RtError, RtResult};

/// Возможность ПРОГОНА, за которой встроенная функция ходит помимо своих
/// аргументов (см. [`BuiltinFn::host_effect`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEffect {
    /// Пишет в поток вывода прогона: `Сообщить`.
    Output,
    /// Отвечает из часов, случайности или аргументов запуска.
    Env,
    /// Читает или пишет через файловую систему прогона.
    Files,
    /// Выбирает временный путь внутри файловой системы из случайности прогона.
    TempFiles,
}

/// Встроенные функции, ответ которых берётся не из аргументов, а из
/// ОКРУЖЕНИЯ прогона: часы, часы в миллисекундах и аргументы запуска.
/// Какие именно — говорит [`BuiltinFn::host_effect`], один источник истины
/// на весь рабочий процесс.
///
/// Отдельный узкий вход, а не четвёртый параметр у `call_builtin_fn_ctx`:
/// окружение есть не у всякого вызывающего. У шима JIT его нет и быть не
/// должно — он работает с sink-потоками и не видит `State`.
///
/// # Errors
///
/// [`RtError::InvalidBytecode`], если сюда пришла не функция окружения.
pub fn call_builtin_env(f: BuiltinFn, env: &mut HostEnv) -> RtResult<BslValue> {
    match f {
        BuiltinFn::CurrentDate => BslValue::current_date(env),
        BuiltinFn::CurrentUniversalDate => BslValue::current_universal_date(env),
        BuiltinFn::CurrentUniversalDateInMilliseconds => {
            BslValue::current_universal_date_in_milliseconds(env)
        }
        BuiltinFn::CommandLineArguments => Ok(BslValue::new_array(
            env.arguments()
                .iter()
                .map(|a| BslValue::Str(BslString::from_str(a)))
                .collect(),
        )),
        _ => Err(RtError::InvalidBytecode(
            "эта встроенная функция не относится к окружению прогона",
        )),
    }
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
    /// Наименьшее из одного или нескольких сравнимых значений.
    Min,
    /// Наибольшее из одного или нескольких сравнимых значений.
    Max,
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
    /// Регистрозависимая проверка непустого префикса.
    StrStartsWith,
    /// Регистрозависимая проверка непустого суффикса.
    StrEndsWith,
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
    /// `НСтр`/`NStr(РесурснаяСтрока[, КодЯзыка])` выбирает одно значение
    /// из записи `ru = '...'; en = '...'`.
    LocalizedString,
    /// `ПустаяСтрока`/`IsBlankString(Значение)` проверяет отсутствие
    /// непробельных символов; `Неопределено` также считается пустым.
    IsBlankString,
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
    /// Снимок текущего исключения BSL-задачи. Значение строит VM, потому
    /// что базовый runtime не владеет стеком исполнения.
    ErrorInfo,
    /// Безопасный текст из неизменяемого снимка `ИнформацияОбОшибке`.
    DetailedErrorDescription,

    /// `Дата(Год, Месяц, День[, Час, Минута, Секунда])` либо
    /// `Дата("ГГГГММДДЧЧММСС")` — одна встроенная функция с перегрузкой по
    /// типу первого аргумента, как и в самой 1С (см. `BslValue::make_date`).
    MakeDate,
    /// `ТекущаяДата`/`CurrentDate` — см. `BslValue::current_date` про то,
    /// почему момент берётся по UTC, а не по локальной зоне.
    CurrentDate,
    /// `ТекущаяУниверсальнаяДата`/`CurrentUniversalDate` — текущий момент
    /// UTC как значение `Дата`.
    CurrentUniversalDate,
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
    /// `Base64Значение(Строка)` — двоичные данные из Base64.
    Base64Value,
    /// Стандартный Base64 без переносов строк.
    GetBase64StringFromBinaryData,
    /// Обратное преобразование Base64 в `ДвоичныеДанные`.
    GetBinaryDataFromBase64String,
    /// Заглавное шестнадцатеричное представление байтов.
    GetHexStringFromBinaryData,
    /// Percent-кодирование UTF-8 строки в одном из двух режимов платформы.
    EncodeString,
    /// Обратное percent-декодирование; `+` не считается пробелом.
    DecodeString,

    /// `ПолучитьДвоичныеДанныеИзСтроки(Строка[, Кодировка][, ДобавлятьBOM])`
    /// (см. `bindata::binary_data_from_string`).
    GetBinaryDataFromString,
    /// `ПолучитьБуферДвоичныхДанныхИзСтроки` — то же самое, но в буфер.
    GetBinaryDataBufferFromString,
    /// `ПолучитьСтрокуИзДвоичныхДанных(Данные[, Кодировка])`; ведущая
    /// сигнатура снимается (измерено).
    GetStringFromBinaryData,
    /// `ПолучитьСтрокуИзБуфераДвоичныхДанных(Буфер[, Кодировка])`.
    GetStringFromBinaryDataBuffer,
    /// Снимок байтов буфера как неизменяемые `ДвоичныеДанные`.
    GetBinaryDataFromBinaryDataBuffer,
    /// Новый изменяемый буфер из байтов `ДвоичныеДанные`.
    GetBinaryDataBufferFromBinaryData,

    /// `АргументыКоманднойСтроки` — массив строк, переданных скрипту после
    /// его имени в командной строке. В 1С такой глобальной функции нет —
    /// это расширение по образцу OneScript, мерить его не на чем; резолвер
    /// принимает и запись без скобок, как у свойства глобального контекста
    /// oscript. В отличие от его `ФиксированногоМассива` возвращается
    /// обычный `Массив`, построенный заново на каждое чтение, — правки не
    /// переживают следующее обращение.
    CommandLineArguments,
    /// Глобальное свойство конфигурации. В standalone-среде коллекция
    /// общих модулей пуста.
    Metadata,

    /// `ПолучитьИмяВременногоФайла([Расширение])` — отсутствующий путь из
    /// пространства host-файловой системы.
    GetTempFileName,
    /// Разделитель пути host-файловой системы.
    GetPathSeparator,
    /// `УдалитьФайлы(Путь)` — файл либо дерево; отсутствующий путь не ошибка.
    DeleteFiles,
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
    ("Мин", BuiltinFn::Min),
    ("Min", BuiltinFn::Min),
    ("Макс", BuiltinFn::Max),
    ("Max", BuiltinFn::Max),
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
    ("СтрНачинаетсяС", BuiltinFn::StrStartsWith),
    ("StrStartsWith", BuiltinFn::StrStartsWith),
    ("СтрЗаканчиваетсяНа", BuiltinFn::StrEndsWith),
    ("StrEndsWith", BuiltinFn::StrEndsWith),
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
    ("НСтр", BuiltinFn::LocalizedString),
    ("NStr", BuiltinFn::LocalizedString),
    ("ПустаяСтрока", BuiltinFn::IsBlankString),
    ("IsBlankString", BuiltinFn::IsBlankString),
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
    ("ИнформацияОбОшибке", BuiltinFn::ErrorInfo),
    ("ErrorInfo", BuiltinFn::ErrorInfo),
    (
        "ПодробноеПредставлениеОшибки",
        BuiltinFn::DetailedErrorDescription,
    ),
    (
        "DetailedErrorDescription",
        BuiltinFn::DetailedErrorDescription,
    ),
    ("Дата", BuiltinFn::MakeDate),
    ("Date", BuiltinFn::MakeDate),
    ("ТекущаяДата", BuiltinFn::CurrentDate),
    ("CurrentDate", BuiltinFn::CurrentDate),
    ("ТекущаяУниверсальнаяДата", BuiltinFn::CurrentUniversalDate),
    ("CurrentUniversalDate", BuiltinFn::CurrentUniversalDate),
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
    ("Метаданные", BuiltinFn::Metadata),
    ("Metadata", BuiltinFn::Metadata),
    ("ПолучитьИмяВременногоФайла", BuiltinFn::GetTempFileName),
    ("GetTempFileName", BuiltinFn::GetTempFileName),
    ("ПолучитьРазделительПути", BuiltinFn::GetPathSeparator),
    ("GetPathSeparator", BuiltinFn::GetPathSeparator),
    ("УдалитьФайлы", BuiltinFn::DeleteFiles),
    ("DeleteFiles", BuiltinFn::DeleteFiles),
    // Оба написания ИЗМЕРЕНЫ на файле схемы: и `СоздатьФабрикуXDTO`, и
    // `CreateXDTOFactory` отдают фабрику.
    // `ФабрикаXDTO` у платформы — СВОЙСТВО глобального контекста, а не
    // функция, и резолвер разрешает именно голое имя (как
    // `АргументыКоманднойСтроки`). Строка в таблице всё равно нужна:
    // текстовый формат байт-кода печатает и разбирает встроенную функцию
    // по имени, а безымянный вариант не пережил бы round-trip.
    ("РазделитьДвоичныеДанные", BuiltinFn::SplitBinaryData),
    ("SplitBinaryData", BuiltinFn::SplitBinaryData),
    ("СоединитьДвоичныеДанные", BuiltinFn::ConcatBinaryData),
    ("ConcatBinaryData", BuiltinFn::ConcatBinaryData),
    ("Base64Значение", BuiltinFn::Base64Value),
    ("Base64Value", BuiltinFn::Base64Value),
    (
        "ПолучитьBase64СтрокуИзДвоичныхДанных",
        BuiltinFn::GetBase64StringFromBinaryData,
    ),
    (
        "GetBase64StringFromBinaryData",
        BuiltinFn::GetBase64StringFromBinaryData,
    ),
    (
        "ПолучитьДвоичныеДанныеИзBase64Строки",
        BuiltinFn::GetBinaryDataFromBase64String,
    ),
    (
        "GetBinaryDataFromBase64String",
        BuiltinFn::GetBinaryDataFromBase64String,
    ),
    (
        "ПолучитьHexСтрокуИзДвоичныхДанных",
        BuiltinFn::GetHexStringFromBinaryData,
    ),
    (
        "GetHexStringFromBinaryData",
        BuiltinFn::GetHexStringFromBinaryData,
    ),
    ("КодироватьСтроку", BuiltinFn::EncodeString),
    ("EncodeString", BuiltinFn::EncodeString),
    ("РаскодироватьСтроку", BuiltinFn::DecodeString),
    ("DecodeString", BuiltinFn::DecodeString),
    (
        "ПолучитьДвоичныеДанныеИзСтроки",
        BuiltinFn::GetBinaryDataFromString,
    ),
    (
        "GetBinaryDataFromString",
        BuiltinFn::GetBinaryDataFromString,
    ),
    (
        "ПолучитьБуферДвоичныхДанныхИзСтроки",
        BuiltinFn::GetBinaryDataBufferFromString,
    ),
    (
        "GetBinaryDataBufferFromString",
        BuiltinFn::GetBinaryDataBufferFromString,
    ),
    (
        "ПолучитьСтрокуИзДвоичныхДанных",
        BuiltinFn::GetStringFromBinaryData,
    ),
    (
        "GetStringFromBinaryData",
        BuiltinFn::GetStringFromBinaryData,
    ),
    (
        "ПолучитьСтрокуИзБуфераДвоичныхДанных",
        BuiltinFn::GetStringFromBinaryDataBuffer,
    ),
    (
        "GetStringFromBinaryDataBuffer",
        BuiltinFn::GetStringFromBinaryDataBuffer,
    ),
    (
        "ПолучитьДвоичныеДанныеИзБуфераДвоичныхДанных",
        BuiltinFn::GetBinaryDataFromBinaryDataBuffer,
    ),
    (
        "GetBinaryDataFromBinaryDataBuffer",
        BuiltinFn::GetBinaryDataFromBinaryDataBuffer,
    ),
    (
        "ПолучитьБуферДвоичныхДанныхИзДвоичныхДанных",
        BuiltinFn::GetBinaryDataBufferFromBinaryData,
    ),
    (
        "GetBinaryDataBufferFromBinaryData",
        BuiltinFn::GetBinaryDataBufferFromBinaryData,
    ),
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

    /// Функция ВСТРОЕННОГО ЯЗЫКА — в отличие от функции глобального
    /// контекста. Разница видна только в позиции оператора: `Строка(1);`
    /// платформа отказывается компилировать («Встроенная функция может
    /// быть использована только в выражении»), а `СтрНайти("а", "б");` —
    /// обычный вызов с отброшенным результатом, и нуль-арная
    /// `ТекущаяУниверсальнаяДатаВМиллисекундах();` оператором просто
    /// выполняется.
    ///
    /// ИЗМЕРЕНО на 8.3.27 (2026-08-14) полным перебором таблицы
    /// [`BUILTIN_FN_NAMES`]: каждое имя вызвано оператором
    /// (`Выполнить("Имя();")`) и в выражении (`Вычислить("Имя()")`), классы
    /// сняты по текстам ошибок компиляции; русское и английское написания
    /// каждого варианта ответили одинаково. Канарейки правила — якоря
    /// `CALL.*` в `open_questions.rs` и скрипт
    /// `tests/conformance/measure/measure-builtin-call.bsl`.
    /// `Вычислить`/`Eval` — тоже функция языка, но в этой таблице её нет:
    /// у резолвера это отдельная форма `DynEval`. Расширение, которого у
    /// платформы нет (`CommandLineArguments` — «Процедура или функция с
    /// указанным именем не определена»), правилом не связано и остаётся
    /// обычной функцией.
    pub fn is_intrinsic(self) -> bool {
        matches!(
            self,
            BuiltinFn::Acos
                | BuiltinFn::AddMonth
                | BuiltinFn::Asin
                | BuiltinFn::Atan
                | BuiltinFn::Char
                | BuiltinFn::CharCode
                | BuiltinFn::Cos
                | BuiltinFn::CurrentDate
                | BuiltinFn::DateBoundaryOf(..)
                | BuiltinFn::DatePartOf(..)
                | BuiltinFn::Exp
                | BuiltinFn::Format
                | BuiltinFn::Left
                | BuiltinFn::Ln
                | BuiltinFn::Log10
                | BuiltinFn::Lower
                | BuiltinFn::Max
                | BuiltinFn::MakeDate
                | BuiltinFn::Mid
                | BuiltinFn::Min
                | BuiltinFn::Pow
                | BuiltinFn::Right
                | BuiltinFn::Round
                | BuiltinFn::Sin
                | BuiltinFn::Sqrt
                | BuiltinFn::StrGetLine
                | BuiltinFn::StrLen
                | BuiltinFn::StrLineCount
                | BuiltinFn::StrReplace
                | BuiltinFn::Tan
                | BuiltinFn::ToNumber
                | BuiltinFn::ToString
                | BuiltinFn::TrimAll
                | BuiltinFn::TrimLeft
                | BuiltinFn::TrimRight
                | BuiltinFn::Trunc
                | BuiltinFn::TypeByName
                | BuiltinFn::TypeOf
                | BuiltinFn::Upper
        )
    }

    /// За чем встроенная функция ходит наружу, помимо своих аргументов.
    ///
    /// ЕДИНСТВЕННЫЙ источник истины на весь рабочий процесс: по нему
    /// интерпретатор выбирает узкий вход ([`call_builtin_env`],
    /// [`call_builtin_files`]), а JIT решает, что не компилировать вовсе.
    /// Раньше это были три независимых списка с комментарием «обязаны
    /// совпадать» — то есть три возможности разойтись.
    ///
    /// `None` — функция считает ответ по одним аргументам, и её вправе
    /// исполнить кто угодно, в том числе шим нативного пути с
    /// sink-потоками и без окружения.
    #[must_use]
    pub fn host_effect(self) -> Option<HostEffect> {
        match self {
            BuiltinFn::Message => Some(HostEffect::Output),
            BuiltinFn::CurrentDate
            | BuiltinFn::CurrentUniversalDate
            | BuiltinFn::CurrentUniversalDateInMilliseconds
            | BuiltinFn::CommandLineArguments => Some(HostEffect::Env),
            BuiltinFn::ValueToFile | BuiltinFn::ValueFromFile => Some(HostEffect::Files),
            BuiltinFn::GetPathSeparator | BuiltinFn::DeleteFiles => Some(HostEffect::Files),
            BuiltinFn::GetTempFileName => Some(HostEffect::TempFiles),
            _ => None,
        }
    }

    /// Глобальная ПРОЦЕДУРА: оператором звать можно, а в позиции выражения
    /// платформа отвечает «Обращение к процедуре как к функции». Замер тот
    /// же, что у [`BuiltinFn::is_intrinsic`].
    pub fn is_procedure(self) -> bool {
        matches!(
            self,
            BuiltinFn::Message | BuiltinFn::FillPropertyValues | BuiltinFn::DeleteFiles
        )
    }

    /// Сохраняет ли байткод фактическое число аргументов вместо
    /// дополнения необязательных позиций значениями `Неопределено`.
    #[must_use]
    pub fn is_variadic(self) -> bool {
        matches!(self, BuiltinFn::Min | BuiltinFn::Max)
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
            | BuiltinFn::StrStartsWith
            | BuiltinFn::StrEndsWith
            | BuiltinFn::StrConcat
            | BuiltinFn::StrGetLine
            | BuiltinFn::EncodeString
            | BuiltinFn::DecodeString => (2, 2),
            BuiltinFn::StrReplace => (3, 3),
            // Платформа принимает как один аргумент, так и 256
            // (`measure-min-max.bsl`). Наш формат хранит `count` в `u8`,
            // поэтому 255 — измеренное техническое ограничение open-bsl.
            BuiltinFn::Min | BuiltinFn::Max => (1, u8::MAX as usize),
            // Границы ИЗМЕРЕНЫ перебором числа аргументов на 8.3.27: при
            // меньшем платформа отвечает «Недостаточно фактических
            // параметров», при большем — «Слишком много фактических
            // параметров».
            BuiltinFn::Round => (3, 3),
            // Длину можно не указывать — до конца строки.
            BuiltinFn::Mid => (2, 3),
            BuiltinFn::StrFind => (2, 5),
            // Третий аргумент включает пустые части и по умолчанию равен
            // `Истина` (oracle `measure-string-split.bsl`).
            BuiltinFn::StrSplit => (2, 3),
            // Позиция по умолчанию — первая.
            BuiltinFn::CharCode => (1, 2),
            // Шаблон плюс до десяти значений.
            BuiltinFn::StrTemplate => (1, 1 + crate::string::MAX_TEMPLATE_ARGS),
            BuiltinFn::LocalizedString => (1, 2),
            BuiltinFn::AddMonth => (2, 2),
            // `Дата(Год, Месяц, День[, Час, Минута, Секунда])` —
            // минимум три; строковая форма `Дата("...")` — это один
            // аргумент, поэтому нижняя граница всё-таки 1, а какая из двух
            // форм имелась в виду, решает тип первого аргумента в
            // `BslValue::make_date`.
            BuiltinFn::MakeDate => (1, 6),
            BuiltinFn::CurrentDate
            | BuiltinFn::CurrentUniversalDate
            | BuiltinFn::CurrentUniversalDateInMilliseconds
            | BuiltinFn::CommandLineArguments
            | BuiltinFn::Metadata
            | BuiltinFn::ErrorInfo
            | BuiltinFn::GetPathSeparator => (0, 0),
            BuiltinFn::GetTempFileName => (0, 1),
            BuiltinFn::DeleteFiles => (1, 1),
            // Оба списка свойств необязательны; недостающие позиции
            // резолвер добьёт `Неопределено`, что и значит «не задан».
            BuiltinFn::FillPropertyValues => (2, 4),
            BuiltinFn::ValueToStringInternal | BuiltinFn::ValueFromStringInternal => (1, 1),
            BuiltinFn::ValueToFile => (2, 2),
            BuiltinFn::ValueFromFile => (1, 1),
            // Обе арности строгие: платформа отвергает и
            // `РазделитьДвоичныеДанные` с одним аргументом, и
            // `СоединитьДвоичныеДанные` без аргументов (пробы
            // `BIN.SPLIT.ONEARG`, `BIN.COMBINE.NOARG`).
            BuiltinFn::SplitBinaryData => (2, 2),
            BuiltinFn::ConcatBinaryData => (1, 1),
            BuiltinFn::GetBinaryDataFromString | BuiltinFn::GetBinaryDataBufferFromString => (1, 3),
            BuiltinFn::GetStringFromBinaryData | BuiltinFn::GetStringFromBinaryDataBuffer => (1, 2),
            // Сообщить(<ТекстСообщения>, <Статус>) — по синтакс-помощнику
            // 8.3.27 второй параметр (СтатусСообщения) необязателен и «в
            // режиме управляемого приложения игнорируется»; open-bsl
            // моделирует управляемый серверный контекст, поэтому статус
            // принимается и игнорируется.
            BuiltinFn::Message => (1, 2),
            _ => (1, 1),
        }
    }
}

/// Методы, которые исполняет базовый рантайм.
///
/// Методы компонентных объектов живут в `MethodDescriptor` своего пакета:
/// общий слой не перечисляет внешний мир и не резервирует его имена.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinMethod {
    Count,
    /// `Массив.ВГраница()` — последний индекс или `-1` для пустого массива.
    UpperBound,
    Add,
    Delete,
    Clear,
    /// `Структура.Вставить(Ключ, Значение)` / `Соответствие.Вставить(Ключ, Значение)`.
    Insert,
    /// `Соответствие.Получить(Ключ)` и `БуферДвоичныхДанных.Получить(Позиция)`.
    Get,
    /// `Структура.Свойство(Ключ[, ЗначениеПоУмолчанию])`.
    Property,
    Find,
    FindRows,
    Sort,
    FillValues,
    Total,
    Copy,
    CopyColumns,
    UnloadColumn,
    LoadColumn,
    Move,
    IndexOf,
    Collapse,
    /// `ЗаписьТекста.Записать(Текст)` и блочная запись в буфер.
    Write,
    /// `ЗаписьТекста.Закрыть()`.
    Close,
    /// `ДвоичныеДанные.Размер()`.
    Size,
    /// `ДвоичныеДанные.ОткрытьПотокДляЧтения()` — объект строит
    /// зарегистрированный компонент потоков.
    OpenStreamForRead,
    /// `ОписаниеТипов.ПривестиЗначение(Значение)`.
    AdjustValue,
    BufSet,
    ReadInt16,
    ReadInt32,
    ReadInt64,
    WriteInt16,
    WriteInt32,
    WriteInt64,
    BufSplit,
    BufConcat,
    BufSlice,
    WriteBitwiseAnd,
    WriteBitwiseOr,
    WriteBitwiseXor,
    WriteBitwiseAndNot,
    Invert,
}

/// Написания методов, которые исполняет базовый рантайм. Методы компонентных
/// объектов перечисляет `MethodDescriptor` соответствующей библиотеки.
pub const BUILTIN_METHOD_NAMES: &[(&str, BuiltinMethod)] = &[
    ("Количество", BuiltinMethod::Count),
    ("Count", BuiltinMethod::Count),
    ("ВГраница", BuiltinMethod::UpperBound),
    ("UBound", BuiltinMethod::UpperBound),
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
    ("Размер", BuiltinMethod::Size),
    ("Size", BuiltinMethod::Size),
    ("ОткрытьПотокДляЧтения", BuiltinMethod::OpenStreamForRead),
    ("OpenStreamForRead", BuiltinMethod::OpenStreamForRead),
    ("ПривестиЗначение", BuiltinMethod::AdjustValue),
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
    ("ПолучитьСрез", BuiltinMethod::BufSlice),
    ("GetSlice", BuiltinMethod::BufSlice),
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
    ///
    /// Ищет по хеш-карте, построенной один раз на процесс: линейный проход
    /// с `to_uppercase` каждой строки таблицы стоил сотни аллокаций на
    /// обращение, а `lookup` зовут не только при резолвинге, но и при
    /// связывании каждого фрагмента `Выполнить`/`Вычислить`. При совпадении
    /// имён карта сохраняет семантику прежнего `find` — победитель тот, кто
    /// в таблице раньше.
    pub fn lookup(name: &str) -> Option<Self> {
        static BY_UPPER: std::sync::OnceLock<std::collections::HashMap<String, BuiltinMethod>> =
            std::sync::OnceLock::new();
        let map = BY_UPPER.get_or_init(|| {
            let mut map = std::collections::HashMap::with_capacity(BUILTIN_METHOD_NAMES.len());
            for (table_name, method) in BUILTIN_METHOD_NAMES {
                map.entry(table_name.to_uppercase()).or_insert(*method);
            }
            map
        });
        map.get(&name.to_uppercase()).copied()
    }

    /// Основное русское имя для перехода legacy-`CallMethod` к открытому
    /// объектному протоколу. В таблице оно всегда предшествует английскому
    /// синониму.
    pub fn primary_name(self) -> &'static str {
        BUILTIN_METHOD_NAMES
            .iter()
            .find(|(_, method)| *method == self)
            .map(|(name, _)| *name)
            .expect("каждый BuiltinMethod обязан иметь строку в таблице имён")
    }

    /// Статическая арность метода: `Some(n)` — метод берёт ровно `n`
    /// аргументов при ЛЮБОМ получателе; `None` — арность полиморфна по типу
    /// получателя (`Добавить` — 0 у таблицы, 1 у массива) или имеет несколько
    /// допустимых форм и решается рантаймом (см. [`call_builtin_method`]).
    ///
    /// Живёт здесь, а не в `bsl-sema`: проверить арность обязан и `bsl-vm`
    /// на КРАФТНУТОМ байт-коде (`bsl-vm` не видит `bsl-sema`), иначе метод с
    /// недостающим аргументом уронил бы VM на `args[0]`. Матч исчерпывающий
    /// и без `_`: новый метод не соберётся, пока его арность не отнесут к
    /// фиксированной или полиморфной. Числа ИЗМЕРЕНЫ — см. комментарии у
    /// ветвей.
    #[must_use]
    pub fn static_arity(self) -> Option<usize> {
        match self {
            BuiltinMethod::Count
            | BuiltinMethod::UpperBound
            | BuiltinMethod::Clear
            | BuiltinMethod::Close
            | BuiltinMethod::Size
            | BuiltinMethod::OpenStreamForRead => Some(0),
            BuiltinMethod::Delete
            | BuiltinMethod::FindRows
            | BuiltinMethod::Total
            | BuiltinMethod::UnloadColumn
            | BuiltinMethod::IndexOf
            | BuiltinMethod::AdjustValue
            | BuiltinMethod::BufSplit
            | BuiltinMethod::BufConcat => Some(1),
            BuiltinMethod::LoadColumn
            | BuiltinMethod::Move
            | BuiltinMethod::WriteBitwiseAnd
            | BuiltinMethod::WriteBitwiseOr
            | BuiltinMethod::WriteBitwiseXor
            | BuiltinMethod::WriteBitwiseAndNot => Some(2),
            BuiltinMethod::Add
            | BuiltinMethod::Insert
            | BuiltinMethod::Get
            | BuiltinMethod::Property
            | BuiltinMethod::Find
            | BuiltinMethod::Sort
            | BuiltinMethod::FillValues
            | BuiltinMethod::Copy
            | BuiltinMethod::CopyColumns
            | BuiltinMethod::Collapse
            | BuiltinMethod::Write
            | BuiltinMethod::ReadInt16
            | BuiltinMethod::ReadInt32
            | BuiltinMethod::ReadInt64
            | BuiltinMethod::WriteInt16
            | BuiltinMethod::WriteInt32
            | BuiltinMethod::WriteInt64
            | BuiltinMethod::BufSlice
            | BuiltinMethod::BufSet
            | BuiltinMethod::Invert => None,
        }
    }

    /// Нужна ли методу возможность host-окружения.
    ///
    /// Имя `Записать` полиморфно: файловый эффект имеет только получатель
    /// `ДвоичныеДанные`, однако JIT не всегда знает тип получателя. Поэтому
    /// одна такая инструкция остаётся интерпретатору, где узкий файловый
    /// вход отличает двоичные данные, а остальные получатели делегирует
    /// обычной диспетчеризации. Остальная часть чанка компилируется.
    #[must_use]
    pub fn host_effect(self) -> Option<HostEffect> {
        match self {
            BuiltinMethod::Write => Some(HostEffect::Files),
            _ => None,
        }
    }
}

fn localized_string(args: &[BslValue]) -> RtResult<BslValue> {
    let source = args[0].as_str("НСтр")?.to_string();
    // ИЗМЕРЕНО(NSTR.MULTI_LANGUAGE): конфигурация oracle-сеанса выбирает
    // английское значение. Явный второй аргумент позволяет выбрать иной
    // язык без зависимости от окружения процесса.
    let language = match args.get(1) {
        None | Some(BslValue::Undefined) => "en".to_string(),
        Some(BslValue::Str(value)) => value.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "НСтр(..., КодЯзыка)",
            });
        }
    };
    let mut input = source.chars().peekable();
    loop {
        while input
            .peek()
            .is_some_and(|ch| ch.is_whitespace() || *ch == ';')
        {
            input.next();
        }
        if input.peek().is_none() {
            return Ok(BslValue::Str(BslString::from_str("")));
        }

        let mut code = String::new();
        while input
            .peek()
            .is_some_and(|ch| !ch.is_whitespace() && *ch != '=')
        {
            let Some(ch) = input.next() else {
                break;
            };
            code.push(ch);
        }
        while input.peek().is_some_and(|ch| ch.is_whitespace()) {
            input.next();
        }
        if input.next() != Some('=') {
            return Ok(BslValue::Str(BslString::from_str("")));
        }
        while input.peek().is_some_and(|ch| ch.is_whitespace()) {
            input.next();
        }
        if input.next() != Some('\'') {
            return Ok(BslValue::Str(BslString::from_str("")));
        }

        let mut value = String::new();
        let mut closed = false;
        while let Some(ch) = input.next() {
            if ch != '\'' {
                value.push(ch);
                continue;
            }
            if input.peek() == Some(&'\'') {
                input.next();
                value.push('\'');
            } else {
                closed = true;
                break;
            }
        }
        if !closed {
            return Ok(BslValue::Str(BslString::from_str("")));
        }
        if code.eq_ignore_ascii_case(&language) {
            return Ok(BslValue::Str(BslString::from_utf8_string(value)));
        }
    }
}

pub fn call_builtin_fn(f: BuiltinFn, args: &[BslValue]) -> RtResult<BslValue> {
    // Сторож входа публичной функции: без него недостающий аргумент падает
    // на `args[0]`. Путь VM защищён периметром образа (`check_call_geometry`),
    // но функция публична — прямой вызывающий её проверки не проходит.
    if args.len() < f.arity_range().0 {
        return Err(RtError::InvalidBytecode(
            "встроенной функции передано меньше аргументов, чем требует её арность",
        ));
    }
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
        BuiltinFn::Min => min_max(args, true),
        BuiltinFn::Max => min_max(args, false),
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
        BuiltinFn::StrFind => args[0].str_find(&args[1], &args[2], &args[3], &args[4]),
        BuiltinFn::StrStartsWith => args[0].str_starts_with(&args[1]),
        BuiltinFn::StrEndsWith => args[0].str_ends_with(&args[1]),
        BuiltinFn::StrReplace => args[0].str_replace(&args[1], &args[2]),
        BuiltinFn::StrSplit => args[0].str_split(&args[1], &args[2]),
        BuiltinFn::StrConcat => args[0].str_join(&args[1]),
        BuiltinFn::StrLineCount => args[0].str_line_count(),
        BuiltinFn::StrGetLine => args[0].str_get_line(&args[1]),
        BuiltinFn::StrTemplate => args[0].str_template(&args[1..]),
        BuiltinFn::LocalizedString => localized_string(args),
        BuiltinFn::IsBlankString => Ok(BslValue::Boolean(match &args[0] {
            BslValue::Undefined => true,
            BslValue::Str(value) => value.to_string().trim().is_empty(),
            _ => false,
        })),
        BuiltinFn::Char => args[0].char_from_code(),
        BuiltinFn::CharCode => args[0].char_code(&args[1]),
        BuiltinFn::ValueIsFilled => Ok(BslValue::Boolean(args[0].is_filled()?)),
        BuiltinFn::TypeOf => args[0].type_of(),
        // Перехвачена в `call_builtin_fn_ctx`: без списка типов прогона
        // видны только нативные имена.
        BuiltinFn::TypeByName => args[0].type_by_name(),
        BuiltinFn::ErrorInfo => Err(RtError::InvalidBytecode(
            "ИнформацияОбОшибке требует контекста текущей BSL-задачи",
        )),
        BuiltinFn::DetailedErrorDescription => crate::detailed_error_description(&args[0]),
        BuiltinFn::MakeDate => BslValue::make_date(args),
        // Три функции окружения перехвачены в `call_builtin_fn_ctx`: часы,
        // случайность и аргументы запуска принадлежат ПРОГОНУ, и без него
        // отвечать нечем. Ошибка, а не `unreachable!`, по той же причине,
        // что у соседей ниже: функция публична, и прямой вызов из
        // Rust-кода не должен ронять процесс.
        BuiltinFn::CurrentDate
        | BuiltinFn::CurrentUniversalDate
        | BuiltinFn::CurrentUniversalDateInMilliseconds
        | BuiltinFn::CommandLineArguments => Err(RtError::InvalidBytecode(
            "функция окружения вызвана без окружения прогона",
        )),
        BuiltinFn::Metadata => Ok(crate::metadata::new_metadata()),
        BuiltinFn::DatePartOf(part) => args[0].date_component(part),
        BuiltinFn::DateBoundaryOf(which) => args[0].date_boundary(which),
        BuiltinFn::AddMonth => args[0].add_month(&args[1]),
        // Перехвачена в `call_builtin_fn_ctx` — без таблицы имён набор
        // полей приёмника не прочитать. Ошибка, а не `unreachable!`: эта
        // функция публична, и ронять процесс на прямом вызове из
        // встраивающего приложения незачем (то же соображение, что и у
        // `RtError::InvalidBytecode`).
        BuiltinFn::SplitBinaryData => args[0].binary_data_split(&args[1]),
        BuiltinFn::ConcatBinaryData => args[0].binary_data_combine(),
        BuiltinFn::Base64Value => {
            let text = args[0].as_str("Base64")?.to_string();
            Ok(crate::encoding::decode_base64(&text)
                .map(BslValue::binary_data_of)
                .unwrap_or(BslValue::Undefined))
        }
        BuiltinFn::GetBinaryDataFromBase64String => {
            let text = args[0]
                .as_str("ПолучитьДвоичныеДанныеИзBase64Строки")?
                .to_string();
            let bytes = crate::encoding::decode_base64(&text).unwrap_or_default();
            Ok(BslValue::binary_data_of(bytes))
        }
        BuiltinFn::GetBase64StringFromBinaryData => {
            let bytes = args[0].binary_data_bytes().ok_or(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ПолучитьBase64СтрокуИзДвоичныхДанных",
            })?;
            Ok(BslValue::Str(BslString::from_utf8_string(
                crate::encoding::encode_base64(bytes),
            )))
        }
        BuiltinFn::GetHexStringFromBinaryData => {
            let bytes = args[0].binary_data_bytes().ok_or(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ПолучитьHexСтрокуИзДвоичныхДанных",
            })?;
            Ok(BslValue::Str(BslString::from_utf8_string(
                crate::encoding::encode_hex(bytes),
            )))
        }
        BuiltinFn::EncodeString => {
            let text = args[0].as_str("КодироватьСтроку")?.to_string();
            let mode = match args[1] {
                BslValue::Enum(crate::EnumValue::StringEncodingUrl) => {
                    crate::encoding::UrlEncodingMode::Url
                }
                BslValue::Enum(crate::EnumValue::StringEncodingUrlInUrl) => {
                    crate::encoding::UrlEncodingMode::UrlInUrl
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "СпособКодированияСтроки",
                        op: "КодироватьСтроку",
                    });
                }
            };
            Ok(BslValue::Str(BslString::from_utf8_string(
                crate::encoding::encode_url(&text, mode),
            )))
        }
        BuiltinFn::DecodeString => {
            let text = args[0].as_str("РаскодироватьСтроку")?.to_string();
            match args[1] {
                BslValue::Enum(
                    crate::EnumValue::StringEncodingUrl | crate::EnumValue::StringEncodingUrlInUrl,
                ) => Ok(BslValue::Str(BslString::from_utf8_string(
                    crate::encoding::decode_url(&text),
                ))),
                _ => Err(RtError::TypeError {
                    expected: "СпособКодированияСтроки",
                    op: "РаскодироватьСтроку",
                }),
            }
        }
        BuiltinFn::GetBinaryDataFromString => crate::bindata::binary_data_from_string(args),
        BuiltinFn::GetBinaryDataBufferFromString => crate::bindata::binary_buffer_from_string(args),
        BuiltinFn::GetStringFromBinaryData => crate::bindata::string_from_binary_data(args),
        BuiltinFn::GetStringFromBinaryDataBuffer => crate::bindata::string_from_binary_buffer(args),
        BuiltinFn::GetBinaryDataFromBinaryDataBuffer => {
            let bytes = args[0].binary_buffer_bytes().ok_or(RtError::TypeError {
                expected: "БуферДвоичныхДанных",
                op: "ПолучитьДвоичныеДанныеИзБуфераДвоичныхДанных",
            })?;
            Ok(BslValue::binary_data_of(bytes))
        }
        BuiltinFn::GetBinaryDataBufferFromBinaryData => {
            let bytes = args[0].binary_data_bytes().ok_or(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op: "ПолучитьБуферДвоичныхДанныхИзДвоичныхДанных",
            })?;
            Ok(BslValue::binary_buffer_of(bytes.to_vec()))
        }
        BuiltinFn::ValueToStringInternal | BuiltinFn::ValueFromStringInternal => {
            Err(RtError::InvalidBytecode(
                "функции внутреннего формата требуют контекста имён: вызывайте call_builtin_fn_ctx",
            ))
        }
        BuiltinFn::ValueToFile | BuiltinFn::ValueFromFile => Err(RtError::InvalidBytecode(
            "файловые функции требуют файловой системы прогона: вызывайте call_builtin_files",
        )),
        BuiltinFn::GetPathSeparator | BuiltinFn::DeleteFiles => Err(RtError::InvalidBytecode(
            "файловые функции требуют файловой системы прогона: вызывайте call_builtin_files",
        )),
        BuiltinFn::GetTempFileName => Err(RtError::InvalidBytecode(
            "временный путь требует файловой системы и случайности прогона",
        )),
        BuiltinFn::FillPropertyValues => Err(RtError::InvalidBytecode(
            "ЗаполнитьЗначенияСвойств требует контекста имён: вызывайте call_builtin_fn_ctx",
        )),
    }
}

fn min_max(args: &[BslValue], minimum: bool) -> RtResult<BslValue> {
    let mut best = args[0].clone();
    for candidate in &args[1..] {
        let order = match (&best, candidate) {
            (BslValue::Boolean(left), BslValue::Boolean(right)) => left.cmp(right),
            (BslValue::Boolean(left), BslValue::Number(right)) => {
                bsl_number::BslNumber::from_i64(i64::from(*left)).cmp(right)
            }
            (BslValue::Number(left), BslValue::Boolean(right)) => {
                left.cmp(&bsl_number::BslNumber::from_i64(i64::from(*right)))
            }
            _ => best.compare(candidate, if minimum { "Мин" } else { "Макс" })?,
        };
        if (minimum && order.is_gt()) || (!minimum && order.is_lt()) {
            best = candidate.clone();
        }
    }
    Ok(best)
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
    if f == BuiltinFn::TypeByName {
        let name = args[0].as_str("Тип")?.to_string();
        // Одно разрешение на оба вызывающих (см. `RuntimeShapes::resolve_type`):
        // ядро раньше компонентов, порядок записан в одной функции. Прежде
        // `Тип` спрашивал компоненты первыми, а `ОписаниеТипов` — ядро.
        return match rt.resolve_type(&name) {
            Some(ty) => Ok(BslValue::Type(ty)),
            None => Err(RtError::UnknownType(name)),
        };
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
    if matches!(
        f.host_effect(),
        Some(HostEffect::Files | HostEffect::TempFiles)
    ) {
        return Err(RtError::InvalidBytecode(
            "файловые функции требуют файловой системы прогона: вызывайте call_builtin_files",
        ));
    }
    call_builtin_fn(f, args)
}

/// `ЗначениеВФайл`/`ЗначениеИзФайла` — единственные встроенные функции,
/// которым нужна ФАЙЛОВАЯ СИСТЕМА прогона.
///
/// Отдельный вход, а не лишний параметр у [`call_builtin_fn_ctx`], по той
/// же причине, что и у [`call_builtin_env`]: остальные встроенные функции
/// об окружении ничего не знают, и требовать его со всех значило бы
/// уронить `Sqrt` там, где окружения нет, — например на нативном пути.
///
/// Арность проверяется ЗДЕСЬ, а не только у вызывающего: функция
/// публична, и опереться на проверку VM она не вправе — короткий срез
/// давал бы панику вместо ошибки, одинаково в debug и release.
///
/// # Errors
///
/// [`RtError::InvalidBytecode`] на неверном числе аргументов и на чужой
/// встроенной функции, [`RtError::TypeError`] на нестроковом имени файла,
/// ошибку ввода-вывода и ошибку разбора внутреннего формата.
pub fn call_builtin_files(
    f: BuiltinFn,
    args: &[BslValue],
    rt: &mut RuntimeShapes,
    files: &dyn crate::FileSystem,
) -> RtResult<BslValue> {
    let bad_arity =
        || RtError::InvalidBytecode("файловая функция вызвана не с тем числом аргументов");
    match f {
        BuiltinFn::ValueToFile => {
            let [path, value] = args else {
                return Err(bad_arity());
            };
            let BslValue::Str(path) = path else {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op: "ЗначениеВФайл(ИмяФайла)",
                });
            };
            crate::vstr::value_to_file(&path.to_string(), value, rt, files)?;
            Ok(BslValue::Undefined)
        }
        BuiltinFn::ValueFromFile => {
            let [path] = args else {
                return Err(bad_arity());
            };
            let BslValue::Str(path) = path else {
                return Err(RtError::TypeError {
                    expected: "Строка",
                    op: "ЗначениеИзФайла(ИмяФайла)",
                });
            };
            crate::vstr::value_from_file(&path.to_string(), rt, files)
        }
        BuiltinFn::GetPathSeparator => {
            let [] = args else {
                return Err(bad_arity());
            };
            let separator = files
                .path_separator()
                .map_err(|error| RtError::IoError(error.to_string()))?;
            Ok(BslValue::Str(BslString::from_utf8_string(separator)))
        }
        BuiltinFn::DeleteFiles => {
            let [path] = args else {
                return Err(bad_arity());
            };
            let path = path.as_str("УдалитьФайлы")?.to_string();
            files
                .remove_path(&path)
                .map_err(|error| RtError::IoError(format!("{path}: {error}")))?;
            Ok(BslValue::Undefined)
        }
        _ => Err(RtError::InvalidBytecode(
            "call_builtin_files вызвана не на файловой функции",
        )),
    }
}

/// `ПолучитьИмяВременногоФайла` соединяет две возможности одного прогона:
/// случайные байты уже получены из [`crate::RandomHandle`], а пространство
/// имён, проверка коллизии и резервирование принадлежат [`crate::FileSystem`].
///
/// # Errors
///
/// [`RtError::InvalidBytecode`] на чужой функции или неверной арности,
/// [`RtError::TypeError`] на нестроковом суффиксе и ошибку файловой системы.
pub fn call_builtin_temp_file(
    f: BuiltinFn,
    args: &[BslValue],
    files: &dyn crate::FileSystem,
    entropy: &[u8; 16],
) -> RtResult<BslValue> {
    if f != BuiltinFn::GetTempFileName || args.len() > 1 {
        return Err(RtError::InvalidBytecode(
            "call_builtin_temp_file вызвана не на своей функции или с неверной арностью",
        ));
    }
    let suffix = match args.first() {
        None | Some(BslValue::Undefined) => String::new(),
        Some(value) => value.as_str("ПолучитьИмяВременногоФайла")?.to_string(),
    };
    let path = files
        .temporary_path(&suffix, entropy)
        .map_err(|error| RtError::IoError(error.to_string()))?;
    Ok(BslValue::Str(BslString::from_utf8_string(path)))
}

/// Необязательный аргумент метода: отсутствующий читается как
/// `Неопределено` — ровно так же, как резолвер выравнивает арность
/// встроенных ФУНКЦИЙ (см. `BuiltinFn::arity_range`). У методов получатель
/// динамический, поэтому выравнивать приходится здесь.
fn arg(args: &[BslValue], i: usize) -> &BslValue {
    args.get(i).unwrap_or(&BslValue::Undefined)
}

/// Читает целое из буфера, начиная с позиции из первого аргумента.
fn read_int_by_receiver(
    obj: &BslValue,
    args: &[BslValue],
    w: crate::bindata::IntWidth,
) -> RtResult<BslValue> {
    crate::bindata::read_int(obj, args, w)
}

/// Записывает целое в буфер, начиная с позиции из первого аргумента.
fn write_int_by_receiver(
    obj: &BslValue,
    args: &[BslValue],
    w: crate::bindata::IntWidth,
) -> RtResult<BslValue> {
    crate::bindata::write_int(obj, args, w)
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
    // Сторож входа: у метода с ФИКСИРОВАННОЙ арностью недостающий аргумент
    // падал бы на `args[0]`. Полиморфные (`static_arity() == None`) проверяет
    // сам обработчик. Путь VM защищён периметром образа, но функция публична.
    if let Some(expected) = m.static_arity()
        && args.len() != expected
    {
        return Err(RtError::InvalidBytecode(
            "число аргументов метода не совпадает с его арностью",
        ));
    }
    match m {
        BuiltinMethod::Count => {
            let len = obj.collection_len()?;
            Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
                len as i64,
            )))
        }
        BuiltinMethod::UpperBound => match obj {
            BslValue::Object(object) if matches!(&**object, BslObject::Array(_)) => {
                let BslObject::Array(items) = &**object else {
                    unreachable!();
                };
                Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
                    items.borrow().len() as i64 - 1,
                )))
            }
            _ => Err(RtError::MethodNotApplicable {
                method: "ВГраница",
                receiver: obj.type_name(),
            }),
        },
        BuiltinMethod::Add => match args {
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
            obj.clear_collection()?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Insert => match obj {
            BslValue::Object(o) if matches!(&**o, BslObject::Map(_)) => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RtError::MethodNotApplicable {
                        method: "Вставить",
                        receiver: obj.type_name(),
                    });
                }
                let value = args.get(1).cloned().unwrap_or(BslValue::Undefined);
                obj.map_insert(args[0].clone(), value)?;
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
        // Арность проверяет обработчик: `Получить()` без
        // аргументов обязано стать понятной ошибкой, а не паникой на
        // `args[0]`.
        BuiltinMethod::Get => match obj {
            BslValue::Object(o) if matches!(&**o, BslObject::Map(_)) => match args {
                [key] => obj.map_get(key),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: obj.type_name(),
                }),
            },
            _ if crate::bindata::is_buffer(obj) => match args {
                [pos] => crate::bindata::get_byte(obj, pos),
                _ => Err(RtError::MethodNotApplicable {
                    method: "Получить",
                    receiver: obj.type_name(),
                }),
            },
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
            if let BslValue::Object(object) = obj
                && let BslObject::Array(items) = &**object
            {
                if args.len() != 1 {
                    return Err(RtError::MethodNotApplicable {
                        method: "Найти",
                        receiver: obj.type_name(),
                    });
                }
                return Ok(items
                    .borrow()
                    .iter()
                    .position(|item| item == value)
                    .map(|index| BslValue::Number(bsl_number::BslNumber::from_i64(index as i64)))
                    .unwrap_or(BslValue::Undefined));
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
        // `Скопировать` у буфера — независимая копия БЕЗ аргументов, у
        // таблицы значений — отбор и список колонок. Одно имя, разный смысл
        // по получателю, как у `Записать` и `Закрыть`.
        BuiltinMethod::Copy => {
            if crate::bindata::is_buffer(obj) {
                return crate::bindata::copy_buffer(obj, args);
            }
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
        // `Записать` дописывает кусок в `ЗаписьТекста` либо выполняет
        // блочную запись в `БуферДвоичныхДанных`.
        BuiltinMethod::Write => {
            if crate::bindata::is_buffer(obj) {
                // У буфера `Записать(Позиция, Источник[, Количество])` —
                // блочная запись; арность и границы проверяет он сам.
                crate::bindata::write_buffer(obj, args)
            } else {
                // Получатель здесь может оказаться и не `ЗаписьТекста`:
                // тогда индексация `args[0]` обязана быть безопасной, а
                // ошибка — понятной.
                match args {
                    [text] => obj.text_writer_write(text),
                    _ => Err(RtError::MethodNotApplicable {
                        method: "Записать",
                        receiver: obj.type_name(),
                    }),
                }
            }
        }
        // `ЗаписьТекста.Закрыть()` сбрасывает буфер и закрывает файл.
        BuiltinMethod::Close => obj.close_object(),
        // У `ДвоичныеДанные` размер — метод; у буфера это свойство.
        BuiltinMethod::Size => obj.binary_data_size(),
        // Для этого метода нужен поставщик из `RuntimeShapes`; обычный
        // вход оставляет понятный отказ, а рабочий перехватывается в
        // `call_builtin_method_ctx` ниже.
        BuiltinMethod::OpenStreamForRead => Err(RtError::MethodNotApplicable {
            method: "ОткрытьПотокДляЧтения",
            receiver: obj.type_name(),
        }),
        BuiltinMethod::AdjustValue => crate::type_description::adjust_value(obj, &args[0]),

        // --- БуферДвоичныхДанных ------------------------------------------
        BuiltinMethod::BufSet => match args {
            [pos, value] => crate::bindata::set_byte(obj, pos, value).map(|()| BslValue::Undefined),
            _ => Err(RtError::MethodNotApplicable {
                method: "Установить",
                receiver: obj.type_name(),
            }),
        },
        // У буфера первым аргументом чтения и записи целого идёт позиция.
        BuiltinMethod::ReadInt16 => read_int_by_receiver(obj, args, crate::bindata::IntWidth::W16),
        BuiltinMethod::ReadInt32 => read_int_by_receiver(obj, args, crate::bindata::IntWidth::W32),
        BuiltinMethod::ReadInt64 => read_int_by_receiver(obj, args, crate::bindata::IntWidth::W64),
        BuiltinMethod::WriteInt16 => {
            write_int_by_receiver(obj, args, crate::bindata::IntWidth::W16)
        }
        BuiltinMethod::WriteInt32 => {
            write_int_by_receiver(obj, args, crate::bindata::IntWidth::W32)
        }
        BuiltinMethod::WriteInt64 => {
            write_int_by_receiver(obj, args, crate::bindata::IntWidth::W64)
        }
        BuiltinMethod::BufSplit => crate::bindata::split(obj, &args[0]),
        BuiltinMethod::BufConcat => crate::bindata::concat(obj, &args[0]),
        BuiltinMethod::BufSlice => crate::bindata::get_slice(obj, args),
        BuiltinMethod::WriteBitwiseAnd => {
            crate::bindata::bitwise(obj, args, crate::bindata::BitOp::And)
        }
        BuiltinMethod::WriteBitwiseOr => {
            crate::bindata::bitwise(obj, args, crate::bindata::BitOp::Or)
        }
        BuiltinMethod::WriteBitwiseXor => {
            crate::bindata::bitwise(obj, args, crate::bindata::BitOp::Xor)
        }
        BuiltinMethod::WriteBitwiseAndNot => {
            crate::bindata::bitwise(obj, args, crate::bindata::BitOp::AndNot)
        }
        BuiltinMethod::Invert => crate::bindata::invert(obj, args),
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
    // Тот же сторож входа, что и у [`call_builtin_method`], и обязательно ДО
    // быстрого пути структуры: `Вставить`/`Удалить` читают `args[0]`/`args[1]`
    // сразу. Статическая проверка арности на связывании закрывает только
    // ЗАКРЫТЫЙ опкод `CallMethod`; у ОТКРЫТОГО (`CallObjectMethod`) метод
    // выбирается по номеру имени уже в рантайме, и до этого сторожа
    // недостающий аргумент ронял процесс, а не давал ошибку.
    if let Some(expected) = m.static_arity()
        && args.len() != expected
    {
        return Err(RtError::InvalidBytecode(
            "число аргументов метода не совпадает с его арностью",
        ));
    }
    if m == BuiltinMethod::OpenStreamForRead {
        let bytes = match obj {
            BslValue::Object(value) => match &**value {
                BslObject::BinaryData(bytes) => std::rc::Rc::clone(bytes),
                _ => {
                    return Err(RtError::MethodNotApplicable {
                        method: "ОткрытьПотокДляЧтения",
                        receiver: obj.type_name(),
                    });
                }
            },
            _ => {
                return Err(RtError::MethodNotApplicable {
                    method: "ОткрытьПотокДляЧтения",
                    receiver: obj.type_name(),
                });
            }
        };
        return rt.open_binary_data_stream(bytes);
    }
    if is_structure(obj) {
        match m {
            BuiltinMethod::Insert => {
                if args.is_empty() || args.len() > 2 {
                    return Err(RtError::InvalidBytecode(
                        "Вставить принимает один или два аргумента",
                    ));
                }
                let field = key_name(&args[0], rt)?;
                let value = args.get(1).cloned().unwrap_or(BslValue::Undefined);
                obj.structure_insert(field, value, &mut rt.shapes)?;
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

/// Исполняет встроенный метод с файловой возможностью прогона.
///
/// Сейчас это только `ДвоичныеДанные.Записать(Путь)`: файл создаётся либо
/// полностью обрезается, что измерено повторной записью `41 -> 4243`.
/// Полиморфные тёзки `Записать` не получают файловую систему и уходят в
/// [`call_builtin_method_ctx`].
///
/// # Errors
///
/// Ошибку арности или типа пути, ошибку записи файла либо ошибку обычного
/// метода для другого получателя.
pub fn call_builtin_method_files(
    m: BuiltinMethod,
    obj: &BslValue,
    args: &[BslValue],
    rt: &mut RuntimeShapes,
    files: &dyn crate::FileSystem,
) -> RtResult<BslValue> {
    if m == BuiltinMethod::Write
        && let BslValue::Object(object) = obj
        && let BslObject::BinaryData(bytes) = &**object
    {
        let [path] = args else {
            return Err(RtError::InvalidBytecode(
                "ДвоичныеДанные.Записать вызвана не с одним аргументом",
            ));
        };
        let path = path.as_str("ДвоичныеДанные.Записать")?.to_string();
        files
            .write(&path, bytes)
            .map_err(|error| RtError::IoError(format!("{path}: {error}")))?;
        return Ok(BslValue::Undefined);
    }
    call_builtin_method_ctx(m, obj, args, rt)
}

#[cfg(test)]
mod name_table_tests {
    use super::*;

    #[test]
    fn base64_builtins_follow_the_measured_invalid_input_contracts() {
        let bad = BslValue::Str(BslString::from_str("Zm$v"));
        assert_eq!(
            call_builtin_fn(BuiltinFn::Base64Value, std::slice::from_ref(&bad))
                .expect("ошибочная запись даёт значение, а не исключение"),
            BslValue::Undefined
        );
        let alias = call_builtin_fn(BuiltinFn::GetBinaryDataFromBase64String, &[bad])
            .expect("алиас возвращает пустые двоичные данные");
        assert_eq!(alias.binary_data_bytes(), Some([].as_slice()));
    }

    #[test]
    fn localized_string_selects_language_and_decodes_quotes_and_lines() {
        let source = BslValue::Str(BslString::from_str(
            "en = 'Hello'; ru = 'Первая\nВторая и ''цитата'''",
        ));
        let ru = BslValue::Str(BslString::from_str("ru"));
        let en = BslValue::Str(BslString::from_str("en"));

        assert_eq!(
            call_builtin_fn(BuiltinFn::LocalizedString, &[source.clone(), ru])
                .unwrap()
                .to_string(),
            "Первая\nВторая и 'цитата'"
        );
        assert_eq!(
            call_builtin_fn(BuiltinFn::LocalizedString, &[source, en])
                .unwrap()
                .to_string(),
            "Hello"
        );
        assert_eq!(
            call_builtin_fn(
                BuiltinFn::LocalizedString,
                &[BslValue::Str(BslString::from_str("не ресурсная строка"))]
            )
            .unwrap()
            .to_string(),
            ""
        );
    }

    #[test]
    fn blank_string_follows_the_measured_primitive_contract() {
        for value in [
            BslValue::Undefined,
            BslValue::Str(BslString::from_str("")),
            BslValue::Str(BslString::from_str(" \t\n\u{a0}")),
        ] {
            assert_eq!(
                call_builtin_fn(BuiltinFn::IsBlankString, &[value]).unwrap(),
                BslValue::Boolean(true)
            );
        }
        for value in [
            BslValue::Str(BslString::from_str(" а ")),
            BslValue::number_from_i64(0),
            BslValue::Boolean(false),
        ] {
            assert_eq!(
                call_builtin_fn(BuiltinFn::IsBlankString, &[value]).unwrap(),
                BslValue::Boolean(false)
            );
        }
    }

    /// Сторож входа `call_builtin_fn`: недостающий аргумент отвергается
    /// ошибкой образа, а не роняет процесс на `args[0]`. Путь VM защищён
    /// периметром (`check_call_geometry`), но функция публична.
    #[test]
    fn call_builtin_fn_rejects_too_few_arguments() {
        assert!(matches!(
            call_builtin_fn(BuiltinFn::Pow, &[]),
            Err(crate::RtError::InvalidBytecode(_))
        ));
    }

    /// То же для метода с ФИКСИРОВАННОЙ арностью (`static_arity() == Some`);
    /// полиморфные (`None`) проверяет сам обработчик в рантайме.
    #[test]
    fn call_builtin_method_rejects_too_few_arguments() {
        assert_eq!(BuiltinMethod::Delete.static_arity(), Some(1));
        assert!(matches!(
            call_builtin_method(BuiltinMethod::Delete, &crate::BslValue::Undefined, &[]),
            Err(crate::RtError::InvalidBytecode(_))
        ));
    }

    /// `BuiltinMethod` — словарь только базового рантайма. Вариант, который умеет ответить
    /// только «метод `bsl-*`», перечисляет мир из общего слоя и обязан жить в таблице своего
    /// компонента. Перебор арностей доходит до ветки метода и не даёт сторожу `static_arity`
    /// скрыть чужой вариант ранним `InvalidBytecode`.
    #[test]
    fn no_builtin_method_answers_for_a_foreign_package() {
        for (_, method) in BUILTIN_METHOD_NAMES {
            for count in 0..=4 {
                let arguments = vec![BslValue::Undefined; count];
                if let Err(RtError::MethodNotApplicable { method: answer, .. }) =
                    call_builtin_method(*method, &BslValue::Undefined, &arguments)
                {
                    assert!(
                        !answer.starts_with("метод bsl-"),
                        "{method:?} принадлежит чужому пакету: {answer}"
                    );
                }
            }
        }
    }

    /// Публичный вход файловых функций проверяет ФОРМУ аргументов сам:
    /// опереться на проверку арности в VM он не вправе — вызвать его
    /// может кто угодно, и короткий срез давал бы панику вместо ошибки,
    /// одинаково в debug и release.
    #[test]
    fn the_file_builtins_reject_a_wrong_argument_count_instead_of_panicking() {
        #[derive(Debug)]
        struct NoFiles;

        impl crate::FileSystem for NoFiles {
            fn read(&self, _path: &str) -> std::io::Result<Vec<u8>> {
                unreachable!("до файловой системы дело не доходит")
            }

            fn write(&self, _path: &str, _data: &[u8]) -> std::io::Result<()> {
                unreachable!("до файловой системы дело не доходит")
            }

            fn metadata(&self, _path: &str) -> std::io::Result<crate::FileMetadata> {
                unreachable!("до файловой системы дело не доходит")
            }

            fn read_dir<'fs>(
                &'fs self,
                _path: &str,
            ) -> std::io::Result<Box<dyn Iterator<Item = std::io::Result<crate::DirEntry>> + 'fs>>
            {
                unreachable!("до файловой системы дело не доходит")
            }

            fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
                unreachable!("до файловой системы дело не доходит")
            }

            fn open(
                &self,
                _path: &str,
                _options: crate::FileOpenOptions,
            ) -> std::io::Result<Box<dyn crate::FileHandle>> {
                unreachable!("до файловой системы дело не доходит")
            }
        }

        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let path = BslValue::Str(BslString::from_str("файл"));
        let bad: [(BuiltinFn, Vec<BslValue>); 5] = [
            (BuiltinFn::ValueFromFile, vec![]),
            (BuiltinFn::ValueFromFile, vec![path.clone(), path.clone()]),
            (BuiltinFn::ValueToFile, vec![]),
            (BuiltinFn::ValueToFile, vec![path.clone()]),
            (
                BuiltinFn::ValueToFile,
                vec![path.clone(), path.clone(), path.clone()],
            ),
        ];
        for (f, args) in bad {
            assert!(
                matches!(
                    call_builtin_files(f, &args, &mut rt, &NoFiles),
                    Err(RtError::InvalidBytecode(_))
                ),
                "{f:?} с {} аргументами обязана дать ошибку",
                args.len()
            );
        }

        // Чужая встроенная функция — тоже ошибка, а не молчание.
        assert!(matches!(
            call_builtin_files(BuiltinFn::Sqrt, &[path], &mut rt, &NoFiles),
            Err(RtError::InvalidBytecode(_))
        ));
    }

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

    /// Проверка арности стоит ДО быстрого пути структуры. ОТКРЫТЫЙ опкод
    /// `CallObjectMethod`
    /// выбирает метод по номеру имени уже в рантайме — статическая проверка
    /// на связывании (она закрывает только закрытый `CallMethod`) сюда не
    /// достаёт. До сторожа недостающий аргумент ронял процесс.
    #[test]
    fn a_structure_method_with_too_few_arguments_is_an_error_not_a_panic() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        for method in [BuiltinMethod::Insert, BuiltinMethod::Delete] {
            let error = call_builtin_method_ctx(method, &structure, &[], &mut rt)
                .expect_err("недостающий аргумент обязан быть ошибкой");
            assert!(matches!(error, RtError::InvalidBytecode(_)), "{error:?}");
        }
    }

    /// Полиморфная арность `Вставить` — один либо два аргумента; лишний
    /// обязан быть ошибкой, а не молча игнорироваться.
    /// `ЕстьАтрибут()`, и `ЕстьАтрибут("а","б","в")`), поэтому сторож ловит и
    /// ЛИШНИЙ аргумент. Быстрый путь структуры читает первые два и молча
    /// игнорировал третий: образ с завышенным `count` исполнялся успешно.
    #[test]
    fn a_structure_method_with_too_many_arguments_is_an_error_too() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        let extra = [
            BslValue::Str(crate::BslString::from_str("б")),
            BslValue::Number(bsl_number::BslNumber::from_i64(2)),
            BslValue::Number(bsl_number::BslNumber::from_i64(3)),
        ];
        let error = call_builtin_method_ctx(BuiltinMethod::Insert, &structure, &extra, &mut rt)
            .expect_err("лишний аргумент обязан быть ошибкой");
        assert!(matches!(error, RtError::InvalidBytecode(_)), "{error:?}");
        // Контроль: обе измеренные формы принимаются.
        call_builtin_method_ctx(BuiltinMethod::Insert, &structure, &extra[..1], &mut rt)
            .expect("значение по умолчанию обязано работать");
        call_builtin_method_ctx(BuiltinMethod::Insert, &structure, &extra[..2], &mut rt)
            .expect("верная арность обязана работать");
    }
}
