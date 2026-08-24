//! Объекты компонента: `ЧтениеJSON`, `ЗаписьJSON`, их настройки — вместе
//! с поверхностью BSL, таблицами методов и свойств.

// --- Склейка с объектами BSL --------------------------------------------
//
// Методы `ЧтениеJSON`/`ЗаписьJSON` живут здесь, а не в `lib.rs`: там
// `BslValue` уже на две с лишним тысячи строк, а семантика JSON целиком
// принадлежит этому модулю. Наружу они уходят через `builtin.rs`, как и
// методы таблицы значений через `table.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use bsl_rt::RuntimeShapes;

use crate::{parse::*, write::*};

use bsl_rt::{
    Arity, BslValue, CallContext, EnumValue, MethodDescriptor, ObjectProtocol, PropertyDescriptor,
    RtError, RtResult, TypeDescriptor,
};

#[derive(Debug, Default)]
pub(crate) struct JsonReaderState {
    pub(crate) parser: Option<JsonParser>,
    pub(crate) current: Option<JsonEvent>,
}

#[derive(Debug, Default)]
pub(crate) struct JsonReaderObject {
    state: Rc<RefCell<JsonReaderState>>,
}

#[derive(Debug, Default)]
pub(crate) struct JsonWriterObject {
    /// Состояние за `Rc<RefCell>`: обработчики достают его из получателя
    /// ссылкой через `as_writer`, обёртка значения не пересобирается.
    writer: Rc<RefCell<Option<JsonWriter>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct JsonWriterSettingsObject(JsonWriterSettings);

#[derive(Debug, Default)]
pub(crate) struct JsonSerializerSettingsObject(Rc<RefCell<JsonSerializerSettings>>);

pub(crate) static READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ЧтениеJSON",
    type_display: "Чтение JSON",
    type_names: &["JSONReader"],
};
pub(crate) static WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ЗаписьJSON",
    type_display: "Запись JSON",
    type_names: &["JSONWriter"],
};
pub(crate) static WRITER_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "ПараметрыЗаписиJSON",
    type_display: "Параметры записи JSON",
    type_names: &["JSONWriterSettings"],
};
pub(crate) static SERIALIZER_SETTINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: env!("CARGO_PKG_NAME"),
    name: "НастройкиСериализацииJSON",
    type_display: "Настройки сериализации JSON",
    type_names: &["JSONSerializerSettings"],
};

pub(crate) fn as_reader(v: &dyn ObjectProtocol) -> RtResult<&std::cell::RefCell<JsonReaderState>> {
    v.downcast_ref::<JsonReaderObject>()
        .map(|reader| reader.state.as_ref())
        .ok_or_else(|| not_applicable(v, "ЧтениеJSON"))
}

pub(crate) fn as_writer(
    v: &dyn ObjectProtocol,
) -> RtResult<&std::cell::RefCell<Option<JsonWriter>>> {
    v.downcast_ref::<JsonWriterObject>()
        .map(|writer| writer.writer.as_ref())
        .ok_or_else(|| not_applicable(v, "ЗаписьJSON"))
}

pub(crate) fn not_applicable(v: &dyn ObjectProtocol, _expected: &str) -> RtError {
    RtError::MethodNotApplicable {
        method: "метод JSON",
        receiver: v.type_descriptor().name,
    }
}

/// Объект за значением аргумента: не-объект получает ту же ошибку «метод
/// JSON не применим», что и объект чужого типа.
pub(crate) fn arg_object(v: &BslValue) -> RtResult<&dyn ObjectProtocol> {
    v.object_ref()
        .map(bsl_rt::ObjectRef::as_dyn)
        .ok_or_else(|| RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: v.type_name(),
        })
}

/// Настройки из аргумента `УстановитьСтроку([Параметры])`. Отсутствующий
/// аргумент — умолчание (переносы есть, отступа нет).
pub(crate) fn settings_from(arg: Option<&BslValue>) -> RtResult<JsonWriterSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(JsonWriterSettings::default()),
        Some(value) => value
            .object_ref()
            .and_then(|object| object.downcast_ref::<JsonWriterSettingsObject>())
            .map(|settings| settings.0.clone())
            .ok_or(RtError::TypeError {
                expected: "ПараметрыЗаписиJSON",
                op: "УстановитьСтроку",
            }),
    }
}

/// Настройки из третьего аргумента `ЗаписатьJSON(Запись, Значение,
/// [Настройки])`. Отсутствующий аргумент — умолчания
/// [`JsonSerializerSettings::default`].
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент задан и не `НастройкиСериализацииJSON`.
pub fn serializer_settings_from(arg: Option<&BslValue>) -> RtResult<JsonSerializerSettings> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(JsonSerializerSettings::default()),
        Some(value) => value
            .object_ref()
            .and_then(|object| object.downcast_ref::<JsonSerializerSettingsObject>())
            .map(|settings| settings.0.borrow().clone())
            .ok_or(RtError::TypeError {
                expected: "НастройкиСериализацииJSON",
                op: "ЗаписатьJSON",
            }),
    }
}

/// `НастройкиСериализацииJSON.<поле>` (чтение) — три свойства, все
/// читаемые и записываемые (см. `set_serializer_setting`).
///
/// Все шесть написаний (рус./англ. на каждое из трёх свойств) — ИЗМЕРЕНО
/// (`JSON.SETTINGS.PROPERTY_NAMES`): `ФорматСериализацииДаты` (с «ы»!) /
/// `DateSerializationFormat`, `ВариантЗаписиДаты` / `DateWritingVariant`,
/// `СериализовыватьМассивыКакОбъекты` / `SerializeArraysAsObjects`. Статья
/// 16.2.3.2 приводит `ФорматСериализацииДат` (без «ы») — это ОПЕЧАТКА
/// статьи, живая 8.3.27 такое имя отвергает («Поле объекта не обнаружено»,
/// снято прогоном фикстуры `json-dates`), поэтому здесь оно НЕ принимается.
///
/// # Errors
///
/// [`RtError::UnknownColumn`] на неизвестном имени; [`RtError::NotAnObject`],
/// если получатель не `НастройкиСериализацииJSON`.
pub fn get_serializer_setting(obj: &dyn ObjectProtocol, name: &str) -> RtResult<BslValue> {
    let settings = obj
        .downcast_ref::<JsonSerializerSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    let s = settings.0.borrow();
    if name.eq_ignore_ascii_case("ФорматСериализацииДаты")
        || name.eq_ignore_ascii_case("DateSerializationFormat")
    {
        Ok(BslValue::Enum(s.date_format.to_enum_value()))
    } else if name.eq_ignore_ascii_case("ВариантЗаписиДаты")
        || name.eq_ignore_ascii_case("DateWritingVariant")
    {
        Ok(BslValue::Enum(s.date_variant.to_enum_value()))
    } else if name.eq_ignore_ascii_case("СериализовыватьМассивыКакОбъекты")
        || name.eq_ignore_ascii_case("SerializeArraysAsObjects")
    {
        Ok(BslValue::Boolean(s.arrays_as_objects))
    } else {
        Err(RtError::UnknownColumn(name.to_string()))
    }
}

/// `НастройкиСериализацииJSON.<поле> = Значение` (запись).
///
/// # Errors
///
/// [`RtError::TypeError`] на значении не того типа; [`RtError::UnknownColumn`]
/// на неизвестном имени; [`RtError::NotAnObject`], если получатель не
/// `НастройкиСериализацииJSON`.
pub fn set_serializer_setting(obj: &dyn ObjectProtocol, name: &str, val: BslValue) -> RtResult<()> {
    let settings = obj
        .downcast_ref::<JsonSerializerSettingsObject>()
        .ok_or(RtError::NotAnObject)?;
    if name.eq_ignore_ascii_case("ФорматСериализацииДаты")
        || name.eq_ignore_ascii_case("DateSerializationFormat")
    {
        let BslValue::Enum(e) = val else {
            return Err(RtError::TypeError {
                expected: "ФорматДатыJSON",
                op: "ФорматСериализацииДаты",
            });
        };
        let format = JsonDateFormat::from_enum_value(e).ok_or(RtError::TypeError {
            expected: "ФорматДатыJSON",
            op: "ФорматСериализацииДаты",
        })?;
        settings.0.borrow_mut().date_format = format;
        Ok(())
    } else if name.eq_ignore_ascii_case("ВариантЗаписиДаты")
        || name.eq_ignore_ascii_case("DateWritingVariant")
    {
        let BslValue::Enum(e) = val else {
            return Err(RtError::TypeError {
                expected: "ВариантЗаписиДатыJSON",
                op: "ВариантЗаписиДаты",
            });
        };
        let variant = JsonDateWritingVariant::from_enum_value(e).ok_or(RtError::TypeError {
            expected: "ВариантЗаписиДатыJSON",
            op: "ВариантЗаписиДаты",
        })?;
        settings.0.borrow_mut().date_variant = variant;
        Ok(())
    } else if name.eq_ignore_ascii_case("СериализовыватьМассивыКакОбъекты")
        || name.eq_ignore_ascii_case("SerializeArraysAsObjects")
    {
        let BslValue::Boolean(b) = val else {
            return Err(RtError::TypeError {
                expected: "Булево",
                op: "СериализовыватьМассивыКакОбъекты",
            });
        };
        settings.0.borrow_mut().arrays_as_objects = b;
        Ok(())
    } else {
        Err(RtError::UnknownColumn(name.to_string()))
    }
}

/// `ЧтениеJSON.УстановитьСтроку(Текст)` / `ЗаписьJSON.УстановитьСтроку([Параметры])`.
///
/// Один метод на два объекта — как и в самой платформе: смысл выбирается
/// по получателю, а не по имени.
///
/// # Errors
///
/// [`RtError::TypeError`], если получатель не объект JSON либо аргумент не
/// того типа.
pub fn set_string(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    if let Ok(reader) = as_reader(obj) {
        let BslValue::Str(text) = args.first().unwrap_or(&BslValue::Undefined) else {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "УстановитьСтроку",
            });
        };
        *reader.borrow_mut() = JsonReaderState {
            parser: Some(JsonParser::from_bsl_string(text)),
            current: None,
        };
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(JsonWriter::to_string_target(settings_from(args.first())?));
    Ok(())
}

/// `ОткрытьФайл(Имя[, Параметры])`. У читателя файл загружается целиком:
/// разборщик и так держит весь текст (`ЧтениеJSON` в платформе потоковый,
/// но наблюдаемой разницы это не даёт, кроме памяти на большом файле).
///
/// # Errors
///
/// [`RtError::IoError`], если файл не читается; [`RtError::TypeError`] при
/// неверных аргументах.
pub fn open_file(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let BslValue::Str(path) = args.first().unwrap_or(&BslValue::Undefined) else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОткрытьФайл",
        });
    };
    let path = path.to_string();
    if let Ok(reader) = as_reader(obj) {
        let text =
            std::fs::read_to_string(&path).map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        // Метка порядка байтов в начале файла — не часть документа.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
        *reader.borrow_mut() = JsonReaderState {
            parser: Some(JsonParser::from_string(text)),
            current: None,
        };
        return Ok(());
    }
    let writer = as_writer(obj)?;
    *writer.borrow_mut() = Some(JsonWriter::to_file(
        std::path::PathBuf::from(path),
        settings_from(args.get(1))?,
    ));
    Ok(())
}

/// `ЧтениеJSON.Прочитать()` -> `Булево`: удалось ли перейти к следующему
/// элементу.
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе; [`RtError::TypeError`], если источник
/// ещё не назначен.
pub fn read(obj: &dyn ObjectProtocol) -> RtResult<bool> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    let Some(parser) = state.parser.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
            op: "Прочитать",
        });
    };
    let event = parser.next_event()?;
    let advanced = event.is_some();
    state.current = event;
    Ok(advanced)
}

/// `ЧтениеJSON.Пропустить()` — проглотить текущее значение целиком вместе
/// с вложенными. Имя именно такое: `ПропуститьЗначение` платформа не знает
/// (измерено — «Метод объекта не обнаружен»).
///
/// # Errors
///
/// [`RtError::Json`] на битом вводе.
pub fn skip(obj: &dyn ObjectProtocol) -> RtResult<()> {
    let reader = as_reader(obj)?;
    let mut state = reader.borrow_mut();
    if state.parser.is_none() {
        return Err(RtError::TypeError {
            expected: "назначенный источник (УстановитьСтроку/ОткрытьФайл)",
            op: "Пропустить",
        });
    }
    // Глубина считается от ТЕКУЩЕГО события: если стоим на начале
    // контейнера, дочитываем до его конца; на скаляре — делать нечего.
    let mut depth = match state.current {
        Some(JsonEvent::ObjectStart) | Some(JsonEvent::ArrayStart) => 1,
        _ => 0,
    };
    while depth > 0 {
        let parser = state.parser.as_mut().expect("наличие проверено выше");
        let Some(event) = parser.next_event()? else {
            break;
        };
        match event {
            JsonEvent::ObjectStart | JsonEvent::ArrayStart => depth += 1,
            JsonEvent::ObjectEnd | JsonEvent::ArrayEnd => depth -= 1,
            _ => {}
        }
        state.current = Some(event);
    }
    // ИЗМЕРЕНО (`JSON.READ.SKIP`): после пропуска читатель стоит на
    // СЛЕДУЮЩЕМ элементе, а не на закрывающей скобке пропущенного. То есть
    // `Пропустить` дочитывает значение и делает ещё один шаг — иначе
    // вызывающий цикл увидел бы лишний `КонецМассива`, которого у
    // платформы нет.
    let parser = state.parser.as_mut().expect("наличие проверено выше");
    let next = parser.next_event()?;
    state.current = next;
    Ok(())
}

/// `ЧтениеJSON.ТипТекущегоЗначения` — член `ТипЗначенияJSON`.
pub fn current_value_type(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    let v = match &state.current {
        None => EnumValue::JsonNothing,
        Some(JsonEvent::ObjectStart) => EnumValue::JsonObjectStart,
        Some(JsonEvent::ObjectEnd) => EnumValue::JsonObjectEnd,
        Some(JsonEvent::ArrayStart) => EnumValue::JsonArrayStart,
        Some(JsonEvent::ArrayEnd) => EnumValue::JsonArrayEnd,
        Some(JsonEvent::PropertyName(_)) => EnumValue::JsonPropertyName,
        Some(JsonEvent::Str(_)) => EnumValue::JsonString,
        Some(JsonEvent::Number(_)) => EnumValue::JsonNumber,
        Some(JsonEvent::Boolean(_)) => EnumValue::JsonBoolean,
        Some(JsonEvent::Null) => EnumValue::JsonNull,
    };
    Ok(BslValue::Enum(v))
}

/// `ЧтениеJSON.ТекущееЗначение`.
///
/// # Errors
///
/// [`RtError::Json`], если у текущего элемента значения нет (мы стоим на
/// скобке). ИЗМЕРЕНО: платформа отвечает ровно так же — «Текущее значение
/// JSON не может быть получено», — а не `Неопределено`.
pub fn current_value(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    let reader = as_reader(obj)?;
    let state = reader.borrow();
    match &state.current {
        Some(JsonEvent::PropertyName(s)) | Some(JsonEvent::Str(s)) => {
            Ok(BslValue::Str(bsl_rt::BslString::from_str(s)))
        }
        Some(JsonEvent::Number(n)) => Ok(BslValue::Number(n.clone())),
        Some(JsonEvent::Boolean(b)) => Ok(BslValue::Boolean(*b)),
        // JSON `null` при чтении становится `Неопределено`, а не `Null` —
        // измерено на `ПрочитатьJSON` (`JSON.DESERIALIZE.VALUE_TYPES`).
        Some(JsonEvent::Null) => Ok(BslValue::Undefined),
        _ => Err(RtError::Json(
            "Текущее значение JSON не может быть получено".to_string(),
        )),
    }
}

/// Общая часть всех пишущих методов: достать writer и применить операцию.
pub(crate) fn with_writer_cell<T>(
    cell: &RefCell<Option<JsonWriter>>,
    f: impl FnOnce(&mut JsonWriter) -> RtResult<T>,
) -> RtResult<T> {
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Err(RtError::TypeError {
            expected: "назначенный приёмник (УстановитьСтроку/ОткрытьФайл)",
            op: "запись JSON",
        });
    };
    f(writer)
}

fn with_writer<T>(
    obj: &dyn ObjectProtocol,
    f: impl FnOnce(&mut JsonWriter) -> RtResult<T>,
) -> RtResult<T> {
    with_writer_cell(as_writer(obj)?, f)
}

pub(crate) fn close_writer_cell(cell: &RefCell<Option<JsonWriter>>) -> RtResult<BslValue> {
    let mut slot = cell.borrow_mut();
    let Some(writer) = slot.as_mut() else {
        return Ok(BslValue::Str(bsl_rt::BslString::from_str("")));
    };
    let text = writer.finish()?;
    *slot = None;
    Ok(BslValue::Str(bsl_rt::BslString::from_utf8_string(text)))
}

/// # Errors
///
/// Ошибку структуры документа либо отсутствия приёмника.
pub fn write_start_object(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_object(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_object)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_start_array(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::begin_array)
}

/// # Errors
///
/// См. [`write_start_object`].
pub fn write_end_array(obj: &dyn ObjectProtocol) -> RtResult<()> {
    with_writer(obj, JsonWriter::end_array)
}

/// # Errors
///
/// [`RtError::TypeError`], если имя не строка.
pub fn write_property_name(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let BslValue::Str(name) = args.first().unwrap_or(&BslValue::Undefined) else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ЗаписатьИмяСвойства",
        });
    };
    with_writer(obj, |w| w.property_name_bsl(name))
}

/// # Errors
///
/// [`RtError::TypeError`] на типе, который платформа не принимает
/// (`Null`, `Неопределено`, `Дата`).
pub fn write_value(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<()> {
    let value = args.first().cloned().unwrap_or(BslValue::Undefined);
    with_writer(obj, |w| w.value(&value))
}

/// `ЗаписьJSON.Закрыть()` -> текст (для строкового приёмника) либо пустая
/// строка (для файлового).
///
/// # Errors
///
/// [`RtError::IoError`] при неудачной записи файла.
pub fn close_writer(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    close_writer_cell(as_writer(obj)?)
}

/// Получатель — `ЗаписьJSON`? Нужно `BslValue::close_object`, чтобы
/// развести одноимённый `Закрыть` у двух разных объектов.
pub fn is_json_writer(v: &BslValue) -> bool {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<JsonWriterObject>())
        .is_some()
}

/// `ЗаписьJSON.ПроверятьСтруктуру` (чтение).
///
/// # Errors
///
/// [`RtError::TypeError`], если приёмник ещё не назначен — то же условие,
/// что и у остальных методов записи.
pub fn get_check_structure(obj: &dyn ObjectProtocol) -> RtResult<BslValue> {
    with_writer(obj, |w| Ok(BslValue::Boolean(w.check_structure())))
}

/// `ЗаписьJSON.ПроверятьСтруктуру` (запись).
///
/// # Errors
///
/// [`RtError::TypeError`], если значение не `Булево` либо приёмник ещё не
/// назначен.
pub fn set_check_structure(obj: &dyn ObjectProtocol, val: BslValue) -> RtResult<()> {
    let BslValue::Boolean(b) = val else {
        return Err(RtError::TypeError {
            expected: "Булево",
            op: "ПроверятьСтруктуру",
        });
    };
    with_writer(obj, |w| {
        w.set_check_structure(b);
        Ok(())
    })
}

pub(crate) fn exact_method_arity(
    _name: &str,
    arguments: &[BslValue],
    count: usize,
) -> RtResult<()> {
    if arguments.len() == count {
        Ok(())
    } else {
        Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: "JSON",
        })
    }
}

/// Создаёт ненастроенный `ЧтениеJSON`.
pub fn new_json_reader() -> BslValue {
    BslValue::new_object(JsonReaderObject {
        state: Rc::new(RefCell::new(JsonReaderState::default())),
    })
}

/// Создаёт ненастроенный `ЗаписьJSON`.
pub fn new_json_writer() -> BslValue {
    BslValue::new_object(JsonWriterObject {
        writer: Rc::new(RefCell::new(None)),
    })
}

/// Создаёт `ПараметрыЗаписиJSON` из нуля, одного или двух аргументов.
///
/// # Errors
///
/// Ошибка арности или [`RtError::TypeError`], если перенос строк
/// или строка отступа имеют неверный тип.
pub fn new_json_writer_settings(arguments: &[BslValue]) -> RtResult<BslValue> {
    if arguments.len() > 2 {
        return Err(RtError::MethodNotApplicable {
            method: "Новый ПараметрыЗаписиJSON",
            receiver: WRITER_SETTINGS_TYPE.name,
        });
    }
    let line_break = arguments.first().unwrap_or(&BslValue::Undefined);
    let indent = arguments.get(1).unwrap_or(&BslValue::Undefined);
    let line_break = match line_break {
        BslValue::Undefined => JsonLineBreak::Auto,
        BslValue::Enum(EnumValue::LineBreakNone) => JsonLineBreak::None,
        BslValue::Enum(EnumValue::LineBreakAuto) => JsonLineBreak::Auto,
        BslValue::Enum(EnumValue::LineBreakWindows) => JsonLineBreak::Windows,
        BslValue::Enum(EnumValue::LineBreakUnix) => JsonLineBreak::Unix,
        _ => {
            return Err(RtError::TypeError {
                expected: "ПереносСтрокJSON",
                op: "Новый ПараметрыЗаписиJSON",
            });
        }
    };
    let indent = match indent {
        BslValue::Undefined => String::new(),
        BslValue::Str(value) => value.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: "Новый ПараметрыЗаписиJSON",
            });
        }
    };
    Ok(BslValue::new_object(JsonWriterSettingsObject(
        JsonWriterSettings { line_break, indent },
    )))
}

/// Создаёт `НастройкиСериализацииJSON` с платформенными умолчаниями.
pub fn new_json_serializer_settings() -> BslValue {
    BslValue::new_object(JsonSerializerSettingsObject(Rc::new(RefCell::new(
        JsonSerializerSettings::default(),
    ))))
}

impl ObjectProtocol for JsonReaderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &READER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        READER_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        READER_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

// Свойства читателя: оба только на чтение — позиция разбора меняется
// методами, не присваиванием.
fn reader_current_value_type(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    current_value_type(receiver)
}

fn reader_current_value(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    current_value(receiver)
}

static READER_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ТипТекущегоЗначения", "CurrentValueType"],
        get: reader_current_value_type,
        set: None,
    },
    PropertyDescriptor {
        names: &["ТекущееЗначение", "CurrentValue"],
        get: reader_current_value,
        set: None,
    },
];

// Обработчики статической таблицы читателя: получатель приходит от
// вызывающего (VM отдаёт исходное значение — без пересборки обёртки на
// каждый вызов), проверки арности — прежние, из веток `method_is`.
fn reader_set_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("УстановитьСтроку", arguments, 1)?;
    set_string(receiver, arguments).map(|()| BslValue::Undefined)
}

fn reader_open_file(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ОткрытьФайл", arguments, 1)?;
    open_file(receiver, arguments).map(|()| BslValue::Undefined)
}

fn reader_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Прочитать", arguments, 0)?;
    read(receiver).map(BslValue::Boolean)
}

fn reader_skip(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Пропустить", arguments, 0)?;
    skip(receiver).map(|()| BslValue::Undefined)
}

const READER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьСтроку", "SetString"],
        Arity::exact(1),
        reader_set_string,
    ),
    MethodDescriptor::new(
        &["ОткрытьФайл", "OpenFile"],
        Arity::range(1, 2),
        reader_open_file,
    ),
    MethodDescriptor::new(&["Прочитать", "Read"], Arity::exact(0), reader_read),
    MethodDescriptor::new(&["Пропустить", "Skip"], Arity::exact(0), reader_skip),
];

// Обработчики статической таблицы писателя: тела — прежние ветки
// `invoke`, состояние достаётся из получателя через `as_writer`.
fn writer_set_string(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if arguments.len() > 1 {
        return Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: WRITER_TYPE.name,
        });
    }
    *as_writer(receiver)?.borrow_mut() = Some(JsonWriter::to_string_target(settings_from(
        arguments.first(),
    )?));
    Ok(BslValue::Undefined)
}

fn writer_open_file(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    if !(1..=2).contains(&arguments.len()) {
        return Err(RtError::MethodNotApplicable {
            method: "метод JSON",
            receiver: WRITER_TYPE.name,
        });
    }
    let BslValue::Str(path) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ОткрытьФайл",
        });
    };
    *as_writer(receiver)?.borrow_mut() = Some(JsonWriter::to_file(
        std::path::PathBuf::from(path.to_string()),
        settings_from(arguments.get(1))?,
    ));
    Ok(BslValue::Undefined)
}

fn writer_close(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("Закрыть", arguments, 0)?;
    close_writer_cell(as_writer(receiver)?)
}

fn writer_start_object(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьНачалоОбъекта", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::begin_object).map(|()| BslValue::Undefined)
}

fn writer_end_object(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьКонецОбъекта", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::end_object).map(|()| BslValue::Undefined)
}

fn writer_start_array(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьНачалоМассива", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::begin_array).map(|()| BslValue::Undefined)
}

fn writer_end_array(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьКонецМассива", arguments, 0)?;
    with_writer_cell(as_writer(receiver)?, JsonWriter::end_array).map(|()| BslValue::Undefined)
}

fn writer_property_name(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьИмяСвойства", arguments, 1)?;
    let BslValue::Str(property) = &arguments[0] else {
        return Err(RtError::TypeError {
            expected: "Строка",
            op: "ЗаписатьИмяСвойства",
        });
    };
    with_writer_cell(as_writer(receiver)?, |writer| {
        writer.property_name_bsl(property)
    })
    .map(|()| BslValue::Undefined)
}

fn writer_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    exact_method_arity("ЗаписатьЗначение", arguments, 1)?;
    with_writer_cell(as_writer(receiver)?, |writer| writer.value(&arguments[0]))
        .map(|()| BslValue::Undefined)
}

const WRITER_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(
        &["УстановитьСтроку", "SetString"],
        Arity::range(0, 1),
        writer_set_string,
    ),
    MethodDescriptor::new(
        &["ОткрытьФайл", "OpenFile"],
        Arity::range(1, 2),
        writer_open_file,
    ),
    MethodDescriptor::new(&["Закрыть", "Close"], Arity::exact(0), writer_close),
    MethodDescriptor::new(
        &["ЗаписатьНачалоОбъекта", "WriteStartObject"],
        Arity::exact(0),
        writer_start_object,
    ),
    MethodDescriptor::new(
        &["ЗаписатьКонецОбъекта", "WriteEndObject"],
        Arity::exact(0),
        writer_end_object,
    ),
    MethodDescriptor::new(
        &["ЗаписатьНачалоМассива", "WriteStartArray"],
        Arity::exact(0),
        writer_start_array,
    ),
    MethodDescriptor::new(
        &["ЗаписатьКонецМассива", "WriteEndArray"],
        Arity::exact(0),
        writer_end_array,
    ),
    MethodDescriptor::new(
        &["ЗаписатьИмяСвойства", "WritePropertyName"],
        Arity::exact(1),
        writer_property_name,
    ),
    MethodDescriptor::new(
        &["ЗаписатьЗначение", "WriteValue"],
        Arity::exact(1),
        writer_value,
    ),
];

impl ObjectProtocol for JsonWriterObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        WRITER_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        WRITER_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

// `ПроверятьСтруктуру` — единственное свойство писателя, и оно
// читается-пишется.
fn writer_get_check_structure(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    get_check_structure(receiver)
}

fn writer_set_check_structure(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    set_check_structure(receiver, value)
}

static WRITER_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ПроверятьСтруктуру", "CheckStructure"],
    get: writer_get_check_structure,
    set: Some(writer_set_check_structure),
}];

// Настройки сериализации: три свойства, все с записью. Имена — измеренные
// (см. `get_serializer_setting`: `ФорматСериализацииДат` без «ы» платформа
// отвергает, это опечатка статьи 16.2.3.2).
fn settings_get(receiver: &dyn ObjectProtocol, name: &'static str) -> RtResult<BslValue> {
    get_serializer_setting(receiver, name)
}

fn settings_get_date_format(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    settings_get(receiver, "ФорматСериализацииДаты")
}

fn settings_set_date_format(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    set_serializer_setting(receiver, "ФорматСериализацииДаты", value)
}

fn settings_get_date_variant(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    settings_get(receiver, "ВариантЗаписиДаты")
}

fn settings_set_date_variant(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    set_serializer_setting(receiver, "ВариантЗаписиДаты", value)
}

fn settings_get_arrays_as_objects(
    receiver: &dyn ObjectProtocol,
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    settings_get(receiver, "СериализовыватьМассивыКакОбъекты")
}

fn settings_set_arrays_as_objects(
    receiver: &dyn ObjectProtocol,
    value: BslValue,
    _context: &mut CallContext<'_>,
) -> RtResult<()> {
    set_serializer_setting(receiver, "СериализовыватьМассивыКакОбъекты", value)
}

static SERIALIZER_SETTINGS_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ФорматСериализацииДаты", "DateSerializationFormat"],
        get: settings_get_date_format,
        set: Some(settings_set_date_format),
    },
    PropertyDescriptor {
        names: &["ВариантЗаписиДаты", "DateWritingVariant"],
        get: settings_get_date_variant,
        set: Some(settings_set_date_variant),
    },
    PropertyDescriptor {
        names: &[
            "СериализовыватьМассивыКакОбъекты",
            "SerializeArraysAsObjects",
        ],
        get: settings_get_arrays_as_objects,
        set: Some(settings_set_arrays_as_objects),
    },
];

impl ObjectProtocol for JsonWriterSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &WRITER_SETTINGS_TYPE
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for JsonSerializerSettingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SERIALIZER_SETTINGS_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        SERIALIZER_SETTINGS_PROPERTIES
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

pub(crate) fn callback_name(
    name_arg: Option<&BslValue>,
    module_arg: Option<&BslValue>,
    op: &'static str,
) -> RtResult<Option<String>> {
    let name = match name_arg {
        None | Some(BslValue::Undefined) => return Ok(None),
        Some(BslValue::Str(value)) => value.to_string(),
        Some(_) => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            });
        }
    };
    if name.is_empty() || matches!(module_arg, None | Some(BslValue::Undefined)) {
        Ok(None)
    } else {
        Ok(Some(name))
    }
}

pub(crate) fn name_list_arg(
    argument: Option<&BslValue>,
    runtime: &RuntimeShapes,
    op: &'static str,
) -> RtResult<Vec<String>> {
    let Some(list) = argument else {
        return Ok(Vec::new());
    };
    if matches!(list, BslValue::Undefined) {
        return Ok(Vec::new());
    }
    let length = list.collection_len().map_err(|_| RtError::TypeError {
        expected: "Массив",
        op,
    })?;
    let mut names = Vec::with_capacity(length);
    for index in 0..length {
        let item = list.get_index(&BslValue::number_from_i64(index as i64), &runtime.names)?;
        if let BslValue::Str(value) = item {
            names.push(value.to_string());
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializer_settings_default_is_iso_local_and_arrays_stay_arrays() {
        let s = JsonSerializerSettings::default();
        assert_eq!(s.date_format, JsonDateFormat::Iso);
        assert_eq!(s.date_variant, JsonDateWritingVariant::Local);
        assert!(!s.arrays_as_objects);
    }

    #[test]
    fn serializer_setting_accessors_round_trip_every_field() {
        let obj = new_json_serializer_settings();
        let obj = obj.object_ref().unwrap().as_dyn();
        set_serializer_setting(
            obj,
            "ФорматСериализацииДаты",
            BslValue::Enum(EnumValue::DateFormatMicrosoft),
        )
        .unwrap();
        set_serializer_setting(
            obj,
            "ВариантЗаписиДаты",
            BslValue::Enum(EnumValue::DateVariantUniversal),
        )
        .unwrap();
        set_serializer_setting(
            obj,
            "СериализовыватьМассивыКакОбъекты",
            BslValue::Boolean(true),
        )
        .unwrap();

        assert_eq!(
            get_serializer_setting(obj, "DateSerializationFormat").unwrap(),
            BslValue::Enum(EnumValue::DateFormatMicrosoft)
        );
        assert_eq!(
            get_serializer_setting(obj, "DateWritingVariant").unwrap(),
            BslValue::Enum(EnumValue::DateVariantUniversal)
        );
        assert_eq!(
            get_serializer_setting(obj, "SerializeArraysAsObjects").unwrap(),
            BslValue::Boolean(true)
        );
        assert!(get_serializer_setting(obj, "НетТакогоСвойства").is_err());
    }

    /// ИЗМЕРЕНО: статья 16.2.3.2 приводит `ФорматСериализацииДат` (без
    /// «ы») — опечатка статьи, живая 8.3.27 такое имя отвергает.
    #[test]
    fn the_article_typo_spelling_of_date_format_property_is_rejected() {
        let obj = new_json_serializer_settings();
        let obj = obj.object_ref().unwrap().as_dyn();
        assert!(get_serializer_setting(obj, "ФорматСериализацииДат").is_err());
    }
}
