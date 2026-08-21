//! Объектный протокол: типы, таблицы методов, индексация.

use super::*;

// --- объектный протокол -----------------------------------------------------

pub(crate) static DOCUMENT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ТабличныйДокумент",
    legacy_type_id: Some(TypeId::SpreadDocument),
};

pub(crate) static AREA_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ОбластьЯчеекТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadArea),
};

pub(crate) static DRAWINGS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "КоллекцияРисунковТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDrawings),
};

pub(crate) static DRAWING_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "РисунокТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDrawing),
};

pub(crate) static PARAMS_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ПараметрыМакетаТабличногоДокумента",
    legacy_type_id: Some(TypeId::SpreadDocParams),
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
pub(crate) fn shared_method(
    receiver: &BslValue,
    method: &str,
    arguments: &[BslValue],
) -> Option<RtResult<BslValue>> {
    let eq =
        |ru: &str, en: &str| method.eq_ignore_ascii_case(ru) || method.eq_ignore_ascii_case(en);
    if eq("Область", "Area") {
        return Some(region(receiver, arguments));
    }
    if eq("Объединить", "Merge") {
        return Some(merge_cells(receiver).map(|()| BslValue::Undefined));
    }
    if eq("Разъединить", "Unmerge") {
        return Some(unmerge_cells(receiver).map(|()| BslValue::Undefined));
    }
    None
}

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

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        let receiver = self.as_value();
        let eq =
            |ru: &str, en: &str| method.eq_ignore_ascii_case(ru) || method.eq_ignore_ascii_case(en);
        if let Some(result) = shared_method(&receiver, method, arguments) {
            return result;
        }
        if eq("Записать", "Write") {
            write(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("Прочитать", "Read") {
            read(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("Вывести", "Output") {
            return output_with_params(&receiver, arguments);
        }
        if eq("ПолучитьОбласть", "GetArea") {
            return get_area(&receiver, arguments);
        }
        if eq("Очистить", "Clear") {
            clear(&receiver)?;
            return Ok(BslValue::Undefined);
        }
        if eq("НачатьГруппуСтрок", "StartRowGroup") {
            begin_row_group(&receiver, arguments)?;
            return Ok(BslValue::Undefined);
        }
        if eq("ЗакончитьГруппуСтрок", "EndRowGroup") {
            end_row_group(&receiver)?;
            return Ok(BslValue::Undefined);
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: DOCUMENT_TYPE.name,
        })
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

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        let receiver = self.as_value();
        if let Some(result) = shared_method(&receiver, method, arguments) {
            return result;
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: AREA_TYPE.name,
        })
    }

    fn is_filled(&self) -> RtResult<bool> {
        Ok(true)
    }
}

impl ObjectProtocol for SpreadDrawingsObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DRAWINGS_TYPE
    }

    fn call_method(
        &self,
        method: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        if method.eq_ignore_ascii_case("Добавить") || method.eq_ignore_ascii_case("Add") {
            let receiver = BslValue::new_object(SpreadDrawingsObject {
                data: self.data.clone(),
            });
            return drawings_add(&receiver, arguments);
        }
        if method.eq_ignore_ascii_case("Количество") || method.eq_ignore_ascii_case("Count")
        {
            return Ok(BslValue::number_from_i64(
                self.data.borrow().drawings().len() as i64,
            ));
        }
        Err(RtError::UnknownMethod {
            method: method.to_string(),
            receiver: DRAWINGS_TYPE.name,
        })
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
