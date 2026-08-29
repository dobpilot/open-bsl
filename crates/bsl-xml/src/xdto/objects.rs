//! Объектный протокол оболочки: таблица методов, свойства, индексы.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static VALUE_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТипЗначенияXDTO",
    type_display: "Тип значения XDTO",
    type_names: &["XDTOValueType"],
};

pub(crate) static OBJECT_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТипОбъектаXDTO",
    type_display: "Тип объекта XDTO",
    type_names: &["XDTOObjectType"],
};

pub(crate) static PROPERTY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СвойствоXDTO",
    type_display: "Свойство XDTO",
    type_names: &["XDTOProperty"],
};

pub(crate) static PROPERTIES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияСвойствXDTO",
    type_display: "Коллекция свойств XDTO",
    type_names: &["XDTOPropertyCollection"],
};

pub(crate) static FACETS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияФасетовXDTO",
    type_display: "Коллекция фасетов XDTO",
    type_names: &["XDTOFacetCollection"],
};

pub(crate) static FACET_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетXDTO",
    type_display: "Фасет XDTO",
    type_names: &["XDTOFacet"],
};

pub(crate) static VALUE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗначениеXDTO",
    type_display: "Значение XDTO",
    type_names: &["XDTODataValue"],
};

pub(crate) static FACTORY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФабрикаXDTO",
    type_display: "Фабрика XDTO",
    type_names: &["XDTOFactory"],
};

pub(crate) static SERIALIZER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СериализаторXDTO",
    type_display: "Сериализатор XDTO",
    type_names: &["XDTOSerializer"],
};

pub(crate) static OBJECT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбъектXDTO",
    type_display: "Объект XDTO",
    type_names: &["XDTODataObject"],
};

pub(crate) static LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокXDTO",
    type_display: "Список XDTO",
    type_names: &["XDTOList"],
};

pub(crate) static SEQUENCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПоследовательностьXDTO",
    type_display: "Последовательность XDTO",
    type_names: &["XDTOSequence"],
};

// Обработчики статической таблицы методов оболочки XDTO. Таблица одна на
// все представления, как прежний общий диспетчер: имя делится
// получателями, получатель выбирает семантику по своему `repr`.
pub(crate) fn shell_repr<'v>(
    receiver: &'v dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'v XdtoRepr> {
    repr_of_object(receiver).ok_or(RtError::MethodNotApplicable {
        method,
        receiver: receiver.type_descriptor().name,
    })
}

pub(crate) fn xdto_type(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "Тип")? {
        XdtoRepr::Object(..) | XdtoRepr::Value(..) => object_type(receiver, arguments),
        _ => factory_type(receiver, arguments),
    }
}

pub(crate) fn xdto_create(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    factory_create(receiver, arguments)
}

pub(crate) fn xdto_read_xml(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "ПрочитатьXML")? {
        XdtoRepr::Serializer(_) => serializer_read_xml(receiver, arguments),
        _ => factory_read_xml(receiver, arguments),
    }
}

pub(crate) fn xdto_write_xml(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "ЗаписатьXML")? {
        XdtoRepr::Serializer(_) => {
            serializer_write_xml(receiver, arguments).map(|()| BslValue::Undefined)
        }
        _ => factory_write_xml(receiver, arguments).map(|()| BslValue::Undefined),
    }
}

pub(crate) fn xdto_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "Получить")? {
        XdtoRepr::Properties(..) | XdtoRepr::Facets(..) => collection_lookup(receiver, arguments),
        XdtoRepr::Object(..) => object_get(receiver, arguments),
        XdtoRepr::List(..) => match arguments {
            [index] => list_get(receiver, shell_index(index)?),
            _ => Err(RtError::MethodNotApplicable {
                method: "Получить",
                receiver: receiver.type_descriptor().name,
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: receiver.type_descriptor().name,
        }),
    }
}

pub(crate) fn xdto_set(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "Установить")? {
        XdtoRepr::Object(..) => object_set(receiver, arguments),
        _ => list_set(receiver, arguments),
    }
}

pub(crate) fn xdto_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "Добавить")? {
        XdtoRepr::Sequence(..) => sequence_add(receiver, arguments),
        _ => list_add(receiver, arguments),
    }
}

pub(crate) fn xdto_insert(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    list_insert(receiver, arguments)
}

pub(crate) fn xdto_delete(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match arguments {
        [index] => list_delete(receiver, index).map(|()| BslValue::Undefined),
        _ => Err(RtError::MethodNotApplicable {
            method: "Удалить",
            receiver: receiver.type_descriptor().name,
        }),
    }
}

pub(crate) fn xdto_clear(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match shell_repr(receiver, "Очистить")? {
        XdtoRepr::Sequence(..) => sequence_clear(receiver).map(|()| BslValue::Undefined),
        _ => list_clear(receiver).map(|()| BslValue::Undefined),
    }
}

pub(crate) fn xdto_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    match collection_len(shell_repr(receiver, "Количество")?) {
        Some(len) => len.map(|len| BslValue::number_from_i64(len as i64)),
        None => Err(RtError::MethodNotApplicable {
            method: "Количество",
            receiver: receiver.type_descriptor().name,
        }),
    }
}

pub(crate) fn xdto_get_list(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_get_list(receiver, arguments)
}

pub(crate) fn xdto_is_set(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_is_set(receiver, arguments)
}

pub(crate) fn xdto_unset(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_unset(receiver, arguments)
}

pub(crate) fn xdto_validate(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_validate(receiver, arguments)
}

pub(crate) fn xdto_properties(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_properties(receiver, arguments)
}

pub(crate) fn xdto_owner(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_owner(receiver, arguments)
}

pub(crate) fn xdto_sequence(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    object_sequence(receiver, arguments)
}

pub(crate) fn xdto_sequence_value(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    sequence_value(receiver, arguments)
}

pub(crate) fn xdto_sequence_property(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    sequence_property(receiver, arguments)
}

// Три измеренных члена сериализатора, до которых очередь не дошла: у
// своего получателя — честный отказ «не поддерживается».
pub(crate) fn xdto_xml_type(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Err(serializer_unsupported(receiver, "XMLТип"))
}

pub(crate) fn xdto_xml_type_of(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Err(serializer_unsupported(receiver, "XMLТипЗнч"))
}

pub(crate) fn xdto_can_read_xml(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    Err(serializer_unsupported(receiver, "ВозможностьЧтенияXML"))
}

pub(crate) const XDTO_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["Тип", "Type"], Arity::range(0, 2), xdto_type),
    MethodDescriptor::new(&["Создать", "Create"], Arity::range(1, 2), xdto_create),
    MethodDescriptor::new(
        &["ПрочитатьXML", "ReadXML"],
        Arity::range(1, 2),
        xdto_read_xml,
    ),
    MethodDescriptor::new(
        &["ЗаписатьXML", "WriteXML"],
        Arity::range(2, 4),
        xdto_write_xml,
    ),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), xdto_get),
    MethodDescriptor::new(&["Установить", "Set"], Arity::exact(2), xdto_set),
    MethodDescriptor::new(&["Добавить", "Add"], Arity::range(1, 2), xdto_add),
    MethodDescriptor::new(&["Вставить", "Insert"], Arity::exact(2), xdto_insert),
    MethodDescriptor::new(&["Удалить", "Delete"], Arity::exact(1), xdto_delete),
    MethodDescriptor::new(&["Очистить", "Clear"], Arity::exact(0), xdto_clear),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), xdto_count),
    MethodDescriptor::new(
        &["ПолучитьСписок", "GetList"],
        Arity::exact(1),
        xdto_get_list,
    ),
    MethodDescriptor::new(&["Установлено", "IsSet"], Arity::exact(1), xdto_is_set),
    MethodDescriptor::new(&["Сбросить", "Unset"], Arity::exact(1), xdto_unset),
    MethodDescriptor::new(&["Проверить", "Validate"], Arity::exact(0), xdto_validate),
    MethodDescriptor::new(
        &["Свойства", "Properties"],
        Arity::exact(0),
        xdto_properties,
    ),
    MethodDescriptor::new(&["Владелец", "Owner"], Arity::exact(0), xdto_owner),
    MethodDescriptor::new(
        &["Последовательность", "Sequence"],
        Arity::exact(0),
        xdto_sequence,
    ),
    MethodDescriptor::new(
        &["ПолучитьЗначение", "GetValue"],
        Arity::exact(1),
        xdto_sequence_value,
    ),
    MethodDescriptor::new(
        &["ПолучитьСвойство", "GetProperty"],
        Arity::exact(1),
        xdto_sequence_property,
    ),
    MethodDescriptor::new(&["XMLТип", "XMLType"], Arity::exact(0), xdto_xml_type),
    MethodDescriptor::new(
        &["XMLТипЗнч", "XMLTypeOf"],
        Arity::exact(1),
        xdto_xml_type_of,
    ),
    MethodDescriptor::new(
        &["ВозможностьЧтенияXML", "CanReadXML"],
        Arity::exact(1),
        xdto_can_read_xml,
    ),
];

impl ObjectProtocol for XdtoShell {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        match &self.repr {
            XdtoRepr::Type(model, i) => match model.types.get(*i) {
                Some(data) if data.is_value() => &VALUE_TYPE_TYPE,
                _ => &OBJECT_TYPE_TYPE,
            },
            XdtoRepr::Property(..) => &PROPERTY_TYPE,
            XdtoRepr::Properties(..) => &PROPERTIES_TYPE,
            XdtoRepr::Facets(..) => &FACETS_TYPE,
            XdtoRepr::Facet(..) => &FACET_TYPE,
            XdtoRepr::Value(..) => &VALUE_TYPE,
            XdtoRepr::Factory(_) => &FACTORY_TYPE,
            XdtoRepr::Serializer(_) => &SERIALIZER_TYPE,
            XdtoRepr::Object(..) => &OBJECT_TYPE,
            XdtoRepr::List(..) => &LIST_TYPE,
            XdtoRepr::Sequence(..) => &SEQUENCE_TYPE,
        }
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_property(self.as_dyn(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        match &self.repr {
            XdtoRepr::Object(..) => set_property(self.as_dyn(), name, value),
            _ => Err(RtError::NotAnObject),
        }
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        XDTO_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        collection_get(&self.repr, shell_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        match collection_len(&self.repr) {
            Some(len) => len,
            None => Err(RtError::NotIndexable),
        }
    }

    // Коллекции модели судятся по длине (измерено: непустые `Свойства` и
    // `Фасеты` дают «Да», пустые — «Нет»). Тип, свойство, фасет, фабрика и
    // сериализатор отвечают измеренной ошибкой «Проверка мутабельных
    // значений на заполненность не поддерживается»; про ЭКЗЕМПЛЯРЫ —
    // `ЗначениеXDTO` и `ОбъектXDTO` — замера нет: проба на значении вешает
    // платформу модальным окном, и они отнесены к соседям по аналогии.
    // НЕ ИЗМЕРЕНО(XDTO.VALUE.FILLED)
    fn is_filled(&self) -> RtResult<bool> {
        match &self.repr {
            XdtoRepr::Properties(..) | XdtoRepr::Facets(..) => match collection_len(&self.repr) {
                Some(len) => Ok(len? > 0),
                None => Ok(false),
            },
            _ => Err(RtError::TypeError {
                expected: "Значение, у которого есть признак заполненности",
                op: "ЗначениеЗаполнено",
            }),
        }
    }

    // Тип, свойство и список — ссылки на место в модели или в хранилище;
    // экземпляр и последовательность — на само хранилище (равенства
    // измерены, см. прежние армы `PartialEq`). Остальные виды равны только
    // по тождеству.
    fn identity_key(&self) -> Option<(usize, usize)> {
        match &self.repr {
            XdtoRepr::Type(model, i) | XdtoRepr::Property(model, i) => {
                Some((Rc::as_ptr(model) as usize, *i))
            }
            XdtoRepr::Object(data) => Some((Rc::as_ptr(data) as usize, 0)),
            XdtoRepr::Sequence(data) => Some((Rc::as_ptr(data) as usize, 0)),
            XdtoRepr::List(data, prop) => Some((Rc::as_ptr(data) as usize, *prop)),
            _ => None,
        }
    }

    fn display(&self) -> String {
        display_text(&self.repr).unwrap_or_else(|| self.type_descriptor().name.to_string())
    }
}

pub(crate) const API_MEMBERS: &[bsl_rt::ObjectMembersDescriptor] = &[
    bsl_rt::ObjectMembersDescriptor::new(&VALUE_TYPE_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&OBJECT_TYPE_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&PROPERTY_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&PROPERTIES_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&FACETS_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&FACET_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&VALUE_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&FACTORY_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&SERIALIZER_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&OBJECT_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&LIST_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
    bsl_rt::ObjectMembersDescriptor::new(&SEQUENCE_TYPE)
        .with_methods(XDTO_METHODS)
        .with_dynamic_properties(),
];

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn shell_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}
