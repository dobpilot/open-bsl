//! Объектный протокол оболочки: таблица методов, свойства, индексы.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static VALUE_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТипЗначенияXDTO",
    legacy_type_id: Some(TypeId::XdtoValueType),
};

pub(crate) static OBJECT_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТипОбъектаXDTO",
    legacy_type_id: Some(TypeId::XdtoObjectType),
};

pub(crate) static PROPERTY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СвойствоXDTO",
    legacy_type_id: Some(TypeId::XdtoProperty),
};

pub(crate) static PROPERTIES_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияСвойствXDTO",
    legacy_type_id: Some(TypeId::XdtoPropertyCollection),
};

pub(crate) static FACETS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияФасетовXDTO",
    legacy_type_id: Some(TypeId::XdtoFacetCollection),
};

pub(crate) static FACET_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетXDTO",
    legacy_type_id: Some(TypeId::XdtoFacet),
};

pub(crate) static VALUE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗначениеXDTO",
    legacy_type_id: Some(TypeId::XdtoDataValue),
};

pub(crate) static FACTORY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФабрикаXDTO",
    legacy_type_id: Some(TypeId::XdtoFactory),
};

pub(crate) static SERIALIZER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СериализаторXDTO",
    legacy_type_id: Some(TypeId::XdtoSerializer),
};

pub(crate) static OBJECT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбъектXDTO",
    legacy_type_id: Some(TypeId::XdtoDataObject),
};

pub(crate) static LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокXDTO",
    legacy_type_id: Some(TypeId::XdtoList),
};

pub(crate) static SEQUENCE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПоследовательностьXDTO",
    legacy_type_id: Some(TypeId::XdtoSequence),
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
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["Тип", "Type"],
        call: xdto_type,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["Создать", "Create"],
        call: xdto_create,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["ПрочитатьXML", "ReadXML"],
        call: xdto_read_xml,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["ЗаписатьXML", "WriteXML"],
        call: xdto_write_xml,
    },
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["Получить", "Get"],
        call: xdto_get,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["Установить", "Set"],
        call: xdto_set,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["Добавить", "Add"],
        call: xdto_add,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["Вставить", "Insert"],
        call: xdto_insert,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["Удалить", "Delete"],
        call: xdto_delete,
    },
    MethodDescriptor {
        code: MethodCode::new(10),
        names: &["Очистить", "Clear"],
        call: xdto_clear,
    },
    MethodDescriptor {
        code: MethodCode::new(11),
        names: &["Количество", "Count"],
        call: xdto_count,
    },
    MethodDescriptor {
        code: MethodCode::new(12),
        names: &["ПолучитьСписок", "GetList"],
        call: xdto_get_list,
    },
    MethodDescriptor {
        code: MethodCode::new(13),
        names: &["Установлено", "IsSet"],
        call: xdto_is_set,
    },
    MethodDescriptor {
        code: MethodCode::new(14),
        names: &["Сбросить", "Unset"],
        call: xdto_unset,
    },
    MethodDescriptor {
        code: MethodCode::new(15),
        names: &["Проверить", "Validate"],
        call: xdto_validate,
    },
    MethodDescriptor {
        code: MethodCode::new(16),
        names: &["Свойства", "Properties"],
        call: xdto_properties,
    },
    MethodDescriptor {
        code: MethodCode::new(17),
        names: &["Владелец", "Owner"],
        call: xdto_owner,
    },
    MethodDescriptor {
        code: MethodCode::new(18),
        names: &["Последовательность", "Sequence"],
        call: xdto_sequence,
    },
    MethodDescriptor {
        code: MethodCode::new(19),
        names: &["ПолучитьЗначение", "GetValue"],
        call: xdto_sequence_value,
    },
    MethodDescriptor {
        code: MethodCode::new(20),
        names: &["ПолучитьСвойство", "GetProperty"],
        call: xdto_sequence_property,
    },
    MethodDescriptor {
        code: MethodCode::new(21),
        names: &["XMLТип", "XMLType"],
        call: xdto_xml_type,
    },
    MethodDescriptor {
        code: MethodCode::new(22),
        names: &["XMLТипЗнч", "XMLTypeOf"],
        call: xdto_xml_type_of,
    },
    MethodDescriptor {
        code: MethodCode::new(23),
        names: &["ВозможностьЧтенияXML", "CanReadXML"],
        call: xdto_can_read_xml,
    },
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

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn shell_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}
