//! Мост «значение BSL ↔ JSON»: сборка значений из событий разбора,
//! сериализация значений в писателя и глобальные функции над ними.

use std::collections::HashMap;
use std::rc::Rc;

use bsl_rt::{BslObject, BslValue, RtError, RtResult, StructureStorage};

use bsl_rt::CallContext;

use crate::{dates::*, objects::*, parse::*, write::*};

/// Реализация `ПрочитатьJSON`.
///
/// `call` — как звать функцию восстановления по имени; `None` означает,
/// что зовущий контекст её предоставить не может (`CallContext::new`
/// против `CallContext::with_function_caller`). Документ без функции
/// восстановления читается и так, а запрошенная функция при `None`
/// становится внятной ошибкой — вместо неё нечего вызвать.
///
/// # Errors
///
/// Ошибка типа, разбора JSON или вызова функции восстановления.
pub fn read_json_builtin(
    arguments: &[BslValue],
    runtime: &mut RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
    call: Option<JsonCallByName<'_>>,
) -> RtResult<BslValue> {
    let as_map = match arguments.get(1) {
        None | Some(BslValue::Undefined) => false,
        Some(BslValue::Boolean(value)) => *value,
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "ПрочитатьJSON(ВозвращатьСоответствие)",
            });
        }
    };
    let date_names = name_list_arg(
        arguments.get(2),
        runtime,
        "ПрочитатьJSON(ИменаСвойствСоЗначениямиДата)",
    )?;
    let date_format =
        optional_date_format_from_arg(arguments.get(3), "ПрочитатьJSON(ОжидаемыйФорматДаты)")?;
    let name = callback_name(
        arguments.get(4),
        arguments.get(5),
        "ПрочитатьJSON(ИмяФункцииВосстановления)",
    )?;
    let restore = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(JsonRestoreFn {
            name,
            extra: arguments.get(6).cloned().unwrap_or(BslValue::Undefined),
            property_names: name_list_arg(
                arguments.get(7),
                runtime,
                "ПрочитатьJSON(ИменаСвойствДляФункцииВосстановления)",
            )?,
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ПрочитатьJSON: функция восстановления требует исполняющей VM".to_string(),
            ));
        }
    };
    read_json(
        &arguments[0],
        as_map,
        &date_names,
        date_format,
        restore,
        runtime,
        zone,
    )
}

/// Реализация `ЗаписатьJSON`.
///
/// `call` — как звать функцию преобразования по имени; про `None` см.
/// [`read_json_builtin`].
///
/// # Errors
///
/// Ошибка типа, записи JSON или вызова функции преобразования.
pub fn write_json_builtin(
    arguments: &[BslValue],
    runtime: &mut RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
    call: Option<JsonCallByName<'_>>,
) -> RtResult<BslValue> {
    let settings = serializer_settings_from(arguments.get(2))?;
    let name = callback_name(
        arguments.get(3),
        arguments.get(4),
        "ЗаписатьJSON(ИмяФункцииПреобразования)",
    )?;
    let convert = match (name, call) {
        (None, _) => None,
        (Some(name), Some(call)) => Some(JsonConvertFn {
            name,
            extra: arguments.get(5).cloned().unwrap_or(BslValue::Undefined),
            call,
        }),
        (Some(_), None) => {
            return Err(RtError::Json(
                "ЗаписатьJSON: функция преобразования требует исполняющей VM".to_string(),
            ));
        }
    };
    write_json(
        &arguments[0],
        &arguments[1],
        &settings,
        convert,
        runtime,
        zone,
    )?;
    Ok(BslValue::Undefined)
}

// --- ПрочитатьJSON / ЗаписатьJSON ---------------------------------------

use bsl_rt::RuntimeShapes;

/// Подготовленные имена свойств в пределах одного `ПрочитатьJSON`.
///
/// Ключ хранит точное написание из JSON: разный регистр может дать
/// две записи в этом кэше, но `NameInterner` всё равно вернёт им один
/// регистронезависимый `NameId`. Это сохраняет семантику и не требует
/// Unicode-нормализации на каждом повторе одной и той же схемы.
type JsonKeyCache = HashMap<Box<str>, bsl_rt::NameId>;

/// Итоговые формы объектов, уже встреченные в текущем документе.
///
/// Первый объект каждой схемы строится обычными `structure_insert`: так
/// сохраняются порог переходов и деградация в словарь. Повторный объект
/// получает ту же форму и готовые слоты сразу, без прохода по цепочке
/// промежуточных форм для каждого поля.
type JsonShapeCache = HashMap<Vec<bsl_rt::NameId>, Rc<bsl_rt::Shape>>;

#[derive(Default)]
pub(crate) struct JsonBuildCache {
    keys: JsonKeyCache,
    shapes: JsonShapeCache,
}

/// Имя свойства годится в поле структуры? Платформа отвергает ключ,
/// который не является идентификатором (измерено: `{"не имя":1}` при
/// разборе в структуру — ошибка), поэтому проверка обязана быть здесь, а
/// не «как получится»: интернер-то примет любую строку.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Разбирает дату из строки JSON. Поддержаны формы ISO без зоны и с `Z`.
///
/// ОТКЛОНЕНИЕ, СОЗНАТЕЛЬНОЕ: платформа на `...Z` переводит момент в
/// ЛОКАЛЬНОЕ время (измерено — `05:06:07Z` вернулось как `8:06:07` в зоне
/// UTC+3), а здесь суффикс только распознаётся, сдвига нет. Причина не в
/// лени: у движка вообще нет понятия часового пояса — `ТекущаяДата` тоже
/// отдаёт UTC (давнее задокументированное отклонение), и вводить смещение
/// ради одной функции значило бы завести полузону, о которой не знает
/// остальной рантайм.
///
/// `ПрочитатьДатуJSON`/`ЗаписатьДатуJSON` (см. `read_json_date`/
/// `write_json_date` ниже) — ИСКЛЮЧЕНИЕ из этого правила, а не отказ от
/// него: смещение машины (`bsl_rt::tz`) там нужно самой сутью функций
/// (варианты `ЛокальнаяДатаСоСмещением`/`УниверсальнаяДата`
/// `ВариантЗаписиДатыJSON` описаны платформой именно через часовой пояс
/// машины), это явно заказанная этим этапом способность, а не тихое
/// распространение зоны на весь модуль `json`. `ИменаСвойствСоЗначениямиДата`
impl JsonRestoreFn<'_> {
    /// Зовётся ли функция восстановления для значения, лежащего под именем
    /// `property` (`None` — элемент массива или корень документа)?
    ///
    /// ИЗМЕРЕНО на 8.3.27, документ `{"а":1,"б":2,"в":{"б":3,"г":4}}`:
    /// * без списка имён функция получает ВСЕ значения — включая элементы
    ///   массивов (`Свойство = Неопределено`) и сам корень;
    /// * со списком `["б"]` — ровно два вызова, оба на свойстве `б`
    ///   (внешнем и вложенном), и НИ ОДНОГО на корне;
    /// * со списком `["БЭ"]` против свойства `бэ` — ни одного вызова,
    ///   то есть сравнение РЕГИСТРОЗАВИСИМОЕ (как и у
    ///   `ИменаСвойствСоЗначениямиДата`, см. [`is_date_property`]).
    fn applies_to(&self, property: Option<&str>) -> bool {
        if self.property_names.is_empty() {
            return true;
        }
        property.is_some_and(|p| self.property_names.iter().any(|n| n == p))
    }
}

/// Всё, что при сборке значения не меняется от узла к узлу.
///
/// Отдельной структурой, потому что без неё `build_value` пришлось бы
/// тащить девять параметров сквозь рекурсию, а функция восстановления
/// добавляет десятый — и повторное `&mut`-заимствование каждого из них на
/// каждом уровне.
pub(crate) struct BuildCtx<'a, 'c> {
    pub(crate) as_map: bool,
    pub(crate) date_names: &'a [String],
    pub(crate) date_format: Option<JsonDateFormat>,
    /// Функция восстановления или `None`, если она не задана.
    pub(crate) restore: Option<JsonRestoreFn<'c>>,
    pub(crate) rt: &'a mut RuntimeShapes,
    /// Часовой пояс прогона: даты `Z` и со смещением переводятся в
    /// местное время, а какое оно — знает окружение, а не машина.
    pub(crate) zone: &'a dyn bsl_rt::TimeZone,
    pub(crate) cache: JsonBuildCache,
}

impl BuildCtx<'_, '_> {
    /// Нужно ли звать функцию восстановления на значении под именем
    /// `property`.
    fn restores(&self, property: Option<&str>) -> bool {
        self.restore
            .as_ref()
            .is_some_and(|r| r.applies_to(property))
    }

    /// Вызов функции восстановления на уже собранном значении.
    ///
    /// # Errors
    ///
    /// Ошибку самого вызова (нет такой функции, не то число параметров) и
    /// любое исключение изнутри функции — ИЗМЕРЕНО, что платформа их не
    /// глотает, а выпускает наружу из `ПрочитатьJSON`.
    fn call_restore(&mut self, property: Option<&str>, value: BslValue) -> RtResult<BslValue> {
        let Some(restore) = self.restore.as_mut() else {
            return Ok(value);
        };
        let args = vec![property_arg(property), value, restore.extra.clone()];
        let (returned, _) = (restore.call)(&restore.name, args)?;
        Ok(returned)
    }
}

/// Первый параметр колбэка: имя свойства или `Неопределено`.
///
/// ИЗМЕРЕНО: `Неопределено` приходит и для элемента массива, и для
/// верхнего уровня документа — платформа не выдумывает им ни индекса, ни
/// пустой строки.
fn property_arg(property: Option<&str>) -> BslValue {
    match property {
        Some(p) => BslValue::Str(bsl_rt::BslString::from_str(p)),
        None => BslValue::Undefined,
    }
}

/// `ПрочитатьJSON(Чтение[, ВозвращатьСоответствие[, ИменаСвойствСоЗначениямиДата
/// [, ОжидаемыйФорматДаты[, ИмяФункцииВосстановления, ...]]]])`.
///
/// `date_format` — четвёртый аргумент платформы: `None`, если он не задан
/// (тогда разбор `ИменаСвойствСоЗначениямиДата` идёт по старому правилу —
/// см. `optional_date_format_from_arg`). `restore` — функция восстановления
/// (пятый-восьмой аргументы), см. [`JsonRestoreFn`].
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе, на ключе, который не может быть именем
/// поля структуры, либо (при заданном `date_format`) на значении из
/// `ИменаСвойствСоЗначениямиДата`, не разобравшемся в этом формате
/// (см. `bad_date_representation`, `JSON.READ_DATE.BAD_FORMAT_TEXT`);
/// ошибку вызова функции восстановления и любое исключение из неё.
pub fn read_json(
    reader: &BslValue,
    as_map: bool,
    date_names: &[String],
    date_format: Option<JsonDateFormat>,
    restore: Option<JsonRestoreFn<'_>>,
    rt: &mut RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    // Первое событие читается здесь же: `ПрочитатьJSON` забирает документ
    // с текущей позиции целиком, и вызывать перед ним `Прочитать()` не
    // требуется.
    let cell = as_reader(arg_object(reader)?)?;
    // Разборщик ВЫНИМАЕТСЯ из ячейки на всё время сборки, а не держится
    // заимствованным: функция восстановления — это пользовательский код,
    // и он волен потрогать тот же самый `ЧтениеJSON`. С `borrow_mut()`
    // такой повторный вход был бы паникой `RefCell` мимо `Попытка`; без
    // разборщика в ячейке он упирается в обычную перехватываемую ошибку
    // «нет назначенного источника». Платформа в этом месте тоже отвечает
    // ошибкой («Недопустимое состояние потока чтения JSON»), а не молча
    // продолжает, — текст у нас свой, как и для остальных ошибок JSON.
    let (first, mut parser) = {
        let mut state = cell.borrow_mut();
        let Some(mut parser) = state.parser.take() else {
            return Err(RtError::TypeError {
                expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
                op: "ПрочитатьJSON",
            });
        };
        // Текущее событие, если на него уже встали ручным `Прочитать()`,
        // иначе следующее: `ПрочитатьJSON` работает в обоих сценариях.
        let first = match state.current.take() {
            Some(e) => Ok(Some(e)),
            None => parser.next_event(),
        };
        match first {
            Ok(first) => (first, parser),
            Err(e) => {
                state.parser = Some(parser);
                return Err(e);
            }
        }
    };

    let mut ctx = BuildCtx {
        as_map,
        date_names,
        date_format,
        restore,
        rt,
        zone,
        cache: JsonBuildCache::default(),
    };
    let built = match first {
        None => Ok(BslValue::Undefined),
        Some(first) => build_value(first, &mut parser, None, &mut ctx, 0),
    };
    // Разборщик возвращается на место при любом исходе: после ошибки
    // `ЧтениеJSON` обязан остаться тем же объектом, у которого можно
    // спросить `Закрыть()`.
    cell.borrow_mut().parser = Some(parser);
    built
}

/// `ОжидаемыйФорматДаты` — четвёртый аргумент `ПрочитатьJSON`.
///
/// Отсутствует (`Неопределено`) — `None`: разбор
/// `ИменаСвойствСоЗначениямиДата` по умолчанию — ISO, но через СТАРЫЙ
/// парсер без сдвига зоны (`parse_json_date`, не
/// `parse_json_date_by_format`) — это отдельное, ранее измеренное
/// намеренное отклонение (см. doc comment на `parse_json_date`), не
/// тронутое добавлением этого аргумента. ИЗМЕРЕНО: разбор СТРОГИЙ даже без
/// явного формата — представление, не разобравшееся как ISO, даёт то же
/// исключение, что и при явном формате (до замера здесь был тихий фолбэк
/// в строку).
///
/// Задан — платформа проверяет представление СТРОГО под этот формат;
/// несовпадение (в том числе значение вовсе не строка — статья приводит
/// пример с числом при `ФорматДатыJSON.JavaScript`) — исключение с тем же
/// текстом, что и у `ПрочитатьДатуJSON` (`JSON.READ_DATE.BAD_FORMAT_TEXT`).
///
/// ИЗМЕРЕНО и НЕ ВОСПРОИЗВОДИТСЯ: платформа различает ПРОПУЩЕННЫЙ аргумент
/// и явно переданное `Неопределено`. `ПрочитатьJSON(Ч, Ложь, , , "Имя")`
/// работает, а `ПрочитатьJSON(Ч, Ложь, Неопределено, Неопределено, "Имя")`
/// падает с «Несоответствие типов (параметр номер '4')» — то есть
/// `ОжидаемыйФорматДаты` принимает только `ФорматДатыJSON`, но
/// необязательность проверяет по факту передачи. Здесь этого различия нет:
/// резолвер добивает необязательные позиции встроенного вызова именно
/// `Неопределено` (см. `call_builtin_with_format`), так что оба написания
/// приходят сюда одинаковыми, и отвергать `Неопределено` значило бы сломать
/// все вызовы с пропущенным форматом. Это НЕ открытый вопрос — поведение
/// платформы известно; воспроизвести его нечем без отдельного маркера
/// «аргумент не передавали» в байт-коде встроенных вызовов.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент задан и не `ФорматДатыJSON`.
pub fn optional_date_format_from_arg(
    arg: Option<&BslValue>,
    op: &'static str,
) -> RtResult<Option<JsonDateFormat>> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Enum(e)) => {
            JsonDateFormat::from_enum_value(*e)
                .map(Some)
                .ok_or(RtError::TypeError {
                    expected: "ФорматДатыJSON",
                    op,
                })
        }
        Some(_) => Err(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op,
        }),
    }
}

/// `ПрочитатьЗначениеJSON(Строка)` -> значение — обратная операция к
/// `ЗаписатьЗначениеJSON`, поверх того же `build_value`, которым разбирает
/// и `ПрочитатьJSON`.
///
/// ИЗМЕРЕНО (`JSON.VALUE.READ_KIND`): объект JSON превращается в
/// `Структура`, а не в `Соответствие` — тот же выбор по умолчанию, что и у
/// `ПрочитатьJSON` без второго аргумента (`JSON.DESERIALIZE.DEFAULT_TYPE`).
///
/// ИЗМЕРЕНО: пустая строка — тоже исключение, а не тихое `Неопределено`
/// (снято прогоном фикстуры `json-dates`; до замера здесь было наоборот).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка; [`RtError::Json`] на
/// пустой строке (в ней нет ни одного события разбора); иначе — см.
/// [`read_json`].
pub fn read_json_value(
    text: &BslValue,
    rt: &mut RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    let BslValue::Str(s) = text else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ПрочитатьЗначениеJSON",
        });
    };
    let mut parser = JsonParser::from_bsl_string(s);
    let Some(first) = parser.next_event()? else {
        return Err(RtError::Json(
            "пустая строка не представляет значение JSON".to_string(),
        ));
    };
    // Функции восстановления у `ПрочитатьЗначениеJSON` нет вовсе — у
    // платформы такого параметра здесь не существует.
    let mut ctx = BuildCtx {
        as_map: false,
        date_names: &[],
        date_format: None,
        restore: None,
        rt,
        zone,
        cache: JsonBuildCache::default(),
    };
    build_value(first, &mut parser, None, &mut ctx, 0)
}

/// Проверяет и интернирует имя один раз за разбор документа.
fn json_field_id(
    name: &str,
    rt: &mut RuntimeShapes,
    cache: &mut JsonKeyCache,
) -> RtResult<bsl_rt::NameId> {
    if let Some(&id) = cache.get(name) {
        return Ok(id);
    }
    if !is_identifier(name) {
        return Err(RtError::Json(format!(
            "ключ «{name}» не может быть именем свойства структуры"
        )));
    }
    let id = rt.names.intern(name);
    cache.insert(name.into(), id);
    Ok(id)
}

/// Собирает JSON-объект в `Структура`.
///
/// Дублирующееся имя перезаписывает прежний слот, но не меняет
/// его позицию. Это та же семантика, что у последовательных
/// `Структура.Вставить`.
fn build_json_structure(
    keys: Vec<String>,
    values: Vec<BslValue>,
    rt: &mut RuntimeShapes,
    cache: &mut JsonBuildCache,
) -> RtResult<BslValue> {
    // На типовых коротких схемах линейный поиск дешевле ещё
    // одной таблицы и её хэширования. Длинная схема переходит на
    // индекс, чтобы дубликаты не превратили большой объект в O(n²).
    const LINEAR_LOOKUP_LIMIT: usize = 16;

    let mut names = Vec::with_capacity(keys.len());
    let mut slots = Vec::with_capacity(values.len());
    let mut positions: Option<HashMap<bsl_rt::NameId, usize>> = None;
    for (key, value) in keys.into_iter().zip(values) {
        let id = json_field_id(&key, rt, &mut cache.keys)?;
        let old_slot = match &positions {
            Some(index) => index.get(&id).copied(),
            None => names.iter().position(|&known| known == id),
        };
        if let Some(slot) = old_slot {
            slots[slot] = value;
        } else {
            let slot = names.len();
            names.push(id);
            slots.push(value);
            if let Some(index) = positions.as_mut() {
                index.insert(id, slot);
            } else if names.len() == LINEAR_LOOKUP_LIMIT + 1 {
                positions = Some(
                    names
                        .iter()
                        .copied()
                        .enumerate()
                        .map(|(slot, name)| (name, slot))
                        .collect(),
                );
            }
        }
    }

    if let Some(shape) = cache.shapes.get(names.as_slice()) {
        return Ok(BslValue::new_structure(shape.clone(), slots));
    }

    let empty = rt.shapes.empty();
    let object = BslValue::new_structure(empty, Vec::new());
    for (&id, value) in names.iter().zip(slots) {
        object.structure_insert(id, value, &mut rt.shapes)?;
    }

    // Словарную структуру не кэшируем: прямое создание с
    // произвольными именами вернуло бы её в таблицу бессрочных форм.
    let built_shape = match &object {
        BslValue::Object(value) => match &**value {
            BslObject::Structure(storage) => match &*storage.borrow() {
                StructureStorage::Shaped { shape, .. } => Some(shape.clone()),
                StructureStorage::Dictionary { .. } => None,
            },
            _ => None,
        },
        _ => None,
    };
    if let Some(shape) = built_shape {
        cache.shapes.insert(names, shape);
    }
    Ok(object)
}

/// Свойство `property` перечислено в `ИменаСвойствСоЗначениямиДата`?
///
/// ИЗМЕРЕНО: сравнение РЕГИСТРОЗАВИСИМОЕ — вопреки общему правилу языка,
/// где идентификаторы регистр не различают. Проба: документ
/// `{"создано":"2014-05-10T13:14:15"}` со списком `["создано"]` даёт `Дата`,
/// а с `["СОЗДАНО"]` — по-прежнему `Строка`. Это имена свойств ДОКУМЕНТА,
/// а не идентификаторы BSL, и платформа обращается с ними как с ключами
/// JSON. До замера здесь стояло `to_uppercase` на обеих сторонах.
fn is_date_property(property: Option<&str>, date_names: &[String]) -> bool {
    property.is_some_and(|p| date_names.iter().any(|n| n == p))
}

/// Сборка значения из события и продолжения потока с вызовом функции
/// восстановления на готовом результате.
///
/// ИЗМЕРЕНО, что порядок вызовов — ОБРАТНЫЙ (сначала дети, потом родитель).
/// Документ
/// `{"чис":1,"стр":"т","лог":true,"нул":null,"об":{"вчис":2,"вмас":[7,8]},
/// "мас":[3,"ф",false,null,{"мчис":4},[5,6]]}` даёт ровно двадцать вызовов
/// в порядке `чис`, `стр`, `лог`, `нул`, `вчис`, элемент, элемент, `вмас`,
/// `об`, элемент, элемент, элемент, элемент, `мчис`, элемент (объект),
/// элемент, элемент, элемент (массив), `мас`, корень: значение приходит в
/// функцию уже собранным из УЖЕ восстановленных детей, а её результат
/// становится тем, что увидит родитель. Поэтому вызов стоит здесь, на
/// выходе, а не в родительских ветках.
fn build_value(
    event: JsonEvent,
    parser: &mut JsonParser,
    property: Option<&str>,
    ctx: &mut BuildCtx<'_, '_>,
    depth: usize,
) -> RtResult<BslValue> {
    let restores = ctx.restores(property);
    let value = build_raw_value(event, parser, property, restores, ctx, depth)?;
    if restores {
        ctx.call_restore(property, value)
    } else {
        Ok(value)
    }
}

/// Сборка значения из события и продолжения потока — без функции
/// восстановления.
///
/// `property` — имя свойства, под которым это значение лежит у родителя:
/// по нему решается, превращать ли строку в дату
/// (`ИменаСвойствСоЗначениямиДата`). `date_format` — четвёртый аргумент
/// `ПрочитатьJSON` (см. `optional_date_format_from_arg`): `None` — старое
/// правило (только ISO, неудача молча оставляет строку), `Some(fmt)` —
/// значение обязано разобраться СТРОГО под этот формат.
///
/// `restores` — будет ли на ЭТОМ значении вызвана функция восстановления.
/// ИЗМЕРЕНО, что она отменяет разбор даты: документ
/// `{"создано":"2014-05-10T13:14:15","прочее":1}` с
/// `ИменаСвойствСоЗначениямиДата = ["создано"]` даёт `Дата` без функции
/// восстановления и с функцией, суженной списком до `прочее`, — но
/// `Строка` (сырое представление), если функция зовётся и на `создано`.
/// То есть функция восстановления имеет приоритет над списком дат, а не
/// получает уже готовую дату.
fn build_raw_value(
    event: JsonEvent,
    parser: &mut JsonParser,
    property: Option<&str>,
    restores: bool,
    ctx: &mut BuildCtx<'_, '_>,
    depth: usize,
) -> RtResult<BslValue> {
    // Как и в `serialize`: рекурсия возможна только на контейнерах, на
    // скалярных событиях предел не срабатывает никогда.
    if depth > MAX_JSON_DEPTH && matches!(event, JsonEvent::ObjectStart | JsonEvent::ArrayStart) {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность документа при чтении JSON",
        });
    }
    // Разбор даты отменяется, если это значение уходит в функцию
    // восстановления (измерено, см. doc comment).
    let is_date = !restores && is_date_property(property, ctx.date_names);
    match event {
        JsonEvent::ObjectStart => {
            let mut keys: Vec<String> = Vec::new();
            let mut values: Vec<BslValue> = Vec::new();
            loop {
                let Some(next) = parser.next_event()? else {
                    break;
                };
                match next {
                    JsonEvent::ObjectEnd => break,
                    JsonEvent::PropertyName(name) => {
                        let Some(value_event) = parser.next_event()? else {
                            break;
                        };
                        // Конец объекта на месте значения — пропущенное
                        // значение (`{"а":}`), разборщик к такому
                        // снисходителен.
                        if value_event == JsonEvent::ObjectEnd {
                            break;
                        }
                        let v = build_value(value_event, parser, Some(&name), ctx, depth + 1)?;
                        keys.push(name);
                        values.push(v);
                    }
                    _ => break,
                }
            }
            if ctx.as_map {
                let map = BslValue::new_map();
                for (k, v) in keys.into_iter().zip(values) {
                    map.map_insert(BslValue::Str(bsl_rt::BslString::from_str(&k)), v)?;
                }
                Ok(map)
            } else {
                build_json_structure(keys, values, ctx.rt, &mut ctx.cache)
            }
        }
        JsonEvent::ArrayStart => {
            let items = BslValue::new_array(Vec::new());
            loop {
                let Some(next) = parser.next_event()? else {
                    break;
                };
                if next == JsonEvent::ArrayEnd {
                    break;
                }
                let v = build_value(next, parser, None, ctx, depth + 1)?;
                items.push_element(v)?;
            }
            Ok(items)
        }
        JsonEvent::Str(s) => {
            // Дата — только если имя свойства перечислено. JSON типа даты
            // не знает, а гадать по виду строки платформа не берётся, и мы
            // тоже: «2024-03-04» может быть просто строкой.
            //
            // ИЗМЕРЕНО: платформа кидает исключение и БЕЗ явного формата
            // (четвёртого аргумента `ПрочитатьJSON`), если значение не
            // разбирается, — до замера здесь был тихий фолбэк в строку, и
            // фикстура `json-dates` упала ровно на этой пробе (была без
            // `Попытка`). Формат по умолчанию — ISO, но именно СТАРЫЙ
            // парсер без сдвига зоны (`parse_json_date`), а не
            // `parse_json_date_by_format(..., Iso, zone)`: сдвиг `Z`/явного
            // смещения в локальное время машины для ЭТОГО (более раннего)
            // пути — отдельное, ранее измеренное намеренное отклонение
            // (см. doc comment на `parse_json_date`), эта правка его не
            // трогает — меняется только СТРОГОСТЬ (ошибка вместо тихого
            // фолбэка), не арифметика разбора.
            if is_date {
                let d = match ctx.date_format {
                    Some(fmt) => parse_json_date_by_format(&s, fmt, ctx.zone),
                    None => parse_json_date(&s),
                }
                .ok_or_else(bad_date_representation)?;
                return Ok(BslValue::Date(d));
            }
            Ok(BslValue::Str(bsl_rt::BslString::from_str(&s)))
        }
        // Число/булево на месте объявленного имени даты — заведомо не
        // текстовое представление ни одного из трёх форматов (у всех троих
        // момент кодируется строкой), поэтому та же ошибка, что и у
        // несоответствующей строки. Статья приводит именно такой пример —
        // число вместо `"new Date(...)"` — при ЯВНО заданном формате;
        // ИЗМЕРЕНО, что без него платформа тоже не прощает (см. выше), так
        // что здесь проверка больше не зависит от `date_format.is_some()`.
        JsonEvent::Number(n) => {
            if is_date {
                return Err(bad_date_representation());
            }
            Ok(BslValue::Number(n))
        }
        JsonEvent::Boolean(b) => {
            if is_date {
                return Err(bad_date_representation());
            }
            Ok(BslValue::Boolean(b))
        }
        // ИЗМЕРЕНО: `null` становится `Неопределено`, а НЕ `Null`. `null`
        // не проверяется на соответствие формату даты даже при явном
        // формате: это осмысленное «нет значения», а не мусор на месте
        // даты, и статья не даёт для него примера — расширять список
        // отвергаемых значений домыслом не стоит.
        JsonEvent::Null => Ok(BslValue::Undefined),
        JsonEvent::PropertyName(s) => Ok(BslValue::Str(bsl_rt::BslString::from_str(&s))),
        JsonEvent::ObjectEnd | JsonEvent::ArrayEnd => Ok(BslValue::Undefined),
    }
}

/// `ЗаписатьJSON(Запись, Значение)`.
///
/// # Errors
///
/// [`RtError::TypeError`] на значении, которое сериализовать нечем
/// (`ТаблицаЗначений` и прочие объекты) — измерено, платформа тоже
/// отвергает.
/// Предел вложенности данных при записи и документа при чтении JSON.
/// И `serialize`, и `build_value` рекурсивны, поэтому глубина входа
/// напрямую расходует стек Rust; без предела циклическая структура в
/// `ЗаписатьJSON` (массив, содержащий сам себя) и документ вида `[[[[…`
/// в `ПрочитатьJSON` валят процесс переполнением стека вместо
/// перехватываемой ошибки.
// НЕ ИЗМЕРЕНО(JSON.MAX_DEPTH) — какую глубину допускает платформа и что
// она делает с циклической структурой в `ЗаписатьJSON`; циклический зонд
// намеренно не ставится — если платформа на нём падает, он уносит весь
// сеанс замеров. Замер даёт нижнюю границу: 400 уровней обязаны работать.
pub(crate) const MAX_JSON_DEPTH: usize = 500;

/// `ЗаписатьJSON(Запись, Значение[, Настройки[, ИмяФункцииПреобразования, ...]])`.
///
/// `convert` — функция преобразования (четвёртый-шестой аргументы
/// платформы), см. [`JsonConvertFn`]. ИЗМЕРЕНО, что её имя само по себе НЕ
/// ошибка на входе и что зовётся она ЛЕНИВО — только там, где встретилось
/// значение, которое сериализовать нечем: `ЗаписатьJSON(Запись, 1,
/// Неопределено, "ИмяФункции", ЭтотОбъект)` пишет `1`, ни разу не позвав
/// функцию, и то же самое верно для `Дата` (её платформа сериализует сама).
///
/// # Errors
///
/// См. `serialize`.
pub fn write_json(
    writer: &BslValue,
    value: &BslValue,
    settings: &JsonSerializerSettings,
    convert: Option<JsonConvertFn<'_>>,
    rt: &RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<()> {
    let cell = as_writer(arg_object(writer)?)?;
    // Приёмник, как и разборщик в `read_json`, ВЫНИМАЕТСЯ из ячейки на
    // время записи: функция преобразования — пользовательский код, который
    // волен позвать `ЗаписатьJSON` на том же самом объекте, а `borrow_mut()`
    // поперёк такого повторного входа был бы паникой `RefCell` мимо
    // `Попытка`. Платформа отвечает на этот случай ошибкой («Неверный
    // порядок записи JSON»), не паникой, — здесь получится своя ошибка про
    // отсутствие назначенного приёмника.
    let Some(mut w) = cell.borrow_mut().take() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "ЗаписатьJSON",
        });
    };
    let mut ctx = SerializeCtx {
        settings,
        single_value_mode: false,
        convert,
        rt,
        zone,
    };
    let written = write_top_level(&mut w, value, &mut ctx);
    // Приёмник возвращается на место при любом исходе: `Закрыть()` после
    // ошибки обязан работать (измерено на отказе функции преобразования на
    // верхнем уровне — там документ пуст, а `Закрыть()` отдаёт пустую
    // строку).
    *cell.borrow_mut() = Some(w);
    written
}

/// Верхний уровень документа. Отдельной функцией из-за `Отказ`: ИЗМЕРЕНО,
/// что отказ функции преобразования на САМОМ значении не пишет вообще
/// ничего — `ЗаписатьJSON(Запись, ТаблицаЗначений, , "Отказная", ЭтотОбъект)`
/// с последующим `Закрыть()` даёт пустую строку, а не `null` и не ошибку.
fn write_top_level(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
) -> RtResult<()> {
    match prepare(value, None, ctx)? {
        Prepared::Skip => Ok(()),
        Prepared::AsIs => serialize(w, value, ctx, 0),
        Prepared::Converted(v) => serialize_converted(w, &v, ctx, 0),
    }
}

/// `ЗаписатьЗначениеJSON(Значение)` — сериализация ОДНОГО значения в
/// строку поверх того же `serialize`, что и `ЗаписатьJSON`, но с
/// `single_value_mode = true`: дата, в том числе вложенная, — исключение
/// (см. обзор задачи в плане реализации, раздел «Этап 0»), и ИЗМЕРЕНО, что
/// `Соответствие` — тоже (`ЗаписатьJSON` с `Соответствие` работает и
/// измерен отдельно, значит отличие — в самой функции
/// `ЗаписатьЗначениеJSON`, не в объектной технике сериализации вообще).
///
/// # Errors
///
/// См. `serialize`; дополнительно [`RtError::TypeError`] на `Дата` или
/// `Соответствие` в любой позиции дерева значения.
pub fn write_json_value(
    value: &BslValue,
    rt: &RuntimeShapes,
    zone: &dyn bsl_rt::TimeZone,
) -> RtResult<BslValue> {
    let mut w = JsonWriter::to_string_target(JsonWriterSettings::default());
    let mut ctx = SerializeCtx {
        settings: &JsonSerializerSettings::default(),
        single_value_mode: true,
        // `ЗаписатьЗначениеJSON` не берёт функцию преобразования вовсе —
        // у платформы такого параметра здесь нет.
        convert: None,
        rt,
        zone,
    };
    serialize(&mut w, value, &mut ctx, 0)?;
    Ok(BslValue::Str(bsl_rt::BslString::from_utf8_string(
        w.finish()?,
    )))
}

/// Ошибка на значении, которое `serialize` сериализовать не умеет
/// (`JSON.SERIALIZE.UNSUPPORTED_TYPE`).
///
/// ИЗМЕРЕНО, что имя функции преобразования само по себе эту ошибку НЕ
/// меняет: без `МодульФункцииПреобразования` платформа функцию не ищет
/// вовсе и отвечает тем же «Значение содержит данные недопустимых типов»,
/// что и без имени. Поэтому текст здесь один на все случаи, когда звать
/// оказалось некого.
fn unsupported_value_error() -> RtError {
    RtError::TypeError {
        expected: "значение, представимое в JSON",
        op: "ЗаписатьJSON",
    }
}

/// Всё, что при сериализации не меняется от узла к узлу.
pub(crate) struct SerializeCtx<'a, 'c> {
    pub(crate) settings: &'a JsonSerializerSettings,
    /// `ЗаписатьЗначениеJSON`: у него свои запреты (`Дата`, `Соответствие`)
    /// и функции преобразования не бывает.
    pub(crate) single_value_mode: bool,
    pub(crate) convert: Option<JsonConvertFn<'c>>,
    pub(crate) rt: &'a RuntimeShapes,
    /// См. `BuildCtx::zone` — при записи зона нужна вариантам
    /// «со смещением» и «универсальная».
    pub(crate) zone: &'a dyn bsl_rt::TimeZone,
}

/// Что писать на месте очередного значения.
enum Prepared {
    /// Само значение: функция преобразования либо не нужна (значение
    /// сериализуемо), либо не задана.
    AsIs,
    /// Результат функции преобразования.
    Converted(BslValue),
    /// `Отказ = Истина`: значение молча выпадает из документа.
    Skip,
}

/// Значения, у которых нет собственного представления в JSON, — ровно те,
/// на которых платформа зовёт функцию преобразования.
///
/// Матч по `BslValue` исчерпывающий намеренно: новый вариант ЗНАЧЕНИЯ обязан
/// решить здесь, сериализуем он сам или уходит в функцию преобразования, —
/// иначе он молча попал бы в «сериализуемые» и упал бы уже в `serialize`.
/// На `BslObject` эта защита НЕ распространяется: ветка написана негативным
/// `matches!`, поэтому новый вариант объекта компилятор здесь не остановит —
/// он по умолчанию попадёт в «несериализуемые», то есть в функцию
/// преобразования. Умолчание консервативное и совпадает с тем, что давал
/// прежний `_ => Err(unsupported_value_error(..))` в `serialize`, но
/// проверить его на новом варианте придётся глазами.
fn needs_convert(value: &BslValue) -> bool {
    match value {
        BslValue::Str(_)
        | BslValue::Number(_)
        | BslValue::Boolean(_)
        | BslValue::Undefined
        | BslValue::Null
        | BslValue::Date(_) => false,
        BslValue::Object(o) => !matches!(
            &**o,
            BslObject::Array(_) | BslObject::Structure(_) | BslObject::Map(_)
        ),
        BslValue::Type(_) | BslValue::Enum(_) | BslValue::EnumType(_) => true,
    }
}

/// Прочтение параметра `Отказ` из финального слота функции преобразования.
///
/// ИЗМЕРЕНО на 8.3.27 четырнадцатью пробами: платформа читает `Отказ` по
/// ОБЫЧНЫМ правилам условия языка, а значение, которое к условию не
/// приводится, отказом не считает. Отказом обернулись `Истина`, `1`, `-1` и
/// строка `"да"`; НЕ обернулись `Ложь`, `0`, `""`, `"   "`, `"абв"`,
/// `Неопределено`, `Null`, пустая и непустая дата, пустой и непустой
/// массив, `Тип("Строка")`. Это ровно [`BslValue::as_condition`] с
/// подавленной ошибкой — включая её нетривиальную часть про строки
/// (истинны только слова «Да»/«Истина»/«True», а не «непустая строка»).
fn refused(final_params: &[BslValue]) -> bool {
    final_params
        .get(3)
        .and_then(|v| v.as_condition().ok())
        .unwrap_or(false)
}

/// Готовит значение к записи в позиции `property`: решает, нужна ли функция
/// преобразования, и зовёт её.
///
/// # Errors
///
/// Ошибку вызова функции (нет такой, не то число параметров) и любое
/// исключение изнутри неё — ИЗМЕРЕНО, что платформа их не глотает.
fn prepare(
    value: &BslValue,
    property: Option<&str>,
    ctx: &mut SerializeCtx<'_, '_>,
) -> RtResult<Prepared> {
    if !needs_convert(value) {
        return Ok(Prepared::AsIs);
    }
    let Some(convert) = ctx.convert.as_mut() else {
        // Звать некого — ошибку выдаст сам `serialize`, чтобы точка отказа
        // была одна.
        return Ok(Prepared::AsIs);
    };
    let args = vec![
        property_arg(property),
        value.clone(),
        convert.extra.clone(),
        BslValue::Boolean(false),
    ];
    let (returned, final_params) = (convert.call)(&convert.name, args)?;
    if refused(&final_params) {
        return Ok(Prepared::Skip);
    }
    Ok(Prepared::Converted(returned))
}

/// Запись значения, УЖЕ прошедшего функцию преобразования.
///
/// ИЗМЕРЕНО, что второй раз на том же месте платформа функцию не зовёт:
/// функция, возвращающая снова `ТаблицаЗначений`, вызывается ровно один раз
/// и запись падает обычной ошибкой типа. При этом возвращённый КОНТЕЙНЕР
/// обходится как обычно — функция, возвращающая `Структура("вложенное",
/// ТаблицаЗначений)`, вызывается на каждом следующем уровне вложенности.
/// Отсюда и разделение: подавляется вызов ровно на этой позиции, а не во
/// всём поддереве.
fn serialize_converted(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    if needs_convert(value) {
        return Err(unsupported_value_error());
    }
    serialize(w, value, ctx, depth)
}

/// Записывает один элемент контейнера, пропуская его при отказе.
///
/// `property` — имя, под которым элемент лежит (`None` для элемента
/// массива); `name` — имя свойства, которое надо написать ПЕРЕД значением,
/// но только если значение в документ попадёт: ИЗМЕРЕНО, что отказ убирает
/// свойство целиком (`{"а": ТаблицаЗначений}` -> `{}`), а не оставляет его
/// с `null`.
fn serialize_member(
    w: &mut JsonWriter,
    name: Option<&str>,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    let prepared = prepare(value, name, ctx)?;
    if matches!(prepared, Prepared::Skip) {
        return Ok(());
    }
    if let Some(name) = name {
        w.property_name(name)?;
    }
    match prepared {
        Prepared::AsIs => serialize(w, value, ctx, depth),
        Prepared::Converted(v) => serialize_converted(w, &v, ctx, depth),
        Prepared::Skip => unreachable!("отказ обработан выше"),
    }
}

/// Общее ядро `ЗаписатьJSON`/`ЗаписатьЗначениеJSON`.
///
/// # Errors
///
/// [`RtError::TypeError`] на значении, которое сериализовать нечем
/// (`ТаблицаЗначений` и прочие объекты, а при `single_value_mode` — ещё и
/// любая `Дата`/`Соответствие`) — измерено, платформа тоже отвергает;
/// [`RtError::StackOverflow`] на слишком глубокой вложенности (см.
/// `MAX_JSON_DEPTH`); ошибку [`format_json_date`] на настройках даты,
/// запрещающих сочетание формата и варианта записи; ошибку вызова функции
/// преобразования и любое исключение из неё.
pub(crate) fn serialize(
    w: &mut JsonWriter,
    value: &BslValue,
    ctx: &mut SerializeCtx<'_, '_>,
    depth: usize,
) -> RtResult<()> {
    // Рекурсия возможна только на контейнерах, поэтому на скалярах предел
    // не срабатывает никогда; проверка стоит одного сравнения `depth`.
    if depth > MAX_JSON_DEPTH && matches!(value, BslValue::Object(_)) {
        return Err(RtError::StackOverflow {
            what: "слишком глубокая вложенность данных при записи JSON \
                   (возможна циклическая ссылка)",
        });
    }
    match value {
        BslValue::Str(_) | BslValue::Number(_) | BslValue::Boolean(_) => w.value(value),
        // `Неопределено` и `Null` отдельным ЗаписатьЗначение платформа не
        // принимает, но в составе сериализуемого значения они обязаны во
        // что-то превращаться — в `null`.
        BslValue::Undefined | BslValue::Null => {
            w.literal("null");
            Ok(())
        }
        BslValue::Date(d) => {
            if ctx.single_value_mode {
                return Err(RtError::TypeError {
                    expected: "значение без Даты (ЗаписатьЗначениеJSON её не сериализует)",
                    op: "ЗаписатьЗначениеJSON",
                });
            }
            // ИЗМЕРЕНО: Microsoft-формат пишется БЕЗ обратных косых
            // (`/Date(мс)/`), так что в содержимом нет символов, которые
            // экранирование JSON вообще трогает (см. `format_json_date`) —
            // обычный `JsonWriter::value` (с проверкой контекста и
            // стандартным экранированием строки) безопасен для всех трёх
            // форматов даты. Какой вид вложенная Microsoft-дата примет в
            // ДОКУМЕНТЕ через `НастройкиСериализацииJSON`, замерит фикстура
            // (проба уже есть в `json-dates.bsl`).
            let content = format_json_date(
                *d,
                ctx.settings.date_format,
                ctx.settings.date_variant,
                ctx.zone,
            )?;
            w.value(&BslValue::Str(bsl_rt::BslString::from_str(&content)))
        }
        BslValue::Object(o) => match &**o {
            BslObject::Array(items) => {
                // Снимок до записи: элемент может оказаться тем же
                // массивом, а `RefCell` вложенного заимствования не
                // переживёт.
                let snapshot: Vec<BslValue> = items.borrow().clone();
                if ctx.settings.arrays_as_objects {
                    // `СериализовыватьМассивыКакОбъекты`: индексы уходят
                    // строковыми именами свойств `"0"`, `"1"`, ...
                    // ИЗМЕРЕНО, что и функция преобразования получает в
                    // `Свойство` этот самый индекс строкой (`"1"`), а не
                    // `Неопределено`, как для настоящего элемента массива:
                    // она видит документ таким, каким он ПИШЕТСЯ.
                    w.begin_object()?;
                    for (i, item) in snapshot.iter().enumerate() {
                        serialize_member(w, Some(&i.to_string()), item, ctx, depth + 1)?;
                    }
                    w.end_object()
                } else {
                    w.begin_array()?;
                    for item in &snapshot {
                        serialize_member(w, None, item, ctx, depth + 1)?;
                    }
                    w.end_array()
                }
            }
            BslObject::Structure(s) => {
                let entries: Vec<(String, BslValue)> = {
                    let s = s.borrow();
                    (0..s.len())
                        .filter_map(|i| s.entry_at(i))
                        .filter_map(|(id, v)| ctx.rt.names.name(id).map(|n| (n.to_string(), v)))
                        .collect()
                };
                w.begin_object()?;
                for (name, v) in &entries {
                    serialize_member(w, Some(name), v, ctx, depth + 1)?;
                }
                w.end_object()
            }
            BslObject::Map(data) => {
                // ИЗМЕРЕНО: `ЗаписатьЗначениеJSON(Соответствие)` — исключение
                // на платформе, вопреки таблице сериализуемых типов из статьи
                // 16.2.1 (снято прогоном фикстуры `json-dates`); `ЗаписатьJSON`
                // с `Соответствие` при этом работает и измерен отдельно
                // (`JSON.SERIALIZE.NESTED`) — отличие именно в
                // `ЗаписатьЗначениеJSON`, а не в объектной технике сериализации.
                if ctx.single_value_mode {
                    return Err(RtError::TypeError {
                        expected: "значение без Соответствия (ЗаписатьЗначениеJSON его не сериализует)",
                        op: "ЗаписатьЗначениеJSON",
                    });
                }
                let entries: Vec<(BslValue, BslValue)> = {
                    let d = data.borrow();
                    (0..d.len()).filter_map(|i| d.entry_at(i)).collect()
                };
                w.begin_object()?;
                for (k, v) in &entries {
                    // Ключ соответствия может быть любым значением, а имя
                    // свойства JSON — только строкой. Числовой ключ
                    // печатается своим строковым видом, остальное —
                    // ошибка типа.
                    let name = match k {
                        BslValue::Str(s) => s.to_string(),
                        BslValue::Number(n) => n.to_canonical(),
                        _ => {
                            return Err(RtError::TypeError {
                                expected: "Строка или Число в ключе Соответствия",
                                op: "ЗаписатьJSON",
                            });
                        }
                    };
                    // Функция преобразования зовётся только на ЗНАЧЕНИИ —
                    // ИЗМЕРЕНО, что несериализуемый КЛЮЧ до неё не доходит
                    // («Недопустимый тип значения ключа элемента
                    // соответствия»), даже когда функция задана.
                    serialize_member(w, Some(&name), v, ctx, depth + 1)?;
                }
                w.end_object()
            }
            _ => Err(unsupported_value_error()),
        },
        _ => Err(unsupported_value_error()),
    }
}

// Ветвь `None` ниже — не остаток снятых JSON-опкодов, а обязательство
// перед сигнатурой: `execution_parts()` отдаёт вызывающего как `Option`,
// потому что `CallContext` строится двумя способами. Опкод
// `CallComponent` — единственный путь к этим функциям в дереве — всегда
// берёт `with_function_caller`, так что сегодня приходит `Some`; но
// контекст без вызывающего (`CallContext::new`) законен, и для него
// чтение и запись без обратного вызова обязаны работать, а не падать.
pub(crate) fn component_read_json(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    context.with_execution_parts(|parts| match parts.function_caller {
        Some(caller) => {
            let stdout = parts.stdout;
            let stderr = parts.stderr;
            let mut call = |name: &str, values: Vec<BslValue>| {
                caller(name, values, &mut *stdout, &mut *stderr)
            };
            read_json_builtin(arguments, parts.runtime_shapes, parts.zone, Some(&mut call))
        }
        None => read_json_builtin(arguments, parts.runtime_shapes, parts.zone, None),
    })
}

pub(crate) fn component_write_json(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    context.with_execution_parts(|parts| match parts.function_caller {
        Some(caller) => {
            let stdout = parts.stdout;
            let stderr = parts.stderr;
            let mut call = |name: &str, values: Vec<BslValue>| {
                caller(name, values, &mut *stdout, &mut *stderr)
            };
            write_json_builtin(arguments, parts.runtime_shapes, parts.zone, Some(&mut call))
        }
        None => write_json_builtin(arguments, parts.runtime_shapes, parts.zone, None),
    })
}

pub(crate) fn component_write_json_date(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    write_json_date(
        &arguments[0],
        &arguments[1],
        arguments.get(2).unwrap_or(&BslValue::Undefined),
        context.zone()?,
    )
}

pub(crate) fn component_read_json_date(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    read_json_date(&arguments[0], &arguments[1], context.zone()?)
}

pub(crate) fn component_write_json_value(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let (runtime, zone) = context.shapes_and_zone()?;
    write_json_value(&arguments[0], runtime, zone)
}

pub(crate) fn component_read_json_value(
    context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let (runtime, zone) = context.shapes_and_zone()?;
    read_json_value(&arguments[0], runtime, zone)
}

pub(crate) fn construct_reader(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_json_reader())
}

pub(crate) fn construct_writer(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_json_writer())
}

pub(crate) fn construct_writer_settings(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    new_json_writer_settings(arguments)
}

pub(crate) fn construct_serializer_settings(
    _context: &mut CallContext<'_>,
    _arguments: &[BslValue],
) -> RtResult<BslValue> {
    Ok(new_json_serializer_settings())
}

#[cfg(test)]
mod tests {
    use bsl_rt::BslNumber;
    use bsl_rt::RuntimeShapes;

    use super::*;

    fn num(s: &str) -> BslNumber {
        BslNumber::parse_canonical(s).unwrap()
    }

    /// Дат со смещением в этих тестах нет, поэтому зона любая — берётся
    /// UTC, чтобы результат не зависел от машины, на которой идёт прогон.
    static MACHINE_ZONE: bsl_rt::FixedTimeZone = bsl_rt::FixedTimeZone::UTC;

    /// Контекст сборки без функции восстановления — то, чем был
    /// `build_value` до появления колбэков.
    fn plain_build_ctx(rt: &mut RuntimeShapes) -> BuildCtx<'_, 'static> {
        BuildCtx {
            as_map: false,
            date_names: &[],
            date_format: None,
            restore: None,
            rt,
            zone: &MACHINE_ZONE,
            cache: JsonBuildCache::default(),
        }
    }

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

    // НЕ ИЗМЕРЕНО(JSON.MAX_DEPTH) — тесты фиксируют ВЫБРАННОЕ поведение:
    // перехватываемая ошибка вместо переполнения стека процесса; предел
    // платформы не замерен.
    #[test]
    fn too_deep_json_document_is_an_error_not_a_crash() {
        let depth = MAX_JSON_DEPTH + 100;
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut parser = JsonParser::new(&text);
        let first = parser.next_event().unwrap().unwrap();
        let e =
            build_value(first, &mut parser, None, &mut plain_build_ctx(&mut rt), 0).unwrap_err();
        assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
    }

    #[test]
    fn json_document_below_the_depth_limit_still_reads() {
        // 400 уровней — нижняя граница из замера: обязана работать.
        let depth = 400;
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut parser = JsonParser::new(&text);
        let first = parser.next_event().unwrap().unwrap();
        build_value(first, &mut parser, None, &mut plain_build_ctx(&mut rt), 0)
            .expect("глубина ниже предела обязана читаться");
    }

    #[test]
    fn cyclic_value_in_write_json_is_an_error_not_a_crash() {
        // Массив, содержащий сам себя, — бесконечная глубина: без предела
        // `serialize` рекурсировал бы до переполнения стека процесса.
        let arr = BslValue::new_array(Vec::new());
        arr.push_element(arr.clone()).unwrap();
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        let settings = JsonSerializerSettings::default();
        let e = serialize(&mut w, &arr, &mut plain_serialize_ctx(&settings, &rt), 0).unwrap_err();
        assert!(matches!(e, RtError::StackOverflow { .. }), "{e:?}");
    }

    #[test]
    fn json_key_cache_reuses_exact_spelling_and_preserves_case_insensitivity() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut cache = JsonKeyCache::new();

        let first = json_field_id("Поле", &mut rt, &mut cache).unwrap();
        let repeated = json_field_id("Поле", &mut rt, &mut cache).unwrap();
        let other_case = json_field_id("поле", &mut rt, &mut cache).unwrap();

        assert_eq!(first, repeated);
        assert_eq!(first, other_case);
        assert_eq!(cache.len(), 2, "кэш различает точные написания");
    }

    #[test]
    fn invalid_json_key_is_not_cached() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut cache = JsonKeyCache::new();

        let error = json_field_id("не имя", &mut rt, &mut cache).unwrap_err();

        assert!(matches!(error, RtError::Json(_)));
        assert!(cache.is_empty());
    }

    #[test]
    fn repeated_json_schema_reuses_shape_and_duplicate_overwrites_slot() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut cache = JsonBuildCache::default();

        let first = build_json_structure(
            vec!["Поле".into(), "поле".into(), "Второе".into()],
            vec![
                BslValue::Number(num("1")),
                BslValue::Number(num("2")),
                BslValue::Number(num("3")),
            ],
            &mut rt,
            &mut cache,
        )
        .unwrap();
        let (first_shape, first_slots) = match &first {
            BslValue::Object(value) => match &**value {
                BslObject::Structure(storage) => match &*storage.borrow() {
                    StructureStorage::Shaped { shape, slots } => (shape.clone(), slots.clone()),
                    StructureStorage::Dictionary { .. } => panic!("ожидалась форма"),
                },
                _ => panic!("ожидалась структура"),
            },
            _ => panic!("ожидался объект"),
        };
        assert_eq!(first_shape.names.len(), 2);
        assert_eq!(
            first_slots,
            vec![BslValue::Number(num("2")), BslValue::Number(num("3"))]
        );
        assert_eq!(cache.shapes.len(), 1);

        let second = build_json_structure(
            vec!["поле".into(), "ВТОРОЕ".into()],
            vec![BslValue::Number(num("4")), BslValue::Number(num("5"))],
            &mut rt,
            &mut cache,
        )
        .unwrap();
        let second_shape = match &second {
            BslValue::Object(value) => match &**value {
                BslObject::Structure(storage) => match &*storage.borrow() {
                    StructureStorage::Shaped { shape, .. } => shape.clone(),
                    StructureStorage::Dictionary { .. } => panic!("ожидалась форма"),
                },
                _ => panic!("ожидалась структура"),
            },
            _ => panic!("ожидался объект"),
        };
        assert!(Rc::ptr_eq(&first_shape, &second_shape));
    }

    #[test]
    fn oversized_json_schema_stays_dictionary_and_is_not_cached() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let mut cache = JsonBuildCache::default();
        let field_count = bsl_rt::MAX_SHAPE_TRANSITIONS as usize + 1;
        let keys = (0..field_count).map(|i| format!("f{i}")).collect();
        let values = (0..field_count)
            .map(|i| BslValue::Number(num(&i.to_string())))
            .collect();

        let object = build_json_structure(keys, values, &mut rt, &mut cache).unwrap();

        let BslValue::Object(value) = &object else {
            panic!("ожидался объект");
        };
        let BslObject::Structure(storage) = &**value else {
            panic!("ожидалась структура");
        };
        assert!(matches!(
            &*storage.borrow(),
            StructureStorage::Dictionary { .. }
        ));
        assert!(cache.shapes.is_empty());
    }

    // --- ПроверятьСтруктуру -------------------------------------------

    // --- ЗаписатьДатуJSON / ПрочитатьДатуJSON --------------------------

    // --- НастройкиСериализацииJSON --------------------------------------

    // --- ЗаписатьЗначениеJSON / ПрочитатьЗначениеJSON -------------------

    #[test]
    fn write_json_value_rejects_date_at_top_level_and_nested() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let e = write_json_value(
            &BslValue::Date(bsl_rt::BslDate::empty()),
            &rt,
            &MACHINE_ZONE,
        )
        .unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }));

        let mut rt2 = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let id = rt2.names.intern("д");
        let s = BslValue::new_structure(rt2.shapes.empty(), Vec::new());
        s.structure_insert(
            id,
            BslValue::Date(bsl_rt::BslDate::empty()),
            &mut rt2.shapes,
        )
        .unwrap();
        let e2 = write_json_value(&s, &rt2, &MACHINE_ZONE).unwrap_err();
        assert!(matches!(e2, RtError::TypeError { .. }));
    }

    /// ИЗМЕРЕНО: `ЗаписатьЗначениеJSON(Соответствие)` — исключение, тогда
    /// как `ЗаписатьJSON` с тем же `Соответствие` по-прежнему работает.
    #[test]
    fn write_json_value_rejects_a_map_while_write_json_still_accepts_it() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let map = BslValue::new_map();
        map.map_insert(
            BslValue::Str(bsl_rt::BslString::from_str("ключ")),
            BslValue::Number(num("1")),
        )
        .unwrap();

        let e = write_json_value(&map, &rt, &MACHINE_ZONE).unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }));

        let mut w = JsonWriter::to_string_target(settings_from(None).unwrap());
        let settings = JsonSerializerSettings::default();
        serialize(&mut w, &map, &mut plain_serialize_ctx(&settings, &rt), 0)
            .expect("ЗаписатьJSON по-прежнему сериализует Соответствие");
    }

    /// Без функции преобразования несериализуемый тип даёт обычную ошибку
    /// типа — `JSON.SERIALIZE.UNSUPPORTED_TYPE`, поведение не изменилось.
    #[test]
    fn write_json_without_a_convert_function_keeps_the_plain_type_error() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let writer = new_json_writer();
        set_string(writer.object_ref().unwrap().as_dyn(), &[]).unwrap();
        let table = BslValue::new_table();
        let e = write_json(
            &writer,
            &table,
            &JsonSerializerSettings::default(),
            None,
            &rt,
            &MACHINE_ZONE,
        )
        .unwrap_err();
        assert!(matches!(e, RtError::TypeError { .. }), "{e:?}");
    }

    /// ИЗМЕРЕНО: пустая строка в `ПрочитатьЗначениеJSON` — исключение, а
    /// не тихое `Неопределено`.
    #[test]
    fn read_json_value_rejects_an_empty_string() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let e = read_json_value(
            &BslValue::Str(bsl_rt::BslString::from_str("")),
            &mut rt,
            &MACHINE_ZONE,
        )
        .unwrap_err();
        assert!(matches!(e, RtError::Json(_)));
    }

    #[test]
    fn write_and_read_json_value_round_trip_scalars_and_structures() {
        let mut rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let s = write_json_value(&BslValue::Number(num("42")), &rt, &MACHINE_ZONE).unwrap();
        assert_eq!(s, BslValue::Str(bsl_rt::BslString::from_str("42")));

        let id = rt.names.intern("а");
        let structure = BslValue::new_structure(rt.shapes.empty(), Vec::new());
        structure
            .structure_insert(id, BslValue::Number(num("1")), &mut rt.shapes)
            .unwrap();
        let text = write_json_value(&structure, &rt, &MACHINE_ZONE).unwrap();
        let BslValue::Str(text) = text else {
            panic!("ожидалась строка")
        };
        let back = read_json_value(&BslValue::Str(text), &mut rt, &MACHINE_ZONE).unwrap();
        // ИЗМЕРЕНО (JSON.VALUE.READ_KIND): «Структура».
        assert_eq!(back.type_name(), "Структура");
    }
}
