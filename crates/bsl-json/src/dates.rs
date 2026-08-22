//! `ЗаписатьДатуJSON` / `ПрочитатьДатуJSON`.

use bsl_rt::{BslValue, RtError, RtResult};

use bsl_rt::{local_date_from_utc_seconds, pseudo_unix_seconds};

use crate::write::*;

pub(crate) fn parse_json_date(text: &str) -> Option<bsl_rt::BslDate> {
    let body = text.strip_suffix('Z').unwrap_or(text);
    let (date, time) = match body.split_once('T') {
        Some((d, t)) => (d, t),
        None => (body, "00:00:00"),
    };
    let mut dp = date.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: u32 = dp.next()?.parse().ok()?;
    let day: u32 = dp.next()?.parse().ok()?;
    let mut tp = time.split(':');
    let hour: u32 = tp.next().unwrap_or("0").parse().ok()?;
    let minute: u32 = tp.next().unwrap_or("0").parse().ok()?;
    // Дробные секунды отбрасываются: у даты 1С разрешение — секунда.
    let sec_text = tp.next().unwrap_or("0");
    let second: u32 = sec_text.split('.').next()?.parse().ok()?;
    bsl_rt::BslDate::from_civil(year, month, day, hour, minute, second)
}

// --- ЗаписатьДатуJSON / ПрочитатьДатуJSON -------------------------------
//
// Единственное место в крейте, где дата интерпретируется относительно
// часового пояса МАШИНЫ (`bsl_rt::tz`), а не как наивный набор полей: сама
// суть `ВариантЗаписиДатыJSON` — «локальная», «локальная со смещением»,
// «универсальная» — это часовой пояс. `BslDate`, переданная сюда, читается
// как НАИВНОЕ ЛОКАЛЬНОЕ время машины: `ЗаписатьДатуJSON` с вариантом
// `УниверсальнаяДата` вычитает офсет машины, `ПрочитатьДатуJSON` с
// UTC-моментом (`Z`, JavaScript, Microsoft) его прибавляет — симметрично,
// так что `ПрочитатьДатуJSON(ЗаписатьДатуJSON(Д, ...), ...) = Д` на машине
// с одним и тем же смещением в оба конца.

pub(crate) fn unix_to_bsl_date(unix_seconds: i64, op: &'static str) -> RtResult<bsl_rt::BslDate> {
    unix_seconds
        .checked_add(bsl_rt::UNIX_EPOCH_SECONDS)
        .and_then(bsl_rt::BslDate::from_seconds)
        .ok_or(RtError::DateOutOfRange { op })
}

/// `+ЧЧ:ММ`/`-ЧЧ:ММ` — знак и величина смещения в секундах.
fn format_offset(offset_seconds: i32) -> String {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let abs = offset_seconds.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
}

/// Содержимое ЗНАЧЕНИЯ даты — без кавычек JSON, их ставит вызывающий
/// (`write_json_date` отдаёт эту строку как есть, `serialize` заворачивает
/// её как обычную строку через `JsonWriter::value`).
///
/// # Errors
///
/// ИЗМЕРЕНО (`JSON.WRITE_DATE.NON_ISO_LOCAL_ERROR`): платформа отвергает
/// сочетание не-ISO формата с не-универсальным вариантом записи — факт
/// исключения подтверждён, точный текст платформы снять не удалось (см.
/// `Anchor` в реестре), здесь [`RtError::Json`] с СОБСТВЕННЫМ текстом по
/// смыслу статьи. [`RtError::DateOutOfRange`], если перевод в UTC
/// вычитанием смещения машины ушёл за границы `0001-01-01..9999-12-31`
/// (пустые и предельные даты около границ диапазона).
pub(crate) fn format_json_date(
    date: bsl_rt::BslDate,
    format: JsonDateFormat,
    variant: JsonDateWritingVariant,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<String> {
    if format != JsonDateFormat::Iso && variant != JsonDateWritingVariant::Universal {
        return Err(RtError::Json(
            "формат даты, отличный от ISO, поддержан только для варианта записи \
             УниверсальнаяДата (JSONDateWritingVariant.UniversalDate)"
                .to_string(),
        ));
    }
    match variant {
        JsonDateWritingVariant::Local => {
            let c = date.to_civil();
            Ok(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                c.year, c.month, c.day, c.hour, c.minute, c.second
            ))
        }
        JsonDateWritingVariant::LocalOffset => {
            let c = date.to_civil();
            let offset = zone.offset_seconds(pseudo_unix_seconds(date));
            Ok(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
                c.year,
                c.month,
                c.day,
                c.hour,
                c.minute,
                c.second,
                format_offset(offset)
            ))
        }
        JsonDateWritingVariant::Universal => {
            let pseudo = pseudo_unix_seconds(date);
            let offset = zone.offset_seconds(pseudo);
            let utc_unix = pseudo - i64::from(offset);
            // ИЗМЕРЕНО: `ЗаписатьДатуJSON(Дата(1,1,1), ISO, УниверсальнаяДата)`
            // на платформе даёт `0001-01-01T00:00:00Z`, а не ошибку — хотя
            // вычитание смещения машины уводит псевдо-момент НИЖЕ пола
            // диапазона (`0001-01-01`) для положительных (восточных)
            // смещений. Платформа клампит результат к полу, а не падает;
            // тот же приём применён здесь. Поведение для дат ВБЛИЗИ пола,
            // но не равных ему, замером не подтверждено — фикстура несёт
            // пробу на этот случай.
            let floor = pseudo_unix_seconds(bsl_rt::BslDate::empty());
            let utc_unix = utc_unix.max(floor);
            match format {
                JsonDateFormat::Iso => {
                    let utc_date = unix_to_bsl_date(utc_unix, "ЗаписатьДатуJSON")?;
                    let c = utc_date.to_civil();
                    Ok(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                        c.year, c.month, c.day, c.hour, c.minute, c.second
                    ))
                }
                JsonDateFormat::JavaScript => Ok(format!("new Date({})", utc_unix * 1000)),
                // ИЗМЕРЕНО: платформа пишет момент БЕЗ обратных косых —
                // `/Date(мс)/`, а не `\/Date(мс)\/`, как ошибочно
                // предполагалось раньше (до замера).
                JsonDateFormat::Microsoft => Ok(format!("/Date({})/", utc_unix * 1000)),
            }
        }
    }
}

/// `ЗаписатьДатуJSON(Дата, Формат[, Вариант])` -> `Строка`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументах не тех типов; иначе см.
/// `format_json_date`.
pub fn write_json_date(
    date: &BslValue,
    format: &BslValue,
    variant: &BslValue,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    let BslValue::Date(d) = date else {
        return Err(RtError::TypeError {
            expected: "Дата",
            op: "ЗаписатьДатуJSON",
        });
    };
    let BslValue::Enum(format_enum) = format else {
        return Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ЗаписатьДатуJSON",
        });
    };
    let format = JsonDateFormat::from_enum_value(*format_enum).ok_or(RtError::TypeError {
        expected: "ФорматДатыJSON",
        op: "ЗаписатьДатуJSON",
    })?;
    let variant = match variant {
        BslValue::Undefined => JsonDateWritingVariant::default(),
        BslValue::Enum(e) => {
            JsonDateWritingVariant::from_enum_value(*e).ok_or(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ЗаписатьДатуJSON",
            })?
        }
        _ => {
            return Err(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ЗаписатьДатуJSON",
            });
        }
    };
    let text = format_json_date(*d, format, variant, zone)?;
    Ok(BslValue::Str(bsl_rt::BslString::from_str(&text)))
}

/// ИЗМЕРЕНО (`JSON.READ_DATE.BAD_FORMAT_TEXT`): факт исключения на
/// неразобравшемся представлении подтверждён, точный текст платформы снять
/// не удалось (см. `Anchor` в реестре — `КраткоеПредставлениеОшибки`
/// внутри `Вычислить` не видит контекст чужого исключения). Текст ниже —
/// «Представление даты имеет неверный формат» — СОБСТВЕННЫЙ, по смыслу
/// платформенных сообщений об ошибках разбора (`XXXИзСтроки`).
pub(crate) fn bad_date_representation() -> RtError {
    RtError::Json("Представление даты имеет неверный формат".to_string())
}

pub(crate) fn utc_millis_to_local_date(
    ms: i64,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<bsl_rt::BslDate> {
    local_date_from_utc_seconds(ms.div_euclid(1000), "ПрочитатьДатуJSON", zone)
}

/// `+ЧЧ:ММ`/`-ЧЧ:ММ` -> секунды. `None` — не разобралось.
fn parse_offset(s: &str) -> Option<i32> {
    let (h, m) = s.split_once(':')?;
    let h: i32 = h.parse().ok()?;
    let m: i32 = m.parse().ok()?;
    Some(h * 3600 + m * 60)
}

/// Хвост ISO-представления после времени суток: маркер зоны или явное
/// смещение (в секундах, уже со знаком).
enum IsoTail {
    Utc,
    Offset(i32),
}

/// Отделяет от ISO-строки суффикс зоны. Знак смещения ищется ПОСЛЕ `T`:
/// до неё дефисы принадлежат самой дате (`2024-03-04`), а время суток
/// `+`/`-` не содержит вовсе.
fn split_iso_tail(text: &str) -> (&str, Option<IsoTail>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(IsoTail::Utc));
    }
    if let Some(t_pos) = text.find('T') {
        let after_time = &text[t_pos + 1..];
        if let Some(rel) = after_time.find(['+', '-']) {
            let sign = after_time.as_bytes()[rel] as char;
            if let Some(magnitude) = parse_offset(&after_time[rel + 1..]) {
                let secs = if sign == '-' { -magnitude } else { magnitude };
                return (&text[..t_pos + 1 + rel], Some(IsoTail::Offset(secs)));
            }
        }
    }
    (text, None)
}

/// Разбор ISO-представления `ПрочитатьДатуJSON` во всех трёх вариантах
/// записи: без зоны (локальное время как есть), `Z` (UTC) и явное
/// смещение.
fn parse_iso_json_date(text: &str, zone: &dyn bsl_rt::TimeZone) -> Option<bsl_rt::BslDate> {
    let (body, tail) = split_iso_tail(text);
    let (date_part, time_part) = body.split_once('T')?;
    let mut dp = date_part.split('-');
    let year: i64 = dp.next()?.parse().ok()?;
    let month: u32 = dp.next()?.parse().ok()?;
    let day: u32 = dp.next()?.parse().ok()?;
    let mut tp = time_part.split(':');
    let hour: u32 = tp.next()?.parse().ok()?;
    let minute: u32 = tp.next()?.parse().ok()?;
    let second: u32 = tp.next()?.split('.').next()?.parse().ok()?;
    let wall = bsl_rt::BslDate::from_civil(year, month, day, hour, minute, second)?;
    match tail {
        // Без указания зоны — то, что и хранится: локальное время машины
        // как есть, без пересчёта (симметрично `Local` при записи).
        None => Some(wall),
        Some(IsoTail::Utc) => utc_millis_to_local_date(pseudo_unix_seconds(wall) * 1000, zone).ok(),
        Some(IsoTail::Offset(off_secs)) => {
            let utc_unix = pseudo_unix_seconds(wall) - i64::from(off_secs);
            utc_millis_to_local_date(utc_unix * 1000, zone).ok()
        }
    }
}

pub(crate) fn parse_json_date_by_format(
    text: &str,
    format: JsonDateFormat,
    zone: &dyn bsl_rt::TimeZone,
) -> Option<bsl_rt::BslDate> {
    match format {
        JsonDateFormat::Iso => parse_iso_json_date(text, zone),
        JsonDateFormat::JavaScript => {
            let inner = text.strip_prefix("new Date(")?.strip_suffix(')')?;
            let ms: i64 = inner.trim().parse().ok()?;
            utc_millis_to_local_date(ms, zone).ok()
        }
        // ИЗМЕРЕНО: платформа кидает исключение на написании с обратными
        // косыми (`\/Date(...)\/ `) — до замера здесь принимались оба
        // написания «на всякий случай», это оказалось лишним снисхождением.
        // Единственная принимаемая форма — `/Date(мс)/ `. Закрывающая
        // скобка идёт ПЕРЕД хвостовой чертой, поэтому в хвосте снимается
        // `)/ ` целиком, а не только сама черта.
        JsonDateFormat::Microsoft => {
            let inner = text.strip_prefix("/Date(")?.strip_suffix(")/")?;
            let ms: i64 = inner.trim().parse().ok()?;
            utc_millis_to_local_date(ms, zone).ok()
        }
    }
}

/// `ПрочитатьДатуJSON(Строка, Формат)` -> `Дата`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументах не тех типов;
/// [`RtError::Json`] (см. `bad_date_representation`) на строке, не
/// разобравшейся в заданном формате.
pub fn read_json_date(
    text: &BslValue,
    format: &BslValue,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    let BslValue::Str(s) = text else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПрочитатьДатуJSON",
        });
    };
    let BslValue::Enum(format_enum) = format else {
        return Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ПрочитатьДатуJSON",
        });
    };
    let format = JsonDateFormat::from_enum_value(*format_enum).ok_or(RtError::TypeError {
        expected: "ФорматДатыJSON",
        op: "ПрочитатьДатуJSON",
    })?;
    let text = s.to_string();
    let date =
        parse_json_date_by_format(&text, format, zone).ok_or_else(bad_date_representation)?;
    Ok(BslValue::Date(date))
}

/// Вызов функции модуля ПО ИМЕНИ — канал из исполняющей VM в рантайм.
///
/// `bsl-rt` не видит `bsl-vm` (зависимость идёт в обратную сторону), поэтому
/// колбэки `ПрочитатьJSON`/`ЗаписатьJSON` приходят сюда сверху замыканием
/// над `bsl_vm::call_module_function`. Аргументы уходят по значению, а
/// возвращается ПАРА: значение `Возврат` и финальные значения слотов
/// параметров — второй элемент нужен ровно затем, чтобы прочитать `Отказ`
/// функции преобразования (запись в параметр без `Знач` наблюдаема только
/// так, см. `call_module_function`).
pub type JsonCallByName<'a> =
    &'a mut dyn FnMut(&str, Vec<BslValue>) -> RtResult<(BslValue, Vec<BslValue>)>;

/// Функция преобразования `ЗаписатьJSON` (статья 16.2).
///
/// Сигнатура на платформе — РОВНО четыре параметра
/// `(Свойство, Значение, ДополнительныеПараметры, Отказ)`; другое их число —
/// ошибка вызова («В методе X количество параметров 2. Ожидаемое
/// количество - 4»), и у нас её даёт `call_module_function` своим текстом.
pub struct JsonConvertFn<'a> {
    /// `ИмяФункцииПреобразования` — четвёртый аргумент `ЗаписатьJSON`.
    pub name: String,
    /// `ДополнительныеПараметрыФункцииПреобразования` — шестой аргумент;
    /// уходит в функцию третьим параметром как есть.
    pub extra: BslValue,
    /// Как позвать функцию модуля по имени.
    pub call: JsonCallByName<'a>,
}

/// Функция восстановления `ПрочитатьJSON` (статья 16.2).
///
/// Сигнатура на платформе — РОВНО три параметра
/// `(Свойство, Значение, ДополнительныеПараметры)`.
pub struct JsonRestoreFn<'a> {
    /// `ИмяФункцииВосстановления` — пятый аргумент `ПрочитатьJSON`.
    pub name: String,
    /// `ДополнительныеПараметрыФункцииВосстановления` — седьмой аргумент.
    pub extra: BslValue,
    /// `ИменаСвойствДляФункцииВосстановления` — восьмой аргумент.
    ///
    /// ИЗМЕРЕНО: пустой список (как и отсутствующий аргумент) означает «для
    /// каждого значения документа», а непустой сужает вызовы до
    /// перечисленных имён свойств НА ЛЮБОЙ ГЛУБИНЕ и заодно отменяет вызов
    /// на корне документа. Сравнение регистрозависимое — см.
    /// `JsonRestoreFn::applies_to`.
    pub property_names: Vec<String>,
    /// Как позвать функцию модуля по имени.
    pub call: JsonCallByName<'a>,
}

#[cfg(test)]
mod tests {
    use bsl_rt::EnumValue;
    use bsl_rt::RuntimeShapes;

    use super::*;
    use crate::bridge::*;
    use crate::objects::settings_from;
    use bsl_rt::TimeZone as _;

    /// Контекст сериализации без функции преобразования.
    fn plain_serialize_ctx<'a>(
        settings: &'a JsonSerializerSettings,
        rt: &'a RuntimeShapes,
    ) -> SerializeCtx<'a, 'static> {
        SerializeCtx {
            settings,
            single_value_mode: false,
            convert: None,
            rt,
            zone: &MACHINE_ZONE,
        }
    }

    fn civil(y: i64, m: u32, d: u32, h: u32, mi: u32, s: u32) -> bsl_rt::BslDate {
        bsl_rt::BslDate::from_civil(y, m, d, h, mi, s).unwrap()
    }

    /// Зона МАШИНЫ: эти тесты сверяют вывод с её смещением, как и раньше,
    /// когда оно бралось из процессного кэша, — поэтому здесь именно она,
    /// а не фиксированная.
    static MACHINE_ZONE: MachineZone = MachineZone;

    struct MachineZone;

    impl bsl_rt::TimeZone for MachineZone {
        fn offset_seconds(&self, unix_seconds: i64) -> i32 {
            bsl_rt::SystemTimeZone::new().offset_seconds(unix_seconds)
        }
    }

    fn machine_zone() -> MachineZone {
        MachineZone
    }

    #[test]
    fn write_json_date_local_variant_is_iso_without_zone() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let s = format_json_date(
            d,
            JsonDateFormat::Iso,
            JsonDateWritingVariant::Local,
            &MACHINE_ZONE,
        )
        .unwrap();
        assert_eq!(s, "2014-05-10T13:14:15");
    }

    #[test]
    fn write_json_date_local_offset_variant_appends_the_machine_offset() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let s = format_json_date(
            d,
            JsonDateFormat::Iso,
            JsonDateWritingVariant::LocalOffset,
            &MACHINE_ZONE,
        )
        .unwrap();
        let offset = machine_zone().offset_seconds(pseudo_unix_seconds(d));
        assert_eq!(s, format!("2014-05-10T13:14:15{}", format_offset(offset)));
    }

    #[test]
    fn write_json_date_universal_variant_covers_all_three_formats() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        let pseudo = pseudo_unix_seconds(d);
        let offset = machine_zone().offset_seconds(pseudo);
        let utc_unix = pseudo - i64::from(offset);

        let iso = format_json_date(
            d,
            JsonDateFormat::Iso,
            JsonDateWritingVariant::Universal,
            &MACHINE_ZONE,
        )
        .expect("универсальный ISO обязан построиться");
        let utc_civil = unix_to_bsl_date(utc_unix, "test").unwrap().to_civil();
        assert_eq!(
            iso,
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                utc_civil.year,
                utc_civil.month,
                utc_civil.day,
                utc_civil.hour,
                utc_civil.minute,
                utc_civil.second
            )
        );

        let js = format_json_date(
            d,
            JsonDateFormat::JavaScript,
            JsonDateWritingVariant::Universal,
            &MACHINE_ZONE,
        )
        .unwrap();
        assert_eq!(js, format!("new Date({})", utc_unix * 1000));

        // ИЗМЕРЕНО: БЕЗ обратных косых — `/Date(мс)/`, не `\/Date(мс)\/`,
        // как ошибочно предполагалось до замера.
        let ms = format_json_date(
            d,
            JsonDateFormat::Microsoft,
            JsonDateWritingVariant::Universal,
            &MACHINE_ZONE,
        )
        .unwrap();
        assert_eq!(ms, format!("/Date({})/", utc_unix * 1000));
        assert!(!ms.contains('\\'), "обратных косых быть не должно: {ms}");
    }

    /// Документ, а не только `format_json_date`, обязан нести дату слово в
    /// слово (в кавычках) — раз в содержимом больше нет символов,
    /// требующих экранирования JSON, обычный `JsonWriter::value` ничего в
    /// нём не меняет.
    #[test]
    fn nested_microsoft_date_matches_the_standalone_content_in_the_document() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new());
        let id = rt.names.intern("д");
        let d = civil(2014, 5, 10, 13, 14, 15);
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        structure
            .structure_insert(id, BslValue::Date(d), &mut rt.shapes)
            .unwrap();

        let settings = JsonSerializerSettings {
            date_format: JsonDateFormat::Microsoft,
            date_variant: JsonDateWritingVariant::Universal,
            arrays_as_objects: false,
        };
        let content = format_json_date(
            d,
            settings.date_format,
            settings.date_variant,
            &MACHINE_ZONE,
        )
        .unwrap();

        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        serialize(
            &mut w,
            &structure,
            &mut plain_serialize_ctx(&settings, &rt),
            0,
        )
        .unwrap();
        let text = w.finish().unwrap();

        assert_eq!(text, format!("{{\n\"д\": \"{content}\"\n}}"));
        assert!(
            !text.contains('\\'),
            "обратных косых быть не должно: {text}"
        );
    }

    #[test]
    fn non_iso_format_requires_the_universal_variant() {
        // Замер JSON.WRITE_DATE.NON_ISO_LOCAL_ERROR: выбранное поведение —
        // ошибка, а не тихая подстановка ISO.
        let d = civil(2024, 1, 1, 0, 0, 0);
        assert!(
            format_json_date(
                d,
                JsonDateFormat::JavaScript,
                JsonDateWritingVariant::Local,
                &MACHINE_ZONE
            )
            .is_err()
        );
        assert!(
            format_json_date(
                d,
                JsonDateFormat::Microsoft,
                JsonDateWritingVariant::LocalOffset,
                &MACHINE_ZONE,
            )
            .is_err()
        );
        assert!(
            format_json_date(
                d,
                JsonDateFormat::JavaScript,
                JsonDateWritingVariant::Universal,
                &MACHINE_ZONE,
            )
            .is_ok()
        );
    }

    #[test]
    fn empty_date_formats_without_error_in_local_variants() {
        // Граница диапазона: пустая дата, оба варианта без пересчёта в UTC.
        let d = bsl_rt::BslDate::empty();
        assert_eq!(
            format_json_date(
                d,
                JsonDateFormat::Iso,
                JsonDateWritingVariant::Local,
                &MACHINE_ZONE
            )
            .unwrap(),
            "0001-01-01T00:00:00"
        );
        assert!(
            format_json_date(
                d,
                JsonDateFormat::Iso,
                JsonDateWritingVariant::LocalOffset,
                &MACHINE_ZONE
            )
            .is_ok()
        );
    }

    /// ИЗМЕРЕНО: `ЗаписатьДатуJSON(Дата(1,1,1), ISO, УниверсальнаяДата)` на
    /// платформе даёт `0001-01-01T00:00:00Z`, а не ошибку — вычитание
    /// смещения машины клампится к полу диапазона.
    #[test]
    fn universal_variant_of_the_empty_date_clamps_to_the_floor_instead_of_erroring() {
        let d = bsl_rt::BslDate::empty();
        let text = format_json_date(
            d,
            JsonDateFormat::Iso,
            JsonDateWritingVariant::Universal,
            &MACHINE_ZONE,
        )
        .expect("клампится, а не падает");
        assert!(
            text.starts_with("0001-01-01T") && text.ends_with('Z'),
            "{text}"
        );
        // На восточном (неотрицательном) смещении платформа измерена ТОЧНО
        // на полу; при отрицательном смещении вычитание и так не уходит за
        // пол, кламп там — no-op, и точное значение не измерено.
        let offset = machine_zone().offset_seconds(pseudo_unix_seconds(d));
        if offset >= 0 {
            assert_eq!(text, "0001-01-01T00:00:00Z");
        }
    }

    #[test]
    fn read_json_date_parses_iso_without_zone() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        assert_eq!(
            parse_json_date_by_format("2014-05-10T13:14:15", JsonDateFormat::Iso, &MACHINE_ZONE),
            Some(d)
        );
    }

    #[test]
    fn read_json_date_parses_iso_with_z_as_a_utc_moment() {
        // ИЗМЕРЕНО по примеру статьи: момент UTC переводится в локальное
        // время МАШИНЫ — то, что и делает `utc_millis_to_local_date`.
        let utc = civil(2014, 5, 10, 9, 14, 15);
        let expected =
            utc_millis_to_local_date(pseudo_unix_seconds(utc) * 1000, &MACHINE_ZONE).unwrap();
        assert_eq!(
            parse_json_date_by_format("2014-05-10T09:14:15Z", JsonDateFormat::Iso, &MACHINE_ZONE),
            Some(expected)
        );
    }

    #[test]
    fn read_json_date_parses_javascript_and_microsoft_examples_from_the_article() {
        // "new Date(1411464940000)" -> 23.09.2014 13:35:40 при UTC+4
        // (пример из статьи) — здесь проверяется не конкретная зона (она
        // зависит от машины), а то, что оба формата дают ОДИН И ТОТ ЖЕ
        // момент. Microsoft — БЕЗ обратных косых (см. `format_json_date`).
        let ms = 1_411_464_940_000i64;
        let expected = utc_millis_to_local_date(ms, &MACHINE_ZONE).unwrap();
        assert_eq!(
            parse_json_date_by_format(
                "new Date(1411464940000)",
                JsonDateFormat::JavaScript,
                &MACHINE_ZONE
            ),
            Some(expected)
        );
        assert_eq!(
            parse_json_date_by_format(
                "/Date(1411464940000)/",
                JsonDateFormat::Microsoft,
                &MACHINE_ZONE
            ),
            Some(expected)
        );
    }

    /// ИЗМЕРЕНО: платформа кидает исключение на написании с обратными
    /// косыми — до замера здесь снисходительно принимались оба написания,
    /// это оказалось лишним. Единственная принимаемая форма — без них.
    #[test]
    fn read_json_date_rejects_microsoft_format_with_backslashes() {
        assert_eq!(
            parse_json_date_by_format(
                "\\/Date(1411464940000)\\/",
                JsonDateFormat::Microsoft,
                &MACHINE_ZONE
            ),
            None
        );
    }

    #[test]
    fn read_json_date_round_trips_every_writing_variant() {
        let d = civil(2014, 5, 10, 13, 14, 15);
        for variant in [
            JsonDateWritingVariant::Local,
            JsonDateWritingVariant::LocalOffset,
            JsonDateWritingVariant::Universal,
        ] {
            let text = format_json_date(d, JsonDateFormat::Iso, variant, &MACHINE_ZONE).unwrap();
            let back = parse_json_date_by_format(&text, JsonDateFormat::Iso, &MACHINE_ZONE);
            assert_eq!(back, Some(d), "вариант {variant:?}, текст {text:?}");
        }
    }

    #[test]
    fn read_json_date_rejects_garbage_in_every_format() {
        assert_eq!(
            parse_json_date_by_format("совсем не дата", JsonDateFormat::Iso, &MACHINE_ZONE),
            None
        );
        assert_eq!(
            parse_json_date_by_format(
                "new Date(не число)",
                JsonDateFormat::JavaScript,
                &MACHINE_ZONE
            ),
            None
        );
        assert_eq!(
            parse_json_date_by_format("Date(123)", JsonDateFormat::Microsoft, &MACHINE_ZONE),
            None,
            "без ведущей `\\/` — не микрософтовский формат"
        );
    }

    /// Округление краёв: доли миллисекунды НИЖЕ секунды отбрасываются к
    /// РАНЕЕ идущей секунде (`div_euclid`, а не усечение к нулю) —
    /// симметрично и на положительной, и на отрицательной стороне эпохи.
    #[test]
    fn millisecond_fraction_rounds_toward_the_earlier_second_on_both_sides_of_the_epoch() {
        assert_eq!(
            parse_json_date_by_format("new Date(999)", JsonDateFormat::JavaScript, &MACHINE_ZONE),
            parse_json_date_by_format("new Date(0)", JsonDateFormat::JavaScript, &MACHINE_ZONE)
        );
        assert_eq!(
            parse_json_date_by_format("new Date(-1)", JsonDateFormat::JavaScript, &MACHINE_ZONE),
            parse_json_date_by_format("new Date(-1000)", JsonDateFormat::JavaScript, &MACHINE_ZONE)
        );
    }

    #[test]
    fn write_json_date_rejects_wrong_argument_types() {
        let format = BslValue::Enum(EnumValue::DateFormatIso);
        assert!(
            write_json_date(
                &BslValue::Undefined,
                &format,
                &BslValue::Undefined,
                &MACHINE_ZONE
            )
            .is_err()
        );
        let date = BslValue::Date(bsl_rt::BslDate::empty());
        assert!(
            write_json_date(
                &date,
                &BslValue::Undefined,
                &BslValue::Undefined,
                &MACHINE_ZONE
            )
            .is_err()
        );
    }

    #[test]
    fn read_json_date_reports_the_chosen_error_text() {
        // Замер JSON.READ_DATE.BAD_FORMAT_TEXT — фиксирует ВЫБРАННЫЙ текст.
        let text = BslValue::Str(bsl_rt::BslString::from_str("мусор"));
        let format = BslValue::Enum(EnumValue::DateFormatIso);
        let e = read_json_date(&text, &format, &MACHINE_ZONE).unwrap_err();
        assert_eq!(e.to_string(), "Представление даты имеет неверный формат");
    }
}
