//! Подмножество XPath над деревом DOM: `ВычислитьВыражениеXPath`,
//! `СоздатьВыражениеXPath`, `СоздатьРазыменовательПИ` и `РезультатXPath`.
//!
//! Второго дерева здесь нет: вычислитель ходит по тем же узлам
//! `DomNode`, что строит `ПостроительDOM`, и
//! возвращает наружу ИХ ЖЕ — тождество узлов сохраняется (измерено:
//! `Э.ДочерниеУзлы[1] = Р.ПолучитьСледующий()` — «Да», и для текста, и для
//! комментария, и для атрибута, и для самого документа).
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xpath.bsl`),
//! именами в том числе: ни одно написание не взято из справки, каждое
//! проверено перебором кандидатов.
//!
//! * метод вычисления живёт ТОЛЬКО у документа:
//!   `Док.ВычислитьВыражениеXPath(Выражение, Узел, Разыменователь)` —
//!   ровно три аргумента либо четыре (четвёртый — член
//!   `ТипРезультатаDOMXPath`; ЧИСЛО на его месте — ошибка, `Неопределено` —
//!   принимается и значит «любой»). У элемента того же метода НЕТ.
//!   Английское написание — `EvaluateXPathExpression` (`Evaluate`,
//!   `EvaluateExpression`, `XPathEvaluate`, `ВычислитьВыражение`,
//!   `ВычислитьXPath` платформа не знает);
//! * `Док.СоздатьВыражениеXPath(Выражение, Разыменователь)` /
//!   `CreateXPathExpression` — РОВНО два аргумента (одноаргументную форму
//!   платформа отвергает), даёт `ВыражениеXPath` с методом
//!   `Вычислить(Узел[, ТипРезультата])` / `Evaluate`. Негодный СИНТАКСИС
//!   ловится уже при создании, а неизвестный ПРЕФИКС — только при
//!   вычислении;
//! * разыменователь создаёт `Док.СоздатьРазыменовательПИ([Узел])` /
//!   `CreateNSResolver` — 0, 1 и даже 2 аргумента платформа принимает, а
//!   `СоздатьРазыменовательПространствИмен` НЕ знает; есть и конструктор
//!   `Новый РазыменовательПространствИменDOM(Узел)` / `DOMNamespaceResolver`
//!   (без аргумента — ошибка). У него один метод:
//!   `НайтиURIПространстваИмен(Префикс)` / `LookupNamespaceURI`
//!   (`НайтиПрефикс` и `LookupPrefix` платформа не знает);
//! * разыменователь СМОТРИТ ОБЛАСТЬ ВИДИМОСТИ своего узла: на документе
//!   `<к:а xmlns:к="urn:1"><к:б xmlns:к="urn:2">…` резолвер от корня даёт
//!   `urn:1`, а от `к:б` — `urn:2`. Резолвер от документа и БЕЗ аргумента
//!   равны резолверу от элемента документа (`urn:1`). От комментария и от
//!   инструкции обработки он поднимается вверх (`urn:к`), а вот ОТ ТЕКСТА —
//!   НЕТ: там `Неопределено`. Не-узел (число, строка) аргументом
//!   принимается и даёт резолвер БЕЗ области видимости;
//! * префикс `xmlns` возвращает пространство имён ПО УМОЛЧАНИЮ (`urn:по`),
//!   пустая строка и `xml` — `Неопределено`: предопределённых префиксов у
//!   платформы нет;
//! * перечисление называется `ТипРезультатаDOMXPath` / `DOMXPathResultType`
//!   и имеет десять членов; `ТипРезультатаXPathDOM` и `XPathResultType`
//!   платформа не знает. Тип результата — `РезультатXPath` / `XPathResult`
//!   («Результат DOM XPath»), тип выражения — `ВыражениеXPath` /
//!   `XPathExpression`;
//! * у результата ИЗМЕРЕНЫ члены `ТипРезультата`/`ResultType`,
//!   `ЧисловоеЗначение`/`NumberValue`, `СтроковоеЗначение`/`StringValue`,
//!   `БулевоЗначение`/`BooleanValue`, `РазмерСнимка`/`SnapshotLength`,
//!   `ЭлементСнимка(Н)`/`SnapshotItem`,
//!   `НеверноеСостояниеИтератора`/`InvalidIteratorState`,
//!   `ПолучитьСледующий()`/`IterateNext()` — и `SingleNodeValue`, у
//!   которого РУССКОГО написания нет: четырнадцать кандидатов
//!   (`ЕдинственныйУзел`, `ЗначениеЕдинственногоУзла`, `ОдиночныйУзел`,
//!   `УзелРезультата`, …) платформа отвергла;
//! * члены разграничены ПО ВИДУ результата: `ЧисловоеЗначение` у набора
//!   узлов — ошибка, `SnapshotLength` и `ЭлементСнимка` у итератора —
//!   ошибка, `SingleNodeValue` у итератора — ошибка. А вот
//!   `ПолучитьСледующий` есть у ВСЕХ видов: у числа, строки и булева он
//!   отдаёт `Неопределено` (не ошибку), у снимка и у «первого узла»
//!   работает наравне с итератором. `НеверноеСостояниеИтератора` читается
//!   у любого вида и всегда отвечал «Нет»;
//! * запрошенный вид НЕ ПРЕОБРАЗУЕТ значение: он объявляет вид результата
//!   и разрешает соответствующий член, а само значение не переносится.
//!   `count(//*)` с запросом `Строка` даёт вид «Строка» и ПУСТУЮ строку, а
//!   не «8»; `1=1` с запросом `Число` — 0, а не 1; `string(…)` с запросом
//!   `Число` — тоже 0. Исключение — набор узлов: числовой член берёт его
//!   РАЗМЕР (`//*[local-name()='цена']` числом — 2), булев — непустоту
//!   (набор — «Да», пустой набор — «Нет»), а строковый пуст и там. Запрос
//!   УЗЛОВОГО вида от числа или строки даёт ПУСТОЙ набор
//!   (`ЭлементСнимка(0)` — `Неопределено`, `SingleNodeValue` —
//!   `Неопределено`), а не ошибку;
//! * КОНТЕКСТНЫМ узлом для документа служит ЭЛЕМЕНТ ДОКУМЕНТА, а не сам
//!   документ: `name(.)` от `Док` даёт `к:каталог`, `count(/ | .)` — 2,
//!   `*` от документа и от корня дают одно и то же, а `child::к:каталог`
//!   от документа — пусто. Контекстом принимаются элемент, атрибут,
//!   комментарий, инструкция обработки И ТЕКСТ (от текстового узла
//!   `string(.)` даёт его текст, `name(..)` — имя родителя, а `name(.)` —
//!   пустую строку); узел чужого документа — ошибка, `Неопределено` —
//!   ошибка;
//! * `position()` и `last()` ВНЕ предиката — ошибка (внутри предиката
//!   работают);
//! * пространства имён — по-настоящему: имя без префикса подбирает только
//!   узлы БЕЗ пространства имён (`//цена` на документе с
//!   `xmlns="urn:по"` не находит ничего), префикс разрешается
//!   разыменователем и сопоставляется по URI (с резолвером от корня
//!   `//к:*` на документе с двумя связываниями `к` находит только узлы
//!   первого), неизвестный префикс — ошибка. Это НЕ то же, что у
//!   `ПолучитьЭлементыПоИмени`, который URI игнорирует;
//! * объявлений `xmlns` на оси `attribute::` НЕТ: у корня с `xmlns:к`,
//!   `xmlns` и `версия` выражение `/*/@*` отдаёт ровно один узел, а
//!   `/*/@xmlns:к` — ни одного (в дереве DOM они при этом обычные
//!   атрибуты — измерено там);
//! * ЧИСЛА считаются в двойной точности и печатаются пятнадцатью
//!   значащими цифрами С ЗАПЯТОЙ: `string(1 div 3)` — `0,333333333333333`
//!   (длина 17, `contains(…, '.')` — «Нет», `contains(…, ',')` — «Да»),
//!   `string(123456789012345678)` — `1,23456789012346e+17`. Деление на
//!   ноль даёт `Infinity`, `-Infinity` и `NaN` (эти три — по-английски и
//!   без запятой), `number('абв')` — `NaN`, `NaN = NaN` — «Нет».
//!   `number('3.5')` — 3,5, а `number('3,5')` — `NaN`: НА ВХОДЕ запятая не
//!   принимается. Литерал с экспонентой (`1e3`) платформа ПРИНИМАЕТ, с
//!   запятой (`0,5`) — нет;
//! * `round(2.5)` — 3, `round(-2.5)` — -2, `-7 mod 2` — -1, `-7 div 2` —
//!   -3,5, `sum(//нет)` — 0, `boolean(0)` и `boolean('')` — «Нет»;
//! * `concat` с ОДНИМ аргументом — ошибка, `count()` без аргументов —
//!   ошибка, `count(5)` — ошибка, `string-length('а','б')` — ошибка:
//!   арность функций платформа проверяет;
//! * секция CDATA и соседний текст — ОДИН текстовый узел
//!   (`count(//text())` на `<а>раз<![CDATA[два]]>три</а>` — 1), а целиком
//!   пробельный текст в дерево не попадает (`count(//text())` на
//!   `<а>\n  <б>х</б>\n</а>` — 1). Оба факта достаются от разбора, а не от
//!   XPath;
//! * набор узлов сравнивается с БУЛЕВЫМ целиком, а не поузлово:
//!   `//нет = false()` — «Да», `//нет = true()` — «Нет»,
//!   `//к:товар = true()` — «Да». С числом и строкой — наоборот,
//!   поузлово: `//цена != 'нет'` — «Да», `//цена = 10` — «Да»;
//! * `lang('ru')` на `<б/>` внутри `<а xml:lang="ru-RU">` — истина, как и
//!   `lang('RU')` и `lang('ru-RU')`, а `lang('en')` — ложь: значение
//!   берётся у ПРЕДКОВ, сравнивается без учёта регистра и принимает
//!   префикс до дефиса. На самом `<а>` `lang('ru')` — тоже истина, а на
//!   документе БЕЗ `xml:lang` — ложь. Сам `xml:lang` при этом — обычный
//!   атрибут и виден на оси `attribute::`: `count(//а/@*)` — 1;
//! * пустой разделитель две функции трактуют ПО-РАЗНОМУ:
//!   `substring-before('абвгд', '')` — пустая строка, а
//!   `substring-after('абвгд', '')` — `абвгд` целиком;
//! * итератор — СНИМОК: добавленный в дерево после начала обхода узел в
//!   него не попадает, `НеверноеСостояниеИтератора` остаётся «Нет», а
//!   свежее вычисление новый узел уже видит.
//!
//! # Поддержанное подмножество
//!
//! Оси: `child`, `descendant`, `parent`, `ancestor`, `self`,
//! `descendant-or-self`, `ancestor-or-self`, `following-sibling`,
//! `preceding-sibling`, `following`, `preceding`, `attribute` — и
//! сокращения `/`, `//`, `.`, `..`, `@`. Проверки узла: имя,
//! `префикс:имя`, `*`, `префикс:*`, `node()`, `text()`, `comment()`,
//! `processing-instruction()` и `processing-instruction('цель')`.
//! Предикаты — любые выражения, включая позиционные. Операторы: `|`,
//! `or`, `and`, `=`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-` (двуместный и
//! унарный), `*`, `div`, `mod`, скобки. Функции — двадцать семь, то есть
//! базовая библиотека XPath 1.0 целиком: `last`, `position`, `count`,
//! `id`, `local-name`,
//! `namespace-uri`, `name`, `string`, `concat`, `starts-with`, `contains`,
//! `substring-before`, `substring-after`, `substring`, `string-length`,
//! `normalize-space`, `translate`, `boolean`, `not`, `true`, `false`,
//! `lang`, `number`, `sum`, `floor`, `ceiling`, `round`.
//!
//! НЕ поддержано — и отвечает ошибкой разбора, а не молчаливым пустым
//! результатом: ось `namespace::`, переменные (`$х`), всё, чего в
//! XPath 1.0 нет вовсе (`if`/`then`/`else`, `for … return` — платформа их
//! тоже отвергает, измерено). Неизвестная функция, неизвестная ось и
//! неизвестный префикс — тоже ошибка.
//!
//! # Сознательные расхождения
//!
//! * **`РазмерСнимка`/`SnapshotLength` отдаёт НАСТОЯЩУЮ длину.** У
//!   платформы этот член сломан: на снимке из ДВУХ узлов он даёт 0, а на
//!   ПУСТОМ — 1, притом `ЭлементСнимка(1)` на том же снимке из двух узлов
//!   исправно отдаёт второй узел. Воспроизводить «длину», которая на самом
//!   деле похожа на признак пустоты, незачем: измеренные значения
//!   сохранены в `measure-xpath.platform.txt`, а здесь длина — это длина.
//! * **`ЧисловоеЗначение` у бесконечности и `NaN` — ошибка.** Платформа
//!   отдаёт там испорченное число: `Строка()` от него пусто, `Цел()` — 0,
//!   `> 1000000` — «Нет», `= 0` — «Нет», `Формат()` печатает мусорные
//!   байты, а в четырёх прогонах подряд одно и то же выражение
//!   (`number(//нет)`) напечаталось пустой строкой и тремя РАЗНЫМИ числами
//!   (`-0,374666640336174704`, `-0,374666640858774704` и лежащее в снимке
//!   `-0,374666640148574704`), — то есть значения там попросту нет. Наше
//!   `Число` — десятичное, бесконечности в нём нет, и выдумывать вместо
//!   неё ноль хуже, чем честно отказать: внутри выражения
//!   `string(1 div 0)` по-прежнему даёт `Infinity`.
//! * **Ось `namespace::` не поддержана.** У платформы она есть и отвечает
//!   на ней узлами, которых в дереве DOM нет вовсе: измерен ТОЛЬКО их
//!   счёт — `count(//к:товар/namespace::*)` даёт 6. Что эти узлы несут,
//!   не измерялось, а заводить в дереве узлы ради воспроизведения
//!   неизмеренного — это не то же, что измерить.
//! * **Документ без элемента документа — ошибка.** Платформу проба
//!   `Новый ДокументDOM` + вычисление вешает намертво (прогон снят по
//!   таймауту), так что её ответ неизвестен в принципе; здесь это
//!   ловимая ошибка.
//! * **Разыменователь по узлу чужого документа — работает.** Тождества
//!   документов здесь не проверяет ни `СоздатьРазыменовательПИ`, ни
//!   конструктор: резолвер создаётся и при вычислении даёт область
//!   видимости СВОЕГО узла — то есть связывания чужого документа, — так
//!   что префикс разрешается в ЧУЖОЙ URI и подбирает только узлы с этим
//!   URI: на документе, где тот же префикс связан иначе, — ни одного.
//!   Платформу и эта проба вешает намертво (тот же таймаут), так что её
//!   ответ неизвестен в принципе; обе пробы поимённо изгнаны из
//!   `measure-xpath.bsl`.
//! * **`id()` всегда пуст.** Атрибутов типа `ID` эта реализация не знает
//!   (внутреннего подмножества DTD не разбирает), ровно как и
//!   `ПолучитьЭлементПоИдентификатору`, который тоже всегда
//!   `Неопределено`. Платформа на `id('нет')` тоже отдала пустой набор.
//! * **Одинарный `/` перед нелатинским именем.** Платформа отвечает
//!   ошибкой разбора на `count(/к:каталог)`, `count(/нет)`, на голый путь
//!   `/к:каталог` и на `string(/а)`, хотя `count(/a)` с ЛАТИНСКИМ именем
//!   на том же месте отдаёт 0, а `count(//к:каталог)`,
//!   `count(/child::к:каталог)` и `count(/*)` отдают 1. Граница проходит
//!   по АЛФАВИТУ имени, а не по наличию префикса: после одинарного `/`
//!   отвергнуты и префиксное `к:каталог`, и беспрефиксные `нет` и `а`.
//!   Это дефект её разборщика, а не правило языка: здесь все эти написания
//!   разбираются одинаково. Обе стороны границы сняты пробами и лежат в
//!   `measure-xpath.platform.txt`.
//! * **Обратные оси нумеруются с ближайшего узла**
//!   (`//к:товар[2]/preceding::*[1]` — `цена` предыдущего товара, а не
//!   первый элемент документа). Здесь это не расхождение, а ИЗМЕРЕНО на
//!   четырёх пробах — оговорено потому, что то же самое велит XPath 1.0 и
//!   спутать источник легко.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::dom::{DomKind, DomNode};
use crate::object::BslObject;
use crate::{BslString, BslValue, EnumValue, RtError, RtResult};

fn bad(what: impl Into<String>) -> RtError {
    RtError::XPath(what.into())
}

fn str_value(s: &str) -> BslValue {
    BslValue::Str(BslString::from_str(s))
}

/// Предел вложенности выражения при разборе.
///
/// Разбор рекурсивен, и без предела текст вида `((((((…1…))))))` уносил бы
/// процесс переполнением стека вместо перехватываемой ошибки. Сто уровней
/// скобок — заведомо больше любого осмысленного выражения и заведомо
/// меньше того, что не держит стек debug-сборки.
const MAX_EXPR_DEPTH: usize = 100;

// --- Числа --------------------------------------------------------------

/// Число в строку так, как это делает XPath платформы: пятнадцать
/// значащих цифр, ЗАПЯТАЯ разделителем дробной части и английские
/// `Infinity`/`-Infinity`/`NaN`. Всё измерено (см. заголовок модуля);
/// правило перехода к экспоненте — то же, что у `%.15g`: показатель
/// меньше -4 или не меньше 15.
pub(crate) fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    // `{:.14e}` — пятнадцать значащих цифр в научной записи; из неё же
    // берётся показатель, по которому выбирается форма.
    let sci = format!("{x:.14e}");
    let (mantissa, exp) = match sci.split_once('e') {
        Some(pair) => pair,
        // Недостижимо: `{:e}` всегда печатает показатель.
        None => return sci.replace('.', ","),
    };
    let exp: i32 = exp.parse().unwrap_or(0);
    let text = if !(-4..15).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", strip_zeros(mantissa), sign, exp.abs())
    } else {
        let decimals = (14 - exp).max(0) as usize;
        strip_zeros(&format!("{x:.decimals$}"))
    };
    text.replace('.', ",")
}

/// Убрать хвостовые нули дробной части вместе с осиротевшей точкой.
fn strip_zeros(text: &str) -> String {
    if !text.contains('.') {
        return text.to_string();
    }
    let trimmed = text.trim_end_matches('0');
    trimmed.strip_suffix('.').unwrap_or(trimmed).to_string()
}

/// Строка в число по правилам XPath: пробелы по краям снимаются,
/// разделитель — ТОЧКА, всё остальное даёт `NaN` (измерено:
/// `number('  7  ')` — 7, `number('3,5')` — `NaN`).
fn string_to_number(text: &str) -> f64 {
    let t = text.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    let body = t.strip_prefix('-').unwrap_or(t);
    // Ручная проверка формы: `parse::<f64>` принял бы `inf`, `NaN` и
    // экспоненту, а лексическая форма числа в XPath — только цифры с
    // точкой.
    let ok = !body.is_empty()
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.chars().filter(|c| *c == '.').count() <= 1
        && body.chars().any(|c| c.is_ascii_digit());
    if !ok {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

// --- Лексика ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Slash,
    DoubleSlash,
    LBracket,
    RBracket,
    LParen,
    RParen,
    At,
    Comma,
    ColonColon,
    Dot,
    DotDot,
    Star,
    Pipe,
    Plus,
    Minus,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Div,
    Mod,
    /// Имя без префикса.
    Name(String),
    /// `префикс:имя`.
    Qualified(String, String),
    /// `префикс:*`.
    PrefixedStar(String),
    Literal(String),
    Number(f64),
}

impl Tok {
    /// Может ли за этим токеном идти ОПЕРАТОР, а не проверка имени.
    /// Правило XPath 1.0 §3.7: `*` считается умножением, а `and`, `or`,
    /// `div`, `mod` — операторами, если предыдущий токен есть и он не
    /// `@`, `::`, `(`, `[`, `,` и не оператор.
    fn allows_operator_after(&self) -> bool {
        !matches!(
            self,
            Tok::At
                | Tok::ColonColon
                | Tok::LParen
                | Tok::LBracket
                | Tok::Comma
                | Tok::Slash
                | Tok::DoubleSlash
                | Tok::Star
                | Tok::Pipe
                | Tok::Plus
                | Tok::Minus
                | Tok::Eq
                | Tok::Ne
                | Tok::Lt
                | Tok::Le
                | Tok::Gt
                | Tok::Ge
                | Tok::And
                | Tok::Or
                | Tok::Div
                | Tok::Mod
        )
    }
}

/// Разрешён ли символ в имени XML. Точный класс имён здесь не нужен:
/// разбор всё равно сверит имя с деревом, а вот кириллицу пропустить
/// обязан — документы в замерах именно такие.
fn is_name_char(c: char, first: bool) -> bool {
    if c.is_ascii_alphabetic() || c == '_' {
        return true;
    }
    if !c.is_ascii() && !c.is_whitespace() && !c.is_control() {
        return true;
    }
    !first && (c.is_ascii_digit() || c == '-' || c == '.')
}

fn lex(text: &str) -> RtResult<Vec<Tok>> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<Tok> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let two = |i: usize| -> Option<char> { chars.get(i + 1).copied() };
        let tok = match c {
            '/' if two(i) == Some('/') => {
                i += 2;
                Tok::DoubleSlash
            }
            '/' => {
                i += 1;
                Tok::Slash
            }
            '[' => {
                i += 1;
                Tok::LBracket
            }
            ']' => {
                i += 1;
                Tok::RBracket
            }
            '(' => {
                i += 1;
                Tok::LParen
            }
            ')' => {
                i += 1;
                Tok::RParen
            }
            '@' => {
                i += 1;
                Tok::At
            }
            ',' => {
                i += 1;
                Tok::Comma
            }
            '|' => {
                i += 1;
                Tok::Pipe
            }
            '+' => {
                i += 1;
                Tok::Plus
            }
            '-' => {
                i += 1;
                Tok::Minus
            }
            '=' => {
                i += 1;
                Tok::Eq
            }
            '!' if two(i) == Some('=') => {
                i += 2;
                Tok::Ne
            }
            '<' if two(i) == Some('=') => {
                i += 2;
                Tok::Le
            }
            '<' => {
                i += 1;
                Tok::Lt
            }
            '>' if two(i) == Some('=') => {
                i += 2;
                Tok::Ge
            }
            '>' => {
                i += 1;
                Tok::Gt
            }
            ':' if two(i) == Some(':') => {
                i += 2;
                Tok::ColonColon
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(bad("в выражении XPath не закрыта строковая константа"));
                }
                let body: String = chars[start..i].iter().collect();
                i += 1;
                Tok::Literal(body)
            }
            '.' if two(i).is_some_and(|c| c.is_ascii_digit()) => {
                let start = i;
                i += 1;
                lex_number(&chars, &mut i, start)?
            }
            '.' if two(i) == Some('.') => {
                i += 2;
                Tok::DotDot
            }
            '.' => {
                i += 1;
                Tok::Dot
            }
            '*' => {
                i += 1;
                Tok::Star
            }
            '$' => {
                return Err(bad(
                    "переменные (`$имя`) в выражениях XPath не поддерживаются",
                ))
            }
            c if c.is_ascii_digit() => {
                let start = i;
                lex_number(&chars, &mut i, start)?
            }
            c if is_name_char(c, true) => {
                let start = i;
                while i < chars.len() && is_name_char(chars[i], false) {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                // `префикс:имя` и `префикс:*`, но не `префикс::ось`.
                if chars.get(i) == Some(&':') && chars.get(i + 1) != Some(&':') {
                    i += 1;
                    if chars.get(i) == Some(&'*') {
                        i += 1;
                        Tok::PrefixedStar(name)
                    } else {
                        let ls = i;
                        while i < chars.len() && is_name_char(chars[i], i == ls) {
                            i += 1;
                        }
                        if i == ls {
                            return Err(bad(format!(
                                "в выражении XPath после «{name}:» нет локального имени"
                            )));
                        }
                        Tok::Qualified(name, chars[ls..i].iter().collect())
                    }
                } else {
                    Tok::Name(name)
                }
            }
            other => {
                return Err(bad(format!(
                    "в выражении XPath недопустимый символ «{other}»"
                )))
            }
        };
        // Правило разрешения неоднозначности из §3.7.
        let operator_position = out.last().is_some_and(Tok::allows_operator_after);
        let tok = if operator_position {
            match tok {
                Tok::Name(ref n) if n == "and" => Tok::And,
                Tok::Name(ref n) if n == "or" => Tok::Or,
                Tok::Name(ref n) if n == "div" => Tok::Div,
                Tok::Name(ref n) if n == "mod" => Tok::Mod,
                other => other,
            }
        } else {
            tok
        };
        out.push(tok);
    }
    Ok(out)
}

/// Числовой литерал: цифры с необязательной дробной частью и — ИЗМЕРЕНО,
/// что платформа их принимает, хотя в грамматике XPath 1.0 их нет, —
/// необязательной экспонентой (`1e3` даёт 1000).
fn lex_number(chars: &[char], i: &mut usize, start: usize) -> RtResult<Tok> {
    while *i < chars.len() && chars[*i].is_ascii_digit() {
        *i += 1;
    }
    if chars.get(*i) == Some(&'.') {
        *i += 1;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    if matches!(chars.get(*i), Some('e' | 'E')) {
        let save = *i;
        *i += 1;
        if matches!(chars.get(*i), Some('+' | '-')) {
            *i += 1;
        }
        if chars.get(*i).is_some_and(|c| c.is_ascii_digit()) {
            while *i < chars.len() && chars[*i].is_ascii_digit() {
                *i += 1;
            }
        } else {
            *i = save;
        }
    }
    let text: String = chars[start..*i].iter().collect();
    text.parse::<f64>()
        .map(Tok::Number)
        .map_err(|_| bad(format!("в выражении XPath негодное число «{text}»")))
}

// --- Дерево выражения -----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Child,
    Descendant,
    DescendantOrSelf,
    Parent,
    Ancestor,
    AncestorOrSelf,
    FollowingSibling,
    PrecedingSibling,
    Following,
    Preceding,
    Attribute,
    Selff,
}

impl Axis {
    fn parse(name: &str) -> Option<Axis> {
        Some(match name {
            "child" => Axis::Child,
            "descendant" => Axis::Descendant,
            "descendant-or-self" => Axis::DescendantOrSelf,
            "parent" => Axis::Parent,
            "ancestor" => Axis::Ancestor,
            "ancestor-or-self" => Axis::AncestorOrSelf,
            "following-sibling" => Axis::FollowingSibling,
            "preceding-sibling" => Axis::PrecedingSibling,
            "following" => Axis::Following,
            "preceding" => Axis::Preceding,
            "attribute" => Axis::Attribute,
            "self" => Axis::Selff,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
enum NodeTest {
    /// Имя: `None` в `local` — это `*`.
    Name {
        prefix: Option<String>,
        local: Option<String>,
    },
    Any,
    Text,
    Comment,
    ProcessingInstruction(Option<String>),
}

#[derive(Debug, Clone)]
struct Step {
    axis: Axis,
    test: NodeTest,
    predicates: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Func {
    Last,
    Position,
    Count,
    Id,
    LocalName,
    NamespaceUri,
    Name,
    String,
    Concat,
    StartsWith,
    Contains,
    SubstringBefore,
    SubstringAfter,
    Substring,
    StringLength,
    NormalizeSpace,
    Translate,
    Boolean,
    Not,
    True,
    False,
    Lang,
    Number,
    Sum,
    Floor,
    Ceiling,
    Round,
}

impl Func {
    fn parse(name: &str) -> Option<Func> {
        Some(match name {
            "last" => Func::Last,
            "position" => Func::Position,
            "count" => Func::Count,
            "id" => Func::Id,
            "local-name" => Func::LocalName,
            "namespace-uri" => Func::NamespaceUri,
            "name" => Func::Name,
            "string" => Func::String,
            "concat" => Func::Concat,
            "starts-with" => Func::StartsWith,
            "contains" => Func::Contains,
            "substring-before" => Func::SubstringBefore,
            "substring-after" => Func::SubstringAfter,
            "substring" => Func::Substring,
            "string-length" => Func::StringLength,
            "normalize-space" => Func::NormalizeSpace,
            "translate" => Func::Translate,
            "boolean" => Func::Boolean,
            "not" => Func::Not,
            "true" => Func::True,
            "false" => Func::False,
            "lang" => Func::Lang,
            "number" => Func::Number,
            "sum" => Func::Sum,
            "floor" => Func::Floor,
            "ceiling" => Func::Ceiling,
            "round" => Func::Round,
            _ => return None,
        })
    }

    /// Сколько аргументов принимает функция: `(минимум, максимум)`.
    /// Платформа арность проверяет (измерено: `count()`,
    /// `string-length('а','б')` и `concat('а')` — ошибки).
    fn arity(self) -> (usize, usize) {
        match self {
            Func::Last | Func::Position | Func::True | Func::False => (0, 0),
            Func::LocalName | Func::NamespaceUri | Func::Name => (0, 1),
            Func::String | Func::StringLength | Func::NormalizeSpace | Func::Number => (0, 1),
            Func::Count | Func::Id | Func::Sum | Func::Boolean | Func::Not | Func::Lang => (1, 1),
            Func::Floor | Func::Ceiling | Func::Round => (1, 1),
            Func::StartsWith | Func::Contains | Func::SubstringBefore | Func::SubstringAfter => {
                (2, 2)
            }
            Func::Substring => (2, 3),
            Func::Translate => (3, 3),
            Func::Concat => (2, usize::MAX),
        }
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Compare(CmpOp, Box<Expr>, Box<Expr>),
    Arith(ArithOp, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    Literal(String),
    Number(f64),
    Call(Func, Vec<Expr>),
    /// Путь: `absolute` — начинать от корня дерева, а не от контекста.
    Path {
        absolute: bool,
        steps: Vec<Step>,
    },
    /// Выражение в скобках с предикатами и продолжением пути:
    /// `(//а)[1]/@б` (измерено, что платформа такое принимает).
    Filter {
        base: Box<Expr>,
        predicates: Vec<Expr>,
        steps: Vec<Step>,
    },
}

// --- Разбор ---------------------------------------------------------------

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Tok, what: &str) -> RtResult<()> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(bad(format!("в выражении XPath ожидалось {what}")))
        }
    }

    fn deeper(&mut self) -> RtResult<()> {
        self.depth += 1;
        if self.depth > MAX_EXPR_DEPTH {
            return Err(bad("выражение XPath вложено слишком глубоко"));
        }
        Ok(())
    }

    fn expr(&mut self) -> RtResult<Expr> {
        self.deeper()?;
        let e = self.or_expr();
        self.depth -= 1;
        e
    }

    fn or_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.and_expr()?;
        while self.eat(&Tok::Or) {
            let right = self.and_expr()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.equality_expr()?;
        while self.eat(&Tok::And) {
            let right = self.equality_expr()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn equality_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.relational_expr()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Eq) => CmpOp::Eq,
                Some(Tok::Ne) => CmpOp::Ne,
                _ => break,
            };
            self.pos += 1;
            let right = self.relational_expr()?;
            left = Expr::Compare(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn relational_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.additive_expr()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Lt) => CmpOp::Lt,
                Some(Tok::Le) => CmpOp::Le,
                Some(Tok::Gt) => CmpOp::Gt,
                Some(Tok::Ge) => CmpOp::Ge,
                _ => break,
            };
            self.pos += 1;
            let right = self.additive_expr()?;
            left = Expr::Compare(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn additive_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.multiplicative_expr()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Plus) => ArithOp::Add,
                Some(Tok::Minus) => ArithOp::Sub,
                _ => break,
            };
            self.pos += 1;
            let right = self.multiplicative_expr()?;
            left = Expr::Arith(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn multiplicative_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.unary_expr()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Star) => ArithOp::Mul,
                Some(Tok::Div) => ArithOp::Div,
                Some(Tok::Mod) => ArithOp::Mod,
                _ => break,
            };
            self.pos += 1;
            let right = self.unary_expr()?;
            left = Expr::Arith(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary_expr(&mut self) -> RtResult<Expr> {
        if self.eat(&Tok::Minus) {
            self.deeper()?;
            let inner = self.unary_expr();
            self.depth -= 1;
            return Ok(Expr::Neg(Box::new(inner?)));
        }
        self.union_expr()
    }

    fn union_expr(&mut self) -> RtResult<Expr> {
        let mut left = self.path_expr()?;
        while self.eat(&Tok::Pipe) {
            let right = self.path_expr()?;
            left = Expr::Union(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Путь либо выражение-фильтр с продолжением.
    fn path_expr(&mut self) -> RtResult<Expr> {
        if self.starts_primary() {
            self.deeper()?;
            let base = self.primary_expr();
            self.depth -= 1;
            let base = base?;
            let mut predicates = Vec::new();
            while self.peek() == Some(&Tok::LBracket) {
                predicates.push(self.predicate()?);
            }
            let mut steps = Vec::new();
            if self.eat(&Tok::DoubleSlash) {
                steps.push(descendant_or_self_step());
                steps.extend(self.relative_path()?);
            } else if self.eat(&Tok::Slash) {
                steps.extend(self.relative_path()?);
            }
            // Без предикатов и без продолжения пути это просто значение:
            // заворачивать его в фильтр нельзя, иначе `count(//*)`
            // потребовал бы набор узлов от числа.
            if predicates.is_empty() && steps.is_empty() {
                return Ok(base);
            }
            return Ok(Expr::Filter {
                base: Box::new(base),
                predicates,
                steps,
            });
        }
        self.location_path()
    }

    /// Начинается ли здесь первичное выражение (а не путь). Имя со
    /// скобкой — вызов функции, но `text()` и соседи по этому правилу не
    /// проходят: это проверки узла в шаге пути.
    fn starts_primary(&self) -> bool {
        match self.peek() {
            Some(Tok::LParen | Tok::Literal(_) | Tok::Number(_)) => true,
            Some(Tok::Name(n)) => {
                self.toks.get(self.pos + 1) == Some(&Tok::LParen) && !is_node_type(n)
            }
            _ => false,
        }
    }

    fn primary_expr(&mut self) -> RtResult<Expr> {
        match self.next() {
            Some(Tok::LParen) => {
                let inner = self.expr()?;
                self.expect(&Tok::RParen, "закрывающая скобка")?;
                Ok(inner)
            }
            Some(Tok::Literal(s)) => Ok(Expr::Literal(s)),
            Some(Tok::Number(x)) => Ok(Expr::Number(x)),
            Some(Tok::Name(name)) => {
                self.expect(&Tok::LParen, "открывающая скобка вызова функции")?;
                let func = Func::parse(&name)
                    .ok_or_else(|| bad(format!("функция XPath «{name}» не поддерживается")))?;
                let mut args = Vec::new();
                if !self.eat(&Tok::RParen) {
                    loop {
                        args.push(self.expr()?);
                        if self.eat(&Tok::Comma) {
                            continue;
                        }
                        self.expect(&Tok::RParen, "закрывающая скобка вызова функции")?;
                        break;
                    }
                }
                let (min, max) = func.arity();
                if args.len() < min || args.len() > max {
                    return Err(bad(format!(
                        "функция XPath «{name}» вызвана с {} аргументами",
                        args.len()
                    )));
                }
                Ok(Expr::Call(func, args))
            }
            _ => Err(bad("в выражении XPath ожидалось значение")),
        }
    }

    fn location_path(&mut self) -> RtResult<Expr> {
        if self.eat(&Tok::DoubleSlash) {
            let mut steps = vec![descendant_or_self_step()];
            steps.extend(self.relative_path()?);
            return Ok(Expr::Path {
                absolute: true,
                steps,
            });
        }
        if self.eat(&Tok::Slash) {
            // Одинокий `/` — это сам корневой узел.
            let steps = if self.starts_step() {
                self.relative_path()?
            } else {
                Vec::new()
            };
            return Ok(Expr::Path {
                absolute: true,
                steps,
            });
        }
        let steps = self.relative_path()?;
        Ok(Expr::Path {
            absolute: false,
            steps,
        })
    }

    fn starts_step(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Tok::At
                    | Tok::Dot
                    | Tok::DotDot
                    | Tok::Star
                    | Tok::Name(_)
                    | Tok::Qualified(..)
                    | Tok::PrefixedStar(_)
            )
        )
    }

    fn relative_path(&mut self) -> RtResult<Vec<Step>> {
        let mut steps = vec![self.step()?];
        loop {
            if self.eat(&Tok::DoubleSlash) {
                steps.push(descendant_or_self_step());
                steps.push(self.step()?);
            } else if self.eat(&Tok::Slash) {
                steps.push(self.step()?);
            } else {
                break;
            }
        }
        Ok(steps)
    }

    fn step(&mut self) -> RtResult<Step> {
        if self.eat(&Tok::Dot) {
            return Ok(Step {
                axis: Axis::Selff,
                test: NodeTest::Any,
                predicates: self.predicates()?,
            });
        }
        if self.eat(&Tok::DotDot) {
            return Ok(Step {
                axis: Axis::Parent,
                test: NodeTest::Any,
                predicates: self.predicates()?,
            });
        }
        let axis = if self.eat(&Tok::At) {
            Axis::Attribute
        } else if let Some(Tok::Name(name)) = self.peek().cloned() {
            if self.toks.get(self.pos + 1) == Some(&Tok::ColonColon) {
                self.pos += 2;
                if name == "namespace" {
                    return Err(bad(
                        "ось `namespace::` в выражениях XPath не поддерживается",
                    ));
                }
                Axis::parse(&name)
                    .ok_or_else(|| bad(format!("ось XPath «{name}» не существует")))?
            } else {
                Axis::Child
            }
        } else {
            Axis::Child
        };
        let test = self.node_test()?;
        Ok(Step {
            axis,
            test,
            predicates: self.predicates()?,
        })
    }

    fn node_test(&mut self) -> RtResult<NodeTest> {
        match self.next() {
            Some(Tok::Star) => Ok(NodeTest::Name {
                prefix: None,
                local: None,
            }),
            Some(Tok::PrefixedStar(prefix)) => Ok(NodeTest::Name {
                prefix: Some(prefix),
                local: None,
            }),
            Some(Tok::Qualified(prefix, local)) => Ok(NodeTest::Name {
                prefix: Some(prefix),
                local: Some(local),
            }),
            Some(Tok::Name(name)) => {
                if self.peek() != Some(&Tok::LParen) {
                    return Ok(NodeTest::Name {
                        prefix: None,
                        local: Some(name),
                    });
                }
                self.pos += 1;
                let test = match name.as_str() {
                    "node" => NodeTest::Any,
                    "text" => NodeTest::Text,
                    "comment" => NodeTest::Comment,
                    "processing-instruction" => {
                        if let Some(Tok::Literal(target)) = self.peek().cloned() {
                            self.pos += 1;
                            NodeTest::ProcessingInstruction(Some(target))
                        } else {
                            NodeTest::ProcessingInstruction(None)
                        }
                    }
                    other => {
                        return Err(bad(format!(
                            "в шаге пути XPath ожидалась проверка узла, а не «{other}(…)»"
                        )))
                    }
                };
                self.expect(&Tok::RParen, "закрывающая скобка проверки узла")?;
                Ok(test)
            }
            _ => Err(bad("в выражении XPath ожидался шаг пути")),
        }
    }

    fn predicates(&mut self) -> RtResult<Vec<Expr>> {
        let mut out = Vec::new();
        while self.peek() == Some(&Tok::LBracket) {
            out.push(self.predicate()?);
        }
        Ok(out)
    }

    fn predicate(&mut self) -> RtResult<Expr> {
        self.expect(&Tok::LBracket, "открывающая квадратная скобка")?;
        let e = self.expr()?;
        self.expect(&Tok::RBracket, "закрывающая квадратная скобка предиката")?;
        Ok(e)
    }
}

fn is_node_type(name: &str) -> bool {
    matches!(name, "node" | "text" | "comment" | "processing-instruction")
}

fn descendant_or_self_step() -> Step {
    Step {
        axis: Axis::DescendantOrSelf,
        test: NodeTest::Any,
        predicates: Vec::new(),
    }
}

fn parse(text: &str) -> RtResult<Expr> {
    let toks = lex(text)?;
    if toks.is_empty() {
        return Err(bad("пустое выражение XPath"));
    }
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
    };
    let e = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(bad("в выражении XPath остался неразобранный текст"));
    }
    Ok(e)
}

// --- Значения вычислителя --------------------------------------------------

#[derive(Debug, Clone)]
enum XValue {
    Nodes(Vec<Rc<DomNode>>),
    Number(f64),
    Str(String),
    Bool(bool),
}

impl XValue {
    fn to_bool(&self) -> bool {
        match self {
            XValue::Nodes(nodes) => !nodes.is_empty(),
            XValue::Number(x) => *x != 0.0 && !x.is_nan(),
            XValue::Str(s) => !s.is_empty(),
            XValue::Bool(b) => *b,
        }
    }

    fn to_str(&self) -> String {
        match self {
            XValue::Nodes(nodes) => nodes
                .first()
                .map(|n| n.xp_string_value())
                .unwrap_or_default(),
            XValue::Number(x) => number_to_string(*x),
            XValue::Str(s) => s.clone(),
            XValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        }
    }

    fn to_number(&self) -> f64 {
        match self {
            XValue::Nodes(_) | XValue::Str(_) => string_to_number(&self.to_str()),
            XValue::Number(x) => *x,
            XValue::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

// --- Вычисление -------------------------------------------------------------

/// Контекст шага: узел плюс позиция и размер, если они определены.
///
/// `position` и `size` — `None` НА ВЕРХНЕМ УРОВНЕ: измерено, что
/// `position()` и `last()` вне предиката платформа считает ошибкой, а не
/// единицей.
struct Ctx {
    node: Rc<DomNode>,
    position: Option<usize>,
    size: Option<usize>,
}

struct Evaluator<'a> {
    root: Rc<DomNode>,
    resolver: &'a XPathResolver,
    /// Номер узла в документном порядке; ключ — адрес узла.
    order: HashMap<usize, usize>,
    /// Все узлы дерева в документном порядке, ВКЛЮЧАЯ атрибуты. Считается
    /// один раз на вычисление: осям `following` и `preceding` нужен весь
    /// документ, и без общего списка каждый контекстный узел обходил бы
    /// дерево заново.
    document: Vec<Rc<DomNode>>,
}

fn key_of(node: &Rc<DomNode>) -> usize {
    Rc::as_ptr(node) as usize
}

fn root_of(node: &Rc<DomNode>) -> Rc<DomNode> {
    let mut cur = node.clone();
    while let Some(parent) = cur.xs_parent() {
        cur = parent;
    }
    cur
}

impl<'a> Evaluator<'a> {
    fn new(root: Rc<DomNode>, resolver: &'a XPathResolver) -> Evaluator<'a> {
        let mut order = HashMap::new();
        let mut document = Vec::new();
        number_document(&root, &mut order, &mut document);
        Evaluator {
            root,
            resolver,
            order,
            document,
        }
    }

    fn order_of(&self, node: &Rc<DomNode>) -> usize {
        self.order.get(&key_of(node)).copied().unwrap_or(usize::MAX)
    }

    fn sort_unique(&self, mut nodes: Vec<Rc<DomNode>>) -> Vec<Rc<DomNode>> {
        nodes.sort_by_key(|n| self.order_of(n));
        nodes.dedup_by(|a, b| Rc::ptr_eq(a, b));
        nodes
    }

    fn eval(&self, expr: &Expr, ctx: &Ctx) -> RtResult<XValue> {
        match expr {
            Expr::Or(a, b) => {
                if self.eval(a, ctx)?.to_bool() {
                    return Ok(XValue::Bool(true));
                }
                Ok(XValue::Bool(self.eval(b, ctx)?.to_bool()))
            }
            Expr::And(a, b) => {
                if !self.eval(a, ctx)?.to_bool() {
                    return Ok(XValue::Bool(false));
                }
                Ok(XValue::Bool(self.eval(b, ctx)?.to_bool()))
            }
            Expr::Compare(op, a, b) => {
                let left = self.eval(a, ctx)?;
                let right = self.eval(b, ctx)?;
                Ok(XValue::Bool(compare(*op, &left, &right)))
            }
            Expr::Arith(op, a, b) => {
                let x = self.eval(a, ctx)?.to_number();
                let y = self.eval(b, ctx)?.to_number();
                Ok(XValue::Number(match op {
                    ArithOp::Add => x + y,
                    ArithOp::Sub => x - y,
                    ArithOp::Mul => x * y,
                    ArithOp::Div => x / y,
                    ArithOp::Mod => x % y,
                }))
            }
            Expr::Neg(a) => Ok(XValue::Number(-self.eval(a, ctx)?.to_number())),
            Expr::Union(a, b) => {
                let left = self.nodes_of(self.eval(a, ctx)?, "|")?;
                let right = self.nodes_of(self.eval(b, ctx)?, "|")?;
                let mut all = left;
                all.extend(right);
                Ok(XValue::Nodes(self.sort_unique(all)))
            }
            Expr::Literal(s) => Ok(XValue::Str(s.clone())),
            Expr::Number(x) => Ok(XValue::Number(*x)),
            Expr::Call(func, args) => self.call(*func, args, ctx),
            Expr::Path { absolute, steps } => {
                let start = if *absolute {
                    vec![self.root.clone()]
                } else {
                    vec![ctx.node.clone()]
                };
                Ok(XValue::Nodes(self.walk(start, steps)?))
            }
            Expr::Filter {
                base,
                predicates,
                steps,
            } => {
                let value = self.eval(base, ctx)?;
                let mut nodes = self.nodes_of(value, "предикат")?;
                for pred in predicates {
                    nodes = self.filter(nodes, pred)?;
                }
                Ok(XValue::Nodes(self.walk(nodes, steps)?))
            }
        }
    }

    /// Набор узлов из значения; всё прочее там, где нужен набор, — ошибка
    /// (измерено: `//к:товар | 5` и `string(//к:товар)/имя` платформа
    /// отвергает).
    fn nodes_of(&self, value: XValue, what: &str) -> RtResult<Vec<Rc<DomNode>>> {
        match value {
            XValue::Nodes(nodes) => Ok(nodes),
            _ => Err(bad(format!(
                "в выражении XPath «{what}» требует набор узлов"
            ))),
        }
    }

    fn walk(&self, start: Vec<Rc<DomNode>>, steps: &[Step]) -> RtResult<Vec<Rc<DomNode>>> {
        let mut current = start;
        for step in steps {
            let mut out: Vec<Rc<DomNode>> = Vec::new();
            for node in &current {
                let mut candidates = self.axis_nodes(step.axis, node);
                candidates.retain(|c| self.test_matches(&step.test, c, step.axis));
                for pred in &step.predicates {
                    candidates = self.filter(candidates, pred)?;
                }
                out.extend(candidates);
            }
            current = self.sort_unique(out);
        }
        Ok(current)
    }

    /// Отбор по предикату. Позиция считается ВНУТРИ набора, полученного от
    /// одного контекстного узла, и просто по порядку: обратные оси уже
    /// отдали свои узлы от ближайшего к дальнему, поэтому у них первым
    /// оказывается ближайший — измерено (`//к:товар[2]/preceding::*[1]`
    /// даёт `цена` предыдущего товара, а не первый элемент документа).
    fn filter(&self, nodes: Vec<Rc<DomNode>>, pred: &Expr) -> RtResult<Vec<Rc<DomNode>>> {
        let size = nodes.len();
        let mut out = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let position = i + 1;
            let ctx = Ctx {
                node: node.clone(),
                position: Some(position),
                size: Some(size),
            };
            let value = self.eval(pred, &ctx)?;
            let keep = match value {
                XValue::Number(x) => x == position as f64,
                other => other.to_bool(),
            };
            if keep {
                out.push(node.clone());
            }
        }
        Ok(out)
    }

    fn axis_nodes(&self, axis: Axis, node: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
        match axis {
            Axis::Selff => vec![node.clone()],
            Axis::Child => node.xs_children(),
            Axis::Descendant => {
                let mut out = Vec::new();
                collect_descendants(node, &mut out);
                out
            }
            Axis::DescendantOrSelf => {
                let mut out = vec![node.clone()];
                collect_descendants(node, &mut out);
                out
            }
            Axis::Parent => node.xs_parent().into_iter().collect(),
            Axis::Ancestor => ancestors(node),
            Axis::AncestorOrSelf => {
                let mut out = vec![node.clone()];
                out.extend(ancestors(node));
                out
            }
            Axis::FollowingSibling | Axis::PrecedingSibling => {
                let Some(parent) = node.xs_parent() else {
                    return Vec::new();
                };
                if node.kind() == DomKind::Attribute {
                    return Vec::new();
                }
                let kids = parent.xs_children();
                let Some(at) = kids.iter().position(|k| Rc::ptr_eq(k, node)) else {
                    return Vec::new();
                };
                if axis == Axis::FollowingSibling {
                    kids[at + 1..].to_vec()
                } else {
                    // Обратная ось: ближайший сосед идёт первым.
                    kids[..at].iter().rev().cloned().collect()
                }
            }
            Axis::Following | Axis::Preceding => {
                let mine = self.order_of(node);
                let mut out: Vec<Rc<DomNode>> = Vec::new();
                for other in self.document.iter().cloned() {
                    if other.kind() == DomKind::Attribute || Rc::ptr_eq(&other, node) {
                        continue;
                    }
                    let there = self.order_of(&other);
                    let related = if axis == Axis::Following {
                        // Потомки контекстного узла в `following` не входят.
                        there > mine && !is_descendant_of(&other, node)
                    } else {
                        there < mine && !is_descendant_of(node, &other)
                    };
                    if related {
                        out.push(other);
                    }
                }
                if axis == Axis::Preceding {
                    out.reverse();
                }
                out
            }
            Axis::Attribute => {
                let mut attrs = node.xp_attributes();
                attrs.retain(|a| !a.xp_is_namespace_declaration());
                attrs
            }
        }
    }

    fn test_matches(&self, test: &NodeTest, node: &Rc<DomNode>, axis: Axis) -> bool {
        match test {
            NodeTest::Any => true,
            NodeTest::Text => matches!(node.kind(), DomKind::Text | DomKind::CdataSection),
            NodeTest::Comment => node.kind() == DomKind::Comment,
            NodeTest::ProcessingInstruction(target) => {
                node.kind() == DomKind::ProcessingInstruction
                    && target.as_ref().is_none_or(|t| t == node.xp_node_name())
            }
            NodeTest::Name { prefix, local } => {
                // Проверка имени применима только к узлам «главного вида»
                // оси: у `attribute::` это атрибуты, у остальных —
                // элементы.
                let wanted_kind = if axis == Axis::Attribute {
                    DomKind::Attribute
                } else {
                    DomKind::Element
                };
                if node.kind() != wanted_kind {
                    return false;
                }
                match prefix {
                    Some(p) => {
                        let uri = match self.resolver.lookup(p) {
                            Some(uri) => uri,
                            // Сюда не попадаем: неизвестный префикс отсеян
                            // раньше, при проверке выражения.
                            None => return false,
                        };
                        if node.xs_uri() != uri {
                            return false;
                        }
                    }
                    // Голая `*` берёт узел ЛЮБОГО пространства имён
                    // (измерено: `//*` находит и `к:товар`, и `цена`), а
                    // вот имя без префикса — только узлы БЕЗ пространства
                    // имён (тоже измерено: `//цена` не находит ничего).
                    None => {
                        if local.is_some() && !node.xs_uri().is_empty() {
                            return false;
                        }
                    }
                }
                match local {
                    None => true,
                    Some(name) => node.xs_local_name() == name,
                }
            }
        }
    }

    fn call(&self, func: Func, args: &[Expr], ctx: &Ctx) -> RtResult<XValue> {
        // Аргумент-набор узлов: у функций с необязательным аргументом
        // пропущенный означает контекстный узел.
        let node_arg = |i: usize| -> RtResult<Vec<Rc<DomNode>>> {
            match args.get(i) {
                None => Ok(vec![ctx.node.clone()]),
                Some(e) => self.nodes_of(self.eval(e, ctx)?, "аргумент"),
            }
        };
        let str_arg = |i: usize| -> RtResult<String> {
            match args.get(i) {
                None => Ok(ctx.node.xp_string_value()),
                Some(e) => Ok(self.eval(e, ctx)?.to_str()),
            }
        };
        let num_arg = |i: usize| -> RtResult<f64> {
            match args.get(i) {
                None => Ok(string_to_number(&ctx.node.xp_string_value())),
                Some(e) => Ok(self.eval(e, ctx)?.to_number()),
            }
        };
        // Обязательный аргумент как есть, без приведения. Арность уже
        // проверена разбором, но индексироваться напрямую всё равно
        // нельзя: отсутствующий аргумент обязан стать ошибкой, а не
        // паникой.
        let value_arg = |i: usize| -> RtResult<XValue> {
            match args.get(i) {
                None => Err(bad("в вызове функции XPath не хватает аргумента")),
                Some(e) => self.eval(e, ctx),
            }
        };
        Ok(match func {
            Func::Last => XValue::Number(
                ctx.size
                    .ok_or_else(|| bad("`last()` вне предиката не имеет смысла"))?
                    as f64,
            ),
            Func::Position => XValue::Number(
                ctx.position
                    .ok_or_else(|| bad("`position()` вне предиката не имеет смысла"))?
                    as f64,
            ),
            Func::Count => XValue::Number(node_arg(0)?.len() as f64),
            // Атрибутов типа `ID` эта реализация не знает — см. заголовок.
            Func::Id => XValue::Nodes(Vec::new()),
            Func::LocalName => {
                XValue::Str(node_arg(0)?.first().map(local_name_of).unwrap_or_default())
            }
            Func::NamespaceUri => XValue::Str(
                node_arg(0)?
                    .first()
                    .map(|n| n.xs_uri().to_string())
                    .unwrap_or_default(),
            ),
            Func::Name => XValue::Str(
                node_arg(0)?
                    .first()
                    .map(qualified_name_of)
                    .unwrap_or_default(),
            ),
            Func::String => XValue::Str(str_arg(0)?),
            Func::Concat => {
                let mut out = String::new();
                for a in args {
                    out.push_str(&self.eval(a, ctx)?.to_str());
                }
                XValue::Str(out)
            }
            Func::StartsWith => {
                let (a, b) = (str_arg(0)?, str_arg(1)?);
                XValue::Bool(a.starts_with(&b))
            }
            Func::Contains => {
                let (a, b) = (str_arg(0)?, str_arg(1)?);
                XValue::Bool(a.contains(&b))
            }
            Func::SubstringBefore => {
                let (a, b) = (str_arg(0)?, str_arg(1)?);
                XValue::Str(match a.find(&b) {
                    Some(at) if !b.is_empty() => a[..at].to_string(),
                    _ => String::new(),
                })
            }
            Func::SubstringAfter => {
                let (a, b) = (str_arg(0)?, str_arg(1)?);
                // ПУСТОЙ разделитель находится в начале, и «после» него лежит
                // вся строка: `substring-after('абвгд','')` — `абвгд`
                // (измерено). У `substring-before` тот же пустой разделитель
                // даёт, наоборот, пустую строку — тоже измерено.
                XValue::Str(match a.find(&b) {
                    Some(at) => a[at + b.len()..].to_string(),
                    None => String::new(),
                })
            }
            Func::Substring => {
                let text: Vec<char> = str_arg(0)?.chars().collect();
                let from = round_half_up(num_arg(1)?);
                let len = match args.get(2) {
                    None => f64::INFINITY,
                    Some(e) => round_half_up(self.eval(e, ctx)?.to_number()),
                };
                XValue::Str(substring(&text, from, len))
            }
            Func::StringLength => XValue::Number(str_arg(0)?.chars().count() as f64),
            Func::NormalizeSpace => {
                XValue::Str(str_arg(0)?.split_whitespace().collect::<Vec<_>>().join(" "))
            }
            Func::Translate => {
                let (text, from, to) = (str_arg(0)?, str_arg(1)?, str_arg(2)?);
                let from: Vec<char> = from.chars().collect();
                let to: Vec<char> = to.chars().collect();
                let mut out = String::new();
                for c in text.chars() {
                    match from.iter().position(|f| *f == c) {
                        None => out.push(c),
                        Some(at) => {
                            if let Some(r) = to.get(at) {
                                out.push(*r);
                            }
                        }
                    }
                }
                XValue::Str(out)
            }
            Func::Boolean => XValue::Bool(value_arg(0)?.to_bool()),
            Func::Not => XValue::Bool(!value_arg(0)?.to_bool()),
            Func::True => XValue::Bool(true),
            Func::False => XValue::Bool(false),
            Func::Lang => {
                let wanted = value_arg(0)?.to_str().to_lowercase();
                XValue::Bool(match lang_of(&ctx.node) {
                    None => false,
                    Some(lang) => {
                        let lang = lang.to_lowercase();
                        lang == wanted || lang.starts_with(&format!("{wanted}-"))
                    }
                })
            }
            Func::Number => XValue::Number(num_arg(0)?),
            Func::Sum => {
                let mut total = 0.0;
                for node in node_arg(0)? {
                    total += string_to_number(&node.xp_string_value());
                }
                XValue::Number(total)
            }
            Func::Floor => XValue::Number(value_arg(0)?.to_number().floor()),
            Func::Ceiling => XValue::Number(value_arg(0)?.to_number().ceil()),
            // Округление XPath — К БОЛЬШЕМУ на середине в обе стороны:
            // `round(2.5)` — 3, `round(-2.5)` — -2 (измерено).
            Func::Round => XValue::Number(round_half_up(value_arg(0)?.to_number())),
        })
    }
}

/// Округление к ближайшему целому, середина — в сторону плюс
/// бесконечности.
fn round_half_up(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    (x + 0.5).floor()
}

fn substring(text: &[char], from: f64, len: f64) -> String {
    if from.is_nan() || len.is_nan() {
        return String::new();
    }
    // Номера в XPath с единицы; конец — `from + len`, оба уже округлены.
    let start = from.max(1.0);
    let end = if len.is_infinite() {
        f64::INFINITY
    } else {
        from + len
    };
    let mut out = String::new();
    for (i, c) in text.iter().enumerate() {
        let pos = (i + 1) as f64;
        if pos >= start && pos < end {
            out.push(*c);
        }
    }
    out
}

/// `local-name()`: у элемента и атрибута — локальное имя, у прочих узлов
/// пусто, а у инструкции обработки — её цель.
fn local_name_of(node: &Rc<DomNode>) -> String {
    match node.kind() {
        DomKind::Element | DomKind::Attribute => node.xs_local_name().to_string(),
        DomKind::ProcessingInstruction => node.xp_node_name().to_string(),
        _ => String::new(),
    }
}

fn qualified_name_of(node: &Rc<DomNode>) -> String {
    match node.kind() {
        DomKind::Element | DomKind::Attribute | DomKind::ProcessingInstruction => {
            node.xp_node_name().to_string()
        }
        _ => String::new(),
    }
}

/// Значение `xml:lang`, действующее на узле.
fn lang_of(node: &Rc<DomNode>) -> Option<String> {
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        for attr in n.xp_attributes() {
            if attr.xp_node_name() == "xml:lang" {
                return Some(attr.xp_string_value());
            }
        }
        cur = n.xs_parent();
    }
    None
}

fn collect_descendants(node: &Rc<DomNode>, out: &mut Vec<Rc<DomNode>>) {
    for child in node.xs_children() {
        out.push(child.clone());
        collect_descendants(&child, out);
    }
}

fn ancestors(node: &Rc<DomNode>) -> Vec<Rc<DomNode>> {
    let mut out = Vec::new();
    let mut cur = node.xs_parent();
    while let Some(n) = cur {
        cur = n.xs_parent();
        out.push(n);
    }
    out
}

fn is_descendant_of(node: &Rc<DomNode>, ancestor: &Rc<DomNode>) -> bool {
    let mut cur = node.xs_parent();
    while let Some(n) = cur {
        if Rc::ptr_eq(&n, ancestor) {
            return true;
        }
        cur = n.xs_parent();
    }
    false
}

/// Пронумеровать дерево в документном порядке: узел, затем его атрибуты,
/// затем дети. Атрибуты идут сразу за своим элементом — так их
/// упорядочивает XPath.
fn number_document(
    node: &Rc<DomNode>,
    order: &mut HashMap<usize, usize>,
    out: &mut Vec<Rc<DomNode>>,
) {
    order.insert(key_of(node), out.len());
    out.push(node.clone());
    for attr in node.xp_attributes() {
        order.insert(key_of(&attr), out.len());
        out.push(attr);
    }
    for child in node.xs_children() {
        number_document(&child, order, out);
    }
}

/// Сравнение по правилам XPath 1.0: набор узлов сравнивается
/// «существует пара», а всё прочее приводится к общему виду.
fn compare(op: CmpOp, left: &XValue, right: &XValue) -> bool {
    let equality = matches!(op, CmpOp::Eq | CmpOp::Ne);
    match (left, right) {
        (XValue::Nodes(a), XValue::Nodes(b)) => {
            for x in a {
                for y in b {
                    if compare_scalar(
                        op,
                        &XValue::Str(x.xp_string_value()),
                        &XValue::Str(y.xp_string_value()),
                    ) {
                        return true;
                    }
                }
            }
            false
        }
        // Набор и БУЛЕВО при сравнении на РАВЕНСТВО: набор приводится к
        // булеву ЦЕЛИКОМ (пуст или нет), а не поузлово. ИЗМЕРЕНО:
        // `//нет = false()` — «Да», хотя ни одного узла для поузлового
        // сравнения там нет.
        (XValue::Nodes(a), XValue::Bool(_)) if equality => {
            compare_scalar(op, &XValue::Bool(!a.is_empty()), right)
        }
        (XValue::Bool(_), XValue::Nodes(b)) if equality => {
            compare_scalar(op, left, &XValue::Bool(!b.is_empty()))
        }
        (XValue::Nodes(a), other) => a
            .iter()
            .any(|x| compare_scalar(op, &node_as(other, x, equality), other)),
        (other, XValue::Nodes(b)) => b
            .iter()
            .any(|y| compare_scalar(op, other, &node_as(other, y, equality))),
        (a, b) => compare_scalar(op, a, b),
    }
}

/// Как приводить строковое значение узла для сравнения с другой стороной:
/// с числом — как число, со строкой — как строка, а в неравенстве (`<`,
/// `>`, `<=`, `>=`) — всегда как число. Булево сюда не попадает: с ним
/// набор сравнивается целиком, см. [`compare`].
fn node_as(other: &XValue, node: &Rc<DomNode>, equality: bool) -> XValue {
    match other {
        XValue::Number(_) => XValue::Number(string_to_number(&node.xp_string_value())),
        _ if !equality => XValue::Number(string_to_number(&node.xp_string_value())),
        _ => XValue::Str(node.xp_string_value()),
    }
}

fn compare_scalar(op: CmpOp, left: &XValue, right: &XValue) -> bool {
    match op {
        CmpOp::Eq | CmpOp::Ne => {
            let equal = match (left, right) {
                (XValue::Bool(_), _) | (_, XValue::Bool(_)) => left.to_bool() == right.to_bool(),
                (XValue::Number(_), _) | (_, XValue::Number(_)) => {
                    left.to_number() == right.to_number()
                }
                _ => left.to_str() == right.to_str(),
            };
            if op == CmpOp::Eq {
                equal
            } else {
                !equal
            }
        }
        CmpOp::Lt => left.to_number() < right.to_number(),
        CmpOp::Le => left.to_number() <= right.to_number(),
        CmpOp::Gt => left.to_number() > right.to_number(),
        CmpOp::Ge => left.to_number() >= right.to_number(),
    }
}

/// Проверить, что все префиксы выражения разыменовываются: неизвестный
/// префикс — ошибка ВЫЧИСЛЕНИЯ (измерено: при создании выражения тот же
/// текст платформа принимает).
fn check_prefixes(expr: &Expr, resolver: &XPathResolver) -> RtResult<()> {
    let mut steps: Vec<&Step> = Vec::new();
    collect_steps(expr, &mut steps);
    for step in steps {
        if let NodeTest::Name {
            prefix: Some(p), ..
        } = &step.test
        {
            if resolver.lookup(p).is_none() {
                return Err(bad(format!(
                    "префикс «{p}» в выражении XPath не разыменовывается"
                )));
            }
        }
    }
    Ok(())
}

fn collect_steps<'a>(expr: &'a Expr, out: &mut Vec<&'a Step>) {
    match expr {
        Expr::Or(a, b)
        | Expr::And(a, b)
        | Expr::Compare(_, a, b)
        | Expr::Arith(_, a, b)
        | Expr::Union(a, b) => {
            collect_steps(a, out);
            collect_steps(b, out);
        }
        Expr::Neg(a) => collect_steps(a, out),
        Expr::Literal(_) | Expr::Number(_) => {}
        Expr::Call(_, args) => {
            for a in args {
                collect_steps(a, out);
            }
        }
        Expr::Path { steps, .. } => {
            for step in steps {
                out.push(step);
                for pred in &step.predicates {
                    collect_steps(pred, out);
                }
            }
        }
        Expr::Filter {
            base,
            predicates,
            steps,
        } => {
            collect_steps(base, out);
            for pred in predicates {
                collect_steps(pred, out);
            }
            for step in steps {
                out.push(step);
                for pred in &step.predicates {
                    collect_steps(pred, out);
                }
            }
        }
    }
}

// --- Значения BSL ------------------------------------------------------------

/// `РазыменовательПространствИменDOM`: узел, чью область видимости он
/// показывает, плюс документ — чтобы дерево жило, пока жив разыменователь.
#[derive(Debug)]
pub struct XPathResolver {
    scope: Option<Rc<DomNode>>,
    /// Документ узла. Не читается НИКОГДА и всё же обязателен: ссылки
    /// вверх по дереву слабые, и без сильной ссылки на документ
    /// разыменователь, переживший своё дерево, перестал бы находить
    /// объявления предков — область видимости молча схлопнулась бы до
    /// самого узла.
    #[allow(dead_code)]
    doc: Rc<DomNode>,
}

impl XPathResolver {
    /// URI, связанный с префиксом в области видимости узла. `xmlns` —
    /// пространство имён по умолчанию, пустой префикс и `xml` — ничего:
    /// всё измерено.
    fn lookup(&self, prefix: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        let mut cur = self.scope.clone()?;
        // От атрибута область видимости берётся у его элемента
        // (измерено: резолвер от атрибута корня разыменовывает `к`), а
        // текстовый узел области видимости НЕ даёт вовсе — тоже измерено.
        if matches!(cur.kind(), DomKind::Text | DomKind::CdataSection) {
            return None;
        }
        // От ДОКУМЕНТА — область видимости его элемента: измерено, что
        // `Док.СоздатьРазыменовательПИ(Док)` разыменовывает `к` так же,
        // как разыменователь от корневого элемента.
        if cur.kind() == DomKind::Document {
            cur = document_element(&cur)?;
        }
        loop {
            let wanted = if prefix == "xmlns" { "" } else { prefix };
            if let Some(uri) = cur.xs_namespace_declaration(wanted) {
                return Some(uri);
            }
            cur = cur.xs_parent()?;
        }
    }
}

/// `ВыражениеXPath` — разобранное выражение вместе со своим
/// разыменователем и документом, которым оно создано.
///
/// Исходный текст здесь не хранится: после разбора он не нужен ни
/// вычислению, ни сообщениям об ошибках (неизвестный префикс называет сам
/// себя), а платформа его наружу не отдаёт — членов `Текст` и `Выражение`
/// у `ВыражениеXPath` нет, измерено.
#[derive(Debug)]
pub struct XPathExpression {
    ast: Expr,
    resolver: Rc<XPathResolver>,
    doc: Rc<DomNode>,
}

/// Вид результата — член `ТипРезультатаDOMXPath`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Number,
    Str,
    Bool,
    UnorderedIterator,
    OrderedIterator,
    UnorderedSnapshot,
    OrderedSnapshot,
    AnyUnorderedNode,
    FirstOrderedNode,
}

impl ResultKind {
    fn enum_value(self) -> EnumValue {
        match self {
            ResultKind::Number => EnumValue::XPathNumber,
            ResultKind::Str => EnumValue::XPathString,
            ResultKind::Bool => EnumValue::XPathBoolean,
            ResultKind::UnorderedIterator => EnumValue::XPathUnorderedNodeIterator,
            ResultKind::OrderedIterator => EnumValue::XPathOrderedNodeIterator,
            ResultKind::UnorderedSnapshot => EnumValue::XPathUnorderedNodeSnapshot,
            ResultKind::OrderedSnapshot => EnumValue::XPathOrderedNodeSnapshot,
            ResultKind::AnyUnorderedNode => EnumValue::XPathAnyUnorderedNode,
            ResultKind::FirstOrderedNode => EnumValue::XPathFirstOrderedNode,
        }
    }

    fn is_nodes(self) -> bool {
        !matches!(
            self,
            ResultKind::Number | ResultKind::Str | ResultKind::Bool
        )
    }

    fn is_snapshot(self) -> bool {
        matches!(
            self,
            ResultKind::UnorderedSnapshot | ResultKind::OrderedSnapshot
        )
    }

    fn is_single_node(self) -> bool {
        matches!(
            self,
            ResultKind::AnyUnorderedNode | ResultKind::FirstOrderedNode
        )
    }
}

/// `РезультатXPath`.
#[derive(Debug)]
pub struct XPathResult {
    kind: ResultKind,
    nodes: Vec<Rc<DomNode>>,
    number: f64,
    text: String,
    flag: bool,
    /// Курсор `ПолучитьСледующий`.
    cursor: Cell<usize>,
    doc: Rc<DomNode>,
}

fn requested_kind(arg: Option<&BslValue>) -> RtResult<Option<ResultKind>> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Enum(member)) => Ok(match member {
            EnumValue::XPathAny => None,
            EnumValue::XPathNumber => Some(ResultKind::Number),
            EnumValue::XPathString => Some(ResultKind::Str),
            EnumValue::XPathBoolean => Some(ResultKind::Bool),
            EnumValue::XPathUnorderedNodeIterator => Some(ResultKind::UnorderedIterator),
            EnumValue::XPathOrderedNodeIterator => Some(ResultKind::OrderedIterator),
            EnumValue::XPathUnorderedNodeSnapshot => Some(ResultKind::UnorderedSnapshot),
            EnumValue::XPathOrderedNodeSnapshot => Some(ResultKind::OrderedSnapshot),
            EnumValue::XPathAnyUnorderedNode => Some(ResultKind::AnyUnorderedNode),
            EnumValue::XPathFirstOrderedNode => Some(ResultKind::FirstOrderedNode),
            _ => {
                return Err(RtError::TypeError {
                    expected: "член ТипРезультатаDOMXPath",
                    op: "ВычислитьВыражениеXPath",
                })
            }
        }),
        Some(_) => Err(RtError::TypeError {
            expected: "член ТипРезультатаDOMXPath",
            op: "ВычислитьВыражениеXPath",
        }),
    }
}

/// Собрать результат нужного вида.
///
/// Запрошенный вид НЕ ПРЕОБРАЗУЕТ значение — он только выбирает, какой из
/// членов результата разрешено читать. Это измерено поимённо и заметно
/// расходится с тем, что сделало бы приведение по XPath: `count(//*)`,
/// запрошенный строкой, даёт ПУСТУЮ строку (а не «8»), набор из двух
/// узлов, запрошенный строкой, — тоже пустую (а не строковое значение
/// первого узла), булево, запрошенное числом, — 0 (а не 1), строка «1»,
/// запрошенная числом, — 0. Читается это как хранилище с отдельным полем
/// на каждый вид, заполняемым только «своим» значением, плюс два поля,
/// которые набор узлов заполняет производными: числом становится
/// КОЛИЧЕСТВО узлов (два узла — 2), а булевым — их наличие (два узла —
/// «Да», ноль узлов — «Нет»). Ровно так здесь и сделано.
fn make_result(value: XValue, requested: Option<ResultKind>, doc: Rc<DomNode>) -> XPathResult {
    let kind = requested.unwrap_or(match value {
        XValue::Nodes(_) => ResultKind::UnorderedIterator,
        XValue::Number(_) => ResultKind::Number,
        XValue::Str(_) => ResultKind::Str,
        XValue::Bool(_) => ResultKind::Bool,
    });
    let nodes = match (&value, kind.is_nodes()) {
        (XValue::Nodes(nodes), true) => nodes.clone(),
        _ => Vec::new(),
    };
    let number = match &value {
        XValue::Nodes(nodes) => nodes.len() as f64,
        XValue::Number(x) => *x,
        XValue::Str(_) | XValue::Bool(_) => 0.0,
    };
    let text = match &value {
        XValue::Str(s) => s.clone(),
        XValue::Nodes(_) | XValue::Number(_) | XValue::Bool(_) => String::new(),
    };
    let flag = match &value {
        XValue::Nodes(nodes) => !nodes.is_empty(),
        XValue::Bool(b) => *b,
        XValue::Number(_) | XValue::Str(_) => false,
    };
    XPathResult {
        kind,
        nodes,
        number,
        text,
        flag,
        cursor: Cell::new(0),
        doc,
    }
}

// --- Приём аргументов -----------------------------------------------------

fn as_document(v: &BslValue, method: &'static str) -> RtResult<Rc<DomNode>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::DomNode(node, _) if node.kind() == DomKind::Document => Ok(node.clone()),
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        }),
    }
}

fn as_node(v: &BslValue) -> Option<(Rc<DomNode>, Rc<DomNode>)> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::DomNode(node, doc) => Some((node.clone(), doc.clone())),
            _ => None,
        },
        _ => None,
    }
}

fn as_resolver(v: &BslValue) -> Option<Rc<XPathResolver>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XPathResolver(r) => Some(r.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn as_result(v: &BslValue, method: &'static str) -> RtResult<Rc<XPathResult>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XPathResult(r) => Ok(r.clone()),
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: v.type_name(),
        }),
    }
}

fn need_expression_text(arg: Option<&BslValue>) -> RtResult<String> {
    match arg {
        Some(BslValue::Str(s)) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка — выражение XPath",
            op: "ВычислитьВыражениеXPath",
        }),
    }
}

/// Контекстный узел вычисления. У ДОКУМЕНТА контекстом служит элемент
/// документа (измерено), текстовый узел контекстом быть не может, а узел
/// чужого документа — ошибка.
fn context_node(arg: Option<&BslValue>, doc: &Rc<DomNode>) -> RtResult<Rc<DomNode>> {
    let Some((node, node_doc)) = arg.and_then(as_node) else {
        return Err(RtError::TypeError {
            expected: "узел DOM",
            op: "ВычислитьВыражениеXPath",
        });
    };
    if !Rc::ptr_eq(&node_doc, doc) {
        return Err(bad(
            "контекстный узел выражения XPath принадлежит другому документу",
        ));
    }
    match node.kind() {
        // Документ контекстом не служит: им служит его ЭЛЕМЕНТ (измерено).
        DomKind::Document => document_element(&node).ok_or_else(|| {
            bad("у документа нет элемента документа, вычислять выражение XPath не от чего")
        }),
        // Всё остальное — включая ТЕКСТ: измерено, что от текстового узла
        // `string(.)` даёт его текст, `name(..)` — имя родителя, а
        // `name(.)` — пустую строку.
        DomKind::Element
        | DomKind::Attribute
        | DomKind::Text
        | DomKind::CdataSection
        | DomKind::Comment
        | DomKind::ProcessingInstruction
        | DomKind::EntityReference => Ok(node),
    }
}

// --- Методы и свойства ------------------------------------------------------

/// `ДокументDOM.СоздатьРазыменовательПИ([Узел])` / `CreateNSResolver`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель — не документ, и
/// [`RtError::TypeError`] при более чем двух аргументах.
pub fn create_ns_resolver(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = as_document(obj, "СоздатьРазыменовательПИ")?;
    // Платформа принимает и ноль, и один, и ДВА аргумента (измерено);
    // смысл имеет только первый.
    if args.len() > 2 {
        return Err(RtError::TypeError {
            expected: "не больше двух аргументов",
            op: "СоздатьРазыменовательПИ",
        });
    }
    // Держать надо дерево ТОГО узла, чью область видимости показывает
    // разыменователь, а не дерево получателя: узел могли взять из чужого
    // документа, ссылки вверх по дереву слабые — и чужое дерево, оставшись
    // без сильной ссылки, освободилось бы, а область видимости молча
    // схлопнулась бы до самого узла (инвариант поля `doc`).
    let (scope, doc) = match args.first() {
        // Без аргумента область видимости — у элемента документа
        // (измерено: такой резолвер разыменовывает префиксы корня).
        None => (document_element(&doc), doc),
        // Не-узел аргументом платформа принимает и даёт резолвер, который
        // не разыменовывает ничего (измерено на числе); области видимости
        // нет, так что держать можно дерево получателя — безразлично какое.
        Some(v) => match as_node(v) {
            Some((node, node_doc)) => (Some(node), node_doc),
            None => (None, doc),
        },
    };
    Ok(BslValue::Object(Rc::new(BslObject::XPathResolver(
        Rc::new(XPathResolver { scope, doc }),
    ))))
}

/// `Новый РазыменовательПространствИменDOM(Узел)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не узел DOM: конструктор, в
/// отличие от метода документа, без узла не работает (измерено).
pub fn new_ns_resolver(arg: &BslValue) -> RtResult<BslValue> {
    let Some((node, doc)) = as_node(arg) else {
        return Err(RtError::TypeError {
            expected: "узел DOM",
            op: "Новый РазыменовательПространствИменDOM",
        });
    };
    Ok(BslValue::Object(Rc::new(BslObject::XPathResolver(
        Rc::new(XPathResolver {
            scope: Some(node),
            doc,
        }),
    ))))
}

fn document_element(doc: &Rc<DomNode>) -> Option<Rc<DomNode>> {
    doc.xs_children()
        .into_iter()
        .find(|c| c.kind() == DomKind::Element)
}

/// `РазыменовательПространствИменDOM.НайтиURIПространстваИмен(Префикс)`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] у чужого получателя,
/// [`RtError::TypeError`], если префикс не строка.
pub fn lookup_namespace_uri(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let resolver = match as_resolver(obj) {
        Some(r) => r,
        None => {
            return Err(RtError::MethodNotApplicable {
                method: "НайтиURIПространстваИмен",
                receiver: obj.type_name(),
            })
        }
    };
    let prefix = match args {
        [BslValue::Str(s)] => s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "ровно один аргумент — префикс строкой",
                op: "НайтиURIПространстваИмен",
            })
        }
    };
    Ok(match resolver.lookup(&prefix) {
        Some(uri) => str_value(&uri),
        None => BslValue::Undefined,
    })
}

/// `ДокументDOM.СоздатьВыражениеXPath(Выражение, Разыменователь)`.
///
/// # Errors
///
/// [`RtError::XPath`] на негодном синтаксисе (измерено, что платформа
/// ловит его именно здесь), [`RtError::TypeError`] на неверных аргументах.
pub fn create_expression(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = as_document(obj, "СоздатьВыражениеXPath")?;
    if args.len() != 2 {
        return Err(RtError::TypeError {
            expected: "ровно два аргумента — выражение и разыменователь",
            op: "СоздатьВыражениеXPath",
        });
    }
    let text = need_expression_text(args.first())?;
    let Some(resolver) = as_resolver(&args[1]) else {
        return Err(RtError::TypeError {
            expected: "РазыменовательПространствИменDOM",
            op: "СоздатьВыражениеXPath",
        });
    };
    let ast = parse(&text)?;
    Ok(BslValue::Object(Rc::new(BslObject::XPathExpression(
        Rc::new(XPathExpression { ast, resolver, doc }),
    ))))
}

/// `ДокументDOM.ВычислитьВыражениеXPath(Выражение, Узел, Разыменователь[, Вид])`.
///
/// # Errors
///
/// [`RtError::XPath`] на негодном выражении, неизвестном префиксе и
/// неподходящем контекстном узле; [`RtError::TypeError`] на неверных
/// аргументах.
pub fn evaluate(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = as_document(obj, "ВычислитьВыражениеXPath")?;
    if args.len() != 3 && args.len() != 4 {
        return Err(RtError::TypeError {
            expected: "три аргумента (выражение, узел, разыменователь) либо четыре",
            op: "ВычислитьВыражениеXPath",
        });
    }
    let text = need_expression_text(args.first())?;
    let context = context_node(args.get(1), &doc)?;
    let Some(resolver) = as_resolver(&args[2]) else {
        return Err(RtError::TypeError {
            expected: "РазыменовательПространствИменDOM",
            op: "ВычислитьВыражениеXPath",
        });
    };
    let kind = requested_kind(args.get(3))?;
    let ast = parse(&text)?;
    run(&ast, &context, &resolver, kind, doc)
}

/// `ВыражениеXPath.Вычислить(Узел[, Вид])`.
///
/// # Errors
///
/// Те же, что у [`evaluate`], кроме разбора: выражение уже разобрано.
pub fn evaluate_expression(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let expr = match obj {
        BslValue::Object(o) => match &**o {
            BslObject::XPathExpression(e) => e.clone(),
            _ => {
                return Err(RtError::MethodNotApplicable {
                    method: "Вычислить",
                    receiver: obj.type_name(),
                })
            }
        },
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: "Вычислить",
                receiver: obj.type_name(),
            })
        }
    };
    if args.is_empty() || args.len() > 2 {
        return Err(RtError::TypeError {
            expected: "один аргумент (контекстный узел) либо два",
            op: "ВыражениеXPath.Вычислить",
        });
    }
    let context = context_node(args.first(), &expr.doc)?;
    let kind = requested_kind(args.get(1))?;
    run(&expr.ast, &context, &expr.resolver, kind, expr.doc.clone())
}

fn run(
    ast: &Expr,
    context: &Rc<DomNode>,
    resolver: &XPathResolver,
    kind: Option<ResultKind>,
    doc: Rc<DomNode>,
) -> RtResult<BslValue> {
    check_prefixes(ast, resolver)?;
    let evaluator = Evaluator::new(root_of(context), resolver);
    let ctx = Ctx {
        node: context.clone(),
        position: None,
        size: None,
    };
    let value = evaluator.eval(ast, &ctx)?;
    let value = match value {
        XValue::Nodes(nodes) => XValue::Nodes(evaluator.sort_unique(nodes)),
        other => other,
    };
    Ok(BslValue::Object(Rc::new(BslObject::XPathResult(Rc::new(
        make_result(value, kind, doc),
    )))))
}

/// `РезультатXPath.ПолучитьСледующий()` / `IterateNext`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] у чужого получателя,
/// [`RtError::TypeError`] при лишних аргументах.
pub fn next_node(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let result = as_result(obj, "ПолучитьСледующий")?;
    if !args.is_empty() {
        return Err(RtError::TypeError {
            expected: "без аргументов",
            op: "ПолучитьСледующий",
        });
    }
    // У числа, строки и булева обход отдаёт `Неопределено`, а не ошибку —
    // измерено на всех трёх.
    let at = result.cursor.get();
    match result.nodes.get(at) {
        None => Ok(BslValue::Undefined),
        Some(node) => {
            result.cursor.set(at + 1);
            Ok(crate::dom::node_value(node, &result.doc))
        }
    }
}

/// `РезультатXPath.ЭлементСнимка(Номер)` / `SnapshotItem`.
///
/// # Errors
///
/// [`RtError::XPath`], если результат — не снимок (измерено: у итератора
/// платформа отвечает ошибкой), [`RtError::TypeError`] на неверном номере.
pub fn snapshot_item(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let result = as_result(obj, "ЭлементСнимка")?;
    if !result.kind.is_snapshot() {
        return Err(bad("`ЭлементСнимка` есть только у результата-снимка"));
    }
    let index = match args {
        [BslValue::Number(n)] => n.clone(),
        _ => {
            return Err(RtError::TypeError {
                expected: "ровно один аргумент — номер узла",
                op: "ЭлементСнимка",
            })
        }
    };
    // Номер вне снимка — `Неопределено`, а не ошибка; отрицательный тоже
    // (измерено оба).
    let Some(at) = index.to_i64_exact() else {
        return Err(RtError::TypeError {
            expected: "целый номер узла",
            op: "ЭлементСнимка",
        });
    };
    if at < 0 {
        return Ok(BslValue::Undefined);
    }
    Ok(match result.nodes.get(at as usize) {
        None => BslValue::Undefined,
        Some(node) => crate::dom::node_value(node, &result.doc),
    })
}

/// Свойства `РезультатXPath`.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на неизвестном имени и [`RtError::XPath`],
/// если член не подходит виду результата (измерено, что платформа
/// разграничивает их именно так) либо число не представимо.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let result = as_result(obj, "свойство результата XPath")?;
    let is = |ru: &str, en: &str| name.eq_ignore_ascii_case(ru) || name.eq_ignore_ascii_case(en);

    if is("ТипРезультата", "ResultType") {
        return Ok(BslValue::Enum(result.kind.enum_value()));
    }
    if is("ЧисловоеЗначение", "NumberValue") {
        if result.kind != ResultKind::Number {
            return Err(bad("`ЧисловоеЗначение` есть только у числового результата"));
        }
        return match bsl_number::BslNumber::from_f64(result.number) {
            Ok(n) => Ok(BslValue::Number(n)),
            // Бесконечность и `NaN` десятичным числом не представимы —
            // см. расхождения в заголовке модуля.
            Err(_) => Err(bad(format!(
                "числовое значение выражения XPath ({}) не представимо числом BSL",
                number_to_string(result.number)
            ))),
        };
    }
    if is("СтроковоеЗначение", "StringValue") {
        if result.kind != ResultKind::Str {
            return Err(bad(
                "`СтроковоеЗначение` есть только у строкового результата",
            ));
        }
        return Ok(str_value(&result.text));
    }
    if is("БулевоЗначение", "BooleanValue") {
        if result.kind != ResultKind::Bool {
            return Err(bad("`БулевоЗначение` есть только у булева результата"));
        }
        return Ok(BslValue::Boolean(result.flag));
    }
    // Русского написания у этого члена НЕТ — измерено перебором.
    if name.eq_ignore_ascii_case("SingleNodeValue") {
        if !result.kind.is_single_node() {
            return Err(bad(
                "`SingleNodeValue` есть только у результата вида «первый узел»",
            ));
        }
        return Ok(match result.nodes.first() {
            None => BslValue::Undefined,
            Some(node) => crate::dom::node_value(node, &result.doc),
        });
    }
    if is("РазмерСнимка", "SnapshotLength") {
        if !result.kind.is_snapshot() {
            return Err(bad("`РазмерСнимка` есть только у результата-снимка"));
        }
        return Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
            result.nodes.len() as i64,
        )));
    }
    if is("НеверноеСостояниеИтератора", "InvalidIteratorState") {
        // Наши результаты — снимки, испортиться обходу нечем; у платформы
        // этот признак тоже всегда отвечал «Нет».
        return Ok(BslValue::Boolean(false));
    }
    Err(RtError::UnknownColumn(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    fn document_of(text: &str) -> BslValue {
        let builder = BslValue::new_dom_builder();
        let reader = BslValue::new_xml_reader();
        crate::xml::set_string(&reader, &[str_value(text)]).unwrap();
        dom::read(&builder, &[reader]).unwrap()
    }

    /// Документ замеров: два товара в пространстве имён `urn:к`, всё
    /// остальное — в пространстве по умолчанию `urn:по`.
    fn sample() -> BslValue {
        document_of(
            "<к:каталог xmlns:к=\"urn:к\" xmlns=\"urn:по\" версия=\"3\"><!--шапка-->\
             <к:товар код=\"1\" вес=\"2\"><имя>яблоко</имя><цена>10</цена></к:товар>\
             <к:товар код=\"2\"><имя>груша</имя><цена>20</цена></к:товар>\
             <прочее к:метка=\"м\">хвост</прочее><?пи данные?></к:каталог>",
        )
    }

    fn resolver_of(doc: &BslValue) -> BslValue {
        create_ns_resolver(doc, &[]).unwrap()
    }

    /// Вычислить выражение и напечатать результат так же, как это делает
    /// contract-скрипт: вид результата плюс его значение.
    fn eval_text(doc: &BslValue, expr: &str) -> String {
        let resolver = resolver_of(doc);
        let result = evaluate(doc, &[str_value(expr), doc.clone(), resolver]).unwrap();
        show(&result)
    }

    fn show(result: &BslValue) -> String {
        let kind = get_property(result, "ТипРезультата").unwrap();
        let BslValue::Enum(member) = kind else {
            panic!("вид результата — не член перечисления");
        };
        match member {
            EnumValue::XPathNumber => format!(
                "число {}",
                match get_property(result, "ЧисловоеЗначение") {
                    Ok(BslValue::Number(n)) => n.to_canonical(),
                    _ => "<не представимо>".to_string(),
                }
            ),
            EnumValue::XPathString => format!(
                "строка [{}]",
                match get_property(result, "СтроковоеЗначение").unwrap() {
                    BslValue::Str(s) => s.to_string(),
                    other => panic!("не строка: {other:?}"),
                }
            ),
            EnumValue::XPathBoolean => format!(
                "булево {:?}",
                get_property(result, "БулевоЗначение").unwrap()
            ),
            _ => {
                let mut out = String::from("узлы:");
                loop {
                    let node = next_node(result, &[]).unwrap();
                    if node == BslValue::Undefined {
                        break;
                    }
                    out.push(' ');
                    out.push_str(&describe(&node));
                }
                out
            }
        }
    }

    fn describe(node: &BslValue) -> String {
        let name = match dom::get_property(node, "ИмяУзла").unwrap() {
            BslValue::Str(s) => s.to_string(),
            other => panic!("имя узла — не строка: {other:?}"),
        };
        match dom::get_property(node, "ЗначениеУзла").unwrap() {
            BslValue::Str(s) => format!("{name}={s}"),
            _ => name,
        }
    }

    #[test]
    fn document_context_is_the_document_element() {
        let doc = sample();
        // Измерено: `name(.)` от документа — имя КОРНЕВОГО элемента, а
        // `*` от документа даёт детей корня, а не сам корень.
        assert_eq!(eval_text(&doc, "name(.)"), "строка [к:каталог]");
        assert_eq!(
            eval_text(&doc, "*"),
            "узлы: к:товар к:товар прочее",
            "контекстом документа служит элемент документа"
        );
        assert_eq!(eval_text(&doc, "count(/ | .)"), "число 2");
    }

    #[test]
    fn a_name_without_a_prefix_matches_only_the_empty_namespace() {
        let doc = sample();
        // `цена` лежит в пространстве по умолчанию, и без префикса не
        // находится — измерено.
        assert_eq!(eval_text(&doc, "//цена"), "узлы:");
        assert_eq!(eval_text(&doc, "count(//к:товар)"), "число 2");
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена']"),
            "узлы: цена цена"
        );
    }

    #[test]
    fn an_unknown_prefix_is_an_error_not_an_empty_set() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let err = evaluate(&doc, &[str_value("//нет:товар"), doc.clone(), resolver]).unwrap_err();
        assert!(
            matches!(err, RtError::XPath(ref m) if m.contains("нет")),
            "ожидалась ошибка про префикс, получено {err:?}"
        );
    }

    #[test]
    fn axes_and_predicates_walk_in_document_order() {
        let doc = sample();
        assert_eq!(eval_text(&doc, "//к:товар[1]/@код"), "узлы: код=1");
        assert_eq!(eval_text(&doc, "//к:товар[last()]/@код"), "узлы: код=2");
        assert_eq!(eval_text(&doc, "//к:товар[@код='2']/@вес"), "узлы:");
        assert_eq!(eval_text(&doc, "//к:товар[not(@вес)]/@код"), "узлы: код=2");
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена']/preceding-sibling::*"),
            "узлы: имя имя"
        );
        assert_eq!(
            eval_text(&doc, "//*[local-name()='имя']/following::*"),
            "узлы: цена к:товар имя цена прочее"
        );
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена']/ancestor::*"),
            "узлы: к:каталог к:товар к:товар"
        );
        assert_eq!(
            eval_text(&doc, "//к:товар/@код/.."),
            "узлы: к:товар к:товар"
        );
        // Обратные оси нумеруются от БЛИЖАЙШЕГО узла — измерено на всех
        // четырёх.
        assert_eq!(
            eval_text(&doc, "//к:товар[2]/preceding::*[1]"),
            "узлы: цена"
        );
        assert_eq!(eval_text(&doc, "//к:товар[2]/preceding::*[2]"), "узлы: имя");
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена']/ancestor::*[1]"),
            "узлы: к:товар к:товар"
        );
        assert_eq!(
            eval_text(
                &doc,
                "//к:товар[2]/*[local-name()='цена']/preceding-sibling::*[1]"
            ),
            "узлы: имя"
        );
    }

    /// Объявления `xmlns` в дереве — обычные атрибуты, а на оси
    /// `attribute::` их нет (измерено).
    #[test]
    fn namespace_declarations_are_not_on_the_attribute_axis() {
        let doc = sample();
        assert_eq!(eval_text(&doc, "count(/*/@*)"), "число 1");
        assert_eq!(eval_text(&doc, "/*/@xmlns:к"), "узлы:");
        assert_eq!(eval_text(&doc, "/*/@*"), "узлы: версия=3");
    }

    #[test]
    fn node_tests_reach_comments_text_and_processing_instructions() {
        let doc = sample();
        assert_eq!(eval_text(&doc, "//comment()"), "узлы: #comment=шапка");
        assert_eq!(eval_text(&doc, "count(//text())"), "число 5");
        assert_eq!(
            eval_text(&doc, "count(//processing-instruction('пи'))"),
            "число 1"
        );
        assert_eq!(
            eval_text(&doc, "count(//processing-instruction('нет'))"),
            "число 0"
        );
        assert_eq!(eval_text(&doc, "count(//node())"), "число 15");
    }

    #[test]
    fn string_and_number_functions_follow_the_measured_platform() {
        let doc = sample();
        assert_eq!(
            eval_text(&doc, "string(/)"),
            "строка [яблоко10груша20хвост]"
        );
        assert_eq!(eval_text(&doc, "string(//к:товар)"), "строка [яблоко10]");
        assert_eq!(eval_text(&doc, "concat('а', 'б', 'в')"), "строка [абв]");
        assert_eq!(eval_text(&doc, "substring('абвгд', 2, 3)"), "строка [бвг]");
        assert_eq!(
            eval_text(&doc, "substring('абвгд', 1.5, 2.6)"),
            "строка [бвг]"
        );
        assert_eq!(eval_text(&doc, "substring('абвгд', -1, 3)"), "строка [а]");
        assert_eq!(
            eval_text(&doc, "translate('абв', 'аб', 'А')"),
            "строка [Ав]"
        );
        assert_eq!(
            eval_text(&doc, "normalize-space('  а   б  ')"),
            "строка [а б]"
        );
        assert_eq!(eval_text(&doc, "sum(//*[local-name()='цена'])"), "число 30");
        assert_eq!(eval_text(&doc, "round(2.5)"), "число 3");
        assert_eq!(eval_text(&doc, "round(-2.5)"), "число -2");
        assert_eq!(eval_text(&doc, "-7 mod 2"), "число -1");
        assert_eq!(eval_text(&doc, "7 div 2"), "число 3.5");
        assert_eq!(eval_text(&doc, "2 + 3 * 4"), "число 14");
        assert_eq!(eval_text(&doc, "(2 + 3) * 4"), "число 20");
    }

    /// Числа печатаются пятнадцатью значащими цифрами и С ЗАПЯТОЙ —
    /// измерено на платформе, в том числе длина строки.
    #[test]
    fn number_to_string_matches_the_platform() {
        assert_eq!(number_to_string(1.0 / 3.0), "0,333333333333333");
        assert_eq!(number_to_string(1.0 / 3.0).chars().count(), 17);
        assert_eq!(number_to_string(0.5), "0,5");
        assert_eq!(number_to_string(-0.5), "-0,5");
        assert_eq!(number_to_string(10.0), "10");
        assert_eq!(number_to_string(1000.0), "1000");
        assert_eq!(number_to_string(2.0 / 3.0), "0,666666666666667");
        assert_eq!(number_to_string(1.0 / 7.0), "0,142857142857143");
        assert_eq!(
            number_to_string(123456789012345678.0),
            "1,23456789012346e+17"
        );
        assert_eq!(number_to_string(f64::INFINITY), "Infinity");
        assert_eq!(number_to_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(number_to_string(f64::NAN), "NaN");
    }

    #[test]
    fn infinities_reach_string_but_not_the_bsl_number() {
        let doc = sample();
        assert_eq!(eval_text(&doc, "string(1 div 0)"), "строка [Infinity]");
        assert_eq!(eval_text(&doc, "string(0 div 0)"), "строка [NaN]");
        assert_eq!(eval_text(&doc, "1 div 0 > 1000000"), "булево Boolean(true)");
        let resolver = resolver_of(&doc);
        let result = evaluate(&doc, &[str_value("1 div 0"), doc.clone(), resolver]).unwrap();
        assert!(
            get_property(&result, "ЧисловоеЗначение").is_err(),
            "бесконечность не представима числом BSL — это должна быть ошибка"
        );
    }

    /// Входная лексика числа — только с точкой: измерено, что
    /// `number('3,5')` даёт `NaN`.
    #[test]
    fn number_parsing_takes_a_dot_and_refuses_a_comma() {
        let doc = sample();
        assert_eq!(eval_text(&doc, "string(number('3.5'))"), "строка [3,5]");
        assert_eq!(eval_text(&doc, "string(number('3,5'))"), "строка [NaN]");
        assert_eq!(eval_text(&doc, "string(number('  7  '))"), "строка [7]");
        assert_eq!(eval_text(&doc, "string(1e3)"), "строка [1000]");
    }

    #[test]
    fn comparisons_expand_node_sets() {
        let doc = sample();
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена'] = '10'"),
            "булево Boolean(true)"
        );
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена'] > 15"),
            "булево Boolean(true)"
        );
        assert_eq!(eval_text(&doc, "//к:товар[@код=2]/@вес"), "узлы:");
        assert_eq!(eval_text(&doc, "count(//к:товар[@код='1'])"), "число 1");
        // С БУЛЕВЫМ набор сравнивается целиком, а не поузлово: измерено,
        // что пустой набор равен `false()`, хотя сравнивать там нечего.
        assert_eq!(eval_text(&doc, "//нет = false()"), "булево Boolean(true)");
        assert_eq!(eval_text(&doc, "//нет = true()"), "булево Boolean(false)");
        assert_eq!(eval_text(&doc, "//нет != true()"), "булево Boolean(true)");
        assert_eq!(
            eval_text(&doc, "//к:товар = true()"),
            "булево Boolean(true)"
        );
        assert_eq!(
            eval_text(&doc, "//к:товар = false()"),
            "булево Boolean(false)"
        );
        assert_eq!(
            eval_text(&doc, "//*[local-name()='цена'] != 'нет'"),
            "булево Boolean(true)"
        );
    }

    /// `lang()` смотрит `xml:lang` НА УЗЛЕ И У ПРЕДКОВ, сравнивает без
    /// учёта регистра и принимает префикс до дефиса. Все шесть значений
    /// измерены на документе `<а xml:lang="ru-RU"><б/></а>`: проба
    /// «ф lang от предка» отдала `1 1 1 0 1 1` ровно в этом порядке.
    #[test]
    fn lang_reads_xml_lang_from_the_ancestors() {
        let doc = document_of("<а xml:lang=\"ru-RU\"><б/></а>");
        assert_eq!(eval_text(&doc, "count(//б[lang('ru')])"), "число 1");
        assert_eq!(eval_text(&doc, "count(//б[lang('RU')])"), "число 1");
        assert_eq!(eval_text(&doc, "count(//б[lang('ru-RU')])"), "число 1");
        assert_eq!(eval_text(&doc, "count(//б[lang('en')])"), "число 0");
        assert_eq!(eval_text(&doc, "count(//а[lang('ru')])"), "число 1");
        // `xml:lang` — обычный атрибут и виден на оси атрибутов
        // (измерено).
        assert_eq!(eval_text(&doc, "count(//а/@*)"), "число 1");
    }

    /// Пустой разделитель у `substring-before` и `substring-after`
    /// разворачивается по-разному — измерено: пустая строка против всей
    /// строки целиком.
    #[test]
    fn an_empty_separator_splits_the_string_at_its_start() {
        let doc = sample();
        assert_eq!(
            eval_text(&doc, "substring-before('абвгд', '')"),
            "строка []"
        );
        assert_eq!(
            eval_text(&doc, "substring-after('абвгд', '')"),
            "строка [абвгд]"
        );
        // Непустой разделитель от правки не пострадал, а отсутствующий
        // по-прежнему даёт пустую строку с обеих сторон.
        assert_eq!(
            eval_text(&doc, "substring-before('а-б', '-')"),
            "строка [а]"
        );
        assert_eq!(eval_text(&doc, "substring-after('а-б', '-')"), "строка [б]");
        assert_eq!(eval_text(&doc, "substring-before('абв', 'я')"), "строка []");
        assert_eq!(eval_text(&doc, "substring-after('абв', 'я')"), "строка []");
    }

    /// Ни одна неподдержанная конструкция не даёт молча пустой результат.
    #[test]
    fn unsupported_syntax_is_an_honest_error() {
        for expr in [
            "//",
            "[1]",
            "count(//*",
            "'абв",
            "нетоси::*",
            "$х",
            "if (1=1) then 2 else 3",
            "for $x in //* return $x",
            "нетфункции()",
            "//к:товар/namespace::*",
            "",
            "count()",
            "concat('а')",
            "string-length('а', 'б')",
        ] {
            let doc = sample();
            let resolver = resolver_of(&doc);
            let err = evaluate(&doc, &[str_value(expr), doc.clone(), resolver]);
            assert!(
                err.is_err(),
                "выражение «{expr}» обязано быть отвергнуто, а не дать пустой результат"
            );
        }
    }

    #[test]
    fn position_outside_a_predicate_is_an_error() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        assert!(evaluate(
            &doc,
            &[str_value("position()"), doc.clone(), resolver.clone()]
        )
        .is_err());
        assert!(evaluate(&doc, &[str_value("last()"), doc.clone(), resolver]).is_err());
        // А внутри предиката — работают.
        assert_eq!(
            eval_text(&doc, "//к:товар[position()=2]/@код"),
            "узлы: код=2"
        );
    }

    #[test]
    fn the_resolver_reads_the_scope_of_its_node() {
        let doc = document_of("<к:а xmlns:к=\"urn:1\"><к:б xmlns:к=\"urn:2\"><к:в/></к:б></к:а>");
        let root = dom::get_property(&doc, "ЭлементДокумента").unwrap();
        let inner = dom::get_property(&root, "ПервыйДочерний").unwrap();
        let at_root = create_ns_resolver(&doc, &[root]).unwrap();
        let at_inner = create_ns_resolver(&doc, &[inner]).unwrap();
        assert_eq!(
            lookup_namespace_uri(&at_root, &[str_value("к")]).unwrap(),
            str_value("urn:1")
        );
        assert_eq!(
            lookup_namespace_uri(&at_inner, &[str_value("к")]).unwrap(),
            str_value("urn:2")
        );
        // Резолвер без узла и резолвер ОТ ДОКУМЕНТА равны резолверу от
        // элемента документа — измерено оба.
        let bare = create_ns_resolver(&doc, &[]).unwrap();
        assert_eq!(
            lookup_namespace_uri(&bare, &[str_value("к")]).unwrap(),
            str_value("urn:1")
        );
        let whole = create_ns_resolver(&doc, std::slice::from_ref(&doc)).unwrap();
        assert_eq!(
            lookup_namespace_uri(&whole, &[str_value("к")]).unwrap(),
            str_value("urn:1")
        );
        // Выборка идёт по URI, а не по префиксу: с резолвером от корня
        // `//к:*` находит только узлы первого связывания (измерено).
        let result = evaluate(&doc, &[str_value("//к:*"), doc.clone(), at_root]).unwrap();
        assert_eq!(show(&result), "узлы: к:а");
    }

    /// Разыменователь, созданный МЕТОДОМ одного документа по узлу другого,
    /// удерживает ЧУЖОЕ дерево: объявление `xmlns:к` висит на корне чужого
    /// документа, а ссылки вверх слабые, так что держи разыменователь
    /// документ получателя — область видимости схлопнулась бы до самого
    /// узла и `к` перестал бы разыменовываться.
    #[test]
    fn the_method_resolver_keeps_a_foreign_tree_alive() {
        let local = document_of("<а/>");
        // Из блока выносится ТОЛЬКО разыменователь: `foreign`, `root` и
        // `inner` держат чужой документ сильно, и они обязаны умереть здесь,
        // иначе проверка ничего не проверяет.
        let resolver = {
            let foreign = document_of("<к:а xmlns:к=\"urn:2\"><к:б><к:в/></к:б></к:а>");
            let root = dom::get_property(&foreign, "ЭлементДокумента").unwrap();
            let inner = dom::get_property(&root, "ПервыйДочерний").unwrap();
            create_ns_resolver(&local, &[inner]).unwrap()
        };
        assert_eq!(
            lookup_namespace_uri(&resolver, &[str_value("к")]).unwrap(),
            str_value("urn:2")
        );
    }

    /// `xmlns` — это пространство имён по умолчанию, `xml` и пустой
    /// префикс не разыменовываются: всё три измерено.
    #[test]
    fn the_resolver_knows_xmlns_but_not_xml() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        assert_eq!(
            lookup_namespace_uri(&resolver, &[str_value("xmlns")]).unwrap(),
            str_value("urn:по")
        );
        assert_eq!(
            lookup_namespace_uri(&resolver, &[str_value("xml")]).unwrap(),
            BslValue::Undefined
        );
        assert_eq!(
            lookup_namespace_uri(&resolver, &[str_value("")]).unwrap(),
            BslValue::Undefined
        );
    }

    /// Запрошенный вид не преобразует значение, а выбирает поле —
    /// измерено поимённо (см. `make_result`).
    #[test]
    fn requested_kinds_pick_a_field_instead_of_converting() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let ask = |expr: &str, member: EnumValue| {
            evaluate(
                &doc,
                &[
                    str_value(expr),
                    doc.clone(),
                    resolver.clone(),
                    BslValue::Enum(member),
                ],
            )
            .unwrap()
        };
        // Пустой набор в трёх видах — измеренные 0, пустая строка и «Нет».
        let as_number = ask("//цена", EnumValue::XPathNumber);
        assert_eq!(show(&as_number), "число 0");
        let as_string = ask("//цена", EnumValue::XPathString);
        assert_eq!(show(&as_string), "строка []");
        let as_bool = ask("//цена", EnumValue::XPathBoolean);
        assert_eq!(show(&as_bool), "булево Boolean(false)");
        // Непустой набор: числом становится КОЛИЧЕСТВО узлов, строкой —
        // пустая строка, булевым — «Да». Всё три измерены.
        let count_of = ask("//*[local-name()='цена']", EnumValue::XPathNumber);
        assert_eq!(show(&count_of), "число 2");
        let text_of = ask("//*[local-name()='цена']", EnumValue::XPathString);
        assert_eq!(show(&text_of), "строка []");
        let flag_of = ask("//к:товар", EnumValue::XPathBoolean);
        assert_eq!(show(&flag_of), "булево Boolean(true)");
        // А скаляр в чужой вид не переводится вовсе: булево числом — 0,
        // строка «1» числом — 0, число булевым — «Нет», число строкой —
        // пустая строка (измерено все четыре).
        assert_eq!(show(&ask("1=1", EnumValue::XPathNumber)), "число 0");
        assert_eq!(
            show(&ask("string(//к:товар/@код)", EnumValue::XPathNumber)),
            "число 0"
        );
        assert_eq!(
            show(&ask("count(//*)", EnumValue::XPathBoolean)),
            "булево Boolean(false)"
        );
        assert_eq!(
            show(&ask("count(//*)", EnumValue::XPathString)),
            "строка []"
        );
        // Снимок от ЧИСЛА даёт пустой набор, а не ошибку (измерено).
        let snapshot = ask("count(//*)", EnumValue::XPathOrderedNodeSnapshot);
        assert_eq!(
            get_property(&snapshot, "РазмерСнимка").unwrap(),
            BslValue::Number(bsl_number::BslNumber::from_i64(0))
        );
        assert_eq!(
            snapshot_item(&snapshot, &[num(0)]).unwrap(),
            BslValue::Undefined
        );
        // Снимок узлов: длина настоящая (сознательное расхождение), номер
        // вне снимка — `Неопределено`.
        let nodes = ask("//к:товар", EnumValue::XPathOrderedNodeSnapshot);
        assert_eq!(
            get_property(&nodes, "РазмерСнимка").unwrap(),
            BslValue::Number(bsl_number::BslNumber::from_i64(2))
        );
        assert_eq!(
            describe(&snapshot_item(&nodes, &[num(1)]).unwrap()),
            "к:товар"
        );
        assert_eq!(
            snapshot_item(&nodes, &[num(2)]).unwrap(),
            BslValue::Undefined
        );
        assert_eq!(
            snapshot_item(&nodes, &[num(-1)]).unwrap(),
            BslValue::Undefined
        );
        // «Первый узел» и его английский член.
        let first = ask("//к:товар", EnumValue::XPathFirstOrderedNode);
        assert_eq!(
            describe(&get_property(&first, "SingleNodeValue").unwrap()),
            "к:товар"
        );
    }

    fn num(v: i64) -> BslValue {
        BslValue::Number(bsl_number::BslNumber::from_i64(v))
    }

    /// Члены разграничены по виду результата — измерено на платформе.
    #[test]
    fn members_belong_to_their_result_kind() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let nodes = evaluate(
            &doc,
            &[str_value("//к:товар"), doc.clone(), resolver.clone()],
        )
        .unwrap();
        assert!(get_property(&nodes, "ЧисловоеЗначение").is_err());
        assert!(get_property(&nodes, "РазмерСнимка").is_err());
        assert!(get_property(&nodes, "SingleNodeValue").is_err());
        assert!(snapshot_item(&nodes, &[num(0)]).is_err());
        let count = evaluate(&doc, &[str_value("count(//*)"), doc.clone(), resolver]).unwrap();
        assert!(get_property(&count, "СтроковоеЗначение").is_err());
        // А обход есть у всех видов: у числа он отдаёт `Неопределено`.
        assert_eq!(next_node(&count, &[]).unwrap(), BslValue::Undefined);
        assert_eq!(
            get_property(&count, "НеверноеСостояниеИтератора").unwrap(),
            BslValue::Boolean(false)
        );
    }

    /// Узлы возвращаются ТЕ ЖЕ, а не копии: тождество измерено на
    /// элементе, атрибуте, тексте, комментарии и самом документе.
    #[test]
    fn results_carry_the_very_same_nodes() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let root = dom::get_property(&doc, "ЭлементДокумента").unwrap();
        // Первый ребёнок корня — комментарий, второй — первый товар.
        let comment = dom::get_property(&root, "ПервыйДочерний").unwrap();
        let second = dom::get_property(&comment, "СледующийСоседний").unwrap();
        let found = evaluate(
            &doc,
            &[str_value("//к:товар"), doc.clone(), resolver.clone()],
        )
        .unwrap();
        assert_eq!(next_node(&found, &[]).unwrap(), second);
        let whole = evaluate(&doc, &[str_value("/"), doc.clone(), resolver]).unwrap();
        assert_eq!(next_node(&whole, &[]).unwrap(), doc);
    }

    /// Контекстные узлы: элемент, атрибут, комментарий и инструкция
    /// обработки — можно, текст — нельзя, чужой документ — нельзя.
    #[test]
    fn context_nodes_follow_the_measured_rules() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let root = dom::get_property(&doc, "ЭлементДокумента").unwrap();
        let version = dom::get_attribute_node(&root, &[str_value("версия")]).unwrap();
        let from_attr = evaluate(&doc, &[str_value("name(.)"), version, resolver.clone()]).unwrap();
        assert_eq!(show(&from_attr), "строка [версия]");
        let comment = dom::get_property(&root, "ПервыйДочерний").unwrap();
        let from_comment =
            evaluate(&doc, &[str_value("string(.)"), comment, resolver.clone()]).unwrap();
        assert_eq!(show(&from_comment), "строка [шапка]");
        let others = dom::get_property(&root, "ПоследнийДочерний").unwrap();
        let from_pi = evaluate(&doc, &[str_value("name(.)"), others, resolver.clone()]).unwrap();
        assert_eq!(show(&from_pi), "строка [пи]");

        // Текстовый узел контекстом СЛУЖИТ: измерено, что `name(.)` от
        // него — пустая строка, `string(.)` — его текст, а `name(..)` —
        // имя родителя.
        let tail = eval_node(&doc, "//*[local-name()='прочее']");
        let text = dom::get_property(&tail, "ПервыйДочерний").unwrap();
        let from_text = evaluate(
            &doc,
            &[str_value("name(..)"), text.clone(), resolver.clone()],
        )
        .unwrap();
        assert_eq!(show(&from_text), "строка [прочее]");
        let text_self = evaluate(&doc, &[str_value("string(.)"), text, resolver.clone()]).unwrap();
        assert_eq!(show(&text_self), "строка [хвост]");

        let other_doc = document_of("<а><б/></а>");
        let other_root = dom::get_property(&other_doc, "ЭлементДокумента").unwrap();
        assert!(
            evaluate(&doc, &[str_value("name(.)"), other_root, resolver]).is_err(),
            "узел чужого документа контекстом быть не может"
        );
    }

    fn eval_node(doc: &BslValue, expr: &str) -> BslValue {
        let resolver = resolver_of(doc);
        let result = evaluate(doc, &[str_value(expr), doc.clone(), resolver]).unwrap();
        next_node(&result, &[]).unwrap()
    }

    /// Выражение переиспользуется, а неизвестный префикс в нём ловится
    /// только при вычислении — измерено обе стороны.
    #[test]
    fn a_created_expression_is_reusable() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let expr = create_expression(&doc, &[str_value("count(//*)"), resolver.clone()]).unwrap();
        let first = evaluate_expression(&expr, std::slice::from_ref(&doc)).unwrap();
        assert_eq!(show(&first), "число 8");
        let товар = eval_node(&doc, "//к:товар");
        let second = evaluate_expression(&expr, &[товар]).unwrap();
        assert_eq!(show(&second), "число 8");

        assert!(
            create_expression(&doc, &[str_value("//[[["), resolver.clone()]).is_err(),
            "негодный синтаксис ловится при СОЗДАНИИ"
        );
        let late = create_expression(&doc, &[str_value("//нет:а"), resolver]).unwrap();
        assert!(
            evaluate_expression(&late, std::slice::from_ref(&doc)).is_err(),
            "неизвестный префикс ловится при ВЫЧИСЛЕНИИ"
        );
    }

    /// Итератор — снимок: добавленный после вычисления узел в него не
    /// попадает, а свежее вычисление его уже видит (измерено).
    #[test]
    fn an_iterator_does_not_see_later_changes() {
        let doc = sample();
        let resolver = resolver_of(&doc);
        let result = evaluate(
            &doc,
            &[str_value("//к:товар"), doc.clone(), resolver.clone()],
        )
        .unwrap();
        let first = next_node(&result, &[]).unwrap();
        assert_eq!(describe(&first), "к:товар");
        let root = dom::get_property(&doc, "ЭлементДокумента").unwrap();
        let fresh = dom::create_element(&doc, &[str_value("urn:к"), str_value("к:товар")]).unwrap();
        dom::append_child(&root, &[fresh]).unwrap();
        assert_eq!(describe(&next_node(&result, &[]).unwrap()), "к:товар");
        assert_eq!(next_node(&result, &[]).unwrap(), BslValue::Undefined);
        let again = evaluate(
            &doc,
            &[str_value("count(//к:товар)"), doc.clone(), resolver],
        )
        .unwrap();
        assert_eq!(show(&again), "число 3");
    }
}
