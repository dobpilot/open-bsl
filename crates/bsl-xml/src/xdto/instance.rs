//! Экземпляр объекта, `СписокXDTO` и `ПоследовательностьXDTO`.

use super::*;

// --- экземпляр -----------------------------------------------------------

/// Хранилище экземпляра `ОбъектXDTO`.
///
/// Слоты здесь параллельны не списку свойств типа, а ПОРЯДКУ ЗАПОЛНЕНИЯ:
/// хранится список записей «свойство — значение». Так устроено потому, что
/// именно этот порядок платформа показывает через `Последовательность()`
/// (измерено: `О.cb.Добавить("аб"); О.ca = "вг"; О.cb.Добавить("де")` даёт
/// `[cb=аб][ca=вг][cb=де]`), а повторная запись одиночного свойства своё
/// место в нём СОХРАНЯЕТ (измерено: `О.ca = "вг"; О.cb.Добавить("аб");
/// О.ca = "де"` -> `[ca=де][cb=аб]`, а после `Сбросить` то же свойство
/// уходит в конец). Массив слотов по числу свойств этот порядок потерял бы,
/// а он наблюдаем.
#[derive(Debug)]
pub struct XdtoObjectData {
    pub(crate) model: Rc<XdtoModel>,
    pub(crate) type_index: usize,
    /// `Владелец()` — объект, в свойство которого этот экземпляр записан
    /// (измерено: `О.anon = А; А.Владелец() = О` — «Да», у отдельно
    /// созданного — `Неопределено`). Ссылка СЛАБАЯ: владелец держит этот
    /// экземпляр в своём хранилище, и сильная обратная ссылка замкнула бы
    /// `Rc` в кольцо, которого счётчик ссылок не разбирает.
    pub(crate) owner: RefCell<Weak<XdtoObjectData>>,
    pub(crate) entries: RefCell<Vec<XdtoEntry>>,
}

/// Одно заполнение свойства. Множественное свойство — это несколько
/// записей с одним и тем же `prop`.
#[derive(Debug)]
pub(crate) struct XdtoEntry {
    /// Номер свойства в [`XdtoModel::properties`].
    pub(crate) prop: usize,
    pub(crate) value: BslValue,
}

impl XdtoObjectData {
    pub(crate) fn type_data(&self) -> RtResult<&XdtoTypeData> {
        self.model.type_at(self.type_index)
    }

    /// Номер свойства по имени: поиск идёт по СПЛЮЩЕННОМУ списку типа и
    /// регистра не различает (измерено: `О.NAME` читает `name`).
    pub(crate) fn property_by_name(&self, name: &str) -> RtResult<Option<usize>> {
        for p in &self.type_data()?.properties {
            if bsl_rt::fold::folded_eq(&self.model.property_at(*p)?.name, name) {
                return Ok(Some(*p));
            }
        }
        Ok(None)
    }

    /// Номера записей хранилища, относящихся к свойству, в порядке
    /// заполнения.
    pub(crate) fn occurrences(&self, prop: usize) -> Vec<usize> {
        self.entries
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.prop == prop)
            .map(|(i, _)| i)
            .collect()
    }

    /// Чтение ОДИНОЧНОГО свойства: последнее заполнение, а если его нет —
    /// значение по умолчанию из `default`/`fixed` (измерено: у свежего
    /// объекта `def` -> 7, `fx` -> 9, `color` -> «red», а свойство без
    /// того и другого -> `Неопределено`).
    pub(crate) fn single_value(&self, prop: usize) -> RtResult<BslValue> {
        if let Some(e) = self.entries.borrow().iter().rev().find(|e| e.prop == prop) {
            return Ok(e.value.clone());
        }
        Ok(match &self.model.property_at(prop)?.default {
            Some(v) => v.value.clone(),
            None => BslValue::Undefined,
        })
    }
}

/// Множественное ли свойство: верхняя граница, отличная от единицы.
/// Измерено на границах `0..-1` (`code`) и `1..5` (`many5`) — оба дают
/// `СписокXDTO`, а `1..1` и `0..1` — само значение.
pub(crate) fn is_multiple(prop: &XdtoPropertyData) -> bool {
    prop.upper != Some(1)
}

/// Хранилище получателя-экземпляра.
pub(crate) fn instance_of<'a>(
    obj: &'a dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'a Rc<XdtoObjectData>> {
    match repr_of_object(obj) {
        Some(XdtoRepr::Object(data)) => Ok(data),
        _ => Err(not_applicable(obj, method)),
    }
}

/// Хранилище и свойство получателя-списка.
pub(crate) fn list_of<'a>(
    obj: &'a dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<(&'a Rc<XdtoObjectData>, usize)> {
    match repr_of_object(obj) {
        Some(XdtoRepr::List(data, prop)) => Ok((data, *prop)),
        _ => Err(not_applicable(obj, method)),
    }
}

/// Свойство, названное аргументом: платформа принимает и имя строкой, и
/// само `СвойствоXDTO` (измерено на `Получить`, `Установлено` и
/// `Сбросить`). Постороннее имя — ошибка, а не `Неопределено` (измерено).
pub(crate) fn property_arg(
    data: &XdtoObjectData,
    arg: &BslValue,
    method: &'static str,
) -> RtResult<usize> {
    let unknown = |name: String| {
        RtError::Xdto(format!(
            "у типа «{}» нет свойства «{name}»",
            data.type_data()
                .map(type_display)
                .unwrap_or_else(|_| String::new())
        ))
    };
    match arg {
        BslValue::Str(s) => {
            let name = s.to_string();
            data.property_by_name(&name)?.ok_or_else(|| unknown(name))
        }
        BslValue::Object(_) => match repr_of(arg) {
            Some(XdtoRepr::Property(model, index)) => {
                // Чужое свойство — из другой модели или из другого типа —
                // не годится: хранилище адресуется номерами СВОЕЙ модели.
                if !Rc::ptr_eq(model, &data.model) || !data.type_data()?.properties.contains(index)
                {
                    return Err(unknown(
                        model
                            .property_at(*index)
                            .map(|p| p.name.clone())
                            .unwrap_or_default(),
                    ));
                }
                Ok(*index)
            }
            _ => Err(RtError::MethodNotApplicable {
                method,
                receiver: arg.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method,
            receiver: arg.type_name(),
        }),
    }
}

/// `anyType` ли это — единственный тип, который принимает в свойство любое
/// значение как есть (измерено на `notype`: строка остаётся строкой, число
/// числом, `ОбъектXDTO` объектом).
pub(crate) fn is_any_type(data: &XdtoTypeData) -> bool {
    data.shape.is_none() && data.base.is_none()
}

/// Наследник ли `candidate` типу `target` — цепочкой базовых типов.
pub(crate) fn derives_from(model: &XdtoModel, candidate: usize, target: usize) -> bool {
    let mut cur = candidate;
    // Длина модели — верхняя граница цепочки и заодно страховка от кольца
    // в испорченной схеме.
    for _ in 0..=model.types.len() {
        if cur == target {
            return true;
        }
        match model.types.get(cur).and_then(|t| t.base) {
            Some(base) => cur = base,
            None => return false,
        }
    }
    false
}

/// Приведение значения к типу свойства — то, что платформа делает при
/// записи.
///
/// Правило одно на все измеренные случаи: значение переводится в
/// ЛЕКСИЧЕСКУЮ ФОРМУ типа-приёмника и разбирается обратно. Отсюда и
/// `О.name = 5` -> строка «5», и `О.name = Дата(2026,8,13)` ->
/// «2026-08-13T00:00:00», и `О.name = Истина` -> «true», и отказы: «true»
/// не разбирается как `xs:int` (`О.id = Истина` — ошибка), «1.5» тоже
/// (`О.id = 1.5` — ошибка), а `О.id = "5"` даёт число 5. Объединение
/// выбирает член по той же лексической форме (измерено: `5` и «5» дают
/// число, «аб» — строку).
pub(crate) fn coerce_to_property(
    owner: &Rc<XdtoObjectData>,
    prop: usize,
    value: BslValue,
) -> RtResult<BslValue> {
    let model = &owner.model;
    let target = model.property_at(prop)?.type_index;
    let data = model.type_at(target)?;
    if data.shape.is_none() {
        // `anyType` принимает что угодно (измерено), остальные типы
        // объектов — только экземпляр своего типа или его наследника
        // (измерено: `ExtType` в свойство типа `RootType` проходит,
        // `EmptyType` — ошибка).
        if is_any_type(data) {
            return Ok(value);
        }
        let Some(XdtoRepr::Object(inst)) = repr_of(&value) else {
            return Err(bad_property_value(model, target, &value));
        };
        if !Rc::ptr_eq(&inst.model, model) || !derives_from(model, inst.type_index, target) {
            return Err(bad_property_value(model, target, &value));
        }
        *inst.owner.borrow_mut() = Rc::downgrade(owner);
        return Ok(value);
    }
    // `ЗначениеXDTO` в свойство простого типа платформа принимает и берёт
    // из него ЗНАЧЕНИЕ, а не лексическую форму: `О.id = Создать(xs:string,
    // "5")` дало число 5, хотя типы источника и приёмника разные
    // (измерено).
    let value = match repr_of(&value) {
        Some(XdtoRepr::Value(_, v)) => v.value.clone(),
        _ => value,
    };
    coerce_to_value_type(model, target, value)
}

/// Значение простого типа из значения BSL — лексический круг из
/// [`coerce_to_property`].
pub(crate) fn coerce_to_value_type(
    model: &Rc<XdtoModel>,
    target: usize,
    value: BslValue,
) -> RtResult<BslValue> {
    let builtin = model.builtin_of(target);
    // Двоичные данные и расширенное имя проходят без круга: обратной
    // лексической формы у них здесь нет (см. «Двоичные лексические формы» и
    // «QName с префиксом» в шапке модуля), а значение уже нужного вида.
    // Фасеты им всё равно достаются — по цепочке типа и по самому
    // значению: длина двоичных данных считается в байтах и без лексической
    // формы.
    if let BslValue::Object(o) = &value {
        let is_expanded_name = value.object_ref().is_some_and(|object| {
            object
                .downcast_ref::<crate::xsd::ExpandedNameObject>()
                .is_some()
        });
        match (builtin, &**o) {
            (Some(BuiltinBsl::Base64 | BuiltinBsl::Hex), BslObject::BinaryData(_)) => {
                check_facet_chain(model, Some(target), "", &value)?;
                return Ok(value);
            }
            (Some(BuiltinBsl::QName), _) if is_expanded_name => {
                check_facet_chain(model, Some(target), "", &value)?;
                return Ok(value);
            }
            _ => {}
        }
    }
    let lexical = lexical_of_value(&value, builtin)
        .ok_or_else(|| bad_property_value(model, target, &value))?;
    // С проверкой по фасетам: платформа не пускает нарушающее значение уже
    // в запись (измерено на присваивании, на `Установить` и на
    // `Добавить`/`Установить`/`Вставить` списка).
    value_from_lexical_checked(model, target, &lexical)
}

/// Лексическая форма значения BSL для типа-приёмника. `None` — значение,
/// у которого лексической формы нет вовсе: `Неопределено`, `Null` и любой
/// объект (измерено: запись `Неопределено`, `Null` и `ОбъектXDTO` в
/// свойство простого типа — ошибка).
pub(crate) fn lexical_of_value(value: &BslValue, target: Option<BuiltinBsl>) -> Option<String> {
    Some(match value {
        BslValue::Str(s) => s.to_string(),
        BslValue::Number(n) => n.to_canonical(),
        BslValue::Boolean(b) => (if *b { "true" } else { "false" }).to_string(),
        BslValue::Date(d) => {
            let c = d.to_civil();
            match target {
                // У приёмника-даты и приёмника-времени формы свои, у всех
                // остальных (в том числе у `xs:string`) — полная запись
                // `dateTime` (измерено: `О.name = Дата(2026,8,13)` дало
                // «2026-08-13T00:00:00»).
                Some(BuiltinBsl::Date) => format!("{:04}-{:02}-{:02}", c.year, c.month, c.day),
                Some(BuiltinBsl::Time) => {
                    format!("{:02}:{:02}:{:02}", c.hour, c.minute, c.second)
                }
                _ => format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                    c.year, c.month, c.day, c.hour, c.minute, c.second
                ),
            }
        }
        _ => return None,
    })
}

pub(crate) fn bad_property_value(
    model: &Rc<XdtoModel>,
    target: usize,
    value: &BslValue,
) -> RtError {
    let name = model
        .type_at(target)
        .map(type_display)
        .unwrap_or_else(|_| String::new());
    RtError::Xdto(format!(
        "значение типа «{}» не годится свойству типа «{name}»",
        value.type_name()
    ))
}

/// Запись ОДИНОЧНОГО свойства: место в порядке заполнения сохраняется, а
/// незаполненное свойство встаёт в конец (измерено обе стороны).
pub(crate) fn set_single(data: &Rc<XdtoObjectData>, prop: usize, value: BslValue) -> RtResult<()> {
    let coerced = coerce_to_property(data, prop, value)?;
    let mut entries = data.entries.borrow_mut();
    match entries.iter_mut().rev().find(|e| e.prop == prop) {
        Some(e) => e.value = coerced,
        None => entries.push(XdtoEntry {
            prop,
            value: coerced,
        }),
    }
    Ok(())
}

/// Чтение свойства экземпляра.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если у типа нет такого свойства (измерено:
/// постороннее имя — ошибка, а не `Неопределено`).
pub(crate) fn object_get_property(data: &Rc<XdtoObjectData>, name: &str) -> RtResult<BslValue> {
    let Some(prop) = data.property_by_name(name)? else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    if is_multiple(data.model.property_at(prop)?) {
        return Ok(shell_value(XdtoRepr::List(data.clone(), prop)));
    }
    data.single_value(prop)
}

/// Запись свойства экземпляра.
///
/// # Errors
///
/// [`RtError::UnknownColumn`], если такого свойства нет; [`RtError::Xdto`],
/// если свойство множественное (оно наполняется через `СписокXDTO` —
/// измерено, что присваивание в него ошибка) либо значение не приводится к
/// типу свойства.
pub fn set_property(obj: &dyn ObjectProtocol, name: &str, value: BslValue) -> RtResult<()> {
    let data = instance_of(obj, "Установить")?;
    let Some(prop) = data.property_by_name(name)? else {
        return Err(RtError::UnknownColumn(name.to_string()));
    };
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{name}» наполняется через «СписокXDTO», \
             а не присваиванием"
        )));
    }
    set_single(data, prop, value)
}

/// `ОбъектXDTO.Получить(Имя|Свойство)` — только ОДИНОЧНОЕ свойство:
/// множественное платформа через `Получить` не отдаёт (измерено), для него
/// есть `ПолучитьСписок`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] на чужом получателе или аргументе,
/// [`RtError::Xdto`], если свойства нет или оно множественное.
pub fn object_get(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Получить")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Получить"));
    };
    let prop = property_arg(data, arg, "Получить")?;
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{}» читается методом «ПолучитьСписок»",
            data.model.property_at(prop)?.name
        )));
    }
    data.single_value(prop)
}

/// `ОбъектXDTO.Установить(Имя|Свойство, Значение)` — тоже только
/// одиночное свойство (измерено: `Установить("code", ...)` — ошибка).
///
/// # Errors
///
/// Те же, что у [`set_property`].
pub fn object_set(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Установить")?;
    let [arg, value] = args else {
        return Err(not_applicable(obj, "Установить"));
    };
    let prop = property_arg(data, arg, "Установить")?;
    if is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "множественное свойство «{}» наполняется через «СписокXDTO»",
            data.model.property_at(prop)?.name
        )));
    }
    set_single(data, prop, value.clone())?;
    Ok(BslValue::Undefined)
}

/// `ОбъектXDTO.ПолучитьСписок(Имя|Свойство)` — тот же список, что отдаёт
/// чтение множественного свойства (измерено: `О.ПолучитьСписок("code")
/// .Добавить(...)` видно через `О.code`). У одиночного свойства списка нет
/// (измерено — ошибка).
///
/// # Errors
///
/// [`RtError::Xdto`], если свойства нет или оно одиночное.
pub fn object_get_list(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "ПолучитьСписок")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "ПолучитьСписок"));
    };
    let prop = property_arg(data, arg, "ПолучитьСписок")?;
    if !is_multiple(data.model.property_at(prop)?) {
        return Err(RtError::Xdto(format!(
            "свойство «{}» одиночное, списка у него нет",
            data.model.property_at(prop)?.name
        )));
    }
    Ok(shell_value(XdtoRepr::List(data.clone(), prop)))
}

/// `ОбъектXDTO.Установлено(Имя|Свойство)`.
///
/// Заполненность — это ЗАПИСЬ в хранилище, а не наличие значения при
/// чтении: у свежего объекта `Установлено("def")` — «Нет», хотя `О.def`
/// отдаёт 7 из `default` (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если у типа нет такого свойства.
pub fn object_is_set(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Установлено")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Установлено"));
    };
    let prop = property_arg(data, arg, "Установлено")?;
    Ok(BslValue::Boolean(!data.occurrences(prop).is_empty()))
}

/// `ОбъектXDTO.Сбросить(Имя|Свойство)` — забыть заполнение: после сброса
/// свойство с `default` снова отдаёт значение по умолчанию, а
/// множественное становится пустым (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если у типа нет такого свойства.
pub fn object_unset(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Сбросить")?;
    let [arg] = args else {
        return Err(not_applicable(obj, "Сбросить"));
    };
    let prop = property_arg(data, arg, "Сбросить")?;
    data.entries.borrow_mut().retain(|e| e.prop != prop);
    Ok(BslValue::Undefined)
}

/// `ОбъектXDTO.Свойства()` — свойства СВОЕГО ТИПА, та же коллекция, что и
/// `Тип().Свойства` (измерено: длины совпадают — 14 и 14).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами.
pub fn object_properties(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Свойства")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Свойства"));
    }
    data.type_data()?;
    Ok(shell_value(XdtoRepr::Properties(
        data.model.clone(),
        data.type_index,
    )))
}

/// `ОбъектXDTO.Владелец()` — объект, в свойство которого этот записан;
/// у отдельно созданного — `Неопределено` (измерено обе стороны).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами. У `СписокXDTO` и `ПоследовательностьXDTO`
/// владелец — ЧЛЕН, а не метод (измерено: `Список.Владелец()` — ошибка),
/// поэтому они сюда не попадают.
pub fn object_owner(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Владелец")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Владелец"));
    }
    Ok(match data.owner.borrow().upgrade() {
        Some(owner) => instance_value(&owner),
        None => BslValue::Undefined,
    })
}

/// `ОбъектXDTO.Проверить()` — проверка ГРАНИЦ ВХОЖДЕНИЯ, своих и
/// вложенных.
///
/// Измерено четырьмя пробами: пустой тип проходит; `RootType` без
/// обязательных свойств отвергается; заполненный, но с пустым вложенным
/// объектом — отвергается, а с заполненным — проходит; шесть значений в
/// свойстве `1..5` — отвергается. Отсюда и правило: у каждого свойства
/// число заполнений обязано лежать между `НижняяГраница` и
/// `ВерхняяГраница`, а вложенные объекты — и в свойстве, и в списке —
/// проверяются рекурсивно.
///
/// ФАСЕТЫ здесь не перепроверяются, и измерить, перепроверяет ли их
/// платформа, не вышло: значение, нарушающее фасет, в экземпляр не
/// попадает ни одним измеренным путём — его отвергают и присваивание, и
/// `Установить`, и `Добавить`/`Установить`/`Вставить` списка, и чтение
/// документа, — так что проверять `Проверить()` не на чем.
/// `НЕ ИЗМЕРЕНО(XDTO.VALIDATION.VALIDATE_FACETS)`.
///
/// # Errors
///
/// [`RtError::Xdto`], если какая-нибудь граница нарушена.
pub fn object_validate(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Проверить")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Проверить"));
    }
    validate_instance(data, data.model.types.len() + 8)?;
    Ok(BslValue::Undefined)
}

pub(crate) fn validate_instance(data: &Rc<XdtoObjectData>, depth: usize) -> RtResult<()> {
    let Some(depth) = depth.checked_sub(1) else {
        return Err(RtError::Xdto(
            "объекты XDTO вложены друг в друга кольцом".to_string(),
        ));
    };
    for p in &data.type_data()?.properties {
        let prop = data.model.property_at(*p)?;
        let places = data.occurrences(*p);
        let count = u32::try_from(places.len()).unwrap_or(u32::MAX);
        if count < prop.lower.unwrap_or(0) {
            return Err(RtError::Xdto(format!(
                "свойство «{}» обязано входить не менее {} раз, а заполнено {count}",
                prop.name,
                prop.lower.unwrap_or(0)
            )));
        }
        if let Some(upper) = prop.upper
            && count > upper
        {
            return Err(RtError::Xdto(format!(
                "свойство «{}» входит не более {upper} раз, а заполнено {count}",
                prop.name
            )));
        }
        for place in places {
            let nested = match data.entries.borrow().get(place).map(|e| e.value.clone()) {
                Some(value) => match repr_of(&value) {
                    Some(XdtoRepr::Object(inst)) => Some(inst.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(nested) = nested {
                validate_instance(&nested, depth)?;
            }
        }
    }
    Ok(())
}

/// `ОбъектXDTO.Последовательность()` — порядок заполнения свойств-элементов.
///
/// Есть она только у ПОСЛЕДОВАТЕЛЬНОГО типа: у `xs:choice` и `xs:all`
/// возвращается объект, а у типа-последовательности (`Упорядоченный` —
/// «Да») — `Неопределено`, а не ошибка (измерено на `RootType` и
/// `SimpContent`).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не экземпляр либо
/// вызов с аргументами.
pub fn object_sequence(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = instance_of(obj, "Последовательность")?;
    if !args.is_empty() {
        return Err(not_applicable(obj, "Последовательность"));
    }
    if !data.type_data()?.sequenced() {
        return Ok(BslValue::Undefined);
    }
    Ok(shell_value(XdtoRepr::Sequence(data.clone())))
}

// --- `СписокXDTO` --------------------------------------------------------

/// Длина списка — число заполнений его свойства.
pub(crate) fn list_len(data: &Rc<XdtoObjectData>, prop: usize) -> usize {
    data.occurrences(prop).len()
}

pub(crate) fn out_of_bounds(i: usize, len: usize) -> RtError {
    RtError::IndexOutOfBounds {
        index: i as i64,
        len,
    }
}

/// `Список[i]`, `Список.Получить(i)` — само значение, а не `ЗначениеXDTO`
/// (измерено: `ТипЗнч(О.code[0])` — «Строка»).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`], если номера в списке нет (измерено:
/// платформа отвечает ошибкой, а не `Неопределено`).
pub fn list_get(obj: &dyn ObjectProtocol, i: usize) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Получить")?;
    list_item(data, prop, i)
}

pub(crate) fn list_item(data: &Rc<XdtoObjectData>, prop: usize, i: usize) -> RtResult<BslValue> {
    let places = data.occurrences(prop);
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let entries = data.entries.borrow();
    entries
        .get(place)
        .map(|e| e.value.clone())
        .ok_or_else(|| broken("значение свойства"))
}

/// `Список.Добавить(Значение)` — значение приводится к типу свойства теми
/// же правилами, что и при записи (измерено: `many5.Добавить(5)` даёт
/// строку «5», `Добавить(Дата)` — «2026-08-13T00:00:00», `Добавить(Истина)`
/// — «true», а `Добавить(Неопределено)` — ошибка).
///
/// # Errors
///
/// [`RtError::Xdto`], если значение не приводится к типу свойства.
pub fn list_add(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Добавить")?;
    let [value] = args else {
        return Err(not_applicable(obj, "Добавить"));
    };
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().push(XdtoEntry {
        prop,
        value: coerced,
    });
    Ok(BslValue::Undefined)
}

/// `Список.Установить(i, Значение)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка (измерено),
/// [`RtError::Xdto`], если значение не приводится к типу свойства.
pub fn list_set(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Установить")?;
    let [index, value] = args else {
        return Err(not_applicable(obj, "Установить"));
    };
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let coerced = coerce_to_property(data, prop, value.clone())?;
    let mut entries = data.entries.borrow_mut();
    let slot = entries
        .get_mut(place)
        .ok_or_else(|| broken("значение свойства"))?;
    slot.value = coerced;
    Ok(BslValue::Undefined)
}

/// `Список.Вставить(i, Значение)` — новое заполнение встаёт на место
/// i-го, то есть и в последовательности оно оказывается перед ним
/// (измерено: после `cb.Добавить("аб"); ca = "вг"; cb.Вставить(0, "де")`
/// последовательность — `[cb=де][cb=аб][ca=вг]`).
///
/// Позиция обязана быть ЗАНЯТОЙ: и в пустой список, и на место сразу за
/// последним элементом платформа вставлять отказывается (измерено оба),
/// так что дописать в конец можно только `Добавить`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка, [`RtError::Xdto`],
/// если значение не приводится к типу свойства.
pub fn list_insert(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let (data, prop) = list_of(obj, "Вставить")?;
    let [index, value] = args else {
        return Err(not_applicable(obj, "Вставить"));
    };
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let at = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().insert(
        at,
        XdtoEntry {
            prop,
            value: coerced,
        },
    );
    Ok(BslValue::Undefined)
}

/// `Список.Удалить(i)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей списка (измерено).
pub fn list_delete(obj: &dyn ObjectProtocol, index: &BslValue) -> RtResult<()> {
    let (data, prop) = list_of(obj, "Удалить")?;
    let places = data.occurrences(prop);
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    data.entries.borrow_mut().remove(place);
    Ok(())
}

/// `Список.Очистить()` — забыть все заполнения этого свойства.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не список.
pub fn list_clear(obj: &dyn ObjectProtocol) -> RtResult<()> {
    let (data, prop) = list_of(obj, "Очистить")?;
    data.entries.borrow_mut().retain(|e| e.prop != prop);
    Ok(())
}

/// Номер элемента из аргумента-числа.
pub(crate) fn index_arg(index: &BslValue, len: usize) -> RtResult<usize> {
    let BslValue::Number(n) = index else {
        return Err(RtError::BadIndex);
    };
    let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(i).map_err(|_| RtError::IndexOutOfBounds { index: i, len })
}

// --- `ПоследовательностьXDTO` --------------------------------------------

/// Хранилище получателя-последовательности.
pub(crate) fn sequence_of<'a>(
    obj: &'a dyn ObjectProtocol,
    method: &'static str,
) -> RtResult<&'a Rc<XdtoObjectData>> {
    match repr_of_object(obj) {
        Some(XdtoRepr::Sequence(data)) => Ok(data),
        _ => Err(not_applicable(obj, method)),
    }
}

/// Элемент ли это последовательности: в неё попадают только свойства формы
/// «Элемент» — запись АТРИБУТА её не удлиняет (измерено на `cat`).
pub(crate) fn in_sequence(model: &XdtoModel, prop: usize) -> RtResult<bool> {
    Ok(model.property_at(prop)?.form == EnumValue::XmlFormElement)
}

/// Номера записей хранилища, попадающих в последовательность.
pub(crate) fn sequence_places(data: &Rc<XdtoObjectData>) -> RtResult<Vec<usize>> {
    let mut places = Vec::new();
    for (i, e) in data.entries.borrow().iter().enumerate() {
        if in_sequence(&data.model, e.prop)? {
            places.push(i);
        }
    }
    Ok(places)
}

pub(crate) fn sequence_len(data: &Rc<XdtoObjectData>) -> RtResult<usize> {
    Ok(sequence_places(data)?.len())
}

/// `Последовательность.ПолучитьЗначение(i)`.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей (измерено).
pub fn sequence_value(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "ПолучитьЗначение")?;
    let [index] = args else {
        return Err(not_applicable(obj, "ПолучитьЗначение"));
    };
    let places = sequence_places(data)?;
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let entries = data.entries.borrow();
    entries
        .get(place)
        .map(|e| e.value.clone())
        .ok_or_else(|| broken("значение свойства"))
}

/// `Последовательность.ПолучитьСвойство(i)` — `СвойствоXDTO`, которым это
/// место заполнено (измерено: печатается именем свойства).
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] за границей.
pub fn sequence_property(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "ПолучитьСвойство")?;
    let [index] = args else {
        return Err(not_applicable(obj, "ПолучитьСвойство"));
    };
    let places = sequence_places(data)?;
    let i = index_arg(index, places.len())?;
    let place = *places
        .get(i)
        .ok_or_else(|| out_of_bounds(i, places.len()))?;
    let prop = data
        .entries
        .borrow()
        .get(place)
        .map(|e| e.prop)
        .ok_or_else(|| broken("значение свойства"))?;
    Ok(property_value(&data.model, prop))
}

/// `Последовательность.Добавить(Свойство, Значение)` — заполнение в конец.
///
/// Свойство берётся только `СвойствоXDTO`, имя строкой платформа не
/// принимает, и атрибут тоже (измерено обе пробы). Заполнение видно и
/// через само свойство: после `Добавить(ca, "аб")` чтение `О.ca` даёт
/// «аб», а второе `Добавить` того же свойства удлиняет
/// последовательность до двух, оставляя в свойстве последнее значение
/// (измерено).
///
/// # Errors
///
/// [`RtError::Xdto`], если свойство чужое, атрибутное либо значение не
/// приводится к его типу.
pub fn sequence_add(obj: &dyn ObjectProtocol, args: &[BslValue]) -> RtResult<BslValue> {
    let data = sequence_of(obj, "Добавить")?;
    let [arg, value] = args else {
        return Err(not_applicable(obj, "Добавить"));
    };
    let BslValue::Object(_) = arg else {
        return Err(RtError::MethodNotApplicable {
            method: "Добавить",
            receiver: arg.type_name(),
        });
    };
    let prop = property_arg(data, arg, "Добавить")?;
    if !in_sequence(&data.model, prop)? {
        return Err(RtError::Xdto(format!(
            "свойство «{}» — не элемент, в последовательность оно не входит",
            data.model.property_at(prop)?.name
        )));
    }
    let coerced = coerce_to_property(data, prop, value.clone())?;
    data.entries.borrow_mut().push(XdtoEntry {
        prop,
        value: coerced,
    });
    Ok(BslValue::Undefined)
}

/// `Последовательность.Очистить()` — забыть заполнения свойств-ЭЛЕМЕНТОВ;
/// атрибуты уцелевают (измерено: после очистки `cat` на месте, `ca` пуст).
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`], если получатель не последовательность.
pub fn sequence_clear(obj: &dyn ObjectProtocol) -> RtResult<()> {
    let data = sequence_of(obj, "Очистить")?;
    let places = sequence_places(data)?;
    let mut entries = data.entries.borrow_mut();
    for place in places.into_iter().rev() {
        entries.remove(place);
    }
    Ok(())
}
