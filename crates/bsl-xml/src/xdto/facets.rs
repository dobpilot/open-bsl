//! Проверка значения по фасетам простого типа.

use super::*;

// --- проверка по фасетам --------------------------------------------------

/// Фасеты ЦЕПОЧКИ базовых типов, начиная с `from`.
///
/// Нужен там, где цепочку не обходит сам разбор. Таких мест два. Первое —
/// встроенные типы: их наследование — это таблица [`BUILTIN_TYPES`], а не
/// разбор, `xs:int` разбирает лексическую форму сам и ни к какому предку
/// за ней не спускается. Собственные фасеты типа ловит и без обхода
/// замыкающий вызов [`check_facets`] в [`value_from_lexical_at`] —
/// границы диапазона лежат у самих `int` (-2147483648..2147483647) и
/// `byte` (-128..127), `minInclusive` — у самого `positiveInteger`,
/// «разряды дробной части» — у самого `integer`. Отсюда видны фасеты
/// ПРЕДКОВ, и без обхода пропали бы ровно они (измерено: `Создать` от
/// `unsignedByte` с «-1» — ошибка, хотя нижняя граница живёт у
/// `nonNegativeInteger` четырьмя уровнями выше, и от `int` с «1.5» —
/// ошибка по «разрядам дробной части» у `integer` двумя уровнями выше).
/// Второе — значения, у которых лексической формы здесь нет вовсе
/// (двоичные данные и расширенное имя, см. шапку модуля): их проверяет
/// [`coerce_to_value_type`], подставляя пустую лексическую форму, отчего
/// перечисление сравнивается ровно по значению.
pub(crate) fn check_facet_chain(
    model: &XdtoModel,
    from: Option<usize>,
    lexical: &str,
    value: &BslValue,
) -> RtResult<()> {
    let mut cur = from;
    // Длина модели — верхняя граница цепочки и заодно страховка от кольца
    // в испорченной модели.
    for _ in 0..=model.types.len() {
        let Some(index) = cur else {
            return Ok(());
        };
        check_facets(model, index, lexical, value)?;
        cur = model.type_at(index)?.base;
    }
    Ok(())
}

/// Проверка значения по СОБСТВЕННЫМ фасетам одного типа.
///
/// Перебор исчерпывающий: все двенадцать видов [`FacetKind`] названы явно,
/// потому что молчаливо пропущенный вид — это молчаливо принятое значение.
/// Два из них проверкой не являются: `whiteSpace` — сознательный no-op
/// (измерено: `Создать` от `xs:token` и от `xs:string` с « аб  вг »
/// отдаёт строку с теми же пробелами, платформа их не сворачивает), а
/// `pattern` — честный отказ, см. [`check_pattern`].
///
/// # Errors
///
/// [`RtError::Xdto`] на нарушенном фасете, на фасете образца и на фасете,
/// применённом к значению не того вида (длина у числа, границы у двоичных
/// данных): такая схема сама себе противоречит, и молчать об этом хуже,
/// чем отказать.
pub(crate) fn check_facets(
    model: &XdtoModel,
    index: usize,
    lexical: &str,
    value: &BslValue,
) -> RtResult<()> {
    let data = model.type_at(index)?;
    if data.facets.is_empty() {
        return Ok(());
    }
    // Перечисление — единственный вид, у которого несколько записей значат
    // ВЫБОР, а не несколько условий подряд, поэтому оно собирается целиком
    // и проверяется после перебора.
    let mut enumeration: Vec<&str> = Vec::new();
    for (kind, spec) in &data.facets {
        match kind {
            FacetKind::Enumeration => enumeration.push(spec),
            FacetKind::WhiteSpace => {}
            FacetKind::Pattern => check_pattern(data, spec)?,
            FacetKind::Length => check_length(data, value, spec, |len, want| len == want, "ровно")?,
            FacetKind::MinLength => {
                check_length(data, value, spec, |len, want| len >= want, "не меньше")?;
            }
            FacetKind::MaxLength => {
                check_length(data, value, spec, |len, want| len <= want, "не больше")?;
            }
            FacetKind::MinInclusive => {
                check_bound(
                    model,
                    index,
                    value,
                    spec,
                    |o| o != Ordering::Less,
                    "не меньше",
                )?;
            }
            FacetKind::MaxInclusive => {
                check_bound(
                    model,
                    index,
                    value,
                    spec,
                    |o| o != Ordering::Greater,
                    "не больше",
                )?;
            }
            FacetKind::MinExclusive => {
                check_bound(
                    model,
                    index,
                    value,
                    spec,
                    |o| o == Ordering::Greater,
                    "больше",
                )?;
            }
            FacetKind::MaxExclusive => {
                check_bound(model, index, value, spec, |o| o == Ordering::Less, "меньше")?;
            }
            FacetKind::TotalDigits => {
                check_digits(data, value, spec, "разрядов", |total, _| total)?;
            }
            FacetKind::FractionDigits => {
                check_digits(data, value, spec, "разрядов дробной части", |_, frac| frac)?;
            }
        }
    }
    if !enumeration.is_empty() {
        check_enumeration(model, index, lexical, value, &enumeration)?;
    }
    Ok(())
}

/// Фасет образца — ЧЕСТНЫЙ ОТКАЗ: движка регулярных выражений в дереве нет
/// (это отдельная задача), а частично истолкованный образец хуже, чем
/// названная вслух неподдержка.
///
/// Исключение ровно одно и вынужденное: образцы ВСТРОЕННЫХ типов XML
/// Schema. У `xs:integer` образец `[\-+]?[0-9]+`, и отказывать по нему
/// значило бы отвергать любую запись в `xs:int`, то есть почти всякую
/// схему. Лексическое пространство встроенных типов и без образца задаёт
/// [`builtin_from_lexical`] — кроме `xs:Name`, `xs:NCName` и
/// `xs:language`, которые отображаются в `Строка` и потому принимают
/// здесь то, что платформа отвергает (измерено: `Создать` от `Name` с
/// «1имя» и с «имя два», от `NCName` с «а:б» — ошибка). Это расхождение
/// названо в шапке модуля.
pub(crate) fn check_pattern(data: &XdtoTypeData, spec: &str) -> RtResult<()> {
    if matches!(data.shape, Some(ValueShape::Builtin(_))) {
        return Ok(());
    }
    Err(RtError::Xdto(format!(
        "фасет образца «{spec}» у типа «{}» не поддерживается: движка \
         регулярных выражений здесь нет",
        type_display(data)
    )))
}

/// Числовое значение фасета — длина или число разрядов.
pub(crate) fn facet_count(data: &XdtoTypeData, spec: &str) -> RtResult<usize> {
    spec.trim().parse::<usize>().map_err(|_| {
        RtError::Xdto(format!(
            "значение фасета «{spec}» у типа «{}» — не целое число",
            type_display(data)
        ))
    })
}

/// Длина значения: у строки — в символах (тем же счётом, что `СтрДлина`), у
/// двоичных данных — в БАЙТАХ, у списочного типа — в ЭЛЕМЕНТАХ (измерено:
/// `maxLength=2` у ограничения `xs:base64Binary` пропускает «0LA=» и
/// отвергает «0LDQsQ==», а у ограничения списка — отвергает третий
/// элемент).
pub(crate) fn value_length(data: &XdtoTypeData, value: &BslValue) -> RtResult<usize> {
    match value {
        BslValue::Str(s) => Ok(s.len_utf16()),
        BslValue::Object(o) => match &**o {
            BslObject::BinaryData(bytes) => Ok(bytes.len()),
            BslObject::Array(items) => Ok(items.borrow().len()),
            _ => Err(facet_not_applicable(data, value, "длины")),
        },
        _ => Err(facet_not_applicable(data, value, "длины")),
    }
}

pub(crate) fn check_length(
    data: &XdtoTypeData,
    value: &BslValue,
    spec: &str,
    ok: impl Fn(usize, usize) -> bool,
    what: &str,
) -> RtResult<()> {
    let want = facet_count(data, spec)?;
    let len = value_length(data, value)?;
    if ok(len, want) {
        return Ok(());
    }
    Err(RtError::Xdto(format!(
        "длина значения типа «{}» обязана быть {what} {want}, а она {len}",
        type_display(data)
    )))
}

/// Значение фасета, разобранное ТЕМ ЖЕ типом, что и проверяемое значение:
/// у `xs:decimal` граница — число, у ограничения `xs:date` — дата
/// (измерено: `minInclusive="2020-01-01"` отвергает «2019-01-01»).
/// Разбирается оно БЕЗ проверки фасетов — иначе граница проверяла бы сама
/// себя.
pub(crate) fn facet_operand(model: &XdtoModel, index: usize, spec: &str) -> RtResult<BslValue> {
    value_from_lexical(model, index, spec).map_err(|_| {
        let name = model
            .type_at(index)
            .map(type_display)
            .unwrap_or_else(|_| String::new());
        RtError::Xdto(format!(
            "значение фасета «{spec}» не разбирается типом «{name}»"
        ))
    })
}

pub(crate) fn check_bound(
    model: &XdtoModel,
    index: usize,
    value: &BslValue,
    spec: &str,
    ok: impl Fn(Ordering) -> bool,
    what: &str,
) -> RtResult<()> {
    let bound = facet_operand(model, index, spec)?;
    let data = model.type_at(index)?;
    let ordering = value
        .compare(&bound, "фасет границы XDTO")
        .map_err(|_| facet_not_applicable(data, value, "границы"))?;
    if ok(ordering) {
        return Ok(());
    }
    Err(RtError::Xdto(format!(
        "значение типа «{}» обязано быть {what} «{spec}»",
        type_display(data)
    )))
}

/// Разряды канонической записи числа: всего и в дробной части. Знак, точка
/// и ведущие нули не считаются, а хвостовые нули дробной части снимает сама
/// нормализация числа — отсюда и измеренное «1.200» у типа с
/// `fractionDigits="2"`, которое платформа принимает.
pub(crate) fn digit_counts(canonical: &str) -> (usize, usize) {
    let body = canonical.strip_prefix('-').unwrap_or(canonical);
    let (int_part, frac_part) = match body.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (body, ""),
    };
    let int_digits = int_part.trim_start_matches('0').len();
    (int_digits + frac_part.len(), frac_part.len())
}

pub(crate) fn check_digits(
    data: &XdtoTypeData,
    value: &BslValue,
    spec: &str,
    what: &str,
    pick: impl Fn(usize, usize) -> usize,
) -> RtResult<()> {
    let want = facet_count(data, spec)?;
    let BslValue::Number(number) = value else {
        return Err(facet_not_applicable(data, value, what));
    };
    let (total, frac) = digit_counts(&number.to_canonical());
    let got = pick(total, frac);
    if got <= want {
        return Ok(());
    }
    Err(RtError::Xdto(format!(
        "у значения типа «{}» {what} не больше {want}, а их {got}",
        type_display(data)
    )))
}

/// Перечисление: значение обязано совпасть с одним из перечисленных ПО
/// ЗНАЧЕНИЮ, а не по записи (измерено на перечислении десятичного типа:
/// при перечисленных «1.0» и «2» проходят и «1», и «2.00», а «3» — нет).
/// Совпадение самих записей остаётся запасным путём — для значений,
/// которые сравнивать по значению нечем (двоичные данные, расширенное имя).
pub(crate) fn check_enumeration(
    model: &XdtoModel,
    index: usize,
    lexical: &str,
    value: &BslValue,
    specs: &[&str],
) -> RtResult<()> {
    for spec in specs {
        if lexical == *spec {
            return Ok(());
        }
        if let Ok(allowed) = value_from_lexical(model, index, spec)
            && *value == allowed
        {
            return Ok(());
        }
    }
    Err(RtError::Xdto(format!(
        "«{lexical}» не входит в перечисление типа «{}»: {}",
        type_display(model.type_at(index)?),
        specs.join(", ")
    )))
}

/// Фасет, применённый к значению не того вида: длина у числа, границы у
/// двоичных данных, разряды у строки. Это противоречие внутри самой схемы,
/// и отказ называет его вслух.
pub(crate) fn facet_not_applicable(data: &XdtoTypeData, value: &BslValue, what: &str) -> RtError {
    RtError::Xdto(format!(
        "фасет {what} у типа «{}» не применим к значению типа «{}»",
        type_display(data),
        value.type_name()
    ))
}

pub(crate) fn bad_lexical(lexical: &str, what: &str) -> RtError {
    RtError::Xdto(format!("«{lexical}» — не лексическая форма {what}"))
}

/// Значение встроенного типа из лексической формы. Правила измерены
/// поимённо: у `xs:boolean` принимаются и слова, и цифры; у чисел —
/// ведущий плюс, хвостовые нули и показатель степени (`1.5E3` -> 1500);
/// пробелы по краям отбрасываются у всех.
pub(crate) fn builtin_from_lexical(
    bsl: BuiltinBsl,
    lexical: &str,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    match bsl {
        // Строка идёт как есть, БЕЗ обрезки: `xs:string` с одними
        // пробелами — это пробелы (фасет `whiteSpace` их не трогает, он
        // только описан).
        BuiltinBsl::Str => Ok(BslValue::Str(BslString::from_str(lexical))),
        // ПУСТАЯ лексическая форма ДЕСЯТИЧНОГО числа — это НОЛЬ, а не
        // ошибка (измерено: `Создать(xs:int, "")` даёт 0, и пустые элементы
        // `<num/>`, `<dec/>` при чтении с типом — тоже 0). На `xs:double`
        // поблажка НЕ распространяется: обе его пробы отвечают ошибкой
        // (измерено — `чв Создать пустой dbl` и `чв пустое dbl`), как у
        // даты, времени и булева, поэтому double уходит в общий разбор.
        BuiltinBsl::Number if lexical.trim().is_empty() => {
            Ok(BslValue::Number(BslNumber::from_i64(0)))
        }
        BuiltinBsl::Number => match BslNumber::parse_canonical(lexical.trim()) {
            Ok(n) => Ok(BslValue::Number(n)),
            Err(_) => Err(bad_lexical(lexical, "числа")),
        },
        BuiltinBsl::Double => parse_exponential(lexical.trim()),
        BuiltinBsl::Boolean => match lexical.trim() {
            "true" | "1" => Ok(BslValue::Boolean(true)),
            "false" | "0" => Ok(BslValue::Boolean(false)),
            _ => Err(bad_lexical(lexical, "«xs:boolean»")),
        },
        BuiltinBsl::Date => parse_xsd_date(lexical.trim(), zone),
        BuiltinBsl::DateTime => parse_xsd_date_time(lexical.trim(), zone),
        BuiltinBsl::Time => parse_xsd_time(lexical.trim(), zone),
        BuiltinBsl::Base64 => {
            let bytes =
                decode_base64(lexical).ok_or_else(|| bad_lexical(lexical, "«base64Binary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        BuiltinBsl::Hex => {
            let bytes =
                decode_hex(lexical.trim()).ok_or_else(|| bad_lexical(lexical, "«hexBinary»"))?;
            Ok(BslValue::Object(Rc::new(BslObject::BinaryData(Rc::from(
                bytes.into_boxed_slice(),
            )))))
        }
        // Префикс платформа не разрешает вовсе: `Создать` от `xs:string`
        // — ошибка, а `просто` даёт имя с ПУСТЫМ URI (измерено).
        BuiltinBsl::QName => {
            let text = lexical.trim();
            if text.contains(':') || text.is_empty() {
                return Err(bad_lexical(lexical, "«QName» без префикса"));
            }
            Ok(crate::xsd::new_expanded_name("", text))
        }
    }
}

/// Лексическая форма `xs:double`/`xs:float`: то же десятичное число, но с
/// необязательным показателем степени. Показатель поддерживают ровно эти
/// два типа — `Создать` от `xs:decimal` с «1.5E3» и от `xs:int` с «1E2»
/// платформа отвергает (измерено), поэтому у остальных числовых типов
/// разбор обычный.
///
/// `INF`, `-INF` и `NaN` платформа принимает (измерено на `INF`), а здесь
/// они отвергаются: `Число` в 1С — десятичное с конечной точностью, и
/// бесконечности в нём нет.
pub(crate) fn parse_exponential(text: &str) -> RtResult<BslValue> {
    let (mantissa, exponent) = match text.split_once(['E', 'e']) {
        Some((m, e)) => {
            let e: i64 = e
                .strip_prefix('+')
                .unwrap_or(e)
                .parse()
                .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
            (m, e)
        }
        None => (text, 0),
    };
    let mantissa = BslNumber::parse_canonical(mantissa).map_err(|_| bad_lexical(text, "числа"))?;
    if exponent == 0 {
        return Ok(BslValue::Number(mantissa));
    }
    // Десятичный сдвиг — это умножение или деление на степень десяти;
    // умножение точное, а деление идёт через ту же операцию, что и
    // обычное `/`, то есть с округлением до 27 знаков.
    let magnitude = u32::try_from(exponent.unsigned_abs())
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    let ten = BslNumber::from_i64(10);
    let mut power = BslNumber::from_i64(1);
    for _ in 0..magnitude {
        power = power
            .mul(&ten)
            .map_err(|_| bad_lexical(text, "числа с показателем степени"))?;
    }
    let scaled = if exponent > 0 {
        mantissa.mul(&power)
    } else {
        mantissa.div(&power)
    };
    scaled
        .map(BslValue::Number)
        .map_err(|_| bad_lexical(text, "числа с показателем степени"))
}

/// `xs:date`: `ГГГГ-ММ-ДД` с необязательным поясом. Пояс не отбрасывается,
/// а пересчитывается в местное время, поэтому `2026-08-12+02:00` на машине
/// с поясом +03:00 дало 12.08.2026 1:00:00 (измерено).
pub(crate) fn parse_xsd_date(text: &str, zone: &dyn bsl_rt::TimeZone) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 10);
    let mut parts = body.split('-');
    let year: i64 = parse_part(parts.next(), text, "даты")?;
    let month: u32 = parse_part(parts.next(), text, "даты")?;
    let day: u32 = parse_part(parts.next(), text, "даты")?;
    if parts.next().is_some() {
        return Err(bad_lexical(text, "даты"));
    }
    let wall = bsl_rt::BslDate::from_civil(year, month, day, 0, 0, 0)
        .ok_or_else(|| bad_lexical(text, "даты"))?;
    Ok(BslValue::Date(apply_zone(wall, tail, zone)?))
}

/// `xs:dateTime`: `ГГГГ-ММ-ДДTЧЧ:ММ:СС` с необязательным поясом.
pub(crate) fn parse_xsd_date_time(text: &str, zone: &dyn bsl_rt::TimeZone) -> RtResult<BslValue> {
    let t = text
        .find('T')
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    let (body, tail) = split_zone(text, t + 1);
    let (date_part, time_part) = body.split_at(t);
    let mut dp = date_part.split('-');
    let year: i64 = parse_part(dp.next(), text, "«dateTime»")?;
    let month: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let day: u32 = parse_part(dp.next(), text, "«dateTime»")?;
    let (hour, minute, second) = parse_clock(&time_part[1..], text)?;
    let wall = bsl_rt::BslDate::from_civil(year, month, day, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "«dateTime»"))?;
    Ok(BslValue::Date(apply_zone(wall, tail, zone)?))
}

/// `xs:time`: `ЧЧ:ММ:СС`. Дата у результата — 01.01.0001 (измерено).
pub(crate) fn parse_xsd_time(text: &str, zone: &dyn bsl_rt::TimeZone) -> RtResult<BslValue> {
    let (body, tail) = split_zone(text, 0);
    let (hour, minute, second) = parse_clock(body, text)?;
    let wall = bsl_rt::BslDate::from_civil(1, 1, 1, hour, minute, second)
        .ok_or_else(|| bad_lexical(text, "времени"))?;
    Ok(BslValue::Date(apply_zone(wall, tail, zone)?))
}

pub(crate) fn parse_part<T: std::str::FromStr>(
    part: Option<&str>,
    text: &str,
    what: &str,
) -> RtResult<T> {
    part.and_then(|p| p.parse().ok())
        .ok_or_else(|| bad_lexical(text, what))
}

pub(crate) fn parse_clock(text: &str, whole: &str) -> RtResult<(u32, u32, u32)> {
    let mut parts = text.split(':');
    let hour: u32 = parse_part(parts.next(), whole, "времени")?;
    let minute: u32 = parse_part(parts.next(), whole, "времени")?;
    // Доли секунды платформа принимает, но `Дата` их не хранит.
    let seconds = parts.next().unwrap_or("0");
    let second: u32 = parse_part(seconds.split('.').next(), whole, "времени")?;
    if parts.next().is_some() {
        return Err(bad_lexical(whole, "времени"));
    }
    Ok((hour, minute, second))
}

/// Хвост часового пояса, если он есть: `Z` либо `±ЧЧ:ММ`. Знак ищется
/// начиная с `from`, чтобы дефисы самой даты не попали в пояс.
///
/// `from` — БАЙТОВОЕ смещение, посчитанное по ожидаемой длине формы
/// (`parse_xsd_date` передаёт 10 — длину `ГГГГ-ММ-ДД`), а лексическая форма
/// приходит из схемы и может быть какой угодно: у `2026-08-1я` смещение 10
/// попадает ВНУТРЬ многобайтового символа. Поэтому срез берётся через
/// `get`: не граница символа — значит пояса тут нет, форма возвращается
/// целиком, и ошибку выдаёт вызывающий разбор (`bad_lexical`), а не паника
/// на пользовательских данных.
pub(crate) fn split_zone(text: &str, from: usize) -> (&str, Option<i32>) {
    if let Some(body) = text.strip_suffix('Z') {
        return (body, Some(0));
    }
    if let Some(tail) = text.get(from..)
        && let Some(rel) = tail.find(['+', '-'])
    {
        let at = from + rel;
        let sign = if text.as_bytes()[at] == b'-' { -1 } else { 1 };
        if let Some((h, m)) = text[at + 1..].split_once(':')
            && let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>())
        {
            return (&text[..at], Some(sign * (h * 3600 + m * 60)));
        }
    }
    (text, None)
}

/// Пояс пересчитывается в МЕСТНОЕ время машины, как это делает платформа
/// (измерено: `2026-08-12T18:41:17Z` дало 21:41:17 на машине с +03:00, а
/// `…+02:00` — 19:41:17). Без пояса запись остаётся как есть.
pub(crate) fn apply_zone(
    wall: bsl_rt::BslDate,
    written: Option<i32>,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<bsl_rt::BslDate> {
    match written {
        None => Ok(wall),
        Some(offset) => bsl_rt::local_date_from_utc_seconds(
            bsl_rt::pseudo_unix_seconds(wall) - i64::from(offset),
            "лексическая форма XDTO",
            zone,
        ),
    }
}

/// Разбор `hexBinary`: пары шестнадцатеричных цифр, регистр не важен.
pub(crate) fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

/// Разбор `base64Binary`. Пробельные символы внутри записи игнорируются —
/// так требует XML Schema и так ведёт себя платформа с многострочным
/// содержимым элемента.
pub(crate) fn decode_base64(text: &str) -> Option<Vec<u8>> {
    bsl_rt::encoding::decode_base64(text)
}
