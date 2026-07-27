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
    /// `Округл(x, ЧислоРазрядов, Режим)` — арность у самой функции в 1С
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
}

impl BuiltinFn {
    pub fn lookup(name: &str) -> Option<Self> {
        Some(match name.to_uppercase().as_str() {
            "SQRT" => BuiltinFn::Sqrt,
            "POW" => BuiltinFn::Pow,
            "LOG" => BuiltinFn::Ln,
            "LOG10" => BuiltinFn::Log10,
            "EXP" => BuiltinFn::Exp,
            "SIN" => BuiltinFn::Sin,
            "COS" => BuiltinFn::Cos,
            "TAN" => BuiltinFn::Tan,
            "ASIN" => BuiltinFn::Asin,
            "ACOS" => BuiltinFn::Acos,
            "ATAN" => BuiltinFn::Atan,
            "ROUND" | "ОКРУГЛ" => BuiltinFn::Round,
            "INT" | "ЦЕЛ" => BuiltinFn::Trunc,
            "MESSAGE" | "СООБЩИТЬ" => BuiltinFn::Message,
            "STRING" | "СТРОКА" => BuiltinFn::ToString,
            "FORMAT" | "ФОРМАТ" => BuiltinFn::Format,
            "NUMBER" | "ЧИСЛО" => BuiltinFn::ToNumber,
            "STRLEN" | "СТРДЛИНА" => BuiltinFn::StrLen,
            "LEFT" | "ЛЕВ" => BuiltinFn::Left,
            "RIGHT" | "ПРАВ" => BuiltinFn::Right,
            "MID" | "СРЕД" => BuiltinFn::Mid,
            "UPPER" | "ВРЕГ" => BuiltinFn::Upper,
            "LOWER" | "НРЕГ" => BuiltinFn::Lower,
            "TRIMALL" | "СОКРЛП" => BuiltinFn::TrimAll,
            "TRIML" | "СОКРЛ" => BuiltinFn::TrimLeft,
            "TRIMR" | "СОКРП" => BuiltinFn::TrimRight,
            "STRFIND" | "СТРНАЙТИ" => BuiltinFn::StrFind,
            "STRREPLACE" | "СТРЗАМЕНИТЬ" => BuiltinFn::StrReplace,
            "STRSPLIT" | "СТРРАЗДЕЛИТЬ" => BuiltinFn::StrSplit,
            "STRCONCAT" | "СТРСОЕДИНИТЬ" => BuiltinFn::StrConcat,
            "STRLINECOUNT" | "СТРЧИСЛОСТРОК" => BuiltinFn::StrLineCount,
            "STRGETLINE" | "СТРПОЛУЧИТЬСТРОКУ" => BuiltinFn::StrGetLine,
            "STRTEMPLATE" | "СТРШАБЛОН" => BuiltinFn::StrTemplate,
            "CHAR" | "СИМВОЛ" => BuiltinFn::Char,
            "CHARCODE" | "КОДСИМВОЛА" => BuiltinFn::CharCode,
            "VALUEISFILLED" | "ЗНАЧЕНИЕЗАПОЛНЕНО" => BuiltinFn::ValueIsFilled,
            "TYPEOF" | "ТИПЗНЧ" => BuiltinFn::TypeOf,
            "TYPE" | "ТИП" => BuiltinFn::TypeByName,

            "DATE" | "ДАТА" => BuiltinFn::MakeDate,
            "CURRENTDATE" | "ТЕКУЩАЯДАТА" => BuiltinFn::CurrentDate,
            "CURRENTUNIVERSALDATEINMILLISECONDS"
            | "ТЕКУЩАЯУНИВЕРСАЛЬНАЯДАТАВМИЛЛИСЕКУНДАХ" => {
                BuiltinFn::CurrentUniversalDateInMilliseconds
            }
            "YEAR" | "ГОД" => BuiltinFn::DatePartOf(DatePart::Year),
            "MONTH" | "МЕСЯЦ" => BuiltinFn::DatePartOf(DatePart::Month),
            "DAY" | "ДЕНЬ" => BuiltinFn::DatePartOf(DatePart::Day),
            "HOUR" | "ЧАС" => BuiltinFn::DatePartOf(DatePart::Hour),
            "MINUTE" | "МИНУТА" => BuiltinFn::DatePartOf(DatePart::Minute),
            "SECOND" | "СЕКУНДА" => BuiltinFn::DatePartOf(DatePart::Second),
            "WEEKDAY" | "ДЕНЬНЕДЕЛИ" => BuiltinFn::DatePartOf(DatePart::Weekday),
            "BEGOFDAY" | "НАЧАЛОДНЯ" => BuiltinFn::DateBoundaryOf(DateBoundary::StartOfDay),
            "ENDOFDAY" | "КОНЕЦДНЯ" => BuiltinFn::DateBoundaryOf(DateBoundary::EndOfDay),
            "BEGOFMONTH" | "НАЧАЛОМЕСЯЦА" => BuiltinFn::DateBoundaryOf(DateBoundary::StartOfMonth),
            "ENDOFMONTH" | "КОНЕЦМЕСЯЦА" => BuiltinFn::DateBoundaryOf(DateBoundary::EndOfMonth),
            "BEGOFYEAR" | "НАЧАЛОГОДА" => BuiltinFn::DateBoundaryOf(DateBoundary::StartOfYear),
            "ENDOFYEAR" | "КОНЕЦГОДА" => BuiltinFn::DateBoundaryOf(DateBoundary::EndOfYear),
            "BEGOFWEEK" | "НАЧАЛОНЕДЕЛИ" => BuiltinFn::DateBoundaryOf(DateBoundary::StartOfWeek),
            "ADDMONTH" | "ДОБАВИТЬМЕСЯЦ" => BuiltinFn::AddMonth,
            _ => return None,
        })
    }

    /// `(минимум, максимум)` аргументов. У большинства встроенных они
    /// совпадают — необязательные аргументы есть у меньшинства, и раньше
    /// их не было вовсе (`arity()` возвращала одно число).
    ///
    /// Недостающие до МАКСИМУМА позиции дополняются `Неопределено` на
    /// этапе резолвинга (`bsl-sema::resolver::resolve_call`), так что в
    /// рантайме `call_builtin_fn` всегда видит ровно `max` аргументов и
    /// сам решает, что значит `Неопределено` на этой позиции. Единственное
    /// исключение — `Округл`, у которого резолвер подставляет литеральные
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
            // минимум три; строковая форма `Дата("...")` это один
            // аргумент, поэтому нижняя граница всё-таки 1, а какая из двух
            // форм имелась в виду, решает тип первого аргумента в
            // `BslValue::make_date`.
            BuiltinFn::MakeDate => (1, 6),
            BuiltinFn::CurrentDate | BuiltinFn::CurrentUniversalDateInMilliseconds => (0, 0),
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
/// Дальше идут волнами — `Найти`/`НайтиСтроки`/`Сортировать`/`Итог`,
/// `Свернуть`/`Скопировать`/`Загрузить-ВыгрузитьКолонку`/`Сдвинуть`, как и
/// описано в брифе — сюда пока не входят.
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
    /// `Итог("Колонка")`.
    Total,
    Write,
    Close,
}

impl BuiltinMethod {
    pub fn lookup(name: &str) -> Option<Self> {
        Some(match name.to_uppercase().as_str() {
            "COUNT" | "КОЛИЧЕСТВО" => BuiltinMethod::Count,
            "ADD" | "ДОБАВИТЬ" => BuiltinMethod::Add,
            "DELETE" | "УДАЛИТЬ" => BuiltinMethod::Delete,
            "CLEAR" | "ОЧИСТИТЬ" => BuiltinMethod::Clear,
            "INSERT" | "ВСТАВИТЬ" => BuiltinMethod::Insert,
            "GET" | "ПОЛУЧИТЬ" => BuiltinMethod::Get,
            "PROPERTY" | "СВОЙСТВО" => BuiltinMethod::Property,
            "FIND" | "НАЙТИ" => BuiltinMethod::Find,
            "FINDROWS" | "НАЙТИСТРОКИ" => BuiltinMethod::FindRows,
            "SORT" | "СОРТИРОВАТЬ" => BuiltinMethod::Sort,
            "TOTAL" | "ИТОГ" => BuiltinMethod::Total,
            "WRITE" | "ЗАПИСАТЬ" => BuiltinMethod::Write,
            "CLOSE" | "ЗАКРЫТЬ" => BuiltinMethod::Close,
            _ => return None,
        })
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
    }
}

/// Арность `Count`/`Delete`/`Clear` не зависит от получателя и уже
/// проверена в `bsl-sema`; арность `Add` — зависит (0 для строки таблицы,
/// 1 для элемента массива/колонки), поэтому здесь просто читаем
/// `args.len()` и решаем сами, а не полагаемся на проверку выше по стеку.
pub fn call_builtin_method(m: BuiltinMethod, obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    match m {
        BuiltinMethod::Count => {
            let len = obj.collection_len()?;
            Ok(BslValue::Number(bsl_number::BslNumber::from_i64(len as i64)))
        }
        BuiltinMethod::Add => match args {
            [] => obj.table_add_row(),
            [v] => match obj.push_element(v.clone()) {
                Ok(()) => Ok(BslValue::Undefined),
                Err(crate::RtError::MethodNotApplicable { .. }) => {
                    obj.table_add_column(v)?;
                    Ok(BslValue::Undefined)
                }
                Err(e) => Err(e),
            },
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
            obj.table_sort(&args[0])?;
            Ok(BslValue::Undefined)
        }
        BuiltinMethod::Total => obj.table_total(&args[0]),
        BuiltinMethod::Write => obj.text_writer_write(&args[0]),
        BuiltinMethod::Close => obj.text_writer_close(),
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
    // `НайтиСтроки` читает ИМЕНА полей структуры поиска (они хранятся
    // `NameId`) и сопоставляет их с именами колонок — единственный метод
    // таблицы, которому нужен интернер.
    if m == BuiltinMethod::FindRows {
        let Some(criteria) = args.first() else {
            return Err(RtError::MethodNotApplicable {
                method: "НайтиСтроки",
                receiver: obj.type_name(),
            });
        };
        return obj.table_find_rows(criteria, &rt.names);
    }
    call_builtin_method(m, obj, args)
}
