//! Поверхность BSL над движком регулярных выражений: четыре глобальные
//! функции и два объекта результата.
//!
//! Машинерия находится во внутреннем модуле `engine`; здесь только то, что видно из BSL:
//! `СтрНайтиПоРегулярномуВыражению` / `СтрНайтиВсеПоРегулярномуВыражению` /
//! `СтрЗаменитьПоРегулярномуВыражению` / `СтрПодобнаПоРегулярномуВыражению`,
//! объекты `РезультатПоискаПоРегулярномуВыражению` и
//! `ГруппаРезультатаПоискаПоРегулярномуВыражению`, а также грамматика
//! строки замены. Всё, что ниже, снято с 8.3.27 одноразовыми пробами; там,
//! где ответ платформы выглядит неожиданным, в комментарии стоит сам ответ.
//!
//! # Арности и синонимы
//!
//! Измерены перебором числа аргументов (граница видна по тому, на каком
//! их количестве платформа отвечает «Недостаточно фактических параметров»
//! и «Слишком много фактических параметров»):
//!
//! | функция | аргументы | английский синоним |
//! |---|---|---|
//! | `СтрНайтиПоРегулярномуВыражению` | 2..7 | `StrFindByRegularExpression` |
//! | `СтрНайтиВсеПоРегулярномуВыражению` | 2..4 | `StrFindAllByRegularExpression` |
//! | `СтрЗаменитьПоРегулярномуВыражению` | 3..5 | `StrReplaceByRegularExpression` |
//! | `СтрПодобнаПоРегулярномуВыражению` | 2..4 | `StrLikeByRegularExpression` |
//!
//! У поиска аргументы идут так: строка, шаблон, `НаправлениеПоиска`,
//! начальная позиция, номер вхождения, игнорировать регистр, многострочный
//! поиск. У остальных трёх последние два — третий-четвёртый (у замены —
//! четвёртый-пятый, третий занят строкой замены).
//!
//! Пропущенный необязательный аргумент здесь неотличим от явного
//! `Неопределено`: резолвер дополняет вызов до максимальной арности именно
//! им (`BuiltinFn::arity_range`), и рантайм видит уже дополненный список.
//! Платформа их различает — `СтрНайтиПоРегулярномуВыражению("абв", "б",
//! Неопределено)` она отвергает как «Несоответствие типов (параметр номер
//! '3')», а тот же вызов с пропущенным третьим аргументом принимает. Это
//! общее свойство всех встроенных функций этого дерева, а не решение,
//! принятое здесь.
//!
//! # Что возвращается
//!
//! `СтрНайтиПоРегулярномуВыражению` возвращает объект ВСЕГДА, а не
//! `Неопределено`: у ненайденного совпадения `НачальнаяПозиция` и `Длина`
//! равны нулю, `Значение` — пустая строка, `ПолучитьГруппы()` — пустой
//! массив. Свойства обоих объектов измерены поимённо: `Значение`/`Value`,
//! `НачальнаяПозиция`/`StartIndex`, `Длина`/`Length` — и только они
//! (`Индекс` и `Группы` платформа отвергает: «Поле объекта не обнаружено»).
//! Группы отдаёт метод `ПолучитьГруппы()`/`GetGroups()`, и НУЛЕВОЙ группы
//! в его массиве нет: у `б(в)` на «абвг» он длиной 1. Конструктора у обоих
//! типов нет («Конструктор не найден»), поэтому в `NEW_TYPES` они не
//! заводятся.
//!
//! Позиции — 1-базовые и считаются в КОД-ЮНИТАХ UTF-16, как и `СтрДлина`:
//! измерено, что `СтрДлина("а" + суррогатная пара + "б")` равна 4, а «б» в
//! той же строке находится на позиции 4. Внутри движок работает в кодовых
//! точках поверх тех же код-юнитов, так что пересчитывать нечего — только
//! прибавить единицу.
//!
//! # Порядок групп
//!
//! По ОТКРЫВАЮЩЕЙ скобке слева направо: `((а)(б))(в)` на «абв» даёт
//! «аб», «а», «б», «в» — сперва внешняя, потом её вложенные, потом
//! следующая. Незахватывающие `(?:…)` в список не попадают.
//! НЕУЧАСТВОВАВШАЯ группа отличается от участвовавшей и пустой не
//! значением, а позицией: `(а)?(б)` на «б» даёт группе 1 позицию 0 и длину
//! 0, а `(а*)(б)` на «вб» — позицию 2 и длину 0. Значение у обеих пустое.
//!
//! # Пустой шаблон и пустая строка
//!
//! Измеренная асимметрия, которую нельзя вывести из общих правил: поиск
//! (`СтрНайти…` и `СтрНайтиВсе…`) не находит НИЧЕГО, если пуст субъект или
//! пуст шаблон, — тогда как замена и проверка на подобие в тех же случаях
//! работают как обычно. `СтрНайтиВсеПоРегулярномуВыражению("аб", "")` —
//! пустой массив, а `СтрЗаменитьПоРегулярномуВыражению("аб", "", "-")` —
//! «-а-б-»; `СтрНайтиВсеПоРегулярномуВыражению("", "а*")` — пустой массив,
//! а `СтрПодобнаПоРегулярномуВыражению("", "а*")` — «Да». Шаблон, который
//! совпадает только с пустотой, но пустым НЕ ЗАПИСАН (`я{0}`, `()`,
//! `(?:я)*`), под эту отсечку не попадает: он даёт обычные три пустых
//! совпадения на «аб».
//!
//! # `СтрПодобнаПоРегулярномуВыражению` — совпадение ЦЕЛИКОМ
//!
//! Не частичное: «абвг» ~ `б` — «Нет», «абвг» ~ `абвг` — «Да», «абвг» ~
//! `аб` — «Нет». И не эмуляция через `^…$`: «аб» + перевод строки ~ `аб`
//! даёт «Нет», хотя `б$` в той же строке находится (у `$` хвостовой
//! перевод строки не считается). Отсюда отдельная операция `matches_full` в движке.
//!
//! # Направление поиска
//!
//! `СНачала` — обычный непересекающийся проход слева направо, тот же, что
//! у `СтрНайтиВсе…`: «аабаа» ~ `а+` даёт «аа» на 1 и «аа» на 4, и
//! `НомерВхождения` нумерует именно их.
//!
//! `СКонца` — НЕ зеркало этого прохода и не «последнее из его совпадений».
//! Он пробует позиции начала справа налево и берёт первую совпавшую:
//! «аабаа» ~ `а+` с `СКонца` даёт «а» на позиции 5, хотя проход слева
//! таким совпадением не кончается. Второе и далее вхождения ищутся левее и
//! не должны налезать на предыдущее — причём совпадение, выданное движком
//! на позиции, ПРОВЕРЯЕТСЯ целиком, а не подрезается: на «аабаа» вторым
//! вхождением идёт «а» на позиции 2, а не «а» на позиции 4, потому что на
//! позиции 4 жадный `а+` берёт «аа» и налезает на первое вхождение.
//! Проверено и на «ааа» ~ `аа` (`СКонца` даёт позицию 2, а второго
//! вхождения нет вовсе), и на «аааа» ~ `аа` (позиции 3, затем 1).
//!
//! `НачальнаяПозиция` ограничивает НАЧАЛО совпадения, но не его конец:
//! «ааа» ~ `аа` с `СКонца` и позицией 2 отдаёт совпадение 2..3.
//!
//! # Грамматика строки замены
//!
//! Ссылки на группы платформа поддерживает — вопреки тому, что можно было
//! ожидать по соседним функциям: `$0` — всё совпадение, `$1`..`$n` —
//! группы. Число цифр, забираемых после `$`, ограничено количеством цифр в
//! номере ПОСЛЕДНЕЙ группы, и это измерено обеими сторонами: при двух
//! группах `$01` даёт группу 0, а следом литеральную «1» («наб1квгд»), и
//! `$12` — группу 1, а следом литеральную «2» («на2квгд»), тогда как при
//! десяти группах `$10` — это группа 10. Жадный разбор «пока номер
//! остаётся допустимым» этими двумя ответами опровергнут.
//!
//! `$`, за которым не идёт цифра, — литеральный доллар (`${1}` так и
//! печатается, `$$` даёт `$$`). Обратная косая экранирует следующий символ
//! и сама пропадает: `\n` — это буква «n», `\\` — одна косая, `\$1` — текст
//! «$1»; хвостовая косая исчезает.
//!
//! # Ссылка на несуществующую группу
//!
//! Платформа НЕ бросает исключение: она возвращает то, что успела собрать,
//! и молча бросает остаток. «вавав» ~ `(а)` со строкой замены `н$5к` даёт
//! «вн» — текст до совпадения плюс литеральное начало замены, а хвост
//! исходной строки в результат уже не попадает. Это воспроизведено здесь
//! дословно (внутреннее состояние `Replacement::Aborted`).
//!
//! ЧЕГО ЗДЕСЬ СОЗНАТЕЛЬНО НЕТ — второй половины того же поведения.
//! Измерено, что на платформе такая ссылка ОТРАВЛЯЕТ шаблон до конца
//! сеанса: после `СтрЗаменитьПоРегулярномуВыражению("абвгд", "(а)(б)",
//! "н$7к")` тот же шаблон `(а)(б)` перестаёт работать вообще — годная
//! замена с ним отдаёт пустую строку, `СтрНайтиПоРегулярномуВыражению` с
//! ним отвечает позицией 0, а `СтрПодобнаПоРегулярномуВыражению` — «Нет»,
//! тогда как ЛЮБОЙ другой текст шаблона работает по-прежнему. Это порча
//! состояния в кэше скомпилированных шаблонов, а не семантика языка:
//! копировать её значило бы намеренно ломать собственные последующие
//! вызовы. Первую половину (обрыв сборки в пределах одного вызова)
//! копируем, вторую — нет, и это единственное сознательное расхождение
//! этого модуля с измеренным поведением.

use bsl_number::BslNumber;
use bsl_rt::{
    Arity, BslString, BslValue, CallContext, EnumValue, FunctionCode, FunctionDescriptor,
    FunctionKind, LibraryDescriptor, MethodDescriptor, ObjectProtocol, PropertyDescriptor, RtError,
    RtResult, TypeDescriptor,
};
use std::rc::Rc;

mod engine;
use engine::{Haystack, Regex};

/// Один промежуток исходной строки: то, что в BSL видно как объект с
/// тремя свойствами.
///
/// `start` — 1-БАЗОВАЯ позиция в код-юнитах, ноль означает «совпадения
/// нет» (у результата) или «группа не участвовала» (у группы). Именно так
/// отвечает платформа, и по этому нулю участие группы и отличается от
/// участия с пустым значением.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RegexSpan {
    value: BslString,
    start: usize,
    length: usize,
}

impl RegexSpan {
    /// Промежуток «ничего не найдено»: пустое значение, нулевые позиция и
    /// длина.
    fn empty() -> RegexSpan {
        RegexSpan {
            value: BslString::from_units(Vec::new()),
            start: 0,
            length: 0,
        }
    }

    fn of(hay: &[u16], span: Option<(usize, usize)>) -> RegexSpan {
        match span {
            Some((from, to)) => RegexSpan {
                value: BslString::from_units(hay[from..to].to_vec()),
                start: from + 1,
                length: to - from,
            },
            None => RegexSpan::empty(),
        }
    }
}

/// `РезультатПоискаПоРегулярномуВыражению` — снимок одного совпадения
/// вместе с его группами.
///
/// Снимок, а не окно в исходную строку: строки BSL неизменяемы, а объект
/// платформы сериализуется и переживает свою строку.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RegexMatchData {
    whole: RegexSpan,
    /// Группы БЕЗ нулевой — ровно то, что отдаёт `ПолучитьГруппы()`.
    groups: Vec<RegexSpan>,
}

impl RegexMatchData {
    fn not_found() -> RegexMatchData {
        RegexMatchData {
            whole: RegexSpan::empty(),
            groups: Vec::new(),
        }
    }

    fn from_match(hay: &[u16], found: &crate::engine::Match) -> RegexMatchData {
        let whole = RegexSpan::of(hay, found.spans.first().copied().flatten());
        let groups = found
            .spans
            .iter()
            .skip(1)
            .map(|span| RegexSpan::of(hay, *span))
            .collect();
        RegexMatchData { whole, groups }
    }
}

pub(crate) static MATCH_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "РезультатПоискаПоРегулярномуВыражению",
    type_display: "Результат поиска по регулярному выражению",
    type_names: &["ResultOfSearchByRegularExpression"],
};

pub(crate) static GROUP_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ГруппаРезультатаПоискаПоРегулярномуВыражению",
    type_display: "Группа результата поиска по регулярному выражению",
    type_names: &["ResultOfSearchByRegularExpressionGroup"],
};

/// Весь результат поиска. `ПолучитьГруппы()` — обычный метод таблицы
/// (`MATCH_METHODS`), а не разбор строки: с ABI-F у результата больше нет
/// переопределения `call_method` — единственного в боевом дереве.
#[derive(Debug)]
struct RegexMatchObject {
    data: Rc<RegexMatchData>,
}

/// Одна группа результата. Номер группы проверен ПРИ ПОСТРОЕНИИ (только
/// `match_get_groups` создаёт группу, и только по `0..groups.len()`),
/// поэтому окно всегда есть — «номер в диапазоне» стал инвариантом, а не
/// проверкой на каждом доступе.
#[derive(Debug)]
struct RegexGroupObject {
    data: Rc<RegexMatchData>,
    index: usize,
}

impl ObjectProtocol for RegexMatchObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &MATCH_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SPAN_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        MATCH_METHODS
    }
}

impl ObjectProtocol for RegexGroupObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &GROUP_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SPAN_PROPERTIES
    }
}

/// `РезультатПоискаПоРегулярномуВыражению.ПолучитьГруппы()` -> массив групп.
fn match_get_groups(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let result = receiver
        .downcast_ref::<RegexMatchObject>()
        .ok_or(RtError::NotAnObject)?;
    let items = (0..result.data.groups.len())
        .map(|index| {
            BslValue::new_object(RegexGroupObject {
                data: result.data.clone(),
                index,
            })
        })
        .collect();
    Ok(BslValue::new_array(items))
}

static MATCH_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["ПолучитьГруппы", "GetGroups"],
    Arity::exact(0),
    match_get_groups,
)];

/// Окно результата за получателем: и весь результат, и его группа делят
/// таблицу свойств, поэтому геттеры принимают любой из двух типов. У группы
/// номер проверен построением, так что окно всегда есть.
fn span_of(receiver: &dyn ObjectProtocol) -> RtResult<&RegexSpan> {
    if let Some(result) = receiver.downcast_ref::<RegexMatchObject>() {
        return Ok(&result.data.whole);
    }
    if let Some(group) = receiver.downcast_ref::<RegexGroupObject>() {
        return group
            .data
            .groups
            .get(group.index)
            .ok_or(RtError::NotAnObject);
    }
    Err(RtError::NotAnObject)
}

fn span_value(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(BslValue::Str(span_of(receiver)?.value.clone()))
}

fn span_start(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    Ok(BslValue::Number(BslNumber::from_i64(
        span_of(receiver)?.start as i64,
    )))
}

fn span_length(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Ok(BslValue::Number(BslNumber::from_i64(
        span_of(receiver)?.length as i64,
    )))
}

/// Три свойства результата и группы — и только они: `ПолучитьГруппы()`
/// метод, а нулевой группы среди групп нет (измерено, см. обзор модуля).
/// Все — только для чтения: платформа записи в них не знает.
static SPAN_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["Значение", "Value"],
        get: span_value,
        set: None,
    },
    PropertyDescriptor {
        names: &["НачальнаяПозиция", "StartIndex"],
        get: span_start,
        set: None,
    },
    PropertyDescriptor {
        names: &["Длина", "Length"],
        get: span_length,
        set: None,
    },
];

// --- разбор аргументов ---------------------------------------------------

/// Направление, в котором ищется совпадение.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    FromBegin,
    FromEnd,
}

/// Строковый аргумент. Платформа на числе и `Неопределено` отвечает
/// «Несоответствие типов (параметр номер 'N')» — здесь это `TypeError`.
fn text(args: &[BslValue], index: usize, op: &'static str) -> RtResult<BslString> {
    match args.get(index) {
        Some(BslValue::Str(s)) => Ok(s.clone()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Булев необязательный аргумент. `Неопределено` — это «аргумент не
/// задан»: резолвер дополняет вызов до максимальной арности именно им.
fn flag(args: &[BslValue], index: usize, op: &'static str) -> RtResult<bool> {
    match args.get(index) {
        None | Some(BslValue::Undefined) => Ok(false),
        Some(BslValue::Boolean(b)) => Ok(*b),
        _ => Err(RtError::TypeError {
            expected: "Булево",
            op,
        }),
    }
}

fn direction(args: &[BslValue], index: usize) -> RtResult<Direction> {
    match args.get(index) {
        None | Some(BslValue::Undefined) => Ok(Direction::FromBegin),
        Some(BslValue::Enum(EnumValue::SearchFromBegin)) => Ok(Direction::FromBegin),
        Some(BslValue::Enum(EnumValue::SearchFromEnd)) => Ok(Direction::FromEnd),
        _ => Err(RtError::TypeError {
            expected: "НаправлениеПоиска",
            op: "СтрНайтиПоРегулярномуВыражению",
        }),
    }
}

/// Числовой необязательный аргумент (позиция, номер вхождения) — ТОЛЬКО
/// проверка типа.
///
/// Диапазон проверяется отдельно и ПОЗЖЕ ([`in_range`]): измерено, что на
/// пустой строке платформа принимает и позицию 0, и позицию 2, отвечая
/// нулём, тогда как на непустой те же значения — «Недопустимое значение
/// параметра». Значит, отсечка по пустоте стоит между проверкой типов и
/// проверкой диапазона, и порядок здесь повторяет измеренный.
///
/// `Неопределено` — аргумент не задан: резолвер дополняет вызов до
/// максимальной арности именно им.
fn number(args: &[BslValue], index: usize) -> RtResult<Option<BslNumber>> {
    match args.get(index) {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Number(n)) => Ok(Some(n.clone())),
        _ => Err(RtError::TypeError {
            expected: "Число",
            op: "СтрНайтиПоРегулярномуВыражению",
        }),
    }
}

/// Целое от единицы до `top` включительно.
///
/// Дробное значение УСЕКАЕТСЯ, а не отвергается: измерено, что позиция
/// 1,5 понимается платформой как 1, и номер вхождения 1,5 — тоже как 1.
///
/// # Errors
///
/// [`RtError::Regex`] с номером параметра в тексте — так же, как отвечает
/// платформа («Недопустимое значение параметра (параметр номер '4')»).
fn in_range(value: Option<&BslNumber>, top: usize, parameter: u32) -> RtResult<Option<usize>> {
    let Some(value) = value else { return Ok(None) };
    value
        .trunc_to_scale(0)
        .to_i64_exact()
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| (1..=top).contains(v))
        .map(Some)
        .ok_or_else(|| RtError::Regex(format!("недопустимое значение параметра номер {parameter}")))
}

// --- проход по совпадениям ------------------------------------------------

/// Непересекающийся проход слева направо, начиная с `from` (0-базовая
/// позиция в код-юнитах).
///
/// Правило продвижения измерено на пустых совпадениях: «баб» ~ `а*` даёт
/// четыре совпадения — на позициях 1, 2, 3 и 4, то есть после пустого
/// совпадения проход сдвигается на ОДНУ КОДОВУЮ ТОЧКУ, а не на код-юнит.
/// Проверено ровно там, где это различимо: у «а» + суррогатная пара + «б»
/// пустые совпадения стоят на 1, 2, 4 и 5 — позиции 3, середины пары,
/// среди них нет.
fn scan(
    re: &Regex,
    units: &[u16],
    hay: &Haystack,
    from: usize,
) -> RtResult<Vec<crate::engine::Match>> {
    let mut found = Vec::new();
    let mut at = from;
    while at <= units.len() {
        let Some(m) = re.find_at(hay, at)? else { break };
        let Some((start, end)) = m.spans.first().copied().flatten() else {
            break;
        };
        found.push(m);
        at = if end > start {
            end
        } else {
            match crate::engine::cp_at(units, start) {
                Some((_, width)) => start + width,
                None => start + 1,
            }
        };
    }
    Ok(found)
}

/// Проход справа налево: позиции начала перебираются от `from` вниз, и
/// каждое найденное на позиции совпадение принимается целиком — либо
/// отвергается, если налезает на предыдущее (см. шапку модуля, «аабаа»).
///
/// Останавливается, как только набрано `needed` вхождений.
fn scan_back(
    re: &Regex,
    units: &[u16],
    hay: &Haystack,
    from: usize,
    needed: usize,
) -> RtResult<Vec<crate::engine::Match>> {
    let mut found = Vec::new();
    let mut limit: Option<usize> = None;
    let mut at = from;
    loop {
        if found.len() >= needed {
            break;
        }
        if let Some(m) = re.match_at(hay, at)?
            && let Some((start, end)) = m.spans.first().copied().flatten()
            && limit.is_none_or(|bound| end <= bound)
        {
            limit = Some(start);
            found.push(m);
        }
        match crate::engine::cp_before(units, at) {
            Some((_, width)) => at -= width,
            None => break,
        }
    }
    Ok(found)
}

// --- четыре функции --------------------------------------------------------

fn match_value(data: RegexMatchData) -> BslValue {
    BslValue::new_object(RegexMatchObject {
        data: Rc::new(data),
    })
}

/// `СтрНайтиПоРегулярномуВыражению(<Строка>, <РегулярноеВыражение>,
/// <НаправлениеПоиска>, <НачальнаяПозиция>, <НомерВхождения>,
/// <ИгнорироватьРегистр>, <МногострочныйПоиск>)`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументе не того типа, [`RtError::Regex`] на
/// неразбираемом шаблоне и на позиции или номере вхождения вне
/// допустимого диапазона.
pub fn find(args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "СтрНайтиПоРегулярномуВыражению";
    let subject = text(args, 0, OP)?;
    let pattern = text(args, 1, OP)?;
    let hay = subject.units();
    let direction = direction(args, 2)?;
    let position = number(args, 3)?;
    let occurrence = number(args, 4)?;
    let icase = flag(args, 5, OP)?;
    let multiline = flag(args, 6, OP)?;
    // Отсечка стоит ДО разбора шаблона: измерено, что
    // `СтрНайтиПоРегулярномуВыражению("", "(")` отвечает объектом-промахом,
    // а не ошибкой, — тогда как у замены и подобия, отсечки не имеющих, тот
    // же шаблон на той же пустой строке даёт исключение. И до проверки
    // диапазона: на пустой строке принимаются и позиция 0, и позиция 2.
    if hay.is_empty() || pattern.units().is_empty() {
        return Ok(match_value(RegexMatchData::not_found()));
    }
    let re = Regex::parse_with(pattern.units(), icase, multiline)?;
    let occurrence = in_range(occurrence.as_ref(), usize::MAX, 5)?.unwrap_or(1);
    let start = match in_range(position.as_ref(), hay.len(), 4)? {
        Some(p) => p - 1,
        None => match direction {
            // Умолчание ИЗМЕРЕНО и у направлений оно разное: слева это
            // первый символ, справа — последний.
            Direction::FromBegin => 0,
            Direction::FromEnd => hay.len() - 1,
        },
    };
    let haystack = Haystack::new(hay);
    let found = match direction {
        Direction::FromBegin => scan(&re, hay, &haystack, start)?
            .into_iter()
            .nth(occurrence - 1),
        Direction::FromEnd => scan_back(&re, hay, &haystack, start, occurrence)?
            .into_iter()
            .nth(occurrence - 1),
    };
    Ok(match_value(match found {
        Some(m) => RegexMatchData::from_match(hay, &m),
        None => RegexMatchData::not_found(),
    }))
}

/// `СтрНайтиВсеПоРегулярномуВыражению(<Строка>, <РегулярноеВыражение>,
/// <ИгнорироватьРегистр>, <МногострочныйПоиск>)` -> `Массив`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументе не того типа, [`RtError::Regex`] на
/// неразбираемом шаблоне.
pub fn find_all(args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "СтрНайтиВсеПоРегулярномуВыражению";
    let subject = text(args, 0, OP)?;
    let pattern = text(args, 1, OP)?;
    let icase = flag(args, 2, OP)?;
    let multiline = flag(args, 3, OP)?;
    let hay = subject.units();
    // Та же отсечка и тот же порядок, что у `find`: измерено, что
    // `СтрНайтиВсеПоРегулярномуВыражению("", "(")` отдаёт пустой массив, а
    // не ошибку разбора.
    if hay.is_empty() || pattern.units().is_empty() {
        return Ok(BslValue::new_array(Vec::new()));
    }
    let re = Regex::parse_with(pattern.units(), icase, multiline)?;
    let haystack = Haystack::new(hay);
    let items = scan(&re, hay, &haystack, 0)?
        .iter()
        .map(|m| match_value(RegexMatchData::from_match(hay, m)))
        .collect();
    Ok(BslValue::new_array(items))
}

/// `СтрПодобнаПоРегулярномуВыражению(<Строка>, <РегулярноеВыражение>,
/// <ИгнорироватьРегистр>, <МногострочныйПоиск>)` -> `Булево`.
///
/// Совпадение ЦЕЛИКОМ, не частичное (см. шапку модуля).
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументе не того типа, [`RtError::Regex`] на
/// неразбираемом шаблоне.
pub fn like(args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "СтрПодобнаПоРегулярномуВыражению";
    let subject = text(args, 0, OP)?;
    let pattern = text(args, 1, OP)?;
    let icase = flag(args, 2, OP)?;
    let multiline = flag(args, 3, OP)?;
    let re = Regex::parse_full_with(pattern.units(), icase, multiline)?;
    let haystack = Haystack::new(subject.units());
    Ok(BslValue::Boolean(re.matches_full(&haystack)?))
}

/// Чем кончилась сборка одной строки замены.
enum Replacement {
    Done(Vec<u16>),
    /// Ссылка на несуществующую группу: платформа отдаёт собранное и
    /// молча бросает остаток (см. шапку модуля).
    Aborted(Vec<u16>),
}

/// Сколько цифр забирать после `$`: столько же, сколько их в номере
/// последней группы.
///
/// Измерено с двух сторон: при двух группах `$01` — это группа 0 и
/// литеральная «1», при десяти `$10` — группа 10.
fn max_capture_digits(groups: usize) -> usize {
    let mut digits = 1;
    let mut bound = 10;
    while groups >= bound {
        digits += 1;
        bound *= 10;
    }
    digits
}

fn expand(replacement: &[u16], hay: &[u16], found: &crate::engine::Match) -> Replacement {
    let groups = found.spans.len().saturating_sub(1);
    let max_digits = max_capture_digits(groups);
    let mut out: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < replacement.len() {
        let unit = replacement[i];
        if unit == u16::from(b'\\') {
            // Экранирование: сама косая пропадает, следующий код-юнит идёт
            // как есть. Хвостовая косая исчезает вместе с концом строки.
            if let Some(next) = replacement.get(i + 1) {
                out.push(*next);
            }
            i += 2;
            continue;
        }
        if unit != u16::from(b'$') {
            out.push(unit);
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut digits = 0;
        let mut group = 0usize;
        while digits < max_digits {
            // Цифра — любая десятичная, не только ASCII: измерено, что
            // `$` + арабо-индийская единица U+0661 при двух группах
            // подставляет первую группу, а `$` + римская U+2170 остаётся
            // литеральным долларом.
            let Some((cp, width)) = crate::engine::cp_at(replacement, j) else {
                break;
            };
            let Some(digit) = crate::engine::decimal_digit_value(cp) else {
                break;
            };
            group = group * 10 + digit as usize;
            digits += 1;
            j += width;
        }
        if digits == 0 {
            // `$` без цифры — литеральный доллар: `${1}` печатается как
            // есть, `$$` даёт `$$`.
            out.push(unit);
            i += 1;
            continue;
        }
        if group > groups {
            return Replacement::Aborted(out);
        }
        if let Some((from, to)) = found.spans.get(group).copied().flatten() {
            out.extend_from_slice(&hay[from..to]);
        }
        i = j;
    }
    Replacement::Done(out)
}

/// `СтрЗаменитьПоРегулярномуВыражению(<Строка>, <РегулярноеВыражение>,
/// <ПодстрокаЗамены>, <ИгнорироватьРегистр>, <МногострочныйПоиск>)`.
///
/// # Errors
///
/// [`RtError::TypeError`] на аргументе не того типа, [`RtError::Regex`] на
/// неразбираемом шаблоне.
pub fn replace(args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "СтрЗаменитьПоРегулярномуВыражению";
    let subject = text(args, 0, OP)?;
    let pattern = text(args, 1, OP)?;
    let replacement = text(args, 2, OP)?;
    let icase = flag(args, 3, OP)?;
    let multiline = flag(args, 4, OP)?;
    let re = Regex::parse_with(pattern.units(), icase, multiline)?;
    let hay = subject.units();
    // Отсечки на пустую строку и пустой шаблон здесь НЕТ — измерено, что
    // замена в обоих случаях работает: «аб» ~ `` даёт «-а-б-».
    let haystack = Haystack::new(hay);
    let mut out: Vec<u16> = Vec::new();
    let mut last = 0usize;
    for m in scan(&re, hay, &haystack, 0)? {
        let Some((start, end)) = m.spans.first().copied().flatten() else {
            continue;
        };
        out.extend_from_slice(&hay[last..start]);
        match expand(replacement.units(), hay, &m) {
            Replacement::Done(text) => out.extend_from_slice(&text),
            Replacement::Aborted(text) => {
                // Хвост исходной строки НЕ дописывается — так отвечает
                // платформа.
                out.extend_from_slice(&text);
                return Ok(BslValue::Str(BslString::from_units(out)));
            }
        }
        last = end;
    }
    out.extend_from_slice(&hay[last..]);
    Ok(BslValue::Str(BslString::from_units(out)))
}

fn call_find(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    find(arguments)
}

fn call_find_all(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    find_all(arguments)
}

fn call_replace(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    replace(arguments)
}

fn call_like(_context: &mut CallContext<'_>, arguments: &[BslValue]) -> RtResult<BslValue> {
    like(arguments)
}

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &[
            "СтрНайтиПоРегулярномуВыражению",
            "StrFindByRegularExpression",
        ],
        arity: Arity::range(2, 7),
        kind: FunctionKind::Function,
        call: call_find,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &[
            "СтрНайтиВсеПоРегулярномуВыражению",
            "StrFindAllByRegularExpression",
        ],
        arity: Arity::range(2, 4),
        kind: FunctionKind::Function,
        call: call_find_all,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &[
            "СтрЗаменитьПоРегулярномуВыражению",
            "StrReplaceByRegularExpression",
        ],
        arity: Arity::range(3, 5),
        kind: FunctionKind::Function,
        call: call_replace,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &[
            "СтрПодобнаПоРегулярномуВыражению",
            "StrLikeByRegularExpression",
        ],
        arity: Arity::range(2, 4),
        kind: FunctionKind::Function,
        call: call_like,
    },
];

/// Типы, которые компонент вводит в язык: по ним работает `Тип("Имя")`.
const TYPES: &[&TypeDescriptor] = &[&crate::GROUP_TYPE, &crate::MATCH_TYPE];

/// Дескриптор статически подключаемого regex-компонента.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        bsl_rt::ObjectContextNeed::Reduced,
    )
    .with_functions(FUNCTIONS)
    .with_types(TYPES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_rt::{NameInterner, RuntimeShapes};

    fn s(text: &str) -> BslValue {
        BslValue::Str(BslString::from_str(text))
    }

    /// Дополнение вызова до максимальной арности `Неопределено` — ровно
    /// то, что делает резолвер перед вызовом встроенной функции.
    fn padded(args: &[BslValue], max: usize) -> Vec<BslValue> {
        let mut out = args.to_vec();
        out.resize(max, BslValue::Undefined);
        out
    }

    fn found(subject: &str, pattern: &str) -> BslValue {
        find(&padded(&[s(subject), s(pattern)], 7)).unwrap()
    }

    fn get_property(value: &BslValue, name: &str) -> RtResult<BslValue> {
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // Нативный контекст: свойствам regex ни потоки, ни зона не нужны.
        let mut context = CallContext::native(&mut shapes, |_value, _spec| {
            unreachable!("форматирование в regex-свойствах не используется")
        });
        value
            .object_ref()
            .ok_or(RtError::NotAnObject)?
            .get_property(name, &mut context)
    }

    fn get_groups(value: &BslValue) -> RtResult<BslValue> {
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // Нативный контекст: методу `ПолучитьГруппы` ни потоки, ни зона не нужны.
        let mut context = CallContext::native(&mut shapes, |_value, _spec| {
            unreachable!("форматирование в regex-методе не используется")
        });
        value.object_ref().ok_or(RtError::NotAnObject)?.call_method(
            "ПолучитьГруппы",
            &[],
            &mut context,
        )
    }

    /// `(значение, начальная позиция, длина)` — три свойства объекта
    /// одним кортежем.
    fn triple(value: &BslValue) -> (String, i64, i64) {
        let text = match get_property(value, "Значение").unwrap() {
            BslValue::Str(s) => s.to_string(),
            other => panic!("Значение не строка: {other:?}"),
        };
        let number = |name: &str| match get_property(value, name).unwrap() {
            BslValue::Number(n) => n.to_i64_exact().unwrap(),
            other => panic!("{name} не число: {other:?}"),
        };
        (text, number("НачальнаяПозиция"), number("Длина"))
    }

    fn items(array: &BslValue) -> Vec<BslValue> {
        let names = NameInterner::new();
        (0..array.collection_len().expect("ожидался массив"))
            .map(|index| {
                array
                    .get_index(&BslValue::Number(BslNumber::from_i64(index as i64)), &names)
                    .unwrap()
            })
            .collect()
    }

    fn groups(value: &BslValue) -> Vec<(String, i64, i64)> {
        let groups = get_groups(value).unwrap();
        items(&groups).iter().map(triple).collect()
    }

    fn replaced(subject: &str, pattern: &str, replacement: &str) -> String {
        match replace(&padded(&[s(subject), s(pattern), s(replacement)], 5)).unwrap() {
            BslValue::Str(s) => s.to_string(),
            other => panic!("замена вернула не строку: {other:?}"),
        }
    }

    fn matches(subject: &str, pattern: &str) -> bool {
        match like(&padded(&[s(subject), s(pattern)], 4)).unwrap() {
            BslValue::Boolean(b) => b,
            other => panic!("подобие вернуло не булево: {other:?}"),
        }
    }

    fn all(subject: &str, pattern: &str) -> Vec<(String, i64, i64)> {
        items(&find_all(&padded(&[s(subject), s(pattern)], 4)).unwrap())
            .iter()
            .map(triple)
            .collect()
    }

    #[test]
    fn a_miss_is_an_object_with_zero_position_not_undefined() {
        let miss = found("абвг", "яя");
        assert_eq!(triple(&miss), (String::new(), 0, 0));
        assert!(groups(&miss).is_empty());
        assert_eq!(
            miss.type_of().unwrap(),
            BslValue::Type(bsl_rt::TypeRef::Object(&MATCH_TYPE))
        );
    }

    #[test]
    fn groups_go_by_opening_parenthesis_and_skip_the_whole_match() {
        let m = found("абв", "((а)(б))(в)");
        assert_eq!(triple(&m), ("абв".to_string(), 1, 3));
        assert_eq!(
            groups(&m),
            vec![
                ("аб".to_string(), 1, 2),
                ("а".to_string(), 1, 1),
                ("б".to_string(), 2, 1),
                ("в".to_string(), 3, 1),
            ]
        );
        // Незахватывающая группа в список не попадает.
        assert_eq!(groups(&found("абв", "(?:а)(б)")).len(), 1);
    }

    #[test]
    fn a_group_that_did_not_take_part_differs_from_an_empty_one_by_position() {
        assert_eq!(groups(&found("б", "(а)?(б)"))[0], (String::new(), 0, 0));
        assert_eq!(groups(&found("вб", "(а*)(б)"))[0], (String::new(), 2, 0));
    }

    #[test]
    fn a_group_object_is_of_its_own_type() {
        let array = get_groups(&found("абвг", "б(в)")).unwrap();
        let item = items(&array).remove(0);
        assert_eq!(
            item.type_of().unwrap(),
            BslValue::Type(bsl_rt::TypeRef::Object(&GROUP_TYPE))
        );
        // Метода `ПолучитьГруппы` у самой группы нет.
        assert!(get_groups(&item).is_err());
    }

    #[test]
    fn only_three_properties_exist_on_both_objects() {
        let m = found("абвг", "б(в)");
        for name in [
            "Значение",
            "Value",
            "НачальнаяПозиция",
            "StartIndex",
            "Длина",
            "Length",
        ] {
            assert!(get_property(&m, name).is_ok(), "нет свойства {name}");
        }
        // Ни `Индекс`, ни `Группы` платформа не знает — и здесь их нет.
        assert!(get_property(&m, "Индекс").is_err());
        assert!(get_property(&m, "Группы").is_err());
    }

    #[test]
    fn a_bad_pattern_is_a_catchable_error_from_every_one_of_the_four() {
        let bad = "(";
        assert!(matches!(
            find(&padded(&[s("абв"), s(bad)], 7)),
            Err(RtError::Regex(_))
        ));
        assert!(matches!(
            find_all(&padded(&[s("абв"), s(bad)], 4)),
            Err(RtError::Regex(_))
        ));
        assert!(matches!(
            like(&padded(&[s("абв"), s(bad)], 4)),
            Err(RtError::Regex(_))
        ));
        assert!(matches!(
            replace(&padded(&[s("абв"), s(bad), s("Х")], 5)),
            Err(RtError::Regex(_))
        ));
    }

    #[test]
    fn the_empty_cutoff_comes_before_the_pattern_is_even_parsed() {
        // У поиска отсечка раньше разбора: негодный шаблон на пустой
        // строке не ошибка, а промах.
        let bad = "(";
        assert_eq!(
            triple(&find(&padded(&[s(""), s(bad)], 7)).unwrap()),
            (String::new(), 0, 0)
        );
        assert!(all("", bad).is_empty());
        // У замены и подобия отсечки нет — там тот же шаблон бросает.
        assert!(matches!(
            like(&padded(&[s(""), s(bad)], 4)),
            Err(RtError::Regex(_))
        ));
        assert!(matches!(
            replace(&padded(&[s(""), s(bad), s("-")], 5)),
            Err(RtError::Regex(_))
        ));
    }

    #[test]
    fn a_fractional_position_is_truncated_rather_than_rejected() {
        let at = |position: &str| {
            find(&[
                s("абаб"),
                s("а"),
                BslValue::Undefined,
                BslValue::Number(BslNumber::parse_canonical(position).unwrap()),
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
            ])
            .unwrap()
        };
        assert_eq!(triple(&at("1.5")), ("а".to_string(), 1, 1));
        assert_eq!(triple(&at("3.9")), ("а".to_string(), 3, 1));
    }

    #[test]
    fn a_digit_after_the_dollar_may_be_any_decimal_one() {
        // Арабо-индийская единица U+0661 — цифра, римская U+2170 — нет.
        assert_eq!(replaced("абвгд", "(а)(б)", "н$\u{661}к"), "наквгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н$\u{2170}к"), "н$\u{2170}квгд");
    }

    #[test]
    fn a_non_string_argument_is_a_type_error() {
        let number = BslValue::Number(BslNumber::from_i64(123));
        assert!(matches!(
            find(&padded(&[number.clone(), s("2")], 7)),
            Err(RtError::TypeError { .. })
        ));
        assert!(matches!(
            find(&padded(&[BslValue::Undefined, s("2")], 7)),
            Err(RtError::TypeError { .. })
        ));
        // Не тот тип у необязательного аргумента — тоже ошибка, а не
        // молчаливое приведение.
        assert!(matches!(
            find(&[
                s("абв"),
                s("б"),
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
                number,
                BslValue::Undefined
            ]),
            Err(RtError::TypeError { .. })
        ));
    }

    #[test]
    fn a_position_outside_the_string_is_rejected_but_not_on_an_empty_one() {
        let at = |subject: &str, position: i64| {
            find(&[
                s(subject),
                s("а"),
                BslValue::Undefined,
                BslValue::Number(BslNumber::from_i64(position)),
                BslValue::Undefined,
                BslValue::Undefined,
                BslValue::Undefined,
            ])
        };
        assert!(at("абаб", 4).is_ok());
        assert!(matches!(at("абаб", 5), Err(RtError::Regex(_))));
        assert!(matches!(at("абаб", 0), Err(RtError::Regex(_))));
        // На пустой строке отсечка срабатывает РАНЬШЕ проверки диапазона:
        // платформа отвечает нулём, а не ошибкой.
        assert_eq!(triple(&at("", 2).unwrap()), (String::new(), 0, 0));
        assert_eq!(triple(&at("", 0).unwrap()), (String::new(), 0, 0));
    }

    #[test]
    fn search_from_the_end_takes_the_rightmost_start_and_does_not_overlap() {
        let back = |subject: &str, pattern: &str, occurrence: i64| {
            let value = find(&[
                s(subject),
                s(pattern),
                BslValue::Enum(EnumValue::SearchFromEnd),
                BslValue::Undefined,
                BslValue::Number(BslNumber::from_i64(occurrence)),
                BslValue::Undefined,
                BslValue::Undefined,
            ])
            .unwrap();
            triple(&value)
        };
        // Жадный квантор справа берёт КОРОТКОЕ совпадение на самой правой
        // позиции, а не длинное из прохода слева.
        assert_eq!(back("аабаа", "а+", 1), ("а".to_string(), 5, 1));
        // Второе вхождение не налезает на первое, и подрезки нет: позиция
        // 4 отвергнута целиком.
        assert_eq!(back("аабаа", "а+", 2), ("а".to_string(), 2, 1));
        assert_eq!(back("ааа", "аа", 1), ("аа".to_string(), 2, 2));
        assert_eq!(back("ааа", "аа", 2), (String::new(), 0, 0));
        assert_eq!(back("аааа", "аа", 2), ("аа".to_string(), 1, 2));
    }

    #[test]
    fn an_empty_pattern_or_an_empty_subject_finds_nothing_but_still_replaces() {
        assert_eq!(triple(&found("аб", "")), (String::new(), 0, 0));
        assert_eq!(triple(&found("", "а*")), (String::new(), 0, 0));
        assert!(all("аб", "").is_empty());
        assert!(all("", "а*").is_empty());
        // Замена и подобие той же отсечки не знают.
        assert_eq!(replaced("аб", "", "-"), "-а-б-");
        assert_eq!(replaced("", "а*", "-"), "-");
        assert!(matches("", "а*"));
        assert!(matches("", ""));
        // Шаблон, совпадающий только с пустотой, но пустым не записанный,
        // под отсечку не попадает.
        assert_eq!(all("аб", "я{0}").len(), 3);
    }

    #[test]
    fn like_wants_the_whole_string_and_backtracks_to_get_it() {
        assert!(!matches("абвг", "б"));
        assert!(!matches("абвг", "аб"));
        assert!(matches("абвг", "абвг"));
        // Первая по приоритету ветвь строку целиком не съедает — перебор
        // обязан продолжиться.
        assert!(matches("аб", "а|аб"));
        assert!(matches("ааа", "а*а"));
        // Хвостовой перевод строки в совпадение входит, поэтому `^…$` тут
        // не эквивалент.
        assert!(!matches("аб\n", "аб"));
    }

    #[test]
    fn empty_matches_step_by_one_code_point() {
        assert_eq!(
            all("баб", "а*"),
            vec![
                (String::new(), 1, 0),
                ("а".to_string(), 2, 1),
                (String::new(), 3, 0),
                (String::new(), 4, 0),
            ]
        );
        // Суррогатная пара — одна кодовая точка: позиции 3, середины пары,
        // среди пустых совпадений нет.
        assert_eq!(
            all("а\u{1F600}б", "я*")
                .iter()
                .map(|(_, start, _)| *start)
                .collect::<Vec<_>>(),
            vec![1, 2, 4, 5]
        );
    }

    #[test]
    fn a_replacement_takes_group_references_with_a_bounded_number_of_digits() {
        assert_eq!(replaced("абв", "(а)(б)", "[$1]"), "[а]в");
        assert_eq!(replaced("абв", "(а)(б)", "[$0]"), "[аб]в");
        assert_eq!(replaced("абв", "(а)(б)", "$2$1"), "бав");
        // При двух группах после `$` берётся ОДНА цифра: `$01` — это
        // группа 0 и литеральная единица, `$12` — группа 1 и двойка.
        assert_eq!(replaced("абвгд", "(а)(б)", "н$01к"), "наб1квгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н$12к"), "на2квгд");
        // При десяти группах — две цифры, и `$10` становится группой 10.
        assert_eq!(
            replaced("абвгдежзик", "(а)(б)(в)(г)(д)(е)(ж)(з)(и)(к)", "н$10к"),
            "нкк"
        );
        // Группа без совпадения подставляется пустотой.
        assert_eq!(replaced("б", "(а)?(б)", "[$1][$2]"), "[][б]");
    }

    #[test]
    fn a_dollar_without_a_digit_is_literal_and_a_backslash_escapes() {
        assert_eq!(replaced("абвгд", "(а)(б)", "н${1}к"), "н${1}квгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н$$к"), "н$$квгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "нк$"), "нк$вгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н$хк"), "н$хквгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н\\хк"), "нхквгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н\\\\к"), "н\\квгд");
        assert_eq!(replaced("абвгд", "(а)(б)", "н\\$1к"), "н$1квгд");
        // Хвостовая косая исчезает вместе с концом строки замены.
        assert_eq!(replaced("абвгд", "(а)(б)", "нк\\"), "нквгд");
    }

    #[test]
    fn a_reference_to_a_missing_group_truncates_instead_of_raising() {
        // Текст ДО совпадения попадает в результат, литеральное начало
        // замены — тоже, а хвост исходной строки уже нет.
        assert_eq!(replaced("вавав", "(а)", "н$5к"), "вн");
        assert_eq!(replaced("абвгд", "(а)(б)", "н$3к"), "н");
        assert_eq!(replaced("абвгд", "аб", "н$1к"), "н");
        // `$0` есть даже там, где групп нет вовсе.
        assert_eq!(replaced("абвгд", "аб", "н$0к"), "набквгд");
    }

    #[test]
    fn flags_come_both_from_arguments_and_from_the_pattern() {
        let with = |subject: &str, pattern: &str, icase: bool, multiline: bool| match like(&[
            s(subject),
            s(pattern),
            BslValue::Boolean(icase),
            BslValue::Boolean(multiline),
        ])
        .unwrap()
        {
            BslValue::Boolean(b) => b,
            other => panic!("не булево: {other:?}"),
        };
        assert!(with("АБВ", "абв", true, false));
        assert!(!with("АБВ", "абв", false, false));
        assert!(matches("АБВ", "(?i)абв"));
        // Многострочность аргументом и флагом — одно и то же.
        assert_eq!(all("аб\nаг", "^а").len(), 1);
        assert_eq!(all("аб\nаг", "(?m)^а").len(), 2);
        let array = find_all(&[
            s("аб\nаг"),
            s("^а"),
            BslValue::Boolean(false),
            BslValue::Boolean(true),
        ])
        .unwrap();
        assert_eq!(items(&array).len(), 2);
    }
}
