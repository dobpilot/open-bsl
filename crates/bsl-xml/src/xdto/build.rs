//! Построение модели по лексической модели схемы.

use super::*;

// --- построение ----------------------------------------------------------

/// Модель типов по одной разобранной схеме.
///
/// # Errors
///
/// [`RtError::Xdto`], если схема ссылается на неизвестный тип, содержит
/// цикл наследования либо значение по умолчанию, которое не разбирается в
/// объявленном типе.
#[cfg_attr(not(test), allow(dead_code))]
pub fn model_of_schema(schema: &Rc<XsSchemaData>) -> RtResult<Rc<XdtoModel>> {
    model_of_schemas(std::slice::from_ref(schema))
}

/// Модель типов по НАБОРУ схем — то, что стоит за `Новый
/// ФабрикаXDTO(НаборСхемXML)`.
///
/// Встроенные типы XML Schema объявляются один раз на всю модель, а имена в
/// ссылках (`base`, `type`, `itemType`, `memberTypes`) разрешаются по всем
/// схемам набора сразу: схема из одного пространства имён вправе ссылаться
/// на тип из другой схемы того же набора. Двусмысленности это не создаёт —
/// `НаборСхемXML` держит не больше одной схемы на пространство имён
/// (измерено, см. [`bsl_rt::xsd`]).
///
/// Пустой набор даёт фабрику с одними встроенными типами — ровно то же, что
/// `Новый ФабрикаXDTO` без аргументов (измерено: у такой фабрики
/// `Тип({...}string)` есть, а `Тип({urn:test}RootType)` — `Неопределено`).
///
/// # Errors
///
/// [`RtError::Xdto`], если какая-нибудь схема набора ссылается на неизвестный
/// тип, содержит цикл наследования либо значение по умолчанию, которое не
/// разбирается в объявленном типе.
pub fn model_of_schemas(schemas: &[Rc<XsSchemaData>]) -> RtResult<Rc<XdtoModel>> {
    let mut builder = Builder::new(schemas);
    builder.declare_builtins();
    builder.declare_schema_types()?;
    builder.link_bases()?;
    builder.build_properties()?;
    Ok(Rc::new(builder.model))
}

/// Место узла в наборе схем: номер схемы и номер узла в ней. Номера узлов
/// у схем свои, поэтому по всему набору однозначна только пара.
pub(crate) type XsPlace = (usize, usize);

pub(crate) struct Builder<'a> {
    pub(crate) schemas: &'a [Rc<XsSchemaData>],
    pub(crate) model: XdtoModel,
    /// Место узла XSD -> номер типа модели, для типов, объявленных схемами.
    pub(crate) from_xs: Vec<(XsPlace, usize)>,
    /// Номер типа модели -> место узла XSD, откуда он построен.
    pub(crate) to_xs: Vec<Option<XsPlace>>,
    /// Тип, чьи свойства сейчас считаются, — страховка от цикла
    /// наследования.
    pub(crate) busy: Vec<bool>,
    pub(crate) done: Vec<bool>,
}

impl<'a> Builder<'a> {
    pub(crate) fn new(schemas: &'a [Rc<XsSchemaData>]) -> Builder<'a> {
        Builder {
            schemas,
            model: XdtoModel {
                types: Vec::new(),
                properties: Vec::new(),
            },
            from_xs: Vec::new(),
            to_xs: Vec::new(),
            busy: Vec::new(),
            done: Vec::new(),
        }
    }

    /// Схема по номеру. Номер приходит только изнутри — из `to_xs` или из
    /// перебора набора, — но подтверждать это `unwrap`ом на
    /// пользовательских данных незачем: испорченный номер значит, что
    /// испорчена сама модель, и об этом есть [`broken`].
    pub(crate) fn schema_at(&self, si: usize) -> RtResult<&XsSchemaData> {
        self.schemas
            .get(si)
            .map(Rc::as_ref)
            .ok_or_else(|| broken("схема"))
    }

    pub(crate) fn push_type(&mut self, data: XdtoTypeData, xs: Option<XsPlace>) -> usize {
        self.model.types.push(data);
        self.to_xs.push(xs);
        self.busy.push(false);
        self.done.push(false);
        if let Some(place) = xs {
            self.from_xs.push((place, self.model.types.len() - 1));
        }
        self.model.types.len() - 1
    }

    /// Встроенные типы пространства XML Schema — они есть у любой фабрики
    /// (измерено: `Новый ФабрикаXDTO` уже знает `{...}string`).
    pub(crate) fn declare_builtins(&mut self) {
        for row in BUILTIN_TYPES {
            // Единственный встроенный тип ОБЪЕКТА — `anyType`, и три его
            // флага измерены разом: открытый, упорядоченный и смешанный.
            // У всех остальных встроенных (это типы значения) те же флаги
            // не читаются вовсе.
            let is_any_type = row.bsl.is_none();
            self.push_type(
                XdtoTypeData {
                    name: row.name.to_string(),
                    ns: XSD_NS.to_string(),
                    base: None,
                    shape: row.bsl.map(ValueShape::Builtin),
                    facets: row
                        .facets
                        .iter()
                        .map(|(k, v)| (*k, (*v).to_string()))
                        .collect(),
                    properties: Vec::new(),
                    open: is_any_type,
                    is_abstract: false,
                    ordered: is_any_type,
                    mixed: is_any_type,
                },
                None,
            );
        }
        for (i, row) in BUILTIN_TYPES.iter().enumerate() {
            let base = row.base.and_then(|name| self.model.find(XSD_NS, name));
            self.model.types[i].base = base;
        }
    }

    /// Именованные глобальные типы каждой схемы набора. Анонимные
    /// объявляются позже, при разборе свойств: на них ссылается только своё
    /// свойство.
    pub(crate) fn declare_schema_types(&mut self) -> RtResult<()> {
        for si in 0..self.schemas.len() {
            // Номера копируются, потому что `declare_type` берёт `&mut
            // self`, а список живёт в схеме за общей ссылкой.
            let nodes: Vec<usize> = self.schema_at(si)?.global_types().to_vec();
            for node in nodes {
                self.declare_type(si, node)?;
            }
        }
        Ok(())
    }

    pub(crate) fn declare_type(&mut self, si: usize, node: usize) -> RtResult<usize> {
        if let Some((_, idx)) = self.from_xs.iter().find(|(p, _)| *p == (si, node)) {
            return Ok(*idx);
        }
        let schema = self.schema_at(si)?;
        // Пространство имён у типа модели — ЦЕЛЕВОЕ пространство схемы, и
        // у анонимного тоже, хотя имя у него пусто (измерено: у типа
        // безымянного `<xs:complexType>` внутри объявления `URI` —
        // `urn:test`). Лексическая модель XSD здесь другая: там у
        // анонимного типа пространство имён пусто.
        let target_ns = schema.target_namespace().to_string();
        let name = schema.name_of(node).to_string();
        let data = match schema.kind_of(node) {
            XsKind::SimpleType => {
                let shape = match schema.simple_variety_of(node) {
                    Some((EnumValue::XsVarietyList, _, _)) => ValueShape::List(None),
                    Some((EnumValue::XsVarietyUnion, _, _)) => ValueShape::Union(Vec::new()),
                    _ => ValueShape::Atomic,
                };
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: Some(shape),
                    facets: schema
                        .facets_of(node)
                        .into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                    properties: Vec::new(),
                    // Четыре флага ниже читаются только у типа ОБЪЕКТА
                    // (у типа значения обращение к ним платформа
                    // отвергает), поэтому у типов значения они выключены
                    // все и одинаково — и у схемных, и у встроенных.
                    open: false,
                    is_abstract: false,
                    ordered: false,
                    mixed: false,
                }
            }
            XsKind::ComplexType => {
                let (mixed, is_abstract) = schema.complex_flags_of(node);
                XdtoTypeData {
                    name,
                    ns: target_ns,
                    base: None,
                    shape: None,
                    facets: Vec::new(),
                    properties: Vec::new(),
                    // `Открытый` — это МАСКА в объявлении типа, и любая из
                    // двух: `xs:any` и `xs:anyAttribute` поодиночке дают
                    // «Да» (измерено, `XDTO.WRITE_ORDER.FLAGS` и
                    // `XDTO.WRITE_ORDER.WILDCARD`). Унаследованная маска
                    // добавляется в [`Builder::ensure_properties`].
                    open: schema.complex_has_wildcard(node),
                    is_abstract,
                    ordered: content_is_ordered(schema, node),
                    mixed,
                }
            }
            other => {
                return Err(RtError::Xdto(format!(
                    "типом XDTO может стать только определение типа, а не «{}»",
                    other.type_name()
                )));
            }
        };
        Ok(self.push_type(data, Some((si, node))))
    }

    /// Базовые типы схемных типов: имя из `base` разрешается в номер. Имя
    /// ищется по ВСЕМУ набору, а не только в своей схеме, — иначе ссылка на
    /// соседнее пространство имён обрывалась бы.
    pub(crate) fn link_bases(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            let Some((si, node)) = self.to_xs[i] else {
                continue;
            };
            let base = if self.model.types[i].is_value() {
                let name = self.schema_at(si)?.simple_base_of(node).cloned();
                match name {
                    Some(n) => Some(self.require_type(&n)?),
                    // Тип значения без явного базового наследует
                    // `anySimpleType` (измерено на списке и объединении).
                    None => self.model.find(XSD_NS, "anySimpleType"),
                }
            } else {
                // У типа объекта базовым становится только ОБЪЕКТНЫЙ
                // базовый тип: у составного типа с простым содержимым
                // платформа отдаёт `anyType`, а простой базовый тип
                // виден свойством `__content` (измерено).
                let name = self.schema_at(si)?.complex_base_of(node).cloned();
                let resolved = match name {
                    Some(n) => Some(self.require_type(&n)?),
                    None => None,
                };
                match resolved {
                    Some(b) if !self.model.types[b].is_value() => Some(b),
                    _ => self.model.find(XSD_NS, "anyType"),
                }
            };
            self.model.types[i].base = base;
        }
        // Тип элемента списка и члены объединения — по тем же именам.
        for i in 0..self.model.types.len() {
            let Some((si, node)) = self.to_xs[i] else {
                continue;
            };
            let Some((variety, item, members)) = self.schema_at(si)?.simple_variety_of(node) else {
                continue;
            };
            let shape = match variety {
                EnumValue::XsVarietyList => {
                    let item = match item.cloned() {
                        Some(n) => Some(self.require_type(&n)?),
                        None => None,
                    };
                    ValueShape::List(item)
                }
                EnumValue::XsVarietyUnion => {
                    let names: Vec<XName> = members.to_vec();
                    let mut resolved = Vec::with_capacity(names.len());
                    for n in &names {
                        resolved.push(self.require_type(n)?);
                    }
                    ValueShape::Union(resolved)
                }
                _ => continue,
            };
            self.model.types[i].shape = Some(shape);
        }
        Ok(())
    }

    /// Тип по имени — с ошибкой вместо `Неопределено`: ссылка на
    /// несуществующий тип делает модель неполной, и молчать об этом хуже,
    /// чем отказать. Ищется по всем схемам набора сразу.
    pub(crate) fn require_type(&self, name: &XName) -> RtResult<usize> {
        self.model.find(&name.uri, &name.local).ok_or_else(|| {
            RtError::Xdto(format!(
                "в схеме нет типа «{}», на который ссылается модель",
                name.display_text()
            ))
        })
    }

    pub(crate) fn build_properties(&mut self) -> RtResult<()> {
        for i in 0..self.model.types.len() {
            self.ensure_properties(i)?;
        }
        Ok(())
    }

    /// Свойства типа объекта: сначала унаследованные, потом собственные
    /// атрибуты, потом собственные элементы (измеренный порядок).
    ///
    /// Здесь же достраивается ОТКРЫТОСТЬ: маска базового типа делает
    /// открытым и наследника (измерено на `ExtOpen`, расширяющем тип с
    /// масками и не несущем своих, — `Открытый` «Да»). Идёт она тем же
    /// путём, что и свойства, и по той же причине: наследование в этой
    /// модели плоское, а расширение и ограничение не различаются.
    ///
    /// Единственное исключение — `anyType`. Сам он открыт, но открытости
    /// не передаёт: тип, ЯВНО его расширяющий, платформа отдаёт закрытым
    /// (`XDTO.WRITE_ORDER.EXT_ANY` — `Нет|Да|Нет` и схемный порядок
    /// записи). Правило это ещё и необходимое: базовый тип у составного
    /// заполнен ВСЕГДА — при отсутствии явного подставляется `anyType`,
    /// — так что без исключения открытым стал бы каждый тип.
    pub(crate) fn ensure_properties(&mut self, index: usize) -> RtResult<()> {
        if self.done[index] {
            return Ok(());
        }
        if self.busy[index] {
            return Err(RtError::Xdto(format!(
                "циклическое наследование типов XDTO вокруг «{}»",
                self.model.types[index].name
            )));
        }
        self.busy[index] = true;
        let mut props = Vec::new();
        if let Some(base) = self.model.types[index].base
            && !self.model.types[base].is_value()
        {
            self.ensure_properties(base)?;
            props.extend_from_slice(&self.model.types[base].properties);
            // Открытость идёт следом за свойствами, но `anyType`
            // её НЕ передаёт: он открыт сам (измерено), однако
            // ни подставленный заглушкой, ни выписанный в схеме
            // явно наследника не открывает — `Closed` и `ExtAny`
            // оба «Нет» (`XDTO.WRITE_ORDER.EXT_ANY`). Иначе
            // открытым стал бы каждый составной тип: базовый у
            // них заполнен всегда.
            if !is_any_type(&self.model.types[base]) && self.model.types[base].open {
                self.model.types[index].open = true;
            }
        }
        if let Some((si, node)) = self.to_xs[index]
            && !self.model.types[index].is_value()
        {
            self.collect_attributes(si, node, &mut props)?;
            self.collect_content(si, node, &mut props)?;
        }
        self.model.types[index].properties = props;
        self.busy[index] = false;
        self.done[index] = true;
        Ok(())
    }

    /// Собственные атрибуты составного типа. Обязательный атрибут даёт
    /// границы `1..1`, необязательный — `0..1` (измерено).
    pub(crate) fn collect_attributes(
        &mut self,
        si: usize,
        node: usize,
        out: &mut Vec<usize>,
    ) -> RtResult<()> {
        let uses: Vec<usize> = self.schema_at(si)?.complex_attribute_uses_of(node).to_vec();
        for use_node in uses {
            let schema = self.schema_at(si)?;
            let Some(view) = schema.attribute_use_of(use_node) else {
                continue;
            };
            let (decl_node, required, lexical, has_constraint) = (
                view.declaration,
                view.required,
                view.lexical.to_string(),
                view.has_constraint,
            );
            let Some(decl) = schema.decl_of(decl_node) else {
                continue;
            };
            let (name, ns) = (decl.name.to_string(), decl.ns.to_string());
            let type_index = self.property_type(si, decl_node)?;
            let default = if has_constraint {
                Some(self.value_of(type_index, &lexical)?)
            } else {
                None
            };
            let property = XdtoPropertyData {
                name,
                ns,
                type_index,
                lower: Some(u32::from(required)),
                upper: Some(1),
                form: EnumValue::XmlFormAttribute,
                default,
            };
            self.model.properties.push(property);
            out.push(self.model.properties.len() - 1);
        }
        Ok(())
    }

    /// Собственное содержимое: либо элементы модели содержимого, либо
    /// текстовое свойство `__content` у типа с простым содержимым.
    pub(crate) fn collect_content(
        &mut self,
        si: usize,
        node: usize,
        out: &mut Vec<usize>,
    ) -> RtResult<()> {
        if let Some(particle) = self.schema_at(si)?.complex_content_of(node) {
            return self.collect_elements(si, particle, Some(1), Some(1), out);
        }
        // Простое содержимое: базовый тип — простой, и платформа
        // показывает его свойством `__content` с формой `Текст`
        // (измерено). Отличать `xs:simpleContent` от `xs:complexContent`
        // отдельным признаком не нужно: у простого содержимого нет модели
        // содержимого, а базовый тип — тип ЗНАЧЕНИЯ, и обе проверки уже
        // сделаны выше. СМЕШАННЫЙ тип сюда не доходит: модель содержимого
        // у него есть, и своего текстового свойства платформа ему не даёт
        // (измерено на `mixed="true"` — там только объявленный элемент).
        let Some(base_name) = self.schema_at(si)?.complex_base_of(node).cloned() else {
            return Ok(());
        };
        let base = self.require_type(&base_name)?;
        if !self.model.types[base].is_value() {
            return Ok(());
        }
        self.model.properties.push(XdtoPropertyData {
            name: CONTENT_PROPERTY.to_string(),
            ns: String::new(),
            type_index: base,
            lower: Some(1),
            upper: Some(1),
            form: EnumValue::XmlFormText,
            default: None,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Разложить фрагмент в свойства, перемножая границы вхождения по
    /// вложенным группам модели.
    pub(crate) fn collect_elements(
        &mut self,
        si: usize,
        particle: usize,
        outer_lower: Option<u32>,
        outer_upper: Option<u32>,
        out: &mut Vec<usize>,
    ) -> RtResult<()> {
        let schema = self.schema_at(si)?;
        let Some((term, min, max)) = schema.particle_of(particle) else {
            return Ok(());
        };
        let lower = fold_bounds(outer_lower, bound_of(min, 1));
        let upper = fold_bounds(outer_upper, bound_of(max, 1));
        if let Some((_, particles)) = schema.model_group_of(term) {
            let inner: Vec<usize> = particles.to_vec();
            for p in inner {
                self.collect_elements(si, p, lower, upper, out)?;
            }
            return Ok(());
        }
        let Some(decl) = schema.decl_of(term) else {
            return Err(RtError::Xdto(
                "термом фрагмента может быть объявление элемента или группа модели".to_string(),
            ));
        };
        let (name, ns, lexical, has_constraint) = (
            decl.name.to_string(),
            decl.ns.to_string(),
            decl.lexical.to_string(),
            decl.has_constraint,
        );
        let type_index = self.property_type(si, term)?;
        let default = if has_constraint {
            Some(self.value_of(type_index, &lexical)?)
        } else {
            None
        };
        self.model.properties.push(XdtoPropertyData {
            name,
            ns,
            type_index,
            lower,
            upper,
            form: EnumValue::XmlFormElement,
            default,
        });
        out.push(self.model.properties.len() - 1);
        Ok(())
    }

    /// Тип свойства: объявленный `type`, встроенный анонимный тип или —
    /// если ни того, ни другого нет — `anyType` (измерено на
    /// `<xs:element name="notype"/>`).
    pub(crate) fn property_type(&mut self, si: usize, decl_node: usize) -> RtResult<usize> {
        let (type_name, anonymous) = match self.schema_at(si)?.decl_of(decl_node) {
            Some(d) => (d.type_name.cloned(), d.anonymous_type),
            None => (None, None),
        };
        if let Some(name) = type_name {
            return self.require_type(&name);
        }
        if let Some(node) = anonymous {
            let index = self.declare_type(si, node)?;
            // Анонимный тип объявлен уже после связывания базовых типов,
            // поэтому его база и свойства достраиваются здесь же.
            self.link_one_base(si, index, node)?;
            self.ensure_properties(index)?;
            return Ok(index);
        }
        self.model
            .find(XSD_NS, "anyType")
            .ok_or_else(|| broken("anyType"))
    }

    /// Базовый тип одного (анонимного) типа — та же логика, что в
    /// [`Builder::link_bases`], но для типа, объявленного позже.
    pub(crate) fn link_one_base(&mut self, si: usize, index: usize, node: usize) -> RtResult<()> {
        if self.model.types[index].base.is_some() {
            return Ok(());
        }
        let base = if self.model.types[index].is_value() {
            match self.schema_at(si)?.simple_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => self.model.find(XSD_NS, "anySimpleType"),
            }
        } else {
            let resolved = match self.schema_at(si)?.complex_base_of(node).cloned() {
                Some(n) => Some(self.require_type(&n)?),
                None => None,
            };
            match resolved {
                Some(b) if !self.model.types[b].is_value() => Some(b),
                _ => self.model.find(XSD_NS, "anyType"),
            }
        };
        self.model.types[index].base = base;
        Ok(())
    }

    /// Значение по умолчанию свойства — из `default` или `fixed` схемы.
    ///
    /// Проверяется по фасетам, потому что платформа проверяет их ЗДЕСЬ, а
    /// не при чтении свойства: фабрика над схемой, где `default="а"` стоит
    /// у типа с `minLength="2"`, не строится вовсе (измерено —
    /// `СоздатьФабрикуXDTO` от такой схемы отвечает ошибкой). Так что
    /// негодное умолчание валит построение модели целиком, а не всплывает
    /// потом на первом же чтении.
    pub(crate) fn value_of(&self, type_index: usize, lexical: &str) -> RtResult<Rc<XdtoValueData>> {
        Ok(Rc::new(XdtoValueData {
            value: value_from_lexical_checked(&self.model, type_index, lexical)?,
            lexical: lexical.to_string(),
            type_index,
        }))
    }
}

/// Имя свойства, которым платформа показывает текст типа с простым
/// содержимым (измерено).
pub(crate) const CONTENT_PROPERTY: &str = "__content";

/// `Упорядоченный` — «Да» у последовательности и у типа без модели
/// содержимого, «Нет» у `xs:choice` и `xs:all` (измерено на пяти типах).
pub(crate) fn content_is_ordered(schema: &XsSchemaData, node: usize) -> bool {
    let Some(particle) = schema.complex_content_of(node) else {
        return true;
    };
    let Some((term, _, _)) = schema.particle_of(particle) else {
        return true;
    };
    match schema.model_group_of(term) {
        Some((EnumValue::XsGroupSequence, _)) => true,
        Some(_) => false,
        None => true,
    }
}

/// Граница вхождения из лексической модели XSD: отсутствующий атрибут —
/// это `default`, а `unbounded` (то есть `u32::MAX`) — `None`.
pub(crate) fn bound_of(raw: Option<u32>, default: u32) -> Option<u32> {
    match raw {
        None => Some(default),
        Some(u32::MAX) => None,
        Some(n) => Some(n),
    }
}

/// Границы перемножаются по вложенным группам модели (измерено:
/// `<xs:choice minOccurs="0">` делает `1..1` вложенного элемента `0..1`, а
/// `<xs:sequence maxOccurs="unbounded">` делает `0..1` -> `0..-1`). Ноль
/// поглощает бесконечность: вхождений всё равно ноль.
pub(crate) fn fold_bounds(outer: Option<u32>, inner: Option<u32>) -> Option<u32> {
    match (outer, inner) {
        (Some(0), _) | (_, Some(0)) => Some(0),
        (Some(a), Some(b)) => Some(a.saturating_mul(b)),
        _ => None,
    }
}
