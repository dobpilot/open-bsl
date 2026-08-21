//! Объектный протокол: типы, таблицы методов, индексация.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТабличныйДокумент",
    type_display: "Табличный документ",
    type_names: &["SpreadsheetDocument"],
};

pub(crate) static AREA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбластьЯчеекТабличногоДокумента",
    type_display: "Область ячеек табличного документа",
    type_names: &["SpreadsheetDocumentRange"],
};

pub(crate) static DRAWINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияРисунковТабличногоДокумента",
    type_display: "Коллекция рисунков табличного документа",
    type_names: &["SpreadsheetDocumentDrawingCollection"],
};

pub(crate) static DRAWING_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "РисунокТабличногоДокумента",
    type_display: "Рисунок табличного документа",
    type_names: &["SpreadsheetDocumentDrawing"],
};

pub(crate) static PARAMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПараметрыМакетаТабличногоДокумента",
    type_display: "Параметры макета табличного документа",
    type_names: &["SpreadsheetDocumentTemplateParameters"],
};

impl SpreadDocumentObject {
    pub(crate) fn as_value(&self) -> BslValue {
        BslValue::new_object(SpreadDocumentObject {
            data: self.data.clone(),
        })
    }
}

impl SpreadAreaObject {
    pub(crate) fn as_value(&self) -> BslValue {
        BslValue::new_object(SpreadAreaObject {
            data: self.data.clone(),
            rect: self.rect,
        })
    }
}

/// Общие методы документа и области: имена делятся получателями, а данные
/// у обоих одни (см. `data`).
/// Получатель нужного типа: чужой получает ту же ошибку «метод не
/// применим», что и прежний строковый путь.
fn document_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r SpreadDocumentObject> {
    receiver
        .downcast_ref::<SpreadDocumentObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: DOCUMENT_TYPE.name,
        })
}

fn area_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r SpreadAreaObject> {
    receiver
        .downcast_ref::<SpreadAreaObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: AREA_TYPE.name,
        })
}

// Три метода — `Область`, `Объединить`, `Разъединить` — есть и у
// документа, и у области: реализация одна, таблицы разные.
fn document_region(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    region(&document_of(receiver, "Область")?.as_value(), arguments)
}

fn document_merge(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    merge_cells(&document_of(receiver, "Объединить")?.as_value()).map(|()| BslValue::Undefined)
}

fn document_unmerge(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    unmerge_cells(&document_of(receiver, "Разъединить")?.as_value()).map(|()| BslValue::Undefined)
}

fn document_write(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    write(&document_of(receiver, "Записать")?.as_value(), arguments)?;
    Ok(BslValue::Undefined)
}

fn document_read(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    read(&document_of(receiver, "Прочитать")?.as_value(), arguments)?;
    Ok(BslValue::Undefined)
}

fn document_output(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    output_with_params(&document_of(receiver, "Вывести")?.as_value(), arguments)
}

fn document_get_area(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    get_area(
        &document_of(receiver, "ПолучитьОбласть")?.as_value(),
        arguments,
    )
}

fn document_clear(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    clear(&document_of(receiver, "Очистить")?.as_value())?;
    Ok(BslValue::Undefined)
}

fn document_begin_row_group(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    begin_row_group(
        &document_of(receiver, "НачатьГруппуСтрок")?.as_value(),
        arguments,
    )?;
    Ok(BslValue::Undefined)
}

fn document_end_row_group(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    end_row_group(&document_of(receiver, "ЗакончитьГруппуСтрок")?.as_value())?;
    Ok(BslValue::Undefined)
}

static DOCUMENT_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["Область", "Area"],
        call: document_region,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["Объединить", "Merge"],
        call: document_merge,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["Разъединить", "Unmerge"],
        call: document_unmerge,
    },
    MethodDescriptor {
        code: MethodCode::new(4),
        names: &["Записать", "Write"],
        call: document_write,
    },
    MethodDescriptor {
        code: MethodCode::new(5),
        names: &["Прочитать", "Read"],
        call: document_read,
    },
    MethodDescriptor {
        code: MethodCode::new(6),
        names: &["Вывести", "Output"],
        call: document_output,
    },
    MethodDescriptor {
        code: MethodCode::new(7),
        names: &["ПолучитьОбласть", "GetArea"],
        call: document_get_area,
    },
    MethodDescriptor {
        code: MethodCode::new(8),
        names: &["Очистить", "Clear"],
        call: document_clear,
    },
    MethodDescriptor {
        code: MethodCode::new(9),
        names: &["НачатьГруппуСтрок", "StartRowGroup"],
        call: document_begin_row_group,
    },
    MethodDescriptor {
        code: MethodCode::new(10),
        names: &["ЗакончитьГруппуСтрок", "EndRowGroup"],
        call: document_end_row_group,
    },
];

fn area_region(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    region(&area_of(receiver, "Область")?.as_value(), arguments)
}

fn area_merge(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    merge_cells(&area_of(receiver, "Объединить")?.as_value()).map(|()| BslValue::Undefined)
}

fn area_unmerge(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    unmerge_cells(&area_of(receiver, "Разъединить")?.as_value()).map(|()| BslValue::Undefined)
}

static AREA_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["Область", "Area"],
        call: area_region,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["Объединить", "Merge"],
        call: area_merge,
    },
    MethodDescriptor {
        code: MethodCode::new(3),
        names: &["Разъединить", "Unmerge"],
        call: area_unmerge,
    },
];

impl ObjectProtocol for SpreadDocumentObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DOCUMENT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        // `Рисунки` и `Параметры` — окна в те же данные.
        if name.eq_ignore_ascii_case("Рисунки") || name.eq_ignore_ascii_case("Drawings") {
            return Ok(BslValue::new_object(SpreadDrawingsObject {
                data: self.data.clone(),
            }));
        }
        if name.eq_ignore_ascii_case("Параметры") || name.eq_ignore_ascii_case("Parameters")
        {
            return Ok(BslValue::new_object(SpreadParamsObject {
                data: self.data.clone(),
            }));
        }
        get_property(&self.as_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_property(&self.as_value(), name, value)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DOCUMENT_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

/// `Вывести` с подстановкой параметров макета: значения из карты
/// параметров источника форматируются `bsl-format` и кладутся в текст
/// ячеек с совпадающим `CellData::parameter` — форматирование живёт
/// здесь, потому что `bsl-format` зависит от `bsl-rt`, а не наоборот.
pub(crate) fn output_with_params(target: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let Some(source) = args.first() else {
        return Err(RtError::MethodNotApplicable {
            method: "Вывести",
            receiver: DOCUMENT_TYPE.name,
        });
    };
    if is_spread_document(source) {
        let params = take_params(source)?;
        if !params.is_empty() {
            let mut formatted: Vec<(String, String)> = Vec::with_capacity(params.len());
            for (name, value, spec) in &params {
                let text = bsl_format::format_value_for_cell(value, spec.as_deref())?;
                formatted.push((name.clone(), text));
            }
            apply_params(source, &formatted)?;
        }
    }
    output(target, args)?;
    Ok(BslValue::Undefined)
}

impl ObjectProtocol for SpreadAreaObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &AREA_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_property(&self.as_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        let receiver = self.as_value();
        // `Значение` и `Расшифровка` — значения ЛЮБОГО типа; в документ
        // уходит их представление по правилам `bsl-format` (измерено).
        if name.eq_ignore_ascii_case("Значение") || name.eq_ignore_ascii_case("Value") {
            return set_value(&receiver, &bsl_format::format_value(&value, None)?);
        }
        if name.eq_ignore_ascii_case("Расшифровка") || name.eq_ignore_ascii_case("Details")
        {
            return set_detail(&receiver, &bsl_format::format_value(&value, None)?);
        }
        set_property(&receiver, name, value)
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        AREA_METHODS
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

fn drawings_of<'r>(
    receiver: &'r dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'r SpreadDrawingsObject> {
    receiver
        .downcast_ref::<SpreadDrawingsObject>()
        .ok_or(RtError::MethodNotApplicable {
            method,
            receiver: DRAWINGS_TYPE.name,
        })
}

fn drawings_method_add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let drawings = drawings_of(receiver, "Добавить")?;
    let receiver = BslValue::new_object(SpreadDrawingsObject {
        data: drawings.data.clone(),
    });
    drawings_add(&receiver, arguments)
}

fn drawings_method_count(
    receiver: &dyn ObjectProtocol,
    _arguments: &[BslValue],
    _c: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let drawings = drawings_of(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(
        drawings.data.borrow().drawings().len() as i64,
    ))
}

static DRAWINGS_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        code: MethodCode::new(1),
        names: &["Добавить", "Add"],
        call: drawings_method_add,
    },
    MethodDescriptor {
        code: MethodCode::new(2),
        names: &["Количество", "Count"],
        call: drawings_method_count,
    },
];

impl ObjectProtocol for SpreadDrawingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DRAWINGS_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DRAWINGS_METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.data.borrow().drawings().len())
    }
}

impl ObjectProtocol for SpreadDrawingObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DRAWING_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        drawing_property(&self.data, self.index, name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_drawing_property(&self.data, self.index, name, &value)
    }
}

impl ObjectProtocol for SpreadParamsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &PARAMS_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        get_param(&self.as_params_value(), name)
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        set_param(&self.as_params_value(), name, value)
    }

    // Параметры макета — источник и приёмник «ЗаполнитьЗначенияСвойств»
    // (измерено): пары отдаются в порядке текста, чужие имена приёмник
    // пропускает.
    fn fill_source_pairs(&self) -> Option<Vec<(String, BslValue)>> {
        let d = self.data.borrow();
        let names = param_names(&d);
        Some(
            names
                .into_iter()
                .map(|name| {
                    let upper = name.to_uppercase();
                    let value = d
                        .params
                        .iter()
                        .find(|(k, _)| **k == upper)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(BslValue::Undefined);
                    (name, value)
                })
                .collect(),
        )
    }

    fn has_property(&self, name: &str) -> bool {
        has_param(&self.data.borrow(), name)
    }

    fn fill_property(&self, name: &str, value: BslValue) -> RtResult<bool> {
        if !self.has_property(name) {
            return Ok(false);
        }
        set_param(&self.as_params_value(), name, value)?;
        Ok(true)
    }
}

impl SpreadParamsObject {
    pub(crate) fn as_params_value(&self) -> BslValue {
        BslValue::new_object(SpreadParamsObject {
            data: self.data.clone(),
        })
    }
}
