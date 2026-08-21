//! `СериализаторXDTO`.

use super::*;

// --- СериализаторXDTO ----------------------------------------------------

/// Пространство имён базовых типов данных 1С. Из него сериализатору здесь
/// нужен ровно один элемент — `Null`; `UUID`, `Type` и коллекции (`Array`,
/// `Structure`, ...) в это пространство тоже попадают, но не поддержаны
/// (см. заголовок модуля).
pub(crate) const V8_CORE_NS: &str = "http://v8.1c.ru/8.1/data/core";

/// `СериализаторXDTO` над готовой моделью типов.
pub fn serializer_value(model: Rc<XdtoModel>) -> BslValue {
    shell_value(XdtoRepr::Serializer(model))
}

/// `Новый СериализаторXDTO(ФабрикаXDTO)`.
///
/// Фабрика ОБЯЗАТЕЛЬНА и других источников у конструктора нет: измерено,
/// что 8.3.27 отвергает и `Новый СериализаторXDTO` без аргумента
/// («Конструктор не найден»), и два аргумента, и строку, и число, и тип
/// XDTO — принимается только сама фабрика. Английское написание `Новый
/// XDTOSerializer(Фаб)` тоже измерено.
///
/// # Errors
///
/// [`RtError::Xdto`], если аргумент — не `ФабрикаXDTO`.
pub fn serializer_of_factory(factory: &BslValue) -> RtResult<BslValue> {
    match repr_of(factory) {
        Some(XdtoRepr::Factory(model)) => Ok(serializer_value(model.clone())),
        _ => Err(bad_serializer_factory(factory)),
    }
}

pub(crate) fn bad_serializer_factory(arg: &BslValue) -> RtError {
    RtError::Xdto(format!(
        "«Новый СериализаторXDTO» строится по «ФабрикаXDTO», а не по «{}»",
        arg.type_name()
    ))
}

/// Модель сериализатора-получателя.
pub(crate) fn serializer_model<'a>(
    obj: &'a dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'a Rc<XdtoModel>> {
    match repr_of_object(obj) {
        Some(XdtoRepr::Serializer(model)) => Ok(model),
        _ => Err(not_applicable(obj, method)),
    }
}

/// Члены сериализатора, до которых эта задача не дошла, — с РАЗНЫМИ
/// причинами.
///
/// Все три измерены существующими, но упираются они в разное.
/// `XMLТип(Тип)` и `XMLТипЗнч(Значение)` отдают `ТипДанныхXML` (у строки —
/// `string` в пространстве XML Schema, у числа — `decimal`, у даты —
/// `dateTime`), то есть реализовать их — значит сперва завести этот тип, а
/// это отдельная работа. `ВозможностьЧтенияXML(Чт)` никакого нового типа не
/// требует: измерено, что он отдаёт БУЛЕВО — «Да» на `<decimal>`
/// пространства XML Schema, «Нет» на чужое имя и на читателя, ещё не
/// сделавшего `Прочитать()`. Он не поддержан по объёму задачи: трёх точек
/// мало, чтобы знать, когда именно платформа отвечает «Да». Молча отвечать
/// чем попало нельзя ни там, ни там, поэтому вызов даёт перехватываемую
/// ошибку, а не тишину, — и она называет ту причину, которая есть на самом
/// деле.
pub fn serializer_unsupported(obj: &dyn ObjectProtocol, method: &'static str) -> RtError {
    if !matches!(repr_of_object(obj), Some(XdtoRepr::Serializer(_))) {
        return not_applicable(obj, method);
    }
    // Список написаний закрыт: сюда приходят ровно три литерала из
    // `call_builtin_method`. Незнакомое имя не получает чужую причину —
    // лучше отказ без объяснения, чем отказ с неверным.
    let reason = match method {
        "XMLТип" | "XMLТипЗнч" => ": он отдаёт «ТипДанныхXML», а этого типа здесь нет",
        "ВозможностьЧтенияXML" => {
            ": по замеру он отдаёт булево, но в этой задаче не реализован"
        }
        _ => "",
    };
    RtError::Xdto(format!(
        "«СериализаторXDTO.{method}» не поддерживается{reason}"
    ))
}

/// Элемент, которым сериализатор пишет значение BSL: имя, пространство
/// имён, признак `xsi:nil` и текст.
///
/// Таблица снята поимённо с 8.3.27 через фабрику из ФАЙЛА XSD, то есть без
/// каких бы то ни было типов конфигурации: число -> `<decimal>` в
/// пространстве XML Schema, строка -> `<string>`, дата -> `<dateTime>`,
/// булево -> `<boolean>`, двоичные данные -> `<base64Binary>`,
/// `Неопределено` -> `<Undefined xsi:nil="true"/>` с ПУСТЫМ пространством
/// имён, `Null` -> `<Null>` в пространстве базовых типов 1С.
///
/// # Errors
///
/// [`RtError::Xdto`], если значению соответствия нет.
pub(crate) fn serialized_of(
    model: &Rc<XdtoModel>,
    value: &BslValue,
) -> RtResult<(String, String, bool, String)> {
    match value {
        // Пространство имён у этого элемента ПУСТОЕ — и это не описка
        // замера: `Тип("Неопределено")` та же платформа пишет как
        // `{http://v8.1c.ru/8.2/data/types}Undefined`, то есть сама себе
        // не соответствует. Воспроизводится измеренное.
        BslValue::Undefined => Ok(("Undefined".to_string(), String::new(), true, String::new())),
        BslValue::Null => Ok((
            "Null".to_string(),
            V8_CORE_NS.to_string(),
            false,
            String::new(),
        )),
        _ => {
            let index = builtin_index_for(model, value).ok_or_else(|| unsupported_value(value))?;
            let data = model.type_at(index)?;
            Ok((
                data.name.clone(),
                data.ns.clone(),
                false,
                lexical_for_write(model, index, value)?,
            ))
        }
    }
}

/// Отказ на значении, которого сериализатор над фабрикой из XSD не пишет.
///
/// Коллекции названы отдельной строкой потому, что причина у них другая и
/// она измерена с обеих сторон: ГЛОБАЛЬНЫЙ сериализатор платформы пишет
/// `Массив` как `<Array xmlns="http://v8.1c.ru/8.1/data/core">` со
/// вложенными `<Value xsi:type="xs:decimal">`, а тот же самый `Массив`
/// через фабрику из нашего XSD 8.3.27 уже отвергает — «Несоответствие
/// типов (параметр номер '2') (Ошибка отображения типов)». Пакет
/// `http://v8.1c.ru/8.1/data/core` приходит из метаданных конфигурации,
/// которых здесь нет и не будет (см. `docs/std-library-plan.md`).
pub(crate) fn unsupported_value(value: &BslValue) -> RtError {
    let collection = matches!(
        value,
        BslValue::Object(o) if matches!(
            &**o,
            BslObject::Array(..)
                | BslObject::Structure(..)
                | BslObject::Map(..)
                | BslObject::ValueTable(..)
        )
    );
    if collection {
        return RtError::Xdto(format!(
            "«{}» не поддерживается: коллекции сериализуются типами пакета \
             «{V8_CORE_NS}», а он берётся из метаданных конфигурации, которых здесь нет",
            value.type_name()
        ));
    }
    RtError::Xdto(format!(
        "«СериализаторXDTO.ЗаписатьXML» не умеет писать «{}»: поддержаны значения \
         встроенных типов XML Schema, «Неопределено» и «Null»",
        value.type_name()
    ))
}

/// Записать элемент сериализатора: имя, объявления, `xsi:nil` и текст.
///
/// Область объявлений здесь СВОЯ, а не та, что у `write_node`, и правило
/// умолчательного объявления другое. Фабрика объявление опускает, когда
/// URI уже привязан (а вне всяких объявлений привязан пустой URI), —
/// сериализатор же пишет `xmlns` ВСЕГДА, включая `xmlns=""` (измерено:
/// `<Undefined xmlns="" …>` и `<мой xmlns="" …>` при заданном имени).
/// Остальное совпадает: `xs` не объявляется, если пространство XML Schema
/// и так стало умолчательным, `xsi` объявляется всегда.
///
/// Имя пишется КАК ЗАДАНО, включая двоеточие: измерено, что
/// `ЗаписатьXML(Зп, 42, "t:мой")` даёт `<t:мой xmlns="">` с необъявленным
/// префиксом — там, где та же проба у фабрики отвергается.
///
/// Вырожденный случай — заданный вручную URI, РАВНЫЙ пространству
/// экземпляров XML Schema, — тоже измерен, и рассуждение тут не помогло бы:
/// `xmlns:xsi` платформа не объявляет вовсе (`<мой xmlns="…instance"
/// xmlns:xs="…">42</мой>`), а когда пометку `nil` писать всё-таки надо,
/// заводит для того же URI ПОРОЖДЁННЫЙ префикс и метит атрибут им — `<мой
/// xmlns="…instance" xmlns:d1p1="…instance" xmlns:xs="…" d1p1:nil="true"/>`.
/// Имя префикса подчиняется правилу записи фабрики, `d<глубина>p<номер>`, и
/// глубина считается ПО ДОКУМЕНТУ: та же проба внутри открытого элемента
/// даёт `d2p1`. Причина у обхода общая с фабрикой — умолчательное
/// объявление на атрибуты не распространяется.
///
/// # Errors
///
/// [`RtError::Xml`], если писатель уже закрыт.
pub(crate) fn write_serialized(
    w: &mut crate::core::XmlWriter,
    name: &str,
    uri: &str,
    nil: bool,
    text: &str,
) -> RtResult<()> {
    // Префикс порождается до начала тега — как и у `write_node`, где
    // глубина берётся у писателя ещё не открытого элемента.
    let generated = (uri == XSI_NS && nil).then(|| format!("d{}p1", w.depth() + 1));
    w.write_start_element(name)?;
    w.write_attribute("xmlns", uri)?;
    if let Some(prefix) = &generated {
        w.write_attribute(&format!("xmlns:{prefix}"), XSI_NS)?;
    }
    if uri != XSD_NS {
        w.write_attribute("xmlns:xs", XSD_NS)?;
    }
    if uri != XSI_NS {
        w.write_attribute("xmlns:xsi", XSI_NS)?;
    }
    if nil {
        let prefix = generated.as_deref().unwrap_or("xsi");
        w.write_attribute(&format!("{prefix}:nil"), "true")?;
    }
    // Пустая лексическая форма даёт схлопнутый `<имя/>` — то же правило,
    // что у записи фабрики.
    if !text.is_empty() {
        w.write_text(text)?;
    }
    w.write_end_element()
}

/// `СериализаторXDTO.ЗаписатьXML(ЗаписьXML, Значение[, Имя[, УРИ]])`.
///
/// Арность измерена: одного аргумента платформа не берёт, пятый отвергает.
/// Имя и URI ведут себя не так, как у фабрики, и это тоже измерено. И
/// пустое имя, и `Неопределено` вместо него значат «имя по умолчанию»
/// (`<decimal …>`, а не ошибка), причём пустое имя отбрасывает и заданный
/// URI (`("", "urn:иное")` -> всё то же умолчание). А вот заданное имя,
/// наоборот, сбрасывает пространство имён в ПУСТОЕ, если URI не назван
/// (`ЗаписатьXML(Зп, 42, "мой")` -> `<мой xmlns="">`), и `Неопределено`
/// вместо URI при этом равно неназванному (`("мой", Неопределено)` ->
/// `<мой xmlns="">`).
///
/// Одно сочетание из этой сетки платформа отвергает, и правило тут не
/// продолжается, а ломается: имя `Неопределено` ВМЕСТЕ с URI, заданным
/// строкой, — «Несоответствие типов (параметр номер '4')», хотя пустое имя
/// с тем же URI проходит, а `Неопределено` в обоих аргументах сразу снова
/// даёт умолчание. То есть `Неопределено` в имени — это не «имя не
/// задано», а значение, под которое четвёртый параметр не подходит по
/// типу; воспроизводится измеренное, а не логика.
///
/// Различить эти случаи можно только ДО того, как имя и URI станут
/// строками: `Неопределено` и пустая строка сходятся в `optional_text`,
/// поэтому проверка стоит раньше него и смотрит на сами значения.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не сериализатор;
/// [`RtError::Xdto`], если аргументы не те или значение не сериализуется;
/// [`RtError::Xml`], если писатель уже закрыт.
pub fn serializer_write_xml(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let model = serializer_model(obj, "ЗаписатьXML")?;
    let (writer, value, name, uri) = match args {
        [writer, value] => (writer, value, None, None),
        [writer, value, name] => (writer, value, Some(name), None),
        [writer, value, name, uri] => (writer, value, Some(name), Some(uri)),
        _ => return Err(not_applicable(obj, "ЗаписатьXML")),
    };
    if !crate::xml::is_xml_writer(writer) {
        return Err(RtError::Xdto("ЗаписатьXML пишет в «ЗаписьXML»".to_string()));
    }
    if matches!(name, Some(BslValue::Undefined)) && matches!(uri, Some(BslValue::Str(_))) {
        return Err(RtError::Xdto(
            "«СериализаторXDTO.ЗаписатьXML»: имя «Неопределено» вместе с заданным URI \
             платформа отвергает — «Несоответствие типов (параметр номер '4')»"
                .to_string(),
        ));
    }
    let given_name = optional_text(name, "ЗаписатьXML")?.filter(|n| !n.is_empty());
    let given_uri = optional_text(uri, "ЗаписатьXML")?;
    let (default_name, default_uri, nil, text) = serialized_of(model, value)?;
    let (name, uri) = match given_name {
        Some(name) => (name, given_uri.unwrap_or_default()),
        None => (default_name, default_uri),
    };
    crate::xml::with_writer(crate::xml::arg_object(writer)?, |w| {
        write_serialized(w, &name, &uri, nil, &text)
    })
}

/// Встроенный тип XML Schema, лексической формой которого сериализатор
/// читает элемент, когда тип назван вторым аргументом ЗНАЧЕНИЕМ `Тип`.
///
/// Таблица обратна [`builtin_index_for`] и измерена с той же стороны:
/// `ПрочитатьXML(Чт, Тип("Число"))` разбирает текст как `xs:decimal`
/// (`<int>42</int>` тоже даёт 42 — имя элемента при заданном типе не
/// смотрится вовсе), `Тип("Дата")` — как `xs:dateTime`, `Тип("Булево")` —
/// как `xs:boolean`, `Тип("ДвоичныеДанные")` — как `xs:base64Binary`. Для
/// `Тип("Массив")`, `Тип("УникальныйИдентификатор")` и
/// `Тип("Неопределено")` платформа отвечает «Отсутствует отображение для
/// типа», то есть отображения нет и у неё.
pub(crate) fn builtin_index_of_type(model: &Rc<XdtoModel>, id: TypeId) -> Option<usize> {
    let name = match id {
        TypeId::String => "string",
        TypeId::Number => "decimal",
        TypeId::Boolean => "boolean",
        TypeId::Date => "dateTime",
        TypeId::BinaryData => "base64Binary",
        _ => return None,
    };
    model.find(XSD_NS, name)
}

/// `СериализаторXDTO.ПрочитатьXML(ЧтениеXML[, Тип])`.
///
/// Второй аргумент — ЗНАЧЕНИЕ `Тип` (тип BSL), а не тип XDTO: измерено,
/// что `Тип("Число")` принимается, а третий аргумент платформа отвергает
/// («Слишком много фактических параметров»).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не сериализатор;
/// [`RtError::Xdto`], если аргументы не те, отображения для типа нет либо
/// текст не разбирается; [`RtError::Xml`] на битой разметке.
pub fn serializer_read_xml(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let model = serializer_model(obj, "ПрочитатьXML")?.clone();
    let (reader, type_arg) = match args {
        [reader] => (reader, None),
        [reader, ty] => (reader, Some(ty)),
        _ => return Err(not_applicable(obj, "ПрочитатьXML")),
    };
    if !crate::xml::is_xml_reader(reader) {
        return Err(RtError::Xdto(
            "ПрочитатьXML читает из «ЧтениеXML»".to_string(),
        ));
    }
    let want = match type_arg {
        None | Some(BslValue::Undefined) => None,
        Some(BslValue::Type(id)) => Some(*id),
        Some(other) => {
            return Err(RtError::Xdto(format!(
                "второй аргумент «СериализаторXDTO.ПрочитатьXML» — значение «Тип», \
                 а не «{}»",
                other.type_name()
            )));
        }
    };
    crate::xml::with_reader(crate::xml::arg_object(reader)?, |state| {
        read_one(state, |parser, head| {
            serializer_read_element(parser, &model, head, want)
        })
    })
}

/// Разбор одного элемента в значение BSL.
///
/// Порядок решений измерен построчно:
///
/// * `xsi:nil="true"` перевешивает всё и даёт `Неопределено` — и у
///   известного имени (`<decimal xsi:nil="true"/>`), и у чужого;
/// * заданный вторым аргументом тип BSL отменяет и имя элемента, и
///   `xsi:type`;
/// * иначе тип ищется по `xsi:type` (он у сериализатора действует ПРЯМО
///   на читаемом элементе — в отличие от фабрики, где пометка на нём не
///   значит ничего), а неизвестное имя типа платформа молча игнорирует;
/// * иначе — по расширенному имени самого элемента;
/// * если тип не нашёлся, элемент читается СТРОКОЙ: так читаются и
///   `<чужой>привет</чужой>`, и `<decimal>42</decimal>` без пространства
///   имён, и `{http://v8.1c.ru/8.1/data/core}UUID`, и `Null`, и `Type`,
///   то есть круг у них лексикой не замыкается (измерено);
/// * если тип нашёлся, но он ОБЪЕКТНЫЙ, — ошибка: измерено, что
///   `<RootType xmlns="urn:test">` даёт «Отсутствует отображение для типа
///   '{urn:test}RootType'», а не строку.
///
/// ФАСЕТЫ здесь не проверяются, и это тоже измерено: чтение
/// сериализатором `<Len xmlns="urn:v">а</Len>` при `minLength="2"` у
/// `Len` отдаёт «а» без ошибки, тогда как то же чтение ФАБРИКОЙ — ошибка.
/// Поэтому разбор идёт через непроверяющий `value_from_lexical`.
pub(crate) fn serializer_read_element(
    parser: &mut crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    head: &ElementHead,
    want: Option<TypeId>,
) -> RtResult<BslValue> {
    if reads_as_nil(parser, head) {
        skip_element(parser)?;
        return Ok(BslValue::Undefined);
    }
    if let Some(id) = want {
        let Some(index) = builtin_index_of_type(model, id) else {
            return Err(RtError::Xdto(format!(
                "отсутствует отображение для типа «{}»",
                id.name()
            )));
        };
        let text = read_text_only(parser, head)?;
        return value_from_lexical(model, index, &text);
    }
    let Some(index) = serializer_type_of(parser, model, head) else {
        let text = read_text_only(parser, head)?;
        return Ok(BslValue::Str(BslString::from_str(&text)));
    };
    let data = model.type_at(index)?;
    if !data.is_value() {
        return Err(RtError::Xdto(format!(
            "отсутствует отображение для типа «{}»",
            type_display(data)
        )));
    }
    let text = read_text_only(parser, head)?;
    value_from_lexical(model, index, &text)
}

/// Тип элемента: сначала `xsi:type`, потом собственное имя.
pub(crate) fn serializer_type_of(
    parser: &crate::core::XmlParser,
    model: &Rc<XdtoModel>,
    head: &ElementHead,
) -> Option<usize> {
    let marked = head.attrs.iter().find(|a| {
        attribute_uri(parser, &a.name) == XSI_NS && crate::core::local_of(&a.name) == "type"
    });
    if let Some(qname) = marked {
        let uri = parser.namespace_of(crate::core::prefix_of(&qname.value));
        if let Some(index) = model.find(&uri, crate::core::local_of(&qname.value)) {
            return Some(index);
        }
        // Неизвестное имя типа платформа ИГНОРИРУЕТ и читает дальше как
        // ни в чём не бывало (измерено: `xsi:type="xs:чепуха"` на
        // элементе `<мой>` дало строку «42», а не ошибку).
    }
    model.find(&head.uri, crate::core::local_of(&head.name))
}
