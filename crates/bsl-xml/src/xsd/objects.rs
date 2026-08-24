//! Объектный протокол: типы, таблицы, индексация.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static BUILDER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПостроительСхемXML",
    type_display: "Построитель схем XML",
    type_names: &["XMLSchemaBuilder"],
};

pub(crate) static SCHEMA_SET_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "НаборСхемXML",
    type_display: "Набор схем XML",
    type_names: &["XMLSchemaSet"],
};

pub(crate) static EXPANDED_NAME_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "РасширенноеИмяXML",
    type_display: "Расширенное имя XML",
    type_names: &["XMLExpandedName"],
};

pub(crate) static NAME_LIST_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокРасширенныхИменXML",
    type_display: "Список расширенных имен XML",
    type_names: &["XMLExpandedNameList"],
};

pub(crate) static SCHEMA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СхемаXML",
    type_display: "Схема XML",
    type_names: &["XMLSchema"],
};

pub(crate) static ELEMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбъявлениеЭлементаXS",
    type_display: "Объявление элемента XML Schema",
    type_names: &["XSElementDeclaration", "ОбъявлениеЭлементаXS"],
};

pub(crate) static ATTRIBUTE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбъявлениеАтрибутаXS",
    type_display: "Объявление атрибута XML Schema",
    type_names: &["XSAttributeDeclaration", "ОбъявлениеАтрибутаXS"],
};

pub(crate) static SIMPLE_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОпределениеПростогоТипаXS",
    type_display: "Определение простого типа XML Schema",
    type_names: &["XSSimpleTypeDefinition", "ОпределениеПростогоТипаXS"],
};

pub(crate) static COMPLEX_TYPE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОпределениеСоставногоТипаXS",
    type_display: "Определение составного типа XML Schema",
    type_names: &["XSComplexTypeDefinition", "ОпределениеСоставногоТипаXS"],
};

pub(crate) static PARTICLE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФрагментXS",
    type_display: "Фрагмент XML Schema",
    type_names: &["XSParticle", "ФрагментXS"],
};

pub(crate) static MODEL_GROUP_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ГруппаМоделиXS",
    type_display: "Группа модели XML Schema",
    type_names: &["XSModelGroup", "ГруппаМоделиXS"],
};

pub(crate) static ATTRIBUTE_USE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИспользованиеАтрибутаXS",
    type_display: "Использование атрибута XML Schema",
    type_names: &["XSAttributeUse", "ИспользованиеАтрибутаXS"],
};

pub(crate) static ANNOTATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "АннотацияXS",
    type_display: "Аннотация XML Schema",
    type_names: &["XSAnnotation", "АннотацияXS"],
};

pub(crate) static DOCUMENTATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ДокументацияXS",
    type_display: "Документация XML Schema",
    type_names: &["XSDocumentation", "ДокументацияXS"],
};

pub(crate) static APP_INFO_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ИнформацияДляПриложенияXS",
    type_display: "Информация для приложения XML Schema",
    type_names: &["XSAppInfo", "ИнформацияДляПриложенияXS"],
};

pub(crate) static FACET_LENGTH_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетДлиныXS",
    type_display: "Фасет длины значения XML Schema",
    type_names: &["XSLengthFacet", "ФасетДлиныXS"],
};

pub(crate) static FACET_MIN_LENGTH_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМинимальнойДлиныXS",
    type_display: "Фасет минимальной длины значения XML Schema",
    type_names: &["XSMinLengthFacet", "ФасетМинимальнойДлиныXS"],
};

pub(crate) static FACET_MAX_LENGTH_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМаксимальнойДлиныXS",
    type_display: "Фасет максимальной длины значения XML Schema",
    type_names: &["XSMaxLengthFacet", "ФасетМаксимальнойДлиныXS"],
};

pub(crate) static FACET_PATTERN_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетОбразцаXS",
    type_display: "Фасет образца значения XML Schema",
    type_names: &["XSPatternFacet", "ФасетОбразцаXS"],
};

pub(crate) static FACET_ENUMERATION_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетПеречисленияXS",
    type_display: "Фасет перечисления значения XML Schema",
    type_names: &["XSEnumerationFacet", "ФасетПеречисленияXS"],
};

pub(crate) static FACET_WHITE_SPACE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетПробельныхСимволовXS",
    type_display: "Фасет пробельных символов XML Schema",
    type_names: &["XSWhitespaceFacet", "ФасетПробельныхСимволовXS"],
};

pub(crate) static FACET_TOTAL_DIGITS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетОбщегоКоличестваРазрядовXS",
    type_display: "Фасет общего количества разрядов значения XML Schema",
    type_names: &["XSTotalDigitsFacet", "ФасетОбщегоКоличестваРазрядовXS"],
};

pub(crate) static FACET_FRACTION_DIGITS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетКоличестваРазрядовДробнойЧастиXS",
    type_display: "Фасет количества разрядов дробной части значения XML Schema",
    type_names: &[
        "XSFractionDigitsFacet",
        "ФасетКоличестваРазрядовДробнойЧастиXS",
    ],
};

pub(crate) static FACET_MIN_INCLUSIVE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМинимальногоВключающегоЗначенияXS",
    type_display: "Фасет минимального включающего значения XML Schema",
    type_names: &[
        "XSMinInclusiveFacet",
        "ФасетМинимальногоВключающегоЗначенияXS",
    ],
};

pub(crate) static FACET_MAX_INCLUSIVE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМаксимальногоВключающегоЗначенияXS",
    type_display: "Фасет максимального включающего значения XML Schema",
    type_names: &[
        "XSMaxInclusiveFacet",
        "ФасетМаксимальногоВключающегоЗначенияXS",
    ],
};

pub(crate) static FACET_MIN_EXCLUSIVE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМинимальногоИсключающегоЗначенияXS",
    type_display: "Фасет минимального исключающего значения XML Schema",
    type_names: &[
        "XSMinExclusiveFacet",
        "ФасетМинимальногоИсключающегоЗначенияXS",
    ],
};

pub(crate) static FACET_MAX_EXCLUSIVE_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФасетМаксимальногоИсключающегоЗначенияXS",
    type_display: "Фасет максимального исключающего значения XML Schema",
    type_names: &[
        "XSMaxExclusiveFacet",
        "ФасетМаксимальногоИсключающегоЗначенияXS",
    ],
};

pub(crate) static LIST_FIXED_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФиксированныйСписокКомпонентXS",
    type_display: "Фиксированный список компонент XML Schema",
    type_names: &["XSComponentFixedList", "ФиксированныйСписокКомпонентXS"],
};

pub(crate) static LIST_PLAIN_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "СписокКомпонентXS",
    type_display: "Список компонент XML Schema",
    type_names: &["XSComponentList", "СписокКомпонентXS"],
};

pub(crate) static LIST_NAMED_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияИменованныхКомпонентXS",
    type_display: "Коллекция именованных компонент XML Schema",
    type_names: &["XSNamedComponentMap", "КоллекцияИменованныхКомпонентXS"],
};

/// Дескриптор компоненты по виду узла — та же таблица, что
/// `XsKind::type_name`/`type_id`, но статиками протокола.
pub(crate) fn component_descriptor(kind: XsKind) -> &'static TypeDescriptor {
    match kind {
        XsKind::Schema => &SCHEMA_TYPE,
        XsKind::Element => &ELEMENT_TYPE,
        XsKind::Attribute => &ATTRIBUTE_TYPE,
        XsKind::SimpleType => &SIMPLE_TYPE_TYPE,
        XsKind::ComplexType => &COMPLEX_TYPE_TYPE,
        XsKind::Particle => &PARTICLE_TYPE,
        XsKind::ModelGroup => &MODEL_GROUP_TYPE,
        XsKind::AttributeUse => &ATTRIBUTE_USE_TYPE,
        XsKind::Annotation => &ANNOTATION_TYPE,
        XsKind::Documentation => &DOCUMENTATION_TYPE,
        XsKind::AppInfo => &APP_INFO_TYPE,
        XsKind::Facet(facet) => match facet {
            FacetKind::Length => &FACET_LENGTH_TYPE,
            FacetKind::MinLength => &FACET_MIN_LENGTH_TYPE,
            FacetKind::MaxLength => &FACET_MAX_LENGTH_TYPE,
            FacetKind::Pattern => &FACET_PATTERN_TYPE,
            FacetKind::Enumeration => &FACET_ENUMERATION_TYPE,
            FacetKind::WhiteSpace => &FACET_WHITE_SPACE_TYPE,
            FacetKind::TotalDigits => &FACET_TOTAL_DIGITS_TYPE,
            FacetKind::FractionDigits => &FACET_FRACTION_DIGITS_TYPE,
            FacetKind::MinInclusive => &FACET_MIN_INCLUSIVE_TYPE,
            FacetKind::MaxInclusive => &FACET_MAX_INCLUSIVE_TYPE,
            FacetKind::MinExclusive => &FACET_MIN_EXCLUSIVE_TYPE,
            FacetKind::MaxExclusive => &FACET_MAX_EXCLUSIVE_TYPE,
        },
    }
}

pub(crate) fn list_descriptor(kind: &XsListKind) -> &'static TypeDescriptor {
    match kind {
        XsListKind::Fixed(_) => &LIST_FIXED_TYPE,
        XsListKind::Plain(_) => &LIST_PLAIN_TYPE,
        XsListKind::Named(_) => &LIST_NAMED_TYPE,
    }
}

macro_rules! receiver_of {
    ($fn_name:ident, $ty:ty, $type_name:expr) => {
        fn $fn_name<'r>(
            receiver: &'r dyn ObjectProtocol,
            method: &'static str,
        ) -> RtResult<&'r $ty> {
            receiver
                .downcast_ref::<$ty>()
                .ok_or(RtError::MethodNotApplicable {
                    method,
                    receiver: $type_name,
                })
        }
    };
}

receiver_of!(builder_of, BuilderObject, BUILDER_TYPE.name);
receiver_of!(schema_set_of, SchemaSetObject, SCHEMA_SET_TYPE.name);
receiver_of!(schema_list_of, SchemaListObject, "Список компонент XS");
receiver_of!(expanded_of, ExpandedNameObject, EXPANDED_NAME_TYPE.name);
receiver_of!(name_list_of, NameListObject, NAME_LIST_TYPE.name);

pub(crate) fn builder_create_schema(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    builder_of(receiver, "СоздатьСхемуXML")?;
    create_schema(&new_builder(), arguments)
}

pub(crate) static BUILDER_METHODS: &[MethodDescriptor] = &[MethodDescriptor {
    names: &["СоздатьСхемуXML", "CreateXMLSchema"],
    call: builder_create_schema,
}];

impl ObjectProtocol for BuilderObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &BUILDER_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        BUILDER_METHODS
    }
}

/// Значение-получатель набора: свободные функции поверхности приняли его
/// значением ещё до перевода объектов на протокол.
pub(crate) fn schema_set_value(set: &SchemaSetObject) -> BslValue {
    BslValue::new_object(SchemaSetObject {
        schemas: set.schemas.clone(),
    })
}

pub(crate) fn schema_set_method_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let set = schema_set_of(receiver, "Добавить")?;
    schema_set_add(&schema_set_value(set), arguments)
}

pub(crate) fn schema_set_method_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let set = schema_set_of(receiver, "Получить")?;
    match arguments {
        [index] => schema_set_get(&schema_set_value(set), set_index(index)?),
        _ => Err(RtError::MethodNotApplicable {
            method: "Получить",
            receiver: SCHEMA_SET_TYPE.name,
        }),
    }
}

pub(crate) fn schema_set_method_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let set = schema_set_of(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(set.schemas.borrow().len() as i64))
}

pub(crate) static SCHEMA_SET_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        names: &["Добавить", "Add"],
        call: schema_set_method_add,
    },
    MethodDescriptor {
        names: &["Получить", "Get"],
        call: schema_set_method_get,
    },
    MethodDescriptor {
        names: &["Количество", "Count"],
        call: schema_set_method_count,
    },
];

impl ObjectProtocol for SchemaSetObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &SCHEMA_SET_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        SCHEMA_SET_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        schema_set_get(&schema_set_value(self), set_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.schemas.borrow().len())
    }

    // Набор — окно в общий список схем: два значения с одним списком равны.
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.schemas) as usize, 0))
    }
}

/// Номер элемента из значения-индекса — та же семантика, что у `[]`
/// встроенных коллекций.
pub(crate) fn set_index(index: &BslValue) -> RtResult<usize> {
    let BslValue::Number(number) = index else {
        return Err(RtError::BadIndex);
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

impl ObjectProtocol for ComponentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        component_descriptor(self.schema.node(self.index).kind)
    }

    /// Свойства компоненты остаются строковым путём: их набор РАЗНЫЙ у
    /// каждого вида компоненты (у объявления элемента одни, у фасета
    /// другие), и таблица, общая на все виды, обещала бы больше, чем
    /// отдаёт. Свёртка имени внутри — общая `folded_eq`.
    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        component_property(&self.schema, self.index, name)
    }

    // Компонента — ссылка на место в схеме: та же схема, тот же номер
    // (измерено на компонентах одной схемы).
    fn identity_key(&self) -> Option<(usize, usize)> {
        Some((Rc::as_ptr(&self.schema) as usize, self.index))
    }
}

pub(crate) fn schema_list_value(list: &SchemaListObject) -> BslValue {
    BslValue::new_object(SchemaListObject {
        schema: list.schema.clone(),
        kind: list.kind.clone(),
    })
}

pub(crate) fn schema_list_method_get(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = schema_list_of(receiver, "Получить")?;
    list_lookup(&schema_list_value(list), arguments)
}

pub(crate) fn schema_list_method_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = schema_list_of(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(list.kind.len() as i64))
}

pub(crate) static SCHEMA_LIST_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        names: &["Получить", "Get"],
        call: schema_list_method_get,
    },
    MethodDescriptor {
        names: &["Количество", "Count"],
        call: schema_list_method_count,
    },
];

impl ObjectProtocol for SchemaListObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        list_descriptor(&self.kind)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        SCHEMA_LIST_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        list_get(&self.schema, &self.kind, set_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.kind.len())
    }
}

pub(crate) fn expanded_local_name(
    receiver: &dyn ObjectProtocol,
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let expanded = expanded_of(receiver, "ЛокальноеИмя")?;
    get_property(&name_value(&expanded.name), "ЛокальноеИмя")
}

pub(crate) fn expanded_namespace_uri(
    receiver: &dyn ObjectProtocol,
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let expanded = expanded_of(receiver, "URIПространстваИмен")?;
    get_property(&name_value(&expanded.name), "URIПространстваИмен")
}

pub(crate) static EXPANDED_NAME_PROPERTIES: &[PropertyDescriptor] = &[
    PropertyDescriptor {
        names: &["ЛокальноеИмя", "LocalName"],
        get: expanded_local_name,
        set: None,
    },
    PropertyDescriptor {
        names: &["URIПространстваИмен", "NamespaceURI"],
        get: expanded_namespace_uri,
        set: None,
    },
];

impl ObjectProtocol for ExpandedNameObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &EXPANDED_NAME_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        EXPANDED_NAME_PROPERTIES
    }

    // Расширенное имя — ЗНАЧЕНИЕ: два отдельно построенных имени с
    // одинаковыми URI и локальным именем равны (измерено).
    fn value_eq(&self, other: &bsl_rt::ObjectRef) -> Option<bool> {
        other
            .downcast_ref::<ExpandedNameObject>()
            .map(|other| self.name == other.name)
    }

    // Печатается СОДЕРЖИМЫМ, а не именем типа: `{urn:t}а`, а при пустом
    // URI — одно локальное имя (измерено).
    fn display(&self) -> String {
        self.name.display_text()
    }
}

pub(crate) fn name_list_method_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let list = name_list_of(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(list.names.len() as i64))
}

pub(crate) static NAME_LIST_METHODS: &[MethodDescriptor] = &[MethodDescriptor {
    names: &["Количество", "Count"],
    call: name_list_method_count,
}];

impl ObjectProtocol for NameListObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &NAME_LIST_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        NAME_LIST_METHODS
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        name_list_get(&self.names, set_index(index)?)
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.names.len())
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(!self.names.is_empty())
    }
}
