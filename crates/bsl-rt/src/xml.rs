//! XML: потоковое чтение и запись.
//!
//! Устроено как `json.rs`: разборщик отдаёт события по одному, `ЧтениеXML`
//! только показывает наружу текущее, а `ЗаписьXML` копит текст и отдаёт его
//! на `Закрыть()`. Второй реализации разбора быть не должно.
//!
//! # Что здесь ИЗМЕРЕНО на 8.3.27
//!
//! Всё перечисленное снято пробами (`tests/conformance/measure/measure-xml.bsl`),
//! а не выведено из спецификации XML — платформа заметно от неё отходит:
//!
//! * узлов ОБЪЯВЛЕНИЯ и КОММЕНТАРИЯ читатель НЕ отдаёт вовсе, хотя члены
//!   `ТипУзлаXML` для них есть; инструкция обработки, наоборот, отдаётся;
//! * секция CDATA не отдельный узел, а часть СОСЕДНЕГО текста:
//!   `<а>раз<![CDATA[два]]>три</а>` — ОДИН узел «раздватри». Комментарий же
//!   текст РАЗРЫВАЕТ: `<а>раз<!--к-->два</а>` — два узла;
//! * текстовый узел целиком из пробелов выбрасывается, но пробел ВОКРУГ
//!   значащего текста сохраняется (`<а> т </а>` — узел « т »);
//! * у текстового узла `Имя` — `#text`;
//! * `<а/>` и `<а></а>` неразличимы: оба дают начало и конец элемента;
//! * битый ввод — ошибка, и пустая строка тоже: документа без корневого
//!   элемента не бывает;
//! * `Пропустить` оставляет читатель НА закрывающем теге; на нетекстовом
//!   узле пропускается остаток РОДИТЕЛЯ;
//! * запись по умолчанию с отступом в ОДИН ТАБ на уровень, но текст
//!   переводов строки вокруг себя не получает, а закрывающий тег встаёт с
//!   новой строки, только если последним в элементе шёл не текст;
//! * `ЗаписатьТекст("")` не делает ничего — элемент остаётся пустым (`<а/>`);
//! * экранируются при записи `&`, `<`, `>`, а в значении атрибута ещё и
//!   `"`; апостроф не экранируется НИГДЕ, табуляция и перевод строки уходят
//!   в атрибут как есть;
//! * `ЗаписатьТекст`/`ЗаписатьАтрибут` принимают ТОЛЬКО строку: число —
//!   ошибка.
//!
//! # Объявление типа документа и сущности
//!
//! Снято скриптом `tests/conformance/measure/measure-xml-dtd.bsl`; узлом
//! объявление по-прежнему не отдаётся, но пройти его надо целиком:
//!
//! * внутреннее подмножество разбирается, а не пропускается до первого `>`:
//!   ни `>`, ни `]>` внутри литерала, комментария или инструкции обработки
//!   его не закрывают. `SYSTEM` берёт один литерал (в кавычках или
//!   апострофах), `PUBLIC` — ДВА, и кириллица в публичном идентификаторе
//!   недопустима, тогда как тот же идентификатор латиницей проходит;
//! * подмножество проверяется, а не проглатывается: `<!ЧУШЬ>`, строчные
//!   `<!doctype` и `<!entity`, текст вместо объявления, условная секция
//!   `<![INCLUDE[…]]>`, `<!ENTITY е>` без значения, `<!ATTLIST>` и
//!   `<!ELEMENT к>` без модели — всё ошибки. Имя в объявлении с корневым
//!   элементом при этом НЕ сверяется, а второе объявление, объявление
//!   внутри элемента и после корня — ошибки;
//! * параметрическая сущность подставляется на месте ссылки, и её текст
//!   замены разбирается как продолжение подмножества:
//!   `<!ENTITY % пе "<!ELEMENT к EMPTY>">%пе;` проходит, а тот же приём с
//!   текстом «х» — ошибка, потому что «х» не объявление;
//! * в СОДЕРЖИМОМ ссылка на объявленную сущность НЕ подставляется —
//!   отдаётся отдельный узел «Ссылка на сущность» с именем сущности и
//!   пустым значением, разрывающий текст на части. В ЗНАЧЕНИИ АТРИБУТА,
//!   наоборот, подставляется, вместе с вложенными ссылками, ссылками на
//!   символ и предопределёнными сущностями; разметка и внешняя сущность
//!   там ошибка, а в содержимом — нет. При повторном объявлении побеждает
//!   ПЕРВОЕ;
//! * рекурсивное объявление само по себе законно, а обращение к нему —
//!   ошибка, и в содержимом тоже, хотя подстановки там нет.
//!
//! # Сознательные расхождения
//!
//! * **Момент отказа на необъявленной сущности.** Платформа отвергает
//!   такой документ ДО первого узла, даже когда ссылка лежит в самой его
//!   глубине: `<к><а/>&ж;</к>` не отдаёт ни одного события. Здесь чтение
//!   остаётся потоковым, и ошибка приходит на той ссылке, где встретилась.
//!   Поймать её раньше можно было бы только просмотром всего документа
//!   вперёд — второй разбор ради проверки, которую всё равно видно по
//!   итогу. Прочие нарушения разметки платформа, наоборот, находит лениво:
//!   на незакрытом `<к><а/>` она отдаёт три узла и лишь потом падает — как
//!   и здесь.
//! * **Параметрические сущности внутри объявлений.** Подстановка сделана
//!   только для ссылок, стоящих в подмножестве САМОСТОЯТЕЛЬНО. `%м;` в теле
//!   `<!ELEMENT к %м;>` платформа развернула бы, здесь тело объявления
//!   пропускается целиком.
//! * **Внешнее подмножество не читается.** Ни платформа его не читает
//!   (сущность из `SYSTEM "нет.dtd"` считается необъявленной — измерено),
//!   ни здесь.
//! * **Бюджет подстановки сущностей.** Объём разворачивания ограничен
//!   `MAX_ENTITY_EXPANSION` обработанными символами текста замены — одним
//!   счётчиком на ВЕСЬ разбор документа (все значения атрибутов делят
//!   общий остаток) и ещё одним на весь разбор внутреннего подмножества.
//!   Счётчик именно на документ, а не на значение атрибута: иначе элемент
//!   с тысячей атрибутов по десятку байт разметки каждый получал бы тысячу
//!   бюджетов. Ограничение нужно не против рекурсии — её ловит точный
//!   детектор, стек имён сущностей, находящихся в развороте, — а против
//!   бомбы БЕЗ рекурсии: 33 объявления с фан-аутом 10 («billion laughs»)
//!   занимают около 2,4 КБ и разворачиваются в 10^33 символа.
//!   Превышение — обычная ошибка разбора, ловимая
//!   `Попытка`. Где эта граница у платформы, здесь НАМЕРЕННО не измеряется,
//!   и открытым вопросом (меткой, записью в реестре и пробой) значение
//!   тоже не оформлено: проба-бомба в `measure-all.bsl` рисковала бы
//!   вешать каждый сеанс замера, а воспроизводить поведение платформы на
//!   документах-бомбах целью совместимости не является.

use std::path::PathBuf;
use std::rc::Rc;

use crate::string::BslString;
use crate::{BslValue, RtError, RtResult};

/// Ошибка разбора или записи. Текст платформы не воспроизводим — он
/// привязан к номерам строк её модуля, — поэтому своё сообщение.
fn bad(what: impl Into<String>) -> RtError {
    RtError::Xml(what.into())
}

/// Имя текстового узла. Не наша выдумка: `Ч.Имя` на тексте отдаёт именно
/// это (измерено).
pub const TEXT_NODE_NAME: &str = "#text";

/// Имя узла-комментария в дереве DOM (`ИмяУзла` — измерено).
pub const COMMENT_NODE_NAME: &str = "#comment";

/// Имя узла документа в дереве DOM (`ДокументDOM.ИмяУзла` — измерено).
pub const DOCUMENT_NODE_NAME: &str = "#document";

/// Имя узла-секции CDATA в дереве DOM. РАЗБОР такого узла не создаёт (секция
/// вливается в текст), но `СоздатьСекциюCDATA` создаёт, и `ИмяУзла` у него
/// именно это (измерено).
pub const CDATA_NODE_NAME: &str = "#cdata-section";

/// Атрибут начального тега.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlAttr {
    /// Имя как в тексте, вместе с префиксом.
    pub name: String,
    pub value: String,
}

/// Событие разбора. Пространство имён кладётся В САМО событие, а не
/// резолвится потом по стеку: к моменту, когда до свойства доберётся
/// пользователь, элемент со своими объявлениями может быть уже закрыт.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlEvent {
    ElementStart {
        name: String,
        uri: String,
        /// Атрибуты РАЗДЕЛЯЮТСЯ с записью в стеке открытых элементов
        /// (`OpenElement::attrs`): построителю DOM они нужны там, а
        /// копировать их на каждый начальный тег — измеримая плата на
        /// разборе (`xml_parse` теряет около двух процентов).
        attrs: Rc<Vec<XmlAttr>>,
    },
    ElementEnd {
        name: String,
        uri: String,
    },
    Text(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
    /// Комментарий. Отдаётся ТОЛЬКО при включённом
    /// [`XmlParser::set_report_comments`], то есть построителю DOM: сам
    /// `ЧтениеXML` комментарии не показывает (измерено), а в дереве DOM
    /// они узлы — тоже измерено.
    Comment(String),
    /// Ссылка на объявленную во внутреннем подмножестве DTD сущность.
    /// Платформа её НЕ подставляет, а разрывает ею текст: содержимое
    /// `<к>раз&е;два</к>` при объявленной `е` — это «раз», ссылка и «два»
    /// (измерено). Имя — имя сущности, значение у такого узла пустое.
    EntityReference {
        name: String,
    },
}

/// Объявление сущности из внутреннего подмножества DTD.
#[derive(Debug, Clone)]
enum EntityDecl {
    /// Внутренняя: текст замены КАК НАПИСАН в объявлении. Вложенные ссылки
    /// разворачиваются на месте использования, а не здесь: измерено, что
    /// взаимно рекурсивные объявления сами по себе ошибкой не считаются
    /// (`<!ENTITY а "&б;"><!ENTITY б "&а;">` без обращения проходит), а вот
    /// обращение к рекурсивной сущности — считается.
    Internal(String),
    /// Внешняя (`SYSTEM`/`PUBLIC`): текста замены нет. Платформа его тоже
    /// не достаёт — в содержимом такая ссылка отдаётся узлом как любая
    /// другая, а в значении атрибута она ошибка (измерено).
    External,
}

/// Бюджет разворачивания сущностей: сколько символов текста замены
/// разрешено ОБРАБОТАТЬ за весь разбор документа (`XmlParser::budget`) и
/// сколько — за разбор внутреннего подмножества. Счётчик общий, а не свой
/// на каждую ссылку и не свой на каждое значение атрибута: иначе документ
/// умножал бы бюджет на число ссылок или на число атрибутов, оставаясь
/// десятком килобайт разметки.
///
/// Считаются именно обработанные символы, а не выданные наружу: цепочка
/// пустых сущностей с фан-аутом (`<!ENTITY е "">` и десять ссылок на неё
/// уровнем выше) даёт пустой результат при экспоненциальной работе, так что
/// бюджет по выводу дыру не закрывает. Рекурсию бюджет не ловит — для неё
/// есть точный детектор, стек имён в развороте; это защита от бомбы без
/// рекурсии. Значение — сознательная граница, см. шапку модуля.
const MAX_ENTITY_EXPANSION: usize = 4 * 1024 * 1024;

/// Списать с бюджета `n` обработанных символов текста замены.
///
/// # Errors
///
/// [`RtError::Xml`], когда бюджет исчерпан.
fn charge(budget: &mut usize, n: usize) -> RtResult<()> {
    match budget.checked_sub(n) {
        Some(left) => {
            *budget = left;
            Ok(())
        }
        None => Err(bad("слишком большой объём подстановки сущностей")),
    }
}

/// Кадр разворачивания: текст замены и позиция разбора в нём.
///
/// Стек таких кадров лежит в куче, а не в нативной рекурсии, потому что
/// глубину задают входные данные: с точным детектором рекурсии она
/// ограничена лишь числом РАЗЛИЧНЫХ объявленных сущностей, и линейная
/// цепочка из десятков тысяч объявлений переполнила бы стек Rust. Каждый
/// кадр разбирается в собственных границах — объявление или ссылка не могут
/// начаться в тексте замены и закончиться снаружи.
struct ExpandFrame {
    /// Имя разворачиваемой сущности; у самого верхнего кадра его нет.
    /// Имена по стеку кадров — это и есть детектор рекурсии.
    name: Option<String>,
    src: Vec<char>,
    i: usize,
}

impl ExpandFrame {
    fn new(name: Option<String>, text: &str) -> Self {
        ExpandFrame {
            name,
            src: text.chars().collect(),
            i: 0,
        }
    }
}

/// Разворачивается ли сущность `name` прямо сейчас — то есть замыкает ли
/// ссылка на неё цикл. Текущий кадр `current` снят со стека, поэтому
/// проверяется отдельно от остальных.
fn in_expansion(stack: &[ExpandFrame], current: &ExpandFrame, name: &str) -> bool {
    std::iter::once(current)
        .chain(stack.iter())
        .any(|f| f.name.as_deref() == Some(name))
}

/// Имя ссылки на сущность внутри текста замены: `i` стоит на `&`, после
/// возврата — сразу за `;`.
///
/// # Errors
///
/// [`RtError::Xml`], если `;` в тексте замены так и не встретилась.
fn reference_name_at(src: &[char], i: &mut usize) -> RtResult<String> {
    *i += 1;
    let start = *i;
    while *i < src.len() && src[*i] != ';' {
        *i += 1;
    }
    if *i >= src.len() {
        return Err(bad("ссылка на сущность без «;»"));
    }
    let name: String = src[start..*i].iter().collect();
    *i += 1;
    Ok(name)
}

// --- Разбор -------------------------------------------------------------

/// Открытый элемент и объявленные ИМЕННО НА НЁМ префиксы.
#[derive(Debug)]
pub struct OpenElement {
    pub name: String,
    pub uri: String,
    /// Атрибуты начального тега — тот же `Rc`, что унесло событие.
    /// Разборщику они не нужны, но нужны построителю DOM: он вправе
    /// начать с середины документа, и тогда предки восстанавливаются из
    /// этого стека ВМЕСТЕ со своими атрибутами (измерено на 8.3.27, см.
    /// `dom.rs`).
    pub attrs: Rc<Vec<XmlAttr>>,
    /// `(префикс, URI)`; пустой префикс — объявление по умолчанию.
    ns: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct XmlParser {
    src: Vec<char>,
    pos: usize,
    open: Vec<OpenElement>,
    /// Корневой элемент уже закрыт — второго документ не допускает.
    root_done: bool,
    /// `<а/>`: начало отдано, конец ждёт следующего вызова.
    pending_end: Option<(String, String)>,
    /// Отдавать ли комментарии отдельным событием. По умолчанию нет —
    /// таково поведение `ЧтениеXML` (измерено); включает его только
    /// построитель DOM.
    report_comments: bool,
    /// Версия из объявления `<?xml version="..."?>`, если оно было.
    /// Нужна `ДокументDOM.ВерсияXML`: измерено, что документ, объявленный
    /// версией `1.1`, отдаёт именно `1.1`, а без объявления — `1.0`.
    xml_version: Option<String>,
    /// Общие сущности внутреннего подмножества. Список, а не словарь: их
    /// единицы, зато порядок прямо задаёт измеренное правило «побеждает
    /// ПЕРВОЕ объявление» (`<!ENTITY е "раз"><!ENTITY е "два">` даёт
    /// «раз»).
    entities: Vec<(String, EntityDecl)>,
    /// Параметрические сущности подмножества — их текст замены
    /// разбирается как продолжение подмножества.
    param_entities: Vec<(String, String)>,
    /// Объявление типа документа уже встречалось: второго документ не
    /// допускает (измерено).
    doctype_done: bool,
    /// Ссылка на сущность, которую предстоит отдать следующим событием.
    /// Устроена как `pending_end`: текстовый прогон обрывается НА ссылке,
    /// отдаёт накопленный текст, а сама ссылка уходит следующим узлом.
    pending_entity: Option<String>,
    /// Остаток бюджета разворачивания сущностей на ВЕСЬ документ, см.
    /// `MAX_ENTITY_EXPANSION`. Поле, а не локальная переменная значения
    /// атрибута, потому что документ волен раздать бомбу по атрибутам:
    /// счётчик на значение умножался бы на их число.
    budget: usize,
}

impl XmlParser {
    pub fn new(text: &str) -> Self {
        XmlParser {
            src: text.chars().collect(),
            pos: 0,
            open: Vec::new(),
            root_done: false,
            pending_end: None,
            report_comments: false,
            xml_version: None,
            entities: Vec::new(),
            param_entities: Vec::new(),
            doctype_done: false,
            pending_entity: None,
            budget: MAX_ENTITY_EXPANSION,
        }
    }

    /// Версия из объявления XML — `None`, если объявления не было.
    pub fn xml_version(&self) -> Option<&str> {
        self.xml_version.as_deref()
    }

    /// Включить или выключить события [`XmlEvent::Comment`].
    pub fn set_report_comments(&mut self, on: bool) {
        self.report_comments = on;
    }

    /// Ещё не закрытые элементы, от корня к текущему. Построитель DOM
    /// достраивает по ним цепочку предков, когда читатель отдан ему уже
    /// внутри документа.
    pub fn open_elements(&self) -> &[OpenElement] {
        &self.open
    }

    /// URI префикса в области видимости ТЕКУЩЕГО стека открытых элементов.
    /// Нужен построителю DOM: у атрибутов пространство имён разрешается по
    /// тем же объявлениям, что и у элемента, но само событие их не несёт.
    pub fn namespace_of(&self, prefix: &str) -> String {
        self.resolve_prefix(prefix)
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.src.get(self.pos + i) == Some(&c))
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    /// Проглотить всё до `marker` включительно. Отсутствие маркера —
    /// незакрытая конструкция, то есть битый документ.
    fn skip_until(&mut self, marker: &str) -> RtResult<String> {
        let start = self.pos;
        while self.pos < self.src.len() {
            if self.starts_with(marker) {
                let inner: String = self.src[start..self.pos].iter().collect();
                self.pos += marker.chars().count();
                return Ok(inner);
            }
            self.pos += 1;
        }
        Err(bad(format!("не найдено закрывающее «{marker}»")))
    }

    /// Имя элемента или атрибута. XML разрешает в именах куда больше, чем
    /// ASCII, поэтому имя — это всё до пробела и до разделителя разметки.
    fn read_name(&mut self) -> RtResult<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || matches!(c, '=' | '/' | '>' | '<' | '?') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(bad("ожидалось имя"));
        }
        Ok(self.src[start..self.pos].iter().collect())
    }

    /// Имя ссылки после уже проглоченного `&`, вместе с закрывающей `;`.
    fn read_reference_name(&mut self) -> RtResult<String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c != ';' && !c.is_whitespace() && c != '<') {
            self.pos += 1;
        }
        if self.peek() != Some(';') {
            return Err(bad("ссылка на сущность без «;»"));
        }
        let name: String = self.src[start..self.pos].iter().collect();
        self.pos += 1;
        Ok(name)
    }

    /// Объявление сущности по имени; побеждает ПЕРВОЕ (измерено).
    fn lookup_entity(&self, name: &str) -> Option<&EntityDecl> {
        self.entities
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d)
    }

    /// Текст замены параметрической сущности; побеждает первое объявление.
    fn lookup_param(&self, name: &str) -> Option<String> {
        self.param_entities
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
    }

    /// Подстановка текста замены сущности В ЗНАЧЕНИЕ АТРИБУТА — больше
    /// подставлять негде: в содержимом платформа ссылку не разворачивает, а
    /// отдаёт узлом, и там работает [`XmlParser::check_entity_reference`].
    /// Поэтому измеренные запреты значения атрибута здесь безусловны и
    /// действуют на всех уровнях разворачивания: РАЗМЕТКА в тексте замены
    /// (`<!ENTITY е "<б/>">`) и внешняя сущность — ошибка, тогда как в
    /// содержимом ни то, ни другое ошибкой не считается.
    ///
    /// Символ `<`, ПРИШЕДШИЙ ИЗ `&lt;`, запретом не считается: запрещена
    /// разметка, а не символ. Разница спецификационная и на платформе не
    /// измерена.
    ///
    /// Рекурсия ловится точно — по именам сущностей, находящихся в развороте,
    /// — а объём работы ограничен `budget`: сколько ещё символов текста
    /// замены разрешено обработать. Счётчик принадлежит вызывающему и живёт
    /// дольше одного вызова: у значений атрибутов это общий на весь разбор
    /// документа остаток `XmlParser::budget`, см. `MAX_ENTITY_EXPANSION`.
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`] на необъявленной, внешней или рекурсивной сущности,
    /// на разметке в тексте замены и на исчерпанном бюджете.
    fn expand_into(&self, raw: &str, budget: &mut usize, out: &mut String) -> RtResult<()> {
        let top = ExpandFrame::new(None, raw);
        charge(budget, top.src.len())?;
        let mut stack = vec![top];
        while let Some(mut frame) = stack.pop() {
            let mut child = None;
            while frame.i < frame.src.len() {
                let c = frame.src[frame.i];
                if c != '&' {
                    if c == '<' {
                        return Err(bad("разметка в значении атрибута"));
                    }
                    out.push(c);
                    frame.i += 1;
                    continue;
                }
                let name = reference_name_at(&frame.src, &mut frame.i)?;
                if let Some(c) = builtin_entity(&name)? {
                    out.push(c);
                    continue;
                }
                match self.lookup_entity(&name) {
                    None => return Err(bad(format!("неизвестная сущность «&{name};»"))),
                    Some(EntityDecl::External) => {
                        return Err(bad(format!(
                            "внешняя сущность «&{name};» в значении атрибута"
                        )))
                    }
                    Some(EntityDecl::Internal(text)) => {
                        if in_expansion(&stack, &frame, &name) {
                            return Err(bad("рекурсивная ссылка на сущность"));
                        }
                        let next = ExpandFrame::new(Some(name), text);
                        charge(budget, next.src.len())?;
                        child = Some(next);
                        break;
                    }
                }
            }
            // Кадр возвращается на стек только вместе с потомком: разбор
            // продолжится с той же позиции, когда текст замены кончится.
            if let Some(next) = child {
                stack.push(frame);
                stack.push(next);
            }
        }
        Ok(())
    }

    /// Проверить ссылку в СОДЕРЖИМОМ. Подстановки здесь нет — платформа
    /// отдаёт узел «Ссылка на сущность», — но объявление обязано
    /// существовать, а его текст замены не должен ссылаться сам на себя:
    /// и то и другое измерено как ошибка.
    ///
    /// Проверка идёт обходом графа ссылок ПО ИМЕНАМ, без сборки текста
    /// замены: доказать нужно ровно «объявлена, всё достижимое объявлено,
    /// цикла нет». Кадры на стеке — сущности «в работе», `done` — уже
    /// проверенные, так что текст каждой достижимой сущности сканируется
    /// один раз, а бюджет разворачивания здесь не нужен вовсе: работа
    /// линейна по сумме объявленных текстов замены, и на бомбе в содержимом
    /// разбор отвечает мгновенно.
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`] на необъявленной сущности — самой ссылки или любой
    /// достижимой из неё — и на цикле в этих ссылках.
    fn check_entity_reference(&self, name: &str) -> RtResult<()> {
        let text = match self.lookup_entity(name) {
            None => return Err(bad(format!("неизвестная сущность «&{name};»"))),
            // Внешняя сущность в содержимом законна, а текста замены у неё
            // нет — обходить нечего.
            Some(EntityDecl::External) => return Ok(()),
            Some(EntityDecl::Internal(text)) => text,
        };
        let mut done: Vec<String> = Vec::new();
        let mut stack = vec![ExpandFrame::new(Some(name.to_string()), text)];
        while let Some(mut frame) = stack.pop() {
            let mut child = None;
            while frame.i < frame.src.len() {
                if frame.src[frame.i] != '&' {
                    // Разметка в тексте замены в содержимом не запрещена
                    // (измерено), так что всё, кроме ссылок, пропускается.
                    frame.i += 1;
                    continue;
                }
                let target = reference_name_at(&frame.src, &mut frame.i)?;
                if builtin_entity(&target)?.is_some() {
                    continue;
                }
                match self.lookup_entity(&target) {
                    None => return Err(bad(format!("неизвестная сущность «&{target};»"))),
                    Some(EntityDecl::External) => continue,
                    Some(EntityDecl::Internal(text)) => {
                        if done.contains(&target) {
                            continue;
                        }
                        if in_expansion(&stack, &frame, &target) {
                            return Err(bad("рекурсивная ссылка на сущность"));
                        }
                        child = Some(ExpandFrame::new(Some(target), text));
                        break;
                    }
                }
            }
            match child {
                Some(next) => {
                    stack.push(frame);
                    stack.push(next);
                }
                // Текст просмотрен целиком и цикла в нём не нашлось —
                // второй раз эту сущность обходить незачем.
                None => {
                    if let Some(checked) = frame.name {
                        done.push(checked);
                    }
                }
            }
        }
        Ok(())
    }

    /// Текстовый прогон до следующей разметки. `None` — прогон выброшен как
    /// целиком пробельный (измерено: такой узел платформа не отдаёт).
    ///
    /// Секция CDATA не прерывает прогон, а вливается в него: измерено, что
    /// `раз<![CDATA[два]]>три` — ОДИН узел.
    fn read_text_run(&mut self) -> RtResult<Option<String>> {
        let mut out = String::new();
        'run: loop {
            while let Some(c) = self.peek() {
                if c == '<' {
                    break;
                }
                if c == '&' {
                    self.pos += 1;
                    let name = self.read_reference_name()?;
                    match builtin_entity(&name)? {
                        Some(c) => out.push(c),
                        None => {
                            // Ссылка на объявленную сущность ОБРЫВАЕТ прогон:
                            // платформа отдаёт её отдельным узлом между
                            // текстовыми (измерено).
                            self.check_entity_reference(&name)?;
                            if self.open.is_empty() {
                                return Err(bad("ссылка на сущность вне корневого элемента"));
                            }
                            self.pending_entity = Some(name);
                            break 'run;
                        }
                    }
                } else {
                    out.push(c);
                    self.pos += 1;
                }
            }
            if self.starts_with("<![CDATA[") {
                self.pos += "<![CDATA[".chars().count();
                out.push_str(&self.skip_until("]]>")?);
                continue;
            }
            break;
        }
        if self.open.is_empty() {
            // Вне корневого элемента текста быть не может; пробелы —
            // могут (перевод строки в конце файла — обычное дело).
            if out.trim().is_empty() {
                return Ok(None);
            }
            return Err(bad("текст вне корневого элемента"));
        }
        // Пробельный прогон выбрасывается независимо от того, пришёл он
        // из обычного текста или из секции CDATA: измерено, что
        // `<а><![CDATA[ ]]></а>` узла не даёт — явная выписка секции
        // значащей его НЕ делает.
        if out.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(out))
    }

    /// Объявления `xmlns` начального тега — в область видимости, остальное
    /// — в атрибуты.
    fn split_namespaces(attrs: &[XmlAttr]) -> Vec<(String, String)> {
        let mut ns = Vec::new();
        for a in attrs {
            if let Some(prefix) = a.name.strip_prefix("xmlns:") {
                ns.push((prefix.to_string(), a.value.clone()));
            } else if a.name == "xmlns" {
                ns.push((String::new(), a.value.clone()));
            }
        }
        ns
    }

    /// URI по префиксу — поиск от вершины стека вниз, как того требует
    /// область видимости XML.
    fn resolve_prefix(&self, prefix: &str) -> String {
        for el in self.open.iter().rev() {
            for (p, uri) in &el.ns {
                if p == prefix {
                    return uri.clone();
                }
            }
        }
        String::new()
    }

    fn read_attributes(&mut self) -> RtResult<Vec<XmlAttr>> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') | Some('/') | None => return Ok(attrs),
                _ => {}
            }
            let name = self.read_name()?;
            self.skip_ws();
            if self.peek() != Some('=') {
                return Err(bad(format!("у атрибута «{name}» нет значения")));
            }
            self.pos += 1;
            self.skip_ws();
            let quote = match self.peek() {
                Some(q @ ('"' | '\'')) => q,
                _ => return Err(bad(format!("значение атрибута «{name}» не в кавычках"))),
            };
            self.pos += 1;
            let mut value = String::new();
            loop {
                match self.peek() {
                    None => return Err(bad(format!("значение атрибута «{name}» не закрыто"))),
                    Some(c) if c == quote => {
                        self.pos += 1;
                        break;
                    }
                    Some('&') => {
                        self.pos += 1;
                        let entity = self.read_reference_name()?;
                        match builtin_entity(&entity)? {
                            Some(c) => value.push(c),
                            // В ЗНАЧЕНИИ АТРИБУТА платформа сущность
                            // подставляет (измерено: `&е;` при
                            // `<!ENTITY е "х">` даёт «х»), в отличие от
                            // содержимого, где та же ссылка — узел.
                            //
                            // Бюджет разворачивания — общий на ВЕСЬ разбор
                            // документа, а не на ссылку и не на значение
                            // атрибута: тот же довод уровнем выше — иначе
                            // элемент с тысячей атрибутов по десятку байт
                            // разметки каждый получил бы тысячу бюджетов.
                            // На время вызова остаток вынимается в локальную
                            // переменную и кладётся обратно, потому что
                            // `expand_into` и `lookup_entity` держат `&self`.
                            None => {
                                let mut budget = self.budget;
                                let expanded = match self.lookup_entity(&entity) {
                                    None => Err(bad(format!("неизвестная сущность «&{entity};»"))),
                                    Some(EntityDecl::External) => Err(bad(format!(
                                        "внешняя сущность «&{entity};» в значении атрибута"
                                    ))),
                                    Some(EntityDecl::Internal(text)) => {
                                        self.expand_into(text, &mut budget, &mut value)
                                    }
                                };
                                self.budget = budget;
                                expanded?;
                            }
                        }
                    }
                    Some(c) => {
                        value.push(c);
                        self.pos += 1;
                    }
                }
            }
            attrs.push(XmlAttr { name, value });
        }
    }

    /// Литерал в кавычках или апострофах; отдаётся содержимое без кавычек.
    fn read_quoted(&mut self) -> RtResult<String> {
        let quote = match self.peek() {
            Some(q @ ('"' | '\'')) => q,
            _ => return Err(bad("ожидался литерал в кавычках")),
        };
        self.pos += 1;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                let s: String = self.src[start..self.pos].iter().collect();
                self.pos += 1;
                return Ok(s);
            }
            self.pos += 1;
        }
        Err(bad("литерал не закрыт"))
    }

    /// Имя в объявлении типа документа. Отдельно от [`XmlParser::read_name`]:
    /// здесь имя обрывает ещё и `[`, потому что подмножество разрешено
    /// писать вплотную (`<!DOCTYPE к[…]>` — измерено, что проходит).
    fn read_doctype_name(&mut self) -> RtResult<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || matches!(c, '[' | '>' | '<' | '"' | '\'') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(bad("у объявления типа документа нет имени"));
        }
        Ok(self.src[start..self.pos].iter().collect())
    }

    /// Объявление типа документа целиком: имя, необязательный внешний
    /// идентификатор, необязательное внутреннее подмножество.
    ///
    /// Проверки не выведены из спецификации, а измерены: `<!DOCTYPE>` без
    /// имени, `<!doctype к>` строчными, `PUBLIC` с одним литералом,
    /// кириллица в публичном идентификаторе, объявление внутри элемента,
    /// после корня и второе подряд — всё это платформа отвергает, а
    /// `<!DOCTYPE к SYSTEM 'нет.dtd'>`, `SYSTEM "не>т.dtd"`, `<!DOCTYPE к >`
    /// и несовпадение имени с корнем — принимает.
    fn read_doctype(&mut self) -> RtResult<()> {
        if !self.open.is_empty() || self.root_done {
            return Err(bad("объявление типа документа не перед корневым элементом"));
        }
        if self.doctype_done {
            return Err(bad("второе объявление типа документа"));
        }
        self.pos += "<!DOCTYPE".chars().count();
        self.skip_ws();
        let _name = self.read_doctype_name()?;
        self.skip_ws();
        if self.starts_with("SYSTEM") {
            self.pos += "SYSTEM".chars().count();
            self.skip_ws();
            self.read_quoted()?;
        } else if self.starts_with("PUBLIC") {
            self.pos += "PUBLIC".chars().count();
            self.skip_ws();
            let pubid = self.read_quoted()?;
            if let Some(c) = pubid.chars().find(|c| !is_pubid_char(*c)) {
                return Err(bad(format!(
                    "недопустимый символ «{c}» в публичном идентификаторе"
                )));
            }
            self.skip_ws();
            // Второй литерал у `PUBLIC` обязателен: без него платформа
            // отвергает документ (измерено).
            self.read_quoted()?;
        }
        self.skip_ws();
        if self.peek() == Some('[') {
            self.pos += 1;
            let subset = self.take_subset()?;
            self.parse_subset(&subset)?;
            self.skip_ws();
        }
        if self.peek() != Some('>') {
            return Err(bad("объявление типа документа не закрыто"));
        }
        self.pos += 1;
        self.doctype_done = true;
        Ok(())
    }

    /// Текст внутреннего подмножества: от позиции сразу за `[` до парного
    /// `]`. Кавычки, комментарии и инструкции обработки уважаются, поэтому
    /// ни `]`, ни `>` внутри литерала или комментария подмножество не
    /// закрывают (измерено на `<!ENTITY е "]>">` и `<!-- ]> -->`).
    fn take_subset(&mut self) -> RtResult<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            match c {
                ']' => {
                    let text: String = self.src[start..self.pos].iter().collect();
                    self.pos += 1;
                    return Ok(text);
                }
                '<' => {
                    if self.starts_with("<!--") {
                        self.pos += 4;
                        self.skip_until("-->")?;
                    } else if self.starts_with("<?") {
                        self.pos += 2;
                        self.skip_until("?>")?;
                    } else {
                        self.skip_markup_declaration()?;
                    }
                }
                _ => self.pos += 1,
            }
        }
        Err(bad("внутреннее подмножество не закрыто"))
    }

    /// Проглотить `<…>` целиком, не спотыкаясь о `>` внутри литералов.
    fn skip_markup_declaration(&mut self) -> RtResult<()> {
        self.pos += 1;
        while let Some(c) = self.peek() {
            match c {
                '>' => {
                    self.pos += 1;
                    return Ok(());
                }
                '"' | '\'' => {
                    self.read_quoted()?;
                }
                _ => self.pos += 1,
            }
        }
        Err(bad("объявление во внутреннем подмножестве не закрыто"))
    }

    /// Разбор внутреннего подмножества.
    ///
    /// Собираются объявления сущностей; остальные объявления только
    /// проверяются на состоятельность и отбрасываются. Строгость измерена:
    /// `<!ЧУШЬ>`, `<!entity …>` строчными, `<!ENTITY е>` без значения,
    /// `<!ATTLIST>` и `<!ELEMENT к>` без модели, условная секция
    /// `<![INCLUDE[…]]>` и просто текст — всё ошибки, а комментарий,
    /// инструкция обработки и два объявления подряд — нет.
    ///
    /// Ссылка на параметрическую сущность разворачивается ЗДЕСЬ И СЕЙЧАС, и
    /// её текст замены разбирается как продолжение подмножества: измерено,
    /// что `<!ENTITY % пе "<!ELEMENT к EMPTY>">%пе;` проходит, а тот же
    /// приём с текстом «х» — ошибка, потому что «х» не объявление.
    ///
    /// Разворачивание идёт стеком кадров, а не нативной рекурсией, и по тем
    /// же правилам, что в [`XmlParser::expand_into`]: цикл ловится по именам
    /// сущностей в развороте, а объём — общим на весь разбор подмножества
    /// бюджетом `MAX_ENTITY_EXPANSION`, без которого фан-аут `%имя;` даёт
    /// экспоненциальную работу на документе в полкилобайта.
    fn parse_subset(&mut self, text: &str) -> RtResult<()> {
        let mut budget = MAX_ENTITY_EXPANSION;
        let mut stack = vec![ExpandFrame::new(None, text)];
        while let Some(mut frame) = stack.pop() {
            let mut child = None;
            while frame.i < frame.src.len() {
                if frame.src[frame.i].is_whitespace() {
                    frame.i += 1;
                    continue;
                }
                if frame.src[frame.i] == '%' {
                    frame.i += 1;
                    let start = frame.i;
                    while frame.i < frame.src.len()
                        && frame.src[frame.i] != ';'
                        && !frame.src[frame.i].is_whitespace()
                    {
                        frame.i += 1;
                    }
                    if frame.src.get(frame.i) != Some(&';') {
                        return Err(bad("ссылка на параметрическую сущность без «;»"));
                    }
                    let name: String = frame.src[start..frame.i].iter().collect();
                    frame.i += 1;
                    let Some(body) = self.lookup_param(&name) else {
                        return Err(bad(format!(
                            "неизвестная параметрическая сущность «%{name};»"
                        )));
                    };
                    if in_expansion(&stack, &frame, &name) {
                        return Err(bad("рекурсивная параметрическая сущность"));
                    }
                    let next = ExpandFrame::new(Some(name), &body);
                    charge(&mut budget, next.src.len())?;
                    child = Some(next);
                    break;
                }
                if frame.src[frame.i] != '<' {
                    return Err(bad("мусор во внутреннем подмножестве"));
                }
                if starts_at(&frame.src, frame.i, "<!--") {
                    frame.i = after(&frame.src, frame.i + 4, "-->")
                        .ok_or_else(|| bad("комментарий не закрыт"))?;
                    continue;
                }
                if starts_at(&frame.src, frame.i, "<?") {
                    frame.i = after(&frame.src, frame.i + 2, "?>")
                        .ok_or_else(|| bad("инструкция обработки не закрыта"))?;
                    continue;
                }
                if starts_at(&frame.src, frame.i, "<!ENTITY") {
                    frame.i += "<!ENTITY".len();
                    self.parse_entity_decl(&frame.src, &mut frame.i)?;
                    continue;
                }
                let known = ["<!ELEMENT", "<!ATTLIST", "<!NOTATION"]
                    .into_iter()
                    .find(|kw| starts_at(&frame.src, frame.i, kw));
                let Some(keyword) = known else {
                    return Err(bad("недопустимое объявление во внутреннем подмножестве"));
                };
                frame.i += keyword.len();
                if !matches!(frame.src.get(frame.i), Some(c) if c.is_whitespace()) {
                    return Err(bad(format!("после «{keyword}» нужен пробел")));
                }
                skip_ws_at(&frame.src, &mut frame.i);
                let name = read_name_at(&frame.src, &mut frame.i);
                if name.is_empty() {
                    return Err(bad(format!("у объявления «{keyword}» нет имени")));
                }
                let tail = decl_tail_at(&frame.src, &mut frame.i)?;
                // `<!ELEMENT к>` платформа отвергает: модель содержимого
                // обязательна (измерено), а у `<!ATTLIST`/`<!NOTATION` хвост
                // проверять нечем — там уже пошли частности DTD.
                if keyword == "<!ELEMENT" && tail.trim().is_empty() {
                    return Err(bad("у объявления элемента нет модели содержимого"));
                }
            }
            // Кадр ссылки возвращается на стек под потомком: подмножество
            // продолжится ровно за `%имя;`, когда текст замены кончится.
            if let Some(next) = child {
                stack.push(frame);
                stack.push(next);
            }
        }
        Ok(())
    }

    /// Объявление сущности; `<!ENTITY` уже проглочено, `i` стоит сразу за
    /// ним.
    fn parse_entity_decl(&mut self, src: &[char], i: &mut usize) -> RtResult<()> {
        if !matches!(src.get(*i), Some(c) if c.is_whitespace()) {
            return Err(bad("после «<!ENTITY» нужен пробел"));
        }
        skip_ws_at(src, i);
        let parametric = src.get(*i) == Some(&'%');
        if parametric {
            *i += 1;
            skip_ws_at(src, i);
        }
        let name = read_name_at(src, i);
        if name.is_empty() {
            return Err(bad("у объявления сущности нет имени"));
        }
        skip_ws_at(src, i);
        let decl = match src.get(*i) {
            Some('"') | Some('\'') => EntityDecl::Internal(quoted_at(src, i)?),
            _ => {
                if starts_at(src, *i, "SYSTEM") {
                    *i += "SYSTEM".len();
                    skip_ws_at(src, i);
                    quoted_at(src, i)?;
                } else if starts_at(src, *i, "PUBLIC") {
                    *i += "PUBLIC".len();
                    skip_ws_at(src, i);
                    quoted_at(src, i)?;
                    skip_ws_at(src, i);
                    quoted_at(src, i)?;
                } else {
                    return Err(bad(format!("у объявления сущности «{name}» нет значения")));
                }
                EntityDecl::External
            }
        };
        decl_tail_at(src, i)?;
        // Побеждает ПЕРВОЕ объявление (измерено), поэтому повторное просто
        // отбрасывается.
        if parametric {
            if let EntityDecl::Internal(text) = decl {
                if !self.param_entities.iter().any(|(n, _)| *n == name) {
                    self.param_entities.push((name, text));
                }
            }
        } else if !self.entities.iter().any(|(n, _)| *n == name) {
            self.entities.push((name, decl));
        }
        Ok(())
    }

    /// Следующее событие; `None` — документ кончился.
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`] на битой разметке: незакрытый элемент, чужой
    /// закрывающий тег, второй корень, текст вне корня, негодное объявление
    /// типа документа или ссылка на необъявленную сущность.
    pub fn read(&mut self) -> RtResult<Option<XmlEvent>> {
        if let Some((name, uri)) = self.pending_end.take() {
            self.open.pop();
            if self.open.is_empty() {
                self.root_done = true;
            }
            return Ok(Some(XmlEvent::ElementEnd { name, uri }));
        }
        loop {
            // Ссылка, на которой оборвался текстовый прогон, — проверка
            // стоит ВНУТРИ цикла: прогон мог оказаться пробельным и уйти в
            // отброс, а событие всё равно причитается.
            if let Some(name) = self.pending_entity.take() {
                return Ok(Some(XmlEvent::EntityReference { name }));
            }
            if self.pos >= self.src.len() {
                if let Some(open) = self.open.last() {
                    return Err(bad(format!("элемент «{}» не закрыт", open.name)));
                }
                if !self.root_done {
                    return Err(bad("в документе нет корневого элемента"));
                }
                return Ok(None);
            }
            // Секция CDATA — не самостоятельный узел, а НАЧАЛО текстового
            // прогона: проверка обязана стоять до общей ветки `<!`, иначе
            // секция уедет в разбор объявления типа документа.
            if self.peek() != Some('<') || self.starts_with("<![CDATA[") {
                if let Some(text) = self.read_text_run()? {
                    return Ok(Some(XmlEvent::Text(text)));
                }
                continue;
            }
            // Объявление XML читатель не отдаёт — измерено. Спецификация
            // держит имя `xml` за платформой, так что это не инструкция
            // обработки.
            if self.starts_with("<?xml") {
                self.pos += 2;
                let decl = self.skip_until("?>")?;
                if self.xml_version.is_none() {
                    self.xml_version = declared_version(&decl);
                }
                continue;
            }
            if self.starts_with("<?") {
                self.pos += 2;
                let target = self.read_name()?;
                self.skip_ws();
                let data = self.skip_until("?>")?;
                return Ok(Some(XmlEvent::ProcessingInstruction {
                    target,
                    data: data.trim_end().to_string(),
                }));
            }
            // Комментарий не отдаётся, но текст вокруг себя РАЗРЫВАЕТ:
            // выход из `read_text_run` уже произошёл, а новый прогон
            // начнётся после `-->` (измерено). Построителю DOM он всё же
            // нужен узлом — тогда включён `report_comments`.
            if self.starts_with("<!--") {
                self.pos += 4;
                let text = self.skip_until("-->")?;
                if self.report_comments {
                    return Ok(Some(XmlEvent::Comment(text)));
                }
                continue;
            }
            // Объявление типа документа узлом не отдаётся — измерено на
            // `<!DOCTYPE а><а/>`, где видны только начало и конец `а`, — но
            // пройти его надо целиком, вместе с внутренним подмножеством.
            if self.starts_with("<!DOCTYPE") {
                self.read_doctype()?;
                continue;
            }
            // Другой разметки на `<!` вне подмножества не бывает: измерено,
            // что `<к><!DOCTYPE а></к>`, `<к/><!DOCTYPE а>` и второе
            // объявление подряд платформа отвергает, а строчное `<!doctype`
            // не признаёт вовсе.
            if self.starts_with("<!") {
                return Err(bad("недопустимая разметка «<!…»"));
            }
            if self.starts_with("</") {
                self.pos += 2;
                let name = self.read_name()?;
                self.skip_ws();
                if self.peek() != Some('>') {
                    return Err(bad(format!("закрывающий тег «{name}» не закрыт")));
                }
                self.pos += 1;
                let open = self
                    .open
                    .pop()
                    .ok_or_else(|| bad(format!("закрывающий тег «{name}» без открывающего")))?;
                if open.name != name {
                    return Err(bad(format!(
                        "закрывающий тег «{name}» не совпадает с открытым «{}»",
                        open.name
                    )));
                }
                if self.open.is_empty() {
                    self.root_done = true;
                }
                return Ok(Some(XmlEvent::ElementEnd {
                    name,
                    uri: open.uri,
                }));
            }
            // Начальный тег.
            if self.root_done {
                return Err(bad("в документе больше одного корневого элемента"));
            }
            self.pos += 1;
            let name = self.read_name()?;
            let attrs = self.read_attributes()?;
            let empty = match self.peek() {
                Some('/') => {
                    self.pos += 1;
                    if self.peek() != Some('>') {
                        return Err(bad(format!("тег «{name}» не закрыт")));
                    }
                    self.pos += 1;
                    true
                }
                Some('>') => {
                    self.pos += 1;
                    false
                }
                _ => return Err(bad(format!("тег «{name}» не закрыт"))),
            };
            let ns = Self::split_namespaces(&attrs);
            let attrs = Rc::new(attrs);
            self.open.push(OpenElement {
                name: name.clone(),
                uri: String::new(),
                attrs: Rc::clone(&attrs),
                ns,
            });
            // Префикс резолвится ПОСЛЕ помещения в стек: элемент вправе
            // пользоваться префиксом, который сам же и объявил.
            let prefix = prefix_of(&name);
            let uri = self.resolve_prefix(prefix);
            if let Some(top) = self.open.last_mut() {
                top.uri = uri.clone();
            }
            if empty {
                self.pending_end = Some((name.clone(), uri.clone()));
            }
            return Ok(Some(XmlEvent::ElementStart { name, uri, attrs }));
        }
    }

    /// Глубина открытых элементов — по ней `Пропустить` понимает, где
    /// остановиться.
    pub fn depth(&self) -> usize {
        self.open.len()
    }
}

/// Символ, встроенный в XML: ссылка на символ либо одна из пяти
/// предопределённых сущностей. `None` — имя не из этого набора, то есть
/// ссылка на объявленную (или необъявленную) сущность.
///
/// # Errors
///
/// [`RtError::Xml`] на негодной ссылке на символ.
fn builtin_entity(name: &str) -> RtResult<Option<char>> {
    if let Some(rest) = name.strip_prefix('#') {
        let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            rest.parse::<u32>().ok()
        };
        return code
            .and_then(char::from_u32)
            .map(Some)
            .ok_or_else(|| bad(format!("недопустимая ссылка на символ «&{name};»")));
    }
    Ok(match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        // Сущности, которой в XML нет и которую не объявили, платформа не
        // прощает: `<а>&nbsp;</а>` — ошибка, а не текст как есть (измерено).
        _ => None,
    })
}

/// Допустим ли символ в публичном идентификаторе. Набор спецификационный, но
/// проверка здесь не по спецификации, а по замеру: `PUBLIC "-//П//DTD Б//RU"`
/// платформа отвергает, а тот же идентификатор латиницей — принимает.
fn is_pubid_char(c: char) -> bool {
    matches!(c, ' ' | '\r' | '\n') || c.is_ascii_alphanumeric() || "-'()+,./:=?;!*#@$_%".contains(c)
}

/// Начинается ли `src` с позиции `at` с подстроки `what` (ASCII).
fn starts_at(src: &[char], at: usize, what: &str) -> bool {
    what.chars()
        .enumerate()
        .all(|(k, c)| src.get(at + k) == Some(&c))
}

/// Позиция сразу за первым вхождением `what`, начиная с `from`.
fn after(src: &[char], from: usize, what: &str) -> Option<usize> {
    let n = what.chars().count();
    (from..src.len())
        .find(|&k| starts_at(src, k, what))
        .map(|k| k + n)
}

fn skip_ws_at(src: &[char], i: &mut usize) {
    while matches!(src.get(*i), Some(c) if c.is_whitespace()) {
        *i += 1;
    }
}

/// Имя внутри подмножества: до пробела и до разделителей объявления.
fn read_name_at(src: &[char], i: &mut usize) -> String {
    let start = *i;
    while let Some(&c) = src.get(*i) {
        if c.is_whitespace() || matches!(c, '>' | '<' | '"' | '\'' | '(' | '[') {
            break;
        }
        *i += 1;
    }
    src[start..*i].iter().collect()
}

/// Литерал в кавычках внутри подмножества.
fn quoted_at(src: &[char], i: &mut usize) -> RtResult<String> {
    let quote = match src.get(*i) {
        Some(&q @ ('"' | '\'')) => q,
        _ => return Err(bad("ожидался литерал в кавычках")),
    };
    *i += 1;
    let start = *i;
    while let Some(&c) = src.get(*i) {
        if c == quote {
            let s: String = src[start..*i].iter().collect();
            *i += 1;
            return Ok(s);
        }
        *i += 1;
    }
    Err(bad("литерал не закрыт"))
}

/// Хвост объявления до его `>`; кавычки уважаются.
fn decl_tail_at(src: &[char], i: &mut usize) -> RtResult<String> {
    let start = *i;
    while let Some(&c) = src.get(*i) {
        match c {
            '>' => {
                let tail: String = src[start..*i].iter().collect();
                *i += 1;
                return Ok(tail);
            }
            '"' | '\'' => {
                quoted_at(src, i)?;
            }
            _ => *i += 1,
        }
    }
    Err(bad("объявление во внутреннем подмножестве не закрыто"))
}

/// Значение псевдоатрибута `version` из текста объявления XML.
///
/// Разбирается отдельной функцией, а не общим `read_attributes`: объявление
/// — не элемент, и его псевдоатрибуты платформа не проверяет вовсе
/// (измерено: `version="1.1"` она принимает и отдаёт как есть).
fn declared_version(decl: &str) -> Option<String> {
    let rest = decl.split_once("version")?.1;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value = rest[quote.len_utf8()..].split(quote).next()?;
    Some(value.to_string())
}

/// Часть имени до двоеточия; без двоеточия префикса нет.
pub fn prefix_of(name: &str) -> &str {
    match name.split_once(':') {
        Some((p, _)) => p,
        None => "",
    }
}

/// Часть имени после двоеточия.
pub fn local_of(name: &str) -> &str {
    match name.split_once(':') {
        Some((_, l)) => l,
        None => name,
    }
}

// --- Запись -------------------------------------------------------------

/// `ПараметрыЗаписиXML(Кодировка, Версия, ИспользоватьОтступ)`.
///
/// Третий параметр гасит И перевод строки, И отступ разом — измерено:
/// `<а><б/></а>` в одну строку.
#[derive(Debug, Clone)]
pub struct XmlWriterSettings {
    /// `None` — не писать `encoding` в объявлении. Именно так ведёт себя
    /// `УстановитьСтроку()` без параметров, тогда как `ОткрытьФайл`
    /// подставляет UTF-8 (измерено).
    pub encoding: Option<String>,
    pub version: String,
    pub indent: bool,
}

impl Default for XmlWriterSettings {
    fn default() -> Self {
        XmlWriterSettings {
            encoding: None,
            version: "1.0".to_string(),
            indent: true,
        }
    }
}

/// Открытый элемент писателя.
#[derive(Debug)]
struct OpenTag {
    name: String,
    /// Последним в этом элементе шёл текст: тогда закрывающий тег встаёт
    /// вплотную, без перевода строки (измерено).
    last_was_text: bool,
}

#[derive(Debug)]
pub struct XmlWriter {
    out: String,
    settings: XmlWriterSettings,
    stack: Vec<OpenTag>,
    /// Начальный тег написан, но не закрыт `>`: можно ещё дописать атрибут
    /// или схлопнуть элемент в `<а/>`.
    pending_start: bool,
    /// Корневой элемент закрыт — второго документ не допускает.
    root_done: bool,
    path: Option<PathBuf>,
}

impl XmlWriter {
    pub fn to_string_target(settings: XmlWriterSettings) -> Self {
        XmlWriter {
            out: String::new(),
            settings,
            stack: Vec::new(),
            pending_start: false,
            root_done: false,
            path: None,
        }
    }

    pub fn to_file(path: PathBuf, settings: XmlWriterSettings) -> Self {
        let mut w = Self::to_string_target(settings);
        w.path = Some(path);
        w
    }

    pub fn is_file_target(&self) -> bool {
        self.path.is_some()
    }

    /// Есть ли открытый элемент. Нужно записи дерева DOM: текстоподобный
    /// узел вне элемента платформа отвергает (измерено на секции CDATA), а
    /// сам `write_cdata` о вложенности не судит.
    pub fn in_element(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Глубина открытых элементов: у корня документа она нулевая, а у
    /// элемента, который сейчас будет записан, — на единицу больше. Нужна
    /// записи XDTO: платформа строит имена порождённых префиксов как
    /// `d<глубина>p<номер>` и считает глубину именно так (измерено — см.
    /// заголовок модуля `xdto`).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Перевод строки и отступ по глубине. Один таб на уровень — измерено.
    ///
    /// Файловый приёмник платформы разделяет строки CRLF, строковый — LF;
    /// оба измерены: строковый — построчными фикстурами `xdto-xml-io`,
    /// файловый — побайтным сличением выгрузки `benchmarks/edata_writer.bsl`
    /// с платформенной (размер сошёлся ровно на сигнатуре UTF-8 и по байту
    /// CR на каждый из 111 007 переводов). Перевод строки внутри ТЕКСТА
    /// узла при этом не преобразуется — записанный `Символ(10)` остаётся
    /// одиночным LF; платформа поступает так же (якорь
    /// `XML.FILE_NEWLINE_IN_TEXT`: CR 2, LF 3).
    fn newline(&mut self, depth: usize) {
        if !self.settings.indent {
            return;
        }
        if self.path.is_some() {
            self.out.push('\r');
        }
        self.out.push('\n');
        for _ in 0..depth {
            self.out.push('\t');
        }
    }

    /// Отбить очередной УЗЕЛ от предыдущего вывода.
    ///
    /// Перевод строки принадлежит НАЧАЛУ узла, а не концу предыдущего, и
    /// перед самым первым узлом его нет вовсе. Различить две модели прямая
    /// запись не позволяла (`ЗаписатьОбъявлениеXML` + элемент даёт
    /// `<?xml ...?>` + перевод строки + элемент при любой из них), а запись
    /// ДЕРЕВА позволила: документ без корня платформа отдаёт как
    /// `<?xml version="1.0"?>` БЕЗ хвостового перевода строки, а одиночный
    /// комментарий — как `<!--к-->` без ведущего.
    fn newline_before_node(&mut self) {
        if self.out.is_empty() {
            return;
        }
        let depth = self.stack.len();
        self.newline(depth);
    }

    /// Дописать `>` у висящего начального тега: содержимое элемента
    /// начинается.
    fn close_pending(&mut self) {
        if self.pending_start {
            self.out.push('>');
            self.pending_start = false;
        }
    }

    fn mark_content(&mut self, text_like: bool) {
        if let Some(top) = self.stack.last_mut() {
            top.last_was_text = text_like;
        }
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если объявление пишется не первым.
    pub fn write_declaration(&mut self) -> RtResult<()> {
        if !self.out.is_empty() || self.pending_start {
            return Err(bad("объявление XML должно идти первым"));
        }
        self.out.push_str("<?xml version=\"");
        self.out.push_str(&self.settings.version);
        self.out.push('"');
        if let Some(enc) = self.settings.encoding.clone() {
            self.out.push_str(" encoding=\"");
            self.out.push_str(&enc);
            self.out.push('"');
        }
        self.out.push_str("?>");
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если корневой элемент уже закрыт.
    pub fn write_start_element(&mut self, name: &str) -> RtResult<()> {
        if self.root_done {
            return Err(bad("корневой элемент уже записан"));
        }
        self.close_pending();
        self.newline_before_node();
        self.mark_content(false);
        self.out.push('<');
        self.out.push_str(name);
        self.pending_start = true;
        self.stack.push(OpenTag {
            name: name.to_string(),
            last_was_text: false,
        });
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если начальный тег уже закрыт: после текста или
    /// вложенного элемента атрибут дописать нельзя (измерено — ошибка).
    pub fn write_attribute(&mut self, name: &str, value: &str) -> RtResult<()> {
        if !self.pending_start {
            return Err(bad("атрибут вне начального тега"));
        }
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        escape_attr(&mut self.out, value);
        self.out.push('"');
        Ok(())
    }

    /// Пустая строка не делает НИЧЕГО: элемент остаётся пустым и
    /// схлопывается в `<а/>` (измерено).
    ///
    /// # Errors
    ///
    /// [`RtError::Xml`], если открытого элемента нет.
    pub fn write_text(&mut self, text: &str) -> RtResult<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.stack.is_empty() {
            return Err(bad("текст вне элемента"));
        }
        self.close_pending();
        escape_text(&mut self.out, text);
        self.mark_content(true);
        Ok(())
    }

    /// # Errors
    ///
    /// [`RtError::Xml`], если открытого элемента нет.
    pub fn write_end_element(&mut self) -> RtResult<()> {
        let Some(top) = self.stack.pop() else {
            return Err(bad("ЗаписатьКонецЭлемента без открытого элемента"));
        };
        if self.pending_start {
            self.out.push_str("/>");
            self.pending_start = false;
        } else {
            if !top.last_was_text {
                let depth = self.stack.len();
                self.newline(depth);
            }
            self.out.push_str("</");
            self.out.push_str(&top.name);
            self.out.push('>');
        }
        if self.stack.is_empty() {
            self.root_done = true;
        }
        self.mark_content(false);
        Ok(())
    }

    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия с остальными методами.
    pub fn write_comment(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        self.newline_before_node();
        self.out.push_str("<!--");
        self.out.push_str(text);
        self.out.push_str("-->");
        self.mark_content(false);
        Ok(())
    }

    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_processing_instruction(&mut self, target: &str, data: &str) -> RtResult<()> {
        self.close_pending();
        self.newline_before_node();
        self.out.push_str("<?");
        self.out.push_str(target);
        if !data.is_empty() {
            self.out.push(' ');
            self.out.push_str(data);
        }
        self.out.push_str("?>");
        self.mark_content(false);
        Ok(())
    }

    /// Секция CDATA ведёт себя ДВОЙСТВЕННО, и это измерено: отступ перед
    /// собой она получает как узел, а закрывающий тег после неё встаёт
    /// вплотную, как после текста.
    ///
    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_cdata(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        self.newline_before_node();
        self.out.push_str("<![CDATA[");
        self.out.push_str(text);
        self.out.push_str("]]>");
        self.mark_content(true);
        Ok(())
    }

    /// Ссылка на сущность в содержимом элемента. Пишется как есть, без
    /// экранирования амперсанда: измерено, что дерево, разобранное из
    /// `<к>раз&е;два</к>`, писатель отдаёт обратно теми же символами.
    /// Считается содержимым НАРАВНЕ С ТЕКСТОМ — закрывающий тег после неё
    /// с новой строки не начинается.
    ///
    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_entity_reference(&mut self, name: &str) -> RtResult<()> {
        self.close_pending();
        self.out.push('&');
        self.out.push_str(name);
        self.out.push(';');
        self.mark_content(true);
        Ok(())
    }

    /// Зеркало `write_cdata`: отступа перед собой НЕ получает, а после себя
    /// закрывающий тег с новой строки оставляет (измерено).
    ///
    /// # Errors
    ///
    /// Не отказывает; `Result` — ради единообразия.
    pub fn write_raw(&mut self, text: &str) -> RtResult<()> {
        self.close_pending();
        self.out.push_str(text);
        self.mark_content(false);
        Ok(())
    }

    /// Отдать накопленное. Незакрытые элементы НЕ дописываются: висящий
    /// начальный тег закрывается одним `>`, и на этом всё (измерено —
    /// `<а>`, а не `<а/>` и не `<а></а>`).
    pub fn finish(&mut self) -> String {
        self.close_pending();
        self.stack.clear();
        std::mem::take(&mut self.out)
    }

    pub fn take_path(&mut self) -> Option<PathBuf> {
        self.path.take()
    }
}

/// Экранирование текста узла: апостроф и кавычка остаются как есть
/// (измерено).
fn escape_text(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

/// Экранирование значения атрибута: к набору текста добавляется кавычка, но
/// НЕ апостроф; табуляция и перевод строки уходят как есть (измерено).
fn escape_attr(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

// --- Склейка с объектами BSL --------------------------------------------
//
// Как и у JSON: методы живут здесь, наружу уходят через `builtin.rs`.

use crate::object::{BslObject, XmlReaderState};
use crate::EnumValue;

fn as_reader(v: &BslValue) -> RtResult<&std::cell::RefCell<XmlReaderState>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XmlReader(state) => Ok(state),
            _ => Err(not_applicable(v)),
        },
        _ => Err(not_applicable(v)),
    }
}

fn as_writer(v: &BslValue) -> RtResult<&std::cell::RefCell<Option<XmlWriter>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::XmlWriter(state) => Ok(state),
            _ => Err(not_applicable(v)),
        },
        _ => Err(not_applicable(v)),
    }
}

fn not_applicable(v: &BslValue) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод XML",
        receiver: v.type_name(),
    }
}

pub fn is_xml_reader(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XmlReader(_)))
}

pub fn is_xml_writer(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::XmlWriter(_)))
}

fn need_str(arg: Option<&BslValue>, op: &'static str) -> RtResult<String> {
    match arg {
        Some(BslValue::Str(s)) => Ok(s.to_string()),
        _ => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Настройки из аргумента `УстановитьСтроку([Параметры])`.
fn settings_from(arg: Option<&BslValue>) -> RtResult<XmlWriterSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(XmlWriterSettings::default()),
        Some(BslValue::Object(o)) => match &**o {
            BslObject::XmlWriterSettings(s) => Ok(s.clone()),
            _ => Err(RtError::TypeError {
                expected: "ПараметрыЗаписиXML",
                op: "УстановитьСтроку",
            }),
        },
        Some(_) => Err(RtError::TypeError {
            expected: "ПараметрыЗаписиXML",
            op: "УстановитьСтроку",
        }),
    }
}

/// `ЧтениеXML.УстановитьСтроку(Текст)` / `ЗаписьXML.УстановитьСтроку([Параметры])`.
///
/// # Errors
///
/// [`RtError::TypeError`], если получатель не объект XML либо аргумент не
/// того типа.
pub fn set_string(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    if let Ok(reader) = as_reader(obj) {
        let text = need_str(args.first(), "УстановитьСтроку")?;
        *reader.borrow_mut() = XmlReaderState::over(XmlParser::new(&text));
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(XmlWriter::to_string_target(settings_from(args.first())?));
    Ok(())
}

/// `ОткрытьФайл(Имя)` у обоих объектов XML.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не читается; [`RtError::TypeError`] при
/// неверном аргументе.
pub fn open_file(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let path = need_str(args.first(), "ОткрытьФайл")?;
    if let Ok(reader) = as_reader(obj) {
        let text = std::fs::read_to_string(&path).map_err(|e| RtError::IoError(e.to_string()))?;
        // Платформа терпит сигнатуру UTF-8 в начале файла, а разборщику
        // она видна как символ перед `<` — снимаем.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
        *reader.borrow_mut() = XmlReaderState::over(XmlParser::new(&text));
        return Ok(());
    }
    let writer = as_writer(obj)?;
    // У файлового приёмника объявление получает `encoding` — измерено на
    // содержимом записанного файла.
    let settings = XmlWriterSettings {
        encoding: Some("UTF-8".to_string()),
        ..XmlWriterSettings::default()
    };
    *writer.borrow_mut() = Some(XmlWriter::to_file(PathBuf::from(path), settings));
    Ok(())
}

/// Разобрать следующий узел. Курсор атрибутов при этом сбрасывается.
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке.
pub fn read(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let Some(parser) = state.parser.as_mut() else {
        return Err(bad("источник для ЧтениеXML не задан"));
    };
    let event = parser.read()?;
    state.attr_cursor = None;
    match event {
        Some(e) => {
            state.depth = state.parser.as_ref().map_or(0, XmlParser::depth);
            state.current = Some(e);
            Ok(BslValue::Boolean(true))
        }
        None => {
            state.current = None;
            Ok(BslValue::Boolean(false))
        }
    }
}

/// `Пропустить()` — проглотить остаток текущего элемента и встать НА его
/// закрывающий тег (измерено; на нетекстовом узле пропускается остаток
/// родителя).
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке или если пропускать нечего.
pub fn skip(obj: &BslValue) -> RtResult<()> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    // Глубина снимается ДО заимствования разборщика: после первого же
    // `read` она уже другая, а нужна та, что была на текущем узле.
    let depth = state.depth;
    if depth == 0 {
        return Err(bad("Пропустить вне элемента"));
    }
    let target = depth - 1;
    let Some(parser) = state.parser.as_mut() else {
        return Err(bad("источник для ЧтениеXML не задан"));
    };
    loop {
        let Some(event) = parser.read()? else {
            state.current = None;
            state.depth = 0;
            return Ok(());
        };
        let now = parser.depth();
        if matches!(event, XmlEvent::ElementEnd { .. }) && now == target {
            state.current = Some(event);
            state.depth = now;
            state.attr_cursor = None;
            return Ok(());
        }
    }
}

/// `ПрочитатьАтрибут()` — курсор по атрибутам текущего элемента.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn read_attribute(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let count = state.attrs().len();
    let next = match state.attr_cursor {
        None => 0,
        Some(i) => i + 1,
    };
    if next >= count {
        state.attr_cursor = Some(count);
        return Ok(BslValue::Boolean(false));
    }
    state.attr_cursor = Some(next);
    Ok(BslValue::Boolean(true))
}

/// `ПерейтиКСодержимому()` -> член `ТипУзлаXML`.
///
/// # Errors
///
/// [`RtError::Xml`] на битой разметке.
pub fn move_to_content(obj: &BslValue) -> RtResult<BslValue> {
    loop {
        {
            let reader = as_reader(obj)?;
            let state = reader.borrow();
            if matches!(
                state.current,
                Some(XmlEvent::ElementStart { .. })
                    | Some(XmlEvent::ElementEnd { .. })
                    | Some(XmlEvent::Text(_))
            ) {
                drop(state);
                return node_type(obj);
            }
        }
        if read(obj)? == BslValue::Boolean(false) {
            return Ok(BslValue::Enum(EnumValue::XmlNothing));
        }
    }
}

/// `ТипУзла` — член `ТипУзлаXML`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn node_type(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if state.attr_cursor.is_some_and(|i| i < state.attrs().len()) {
        return Ok(BslValue::Enum(EnumValue::XmlAttribute));
    }
    let v = match &state.current {
        None => EnumValue::XmlNothing,
        Some(XmlEvent::ElementStart { .. }) => EnumValue::XmlElementStart,
        Some(XmlEvent::ElementEnd { .. }) => EnumValue::XmlElementEnd,
        Some(XmlEvent::Text(_)) => EnumValue::XmlText,
        Some(XmlEvent::ProcessingInstruction { .. }) => EnumValue::XmlProcessingInstruction,
        // Недостижимо: комментарии разборщик отдаёт только построителю
        // DOM, а тот не оставляет их в состоянии читателя. Ветка написана
        // явно, чтобы `match` оставался исчерпывающим.
        Some(XmlEvent::Comment(_)) => EnumValue::XmlComment,
        Some(XmlEvent::EntityReference { .. }) => EnumValue::XmlEntityReference,
    };
    Ok(BslValue::Enum(v))
}

/// `Имя` текущего узла; у текста это `#text` (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn name(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if let Some(a) = state.current_attr() {
        return Ok(BslValue::Str(BslString::from_str(&a.name)));
    }
    let s = match &state.current {
        None => String::new(),
        Some(XmlEvent::ElementStart { name, .. }) | Some(XmlEvent::ElementEnd { name, .. }) => {
            name.clone()
        }
        Some(XmlEvent::Text(_)) => TEXT_NODE_NAME.to_string(),
        Some(XmlEvent::ProcessingInstruction { target, .. }) => target.clone(),
        // Недостижимо — см. `node_type`.
        Some(XmlEvent::Comment(_)) => COMMENT_NODE_NAME.to_string(),
        // У ссылки на сущность `Имя` — имя сущности, а `Значение` пусто
        // (измерено).
        Some(XmlEvent::EntityReference { name }) => name.clone(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `Значение` текущего узла; у элемента оно пустое (измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn value(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    if let Some(a) = state.current_attr() {
        return Ok(BslValue::Str(BslString::from_str(&a.value)));
    }
    let s = match &state.current {
        Some(XmlEvent::Text(t)) => t.clone(),
        Some(XmlEvent::ProcessingInstruction { data, .. }) => data.clone(),
        // Недостижимо — см. `node_type`; ответ дан по образцу дерева DOM,
        // где значение комментария и есть его текст.
        Some(XmlEvent::Comment(t)) => t.clone(),
        // У элемента значения нет (измерено), у ссылки на сущность — тоже:
        // `Значение` на ней пусто, хотя текст замены известен.
        Some(XmlEvent::ElementStart { .. })
        | Some(XmlEvent::ElementEnd { .. })
        | Some(XmlEvent::EntityReference { .. })
        | None => String::new(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `ЛокальноеИмя` — имя без префикса.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn local_name(obj: &BslValue) -> RtResult<BslValue> {
    let full = name(obj)?;
    let BslValue::Str(s) = &full else {
        return Ok(full);
    };
    Ok(BslValue::Str(BslString::from_str(local_of(&s.to_string()))))
}

/// `Префикс` — часть имени до двоеточия.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn prefix(obj: &BslValue) -> RtResult<BslValue> {
    let full = name(obj)?;
    let BslValue::Str(s) = &full else {
        return Ok(full);
    };
    Ok(BslValue::Str(BslString::from_str(prefix_of(
        &s.to_string(),
    ))))
}

/// `URIПространстваИмен` текущего элемента.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn namespace_uri(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let s = match &state.current {
        Some(XmlEvent::ElementStart { uri, .. }) | Some(XmlEvent::ElementEnd { uri, .. }) => {
            uri.clone()
        }
        _ => String::new(),
    };
    Ok(BslValue::Str(BslString::from_str(&s)))
}

/// `КоличествоАтрибутов()`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn attribute_count(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    Ok(BslValue::Number(bsl_number::BslNumber::from_i64(
        state.attrs().len() as i64,
    )))
}

/// `ИмяАтрибута(Индекс)`. Индекс за границей — `Неопределено`, как и у
/// `ЗначениеАтрибута` (у которого это измерено).
///
/// # Errors
///
/// [`RtError::BadIndex`], если индекс не целое неотрицательное.
pub fn attribute_name(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let idx = index_arg(args.first())?;
    // Индекс за границей списка даёт `Неопределено` — измерено отдельно
    // от `ЗначениеАтрибута`, у обоих одинаково.
    Ok(state.attrs().get(idx).map_or(BslValue::Undefined, |a| {
        BslValue::Str(BslString::from_str(&a.name))
    }))
}

/// `ЗначениеАтрибута(ИмяЛибоИндекс)` -> значение либо `Неопределено`
/// (измерено: у отсутствующего атрибута тип результата — «Не определено»).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка и не число.
pub fn attribute_value(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    match args.first() {
        Some(BslValue::Str(s)) => {
            let wanted = s.to_string();
            Ok(state
                .attrs()
                .iter()
                .find(|a| a.name == wanted)
                .map_or(BslValue::Undefined, |a| {
                    BslValue::Str(BslString::from_str(&a.value))
                }))
        }
        Some(BslValue::Number(_)) => {
            let idx = index_arg(args.first())?;
            Ok(state.attrs().get(idx).map_or(BslValue::Undefined, |a| {
                BslValue::Str(BslString::from_str(&a.value))
            }))
        }
        _ => Err(RtError::TypeError {
            expected: "Строка либо Число",
            op: "ЗначениеАтрибута",
        }),
    }
}

fn index_arg(arg: Option<&BslValue>) -> RtResult<usize> {
    match arg {
        Some(BslValue::Number(n)) => {
            let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
            usize::try_from(i).map_err(|_| RtError::BadIndex)
        }
        _ => Err(RtError::TypeError {
            expected: "Число",
            op: "индекс атрибута",
        }),
    }
}

// --- Методы записи ------------------------------------------------------

/// Доступ к писателю получателя. `pub(crate)`, потому что тем же писателем
/// пишет дерево DOM (`dom::write`): второго сериализатора XML в рантайме нет.
pub(crate) fn with_writer<R>(
    obj: &BslValue,
    f: impl FnOnce(&mut XmlWriter) -> RtResult<R>,
) -> RtResult<R> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let w = slot
        .as_mut()
        .ok_or_else(|| bad("приёмник для ЗаписьXML не задан"))?;
    f(w)
}

/// Доступ к состоянию читателя. `pub(crate)` по той же причине, что и
/// [`with_writer`]: разбор XML в экземпляры XDTO (`xdto::factory_read_xml`)
/// идёт по СОБЫТИЯМ того же `ЧтениеXML`, и второго разборщика для этого
/// заводить не за чем. Читатель после вызова остаётся там, куда его
/// подвинул `f`, — позиция наблюдаема из BSL.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`, плюс
/// всё, чем ответит `f`.
pub(crate) fn with_reader<R>(
    obj: &BslValue,
    f: impl FnOnce(&mut XmlReaderState) -> RtResult<R>,
) -> RtResult<R> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    f(&mut state)
}

/// `ЗаписатьОбъявлениеXML()`.
///
/// # Errors
///
/// [`RtError::Xml`], если объявление пишется не первым.
pub fn write_declaration(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_declaration)
}

/// `ЗаписатьНачалоЭлемента(Имя)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если имя не строка; [`RtError::Xml`], если
/// корневой элемент уже записан.
pub fn write_start_element(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьНачалоЭлемента")?;
    with_writer(obj, |w| w.write_start_element(&name))
}

/// `ЗаписатьКонецЭлемента()`.
///
/// # Errors
///
/// [`RtError::Xml`], если открытого элемента нет.
pub fn write_end_element(obj: &BslValue) -> RtResult<()> {
    with_writer(obj, XmlWriter::write_end_element)
}

/// `ЗаписатьАтрибут(Имя, Значение)` — оба только строки (измерено: число
/// даёт ошибку).
///
/// # Errors
///
/// [`RtError::TypeError`] на нестроковом аргументе; [`RtError::Xml`], если
/// начальный тег уже закрыт.
pub fn write_attribute(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let name = need_str(args.first(), "ЗаписатьАтрибут")?;
    let value = need_str(args.get(1), "ЗаписатьАтрибут")?;
    with_writer(obj, |w| w.write_attribute(&name, &value))
}

/// `ЗаписатьТекст(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_text(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьТекст")?;
    with_writer(obj, |w| w.write_text(&text))
}

/// `ЗаписатьКомментарий(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_comment(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьКомментарий")?;
    with_writer(obj, |w| w.write_comment(&text))
}

/// `ЗаписатьСекциюCDATA(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_cdata(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьСекциюCDATA")?;
    with_writer(obj, |w| w.write_cdata(&text))
}

/// `ЗаписатьИнструкциюОбработки(Имя, Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_processing_instruction(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let target = need_str(args.first(), "ЗаписатьИнструкциюОбработки")?;
    let data = need_str(args.get(1), "ЗаписатьИнструкциюОбработки")?;
    with_writer(obj, |w| w.write_processing_instruction(&target, &data))
}

/// `ЗаписатьБезОбработки(Текст)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка.
pub fn write_raw(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let text = need_str(args.first(), "ЗаписатьБезОбработки")?;
    with_writer(obj, |w| w.write_raw(&text))
}

/// `ЗаписьXML.Закрыть()` -> текст для строкового приёмника либо пустая
/// строка для файлового. Второй вызов подряд отдаёт пустую строку —
/// измерено.
///
/// # Errors
///
/// [`RtError::IoError`], если файл не записался.
pub fn close_writer(obj: &BslValue) -> RtResult<BslValue> {
    let writer = as_writer(obj)?;
    let mut slot = writer.borrow_mut();
    let Some(w) = slot.as_mut() else {
        return Ok(BslValue::Str(BslString::from_str("")));
    };
    let text = w.finish();
    if let Some(path) = w.take_path() {
        // Файл платформа начинает сигнатурой UTF-8 — измерено побайтным
        // сличением выгрузки `edata_writer` (первые три байта EF BB BF).
        let mut bytes = Vec::with_capacity(3 + text.len());
        bytes.extend_from_slice(b"\xef\xbb\xbf");
        bytes.extend_from_slice(text.as_bytes());
        std::fs::write(&path, bytes).map_err(|e| RtError::IoError(e.to_string()))?;
        *slot = None;
        return Ok(BslValue::Str(BslString::from_str("")));
    }
    *slot = None;
    Ok(BslValue::Str(BslString::from_str(&text)))
}

/// `ЧтениеXML.Закрыть()` — источник отпускается, объект остаётся годным для
/// нового `УстановитьСтроку`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ЧтениеXML`.
pub fn close_reader(obj: &BslValue) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    *reader.borrow_mut() = XmlReaderState::default();
    Ok(BslValue::Undefined)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn events(text: &str) -> Vec<XmlEvent> {
        let mut p = XmlParser::new(text);
        let mut out = Vec::new();
        while let Some(e) = p.read().expect("разбор") {
            out.push(e);
        }
        out
    }

    fn start(name: &str) -> XmlEvent {
        XmlEvent::ElementStart {
            name: name.into(),
            uri: String::new(),
            attrs: Rc::new(Vec::new()),
        }
    }

    fn end(name: &str) -> XmlEvent {
        XmlEvent::ElementEnd {
            name: name.into(),
            uri: String::new(),
        }
    }

    #[test]
    fn empty_element_is_indistinguishable_from_a_pair_of_tags() {
        // Замеры XML.READ.EMPTY_ELEMENT и XML.READ.EMPTY_PAIR.
        assert_eq!(events("<а/>"), vec![start("а"), end("а")]);
        assert_eq!(events("<а></а>"), events("<а/>"));
    }

    #[test]
    fn declaration_and_comment_are_not_reported_but_a_comment_splits_text() {
        // Замеры XML.READ.DECLARATION, XML.READ.COMMENT и
        // XML.READ.TEXT_SPLIT_BY_COMMENT.
        assert_eq!(
            events("<?xml version=\"1.0\"?><а/>"),
            vec![start("а"), end("а")]
        );
        assert_eq!(events("<а><!-- сюда --></а>"), vec![start("а"), end("а")]);
        assert_eq!(
            events("<а>раз<!--к-->два</а>"),
            vec![
                start("а"),
                XmlEvent::Text("раз".into()),
                XmlEvent::Text("два".into()),
                end("а"),
            ]
        );
    }

    #[test]
    fn cdata_merges_into_the_surrounding_text_run() {
        // Замер XML.READ.TEXT_TWICE_SPLIT: ОДИН узел, а не три. Именно
        // этим секция отличается от комментария.
        assert_eq!(
            events("<а>раз<![CDATA[два]]>три</а>"),
            vec![start("а"), XmlEvent::Text("раздватри".into()), end("а")]
        );
        // Замер XML.READ.CDATA: секция в начале содержимого — тот же текст.
        assert_eq!(
            events("<а><![CDATA[<не разметка>]]></а>"),
            vec![start("а"), XmlEvent::Text("<не разметка>".into()), end("а"),]
        );
    }

    #[test]
    fn whitespace_only_text_is_dropped_but_padding_survives() {
        // Замеры XML.READ.WHITESPACE_BETWEEN и XML.READ.TEXT_PADDED.
        assert_eq!(
            events("<а>  <б/>  </а>"),
            vec![start("а"), start("б"), end("б"), end("а")]
        );
        assert_eq!(
            events("<а> т </а>"),
            vec![start("а"), XmlEvent::Text(" т ".into()), end("а")]
        );
    }

    #[test]
    fn entities_and_character_references_are_decoded() {
        // Замеры XML.READ.ENTITIES и XML.READ.CHAR_REF.
        assert_eq!(
            events("<а>&amp;&lt;&gt;&quot;&apos;</а>"),
            vec![start("а"), XmlEvent::Text("&<>\"'".into()), end("а")]
        );
        assert_eq!(
            events("<а>&#65;&#x42;</а>"),
            vec![start("а"), XmlEvent::Text("AB".into()), end("а")]
        );
    }

    #[test]
    fn namespace_declaration_stays_an_attribute_and_resolves_the_prefix() {
        // Замеры XML.READ.NS_ATTR_COUNT и XML.READ.NAME_PARTS_NS: объявление
        // видно среди атрибутов И при этом резолвит префикс своего же
        // элемента.
        let ev = events("<п:а xmlns:п=\"http://прим\" х=\"1\">т</п:а>");
        let XmlEvent::ElementStart { name, uri, attrs } = &ev[0] else {
            panic!("ожидалось начало элемента: {ev:?}");
        };
        assert_eq!(name, "п:а");
        assert_eq!(uri, "http://прим");
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].name, "xmlns:п");
        assert_eq!(attrs[1].name, "х");
        assert_eq!(local_of(name), "а");
        assert_eq!(prefix_of(name), "п");
    }

    #[test]
    fn broken_markup_is_an_error() {
        // Замеры XML.READ.UNCLOSED, GARBAGE, EMPTY_STRING и TWO_ROOTS.
        for text in ["<а><б></а>", "не разметка", "", "<а/><б/>"] {
            let mut p = XmlParser::new(text);
            let mut failed = false;
            loop {
                match p.read() {
                    Ok(Some(_)) => continue,
                    Ok(None) => break,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            assert!(failed, "битый ввод принят молча: {text:?}");
        }
    }

    fn write(f: impl FnOnce(&mut XmlWriter)) -> String {
        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        f(&mut w);
        w.finish()
    }

    #[test]
    fn default_formatting_indents_elements_but_not_text() {
        // Замеры XML.WRITE.DEFAULT_FORMAT и XML.WRITE.DEEP_INDENT.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_start_element("б").unwrap();
            w.write_text("т").unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а>\n\t<б>т</б>\n</а>");

        let deep = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_start_element("б").unwrap();
            w.write_start_element("в").unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(deep, "<а>\n\t<б>\n\t\t<в/>\n\t</б>\n</а>");
    }

    #[test]
    fn file_target_uses_crlf_for_structure_but_not_inside_text() {
        // Файловый приёмник разделяет строки CRLF (измерено выгрузкой
        // `edata_writer`), а перевод внутри текста узла не преобразует —
        // как и платформа (якорь `XML.FILE_NEWLINE_IN_TEXT`).
        let mut w = XmlWriter::to_file(
            PathBuf::from("/nonexistent/unused.xml"),
            XmlWriterSettings::default(),
        );
        w.write_start_element("а").unwrap();
        w.write_start_element("б").unwrap();
        w.write_text("т\nт").unwrap();
        w.write_end_element().unwrap();
        w.write_end_element().unwrap();
        assert_eq!(w.finish(), "<а>\r\n\t<б>т\nт</б>\r\n</а>");
    }

    #[test]
    fn mixed_content_keeps_the_closing_tag_tight_after_text() {
        // Замер XML.WRITE.MIXED.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("т").unwrap();
            w.write_start_element("б").unwrap();
            w.write_end_element().unwrap();
            w.write_text("у").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а>т\n\t<б/>у</а>");
    }

    #[test]
    fn cdata_and_raw_are_mirror_images_of_each_other() {
        // Замеры XML.WRITE.CDATA_SECTION и XML.WRITE.RAW: секция получает
        // отступ перед собой, но не перевод строки после; у записи без
        // обработки ровно наоборот.
        let cdata = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_cdata("<не разметка>").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(cdata, "<а>\n\t<![CDATA[<не разметка>]]></а>");

        let raw = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_raw("<сырое/>").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(raw, "<а><сырое/>\n</а>");
    }

    #[test]
    fn empty_text_leaves_the_element_collapsed() {
        // Замер XML.WRITE.TEXT_EMPTY.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(out, "<а/>");
    }

    #[test]
    fn escaping_differs_between_text_and_attribute() {
        // Замеры XML.WRITE.ESCAPE_TEXT и XML.WRITE.ESCAPE_ATTR: апостроф не
        // экранируется нигде, кавычка — только в атрибуте.
        let text = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_text("&<>\"'").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(text, "<а>&amp;&lt;&gt;\"'</а>");

        let attr = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_attribute("х", "&<>\"'").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(attr, "<а х=\"&amp;&lt;&gt;&quot;'\"/>");
    }

    #[test]
    fn unclosed_element_is_not_completed_on_finish() {
        // Замер XML.WRITE.UNCLOSED: именно `<а>`, а не `<а/>`.
        let out = write(|w| {
            w.write_start_element("а").unwrap();
        });
        assert_eq!(out, "<а>");
    }

    #[test]
    fn structure_violations_are_rejected() {
        // Замеры XML.WRITE.UNBALANCED_END, ATTR_AFTER_TEXT, TWO_ROOTS и
        // DECL_LATE.
        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        assert!(w.write_end_element().is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        w.write_text("т").unwrap();
        assert!(w.write_attribute("х", "1").is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        w.write_end_element().unwrap();
        assert!(w.write_start_element("б").is_err());

        let mut w = XmlWriter::to_string_target(XmlWriterSettings::default());
        w.write_start_element("а").unwrap();
        assert!(w.write_declaration().is_err());
    }

    #[test]
    fn indent_flag_removes_every_line_break() {
        // Замеры XML.WRITE.SETTINGS_NO_INDENT и XML.WRITE.DECL_NO_INDENT.
        let settings = XmlWriterSettings {
            encoding: Some("UTF-8".to_string()),
            version: "1.0".to_string(),
            indent: false,
        };
        let mut w = XmlWriter::to_string_target(settings);
        w.write_declaration().unwrap();
        w.write_start_element("а").unwrap();
        w.write_start_element("б").unwrap();
        w.write_end_element().unwrap();
        w.write_end_element().unwrap();
        assert_eq!(
            w.finish(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><а><б/></а>"
        );
    }

    #[test]
    fn round_trip_survives_escaping() {
        // Замер XML.ROUND_TRIP.
        let text = write(|w| {
            w.write_start_element("а").unwrap();
            w.write_attribute("х", "1").unwrap();
            w.write_text("т&т").unwrap();
            w.write_end_element().unwrap();
        });
        assert_eq!(text, "<а х=\"1\">т&amp;т</а>");
        let ev = events(&text);
        assert_eq!(ev[1], XmlEvent::Text("т&т".into()));
    }

    // --- Внутреннее подмножество DTD -----------------------------------

    /// Ошибка разбора, а не события: пробы, где документ обязан быть
    /// отвергнут.
    fn fails(text: &str) -> bool {
        let mut p = XmlParser::new(text);
        loop {
            match p.read() {
                Ok(None) => return false,
                Ok(Some(_)) => {}
                Err(_) => return true,
            }
        }
    }

    #[test]
    fn internal_subset_is_scanned_past_its_traps() {
        // Якоря DTD.SUBSET.ATTLIST, DTD.SUBSET.LITERAL_GT,
        // DTD.SUBSET.LITERAL_BRACKET, DTD.SUBSET.LITERAL_APOS,
        // DTD.SUBSET.COMMENT и DTD.SUBSET.PI: ни `>`, ни `]>` внутри
        // литерала, комментария или инструкции обработки подмножества не
        // закрывают.
        assert_eq!(
            events("<!DOCTYPE к [<!ATTLIST т ид ID #IMPLIED>]><к/>"),
            vec![start("к"), end("к")]
        );
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY е \"a>b\">]><к/>"),
            vec![start("к"), end("к")]
        );
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY е \"]>\">]><к/>"),
            vec![start("к"), end("к")]
        );
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY е \']>\'>]><к/>"),
            vec![start("к"), end("к")]
        );
        assert_eq!(
            events("<!DOCTYPE к [<!-- ]> -->]><к/>"),
            vec![start("к"), end("к")]
        );
        assert_eq!(
            events("<!DOCTYPE к [<?пи ]> ?>]><к/>"),
            vec![start("к"), end("к")]
        );
        // Якорь DTD.HEADER.TIGHT_SUBSET: подмножество вплотную к имени.
        assert_eq!(
            events("<!DOCTYPE к[<!ELEMENT к EMPTY>]><к/>"),
            vec![start("к"), end("к")]
        );
        // Якорь XML.READ.DOCTYPE: голое объявление как прежде.
        assert_eq!(events("<!DOCTYPE а><а/>"), vec![start("а"), end("а")]);
    }

    #[test]
    fn a_broken_doctype_is_an_error_and_not_a_hang() {
        // Якоря DTD.SUBSET.UNTERMINATED, DTD.HEADER.UNTERMINATED,
        // DTD.HEADER.NO_NAME, DTD.HEADER.LOWERCASE, DTD.SUBSET.GARBAGE,
        // DTD.SUBSET.UNKNOWN_DECL, DTD.SUBSET.LOWERCASE_DECL,
        // DTD.SUBSET.CONDITIONAL, DTD.HEADER.PUBLIC_ONE_LITERAL,
        // DTD.HEADER.PUBLIC_CYRILLIC, DTD.HEADER.TWICE,
        // DTD.HEADER.INSIDE_ELEMENT и DTD.HEADER.AFTER_ROOT.
        assert!(fails("<!DOCTYPE к [<!ELEMENT к EMPTY><к/>"));
        assert!(fails("<!DOCTYPE к"));
        assert!(fails("<!DOCTYPE><к/>"));
        assert!(fails("<!doctype к><к/>"));
        assert!(fails("<!DOCTYPE к [чушь]><к/>"));
        assert!(fails("<!DOCTYPE к [<!ЧУШЬ>]><к/>"));
        assert!(fails("<!DOCTYPE к [<!entity е \"х\">]><к/>"));
        assert!(fails("<!DOCTYPE к [<![INCLUDE[<!ELEMENT к EMPTY>]]>]><к/>"));
        assert!(fails("<!DOCTYPE к PUBLIC \"-//A//DTD B//EN\"><к/>"));
        assert!(fails(
            "<!DOCTYPE к PUBLIC \"-//П//DTD Б//RU\" \"нет.dtd\"><к/>"
        ));
        assert!(fails("<!DOCTYPE к><!DOCTYPE к><к/>"));
        assert!(fails("<к><!DOCTYPE а></к>"));
        assert!(fails("<к/><!DOCTYPE а>"));
        // Якорь DTD.HEADER.NAME_MISMATCH: с корнем имя НЕ сверяется.
        assert_eq!(events("<!DOCTYPE а [ ]><б/>"), vec![start("б"), end("б")]);
    }

    #[test]
    fn a_declared_entity_becomes_a_node_of_its_own_in_content() {
        // Якоря DTD.ENTITY.CONTENT и DTD.ENTITY.SPLIT: подстановки нет,
        // текст вокруг ссылки разрывается.
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY е \"х\">]><к>раз&е;два</к>"),
            vec![
                start("к"),
                XmlEvent::Text("раз".into()),
                XmlEvent::EntityReference { name: "е".into() },
                XmlEvent::Text("два".into()),
                end("к"),
            ]
        );
        // Якорь DTD.ENTITY.PREDEFINED: предопределённая сущность остаётся
        // текстом и при объявленном подмножестве.
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY е \"х\">]><к>&amp;</к>"),
            vec![start("к"), XmlEvent::Text("&".into()), end("к")]
        );
        // Якоря DTD.ENTITY.UNDECLARED, DTD.ENTITY.EXTERNAL_DTD и
        // DTD.ENTITY.SELF_USE.
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"х\">]><к>&ж;</к>"));
        assert!(fails("<!DOCTYPE к SYSTEM \"нет.dtd\"><к>&е;</к>"));
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"&е;\">]><к>&е;</к>"));
        // Якорь DTD.ENTITY.MUTUAL_DECL: объявления без обращения — не
        // ошибка.
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY а \"&б;\"><!ENTITY б \"&а;\">]><к/>"),
            vec![start("к"), end("к")]
        );
    }

    #[test]
    fn an_entity_is_substituted_in_an_attribute_value() {
        // Якоря DTD.ATTR.SIMPLE, DTD.ATTR.NESTED, DTD.ATTR.CHAR_REF,
        // DTD.ATTR.AMP и DTD.ATTR.DUPLICATE.
        let value = |text: &str| {
            let ev = events(text);
            let XmlEvent::ElementStart { attrs, .. } = &ev[0] else {
                panic!("первым событием обязан быть начальный тег");
            };
            attrs[0].value.clone()
        };
        assert_eq!(value("<!DOCTYPE к [<!ENTITY е \"х\">]><к з=\"&е;\"/>"), "х");
        assert_eq!(
            value("<!DOCTYPE к [<!ENTITY а \"х\"><!ENTITY б \"&а;у\">]><к з=\"&б;\"/>"),
            "ху"
        );
        assert_eq!(
            value("<!DOCTYPE к [<!ENTITY е \"&#65;\">]><к з=\"&е;\"/>"),
            "A"
        );
        assert_eq!(
            value("<!DOCTYPE к [<!ENTITY е \"&amp;\">]><к з=\"&е;\"/>"),
            "&"
        );
        assert_eq!(
            value("<!DOCTYPE к [<!ENTITY е \"раз\"><!ENTITY е \"два\">]><к з=\"&е;\"/>"),
            "раз"
        );
        // Якоря DTD.ATTR.MARKUP, DTD.ATTR.EXTERNAL и DTD.ATTR.RECURSION:
        // рекурсия обязана кончаться ошибкой, а не бесконечным разбором.
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"<б/>\">]><к з=\"&е;\"/>"));
        assert!(fails(
            "<!DOCTYPE к [<!ENTITY е SYSTEM \"нет.txt\">]><к з=\"&е;\"/>"
        ));
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"&е;\">]><к з=\"&е;\"/>"));
    }

    #[test]
    fn a_parameter_entity_is_replaced_inside_the_subset() {
        // Якоря DTD.PE.DECLARATION, DTD.PE.BAD_REPLACEMENT,
        // DTD.PE.UNDECLARED и DTD.PE.RECURSION: текст замены разбирается
        // как продолжение подмножества, поэтому «х» объявлением не станет.
        assert_eq!(
            events("<!DOCTYPE к [<!ENTITY % пе \"<!ELEMENT к EMPTY>\">%пе;]><к/>"),
            vec![start("к"), end("к")]
        );
        assert!(fails("<!DOCTYPE к [<!ENTITY % пе \"х\">%пе;]><к/>"));
        assert!(fails("<!DOCTYPE к [%пе;]><к/>"));
        assert!(fails("<!DOCTYPE к [<!ENTITY % пе \"%пе;\">%пе;]><к/>"));
    }

    /// Подмножество «billion laughs»: `е0` — текст `leaf`, каждая следующая
    /// сущность ссылается на предыдущую `fanout` раз. Рекурсии здесь нет, а
    /// работа разворачивания — `fanout^levels` при документе в несколько
    /// сотен байт.
    fn entity_bomb(levels: usize, fanout: usize, leaf: &str) -> String {
        let mut doc = format!("<!DOCTYPE к [<!ENTITY е0 \"{leaf}\">");
        for level in 1..=levels {
            doc.push_str(&format!("<!ENTITY е{level} \""));
            for _ in 0..fanout {
                doc.push_str(&format!("&е{};", level - 1));
            }
            doc.push_str("\">");
        }
        doc.push_str("]>");
        doc
    }

    #[test]
    fn entity_bomb_in_attribute_hits_the_expansion_budget() {
        // В значении атрибута подстановка настоящая, поэтому бомба обязана
        // упереться в бюджет и стать обычной ошибкой разбора — не съеденной
        // памятью и не разбором на минуты. Второй документ — та же бомба с
        // ПУСТЫМ текстом в основании: наружу она не выдаёт ни символа, так
        // что бюджет по длине вывода её бы не поймал, а по обработанным
        // символам — ловит.
        for leaf in ["ааааааааа", ""] {
            let doc = format!("{}<к з=\"&е9;\"/>", entity_bomb(9, 10, leaf));
            assert!(
                doc.len() < 1024,
                "документ-бомба — сотни байт: {}",
                doc.len()
            );
            let started = Instant::now();
            assert!(fails(&doc));
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "бюджет обязан оборвать разворачивание сразу"
            );
        }
    }

    #[test]
    fn references_in_one_attribute_value_share_the_budget() {
        // Сущность на 900 000 символов в бюджет укладывается, пять таких
        // подряд — уже нет: счётчик один на всё значение, поэтому число
        // ссылок его не умножает.
        let subset = entity_bomb(5, 10, "ааааааааа");
        let ev = events(&format!("{subset}<к з=\"&е5;\"/>"));
        let XmlEvent::ElementStart { attrs, .. } = &ev[0] else {
            panic!("первым событием обязан быть начальный тег");
        };
        assert_eq!(attrs[0].value.chars().count(), 900_000);
        let mut doc = subset;
        doc.push_str("<к з=\"");
        for _ in 0..5 {
            doc.push_str("&е5;");
        }
        doc.push_str("\"/>");
        assert!(fails(&doc));
    }

    #[test]
    fn the_expansion_budget_is_not_multiplied_by_the_attribute_count() {
        // Счётчик один на ВЕСЬ разбор документа, а не на значение атрибута:
        // иначе документ раздаёт бомбу по атрибутам и получает по бюджету на
        // каждый, платя за это десятком байт разметки. Один разворот `&е5;`
        // обрабатывает 1 344 440 символов, значит из четырёх мебибайт
        // помещается ровно три, и четвёртый атрибут обязан упереться в
        // потолок — сорок атрибутов до конца не дойдут.
        let mut doc = entity_bomb(5, 10, "ааааааааа");
        doc.push_str("<к");
        for n in 0..40 {
            doc.push_str(&format!(" з{n}=\"&е5;\""));
        }
        doc.push_str("/>");
        let started = Instant::now();
        assert!(fails(&doc));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "общий бюджет обязан оборвать разбор на четвёртом атрибуте"
        );
    }

    #[test]
    fn entity_bomb_in_content_is_checked_without_expansion() {
        // В СОДЕРЖИМОМ подстановки нет (измерено — отдаётся узел-ссылка),
        // поэтому та же бомба разбирается успешно: проверка обходит граф
        // ссылок по именам и каждую сущность просматривает один раз.
        let doc = format!("{}<к>&е9;</к>", entity_bomb(9, 10, "ааааааааа"));
        let started = Instant::now();
        assert_eq!(
            events(&doc),
            vec![
                start("к"),
                XmlEvent::EntityReference { name: "е9".into() },
                end("к"),
            ]
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "проверка ссылки не должна ничего разворачивать"
        );
    }

    #[test]
    fn parameter_entity_bomb_hits_the_expansion_budget() {
        // Тот же фан-аут для `%имя;`: без бюджета разбор подмножества растёт
        // экспоненциально, оставаясь документом в полкилобайта.
        let mut doc = String::from("<!DOCTYPE к [<!ENTITY % п0 \"<!ELEMENT а EMPTY>\">");
        for level in 1..=9 {
            doc.push_str(&format!("<!ENTITY % п{level} \""));
            for _ in 0..10 {
                doc.push_str(&format!("%п{};", level - 1));
            }
            doc.push_str("\">");
        }
        doc.push_str("%п9;]><к/>");
        assert!(
            doc.len() < 1024,
            "документ-бомба — сотни байт: {}",
            doc.len()
        );
        let started = Instant::now();
        assert!(fails(&doc));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "бюджет обязан оборвать разворачивание сразу"
        );
    }

    #[test]
    fn deep_linear_entity_chain_expands_in_attribute() {
        // Тысяча РАЗЛИЧНЫХ сущностей цепочкой — рекурсии нет, значит и
        // ошибки быть не должно: глубина ограничена только числом
        // объявлений, а стек кадров лежит в куче.
        let mut doc = String::from("<!DOCTYPE к [<!ENTITY е0 \"х\">");
        for level in 1..1000 {
            doc.push_str(&format!("<!ENTITY е{level} \"&е{};\">", level - 1));
        }
        doc.push_str("]><к з=\"&е999;\"/>");
        let ev = events(&doc);
        let XmlEvent::ElementStart { attrs, .. } = &ev[0] else {
            panic!("первым событием обязан быть начальный тег");
        };
        assert_eq!(attrs[0].value, "х");
        // Две ссылки на одну сущность рядом — не рекурсия: в развороте
        // находится стек имён, а не множество уже виденных.
        let ev = events("<!DOCTYPE к [<!ENTITY а \"х\"><!ENTITY б \"&а;&а;\">]><к з=\"&б;\"/>");
        let XmlEvent::ElementStart { attrs, .. } = &ev[0] else {
            panic!("первым событием обязан быть начальный тег");
        };
        assert_eq!(attrs[0].value, "хх");
    }

    #[test]
    fn deep_linear_parameter_entity_chain_parses() {
        let mut doc = String::from("<!DOCTYPE к [<!ENTITY % п0 \"<!ELEMENT к EMPTY>\">");
        for level in 1..1000 {
            doc.push_str(&format!("<!ENTITY % п{level} \"%п{};\">", level - 1));
        }
        doc.push_str("%п999;]><к/>");
        assert_eq!(events(&doc), vec![start("к"), end("к")]);
    }

    #[test]
    fn recursive_entities_are_rejected_at_the_reference() {
        // Детектор точный — имя, уже находящееся в развороте, — поэтому
        // ошибкой остаётся и прямая рекурсия, и взаимная, и через третью
        // сущность, и в содержимом, где подстановки нет вовсе.
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"&е;\">]><к з=\"&е;\"/>"));
        assert!(fails("<!DOCTYPE к [<!ENTITY е \"&е;\">]><к>&е;</к>"));
        let mutual = "<!DOCTYPE к [<!ENTITY а \"&б;\"><!ENTITY б \"&а;\">]>";
        assert!(fails(&format!("{mutual}<к з=\"&а;\"/>")));
        assert!(fails(&format!("{mutual}<к>&а;</к>")));
        let ring = "<!DOCTYPE к [<!ENTITY а \"&б;\"><!ENTITY б \"&в;\"><!ENTITY в \"&а;\">]>";
        assert!(fails(&format!("{ring}<к з=\"&а;\"/>")));
        assert!(fails(&format!("{ring}<к>&а;</к>")));
        assert!(fails(
            "<!DOCTYPE к [<!ENTITY % а \"%б;\"><!ENTITY % б \"%а;\">%а;]><к/>"
        ));
    }
}
