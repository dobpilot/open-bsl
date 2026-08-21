//! Мост к значениям BSL: свойства областей, ячеек и документа.

use super::*;

// --- мост к значениям BSL -------------------------------------------------

pub(crate) fn number(v: &BslValue, what: &str) -> RtResult<i64> {
    match v {
        BslValue::Number(n) => n
            .to_i64_exact()
            .ok_or_else(|| bad(format!("{what}: ожидалось целое число"))),
        _ => Err(bad(format!("{what}: ожидалось число"))),
    }
}

pub(crate) fn int_value(n: i64) -> BslValue {
    BslValue::Number(BslNumber::from_i64(n))
}

/// Число с дробной частью — им наружу отдаются поля страницы
/// (`ТипЗнч(ТабДок.ПолеСлева)` — «Число», измерено).
///
/// `from_f64` отказывает только на нечисле и бесконечности, а в поле
/// попадает лишь проверенное [`number_f64`] либо сотая доля целого из MXL,
/// поэтому запасной ноль недостижим.
pub(crate) fn mm_value(mm: f64) -> BslValue {
    BslValue::Number(BslNumber::from_f64(mm).unwrap_or(BslNumber::ZERO))
}

/// Прочитать число, допуская дробное: `ПолеСлева = 12.7` — законное
/// значение, а `number` требует целого.
///
/// Строка тоже принимается, и это не вольность: платформа на
/// `ПолеСлева = "10"` кладёт в свойство ЧИСЛО 10 (измерено — `ТипЗнч`
/// после присваивания даёт «Число»), на `"12,7"` — 12,7, то есть
/// разделителем дробной части служит ЗАПЯТАЯ, а на `"не число"` отвечает
/// ошибкой. Точка тоже разбирается: у платформы это не проверялось, но
/// отказывать в записи из-за разделителя было бы хуже.
pub(crate) fn number_f64(v: &BslValue, what: &str) -> RtResult<f64> {
    let x = match v {
        BslValue::Number(n) => n.to_f64(),
        BslValue::Str(s) => s
            .to_string()
            .trim()
            .replace(',', ".")
            .parse::<f64>()
            .map_err(|_| bad(format!("{what}: строка не преобразуется в число")))?,
        _ => return Err(bad(format!("{what}: ожидалось число"))),
    };
    if x.is_finite() {
        Ok(x)
    } else {
        Err(bad(format!("{what}: ожидалось конечное число")))
    }
}

/// `ТабличныйДокумент`.
#[derive(Debug)]
pub struct SpreadDocumentObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
}

/// `ОбластьЯчеекТабличногоДокумента` — ссылка на прямоугольник в документе.
#[derive(Debug)]
pub struct SpreadAreaObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
    pub(crate) rect: Rect,
}

/// `КоллекцияРисунковТабличногоДокумента` — окно в тот же документ.
#[derive(Debug)]
pub struct SpreadDrawingsObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
}

/// `РисунокТабличногоДокумента` — документ и номер рисунка в нём.
#[derive(Debug)]
pub struct SpreadDrawingObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
    pub(crate) index: usize,
}

/// `ПараметрыМакетаТабличногоДокумента` — обёртка над теми же данными.
#[derive(Debug)]
pub struct SpreadParamsObject {
    pub(crate) data: Rc<RefCell<SpreadDocData>>,
}

/// Данные документа у значения — и у самого документа, и у его области.
pub(crate) fn data(v: &BslValue) -> Option<Rc<RefCell<SpreadDocData>>> {
    let object = v.object_ref()?;
    if let Some(document) = object.downcast_ref::<SpreadDocumentObject>() {
        return Some(document.data.clone());
    }
    object
        .downcast_ref::<SpreadAreaObject>()
        .map(|area| area.data.clone())
}

pub(crate) fn rect(v: &BslValue) -> Option<Rect> {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<SpreadAreaObject>())
        .map(|area| area.rect)
}

/// Область ячеек — получатель `Значение`, которое ВМ перехватывает.
pub fn is_area(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<SpreadAreaObject>().is_some())
}

/// Положить представление значения в левую верхнюю ячейку области.
///
/// # Errors
///
/// Ошибка, если получатель — не область ячеек.
pub fn set_detail(obj: &BslValue, presentation: &str) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Расшифровка: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .set_cell_detail(rect.r1, rect.c1, presentation);
    Ok(())
}

/// Положить представление значения в левую верхнюю ячейку области.
///
/// # Errors
///
/// Ошибка, если получатель — не область ячеек.
pub fn set_value(obj: &BslValue, presentation: &str) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Значение: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .set_cell_value(rect.r1, rect.c1, presentation);
    Ok(())
}

pub fn is_spread_document(v: &BslValue) -> bool {
    v.object_ref()
        .is_some_and(|object| object.downcast_ref::<SpreadDocumentObject>().is_some())
}

pub fn new_document() -> BslValue {
    BslValue::new_object(SpreadDocumentObject {
        data: Rc::new(RefCell::new(SpreadDocData::new())),
    })
}

/// `Область(СтрокаНач, КолонкаНач, СтрокаКон, КолонкаКон)`. Строковая
/// адресация (`"R1C1:R2C3"`) пока не поддержана — платформа её принимает,
/// но здесь она нужна отдельным разбором.
pub fn region(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = data(obj).ok_or_else(|| bad("Область: не табличный документ"))?;
    let rect = match args {
        [r1, c1, r2, c2] => Rect::from_api(
            number(r1, "Область")?,
            number(c1, "Область")?,
            number(r2, "Область")?,
            number(c2, "Область")?,
        ),
        [address] => match address {
            BslValue::Str(s) => {
                let (h, w) = {
                    let d = doc.borrow();
                    (d.height(), d.width())
                };
                parse_address(&s.to_string(), h, w)
                    .ok_or_else(|| bad(format!("Область не найдена: {s}")))?
            }
            _ => return Err(bad("Область: ожидались координаты или адрес")),
        },
        _ => return Err(bad("Область: ожидалось 1 или 4 аргумента")),
    };
    Ok(BslValue::new_object(SpreadAreaObject { data: doc, rect }))
}

/// Адрес вида `R1C1`, `R1C1:R2C3`, `R2`, `R2:R3`, `C1`, `C1:C2` — набор,
/// который принимает платформа (измерено). Отсутствие оси означает «вся»,
/// и здесь это разворачивается в границы, а не в -1: модель областей у нас
/// прямоугольная.
pub(crate) fn parse_address(s: &str, height: u32, width: u32) -> Option<Rect> {
    fn part(s: &str) -> Option<(Option<i64>, Option<i64>)> {
        let up = s.trim().to_uppercase();
        let (mut row, mut col) = (None, None);
        let mut chars = up.chars().peekable();
        while let Some(c) = chars.next() {
            let mut number = String::new();
            while chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                number.push(chars.next()?);
            }
            let n: i64 = number.parse().ok()?;
            match c {
                'R' | 'С' => row = Some(n),
                'C' => col = Some(n),
                _ => return None,
            }
        }
        if row.is_none() && col.is_none() {
            return None;
        }
        Some((row, col))
    }
    let (start, end) = match s.split_once(':') {
        Some((a, b)) => (part(a)?, part(b)?),
        None => (part(s)?, part(s)?),
    };
    // Отсутствующая ось означает «вся», и разворачивается она по ГРАНИЦАМ
    // документа, а не в бесконечность.
    let kind = match (start.0.is_some(), start.1.is_some()) {
        (true, false) => AreaKind::Rows,
        (false, true) => AreaKind::Columns,
        _ => AreaKind::Rect,
    };
    let mut rect = Rect::from_api(
        start.0.unwrap_or(1),
        start.1.unwrap_or(1),
        end.0.unwrap_or_else(|| i64::from(height.max(1))),
        end.1.unwrap_or_else(|| i64::from(width.max(1))),
    );
    rect.kind = kind;
    Some(rect)
}

/// `ПолучитьОбласть` — КОПИЯ участка как самостоятельный документ.
pub fn get_area(obj: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    let doc = data(obj).ok_or_else(|| bad("ПолучитьОбласть: не табличный документ"))?;
    let (h, w) = {
        let d = doc.borrow();
        (d.height(), d.width())
    };
    let rect = match args {
        [r1, c1, r2, c2] => Rect::from_api(
            number(r1, "ПолучитьОбласть")?,
            number(c1, "ПолучитьОбласть")?,
            number(r2, "ПолучитьОбласть")?,
            number(c2, "ПолучитьОбласть")?,
        ),
        [BslValue::Str(s)] => {
            let name = s.to_string();
            let d = doc.borrow();
            match d.area_named(&name) {
                Some(a) => Rect {
                    r1: a.r1,
                    c1: a.c1,
                    r2: if a.kind == AreaKind::Columns {
                        h.saturating_sub(1)
                    } else {
                        a.r2
                    },
                    c2: if a.kind == AreaKind::Rows {
                        w.saturating_sub(1)
                    } else {
                        a.c2
                    },
                    kind: a.kind,
                },
                None => parse_address(&name, h, w)
                    .ok_or_else(|| bad(format!("Область не найдена: {name}")))?,
            }
        }
        _ => return Err(bad("ПолучитьОбласть: ожидалось 1 или 4 аргумента")),
    };
    let cut = doc.borrow().extract(rect.r1, rect.c1, rect.r2, rect.c2);
    Ok(BslValue::new_object(SpreadDocumentObject {
        data: Rc::new(RefCell::new(cut)),
    }))
}

/// `Вывести(Документ)` — приёмник наращивается вниз. Область ячеек платформа
/// здесь НЕ принимает, и мы тоже.
pub fn output(target: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let target = data(target).ok_or_else(|| bad("Вывести: не табличный документ"))?;
    let source = match args {
        [v] if is_spread_document(v) => data(v).expect("проверено выше"),
        _ => return Err(bad("Вывести: ожидался табличный документ")),
    };
    // Копия снимается ДО заимствования приёмника: `Вывести` самого себя —
    // законный вызов, и без копии он упёрся бы в двойное заимствование.
    let copy = source.borrow().clone();
    target.borrow_mut().append(&copy);
    Ok(())
}

/// `Прочитать(Файл)` — загрузка .mxl в СУЩЕСТВУЮЩИЙ документ, как у
/// платформы: она не отдаёт новый объект, а заменяет содержимое приёмника.
pub fn read(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("Прочитать: не табличный документ"))?;
    let path = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => return Err(bad("Прочитать: ожидался путь к файлу")),
    };
    let bytes = std::fs::read(&path).map_err(|e| bad(format!("не читается {path}: {e}")))?;
    // Формат выбирается по СОДЕРЖИМОМУ, а не по расширению.
    //
    // ОТСТУПЛЕНИЕ ОТ ПЛАТФОРМЫ, намеренное. У неё `Прочитать` берёт .mxl и
    // .xlsx, а макеты живут в конфигурации и достаются `ПолучитьМакет`.
    // Конфигурации у нас нет вовсе, поэтому XML-макет читается тем же
    // методом — иначе до макетов было бы не добраться никак.
    *doc.borrow_mut() = if bytes.starts_with(MXL_SIGNATURE) {
        from_mxl_bytes(&bytes)?
    } else {
        let text = String::from_utf8(bytes)
            .map_err(|_| bad(format!("{path}: не MXL и не текст в UTF-8")))?;
        crate::template::from_template_xml(&text)?
    };
    Ok(())
}

/// `НачатьГруппуСтрок([Имя][, Сворачиваемость])`. Второй аргумент — именно
/// СВОРАЧИВАЕМОСТЬ: `Ложь` даёт свёрнутую группу (измерено).
/// `Рисунки.Добавить(ТипРисунка)`. Поддержан только прямоугольник:
/// остальные типы платформа знает, но их содержимое — отдельные разделы
/// формата, которых здесь нет.
/// `Рисунки.Добавить(ТипРисунка)`.
pub fn drawings_add(obj: &BslValue, _args: &[BslValue]) -> RtResult<BslValue> {
    let doc = match obj
        .object_ref()
        .and_then(|object| object.downcast_ref::<SpreadDrawingsObject>())
    {
        Some(drawings) => drawings.data.clone(),
        None => return Err(bad("Добавить: не коллекция рисунков")),
    };
    let number_of = doc.borrow_mut().add_drawing(0.0, 0.0, 0.0, 0.0);
    Ok(BslValue::new_object(SpreadDrawingObject {
        data: doc,
        index: number_of - 1,
    }))
}

/// Свойства рисунка. Геометрия отдаётся в миллиметрах — но НЕ теми, что
/// задали: она защёлкивается на сетку четвертей пункта, поэтому 10 мм
/// читаются обратно как 9,96597… (измерено на платформе, и здесь так же).
pub fn drawing_property(
    doc: &Rc<RefCell<SpreadDocData>>,
    i: usize,
    name: &str,
) -> RtResult<BslValue> {
    let d = doc.borrow();
    let drawing = d
        .drawings()
        .get(i)
        .ok_or_else(|| bad("рисунок уже удалён"))?;
    // Значение защёлкивается на сетку и отдаётся с четырнадцатью знаками
    // после запятой — столько же печатает платформа.
    let mm = |v: f64| -> RtResult<BslValue> {
        let qp = (v * MM_TO_QP).round() as i128;
        let mantissa = (f64::from(qp as i32) * 25.4 / 288.0 * 1e14).round() as i128;
        Ok(BslValue::Number(BslNumber::from_parts(mantissa, 14)?))
    };
    match () {
        _ if folded_eq(name, "Имя") || folded_eq(name, "Name") => {
            Ok(BslValue::Str(BslString::from_str(&drawing.name)))
        }
        _ if folded_eq(name, "Лево") || folded_eq(name, "Left") => mm(drawing.left),
        _ if folded_eq(name, "Верх") || folded_eq(name, "Top") => mm(drawing.top),
        _ if folded_eq(name, "Ширина") || folded_eq(name, "Width") => mm(drawing.width),
        _ if folded_eq(name, "Высота") || folded_eq(name, "Height") => mm(drawing.height),
        _ => Err(RtError::UnknownColumn(name.to_string())),
    }
}

pub fn set_drawing_property(
    doc: &Rc<RefCell<SpreadDocData>>,
    i: usize,
    name: &str,
    val: &BslValue,
) -> RtResult<()> {
    if folded_eq(name, "Имя") || folded_eq(name, "Name") {
        let name = val.to_string();
        let mut d = doc.borrow_mut();
        if let Some(drawing) = d.drawings_mut().get_mut(i) {
            drawing.name = name;
        }
        return Ok(());
    }
    let number = match val {
        BslValue::Number(n) => n
            .to_string()
            .replace(',', ".")
            .parse::<f64>()
            .unwrap_or(0.0),
        _ => return Err(bad("геометрия рисунка: ожидалось число")),
    };
    let mut d = doc.borrow_mut();
    {
        let Some(drawing) = d.drawings_mut().get_mut(i) else {
            return Ok(());
        };
        match () {
            _ if folded_eq(name, "Лево") || folded_eq(name, "Left") => drawing.left = number,
            _ if folded_eq(name, "Верх") || folded_eq(name, "Top") => drawing.top = number,
            _ if folded_eq(name, "Ширина") || folded_eq(name, "Width") => {
                drawing.width = number
            }
            _ if folded_eq(name, "Высота") || folded_eq(name, "Height") => {
                drawing.height = number
            }
            _ => return Err(RtError::UnknownColumn(name.to_string())),
        }
    }
    d.refresh_drawing_bounds(i);
    Ok(())
}

pub fn begin_row_group(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("НачатьГруппуСтрок: не табличный документ"))?;
    let name = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => String::new(),
    };
    let collapsed = matches!(args.get(1), Some(BslValue::Boolean(false)));
    doc.borrow_mut().begin_row_group(&name, collapsed);
    Ok(())
}

pub fn end_row_group(obj: &BslValue) -> RtResult<()> {
    data(obj)
        .ok_or_else(|| bad("ЗакончитьГруппуСтрок: не табличный документ"))?
        .borrow_mut()
        .end_row_group();
    Ok(())
}

pub fn clear(obj: &BslValue) -> RtResult<()> {
    data(obj)
        .ok_or_else(|| bad("Очистить: не табличный документ"))?
        .borrow_mut()
        .clear();
    Ok(())
}

/// `Записать(Файл [, ТипФайла])`. Без типа — MXL (измерено).
pub fn write(obj: &BslValue, args: &[BslValue]) -> RtResult<()> {
    let doc = data(obj).ok_or_else(|| bad("Записать: не табличный документ"))?;
    let path = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => return Err(bad("Записать: ожидался путь к файлу")),
    };
    let kind = match args.get(1) {
        None => FileKind::Mxl,
        Some(BslValue::Enum(e)) => match e {
            bsl_rt::EnumValue::SpreadFileMxl => FileKind::Mxl,
            bsl_rt::EnumValue::SpreadFileTxt => FileKind::Txt,
            bsl_rt::EnumValue::SpreadFileXlsx => FileKind::Xlsx,
            bsl_rt::EnumValue::SpreadFilePdf => FileKind::Pdf,
            _ => return Err(bad("Записать: неподдерживаемый тип файла")),
        },
        Some(_) => return Err(bad("Записать: ожидался ТипФайлаТабличногоДокумента")),
    };
    let d = doc.borrow();
    write_file(&d, &path, kind)
}

pub fn merge_cells(obj: &BslValue) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Объединить: не область ячеек"))?;
    let doc = data(obj).expect("область всегда при документе");
    let mut d = doc.borrow_mut();
    // Вид области решает, в КАКОЙ из трёх списков ляжет объединение.
    match rect.kind {
        AreaKind::Rows => d.merge_rows(rect.r1, rect.r2),
        AreaKind::Columns => d.merge_columns(rect.c1, rect.c2),
        AreaKind::Rect => d.merge(Merge::new(rect.r1, rect.c1, rect.r2, rect.c2)),
    }
    Ok(())
}

pub fn unmerge_cells(obj: &BslValue) -> RtResult<()> {
    let rect = rect(obj).ok_or_else(|| bad("Разъединить: не область ячеек"))?;
    data(obj)
        .expect("область всегда при документе")
        .borrow_mut()
        .unmerge(rect.r1, rect.c1, rect.r2, rect.c2);
    Ok(())
}

/// Чтение свойства документа или области.
pub fn get_property(obj: &BslValue, name: &str) -> RtResult<BslValue> {
    let doc = data(obj).ok_or(RtError::NotAnObject)?;
    let d = doc.borrow();
    if let Some(rect) = rect(obj) {
        return match () {
            _ if folded_eq(name, "Текст") || folded_eq(name, "Text") => {
                // У ячейки со значением `Текст` отдаёт его ПРЕДСТАВЛЕНИЕ —
                // измерено: после `Значение = 42` текст равен «42».
                let text = d
                    .cell_value(rect.r1, rect.c1)
                    .unwrap_or_else(|| d.cell_text(rect.r1, rect.c1));
                Ok(BslValue::Str(BslString::from_str(&text)))
            }
            _ if folded_eq(name, "Расшифровка") || folded_eq(name, "Details") => {
                Ok(BslValue::Str(BslString::from_str(
                    &d.cell_detail(rect.r1, rect.c1).unwrap_or_default(),
                )))
            }
            _ if folded_eq(name, "ПараметрРасшифровки") || folded_eq(name, "DetailsParameter") => {
                Ok(BslValue::Str(BslString::from_str(
                    &d.cell_detail_param(rect.r1, rect.c1),
                )))
            }
            _ if folded_eq(name, "СодержитЗначение") || folded_eq(name, "ContainsValue") => {
                Ok(BslValue::Boolean(d.cell_value(rect.r1, rect.c1).is_some()))
            }
            _ if folded_eq(name, "Параметр") || folded_eq(name, "Parameter") => {
                // У обычного документа платформа отдаёт параметр ПУСТЫМ даже
                // из своего файла (измерено), поэтому здесь тоже пусто.
                Ok(BslValue::Str(BslString::from_str("")))
            }
            _ if folded_eq(name, "Имя") || folded_eq(name, "Name") => {
                let name = d
                    .names_iter()
                    .find(|(_, a)| a.r1 == rect.r1 && a.c1 == rect.c1)
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default();
                Ok(BslValue::Str(BslString::from_str(&name)))
            }
            _ => Err(RtError::UnknownColumn(name.to_string())),
        };
    }
    match () {
        _ if folded_eq(name, "ВысотаТаблицы") || folded_eq(name, "TableHeight") => {
            Ok(int_value(i64::from(d.height())))
        }
        _ if folded_eq(name, "ШиринаТаблицы") || folded_eq(name, "TableWidth") => {
            Ok(int_value(i64::from(d.width())))
        }
        _ if folded_eq(name, "ОтображатьСетку") || folded_eq(name, "ShowGrid") => {
            Ok(BslValue::Boolean(d.show_grid))
        }
        _ if folded_eq(name, "ФиксацияСверху") || folded_eq(name, "FixedTop") => {
            Ok(int_value(d.fix_top))
        }
        _ if folded_eq(name, "ФиксацияСлева") || folded_eq(name, "FixedLeft") => {
            Ok(int_value(d.fix_left))
        }
        // Поля страницы — в миллиметрах, умолчание 10 у каждого (измерено
        // на пустом документе 8.3.27).
        // Английские написания измерены перебором в
        // `tests/conformance/measure/measure-pdf-write.bsl`: платформа
        // знает `LeftMargin` и его собратьев, а `FieldLeft`, `MarginLeft` и
        // `FieldOnLeft` отвергает — все три пробы дали ошибку.
        _ if folded_eq(name, "ПолеСлева") || folded_eq(name, "LeftMargin") => {
            Ok(mm_value(d.margins.left))
        }
        _ if folded_eq(name, "ПолеСправа") || folded_eq(name, "RightMargin") => {
            Ok(mm_value(d.margins.right))
        }
        _ if folded_eq(name, "ПолеСверху") || folded_eq(name, "TopMargin") => {
            Ok(mm_value(d.margins.top))
        }
        _ if folded_eq(name, "ПолеСнизу") || folded_eq(name, "BottomMargin") => {
            Ok(mm_value(d.margins.bottom))
        }
        _ if folded_eq(name, "ОриентацияСтраницы") || folded_eq(name, "PageOrientation") => {
            Ok(BslValue::Enum(if d.landscape {
                bsl_rt::EnumValue::PageOrientationLandscape
            } else {
                bsl_rt::EnumValue::PageOrientationPortrait
            }))
        }
        _ => Err(RtError::UnknownColumn(name.to_string())),
    }
}

/// Запись свойства документа или области.
pub fn set_property(obj: &BslValue, name: &str, val: BslValue) -> RtResult<()> {
    let doc = data(obj).ok_or(RtError::NotAnObject)?;
    if let Some(rect) = rect(obj) {
        let mut d = doc.borrow_mut();
        if folded_eq(name, "Текст") || folded_eq(name, "Text") {
            let text = val.to_string();
            for r in rect.r1..=rect.r2 {
                for c in rect.c1..=rect.c2 {
                    d.set_cell_text(r, c, &text);
                }
            }
            return Ok(());
        }
        if folded_eq(name, "ПараметрРасшифровки") || folded_eq(name, "DetailsParameter")
        {
            d.set_cell_detail_param(rect.r1, rect.c1, &val.to_string());
            return Ok(());
        }
        if folded_eq(name, "СодержитЗначение") || folded_eq(name, "ContainsValue") {
            // Платформа держит это отдельным переключателем: пока он не
            // взведён, `Значение` не пишется вовсе (измерено — «Поле объекта
            // недоступно для записи»). Взведение само по себе кладёт в
            // ячейку пустое значение.
            if matches!(val, BslValue::Boolean(true)) {
                if d.cell_value(rect.r1, rect.c1).is_none() {
                    d.set_cell_value(rect.r1, rect.c1, "");
                }
            } else {
                d.set_cell_text(rect.r1, rect.c1, "");
            }
            return Ok(());
        }
        if folded_eq(name, "Параметр") || folded_eq(name, "Parameter") {
            d.set_cell_parameter(rect.r1, rect.c1, &val.to_string());
            return Ok(());
        }
        if folded_eq(name, "Имя") || folded_eq(name, "Name") {
            let name = val.to_string();
            if name.is_empty() {
                let old: Vec<String> = d
                    .names_iter()
                    .filter(|(_, a)| a.r1 == rect.r1 && a.c1 == rect.c1)
                    .map(|(n, _)| n.clone())
                    .collect();
                for n in old {
                    d.clear_area_name(&n);
                }
            } else {
                d.set_area_name(&name, NamedArea::rect(rect.r1, rect.c1, rect.r2, rect.c2));
            }
            return Ok(());
        }
        if folded_eq(name, "ШиринаКолонки") || folded_eq(name, "ColumnWidth") {
            let width = number(&val, "ШиринаКолонки")?;
            for c in rect.c1..=rect.c2 {
                d.set_col_width(c, width);
            }
            return Ok(());
        }
        if folded_eq(name, "ВысотаСтроки") || folded_eq(name, "RowHeight") {
            let height = number(&val, "ВысотаСтроки")?;
            for r in rect.r1..=rect.r2 {
                d.set_row_height(r, height);
            }
            return Ok(());
        }
        return Err(RtError::UnknownColumn(name.to_string()));
    }
    let mut d = doc.borrow_mut();
    if folded_eq(name, "ОтображатьСетку") || folded_eq(name, "ShowGrid") {
        d.show_grid = matches!(val, BslValue::Boolean(true));
        return Ok(());
    }
    if folded_eq(name, "ФиксацияСверху") || folded_eq(name, "FixedTop") {
        d.fix_top = number(&val, "ФиксацияСверху")?;
        return Ok(());
    }
    if folded_eq(name, "ФиксацияСлева") || folded_eq(name, "FixedLeft") {
        d.fix_left = number(&val, "ФиксацияСлева")?;
        return Ok(());
    }
    // Поля страницы в миллиметрах. Значение принимается как есть, без
    // ограничения снизу: платформа его тоже не поджимает (измерено —
    // `ПолеСлева = -5` читается обратно как -5, а 500 как 500). Не измерено
    // другое — как отрицательное поле ложится в РАСКЛАДКУ её печати; тихо
    // подменять пользовательское число из-за этого нельзя.
    for (ru, en, field) in [
        ("ПолеСлева", "LeftMargin", 0),
        ("ПолеСправа", "RightMargin", 1),
        ("ПолеСверху", "TopMargin", 2),
        ("ПолеСнизу", "BottomMargin", 3),
    ] {
        if folded_eq(name, ru) || folded_eq(name, en) {
            let mm = number_f64(&val, ru)?;
            let margins = &mut d.margins;
            match field {
                0 => margins.left = mm,
                1 => margins.right = mm,
                2 => margins.top = mm,
                _ => margins.bottom = mm,
            }
            return Ok(());
        }
    }
    if folded_eq(name, "ОриентацияСтраницы") || folded_eq(name, "PageOrientation")
    {
        d.landscape = match val {
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationLandscape) => true,
            BslValue::Enum(bsl_rt::EnumValue::PageOrientationPortrait) => false,
            _ => return Err(bad("ОриентацияСтраницы: ожидался член ОриентацияСтраницы")),
        };
        return Ok(());
    }
    Err(RtError::UnknownColumn(name.to_string()))
}
