//! Разрешённая модель типов: типы, свойства, их связи.

use super::*;

// --- модель --------------------------------------------------------------

/// Устройство типа ЗНАЧЕНИЯ — то, от чего зависит разбор лексической
/// формы.
#[derive(Debug, Clone)]
pub(crate) enum ValueShape {
    /// Встроенный тип с прямым отображением в тип BSL.
    Builtin(BuiltinBsl),
    /// Атомарный производный тип: отображение берётся у базового
    /// (измерено на `Code` и `Small`).
    Atomic,
    /// Список: значение — `ФиксированныйМассив` значений типа элемента,
    /// лексическая форма разделяется пробельными символами.
    List(Option<usize>),
    /// Объединение: тип выбирается ПЕРВЫМ членом, который принимает
    /// лексическую форму (измерено).
    Union(Vec<usize>),
}

/// Тип модели: и тип значения, и тип объекта — разницу несёт `shape`.
#[derive(Debug)]
pub(crate) struct XdtoTypeData {
    pub(crate) name: String,
    pub(crate) ns: String,
    /// Базовый тип; `None` только у `anyType`.
    pub(crate) base: Option<usize>,
    /// `None` — это тип ОБЪЕКТА.
    pub(crate) shape: Option<ValueShape>,
    /// Фасеты типа значения: вид и лексическая запись значения.
    pub(crate) facets: Vec<(FacetKind, String)>,
    /// Свойства типа объекта — уже сплющенные вместе с унаследованными.
    pub(crate) properties: Vec<usize>,
    pub(crate) open: bool,
    pub(crate) is_abstract: bool,
    pub(crate) ordered: bool,
    pub(crate) mixed: bool,
}

impl XdtoTypeData {
    pub(crate) fn is_value(&self) -> bool {
        self.shape.is_some()
    }

    /// `Последовательный` — во всех измеренных случаях «НЕ Упорядоченный
    /// ИЛИ Смешанный ИЛИ Открытый»: последовательность даёт «Нет»,
    /// `xs:choice` и `xs:all` — «Да», смешанный тип и `anyType` — «Да»,
    /// тип с маской — «Да» при `Упорядоченный` «Да» и `Смешанный` «Нет»
    /// (`XDTO.WRITE_ORDER.FLAGS`; последнее слагаемое и добавлено этим
    /// замером).
    ///
    /// Флаг не косметический: от него зависит ПОРЯДОК записи свойств в
    /// [`write_properties`] и наличие `Последовательность()`.
    pub(crate) fn sequenced(&self) -> bool {
        !self.ordered || self.mixed || self.open
    }
}

/// Свойство типа объекта.
#[derive(Debug)]
pub(crate) struct XdtoPropertyData {
    pub(crate) name: String,
    pub(crate) ns: String,
    pub(crate) type_index: usize,
    /// `None` — `unbounded`; наружу обе границы уходят числом, где
    /// `unbounded` — это `-1` (измерено).
    pub(crate) lower: Option<u32>,
    pub(crate) upper: Option<u32>,
    /// Член `ФормаXML`.
    pub(crate) form: EnumValue,
    pub(crate) default: Option<Rc<XdtoValueData>>,
}

/// `ЗначениеXDTO` — значение BSL вместе с лексической формой, из которой
/// оно получено, и номером типа, по которому разбиралось.
#[derive(Debug)]
pub struct XdtoValueData {
    pub(crate) value: BslValue,
    pub(crate) lexical: String,
    /// Номер типа в модели — его отдаёт метод `Тип()` (измерено:
    /// `Создать(xs:int, "42").Тип()` -> «Тип значения XDTO
    /// [{...}int]`»).
    pub(crate) type_index: usize,
}

/// Разрешённая модель типов одной схемы вместе со встроенными типами XML
/// Schema. Значение `ТипЗначенияXDTO` — это `Rc` на неё плюс номер типа,
/// `СвойствоXDTO` — тот же `Rc` плюс номер свойства.
pub struct XdtoModel {
    pub(crate) types: Vec<XdtoTypeData>,
    pub(crate) properties: Vec<XdtoPropertyData>,
    /// Часовой пояс ПРОГОНА, в котором фабрика построена.
    ///
    /// Зона нужна одному месту — переводу лексической формы `xs:dateTime`
    /// с явным смещением в местное время (`facets::apply_zone`), — но это
    /// место лежит на дне разбора, куда параметром её пришлось бы тянуть
    /// через три десятка сигнатур. Модель же есть у КАЖДОЙ из них: она
    /// первый аргумент почти всюду. Отсюда и решение: фабрика запоминает
    /// зону при построении и толкует лексические формы в ней.
    ///
    /// Расхождение с «зоной текущего прогона» возможно только если хост
    /// передаст фабрику из одной `State` в другую — на платформе такого
    /// понятия нет, и замерить его не у чего.
    pub(crate) zone: std::rc::Rc<dyn bsl_rt::TimeZone>,
}

/// Зона в отладочную печать не идёт: у неё нет содержательного
/// представления, а `{:?}` модели читают в диагностике типов.
impl std::fmt::Debug for XdtoModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XdtoModel")
            .field("types", &self.types)
            .field("properties", &self.properties)
            .finish_non_exhaustive()
    }
}

impl XdtoModel {
    /// Зона, в которой эта фабрика толкует лексические формы дат.
    pub(crate) fn zone(&self) -> &dyn bsl_rt::TimeZone {
        self.zone.as_ref()
    }

    pub(crate) fn type_at(&self, i: usize) -> RtResult<&XdtoTypeData> {
        self.types.get(i).ok_or_else(|| broken("тип"))
    }

    pub(crate) fn property_at(&self, i: usize) -> RtResult<&XdtoPropertyData> {
        self.properties.get(i).ok_or_else(|| broken("свойство"))
    }

    /// Тип по расширенному имени — то, что делает `ФабрикаXDTO.Тип(URI,
    /// Имя)`. Анонимные типы сюда не попадают: у них нет имени.
    pub fn find(&self, uri: &str, name: &str) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        self.types
            .iter()
            .position(|t| t.name == name && t.ns == uri)
    }

    /// Тип BSL, в который отображается тип значения; `None` — у типа
    /// объекта либо у списка и объединения, где тип зависит от значения.
    pub fn builtin_of(&self, index: usize) -> Option<BuiltinBsl> {
        let mut cur = index;
        // Цепочка базовых типов конечна: длина модели — верхняя граница,
        // и она же страхует от цикла в испорченной схеме.
        for _ in 0..=self.types.len() {
            match self.types.get(cur)?.shape.as_ref()? {
                ValueShape::Builtin(b) => return Some(*b),
                ValueShape::List(_) | ValueShape::Union(_) => return None,
                ValueShape::Atomic => cur = self.types.get(cur)?.base?,
            }
        }
        None
    }
}

pub(crate) fn broken(what: &str) -> RtError {
    RtError::Xdto(format!("модель типов XDTO повреждена: нет узла «{what}»"))
}
