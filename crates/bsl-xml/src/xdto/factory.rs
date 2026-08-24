//! `ФабрикаXDTO`: типы, создание значений и экземпляров.

use super::*;

// --- фабрика -------------------------------------------------------------

/// `ФабрикаXDTO` над готовой моделью типов.
///
/// Фабрика — это СНИМОК: `Новый ФабрикаXDTO(Наб)` строит модель на месте, и
/// схема, добавленная в тот же набор позже, ей уже не видна (измерено:
/// `Ф = Новый ФабрикаXDTO(Н); Н.Добавить(Схема); Ф.Тип(...)` ->
/// `Неопределено` — промах поиска, а не ошибка). Отсюда и `Rc<XdtoModel>`
/// вместо ссылки на набор.
pub fn factory_value(model: Rc<XdtoModel>) -> BslValue {
    shell_value(XdtoRepr::Factory(model))
}

/// `ОбъектXDTO` — свежий экземпляр типа объекта: хранилище пусто, владельца
/// нет.
pub(crate) fn object_value(model: &Rc<XdtoModel>, index: usize) -> BslValue {
    instance_value(&Rc::new(XdtoObjectData {
        model: model.clone(),
        type_index: index,
        owner: RefCell::new(Weak::new()),
        entries: RefCell::new(Vec::new()),
    }))
}

/// Значение вокруг готового хранилища — им же отдаётся `Владелец()`.
pub(crate) fn instance_value(data: &Rc<XdtoObjectData>) -> BslValue {
    shell_value(XdtoRepr::Object(data.clone()))
}

/// `СоздатьФабрикуXDTO(Путь)` — фабрика по файлу XSD.
///
/// Источник у этой функции ровно один — путь к файлу: схему, набор схем,
/// текст схемы, число и вызов без аргументов платформа отвергает
/// (измерено все пять). Схема разбирается тем же путём, что и
/// `ПостроительСхемXML.СоздатьСхемуXML`, — второго разборщика в проекте
/// нет.
///
/// # Errors
///
/// [`RtError::Xdto`], если аргумент не строка или файла нет;
/// [`RtError::Xsd`] и [`RtError::Xml`], если содержимое файла — не схема.
pub fn factory_of_file(
    args: &[BslValue],
    zone: Rc<dyn bsl_rt::TimeZone>,
    files: &dyn bsl_rt::FileSystem,
) -> RtResult<BslValue> {
    let [BslValue::Str(path)] = args else {
        return Err(RtError::Xdto(
            "СоздатьФабрикуXDTO берёт один аргумент — путь к файлу XSD".to_string(),
        ));
    };
    let path = path.to_string();
    // Схема читается файловой системой СЕССИИ (ABI-G). Фабрика после
    // построения к путям не обращается, поэтому берёт ФС ссылкой на время
    // конструктора (BORROW), а не запоминает её.
    let bytes = files
        .read(&path)
        .map_err(|e| RtError::Xdto(format!("файл схемы «{path}» не прочитан: {e}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| RtError::Xdto(format!("файл схемы «{path}» не в UTF-8: {e}")))?;
    // Сигнатуру UTF-8 разборщик видит как символ перед `<` — снимаем её
    // так же, как `ЧтениеXML.ОткрытьФайл`.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let schema = crate::xsd::schema_of_text(text)?;
    Ok(factory_value(model_of_schemas(&[schema], zone)?))
}

/// `Новый ФабрикаXDTO([НаборСхемXML])` — фабрика по набору схем.
///
/// Аргумент необязателен: без него получается фабрика с одними встроенными
/// типами XML Schema (измерено). Пустой набор даёт ровно её же. Всё
/// остальное — путь, схема, текст, массив — платформа отвергает
/// (измерено), и здесь так же.
///
/// # Errors
///
/// [`RtError::Xdto`], если аргумент не `НаборСхемXML`; ошибки построения
/// модели — из `model_of_schemas`.
pub fn factory_of_schema_set(arg: &BslValue, zone: Rc<dyn bsl_rt::TimeZone>) -> RtResult<BslValue> {
    let schemas: Vec<Rc<XsSchemaData>> = match arg {
        BslValue::Undefined => Vec::new(),
        _ => match arg
            .object_ref()
            .and_then(|object| object.downcast_ref::<crate::xsd::SchemaSetObject>())
        {
            Some(set) => set.schemas.borrow().clone(),
            None => return Err(bad_factory_source()),
        },
    };
    Ok(factory_value(model_of_schemas(&schemas, zone)?))
}

pub(crate) fn bad_factory_source() -> RtError {
    RtError::Xdto(
        "Новый ФабрикаXDTO берёт либо ничего, либо НаборСхемXML; \
         фабрику по файлу XSD строит СоздатьФабрикуXDTO"
            .to_string(),
    )
}

/// Фабрика ли это — вне тестов диспетчер ветвится по представлению
/// напрямую, поэтому lib-цель проверку не видит.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_factory(v: &BslValue) -> bool {
    matches!(repr_of(v), Some(XdtoRepr::Factory(_)))
}

pub(crate) fn not_applicable(obj: &dyn ObjectProtocol, method: &'static str) -> RtError {
    RtError::MethodNotApplicable {
        method,
        receiver: obj.type_descriptor().name,
    }
}

/// Модель фабрики-получателя.
pub(crate) fn factory_model<'a>(
    obj: &'a dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'a Rc<XdtoModel>> {
    match repr_of_object(obj) {
        Some(XdtoRepr::Factory(model)) => Ok(model),
        _ => Err(not_applicable(obj, method)),
    }
}

/// `ФабрикаXDTO.Тип(URI, Имя)` и `ФабрикаXDTO.Тип(РасширенноеИмяXML)`.
///
/// Неизвестное имя — `Неопределено`, а не ошибка (измерено, как и то, что
/// объявление глобального ЭЛЕМЕНТА типом не является). Одна строка вместо
/// пары, три аргумента, числа вместо имён — ошибка (измерено все четыре
/// пробы).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика либо
/// аргументы не той формы.
pub fn factory_type(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let model = factory_model(obj, "Тип")?;
    let found = match args {
        [BslValue::Str(uri), BslValue::Str(name)] => {
            model.find(&uri.to_string(), &name.to_string())
        }
        [name_arg @ BslValue::Object(_)] => match name_arg
            .object_ref()
            .and_then(|object| object.downcast_ref::<crate::xsd::ExpandedNameObject>())
        {
            Some(expanded) => model.find(&expanded.name.uri, &expanded.name.local),
            None => return Err(not_applicable(obj, "Тип")),
        },
        _ => return Err(not_applicable(obj, "Тип")),
    };
    Ok(match found {
        Some(index) => type_value(model, index),
        None => BslValue::Undefined,
    })
}

/// `ФабрикаXDTO.Создать(Тип[, Лексика])`.
///
/// Смысл вызова решает вид типа, и оба измерены. У типа ЗНАЧЕНИЯ вызов без
/// лексической формы отдаёт `Неопределено`, а с формой — `ЗначениеXDTO`,
/// разобранное тем же путём, что и `ЗначениеПоУмолчанию` свойства. У типа
/// ОБЪЕКТА лексической формы быть не должно (`Создать(ТипОбъекта, "аб")` —
/// ошибка), а результат — `ОбъектXDTO`; абстрактный тип платформа
/// инстанцировать отказывается.
///
/// ФАСЕТЫ ЗДЕСЬ ПРОВЕРЯЮТСЯ — платформа проверяет их именно тут (измерено:
/// `Создать` от типа с перечислением `red|green` с «синий», от ограничения
/// длины с «а», от ограничения диапазона с «-1» и от `xs:int` с
/// «3000000000» — ошибка).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не фабрика, первый
/// аргумент не тип XDTO либо аргументов не то количество;
/// [`RtError::Xdto`], если лексическая форма не разбирается в этом типе,
/// нарушает его фасет или тип объекта абстрактный.
pub fn factory_create(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    factory_model(obj, "Создать")?;
    let [first, rest @ ..] = args else {
        return Err(not_applicable(obj, "Создать"));
    };
    // Третий аргумент платформа принимает (измерено: `Создать(Тип, "42",
    // 1)` отдаёт `ЗначениеXDTO`), а четвёртый уже нет. Что он значит, не
    // измерено, поэтому здесь он принимается и ни на что не влияет:
    // додумывать ему смысл хуже, чем отвергать программу, которая на
    // платформе работает.
    if rest.len() > 2 {
        return Err(not_applicable(obj, "Создать"));
    }
    let Some(XdtoRepr::Type(model, index)) = repr_of(first) else {
        return Err(not_applicable(obj, "Создать"));
    };
    // Модель берётся у САМОГО типа, а не у фабрики-получателя: тип и так
    // несёт свою модель, и чужой тип строил бы значение по своей. Что
    // платформа делает с типом из другой фабрики, не измерено.
    let data = model.type_at(*index)?;
    if !data.is_value() {
        if !rest.is_empty() {
            return Err(not_applicable(obj, "Создать"));
        }
        if data.is_abstract {
            return Err(RtError::Xdto(format!(
                "абстрактный тип «{}» экземпляров не имеет",
                type_display(data)
            )));
        }
        return Ok(object_value(model, *index));
    }
    let Some(lexical) = rest.first() else {
        // Тип значения без лексической формы — `Неопределено` (измерено).
        return Ok(BslValue::Undefined);
    };
    let BslValue::Str(text) = lexical else {
        return Err(RtError::Xdto(
            "лексическая форма значения XDTO — это строка".to_string(),
        ));
    };
    let text = text.to_string();
    Ok(data_value(
        model,
        &Rc::new(XdtoValueData {
            value: value_from_lexical_checked(model, *index, &text)?,
            lexical: text,
            type_index: *index,
        }),
    ))
}

/// `ОбъектXDTO.Тип()` — свой тип XDTO. Именно МЕТОД: обращение к `Тип` как
/// к свойству платформа отвергает, а `Тип()` отдаёт тот же тип, что и
/// `Фабрика.Тип(URI, Имя)` (измерено обе стороны, включая равенство).
/// Аргументов у него нет — `Тип(1)` платформа не берёт.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не `ОбъектXDTO` либо
/// вызов с аргументами.
pub fn object_type(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    if !args.is_empty() {
        return Err(not_applicable(obj, "Тип"));
    }
    // `ЗначениеXDTO` отвечает на `Тип()` так же — типом, по которому
    // разобрана его лексическая форма (измерено: `Создать(xs:int, "42")
    // .Тип()` -> `{...}int`).
    if let Some(XdtoRepr::Value(model, data)) = repr_of_object(obj) {
        model.type_at(data.type_index)?;
        return Ok(type_value(model, data.type_index));
    }
    let data = instance_of(obj, "Тип")?;
    // Номер проверяется до построения значения: испорченная модель обязана
    // отвечать ошибкой, а не типом, которого нет.
    data.type_data()?;
    Ok(type_value(&data.model, data.type_index))
}

/// Член `ВидФасетаXDTO` по виду фасета лексической модели XSD.
pub(crate) fn facet_kind_value(kind: FacetKind) -> EnumValue {
    match kind {
        FacetKind::Length => EnumValue::XdtoFacetLength,
        FacetKind::MinLength => EnumValue::XdtoFacetMinLength,
        FacetKind::MaxLength => EnumValue::XdtoFacetMaxLength,
        FacetKind::Pattern => EnumValue::XdtoFacetPattern,
        FacetKind::Enumeration => EnumValue::XdtoFacetEnumeration,
        FacetKind::WhiteSpace => EnumValue::XdtoFacetWhiteSpace,
        FacetKind::TotalDigits => EnumValue::XdtoFacetTotalDigits,
        FacetKind::FractionDigits => EnumValue::XdtoFacetFractionDigits,
        FacetKind::MinInclusive => EnumValue::XdtoFacetMinInclusive,
        FacetKind::MaxInclusive => EnumValue::XdtoFacetMaxInclusive,
        FacetKind::MinExclusive => EnumValue::XdtoFacetMinExclusive,
        FacetKind::MaxExclusive => EnumValue::XdtoFacetMaxExclusive,
    }
}

/// Свойство значения модели типов.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого члена у этого вида значения
/// нет; [`RtError::Xdto`], если модель ссылается на несуществующий узел.
pub fn get_property(obj: &dyn ObjectProtocol, name: &str) -> RtResult<BslValue> {
    let unknown = || RtError::UnknownColumn(name.to_string());
    // Сравнение — через `fold`, а не `eq_ignore_ascii_case`: имена членов
    // здесь РУССКИЕ, а ASCII-свёртка кириллицу не трогает. Имя приходит в
    // том написании, в каком его первым увидел интерн полей, и `Значение`
    // в скрипте, где раньше встретилось `значение`, доходит сюда строчным.
    let is =
        |ru: &str, en: &str| bsl_rt::fold::folded_eq(name, ru) || bsl_rt::fold::folded_eq(name, en);
    let Some(o) = repr_of_object(obj) else {
        return Err(unknown());
    };
    match o {
        XdtoRepr::Type(model, i) => {
            let data = model.type_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("БазовыйТип", "BaseType") {
                return Ok(match data.base {
                    Some(b) => type_value(model, b),
                    None => BslValue::Undefined,
                });
            }
            if data.is_value() {
                if is("Фасеты", "Facets") {
                    // У типа БЕЗ фасетов это `Неопределено`, а не пустая
                    // коллекция (измерено на `xs:date`).
                    return Ok(if data.facets.is_empty() {
                        BslValue::Undefined
                    } else {
                        shell_value(XdtoRepr::Facets(model.clone(), *i))
                    });
                }
                return Err(unknown());
            }
            if is("Свойства", "Properties") {
                return Ok(shell_value(XdtoRepr::Properties(model.clone(), *i)));
            }
            if is("Открытый", "Open") {
                return Ok(BslValue::Boolean(data.open));
            }
            if is("Абстрактный", "Abstract") {
                return Ok(BslValue::Boolean(data.is_abstract));
            }
            if is("Упорядоченный", "Ordered") {
                return Ok(BslValue::Boolean(data.ordered));
            }
            if is("Последовательный", "Sequenced") {
                return Ok(BslValue::Boolean(data.sequenced()));
            }
            if is("Смешанный", "Mixed") {
                return Ok(BslValue::Boolean(data.mixed));
            }
            Err(unknown())
        }
        XdtoRepr::Property(model, i) => {
            let data = model.property_at(*i)?;
            if is("Имя", "Name") {
                return Ok(str_value(&data.name));
            }
            if is("URIПространстваИмен", "NamespaceURI") {
                return Ok(str_value(&data.ns));
            }
            if is("Тип", "Type") {
                return Ok(type_value(model, data.type_index));
            }
            if is("НижняяГраница", "LowerBound") {
                return Ok(bound_value(data.lower));
            }
            if is("ВерхняяГраница", "UpperBound") {
                return Ok(bound_value(data.upper));
            }
            if is("Форма", "Form") {
                return Ok(BslValue::Enum(data.form));
            }
            if is("ЗначениеПоУмолчанию", "DefaultValue") {
                return Ok(match &data.default {
                    Some(v) => data_value(model, v),
                    None => BslValue::Undefined,
                });
            }
            Err(unknown())
        }
        XdtoRepr::Facet(model, type_index, facet_index) => {
            let data = model.type_at(*type_index)?;
            let (kind, lexical) = data
                .facets
                .get(*facet_index)
                .ok_or_else(|| broken("фасет"))?;
            // Английское имя `Вид` — `Type`, а не `Kind`: `Kind` платформа
            // отвергает (измерено обе пробы).
            if is("Вид", "Type") {
                return Ok(BslValue::Enum(facet_kind_value(*kind)));
            }
            // `Значение` фасета — ВСЕГДА строка, даже у числовых
            // (измерено).
            if is("Значение", "Value") {
                return Ok(str_value(lexical));
            }
            Err(unknown())
        }
        XdtoRepr::Value(_, data) => {
            if is("Значение", "Value") {
                return Ok(data.value.clone());
            }
            if is("ЛексическоеЗначение", "LexicalValue") {
                return Ok(str_value(&data.lexical));
            }
            Err(unknown())
        }
        // Своих читаемых членов у фабрики нет: `Тип` и `Создать` — методы,
        // а на постороннее имя платформа отвечает ошибкой (измерено
        // `Фаб.НетТакогоЧлена`). `Пакеты` этой реализацией не поддержаны.
        XdtoRepr::Factory(_) => Err(unknown()),
        // У экземпляра члены — это СВОЙСТВА ЕГО ТИПА: `Тип`, `Владелец`,
        // `Свойства` читаются методами, а не точкой (измерено — все три
        // как члены отвергнуты).
        XdtoRepr::Object(data) => object_get_property(data, name),
        // А вот у списка и последовательности владелец, наоборот, ЧЛЕН:
        // `Список.Владелец` даёт объект, `Список.Владелец()` — ошибка
        // (измерено обе пробы, и то же самое у последовательности).
        XdtoRepr::List(data, _) | XdtoRepr::Sequence(data) => {
            if is("Владелец", "Owner") {
                return Ok(instance_value(data));
            }
            Err(unknown())
        }
        _ => Err(unknown()),
    }
}

/// Длина коллекции свойств или фасетов.
///
/// # Errors
///
/// [`RtError::Xdto`], если модель ссылается на несуществующий тип.
pub fn collection_len(obj: &XdtoRepr) -> Option<RtResult<usize>> {
    match obj {
        XdtoRepr::Properties(model, i) => Some(model.type_at(*i).map(|data| data.properties.len())),
        XdtoRepr::Facets(model, i) => Some(model.type_at(*i).map(|data| data.facets.len())),
        // Длина есть и у экземплярных коллекций: `Количество()` измерено и
        // у списка, и у последовательности. Разница между ними в другом —
        // список ещё и ИНДЕКСИРУЕТСЯ, а последовательность нет, поэтому
        // `Для Каждого` по ней платформа отвергает (измерено).
        XdtoRepr::List(data, prop) => Some(Ok(list_len(data, *prop))),
        XdtoRepr::Sequence(data) => Some(sequence_len(data)),
        _ => None,
    }
}

/// Элемент коллекции по номеру.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номер за границей.
pub fn collection_get(obj: &XdtoRepr, i: usize) -> RtResult<BslValue> {
    match obj {
        XdtoRepr::Properties(model, t) => {
            let data = model.type_at(*t)?;
            match data.properties.get(i) {
                Some(p) => Ok(property_value(model, *p)),
                None => Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.properties.len(),
                }),
            }
        }
        XdtoRepr::Facets(model, t) => {
            let data = model.type_at(*t)?;
            if i < data.facets.len() {
                Ok(shell_value(XdtoRepr::Facet(model.clone(), *t, i)))
            } else {
                Err(RtError::IndexOutOfBounds {
                    index: i as i64,
                    len: data.facets.len(),
                })
            }
        }
        // `Список[i]` и `Для Каждого` по списку — измерены оба.
        XdtoRepr::List(data, prop) => list_item(data, *prop, i),
        _ => Err(RtError::NotIndexable),
    }
}

/// `Получить` у коллекции свойств (имя или номер) и у коллекции фасетов
/// (только номер — поиск по имени платформа отвергает, измерено).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не коллекция модели
/// типов или аргумент не тот; [`RtError::IndexOutOfBounds`] на номере за
/// границей.
pub fn collection_lookup(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let not_applicable = || RtError::MethodNotApplicable {
        method: "Получить",
        receiver: obj.type_descriptor().name,
    };
    let Some(o) = repr_of_object(obj) else {
        return Err(not_applicable());
    };
    let [arg] = args else {
        return Err(not_applicable());
    };
    match (o, arg) {
        (XdtoRepr::Properties(model, t), BslValue::Str(s)) => {
            let data = model.type_at(*t)?;
            let name = s.to_string();
            // Неизвестное имя — `Неопределено`, а не ошибка (измерено).
            Ok(data
                .properties
                .iter()
                .find(|p| {
                    model
                        .properties
                        .get(**p)
                        .is_some_and(|prop| prop.name == name)
                })
                .map_or(BslValue::Undefined, |p| property_value(model, *p)))
        }
        (XdtoRepr::Properties(..) | XdtoRepr::Facets(..), BslValue::Number(n)) => {
            let index = n.to_i64_exact().ok_or_else(not_applicable)?;
            let len = match collection_len(o) {
                Some(len) => len?,
                None => return Err(not_applicable()),
            };
            let index =
                usize::try_from(index).map_err(|_| RtError::IndexOutOfBounds { index, len })?;
            collection_get(o, index)
        }
        _ => Err(not_applicable()),
    }
}
