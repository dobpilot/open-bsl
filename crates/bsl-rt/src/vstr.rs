//! Внутренний строковый формат платформы: `ЗначениеВСтрокуВнутр` и
//! `ЗначениеИзСтрокиВнутр`.
//!
//! Формат недокументирован и целиком ИЗМЕРЕН на 8.3.27 (разведочная
//! батарея, затем контракт `tests/conformance/measure/measure-vstr.bsl`):
//!
//! * скаляры — `{"N",0.1}`, `{"S","а""б"}` (кавычка удваивается, переводы
//!   строк и табуляция лежат сырыми), `{"B",1}`/`{"B",0}`,
//!   `{"D",ГГГГММДДччммсс}` (всегда 14 цифр, без кавычек), `{"U"}` —
//!   `Неопределено`, `{"L"}` — `NULL`, `{"T",<uuid типа>}`;
//! * коллекции — `{"#",<uuid вида>,` перевод строки, `{<число элементов>`,
//!   затем перед КАЖДЫМ элементом `,` и перевод строки, после последнего —
//!   перевод строки, `}`, перевод строки, `}`. Пустая коллекция несёт
//!   `{0}` без внутренних переводов. Элемент `Структуры` — тройка строк
//!   `{`, `{"S","Имя"},`, `<значение>`, `}`; элемент `Соответствия` — то
//!   же с произвольным ключом на месте имени.
//!
//! Число печатается ровно тем же видом, что `Формат(x, "ЧГ=0; ЧРД=.")` —
//! на `1/2^28` платформа выдала в сериализации в точности строку якоря
//! `NUM.DIV.EXACT_TIE`, поэтому здесь используется тот же канонический
//! вывод `BslNumber::to_canonical`.
//!
//! Ссылочная целостность: ИЗМЕРЕНО, что платформа её НЕ сохраняет.
//! Повторные вхождения одного объекта разворачиваются в независимые копии
//! (проба `REF.TWICE`), и обратное чтение на самой платформе возвращает
//! копии (`RT.IDENTITY`); циклическое значение роняет клиент 1С
//! переполнением стека. Здесь повторы сериализуются так же — копиями, —
//! а цикл и сверхглубокая вложенность дают перехватываемую
//! [`RtError::StackOverflow`] вместо падения процесса: краш платформы —
//! не то поведение, которое стоит воспроизводить.

use std::collections::HashMap;
use std::rc::Rc;

use bsl_number::BslNumber;

use crate::date::BslDate;
use crate::object::BslObject;
use crate::runtime_shapes::RuntimeShapes;
use crate::types::TypeId;
use crate::{BslString, BslValue, RtError, RtResult};

/// Идентификаторы видов сериализуемых объектов — измерены на 8.3.27.
const ARRAY_ID: &str = "51e7a0d2-530b-11d4-b98a-008048da3034";
/// `ФиксированныйМассив` — полезная нагрузка та же, что у массива.
const FIXED_ARRAY_ID: &str = "4500381b-db30-4a10-9db4-990038032acf";
const STRUCTURE_ID: &str = "4238019d-7e49-4fc9-91db-b6b951d5cf8e";
const MAP_ID: &str = "3d48feae-a9c6-4c5a-a099-9eb6477630c6";
const VALUE_TABLE_ID: &str = "acf6192e-81ca-46ef-93a6-5a6968b78663";
/// `СписокЗначений` и `УникальныйИдентификатор` не материализуются:
/// разбор их идентификаторы не сверяет — любой незнакомый вид уходит в
/// непрозрачное значение целиком. Константы документируют измеренные
/// GUID и используются транзитными тестами, поэтому вне тестовой сборки
/// они законно «не нужны».
#[cfg_attr(not(test), allow(dead_code))]
const VALUE_LIST_ID: &str = "4772b3b4-f4a3-49c0-a1a5-8cb5961511a3";
#[cfg_attr(not(test), allow(dead_code))]
const UUID_VALUE_ID: &str = "fc01b5df-97fe-449b-83d4-218a090e681e";
const TYPE_DESCRIPTION_ID: &str = "f5c65050-3bbb-11d5-b988-0050bae0a95d";

/// UUID ТИПОВ для `{"T",…}` — измерены на 8.3.27 поимённой батареей.
/// У коллекций UUID типа СОВПАДАЕТ с UUID вида объекта в `{"#",…}` —
/// это наблюдение платформы, а не наше упрощение.
const TYPE_UUIDS: &[(TypeId, &str)] = &[
    (TypeId::String, "9b6abf8b-0173-48e5-b0a0-83b21fcf63c5"),
    (TypeId::Number, "b0be78f2-0ee6-4d31-a3bb-77dd32ba5bec"),
    (TypeId::Date, "aae38c48-a877-411c-a6d3-fbaa1f83c4bd"),
    (TypeId::Boolean, "5d4125ad-f6e7-4313-be32-f71d0ab60915"),
    (TypeId::Undefined, "ee8d3e7c-f930-4a76-8aad-4ff9083a6ea6"),
    (TypeId::Null, "af40a278-63bc-478e-91a8-19e0d16b10b5"),
    (TypeId::Type, "d47d59f8-73f0-481c-8b5e-f6384c0a4804"),
    (TypeId::Array, ARRAY_ID),
    (TypeId::Structure, STRUCTURE_ID),
    (TypeId::Map, MAP_ID),
    (TypeId::ValueTable, VALUE_TABLE_ID),
    (TypeId::TypeDescription, TYPE_DESCRIPTION_ID),
];

/// Предел вложенности при записи и чтении — той же природы, что
/// `MAX_JSON_DEPTH`: обе стороны рекурсивны, и глубина входа напрямую
/// расходует стек Rust. Платформа предела не имеет и на цикле падает
/// (измерено); здесь вместо этого перехватываемая ошибка.
const MAX_VSTR_DEPTH: usize = 500;

fn err(msg: impl Into<String>) -> RtError {
    RtError::Vstr(msg.into())
}

// --- Запись ----------------------------------------------------------------

/// `ЗначениеВСтрокуВнутр(Значение)`.
///
/// Запись потоковая: значение печатается сразу в выходную строку через
/// `Writer`, без промежуточного дерева. Дерево `Node` осталось только
/// у разбора — там отдельный слой оправдан контекстной интерпретацией
/// лексем, а на записи оно стоило дороже самой печати: на таблице в
/// полмиллиона строк аллокации и освобождение узлов занимали больше
/// времени, чем формирование текста.
///
/// # Errors
///
/// [`RtError::Vstr`] на значении, которого во внутреннем формате нет
/// (объекты с состоянием вроде `ЧтениеJSON`), и
/// [`RtError::StackOverflow`] на циклической или сверхглубокой структуре.
pub fn value_to_string_internal(v: &BslValue, rt: &RuntimeShapes) -> RtResult<String> {
    let mut path: Vec<*const BslObject> = Vec::new();
    let mut w = Writer::new();
    value_to_writer(v, rt, &mut path, &mut w)?;
    Ok(w.finish())
}

/// Состояние одного открытого списка у [`Writer`].
struct ListState {
    /// В списке ещё не напечатано ни одного элемента.
    first: bool,
    /// Последний напечатанный элемент был списком или сырым поддеревом.
    last_was_list: bool,
}

/// Потоковый писатель внутреннего формата.
///
/// Правило переводов строк ИЗМЕРЕНО и одно на весь формат: внутри списка
/// перевод строки ставится перед каждым элементом-СПИСКОМ, и перед
/// закрывающей скобкой, если последний элемент — список; лексемы и
/// строки идут через запятую без переводов. Это правило воспроизводит все
/// снятые образцы — от `{"N",42}` до разметки `ТаблицыЗначений`.
struct Writer {
    out: String,
    /// Стек открытых списков; на верхнем уровне значения пуст.
    stack: Vec<ListState>,
}

impl Writer {
    fn new() -> Self {
        Writer {
            out: String::new(),
            stack: Vec::new(),
        }
    }

    fn finish(self) -> String {
        self.out
    }

    /// Разделители перед очередным элементом текущего списка.
    fn begin_item(&mut self, is_list: bool) {
        if let Some(top) = self.stack.last_mut() {
            if !top.first {
                self.out.push(',');
            }
            top.first = false;
            top.last_was_list = is_list;
            if is_list {
                self.out.push('\n');
            }
        }
    }

    fn open(&mut self) {
        self.begin_item(true);
        self.out.push('{');
        self.stack.push(ListState {
            first: true,
            last_was_list: false,
        });
    }

    fn close(&mut self) {
        let top = self.stack.pop().expect("close без парного open");
        if top.last_was_list {
            self.out.push('\n');
        }
        self.out.push('}');
    }

    /// Голая лексема; принимает всё печатаемое, чтобы числа и
    /// идентификаторы строк не проходили через промежуточную `String`.
    fn bare(&mut self, text: impl std::fmt::Display) {
        use std::fmt::Write;
        self.begin_item(false);
        let _ = write!(self.out, "{text}");
    }

    /// Строковая лексема в кавычках.
    fn quoted(&mut self, s: &str) {
        self.begin_item(false);
        write_quoted(&mut self.out, s);
    }

    /// Строковое значение BSL: кавычки и удвоение ставятся прямо по
    /// UTF-16-юнитам, без промежуточной `String`. Декодирование то же
    /// lossy, что у `Display` строки.
    fn quoted_units(&mut self, s: &BslString) {
        self.begin_item(false);
        self.out.push('"');
        for ch in char::decode_utf16(s.units().iter().copied()) {
            let ch = ch.unwrap_or(char::REPLACEMENT_CHARACTER);
            if ch == '"' {
                self.out.push('"');
            }
            self.out.push(ch);
        }
        self.out.push('"');
    }

    /// Уже отрисованное поддерево непрозрачного значения — вставляется как
    /// есть. Оно всегда начинается со скобки, поэтому в правиле переводов
    /// строк ведёт себя как список.
    fn raw(&mut self, text: &str) {
        self.begin_item(true);
        self.out.push_str(text);
    }
}

fn value_to_writer(
    v: &BslValue,
    rt: &RuntimeShapes,
    path: &mut Vec<*const BslObject>,
    w: &mut Writer,
) -> RtResult<()> {
    match v {
        BslValue::Undefined => {
            w.open();
            w.quoted("U");
            w.close();
        }
        BslValue::Null => {
            w.open();
            w.quoted("L");
            w.close();
        }
        BslValue::Boolean(b) => {
            w.open();
            w.quoted("B");
            w.bare(if *b { "1" } else { "0" });
            w.close();
        }
        BslValue::Number(n) => {
            w.open();
            w.quoted("N");
            w.bare(n.to_canonical());
            w.close();
        }
        BslValue::Str(s) => {
            w.open();
            w.quoted("S");
            w.quoted_units(s);
            w.close();
        }
        BslValue::Date(d) => {
            let c = d.to_civil();
            w.open();
            w.quoted("D");
            w.bare(format_args!(
                "{:04}{:02}{:02}{:02}{:02}{:02}",
                c.year, c.month, c.day, c.hour, c.minute, c.second
            ));
            w.close();
        }
        BslValue::Type(t) => {
            let uuid = TYPE_UUIDS
                .iter()
                .find(|(known, _)| known == t)
                .map(|(_, uuid)| *uuid)
                .ok_or_else(|| {
                    err(format!(
                        "UUID типа «{}» во внутреннем формате не измерен",
                        v.type_name()
                    ))
                })?;
            w.open();
            w.quoted("T");
            w.bare(uuid);
            w.close();
        }
        BslValue::Object(o) => {
            // Повтор объекта НА ТЕКУЩЕМ ПУТИ — цикл: платформа на нём
            // падает, здесь — ошибка. Повтор в разных ветках (не на пути)
            // законен и разворачивается в копию, как делает платформа.
            let ptr = Rc::as_ptr(o);
            if path.contains(&ptr) {
                return Err(RtError::StackOverflow {
                    what: "циклическое значение в ЗначениеВСтрокуВнутр",
                });
            }
            if path.len() >= MAX_VSTR_DEPTH {
                return Err(RtError::StackOverflow {
                    what: "слишком глубокая вложенность в ЗначениеВСтрокуВнутр",
                });
            }
            path.push(ptr);
            let result = object_to_writer(o, v.type_name(), rt, path, w);
            path.pop();
            result?;
        }
        other => {
            return Err(err(format!(
                "значение типа «{}» не представимо во внутреннем формате",
                other.type_name()
            )))
        }
    }
    Ok(())
}

fn object_to_writer(
    o: &Rc<BslObject>,
    type_name: &'static str,
    rt: &RuntimeShapes,
    path: &mut Vec<*const BslObject>,
    w: &mut Writer,
) -> RtResult<()> {
    match &**o {
        BslObject::Array(items) => {
            // Снимок до записи — по той же причине, что в `json.rs`:
            // элемент может оказаться тем же массивом, и вложенное
            // заимствование `RefCell` этого не переживёт.
            let snapshot: Vec<BslValue> = items.borrow().clone();
            w.open();
            w.quoted("#");
            w.bare(ARRAY_ID);
            w.open();
            w.bare(snapshot.len());
            for item in &snapshot {
                value_to_writer(item, rt, path, w)?;
            }
            w.close();
            w.close();
        }
        BslObject::Structure(s) => {
            let entries: Vec<(String, BslValue)> = {
                let s = s.borrow();
                (0..s.len())
                    .filter_map(|i| s.entry_at(i))
                    .filter_map(|(id, v)| rt.names.name(id).map(|n| (n.to_string(), v)))
                    .collect()
            };
            w.open();
            w.quoted("#");
            w.bare(STRUCTURE_ID);
            w.open();
            w.bare(entries.len());
            for (name, value) in &entries {
                w.open();
                w.open();
                w.quoted("S");
                w.quoted(name);
                w.close();
                value_to_writer(value, rt, path, w)?;
                w.close();
            }
            w.close();
            w.close();
        }
        BslObject::Map(data) => {
            // Порядок пар у платформы — внутренний порядок её хеш-таблицы:
            // на трёх ключах он совпал с обратным порядком вставки, на
            // четырёх — уже нет. Точного правила нет, а семантика
            // `Соответствия` неупорядоченная, поэтому здесь ПРЯМОЙ порядок
            // вставки: чтение вставляет пары в порядке текста, и повторная
            // сериализация возвращает строку платформы байт в байт —
            // транзитная стабильность важнее похожести на её хеш-порядок.
            // Строка `VSTR.FORMAT.MAP.ORDER` контракта сходиться не обязана
            // (см. шапку measure-vstr.bsl).
            let entries: Vec<(BslValue, BslValue)> = {
                let d = data.borrow();
                (0..d.len()).filter_map(|i| d.entry_at(i)).collect()
            };
            w.open();
            w.quoted("#");
            w.bare(MAP_ID);
            w.open();
            w.bare(entries.len());
            for (key, value) in &entries {
                w.open();
                value_to_writer(key, rt, path, w)?;
                value_to_writer(value, rt, path, w)?;
                w.close();
            }
            w.close();
            w.close();
        }
        BslObject::ValueTable(data) => table_to_writer(data, rt, path, w)?,
        // `ОписаниеТипов` — тот же `{"Pattern",…}`, что у колонок таблицы,
        // под своим видом объекта (измерено, пробы `CMP.VALUE*`).
        BslObject::TypeDescription(type_ids) => {
            w.open();
            w.quoted("#");
            w.bare(TYPE_DESCRIPTION_ID);
            w.open();
            w.quoted("Pattern");
            for letter in canonical_letters(type_ids)? {
                w.open();
                w.quoted(letter);
                w.close();
            }
            w.close();
            w.close();
        }
        // Непрозрачное значение возвращается в текст ровно тем поддеревом,
        // из которого было прочитано, — транзит без потерь.
        BslObject::VstrOpaque(text) => w.raw(text),
        _ => {
            return Err(err(format!(
                "значение типа «{type_name}» не представимо во внутреннем формате"
            )))
        }
    }
    Ok(())
}

fn write_quoted(out: &mut String, s: &str) {
    // Кавычка удваивается; всё остальное, включая переводы строк и
    // табуляцию, платформа держит в строке сырым (измерено).
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push('"');
        }
        out.push(ch);
    }
    out.push('"');
}

// --- ТаблицаЗначений --------------------------------------------------------
//
// Разметка ИЗМЕРЕНА двумя батареями (16 проб: пустая таблица, разные
// прямоугольники, типы и квалификаторы колонок, пропуски `Неопределено`):
//
//   {9, <колонки>, <блок строк>, {0,0}}
//   колонка:    {<индекс>, "Имя", <ОписаниеТипов>, "Заголовок", <Ширина>}
//   ОписаниеТипов: {"Pattern"} без ограничения; {"Pattern",{"N"},…} с
//               типами (буква как у скаляров; после буквы могут идти
//               квалификаторы: {"N",10,2,0}, {"S",20,1})
//   блок строк: {2, <n колонок>, <к,к на каждую колонку>,
//               {1, <n строк>, <строка>…}, <n колонок - 1>, <n строк - 1>}
//   строка:     {2, <индекс>, <n значений>, <значение>…, 0} — хвостовые
//               `Неопределено` отброшены, внутренние пишутся явным {"U"};
//               `0` и `Ложь` сохраняются.

/// Буквы типов в `{"Pattern",…}` — те же теги, что у скалярных значений,
/// В КАНОНИЧЕСКОМ ПОРЯДКЕ ПЛАТФОРМЫ: измерено на всех перестановках
/// (`CMP.*` разведочной батареи), что порядок задания в `ОписаниеТипов`
/// не влияет — платформа всегда пишет `B`, `S`, `D`, `N`.
const COLUMN_TYPE_LETTERS: &[(TypeId, &str)] = &[
    (TypeId::Boolean, "B"),
    (TypeId::String, "S"),
    (TypeId::Date, "D"),
    // Позиция `Null` в каноне видна из реальных выгрузок: `B,L`, `S,…,L`,
    // `D,…,L`, но `L,N` — между датой и числом.
    (TypeId::Null, "L"),
    (TypeId::Number, "N"),
];

fn letter_of(id: TypeId) -> Option<&'static str> {
    COLUMN_TYPE_LETTERS
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, letter)| *letter)
}

/// Буквы описания типов в каноническом порядке платформы; дубликаты
/// схлопываются самим обходом канона.
fn canonical_letters(type_ids: &[TypeId]) -> RtResult<Vec<&'static str>> {
    for type_id in type_ids {
        if !COLUMN_TYPE_LETTERS
            .iter()
            .any(|(known, _)| known == type_id)
        {
            return Err(err(format!(
                "тип «{:?}» в описании типов не представим во внутреннем формате",
                type_id
            )));
        }
    }
    Ok(COLUMN_TYPE_LETTERS
        .iter()
        .filter(|(known, _)| type_ids.contains(known))
        .map(|(_, letter)| *letter)
        .collect())
}

fn table_to_writer(
    data: &Rc<std::cell::RefCell<crate::ValueTableData>>,
    rt: &RuntimeShapes,
    path: &mut Vec<*const BslObject>,
    w: &mut Writer,
) -> RtResult<()> {
    // Снимок — по той же причине, что у массива: ячейка может ссылаться
    // на объект, чья запись снова заглянет в таблицу.
    let (names, types, extras, columns) = {
        let t = data.borrow();
        (
            t.column_names.clone(),
            t.column_types.clone(),
            t.column_vstr.clone(),
            t.columns.clone(),
        )
    };
    let ncols = names.len();
    let nrows = columns.first().map_or(0, |c| c.len());
    let row_ids = data.borrow().row_ids.clone();

    // Обёртка `{"#",<uuid>,` и полезная нагрузка `{9, колонки, блок строк,
    // {0,0}}` — разметка описана в обзоре раздела выше.
    w.open();
    w.quoted("#");
    w.bare(VALUE_TABLE_ID);
    w.open();
    w.bare("9");

    w.open();
    w.bare(ncols);
    for (idx, name) in names.iter().enumerate() {
        let extra = extras.get(idx).cloned().unwrap_or_default();
        w.open();
        // Идентификатор колонки: прочитанный из исходной строки или
        // позиция — у свежей таблицы платформы они совпадают.
        w.bare(extra.id.clone().unwrap_or_else(|| idx.to_string()));
        w.quoted(name);
        // Колонка, прочитанная из строки платформы, несёт сырое описание
        // типов (включая квалификаторы, которых модель колонок не хранит)
        // — оно возвращается в текст как есть ради тождественного
        // транзита. Колонка, созданная кодом, строит описание из типов.
        if let Some(raw) = &extra.raw_pattern {
            w.raw(raw);
        } else {
            w.open();
            w.quoted("Pattern");
            if let Some(Some(column_types)) = types.get(idx) {
                let ids: Vec<crate::TypeId> = column_types.iter().map(|t| t.id).collect();
                for letter in canonical_letters(&ids).map_err(|_| {
                    err(format!(
                        "тип колонки «{name}» не представим во внутреннем формате"
                    ))
                })? {
                    let quals = column_types
                        .iter()
                        .find(|t| letter_of(t.id) == Some(letter))
                        .map(|t| t.quals.as_slice())
                        .unwrap_or(&[]);
                    w.open();
                    w.quoted(letter);
                    for q in quals {
                        w.bare(q);
                    }
                    w.close();
                }
            }
            w.close();
        }
        w.quoted(&extra.title);
        w.bare(if extra.width.is_empty() {
            "0"
        } else {
            &extra.width
        });
        w.close();
    }
    w.close();

    // Пары заголовка — (позиция, идентификатор колонки): на свежей
    // таблице это (к, к), после удаления колонок идентификаторы разрежены
    // (измерено на реальных выгрузках).
    let col_id_of = |k: usize| -> String {
        extras
            .get(k)
            .and_then(|e| e.id.clone())
            .unwrap_or_else(|| k.to_string())
    };
    w.open();
    w.bare("2");
    w.bare(ncols);
    for k in 0..ncols {
        w.bare(k);
        w.bare(col_id_of(k));
    }
    w.open();
    w.bare("1");
    w.bare(nrows);
    for row in 0..nrows {
        // Хвостовые `Неопределено` отброшены, внутренние пишутся явно.
        let mut nvals = ncols;
        while nvals > 0 && matches!(columns[nvals - 1][row], BslValue::Undefined) {
            nvals -= 1;
        }
        w.open();
        w.bare("2");
        // Вторая лексема — СТАБИЛЬНЫЙ идентификатор строки, не порядковый
        // номер: на свежей таблице они совпадают, но после
        // `Свернуть`/`Скопировать` платформа сохраняет исходные номера
        // (видно на реальных выгрузках), и транзит обязан их вернуть.
        w.bare(row_ids.get(row).copied().unwrap_or(row as u64));
        w.bare(nvals);
        for col in &columns[..nvals] {
            value_to_writer(&col[row], rt, path, w)?;
        }
        w.bare("0");
        w.close();
    }
    w.close();
    // `X` — прочитанное сырьё, если таблица пришла из внутреннего формата;
    // иначе максимальный идентификатор колонки (так пишет платформа для
    // таблиц, не проходивших `Скопировать` со списком колонок).
    let tail_x = data.borrow().vstr_tail_x.clone().unwrap_or_else(|| {
        (0..ncols)
            .map(|k| col_id_of(k).parse::<i64>().unwrap_or(k as i64))
            .max()
            .unwrap_or(-1)
            .to_string()
    });
    w.bare(tail_x);
    // `Y` — максимальный идентификатор строки: совпадает с платформой на
    // всех девяти реальных выгрузках и на всех свежих таблицах.
    let tail_y = row_ids.iter().max().map_or(-1, |m| *m as i64);
    w.bare(tail_y);
    w.close();

    w.open();
    w.bare("0");
    w.bare("0");
    w.close();
    w.close();
    w.close();
    Ok(())
}

// --- Файловая пара ----------------------------------------------------------

/// `ЗначениеВФайл(ИмяФайла, Значение)`.
///
/// Платформа пишет UTF-8 С BOM и переводами строк CRLF — включая переводы
/// ВНУТРИ строковых значений (измерено побайтовым сравнением файлов обеих
/// сторон); обратное чтение нормализует пары обратно, поэтому значение
/// дорогу переживает. Одно сознательное расхождение: запись в
/// несуществующий каталог у платформы МОЛЧА не делает ничего — здесь это
/// ошибка, молчаливая потеря данных хуже несовместимости.
///
/// # Errors
///
/// Ошибки сериализации значения и файлового ввода-вывода.
pub fn value_to_file(path: &str, v: &BslValue, rt: &RuntimeShapes) -> RtResult<()> {
    use std::io::Write;

    let text = value_to_string_internal(v, rt)?;
    let io_err = |e: std::io::Error| RtError::IoError(format!("ЗначениеВФайл: {e}"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).map_err(io_err)?);
    out.write_all("\u{feff}".as_bytes()).map_err(io_err)?;
    // Перевод LF в CRLF — кусками между переводами строк, без
    // посимвольного декодирования: байт `\n` не встречается внутри
    // многобайтовых последовательностей UTF-8.
    let mut first = true;
    for chunk in text.split('\n') {
        if !first {
            out.write_all(b"\r\n").map_err(io_err)?;
        }
        first = false;
        out.write_all(chunk.as_bytes()).map_err(io_err)?;
    }
    out.flush().map_err(io_err)
}

/// `ЗначениеИзФайла(ИмяФайла)`.
///
/// BOM срезается, пары `\r\n` внутри строковых лексем нормализуются в
/// `\n` прямо при сканировании — так строковые значения возвращаются в
/// исходный вид (измерено: платформа из своего же файла читает строку
/// БЕЗ `\r`), а полная копия текста ради `replace` не нужна. Между
/// лексемами `\r` — обычный пробельный символ. Одиночный `\r` в данных
/// не трогается.
// НЕ ИЗМЕРЕНО(VSTR.FORMAT) — поведение платформенной пары на одиночном
// `\r` внутри строкового значения; здесь он проходит как есть.
///
/// # Errors
///
/// Ошибки файлового ввода-вывода и разбора внутреннего формата.
pub fn value_from_file(path: &str, rt: &mut RuntimeShapes) -> RtResult<BslValue> {
    // Байты, а не `read_to_string`: валидация UTF-8 идёт лениво, по
    // материализуемым лексемам, — не-UTF-8 в данных даёт ошибку разбора,
    // а не чтения файла.
    let raw = std::fs::read(path).map_err(|e| RtError::IoError(format!("ЗначениеИзФайла: {e}")))?;
    let text = raw.strip_prefix("\u{feff}".as_bytes()).unwrap_or(&raw);
    parse_and_convert(text, true, rt)
}

// --- Чтение ----------------------------------------------------------------

/// `ЗначениеИзСтрокиВнутр(Строка)`.
///
/// Разбор терпим к пробелам и переводам строк между лексемами: платформа
/// принимает и «плотную» запись без переводов (измерено), поэтому решает
/// только структура скобок и запятых.
///
/// # Errors
///
/// [`RtError::Vstr`] на тексте, не являющемся внутренним форматом, и на
/// видах объектов, которых в этой реализации нет.
pub fn value_from_string_internal(text: &str, rt: &mut RuntimeShapes) -> RtResult<BslValue> {
    // В отличие от файловой пары, строка из памяти разбирается как есть:
    // нормализация `\r\n` внутри строковых лексем на платформе для этого
    // пути не измерена.
    parse_and_convert(text.as_bytes(), false, rt)
}

fn parse_and_convert(text: &[u8], crlf_to_lf: bool, rt: &mut RuntimeShapes) -> RtResult<BslValue> {
    let mut r = Reader {
        p: Parser {
            text,
            pos: 0,
            crlf_to_lf,
        },
        rt,
        strings: ValueCache::new(),
        numbers: ValueCache::new(),
        dates: ValueCache::new(),
        opaques: ValueCache::new(),
    };
    let value = r.read_value(0)?;
    r.p.skip_ws();
    if r.p.pos != r.p.text.len() {
        return Err(err("лишний текст после значения во внутреннем формате"));
    }
    Ok(value)
}

/// Синтаксическое дерево РАЗБОРА: до интерпретации тегов текст — это
/// просто вложенные списки из строковых и «голых» лексем. Отдельный слой
/// нужен, потому что смысл лексемы зависит от вида объекта: в `{"D",…}`
/// голая лексема — дата с ведущими нулями, в теле `ТаблицыЗначений` —
/// служебное число. Запись деревом не пользуется — она потоковая
/// (см. [`Writer`]).
#[derive(Clone)]
enum Node {
    List(Vec<Node>),
    Str(String),
    Bare(String),
}

/// Курсор по байтам исходного текста. Вся структура формата — скобки,
/// запятые, кавычки, пробельные символы — это ASCII, поэтому разбор идёт
/// по байтам без посимвольного декодирования. Валидация UTF-8 ЛЕНИВАЯ:
/// файл читается байтами, а в `&str` превращаются только материализуемые
/// лексемы — вместе с интерн-кэшем [`Reader`] это значит, что валидируются
/// только уникальные значения, а не все мегабайты входа. Раньше здесь был
/// `Vec<char>` — на файле в 130 МБ он стоил полгигабайта пиковой памяти и
/// отдельного прохода декодирования.
struct Parser<'a> {
    text: &'a [u8],
    pos: usize,
    /// Нормализовать `\r\n` в `\n` внутри строковых лексем — режим
    /// файловой пары (см. [`value_from_file`]); `ЗначениеИзСтрокиВнутр`
    /// оставляет строку как есть.
    crlf_to_lf: bool,
}

/// Лексема как `&str`: единственная точка валидации UTF-8 при чтении.
fn utf8(bytes: &[u8]) -> RtResult<&str> {
    std::str::from_utf8(bytes).map_err(|_| err("текст во внутреннем формате не в UTF-8"))
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn parse_node(&mut self, depth: usize) -> RtResult<Node> {
        if depth > MAX_VSTR_DEPTH {
            return Err(RtError::StackOverflow {
                what: "слишком глубокая вложенность в ЗначениеИзСтрокиВнутр",
            });
        }
        self.skip_ws();
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(Node::List(items));
                }
                loop {
                    items.push(self.parse_node(depth + 1)?);
                    self.skip_ws();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(Node::List(items));
                        }
                        _ => return Err(err("ожидалась «,» или «}» во внутреннем формате")),
                    }
                }
            }
            Some(b'"') => {
                self.pos += 1;
                let mut s = String::new();
                loop {
                    // Ровный кусок до ближайшего особого байта уходит в
                    // строку одним `push_str`, без обхода по символам.
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if c == b'"' || (self.crlf_to_lf && c == b'\r') {
                            break;
                        }
                        self.pos += 1;
                    }
                    s.push_str(utf8(&self.text[start..self.pos])?);
                    match self.peek() {
                        Some(b'"') => {
                            self.pos += 1;
                            // Удвоенная кавычка — экранированная; одиночная
                            // закрывает строку.
                            if self.peek() == Some(b'"') {
                                s.push('"');
                                self.pos += 1;
                            } else {
                                return Ok(Node::Str(s));
                            }
                        }
                        Some(b'\r') => {
                            self.pos += 1;
                            // Пара `\r\n` схлопывается в `\n`, одиночный
                            // `\r` остаётся — та же семантика, что была у
                            // предварительного `replace("\r\n", "\n")`.
                            if self.peek() != Some(b'\n') {
                                s.push('\r');
                            }
                        }
                        _ => return Err(err("незакрытая строка во внутреннем формате")),
                    }
                }
            }
            Some(_) => {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if matches!(c, b'{' | b'}' | b',' | b'"' | b' ' | b'\n' | b'\r' | b'\t') {
                        break;
                    }
                    self.pos += 1;
                }
                if self.pos == start {
                    return Err(err("неожиданный символ во внутреннем формате"));
                }
                Ok(Node::Bare(utf8(&self.text[start..self.pos])?.to_string()))
            }
            None => Err(err("неожиданный конец текста во внутреннем формате")),
        }
    }

    /// Начинается ли на курсоре голая лексема (после `skip_ws`).
    fn at_bare(&self) -> bool {
        !matches!(
            self.peek(),
            Some(b'{' | b'}' | b',' | b'"' | b' ' | b'\n' | b'\r' | b'\t') | None
        )
    }

    /// Голая лексема срезом исходника; пустая — ошибка, как в
    /// [`Parser::parse_node`].
    fn read_bare_tok(&mut self) -> RtResult<&'a [u8]> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, b'{' | b'}' | b',' | b'"' | b' ' | b'\n' | b'\r' | b'\t') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(err("неожиданный символ во внутреннем формате"));
        }
        Ok(&self.text[start..self.pos])
    }

    /// Строковая лексема СЫРЫМ срезом между кавычками: удвоение кавычек и
    /// пары `\r\n` не развёрнуты — такой срез служит ключом кэша значений.
    /// Курсор должен стоять на открывающей кавычке.
    fn read_quoted_raw(&mut self) -> RtResult<&'a [u8]> {
        self.pos += 1;
        let start = self.pos;
        loop {
            match self.peek() {
                Some(b'"') => {
                    if self.text.get(self.pos + 1) == Some(&b'"') {
                        self.pos += 2;
                    } else {
                        let end = self.pos;
                        self.pos += 1;
                        return Ok(&self.text[start..end]);
                    }
                }
                Some(_) => self.pos += 1,
                None => return Err(err("незакрытая строка во внутреннем формате")),
            }
        }
    }

    /// Конец поддерева, начинающегося на `start` со скобки `{`: чистый
    /// байтовый скан со счётом скобок вне строковых лексем, без разбора и
    /// аллокаций. Удвоенная кавычка переключает признак строки дважды и
    /// потому учитывается сама собой. `None` — текст обрывается; ошибку
    /// с точным сообщением тогда даёт настоящий разбор.
    fn subtree_end(&self, start: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut in_string = false;
        for (i, &b) in self.text[start..].iter().enumerate() {
            match b {
                b'"' => in_string = !in_string,
                b'{' if !in_string => depth += 1,
                b'}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(start + i + 1);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// Развёртка сырой строковой лексемы: `""` схлопывается в `"`, в файловом
/// режиме пара `\r\n` — в `\n` (одиночный `\r` остаётся). `None` — правок
/// не нужно, содержимое годится срезом как есть; это обычный случай.
fn unquote(raw: &str, crlf_to_lf: bool) -> Option<String> {
    if !raw.contains('"') && !(crlf_to_lf && raw.contains('\r')) {
        return None;
    }
    let bytes = raw.as_bytes();
    let mut s = String::with_capacity(raw.len());
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            // Кусок вместе с первой кавычкой, вторая пропускается.
            b'"' => {
                s.push_str(&raw[start..=i]);
                i += 2;
                start = i;
            }
            b'\r' if crlf_to_lf => {
                s.push_str(&raw[start..i]);
                i += 1;
                if bytes.get(i) != Some(&b'\n') {
                    s.push('\r');
                }
                start = i;
            }
            _ => i += 1,
        }
    }
    s.push_str(&raw[start..]);
    Some(s)
}

/// Хешер интерн-кэшей — FxHash: восемь байтов за шаг, умножение с
/// поворотом. Ключи кэшей — срезы исходного текста, их миллионы, и
/// стойкий к затравке SipHash стандартной таблицы на них заметен в
/// профиле; кэш живёт не дольше одного вызова чтения, атак на затравку
/// тут нет.
#[derive(Default)]
struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let word = u64::from_le_bytes(chunk.try_into().expect("ровно восемь байтов"));
            self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
        }
        let mut tail = 0u64;
        for &b in chunks.remainder().iter().rev() {
            tail = (tail << 8) | u64::from(b);
        }
        self.0 = (self.0.rotate_left(5) ^ tail).wrapping_mul(SEED);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Интерн-кэш чтения: срез исходника — готовое значение. Кэш
/// самоотключается на данных без повторов: колонка уникальных ключей
/// иначе оплачивает и хеш, и рехеш растущей таблицы, ничего не получая
/// взамен, — а решить заранее, какие лексемы будут повторяться, нельзя.
struct ValueCache<'a> {
    map: HashMap<&'a [u8], BslValue, std::hash::BuildHasherDefault<FxHasher>>,
    hits: u64,
    misses: u64,
}

impl<'a> ValueCache<'a> {
    fn new() -> Self {
        ValueCache {
            map: HashMap::default(),
            hits: 0,
            misses: 0,
        }
    }

    /// Пока идёт разогрев, кэш работает всегда; после — только если
    /// попадание хотя бы каждое четвёртое. Счётчики продолжают жить,
    /// поэтому решение принимается по всей истории, а не по окну.
    fn active(&self) -> bool {
        self.misses < 65_536 || self.hits * 4 >= self.misses
    }

    fn get(&mut self, key: &[u8]) -> Option<BslValue> {
        if !self.active() {
            return None;
        }
        match self.map.get(key) {
            Some(v) => {
                self.hits += 1;
                Some(v.clone())
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    fn insert(&mut self, key: &'a [u8], value: BslValue) {
        if self.active() {
            self.map.insert(key, value);
        }
    }
}

/// Сообщение строгой формы `{"B",1}`/`{"N",…}`/`{"D",…}`/`{"T",…}`.
fn one_arg_err(tag: &str) -> RtError {
    err(format!("у значения вида «{tag}» ровно один аргумент"))
}

/// Потоковое чтение поверх [`Parser`]: горячие формы — скаляры, массивы,
/// структуры, соответствия и разметка `ТаблицыЗначений` — превращаются в
/// значения прямо с курсора, без промежуточного дерева: на выгрузке в
/// сотни тысяч строк дерево [`Node`] стоило дороже самого чтения. Дерево
/// осталось холодным случаям, где нужна каноническая перерисовка либо
/// терпимость к незнакомой форме — непрозрачным значениям,
/// `ОписаниеТипов` и описаниям типов колонок: такой случай читается
/// откатом курсора к началу поддерева и старым [`convert`], поэтому его
/// семантика не может разойтись с прежней. Тексты ошибок на заведомо
/// битых строках таблиц могут отличаться от древесного разбора — сами
/// ошибки остаются ошибками.
///
/// Повторяющиеся строковые, числовые и датовые значения интернируются по
/// сырому срезу лексемы (раздельными таблицами — одинаковый текст в
/// разных видах значения даёт разные значения): в реальных выгрузках
/// значения массово повторяются, и кэш заменяет миллионы аллокаций и
/// перекодировок UTF-8 → UTF-16 тысячами. Возврат из кэша — клон `Rc`,
/// поэтому повторы ещё и разделяют память, а сравнение таких строк
/// попадает в быстрый путь `Rc::ptr_eq`.
struct Reader<'a, 'r> {
    p: Parser<'a>,
    rt: &'r mut RuntimeShapes,
    strings: ValueCache<'a>,
    numbers: ValueCache<'a>,
    dates: ValueCache<'a>,
    /// Целые холодные поддеревья по их сырому срезу: ссылки объектов базы
    /// в ячейках реальных выгрузок повторяются так же массово, как строки,
    /// и на повторе достаточно перескочить поддерево байтовым сканом.
    opaques: ValueCache<'a>,
}

impl<'a> Reader<'a, '_> {
    fn read_value(&mut self, depth: usize) -> RtResult<BslValue> {
        if depth > MAX_VSTR_DEPTH {
            return Err(RtError::StackOverflow {
                what: "слишком глубокая вложенность в ЗначениеИзСтрокиВнутр",
            });
        }
        self.p.skip_ws();
        let start = self.p.pos;
        if self.p.peek() != Some(b'{') {
            return Err(err("значение во внутреннем формате начинается со скобки"));
        }
        self.p.pos += 1;
        self.p.skip_ws();
        match self.p.peek() {
            Some(b'}') => return Err(err("пустой список во внутреннем формате")),
            Some(b'"') => {}
            _ => return Err(err("первый элемент списка — строковый тег вида значения")),
        }
        let tag = self.p.read_quoted_raw()?;
        match tag {
            b"U" => {
                self.skip_list_tail(depth)?;
                Ok(BslValue::Undefined)
            }
            b"L" => {
                self.skip_list_tail(depth)?;
                Ok(BslValue::Null)
            }
            b"B" => match self.one_bare_tok("B")? {
                b"0" => Ok(BslValue::Boolean(false)),
                b"1" => Ok(BslValue::Boolean(true)),
                other => Err(err(format!(
                    "«{}» не булево во внутреннем формате",
                    String::from_utf8_lossy(other)
                ))),
            },
            b"N" => {
                let raw = self.one_bare_tok("N")?;
                if let Some(v) = self.numbers.get(raw) {
                    return Ok(v);
                }
                let text = utf8(raw)?;
                let n = BslNumber::parse_canonical(text)
                    .map_err(|_| err(format!("«{text}» не число во внутреннем формате")))?;
                let v = BslValue::Number(n);
                self.numbers.insert(raw, v.clone());
                Ok(v)
            }
            b"S" => {
                let shape = "у строкового значения ровно один строковый аргумент";
                self.expect_comma(shape)?;
                self.p.skip_ws();
                if self.p.peek() != Some(b'"') {
                    return Err(err(shape));
                }
                let raw = self.p.read_quoted_raw()?;
                self.expect_close(shape)?;
                if let Some(v) = self.strings.get(raw) {
                    return Ok(v);
                }
                let text = utf8(raw)?;
                let v = match unquote(text, self.p.crlf_to_lf) {
                    Some(s) => BslValue::Str(BslString::from_str(&s)),
                    None => BslValue::Str(BslString::from_str(text)),
                };
                self.strings.insert(raw, v.clone());
                Ok(v)
            }
            b"D" => {
                let raw = self.one_bare_tok("D")?;
                if let Some(v) = self.dates.get(raw) {
                    return Ok(v);
                }
                let v = parse_date(utf8(raw)?)?;
                self.dates.insert(raw, v.clone());
                Ok(v)
            }
            b"T" => {
                let uuid = self.one_bare_tok("T")?;
                let uuid = utf8(uuid)?;
                match TYPE_UUIDS
                    .iter()
                    .find(|(_, known)| known.eq_ignore_ascii_case(uuid))
                {
                    Some((t, _)) => Ok(BslValue::Type(*t)),
                    None => self.read_cold_interned(start, depth),
                }
            }
            b"#" => self.read_object(start, depth),
            other => {
                let shown = String::from_utf8_lossy(other);
                let shown =
                    unquote(&shown, self.p.crlf_to_lf).unwrap_or_else(|| shown.into_owned());
                Err(err(format!(
                    "вид значения «{shown}» во внутреннем формате не поддержан"
                )))
            }
        }
    }

    /// Холодный путь: курсор откатывается к началу поддерева, оно
    /// разбирается деревом и уходит в [`convert`] — непрозрачные значения
    /// и `ОписаниеТипов` получают ровно ту же канонизацию, что и раньше.
    /// Результат интернируется по сырому срезу поддерева: ссылки объектов
    /// базы в ячейках повторяются массово, и на повторе поддерево
    /// перескакивается байтовым сканом без разбора. Кэшировать безопасно
    /// всё, что отсюда выходит, — непрозрачные значения и `ОписаниеТипов`
    /// неизменяемы.
    fn read_cold_interned(&mut self, start: usize, depth: usize) -> RtResult<BslValue> {
        if let Some(end) = self.p.subtree_end(start) {
            let raw = &self.p.text[start..end];
            if let Some(v) = self.opaques.get(raw) {
                self.p.pos = end;
                return Ok(v);
            }
            self.p.pos = start;
            let node = self.p.parse_node(depth)?;
            debug_assert_eq!(self.p.pos, end, "скан скобок разошёлся с разбором");
            let v = convert(&node, self.rt, depth)?;
            self.opaques.insert(raw, v.clone());
            return Ok(v);
        }
        // Оборванный текст: настоящий разбор даст точную ошибку.
        self.p.pos = start;
        let node = self.p.parse_node(depth)?;
        convert(&node, self.rt, depth)
    }

    /// Хвост списка после `{"U"`/`{"L"`: платформенные формы пусты, но
    /// древесный разбор принимал и лишние элементы — они молча
    /// выбрасываются, здесь так же.
    fn skip_list_tail(&mut self, depth: usize) -> RtResult<()> {
        loop {
            self.p.skip_ws();
            match self.p.peek() {
                Some(b'}') => {
                    self.p.pos += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.p.pos += 1;
                    self.p.skip_ws();
                    let _ = self.p.parse_node(depth + 1)?;
                }
                _ => return Err(err("ожидалась «,» или «}» во внутреннем формате")),
            }
        }
    }

    /// Один любой узел без материализации — служебные хвосты строк и
    /// блока строк. Голая лексема читается без аллокации.
    fn skip_any_token(&mut self, depth: usize) -> RtResult<()> {
        self.p.skip_ws();
        match self.p.peek() {
            Some(b'"') => self.p.read_quoted_raw().map(|_| ()),
            Some(b'{') => self.p.parse_node(depth + 1).map(|_| ()),
            _ => self.p.read_bare_tok().map(|_| ()),
        }
    }

    fn expect_comma(&mut self, shape_err: &'static str) -> RtResult<()> {
        self.p.skip_ws();
        if self.p.peek() == Some(b',') {
            self.p.pos += 1;
            Ok(())
        } else {
            Err(err(shape_err))
        }
    }

    fn expect_close(&mut self, shape_err: &'static str) -> RtResult<()> {
        self.p.skip_ws();
        if self.p.peek() == Some(b'}') {
            self.p.pos += 1;
            Ok(())
        } else {
            Err(err(shape_err))
        }
    }

    /// Строгая форма `{"<тег>",<лексема>}` — единственный голый аргумент.
    fn one_bare_tok(&mut self, tag: &'static str) -> RtResult<&'a [u8]> {
        self.p.skip_ws();
        if self.p.peek() != Some(b',') {
            return Err(one_arg_err(tag));
        }
        self.p.pos += 1;
        self.p.skip_ws();
        if !self.p.at_bare() {
            return Err(one_arg_err(tag));
        }
        let tok = self.p.read_bare_tok()?;
        self.p.skip_ws();
        if self.p.peek() != Some(b'}') {
            return Err(one_arg_err(tag));
        }
        self.p.pos += 1;
        Ok(tok)
    }

    /// `{"#",<uuid вида>,<нагрузка>}`.
    fn read_object(&mut self, start: usize, depth: usize) -> RtResult<BslValue> {
        let shape = "у объекта во внутреннем формате вид и полезная нагрузка";
        self.expect_comma(shape)?;
        self.p.skip_ws();
        if !self.p.at_bare() {
            return Err(err(shape));
        }
        let kind = self.p.read_bare_tok()?;
        // Незнакомый вид — непрозрачное значение с любой формой нагрузки;
        // `ОписаниеТипов` тоже уходит в дерево: его нагрузка либо
        // материализуется, либо канонически перерисовывается целиком.
        let known = matches!(
            utf8(kind).unwrap_or(""),
            ARRAY_ID | FIXED_ARRAY_ID | STRUCTURE_ID | MAP_ID | VALUE_TABLE_ID
        );
        if !known {
            return self.read_cold_interned(start, depth);
        }
        let kind = utf8(kind)?;
        self.expect_comma(shape)?;
        self.p.skip_ws();
        if self.p.peek() != Some(b'{') {
            return Err(err("полезная нагрузка объекта — список"));
        }
        self.p.pos += 1;
        let value = match kind {
            ARRAY_ID | FIXED_ARRAY_ID => self.read_array(depth)?,
            STRUCTURE_ID => self.read_structure(depth)?,
            MAP_ID => self.read_map(depth)?,
            VALUE_TABLE_ID => self.read_table(depth)?,
            _ => unreachable!("незнакомый вид ушёл в дерево выше"),
        };
        // Ровно три элемента: тег, вид, нагрузка.
        self.expect_close(shape)?;
        Ok(value)
    }

    /// Счёт элементов коллекции перед содержимым: `{<число>, …}`.
    fn read_counted_prefix(&mut self) -> RtResult<usize> {
        self.p.skip_ws();
        if !self.p.at_bare() {
            return Err(err("коллекция начинается с числа элементов"));
        }
        utf8(self.p.read_bare_tok()?)?
            .parse()
            .map_err(|_| err("число элементов коллекции — целое"))
    }

    /// Разделитель перед очередным элементом коллекции: `,` — элемент
    /// есть, `}` — коллекция закончилась.
    fn next_item(&mut self) -> RtResult<bool> {
        self.p.skip_ws();
        match self.p.peek() {
            Some(b'}') => {
                self.p.pos += 1;
                Ok(false)
            }
            Some(b',') => {
                self.p.pos += 1;
                Ok(true)
            }
            _ => Err(err("ожидалась «,» или «}» во внутреннем формате")),
        }
    }

    fn check_count(declared: usize, got: usize) -> RtResult<()> {
        if declared != got {
            return Err(err(format!(
                "коллекция объявляет {declared} элементов, а несёт {got}"
            )));
        }
        Ok(())
    }

    fn read_array(&mut self, depth: usize) -> RtResult<BslValue> {
        let declared = self.read_counted_prefix()?;
        let items = BslValue::new_array(Vec::new());
        let mut got = 0;
        while self.next_item()? {
            items.push_element(self.read_value(depth + 1)?)?;
            got += 1;
        }
        Self::check_count(declared, got)?;
        Ok(items)
    }

    fn read_structure(&mut self, depth: usize) -> RtResult<BslValue> {
        let pair_err = "элемент структуры — пара «имя, значение»";
        let declared = self.read_counted_prefix()?;
        let object = {
            let empty = self.rt.shapes.empty();
            BslValue::new_structure(empty, Vec::new())
        };
        let mut got = 0;
        while self.next_item()? {
            self.p.skip_ws();
            if self.p.peek() != Some(b'{') {
                return Err(err(pair_err));
            }
            self.p.pos += 1;
            let name = match self.read_value(depth + 1)? {
                BslValue::Str(s) => s.to_string(),
                _ => return Err(err("имя поля структуры — строка")),
            };
            self.expect_comma(pair_err)?;
            let id = self.rt.names.intern(&name);
            let value = self.read_value(depth + 1)?;
            self.expect_close(pair_err)?;
            object.structure_insert(id, value, &mut self.rt.shapes)?;
            got += 1;
        }
        Self::check_count(declared, got)?;
        Ok(object)
    }

    fn read_map(&mut self, depth: usize) -> RtResult<BslValue> {
        let pair_err = "элемент соответствия — пара «ключ, значение»";
        let declared = self.read_counted_prefix()?;
        let map = BslValue::new_map();
        let mut got = 0;
        while self.next_item()? {
            self.p.skip_ws();
            if self.p.peek() != Some(b'{') {
                return Err(err(pair_err));
            }
            self.p.pos += 1;
            let key = self.read_value(depth + 1)?;
            self.expect_comma(pair_err)?;
            let value = self.read_value(depth + 1)?;
            self.expect_close(pair_err)?;
            map.map_insert(key, value)?;
            got += 1;
        }
        Self::check_count(declared, got)?;
        Ok(map)
    }

    /// Разметка `ТаблицыЗначений` с курсора — структура описана над
    /// [`table_to_writer`]. Открывающая скобка нагрузки уже прочитана.
    fn read_table(&mut self, depth: usize) -> RtResult<BslValue> {
        let shape = "разметка ТаблицыЗначений — версия, колонки, строки, индексы";
        self.p.skip_ws();
        if !self.p.at_bare() {
            return Err(err(shape));
        }
        let version = self.p.read_bare_tok()?;
        if version != b"9" {
            return Err(err(format!(
                "версия разметки ТаблицыЗначений «{}» не поддержана (измерена 9)",
                String::from_utf8_lossy(version)
            )));
        }
        self.expect_comma(shape)?;
        self.p.skip_ws();
        if self.p.peek() != Some(b'{') {
            return Err(err(shape));
        }
        self.p.pos += 1;

        let table = crate::ValueTableData::new();
        {
            let mut t = table.borrow_mut();

            // Колонки: {<число>, {<колонка>}, …}.
            let declared_cols = self.read_counted_prefix()?;
            let col_shape = "колонка ТаблицыЗначений — идентификатор, имя, типы, заголовок, ширина";
            let mut got_cols = 0;
            while self.next_item()? {
                self.p.skip_ws();
                if self.p.peek() != Some(b'{') {
                    return Err(err("колонка ТаблицыЗначений — список"));
                }
                self.p.pos += 1;
                self.p.skip_ws();
                if !self.p.at_bare() {
                    return Err(err(col_shape));
                }
                let col_id = utf8(self.p.read_bare_tok()?)?;
                self.expect_comma(col_shape)?;
                self.p.skip_ws();
                if self.p.peek() != Some(b'"') {
                    return Err(err(col_shape));
                }
                let name_raw = utf8(self.p.read_quoted_raw()?)?;
                let name = match unquote(name_raw, self.p.crlf_to_lf) {
                    Some(s) => s,
                    None => name_raw.to_string(),
                };
                self.expect_comma(col_shape)?;
                self.p.skip_ws();
                if self.p.peek() != Some(b'{') {
                    return Err(err(col_shape));
                }
                // Описание типов — деревом: холодное место с канонической
                // перерисовкой, общей с древесным разбором.
                let pattern_node = self.p.parse_node(depth + 1)?;
                let Node::List(pattern) = &pattern_node else {
                    return Err(err(col_shape));
                };
                let (types, raw_pattern) = column_pattern(pattern)?;
                self.expect_comma(col_shape)?;
                self.p.skip_ws();
                if self.p.peek() != Some(b'"') {
                    return Err(err(col_shape));
                }
                let title_raw = utf8(self.p.read_quoted_raw()?)?;
                let title = match unquote(title_raw, self.p.crlf_to_lf) {
                    Some(s) => s,
                    None => title_raw.to_string(),
                };
                self.expect_comma(col_shape)?;
                self.p.skip_ws();
                if !self.p.at_bare() {
                    return Err(err(col_shape));
                }
                let width = utf8(self.p.read_bare_tok()?)?;
                self.expect_close(col_shape)?;

                t.add_column(&name);
                let slot = t.column_types.len() - 1;
                t.column_types[slot] = types;
                t.column_vstr[slot] = crate::table::ColumnVstr {
                    id: Some(col_id.to_string()),
                    raw_pattern: Some(raw_pattern),
                    title,
                    width: width.to_string(),
                };
                got_cols += 1;
            }
            Self::check_count(declared_cols, got_cols)?;
            let ncols = t.column_names.len();

            // Блок строк: служебные пары до первого вложенного списка
            // игнорируются, как и в древесном разборе.
            self.expect_comma(shape)?;
            self.p.skip_ws();
            if self.p.peek() != Some(b'{') {
                return Err(err(shape));
            }
            self.p.pos += 1;
            let mut first = true;
            loop {
                if first {
                    self.p.skip_ws();
                    if self.p.peek() == Some(b'}') {
                        return Err(err("в блоке строк ТаблицыЗначений нет списка строк"));
                    }
                } else if !self.next_item()? {
                    return Err(err("в блоке строк ТаблицыЗначений нет списка строк"));
                } else {
                    self.p.skip_ws();
                }
                first = false;
                match self.p.peek() {
                    Some(b'{') => break,
                    Some(b'"') => {
                        let _ = self.p.read_quoted_raw()?;
                    }
                    _ => {
                        let _ = self.p.read_bare_tok()?;
                    }
                }
            }

            // Внутренний список: {1, <число строк>, <строка>, …}.
            let rows_shape = "список строк ТаблицыЗначений начинается с числа строк";
            self.p.pos += 1;
            self.p.skip_ws();
            if !self.p.at_bare() {
                return Err(err(rows_shape));
            }
            let _ = self.p.read_bare_tok()?;
            self.expect_comma(rows_shape)?;
            self.p.skip_ws();
            if !self.p.at_bare() {
                return Err(err(rows_shape));
            }
            let declared_rows = String::from_utf8_lossy(self.p.read_bare_tok()?).into_owned();
            let expected_rows: Option<usize> = declared_rows.parse().ok();

            let mut file_row_ids = Vec::with_capacity(expected_rows.unwrap_or(0));
            while self.next_item()? {
                self.p.skip_ws();
                if self.p.peek() != Some(b'{') {
                    return Err(err("строка ТаблицыЗначений — список"));
                }
                self.p.pos += 1;
                // {2, <идентификатор строки>, <n значений>, <значения>, 0}
                let short = "строка ТаблицыЗначений короче служебной обвязки";
                self.p.skip_ws();
                if matches!(self.p.peek(), Some(b'}' | b',') | None) {
                    return Err(err(short));
                }
                self.skip_any_token(depth)?;
                self.expect_comma(short)?;
                self.p.skip_ws();
                if !self.p.at_bare() {
                    return Err(err(
                        "второй элемент строки ТаблицыЗначений — её идентификатор",
                    ));
                }
                let row_id: u64 = utf8(self.p.read_bare_tok()?)?
                    .parse()
                    .map_err(|_| err("идентификатор строки ТаблицыЗначений — целое"))?;
                self.expect_comma(short)?;
                self.p.skip_ws();
                if !self.p.at_bare() {
                    return Err(err(
                        "третий элемент строки ТаблицыЗначений — число значений",
                    ));
                }
                let stored: usize = utf8(self.p.read_bare_tok()?)?
                    .parse()
                    .map_err(|_| err("число значений строки ТаблицыЗначений — целое"))?;
                if stored > ncols {
                    return Err(err(format!(
                        "строка ТаблицыЗначений объявляет {stored} значений, а несёт {stored}"
                    )));
                }
                file_row_ids.push(row_id);
                let _ = t.add_row();
                let pos = t.row_ids.len() - 1;
                for k in 0..stored {
                    if self.expect_comma(short).is_err() {
                        return Err(err(format!(
                            "строка ТаблицыЗначений объявляет {stored} значений, а несёт {k}"
                        )));
                    }
                    t.columns[k][pos] = self.read_value(depth + 1)?;
                }
                // Хвостовая служебная лексема — ровно одна, любая.
                let mut extras = 0usize;
                loop {
                    self.p.skip_ws();
                    match self.p.peek() {
                        Some(b'}') => {
                            self.p.pos += 1;
                            break;
                        }
                        Some(b',') => {
                            self.p.pos += 1;
                            self.skip_any_token(depth)?;
                            extras += 1;
                        }
                        _ => return Err(err("ожидалась «,» или «}» во внутреннем формате")),
                    }
                }
                if extras != 1 {
                    if stored == 0 && extras == 0 {
                        return Err(err(short));
                    }
                    return Err(err(format!(
                        "строка ТаблицыЗначений объявляет {stored} значений, а несёт {}",
                        stored + extras - 1
                    )));
                }
            }
            if expected_rows != Some(file_row_ids.len()) {
                return Err(err(format!(
                    "ТаблицаЗначений объявляет {declared_rows} строк, а несёт {}",
                    file_row_ids.len()
                )));
            }

            // Хвост блока строк: первым может идти сырое `X`; остальное,
            // включая `Y`, игнорируется — `Y` вычисляется при записи.
            let mut first_after = true;
            while self.next_item()? {
                self.p.skip_ws();
                if first_after && self.p.at_bare() {
                    t.vstr_tail_x = Some(utf8(self.p.read_bare_tok()?)?.to_string());
                } else {
                    self.skip_any_token(depth)?;
                }
                first_after = false;
            }
            t.set_row_ids(file_row_ids);
        }

        // Индексы — один любой узел, затем конец нагрузки: ровно четыре
        // элемента разметки.
        self.expect_comma(shape)?;
        self.skip_any_token(depth)?;
        self.expect_close(shape)?;
        Ok(BslValue::Object(Rc::new(BslObject::ValueTable(table))))
    }
}

fn convert(node: &Node, rt: &mut RuntimeShapes, depth: usize) -> RtResult<BslValue> {
    if depth > MAX_VSTR_DEPTH {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность в ЗначениеИзСтрокиВнутр",
        });
    }
    let Node::List(items) = node else {
        return Err(err("значение во внутреннем формате начинается со скобки"));
    };
    let tag = items
        .first()
        .ok_or_else(|| err("пустой список во внутреннем формате"))?;
    let Node::Str(tag) = tag else {
        return Err(err("первый элемент списка — строковый тег вида значения"));
    };
    match tag.as_str() {
        "U" => Ok(BslValue::Undefined),
        "L" => Ok(BslValue::Null),
        "B" => match one_bare(items, "B")? {
            "0" => Ok(BslValue::Boolean(false)),
            "1" => Ok(BslValue::Boolean(true)),
            other => Err(err(format!("«{other}» не булево во внутреннем формате"))),
        },
        "N" => {
            let text = one_bare(items, "N")?;
            let n = BslNumber::parse_canonical(text)
                .map_err(|_| err(format!("«{text}» не число во внутреннем формате")))?;
            Ok(BslValue::Number(n))
        }
        "S" => match items.as_slice() {
            [_, Node::Str(s)] => Ok(BslValue::Str(BslString::from_str(s))),
            _ => Err(err("у строкового значения ровно один строковый аргумент")),
        },
        "D" => {
            let digits = one_bare(items, "D")?;
            parse_date(digits)
        }
        "T" => {
            let uuid = one_bare(items, "T")?;
            match TYPE_UUIDS
                .iter()
                .find(|(_, known)| known.eq_ignore_ascii_case(uuid))
            {
                Some((t, _)) => Ok(BslValue::Type(*t)),
                // Неизвестный UUID — тип из конфигурации базы (например,
                // `СправочникСсылка.Имя`): материализовать нечем, но
                // транзит обязан его сохранить.
                None => Ok(opaque(node)),
            }
        }
        "#" => convert_object(node, items, rt, depth),
        other => Err(err(format!(
            "вид значения «{other}» во внутреннем формате не поддержан"
        ))),
    }
}

/// Единственный аргумент-лексема тега (`{"B",1}`, `{"N",0.1}`, `{"D",…}`).
fn one_bare<'n>(items: &'n [Node], tag: &str) -> RtResult<&'n str> {
    match items {
        [_, Node::Bare(b)] => Ok(b),
        _ => Err(err(format!("у значения вида «{tag}» ровно один аргумент"))),
    }
}

fn parse_date(digits: &str) -> RtResult<BslValue> {
    if digits.len() != 14 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err("дата во внутреннем формате — ровно 14 цифр"));
    }
    let part = |from: usize, to: usize| digits[from..to].parse::<i64>().unwrap_or(0);
    // Пустая дата платформы — `00010101000000`, это `0001-01-01`.
    BslDate::from_civil(
        part(0, 4),
        part(4, 6) as u32,
        part(6, 8) as u32,
        part(8, 10) as u32,
        part(10, 12) as u32,
        part(12, 14) as u32,
    )
    .map(BslValue::Date)
    .ok_or_else(|| err(format!("«{digits}» не дата во внутреннем формате")))
}

/// Печать разобранного поддерева тем же правилом переводов строк, что у
/// потоковой записи (см. [`Writer`]). Нужна только разбору: непрозрачные
/// значения и сырые описания колонок хранятся канонически отрисованным
/// текстом, который обратная сериализация вернёт байт в байт.
fn render(node: &Node, out: &mut String) {
    match node {
        Node::Bare(b) => out.push_str(b),
        Node::Str(s) => write_quoted(out, s),
        Node::List(items) => {
            out.push('{');
            let mut last_was_list = false;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                last_was_list = matches!(item, Node::List(_));
                if last_was_list {
                    out.push('\n');
                }
                render(item, out);
            }
            if last_was_list {
                out.push('\n');
            }
            out.push('}');
        }
    }
}

/// Непрозрачное значение: канонически отрисованное поддерево, которое
/// обратная сериализация вернёт байт в байт.
fn opaque(node: &Node) -> BslValue {
    let mut text = String::new();
    render(node, &mut text);
    BslValue::Object(Rc::new(BslObject::VstrOpaque(text)))
}

fn convert_object(
    node: &Node,
    items: &[Node],
    rt: &mut RuntimeShapes,
    depth: usize,
) -> RtResult<BslValue> {
    let Some(Node::Bare(kind)) = items.get(1) else {
        return Err(err(
            "у объекта во внутреннем формате вид и полезная нагрузка",
        ));
    };
    // Виды, которые эта реализация не материализует, — ссылки объектов
    // базы, СписокЗначений, УникальныйИдентификатор и любой незнакомый
    // UUID — сохраняются непрозрачно, С ЛЮБОЙ формой полезной нагрузки:
    // у УникальногоИдентификатора она, например, голая лексема, а не
    // список. Материализуемые виды дальше требуют список строгой формы.
    // НЕ ИЗМЕРЕНО(VSTR.FORMAT) — разметка СпискаЗначений и ссылок объектов
    // базы не реверсирована (см. вопрос в реестре); непрозрачное хранение
    // гарантирует только транзит байт в байт, не материализацию.
    let known_kind = matches!(
        kind.as_str(),
        ARRAY_ID | FIXED_ARRAY_ID | STRUCTURE_ID | MAP_ID | VALUE_TABLE_ID | TYPE_DESCRIPTION_ID
    );
    if !known_kind {
        return Ok(opaque(node));
    }
    let [_, _, payload] = items else {
        return Err(err(
            "у объекта во внутреннем формате вид и полезная нагрузка",
        ));
    };
    let Node::List(payload) = payload else {
        return Err(err("полезная нагрузка объекта — список"));
    };
    match kind.as_str() {
        // `ФиксированныйМассив` читается ОБЫЧНЫМ массивом: своего
        // неизменяемого вида здесь нет, а терять данные хуже, чем терять
        // неизменяемость.
        ARRAY_ID | FIXED_ARRAY_ID => {
            let elems = counted_elements(payload)?;
            let items = BslValue::new_array(Vec::new());
            for node in elems {
                items.push_element(convert(node, rt, depth + 1)?)?;
            }
            Ok(items)
        }
        STRUCTURE_ID => {
            let elems = counted_elements(payload)?;
            let object = {
                let empty = rt.shapes.empty();
                BslValue::new_structure(empty, Vec::new())
            };
            for node in elems {
                let Node::List(pair) = node else {
                    return Err(err("элемент структуры — пара «имя, значение»"));
                };
                let [name, value] = pair.as_slice() else {
                    return Err(err("элемент структуры — пара «имя, значение»"));
                };
                let name = match convert(name, rt, depth + 1)? {
                    BslValue::Str(s) => s.to_string(),
                    _ => return Err(err("имя поля структуры — строка")),
                };
                let id = rt.names.intern(&name);
                let value = convert(value, rt, depth + 1)?;
                object.structure_insert(id, value, &mut rt.shapes)?;
            }
            Ok(object)
        }
        MAP_ID => {
            let elems = counted_elements(payload)?;
            let map = BslValue::new_map();
            for node in elems {
                let Node::List(pair) = node else {
                    return Err(err("элемент соответствия — пара «ключ, значение»"));
                };
                let [key, value] = pair.as_slice() else {
                    return Err(err("элемент соответствия — пара «ключ, значение»"));
                };
                let key = convert(key, rt, depth + 1)?;
                let value = convert(value, rt, depth + 1)?;
                map.map_insert(key, value)?;
            }
            Ok(map)
        }
        VALUE_TABLE_ID => table_from_payload(payload, rt, depth),
        TYPE_DESCRIPTION_ID => {
            // Полезная нагрузка — сам `{"Pattern",…}`. Материализуется
            // только чисто-буквенное описание; квалификаторы и ссылочные
            // типы внутри уходят в непрозрачное значение — терять их при
            // транзите нельзя, а модель `ОписаниеТипов` их не хранит.
            let [Node::Str(tag), type_nodes @ ..] = payload.as_slice() else {
                return Ok(opaque(node));
            };
            if tag != "Pattern" {
                return Err(err(format!(
                    "описание типов начинается с «Pattern», не «{tag}»"
                )));
            }
            let mut ids = Vec::with_capacity(type_nodes.len());
            for type_node in type_nodes {
                let Node::List(t) = type_node else {
                    return Ok(opaque(node));
                };
                let [Node::Str(letter)] = t.as_slice() else {
                    return Ok(opaque(node));
                };
                let Some((id, _)) = COLUMN_TYPE_LETTERS
                    .iter()
                    .find(|(_, known)| known == letter)
                else {
                    return Ok(opaque(node));
                };
                ids.push(*id);
            }
            Ok(BslValue::Object(Rc::new(BslObject::TypeDescription(ids))))
        }
        // Недостижимо: незнакомые виды ушли в непрозрачное значение выше.
        other => Err(err(format!(
            "вид объекта {other} во внутреннем формате не поддержан"
        ))),
    }
}

/// Разбор `ТаблицыЗначений` из дерева — разметка описана над
/// [`table_to_writer`]; потоковый двойник — [`Reader::read_table`].
/// Служебные индексы (номер колонки, номер строки, пары в блоке строк)
/// принимаются, но не проверяются: физический порядок узлов и есть
/// порядок таблицы. Заголовок, ширина и квалификаторы типов колонок
/// в этой реализации не хранятся и отбрасываются при чтении.
fn table_from_payload(
    payload: &[Node],
    rt: &mut RuntimeShapes,
    depth: usize,
) -> RtResult<BslValue> {
    let [Node::Bare(version), Node::List(cols), Node::List(rows_block), _indexes] = payload else {
        return Err(err(
            "разметка ТаблицыЗначений — версия, колонки, строки, индексы",
        ));
    };
    if version != "9" {
        return Err(err(format!(
            "версия разметки ТаблицыЗначений «{version}» не поддержана (измерена 9)"
        )));
    }

    let table = crate::ValueTableData::new();
    {
        let mut t = table.borrow_mut();

        for col in counted_elements(cols)? {
            let Node::List(col) = col else {
                return Err(err("колонка ТаблицыЗначений — список"));
            };
            let [Node::Bare(col_id), Node::Str(name), Node::List(pattern), Node::Str(title), Node::Bare(width)] =
                col.as_slice()
            else {
                return Err(err(
                    "колонка ТаблицыЗначений — идентификатор, имя, типы, заголовок, ширина",
                ));
            };
            let (types, raw) = column_pattern(pattern)?;
            t.add_column(name);
            let slot = t.column_types.len() - 1;
            t.column_types[slot] = types;
            t.column_vstr[slot] = crate::table::ColumnVstr {
                id: Some(col_id.clone()),
                raw_pattern: Some(raw),
                title: title.clone(),
                width: width.clone(),
            };
        }
        let ncols = t.column_names.len();

        // Внутри блока строк ровно один вложенный список — {1, n, строки};
        // сразу за ним идут две служебные лексемы `X,Y`. `Y` на всех
        // измеренных данных равен максимальному идентификатору строки и
        // вычисляется заново; `X` не реверсирован и хранится сырым.
        let inner_at = rows_block
            .iter()
            .position(|n| matches!(n, Node::List(_)))
            .ok_or_else(|| err("в блоке строк ТаблицыЗначений нет списка строк"))?;
        let Node::List(inner) = &rows_block[inner_at] else {
            unreachable!("позиция найдена по этому же условию");
        };
        if let Some(Node::Bare(x)) = rows_block.get(inner_at + 1) {
            t.vstr_tail_x = Some(x.clone());
        }
        let [Node::Bare(_), Node::Bare(declared), rows @ ..] = inner.as_slice() else {
            return Err(err("список строк ТаблицыЗначений начинается с числа строк"));
        };
        if declared.parse::<usize>() != Ok(rows.len()) {
            return Err(err(format!(
                "ТаблицаЗначений объявляет {declared} строк, а несёт {}",
                rows.len()
            )));
        }
        let mut file_row_ids = Vec::with_capacity(rows.len());
        for row in rows {
            let Node::List(row) = row else {
                return Err(err("строка ТаблицыЗначений — список"));
            };
            // {2, <идентификатор строки>, <n значений>, <значения>, 0}
            if row.len() < 4 {
                return Err(err("строка ТаблицыЗначений короче служебной обвязки"));
            }
            let Node::Bare(row_id) = &row[1] else {
                return Err(err(
                    "второй элемент строки ТаблицыЗначений — её идентификатор",
                ));
            };
            let row_id: u64 = row_id
                .parse()
                .map_err(|_| err("идентификатор строки ТаблицыЗначений — целое"))?;
            file_row_ids.push(row_id);
            let Node::Bare(stored) = &row[2] else {
                return Err(err(
                    "третий элемент строки ТаблицыЗначений — число значений",
                ));
            };
            let stored: usize = stored
                .parse()
                .map_err(|_| err("число значений строки ТаблицыЗначений — целое"))?;
            let values = &row[3..row.len() - 1];
            if values.len() != stored || stored > ncols {
                return Err(err(format!(
                    "строка ТаблицыЗначений объявляет {stored} значений, а несёт {}",
                    values.len()
                )));
            }
            let _ = t.add_row();
            let pos = t.row_ids.len() - 1;
            for (k, node) in values.iter().enumerate() {
                t.columns[k][pos] = convert(node, rt, depth + 1)?;
            }
            // Хвост до `ncols` остаётся `Неопределено` — так платформа
            // кодирует пропуски (измерено: хвостовые не пишутся).
        }
        t.set_row_ids(file_row_ids);
    }
    Ok(BslValue::Object(Rc::new(BslObject::ValueTable(table))))
}

/// Описание типов колонки `{"Pattern",…}` из дерева: типы «насколько
/// возможно» и каноническая перерисовка исходника для транзита. Общее
/// место древесного и потокового чтения.
///
/// Буквы собираются в типы колонки не строже, чем можно: после буквы
/// могут идти квалификаторы (`{"N",10,2,0}`) — они сохраняются при
/// колонке, — а ссылочный компонент (`{"#",<uuid>}`) или незнакомая
/// буква обнуляют ограничение целиком: ложно сузить тип хуже, чем не
/// ограничить. Точный исходник в любом случае возвращается второй
/// компонентой — транзит от обнуления не страдает.
fn column_pattern(pattern: &[Node]) -> RtResult<(Option<Vec<crate::table::ColumnType>>, String)> {
    let [Node::Str(tag), type_nodes @ ..] = pattern else {
        return Err(err("описание типов колонки начинается с «Pattern»"));
    };
    if tag != "Pattern" {
        return Err(err(format!(
            "описание типов колонки — «Pattern», не «{tag}»"
        )));
    }
    let mut ids = Vec::with_capacity(type_nodes.len());
    let mut representable = true;
    for node in type_nodes {
        let parts = match node {
            Node::List(t) => Some(t),
            _ => None,
        };
        let letter = parts.and_then(|t| match t.first() {
            Some(Node::Str(letter)) => Some(letter),
            _ => None,
        });
        match letter.and_then(|letter| {
            COLUMN_TYPE_LETTERS
                .iter()
                .find(|(_, known)| known == letter)
                .map(|(id, _)| *id)
        }) {
            Some(id) => {
                let quals = parts
                    .map(|t| {
                        t[1..]
                            .iter()
                            .filter_map(|q| match q {
                                Node::Bare(b) => Some(b.clone()),
                                Node::Str(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ids.push(crate::table::ColumnType { id, quals });
            }
            None => {
                representable = false;
                break;
            }
        }
    }
    let types = if type_nodes.is_empty() || !representable {
        None
    } else {
        Some(ids)
    };
    let mut raw = String::new();
    render(&Node::List(pattern.to_vec()), &mut raw);
    Ok((types, raw))
}

/// Полезная нагрузка коллекции: `{<число элементов>, <эл>, …}`. Счёт
/// платформа пишет всегда; расхождение счёта с фактическим числом
/// элементов — ошибка формата, а не повод угадывать.
fn counted_elements(payload: &[Node]) -> RtResult<&[Node]> {
    let [Node::Bare(count), elems @ ..] = payload else {
        return Err(err("коллекция начинается с числа элементов"));
    };
    let declared: usize = count
        .parse()
        .map_err(|_| err("число элементов коллекции — целое"))?;
    if declared != elems.len() {
        return Err(err(format!(
            "коллекция объявляет {declared} элементов, а несёт {}",
            elems.len()
        )));
    }
    Ok(elems)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt() -> RuntimeShapes {
        RuntimeShapes::seeded(Vec::new(), Vec::new())
    }

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    fn s(text: &str) -> BslValue {
        BslValue::Str(BslString::from_str(text))
    }

    fn write(v: &BslValue) -> String {
        value_to_string_internal(v, &rt()).expect("сериализация")
    }

    fn read(text: &str) -> BslValue {
        value_from_string_internal(text, &mut rt()).expect("разбор")
    }

    // Эталонные строки ниже сняты с платформы 8.3.27 разведочной батареей
    // (см. обзор модуля) — их нельзя «поправить красивее», они байт в байт.

    #[test]
    fn scalars_match_the_platform_byte_for_byte() {
        assert_eq!(write(&num("0")), r#"{"N",0}"#);
        assert_eq!(write(&num("-42")), r#"{"N",-42}"#);
        assert_eq!(write(&num("0.1")), r#"{"N",0.1}"#);
        assert_eq!(write(&s("а\"б")), "{\"S\",\"а\"\"б\"}");
        assert_eq!(write(&s("а\nб")), "{\"S\",\"а\nб\"}");
        assert_eq!(write(&BslValue::Boolean(true)), r#"{"B",1}"#);
        assert_eq!(write(&BslValue::Boolean(false)), r#"{"B",0}"#);
        assert_eq!(write(&BslValue::Undefined), r#"{"U"}"#);
        assert_eq!(write(&BslValue::Null), r#"{"L"}"#);
        let d = BslDate::from_civil(2024, 5, 6, 7, 8, 9).unwrap();
        assert_eq!(write(&BslValue::Date(d)), r#"{"D",20240506070809}"#);
        let empty = BslDate::from_civil(1, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(write(&BslValue::Date(empty)), r#"{"D",00010101000000}"#);
    }

    #[test]
    fn types_round_trip_and_match_measured_uuids() {
        let t = BslValue::Type(TypeId::String);
        let text = write(&t);
        assert_eq!(text, r#"{"T",9b6abf8b-0173-48e5-b0a0-83b21fcf63c5}"#);
        assert_eq!(read(&text), t);
        // Тип, UUID которого не измерен, — ошибка, а не выдуманный UUID.
        let e = value_to_string_internal(&BslValue::Type(TypeId::JsonReader), &rt()).unwrap_err();
        assert!(matches!(e, RtError::Vstr(_)), "{e:?}");
    }

    #[test]
    fn simple_array_matches_the_platform_byte_for_byte() {
        let arr = BslValue::new_array(vec![num("1"), s("а")]);
        assert_eq!(
            write(&arr),
            "{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,\n{2,\n{\"N\",1},\n{\"S\",\"а\"}\n}\n}"
        );
        let empty = BslValue::new_array(Vec::new());
        assert_eq!(
            write(&empty),
            "{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,\n{0}\n}"
        );
    }

    #[test]
    fn structure_matches_the_platform_byte_for_byte() {
        let mut context = rt();
        let object = BslValue::new_structure(context.shapes.empty(), Vec::new());
        let id_a = context.names.intern("Азбука");
        let id_b = context.names.intern("Б");
        object
            .structure_insert(id_a, num("1"), &mut context.shapes)
            .unwrap();
        object
            .structure_insert(id_b, s("х"), &mut context.shapes)
            .unwrap();
        let text = value_to_string_internal(&object, &context).unwrap();
        assert_eq!(
            text,
            "{\"#\",4238019d-7e49-4fc9-91db-b6b951d5cf8e,\n{2,\n{\n{\"S\",\"Азбука\"},\n{\"N\",1}\n},\n{\n{\"S\",\"Б\"},\n{\"S\",\"х\"}\n}\n}\n}"
        );
    }

    #[test]
    fn repeated_object_unfolds_into_copies_like_the_platform() {
        // ИЗМЕРЕНО (`REF.TWICE`): платформа не сохраняет ссылочную
        // целостность — повтор разворачивается в копию.
        let inner = BslValue::new_array(vec![num("7")]);
        let outer = BslValue::new_array(vec![inner.clone(), inner]);
        let text = write(&outer);
        let inner_text = "{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,\n{1,\n{\"N\",7}\n}\n}";
        assert_eq!(
            text,
            format!(
                "{{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,\n{{2,\n{inner_text},\n{inner_text}\n}}\n}}"
            )
        );
        // И обратно повторы читаются НЕЗАВИСИМЫМИ объектами, как на
        // платформе (`RT.IDENTITY`).
        let back = read(&text);
        let first = back.get_index(&num("0"), &rt().names).unwrap();
        first.push_element(num("99")).unwrap();
        let second = back.get_index(&num("1"), &rt().names).unwrap();
        assert_eq!(second.collection_len().unwrap(), 1);
    }

    #[test]
    fn round_trip_preserves_types_and_content() {
        // Соответствие здесь ОДНОКЛЮЧЕВОЕ: при нескольких ключах повторная
        // сериализация не идемпотентна и у платформы — пары пишутся в
        // обратном порядке вставки, а чтение вставляет их в порядке текста.
        let mut context = rt();
        let map = BslValue::new_map();
        map.map_insert(s("с"), num("1")).unwrap();
        let arr = BslValue::new_array(vec![
            num("0.1"),
            s("а\"б\nв"),
            BslValue::Date(BslDate::from_civil(2024, 5, 6, 7, 8, 9).unwrap()),
            BslValue::Null,
            BslValue::Type(TypeId::Date),
            map,
        ]);
        let text = value_to_string_internal(&arr, &context).unwrap();
        let back = value_from_string_internal(&text, &mut context).unwrap();
        let again = value_to_string_internal(&back, &context).unwrap();
        assert_eq!(text, again, "повторная сериализация обязана совпасть");
    }

    #[test]
    fn map_pairs_serialize_in_insertion_order_for_transit_stability() {
        // Прямой порядок вставки: строка платформы после чтения и
        // повторной записи возвращается байт в байт (порядок пар в тексте
        // = порядок вставки при чтении = порядок записи).
        let map = BslValue::new_map();
        map.map_insert(s("первый"), num("1")).unwrap();
        map.map_insert(s("второй"), num("2")).unwrap();
        let text = write(&map);
        let a = text.find("первый").expect("есть первый");
        let b = text.find("второй").expect("есть второй");
        assert!(a < b, "пары обязаны идти в порядке вставки: {text}");
    }

    #[test]
    fn platform_map_text_survives_the_transit_byte_for_byte() {
        // РЕАЛЬНАЯ строка платформы (проба `MAP.MIXED` разведочной
        // батареи): её хеш-порядок пар обязан пережить транзит
        // чтение→запись без изменений.
        let text = "{\"#\",3d48feae-a9c6-4c5a-a099-9eb6477630c6,\n{3,\n{\n{\"B\",1},\n{\"U\"}\n},\n{\n{\"N\",2},\n{\"S\",\"д\"}\n},\n{\n{\"S\",\"с\"},\n{\"N\",1}\n}\n}\n}";
        let mut context = rt();
        let back = value_from_string_internal(text, &mut context).unwrap();
        let again = value_to_string_internal(&back, &context).unwrap();
        assert_eq!(text, again);
    }

    #[test]
    fn parser_accepts_dense_text_without_newlines() {
        // Платформа принимает запись без переводов строк — разбор решает
        // только структура скобок и запятых.
        let v = read("{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,{2,{\"N\",1},{\"S\",\"а\"}}}");
        assert_eq!(v.collection_len().unwrap(), 2);
    }

    #[test]
    fn fixed_array_reads_as_a_plain_array() {
        let v = read("{\"#\",4500381b-db30-4a10-9db4-990038032acf,\n{1,\n{\"N\",1}\n}\n}");
        assert_eq!(v.collection_len().unwrap(), 1);
    }

    fn table_value(t: Rc<std::cell::RefCell<crate::ValueTableData>>) -> BslValue {
        BslValue::Object(Rc::new(BslObject::ValueTable(t)))
    }

    #[test]
    fn empty_value_table_matches_the_platform_byte_for_byte() {
        // Эталон `VT.EMPTY` разведочной батареи.
        let v = table_value(crate::ValueTableData::new());
        assert_eq!(
            write(&v),
            "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{0},\n{2,0,\n{1,0},-1,-1},\n{0,0}\n}\n}"
        );
    }

    #[test]
    fn one_cell_value_table_matches_the_platform_byte_for_byte() {
        // Эталон `VT.COL1ROW1`.
        let t = crate::ValueTableData::new();
        {
            let mut t = t.borrow_mut();
            t.add_column("А");
            t.add_row();
            t.columns[0][0] = num("5");
        }
        assert_eq!(
            write(&table_value(t)),
            "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"А\",\n{\"Pattern\"},\"\",0}\n},\n{2,1,0,0,\n{1,1,\n{2,0,1,\n{\"N\",5},0}\n},0,0},\n{0,0}\n}\n}"
        );
    }

    #[test]
    fn typed_column_matches_the_platform_byte_for_byte() {
        // Эталон `VT.TYPE.S`.
        let t = crate::ValueTableData::new();
        {
            let mut t = t.borrow_mut();
            t.add_column("С");
            t.column_types[0] = Some(vec![crate::table::ColumnType::plain(TypeId::String)]);
        }
        assert_eq!(
            write(&table_value(t)),
            "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"С\",\n{\"Pattern\",\n{\"S\"}\n},\"\",0}\n},\n{2,1,0,0,\n{1,0},0,-1},\n{0,0}\n}\n}"
        );
    }

    #[test]
    fn composite_column_types_serialize_in_the_canonical_platform_order() {
        // Эталоны `CMP.NS`/`CMP.SN`: порядок задания типов не влияет —
        // платформа всегда пишет буквы в порядке B, S, D, N.
        let expected = "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"К\",\n{\"Pattern\",\n{\"S\"},\n{\"N\"}\n},\"\",0}\n},\n{2,1,0,0,\n{1,0},0,-1},\n{0,0}\n}\n}";
        for order in [
            vec![TypeId::Number, TypeId::String],
            vec![TypeId::String, TypeId::Number],
        ]
        .map(|ids| {
            ids.into_iter()
                .map(crate::table::ColumnType::plain)
                .collect::<Vec<_>>()
        }) {
            let t = crate::ValueTableData::new();
            {
                let mut t = t.borrow_mut();
                t.add_column("К");
                t.column_types[0] = Some(order);
            }
            assert_eq!(write(&table_value(t)), expected);
        }
    }

    #[test]
    fn type_description_value_matches_the_platform_and_round_trips() {
        // Эталоны `CMP.VALUE`/`CMP.VALUE.EMPTY`: значение `ОписаниеТипов`
        // — тот же {"Pattern",…} под своим видом объекта.
        let td = BslValue::Object(Rc::new(BslObject::TypeDescription(vec![
            TypeId::Number,
            TypeId::String,
        ])));
        let text = write(&td);
        assert_eq!(
            text,
            "{\"#\",f5c65050-3bbb-11d5-b988-0050bae0a95d,\n{\"Pattern\",\n{\"S\"},\n{\"N\"}\n}\n}"
        );
        let back = assert_transit(&text);
        assert_eq!(back.type_name(), "ОписаниеТипов");
        let empty = BslValue::Object(Rc::new(BslObject::TypeDescription(Vec::new())));
        assert_eq!(
            write(&empty),
            "{\"#\",f5c65050-3bbb-11d5-b988-0050bae0a95d,\n{\"Pattern\"}\n}"
        );
    }

    #[test]
    fn composite_type_with_a_reference_survives_the_transit() {
        // РЕАЛЬНАЯ строка платформы (`CMP.REF`): составной тип колонки со
        // ссылочным типом из конфигурации — сырое описание сохраняется.
        let text = "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"К\",\n{\"Pattern\",\n{\"N\"},\n{\"#\",c0c2cbee-990e-410d-9c63-b5222c1dacba}\n},\"\",0}\n},\n{2,1,0,0,\n{1,0},0,-1},\n{0,0}\n}\n}";
        assert_transit(text);
        // То же — для самостоятельного значения ОписаниеТипов со ссылкой:
        // материализовать нечем, но транзит непрозрачным значением обязан
        // быть тождественным.
        let value = "{\"#\",f5c65050-3bbb-11d5-b988-0050bae0a95d,\n{\"Pattern\",\n{\"N\"},\n{\"#\",c0c2cbee-990e-410d-9c63-b5222c1dacba}\n}\n}";
        let v = assert_transit(value);
        assert_eq!(v.type_name(), "НепрозрачноеЗначение");
    }

    #[test]
    fn undefined_in_the_middle_survives_the_round_trip() {
        // Эталоны `VT2.U_MID`/`VT2.RT_MID`: хвостовые `Неопределено`
        // отброшены, внутреннее — явным `{"U"}`, чтение восстанавливает.
        let t = crate::ValueTableData::new();
        {
            let mut t = t.borrow_mut();
            t.add_column("А");
            t.add_column("Б");
            t.add_column("В");
            t.add_row();
            t.columns[0][0] = num("1");
            t.columns[2][0] = num("3");
        }
        let v = table_value(t);
        let text = write(&v);
        assert_eq!(
            text,
            "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{3,\n{0,\"А\",\n{\"Pattern\"},\"\",0},\n{1,\"Б\",\n{\"Pattern\"},\"\",0},\n{2,\"В\",\n{\"Pattern\"},\"\",0}\n},\n{2,3,0,0,1,1,2,2,\n{1,1,\n{2,0,3,\n{\"N\",1},\n{\"U\"},\n{\"N\",3},0}\n},2,0},\n{0,0}\n}\n}"
        );
        let mut context = rt();
        let back = value_from_string_internal(&text, &mut context).unwrap();
        let again = value_to_string_internal(&back, &context).unwrap();
        assert_eq!(text, again, "повторная сериализация ТЗ обязана совпасть");
    }

    /// Транзит: чтение и обратная запись обязаны вернуть строку байт в
    /// байт — это контракт обмена 1С -> open-bsl -> 1С.
    fn assert_transit(text: &str) -> BslValue {
        let mut context = rt();
        let back = value_from_string_internal(text, &mut context).unwrap();
        let again = value_to_string_internal(&back, &context).unwrap();
        assert_eq!(text, again, "транзит обязан быть тождественным");
        back
    }

    #[test]
    fn uuid_value_survives_the_transit_as_opaque() {
        // РЕАЛЬНАЯ строка платформы (проба `UUID` разведочной батареи).
        let text = format!("{{\"#\",{UUID_VALUE_ID},12345678-9abc-def0-1234-56789abcdef0}}");
        let v = assert_transit(&text);
        assert_eq!(v.type_name(), "НепрозрачноеЗначение");
    }

    #[test]
    fn value_list_survives_the_transit_as_opaque() {
        // РЕАЛЬНАЯ строка платформы (проба `VL.ONE`): разметка не
        // материализуется, но транзит тождественен.
        let text = format!(
            "{{\"#\",{VALUE_LIST_ID},\n{{6,1e512aab-1b41-4ef6-9375-f0137be9dd91,0,0,\n{{1,\n{{1e512aab-1b41-4ef6-9375-f0137be9dd91,\n{{\"первый\",0,\n{{\"N\",1}},\n{{4,0,\n{{0}},\"\",-1,-1,0,0,\"\"}},0,0,\"\"}}\n}}\n}},\n{{\"Pattern\"}},0,0}}\n}}"
        );
        assert_transit(&text);
    }

    #[test]
    fn database_reference_shape_survives_the_transit_inside_a_container() {
        // Ссылка объекта базы — незнакомый UUID вида: хранится непрозрачно
        // и переживает транзит и сама по себе, и внутри коллекции.
        let ref_text =
            "{\"#\",aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee,bbbbbbbb-cccc-dddd-eeee-ffffffffffff}";
        assert_transit(ref_text);
        let in_array = format!("{{\"#\",{ARRAY_ID},\n{{2,\n{ref_text},\n{ref_text}\n}}\n}}");
        assert_transit(&in_array);
    }

    #[test]
    fn database_references_survive_the_transit_and_compare_by_value() {
        // РЕАЛЬНЫЕ строки платформы (батарея `REF.*` на конфигурации с
        // справочником/документом/перечислением): вид ссылки —
        // `{"#",<uuid типа>,<код класса>:<32 hex>}`, полезная нагрузка —
        // голая лексема с двоеточием.
        let cat =
            "{\"#\",c0c2cbee-990e-410d-9c63-b5222c1dacba,53:866e48f17f20bbba11f190e0bddf9004}";
        let enum_ref =
            "{\"#\",67ca3817-4032-4140-ba35-d1324b0d84ed,55:8e402318585c2355415dbf5d499c83fe}";
        let empty =
            "{\"#\",c0c2cbee-990e-410d-9c63-b5222c1dacba,53:00000000000000000000000000000000}";
        assert_transit(cat);
        assert_transit(enum_ref);
        assert_transit(empty);

        // Платформа: перечитанная ссылка РАВНА исходной (`REF.CAT.RT`).
        // Непрозрачные значения сравниваются по тексту — то же поведение.
        let mut context = rt();
        let a = value_from_string_internal(cat, &mut context).unwrap();
        let b = value_from_string_internal(cat, &mut context).unwrap();
        assert!(a.eq_value(&b), "одинаковые ссылки обязаны быть равны");
        let other = value_from_string_internal(enum_ref, &mut context).unwrap();
        assert!(!a.eq_value(&other), "разные ссылки равны быть не обязаны");
    }

    #[test]
    fn unknown_type_uuid_survives_the_transit_as_opaque() {
        // `{"T",…}` с UUID типа из чужой конфигурации.
        let v = assert_transit("{\"T\",aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee}");
        assert_eq!(v.type_name(), "НепрозрачноеЗначение");
    }

    #[test]
    fn sparse_row_and_column_ids_survive_the_transit() {
        // Синтетическая таблица «со следами жизни»: колонки с разреженными
        // идентификаторами (0, 2 — вторая и третья удалены), строки с
        // идентификаторами 5 и 9 не по порядку, служебный `X` хвоста не
        // равен ни числу колонок, ни максимальному идентификатору. Такие
        // строки порождают реальные выгрузки платформы после
        // `Свернуть`/`Скопировать`/удалений — транзит обязан вернуть всё
        // байт в байт, а не перенумеровать.
        let text = "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{2,\n{0,\"А\",\n{\"Pattern\"},\"\",0},\n{2,\"Б\",\n{\"Pattern\"},\"\",0}\n},\n{2,2,0,0,1,2,\n{1,2,\n{2,9,1,\n{\"N\",1},0},\n{2,5,2,\n{\"N\",2},\n{\"N\",3},0}\n},7,9},\n{0,0}\n}\n}";
        let back = assert_transit(text);
        // Идентификаторы строк легли в `row_ids`, а не только в текст.
        let BslValue::Object(o) = &back else {
            panic!("ожидался объект")
        };
        let BslObject::ValueTable(data) = &**o else {
            panic!("ожидалась таблица")
        };
        assert_eq!(data.borrow().row_ids, vec![9, 5]);
    }

    #[test]
    fn column_title_width_and_qualifiers_survive_the_transit() {
        // РЕАЛЬНЫЕ строки платформы: `VT.TITLE` (заголовок и ширина) и
        // `VT.TYPE.NQ` (квалификаторы числа) — модель колонок этого не
        // хранит семантически, но транзит обязан вернуть байт в байт.
        let title = "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"А\",\n{\"Pattern\"},\"Заголовок А\",15}\n},\n{2,1,0,0,\n{1,0},0,-1},\n{0,0}\n}\n}";
        assert_transit(title);
        let qualifiers = "{\"#\",acf6192e-81ca-46ef-93a6-5a6968b78663,\n{9,\n{1,\n{0,\"Ч\",\n{\"Pattern\",\n{\"N\",10,2,0}\n},\"\",0}\n},\n{2,1,0,0,\n{1,0},0,-1},\n{0,0}\n}\n}";
        assert_transit(qualifiers);
    }

    #[test]
    fn file_pair_matches_the_platform_byte_for_byte_and_round_trips() {
        // Платформенный файл: UTF-8 с BOM и CRLF, включая перевод внутри
        // строкового значения (измерено побайтово, серия vfile-*).
        let dir = std::env::temp_dir().join(format!("open-bsl-vstr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("проба.txt");
        let path_str = path.to_str().unwrap();

        let mut context = rt();
        let value = s("а\"б\nв");
        value_to_file(path_str, &value, &context).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF], "BOM обязателен");
        let body = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert_eq!(body, "{\"S\",\"а\"\"б\r\nв\"}", "перевод строки — CRLF");

        // Чтение нормализует пары обратно: значение равно исходному.
        let back = value_from_file(path_str, &mut context).unwrap();
        assert!(back.eq_value(&value), "строка обязана вернуться без \\r");

        // Файл без BOM и с сырыми LF (как выгрузки внешних инструментов)
        // читается так же.
        std::fs::write(&path, "{\"N\",42}\n").unwrap();
        let n = value_from_file(path_str, &mut context).unwrap();
        assert_eq!(n, num("42"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cyclic_value_is_a_catchable_error_not_a_crash() {
        // ИЗМЕРЕНО: платформа на этом значении ПАДАЕТ (сегфолт клиента).
        // Здесь — перехватываемая ошибка; краш не воспроизводим сознательно.
        let arr = BslValue::new_array(Vec::new());
        arr.push_element(arr.clone()).unwrap();
        let e = value_to_string_internal(&arr, &rt()).unwrap_err();
        assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
    }

    #[test]
    fn garbage_and_count_mismatch_are_errors() {
        let mut context = rt();
        assert!(matches!(
            value_from_string_internal("не формат", &mut context),
            Err(RtError::Vstr(_))
        ));
        assert!(matches!(
            value_from_string_internal(
                "{\"#\",51e7a0d2-530b-11d4-b98a-008048da3034,{2,{\"N\",1}}}",
                &mut context
            ),
            Err(RtError::Vstr(_))
        ));
        assert!(matches!(
            value_from_string_internal("{\"S\",\"незакрытая}", &mut context),
            Err(RtError::Vstr(_))
        ));
    }

    #[test]
    fn double_serialization_escapes_like_the_platform() {
        // Эталон `VSTR.NESTED`: сериализация строки, которая сама является
        // сериализацией, — обычное строковое экранирование кавычек.
        let once = write(&s("а\"б"));
        let twice = write(&s(&once));
        assert_eq!(twice, "{\"S\",\"{\"\"S\"\",\"\"а\"\"\"\"б\"\"}\"}");
    }
}
