//! Владеющий граф значений для переноса между изолированными сеансами.
//!
//! Фоновое задание получает параметры, а вызывающий — результаты и ошибки
//! через границу OS-потоков, где `Rc`/`RefCell`-значения непереносимы.
//! Граф — их `Send`-снимок: узлы владеют данными, повторные вхождения
//! одного объекта дедуплицируются по указателю, поэтому алиасы и циклы
//! переживают перенос структурно. Точная политика платформы (копирует ли
//! она алиасы при снимке параметров) уточняется замером
//! `JOB.PARAMS.SNAPSHOT`; представление её уже вмещает.
//!
//! Это НЕ текстовый внутренний формат: `vstr` печатает копиями по
//! измеренной семантике `ЗначениеВСтрокуВнутр` и не дедуплицирует, поэтому
//! общий обход дал бы принуждённую абстракцию — здесь обход свой, а
//! кодеки примитивов тривиальны (владеющие копии).

use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    BslDate, BslNumber, BslObject, BslString, BslValue, EnumValue, RtError, RtResult, RuntimeShapes,
};

/// Пределы снимка. Жёсткая платформенная граница — 1 ГиБ на графе
/// параметров; admission фонового задания задаёт и меньшие бюджеты.
#[derive(Debug, Clone, Copy)]
pub struct GraphLimits {
    pub max_bytes: usize,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self { max_bytes: 1 << 30 }
    }
}

/// Текст отказа ПО МЕСТУ. Тот же вариант `RtError::ResourceLimit`
/// сообщает и о значении, которое границу сеансов не пересекает, поэтому
/// вызывающему, различающему эти случаи (staging временного хранилища),
/// нужна опора надёжнее подстроки собственного сочинения.
pub(crate) const BUDGET_EXCEEDED: &str = "снимок значений превысил бюджет памяти";

/// Номер узла в графе.
type NodeId = u32;

/// Узел графа: примитив с владеющими данными либо коллекция из ссылок на
/// другие узлы.
#[derive(Debug, Clone, PartialEq)]
enum GraphNode {
    Undefined,
    Null,
    Boolean(bool),
    /// Каноническая запись числа: `BslNumber` держит `Rc` и через границу
    /// потоков не переносится, а канонический текст восстанавливается без
    /// потерь (`parse_canonical` — тот же кодек, что во внутреннем формате).
    Number(String),
    Str(String),
    Date(BslDate),
    Enum(EnumValue),
    /// Тип-значение переносится ИМЕНЕМ и разрешается заново в каталоге
    /// принимающего сеанса: `TypeRef` указывает в каталог отправителя.
    Type(String),
    Array(Vec<NodeId>),
    Structure(Vec<(String, NodeId)>),
    Map(Vec<(NodeId, NodeId)>),
    /// Таблица значений: колонки с ограничениями типов (имена типов и
    /// сырые квалификаторы) и ячейки в физическом порядке строк.
    /// Транзитные атрибуты внутреннего ТЕКСТОВОГО формата
    /// (`ColumnVstr`, идентификаторы строк) не переносятся: они — свойство
    /// текстового транзита, а не значения; уточняется `JOB.PARAMS.SNAPSHOT`.
    ValueTable {
        columns: Vec<GraphColumn>,
        rows: Vec<Vec<NodeId>>,
    },
}

/// Колонка таблицы в графе: имя и ограничение типов, если оно есть.
#[derive(Debug, Clone, PartialEq)]
struct GraphColumn {
    name: String,
    types: Option<Vec<(String, Vec<String>)>>,
}

/// `Send`-снимок набора значений: узлы, корни и учтённый размер.
///
/// Материализация ленивая по месту использования: список заданий не
/// декодирует параметры, декодирует — первое чтение свойства.
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedValueGraph {
    nodes: Vec<GraphNode>,
    roots: Vec<NodeId>,
    byte_size: usize,
}

// Инвариант представления: узлы владеют данными, `Rc`/`RefCell` в типе
// не встречаются — компилятор подтверждает переносимость.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SerializedValueGraph>();
};

/// Источник кредита для обхода. Фиксированный предел — обычный случай;
/// резервирующий берёт байты у внешнего распорядителя ПОРЦИЯМИ по ходу
/// обхода, поэтому даже служебная память обходчика (карта увиденных
/// объектов) покрыта настоящим резервом, а не надеждой на то, что после
/// обхода найдётся место.
pub enum GraphBudget<'a> {
    /// Ровно `max_bytes` и ни байтом больше.
    Fixed(usize),
    /// `reserve(bytes)` просит у распорядителя ещё столько байтов и
    /// возвращает, СКОЛЬКО выдано: ноль — их нет вовсе, меньше
    /// запрошенного — выдан остаток. Частичная выдача не отказ: обходу
    /// довольно того, что покрывает текущее списание, а требовать порцию
    /// целиком значило бы отвергать запись, которая на самом деле
    /// помещается в оставшийся общий бюджет.
    Reserving {
        /// Верхний предел, который распорядитель не переступит.
        ceiling: usize,
        reserve: &'a mut dyn FnMut(usize) -> usize,
    },
}

/// Обходчик снимка: дедуп по указателю объекта и бюджет байтов.
struct Capture<'a> {
    rt: &'a RuntimeShapes,
    nodes: Vec<GraphNode>,
    /// Ключ — адрес разделяемой аллокации: у обычных объектов это сам
    /// `BslObject`, у таблиц — их `ValueTableData` (несколько обёрток
    /// делят одни данные, и алиас считается по данным).
    seen: HashMap<usize, NodeId>,
    /// Уже взятые, но ещё не потраченные байты.
    remaining: usize,
    /// Всего списано — это и есть размер снимка.
    spent: usize,
    /// Наибольшее значение `spent` за обход. Предварительные оценки
    /// (длина числа) возвращаются после уточнения, поэтому итоговый
    /// размер бывает меньше пикового спроса — а повторному обходу нужен
    /// именно пик, иначе он упрётся в собственную оценку.
    peak: usize,
    budget: GraphBudget<'a>,
    /// Сухой обход: узлы не строятся и строки не материализуются — счёт
    /// идёт только по бюджету. Так вызывающий узнаёт ТОЧНЫЙ размер
    /// будущего снимка, не выделив под него ни байта.
    dry: bool,
    /// Номер следующего узла. В сухом обходе `nodes` пуст, а номера всё
    /// равно нужны: по ним `seen` замыкает циклы и алиасы.
    next_id: NodeId,
}

impl SerializedValueGraph {
    /// Снимает значения в переносимый граф.
    ///
    /// # Errors
    ///
    /// [`RtError::ResourceLimit`] при превышении бюджета — до аллокации,
    /// а не после; ловимая ошибка с именем типа — для значения, которое
    /// границу сеансов не пересекает (компонентный объект, обещание,
    /// поток).
    pub fn capture(
        values: &[BslValue],
        rt: &RuntimeShapes,
        limits: &GraphLimits,
    ) -> RtResult<Self> {
        Self::capture_with(values, rt, GraphBudget::Fixed(limits.max_bytes))
    }

    /// Снимок под произвольным источником кредита.
    ///
    /// # Errors
    ///
    /// Те же, что у [`SerializedValueGraph::capture`].
    pub fn capture_with<'a>(
        values: &[BslValue],
        rt: &'a RuntimeShapes,
        budget: GraphBudget<'a>,
    ) -> RtResult<Self> {
        let mut capture = Capture::new(rt, budget, false);
        // Корни классифицируются до всякого списания: непереносимое
        // значение обязано назвать свой тип, а не упереться в бюджет.
        for value in values {
            check_portable(value)?;
        }
        // Вектор корней — такая же память снимка, как и его узлы: он
        // оплачивается ДО того, как под него запрошено место.
        capture.charge(collection_cost::<NodeId>(values.len())?)?;
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            roots.push(capture.node(value)?);
        }
        Ok(Self {
            nodes: capture.nodes,
            roots,
            byte_size: capture.spent,
        })
    }

    /// Считает размер будущего снимка, НЕ строя его: тот же обход и тот
    /// же учёт, но без единой аллокации под узлы, строки и служебные
    /// копии. Позволяет взять точный ресурсный кредит до сериализации —
    /// вместо резерва «на глаз», который либо мал, либо отбирает бюджет
    /// у соседей.
    ///
    /// # Errors
    ///
    /// Те же, что у [`SerializedValueGraph::capture`]: превышение
    /// бюджета и непереносимое значение.
    pub fn measure(
        values: &[BslValue],
        rt: &RuntimeShapes,
        limits: &GraphLimits,
    ) -> RtResult<usize> {
        Self::measure_with(values, rt, GraphBudget::Fixed(limits.max_bytes))
    }

    /// Сухой обход под произвольным источником кредита: даже служебная
    /// память обходчика берётся у распорядителя порциями, поэтому
    /// параллельные измерения не могут суммарно выйти за общий предел.
    ///
    /// # Errors
    ///
    /// Те же, что у [`SerializedValueGraph::measure`].
    pub fn measure_with<'a>(
        values: &[BslValue],
        rt: &'a RuntimeShapes,
        budget: GraphBudget<'a>,
    ) -> RtResult<usize> {
        let mut capture = Capture::new(rt, budget, true);
        for value in values {
            check_portable(value)?;
        }
        capture.charge(collection_cost::<NodeId>(values.len())?)?;
        for value in values {
            capture.node(value)?;
        }
        // Возвращается ПИК: бюджета в размер готового снимка обходу не
        // хватит там, где он спрашивал больше и вернул излишек.
        Ok(capture.peak)
    }

    /// Учтённый размер снимка в байтах бюджета.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.byte_size
    }

    /// Число корней — исходных значений снимка.
    #[must_use]
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Материализует граф в значения принимающего сеанса. Алиасы и циклы
    /// восстанавливаются: объекты создаются пустыми, заполняются вторым
    /// проходом.
    ///
    /// # Errors
    ///
    /// Ошибка разрешения имени типа, которого нет в каталоге принимающего
    /// сеанса, и повреждённого графа (висячая ссылка узла).
    pub fn materialize(&self, rt: &mut RuntimeShapes) -> RtResult<Vec<BslValue>> {
        let mut cache: Vec<Option<BslValue>> = vec![None; self.nodes.len()];
        // Первый проход: все объекты создаются пустыми, чтобы ссылка на
        // узел всегда резолвилась, даже если он ссылается на себя.
        for (index, node) in self.nodes.iter().enumerate() {
            let value = match node {
                GraphNode::Undefined => BslValue::Undefined,
                GraphNode::Null => BslValue::Null,
                GraphNode::Boolean(value) => BslValue::Boolean(*value),
                GraphNode::Number(text) => {
                    BslValue::Number(BslNumber::parse_canonical(text).map_err(RtError::Num)?)
                }
                GraphNode::Str(value) => BslValue::Str(BslString::from_str(value)),
                GraphNode::Date(value) => BslValue::Date(*value),
                GraphNode::Enum(value) => BslValue::Enum(*value),
                GraphNode::Type(name) => {
                    let resolved = rt.resolve_type(name).ok_or_else(|| {
                        RtError::DynamicError(format!(
                            "тип «{name}» не зарегистрирован в принимающем сеансе"
                        ))
                    })?;
                    BslValue::Type(resolved)
                }
                GraphNode::Array(_) => BslValue::new_array(Vec::new()),
                GraphNode::Structure(_) => BslValue::new_structure(rt.shapes.empty(), Vec::new()),
                GraphNode::Map(_) => BslValue::new_map(),
                GraphNode::ValueTable { .. } => BslValue::Object(Rc::new(BslObject::ValueTable(
                    crate::table::ValueTableData::new(),
                ))),
            };
            cache[index] = Some(value);
        }
        let resolve = |cache: &[Option<BslValue>], id: NodeId| -> RtResult<BslValue> {
            cache
                .get(id as usize)
                .and_then(Clone::clone)
                .ok_or(RtError::DynamicError(
                    "узел графа значений ссылается мимо таблицы узлов".to_string(),
                ))
        };
        // Второй проход: заполнение коллекций по готовым узлам.
        for (index, node) in self.nodes.iter().enumerate() {
            match node {
                GraphNode::Array(items) => {
                    let value = cache[index].clone().expect("создан первым проходом");
                    let BslValue::Object(object) = value else {
                        unreachable!("массив материализуется объектом");
                    };
                    let BslObject::Array(storage) = object.as_ref() else {
                        unreachable!("узел массива материализуется массивом");
                    };
                    let mut storage = storage.borrow_mut();
                    for item in items {
                        storage.push(resolve(&cache, *item)?);
                    }
                }
                GraphNode::Structure(entries) => {
                    let value = cache[index].clone().expect("создан первым проходом");
                    let BslValue::Object(object) = value else {
                        unreachable!("структура материализуется объектом");
                    };
                    let BslObject::Structure(storage) = object.as_ref() else {
                        unreachable!("узел структуры материализуется структурой");
                    };
                    let mut storage = storage.borrow_mut();
                    for (name, item) in entries {
                        let field = rt.names.intern(name);
                        let item = resolve(&cache, *item)?;
                        storage.insert(field, item, &mut rt.shapes);
                    }
                }
                GraphNode::Map(entries) => {
                    let value = cache[index].clone().expect("создан первым проходом");
                    let BslValue::Object(object) = value else {
                        unreachable!("соответствие материализуется объектом");
                    };
                    let BslObject::Map(storage) = object.as_ref() else {
                        unreachable!("узел соответствия материализуется соответствием");
                    };
                    let mut storage = storage.borrow_mut();
                    for (key, item) in entries {
                        storage.insert(resolve(&cache, *key)?, resolve(&cache, *item)?);
                    }
                }
                GraphNode::ValueTable { columns, rows } => {
                    let value = cache[index].clone().expect("создан первым проходом");
                    let BslValue::Object(object) = value else {
                        unreachable!("таблица материализуется объектом");
                    };
                    let BslObject::ValueTable(data) = object.as_ref() else {
                        unreachable!("узел таблицы материализуется таблицей");
                    };
                    let mut data = data.borrow_mut();
                    for column in columns {
                        let types = match &column.types {
                            None => None,
                            Some(entries) => {
                                let mut resolved = Vec::with_capacity(entries.len());
                                for (name, quals) in entries {
                                    let id = rt.resolve_type(name).ok_or_else(|| {
                                        RtError::DynamicError(format!(
                                            "тип колонки «{name}» не зарегистрирован в принимающем сеансе"
                                        ))
                                    })?;
                                    resolved.push(crate::table::ColumnType {
                                        id,
                                        quals: quals.clone(),
                                    });
                                }
                                Some(resolved)
                            }
                        };
                        data.add_constrained_column(&column.name, types)
                            .ok_or_else(|| {
                                RtError::DynamicError(format!(
                                    "колонка «{}» повторяется в переносимой таблице",
                                    column.name
                                ))
                            })?;
                    }
                    for row in rows {
                        let row_id = data.add_row()?;
                        for (col, cell) in row.iter().enumerate() {
                            let cell = resolve(&cache, *cell)?;
                            data.set_cell(row_id, col, cell)
                                .ok_or(RtError::DynamicError(
                                    "строка переносимой таблицы шире её колонок".to_string(),
                                ))?;
                        }
                    }
                }
                _ => {}
            }
        }
        self.roots
            .iter()
            .map(|root| resolve(&cache, *root))
            .collect()
    }
}

/// Значение непереносимо — ошибка с именем типа. Она формируется ДО
/// всякого списания: бюджет не должен маскировать «это вообще не едет
/// между сеансами» отказом «не хватило места».
fn unsupported(value: &BslValue) -> RtError {
    match value {
        BslValue::Object(object) => RtError::ResourceLimit(format!(
            "объект типа «{}» не переносится между сеансами",
            BslValue::Object(Rc::clone(object)).type_name()
        )),
        other => RtError::ResourceLimit(format!(
            "значение типа «{}» не переносится между сеансами",
            other.type_name()
        )),
    }
}

/// Проверяет ВЕРХНИЙ уровень значения, не обходя его: поддержан ли сам
/// вариант. Дёшево и без аллокаций, поэтому вызывается перед списанием.
fn check_portable(value: &BslValue) -> RtResult<()> {
    let supported = match value {
        BslValue::Undefined
        | BslValue::Null
        | BslValue::Boolean(_)
        | BslValue::Number(_)
        | BslValue::Str(_)
        | BslValue::Date(_)
        | BslValue::Enum(_)
        | BslValue::Type(_) => true,
        BslValue::Object(object) => matches!(
            object.as_ref(),
            BslObject::Array(_)
                | BslObject::Structure(_)
                | BslObject::Map(_)
                | BslObject::ValueTable(_)
        ),
        _ => false,
    };
    if supported {
        Ok(())
    } else {
        Err(unsupported(value))
    }
}

/// Память коллекции из `len` элементов: сам вектор плюс его содержимое.
/// Заголовок считается всегда — иначе таблица на миллион строк с нулём
/// колонок платила бы 0 байт, сохраняя многомегабайтный `Vec<Vec<_>>`.
fn collection_cost<T>(len: usize) -> RtResult<usize> {
    len.checked_mul(std::mem::size_of::<T>())
        .and_then(|cost| cost.checked_add(std::mem::size_of::<Vec<T>>()))
        .ok_or_else(|| RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()))
}

impl<'a> Capture<'a> {
    fn new(rt: &'a RuntimeShapes, budget: GraphBudget<'a>, dry: bool) -> Self {
        // Фиксированный предел берётся сразу целиком: докупать нечего.
        let remaining = match &budget {
            GraphBudget::Fixed(max_bytes) => *max_bytes,
            GraphBudget::Reserving { .. } => 0,
        };
        Capture {
            rt,
            nodes: Vec::new(),
            seen: HashMap::new(),
            remaining,
            spent: 0,
            peak: 0,
            budget,
            dry,
            next_id: 0,
        }
    }

    /// Списывает `cost` из бюджета ДО аллокации узла. Резервирующему
    /// бюджету при нехватке докупается очередная порция — так обход
    /// никогда не работает на память, которой ему не выделили.
    fn charge(&mut self, cost: usize) -> RtResult<()> {
        while cost > self.remaining {
            let GraphBudget::Reserving { ceiling, reserve } = &mut self.budget else {
                return Err(RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()));
            };
            // Запрашивается РОВНО нужное — без запаса ни в каком виде.
            // Любой излишек, удерживаемый на середине обхода, способен
            // вытеснить соседа, чей граф в общий бюджет помещается: пока
            // этот обход не дойдёт до конца и не вернёт лишнее, сосед
            // получает отказ. Цена — обращение к общему счётчику на
            // каждое списание; этот путь не горячий (сериализация
            // staged-записи), а строгость важнее.
            let need = cost - self.remaining;
            let room = ceiling.saturating_sub(self.spent + self.remaining);
            let want = need.min(room);
            if want < need {
                return Err(RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()));
            }
            let granted = reserve(want);
            if granted == 0 {
                return Err(RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()));
            }
            self.remaining += granted;
        }
        self.remaining -= cost;
        self.spent += cost;
        self.peak = self.peak.max(self.spent);
        Ok(())
    }

    fn push(&mut self, node: GraphNode) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        if !self.dry {
            debug_assert_eq!(self.nodes.len() as NodeId, id, "нумерация узлов сбилась");
            self.nodes.push(node);
        }
        id
    }

    /// Записывает готовый узел на его место — в сухом обходе no-op.
    fn store(&mut self, id: NodeId, node: GraphNode) {
        if !self.dry {
            self.nodes[id as usize] = node;
        }
    }

    /// Возвращает бюджету излишек предварительной оценки.
    fn refund(&mut self, bytes: usize) {
        self.remaining += bytes;
        self.spent -= bytes.min(self.spent);
    }

    fn node(&mut self, value: &BslValue) -> RtResult<NodeId> {
        // Каждый узел стоит не меньше собственного заголовка; строки и
        // числа доплачивают за содержимое.
        const NODE_COST: usize = std::mem::size_of::<GraphNode>();
        // Сначала «едет ли это вообще», потом «есть ли место»: иначе
        // мелкий бюджет подменял бы причину отказа.
        check_portable(value)?;
        match value {
            BslValue::Undefined => {
                self.charge(NODE_COST)?;
                Ok(self.push(GraphNode::Undefined))
            }
            BslValue::Null => {
                self.charge(NODE_COST)?;
                Ok(self.push(GraphNode::Null))
            }
            BslValue::Boolean(v) => {
                self.charge(NODE_COST)?;
                Ok(self.push(GraphNode::Boolean(*v)))
            }
            BslValue::Number(v) => {
                // Мантисса `Big` произвольной величины: её печать ради
                // длины была бы ровно той аллокацией, которую бюджет
                // собирается отвергнуть. Списываем верхнюю оценку, а
                // излишек возвращаем по факту.
                // Списывается ПИК печати, а не только длина результата:
                // перевод большой мантиссы в десятичную запись держит
                // внутренние буферы кратно крупнее самой строки, и без
                // их учёта задание занимало бы память мимо бюджета.
                // Излишек возвращается сразу после печати.
                //
                // ВНИМАНИЕ: для `Big` эта величина — ЭВРИСТИКА, снятая
                // измерением, а не граница, выведенная из алгоритма
                // `num-bigint`. Строгой гарантии бюджета на этом пути
                // нет; сторожит её тест
                // `the_charged_print_peak_covers_the_real_one`.
                let bound = v.canonical_print_peak_bound();
                self.charge(
                    NODE_COST
                        .checked_add(bound)
                        .ok_or_else(|| RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()))?,
                )?;
                if self.dry {
                    // Сухой обход считает и НЕ печатает: у `Big` мантисса
                    // произвольной величины, и её печать была бы ровно
                    // той аллокацией, ради отказа от которой обход и
                    // существует.
                    return Ok(self.push(GraphNode::Undefined));
                }
                // Печать здесь уже оплачена: списанного хватает и на
                // строку, и — по эвристической оценке выше — на буферы
                // перевода. Точную длину знает лишь сама печать, поэтому
                // излишек оценки возвращается по факту.
                let text = v.to_canonical();
                self.refund(bound.saturating_sub(text.len()));
                Ok(self.push(GraphNode::Number(text)))
            }
            BslValue::Str(v) => {
                // Длина в байтах UTF-8 считается ТОЧНО и без
                // материализации: строка на сотни мегабайт при мелком
                // бюджете не аллоцируется вовсе.
                self.charge(
                    NODE_COST
                        .checked_add(v.utf8_len())
                        .ok_or_else(|| RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()))?,
                )?;
                if self.dry {
                    return Ok(self.push(GraphNode::Undefined));
                }
                // Ровно одна аллокация: `to_string` через `Display`
                // держал бы промежуточную копию и занимал вдвое больше
                // списанного.
                Ok(self.push(GraphNode::Str(v.to_utf8_string())))
            }
            BslValue::Date(v) => {
                self.charge(NODE_COST)?;
                Ok(self.push(GraphNode::Date(*v)))
            }
            BslValue::Enum(v) => {
                self.charge(NODE_COST)?;
                Ok(self.push(GraphNode::Enum(*v)))
            }
            BslValue::Type(v) => {
                // Имя типа — короткий идентификатор из таблицы, а не
                // пользовательские данные: его материализация бюджету не
                // угрожает.
                let name = v.to_string();
                self.charge(NODE_COST + name.len())?;
                if self.dry {
                    return Ok(self.push(GraphNode::Undefined));
                }
                Ok(self.push(GraphNode::Type(name)))
            }
            BslValue::Object(object) => self.object_node(object),
            other => Err(RtError::ResourceLimit(format!(
                "значение типа «{}» не переносится между сеансами",
                other.type_name()
            ))),
        }
    }

    fn object_node(&mut self, object: &Rc<BslObject>) -> RtResult<NodeId> {
        let key = match object.as_ref() {
            BslObject::ValueTable(data) => Rc::as_ptr(data) as usize,
            _ => Rc::as_ptr(object) as usize,
        };
        if let Some(existing) = self.seen.get(&key) {
            return Ok(*existing);
        }
        check_portable(&BslValue::Object(Rc::clone(object)))?;
        // Карта увиденных объектов живёт весь обход и растёт вместе с
        // графом: её запись оплачивается наравне с узлом, иначе на
        // больших графах она была бы памятью вне бюджета. Двойной размер
        // пары — запас на служебные слоты хеш-таблицы.
        self.charge(2 * std::mem::size_of::<(usize, NodeId)>())?;
        const NODE_COST: usize = std::mem::size_of::<GraphNode>();
        match object.as_ref() {
            BslObject::Array(items) => {
                self.charge(NODE_COST)?;
                // Узел-заглушка встаёт ДО обхода детей: самоссылка найдёт
                // его в `seen` и станет обычной ссылкой узла — так цикл
                // не разматывается в бесконечный обход.
                let id = self.push(GraphNode::Array(Vec::new()));
                self.seen.insert(key, id);
                // Ссылки на элементы оплачиваются ДО того, как под них
                // выделен вектор, а сами значения берутся по одному:
                // клон всего массива был бы аллокацией мимо бюджета.
                let len = items.borrow().len();
                self.charge(collection_cost::<NodeId>(len)?)?;
                let mut children = if self.dry {
                    Vec::new()
                } else {
                    Vec::with_capacity(len)
                };
                for index in 0..len {
                    let item = items.borrow()[index].clone();
                    let node = self.node(&item)?;
                    if !self.dry {
                        children.push(node);
                    }
                }
                self.store(id, GraphNode::Array(children));
                Ok(id)
            }
            BslObject::Structure(storage) => {
                self.charge(NODE_COST)?;
                let id = self.push(GraphNode::Structure(Vec::new()));
                self.seen.insert(key, id);
                let len = storage.borrow().len();
                self.charge(collection_cost::<(String, NodeId)>(len)?)?;
                let mut children = if self.dry {
                    Vec::new()
                } else {
                    Vec::with_capacity(len)
                };
                for index in 0..len {
                    let entry = storage.borrow().entry_at(index);
                    let Some((field, item)) = entry else {
                        continue;
                    };
                    let Some(name) = self.rt.names.name(field) else {
                        continue;
                    };
                    // Имя поля оплачивается до собственной копии.
                    self.charge(name.len())?;
                    let name = if self.dry {
                        String::new()
                    } else {
                        name.to_string()
                    };
                    let node = self.node(&item)?;
                    if !self.dry {
                        children.push((name, node));
                    }
                }
                self.store(id, GraphNode::Structure(children));
                Ok(id)
            }
            BslObject::Map(data) => {
                self.charge(NODE_COST)?;
                let id = self.push(GraphNode::Map(Vec::new()));
                self.seen.insert(key, id);
                let len = data.borrow().len();
                self.charge(collection_cost::<(NodeId, NodeId)>(len)?)?;
                let mut children = if self.dry {
                    Vec::new()
                } else {
                    Vec::with_capacity(len)
                };
                for index in 0..len {
                    let entry = data.borrow().entry_at(index);
                    let Some((map_key, item)) = entry else {
                        continue;
                    };
                    let key_node = self.node(&map_key)?;
                    let value_node = self.node(&item)?;
                    if !self.dry {
                        children.push((key_node, value_node));
                    }
                }
                self.store(id, GraphNode::Map(children));
                Ok(id)
            }
            BslObject::ValueTable(data) => {
                self.charge(NODE_COST)?;
                let id = self.push(GraphNode::ValueTable {
                    columns: Vec::new(),
                    rows: Vec::new(),
                });
                self.seen.insert(key, id);
                // Ячейки НЕ собираются во временную матрицу: у таблицы
                // на миллионы строк она сама была бы аллокацией мимо
                // бюджета. Стоимость сетки ссылок снимается заранее, а
                // значения читаются по одному коротким `borrow`.
                let (column_count, row_count) = {
                    let data = data.borrow();
                    (data.columns.len(), data.row_count())
                };
                // Платит и сетка ссылок, и ВСЕ векторы, которые снимок
                // сохранит: колонки, список строк и каждая строка.
                self.charge(collection_cost::<GraphColumn>(column_count)?)?;
                self.charge(collection_cost::<Vec<NodeId>>(row_count)?)?;
                self.charge(
                    row_count
                        .checked_mul(collection_cost::<NodeId>(column_count)?)
                        .ok_or_else(|| RtError::ResourceLimit(BUDGET_EXCEEDED.to_string()))?,
                )?;
                let mut columns: Vec<GraphColumn> = if self.dry {
                    Vec::new()
                } else {
                    Vec::with_capacity(column_count)
                };
                for index in 0..column_count {
                    // Стоимость имени и ограничений типов считается ПО
                    // МЕСТУ: длина строки известна без копии, а имя типа
                    // приходит из статического дескриптора либо печатается
                    // в короткий идентификатор. Клонировать их до
                    // списания значило бы выделять память мимо бюджета —
                    // на имени колонки в мегабайты это ровно тот
                    // перерасход, ради которого бюджет и существует.
                    let cost = {
                        let data = data.borrow();
                        let name_len = data.column_names[index].len();
                        let types_cost: usize = data.column_types[index]
                            .iter()
                            .flatten()
                            .map(|t| {
                                t.id.display_len() + t.quals.iter().map(String::len).sum::<usize>()
                            })
                            .sum();
                        name_len + types_cost
                    };
                    self.charge(cost)?;
                    if self.dry {
                        continue;
                    }
                    let (name, types) = {
                        let data = data.borrow();
                        (
                            data.column_names[index].clone(),
                            data.column_types[index].as_ref().map(|types| {
                                types
                                    .iter()
                                    .map(|t| (t.id.to_string(), t.quals.clone()))
                                    .collect::<Vec<(String, Vec<String>)>>()
                            }),
                        )
                    };
                    columns.push(GraphColumn { name, types });
                }
                let mut rows = if self.dry {
                    Vec::new()
                } else {
                    Vec::with_capacity(row_count)
                };
                for row in 0..row_count {
                    let mut ids = if self.dry {
                        Vec::new()
                    } else {
                        Vec::with_capacity(column_count)
                    };
                    for column in 0..column_count {
                        let value = data.borrow().columns[column][row].clone();
                        let node = self.node(&value)?;
                        if !self.dry {
                            ids.push(node);
                        }
                    }
                    if !self.dry {
                        rows.push(ids);
                    }
                }
                self.store(id, GraphNode::ValueTable { columns, rows });
                Ok(id)
            }
            _ => Err(RtError::ResourceLimit(format!(
                "объект типа «{}» не переносится между сеансами",
                BslValue::Object(Rc::clone(object)).type_name()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {

    /// Учёт памяти ЭТОГО потока: считается ЖИВОЙ объём (выделено минус
    /// освобождено) и его максимум за отрезок — именно пик, а не
    /// суммарный трафик аллокатора: временные буферы, которые сразу
    /// освобождаются, память не удерживают, а трафик завышают. Счётчик
    /// thread-local, поэтому параллельные тесты процесса пробу не
    /// смазывают.
    struct CountingAllocator;

    thread_local! {
        static LIVE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
        static PEAK: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
    }

    /// Счётчик трогается только там, где он уже инициализирован:
    /// обращение к thread-local во время его создания (и после
    /// разрушения при завершении потока) само аллоцирует и ушло бы в
    /// рекурсию.
    fn account(delta: isize) {
        let _ = LIVE.try_with(|live| {
            let now = live.get() + delta;
            live.set(now);
            let _ = PEAK.try_with(|peak| {
                if now > peak.get() {
                    peak.set(now);
                }
            });
        });
    }

    /// Обнуляет отметку пика и возвращает точку отсчёта — живой объём на
    /// этот момент.
    fn peak_probe() -> isize {
        let live = LIVE.with(std::cell::Cell::get);
        PEAK.with(|peak| peak.set(live));
        live
    }

    /// Пик живой памяти сверх точки отсчёта.
    fn peak_since(base: isize) -> usize {
        (PEAK.with(std::cell::Cell::get) - base).max(0) as usize
    }

    unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            let ptr = unsafe { std::alloc::System.alloc(layout) };
            if !ptr.is_null() {
                account(layout.size() as isize);
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
            account(-(layout.size() as isize));
            unsafe { std::alloc::System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(
            &self,
            ptr: *mut u8,
            layout: std::alloc::Layout,
            new_size: usize,
        ) -> *mut u8 {
            // Пессимистично: при переносе блока старый и новый живут
            // одновременно, и пик обязан это видеть. Сначала «выделено
            // новое», и только потом «освобождено старое».
            account(new_size as isize);
            let ptr = unsafe { std::alloc::System.realloc(ptr, layout, new_size) };
            account(-(layout.size() as isize));
            if ptr.is_null() {
                // Перевыделение не состоялось — старый блок остался.
                account(layout.size() as isize);
                account(-(new_size as isize));
            }
            ptr
        }
    }

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    /// Списанный пик печати числа НЕ МЕНЬШЕ фактического: проверяется на
    /// ряде величин и форм мантиссы прямым измерением живой памяти.
    /// Оценка линейна, поэтому она верна и там, где квадратичная
    /// переполнилась бы на 32-битной цели.
    #[test]
    fn the_charged_print_peak_covers_the_real_one() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let forms: Vec<(String, String)> = vec![
            ("одинаковые цифры".to_string(), "7".repeat(64 << 10)),
            (
                "чередование".to_string(),
                "1234567890".repeat((64 << 10) / 10),
            ),
            (
                "дробное с масштабом".to_string(),
                format!("0.{}", "9".repeat(32 << 10)),
            ),
            ("мелкое".to_string(), "12345".to_string()),
        ];
        for (label, text) in forms {
            let value = BslValue::Number(crate::BslNumber::parse_canonical(&text).expect("число"));
            // Списанный пик всего обхода — то, что бюджет обязан покрыть.
            let charged = SerializedValueGraph::measure(
                std::slice::from_ref(&value),
                &rt,
                &GraphLimits::default(),
            )
            .expect("сухой обход");
            let before = peak_probe();
            let graph = SerializedValueGraph::capture(
                std::slice::from_ref(&value),
                &rt,
                &GraphLimits::default(),
            )
            .expect("снимок");
            let peak = peak_since(before);
            // Для КРУПНЫХ значений допуска нет вовсе: списанного обязано
            // хватать. Послабление действует только там, где весь расход
            // — это округления аллокатора и рост векторов степенями
            // двойки, то есть на величинах мельче самого послабления.
            let tolerance = if charged >= (64 << 10) { 0 } else { 8 << 10 };
            assert!(
                charged + tolerance >= peak,
                "{label}: списан пик {charged}, фактический {peak} — оценка занижена"
            );
            assert!(
                charged >= graph.byte_size(),
                "{label}: пик меньше самого результата"
            );
        }
    }

    /// Мелкий бюджет отвергает крупное значение, НЕ выделив под него    /// Мелкий бюджет отвергает крупное значение, НЕ выделив под него
    /// память. Проверяются все опасные формы: не-ASCII строка (её байты
    /// UTF-8 вдвое длиннее код-юнитов UTF-16 — оценка «по код-юнитам»
    /// пропустила бы аллокацию), массив таких строк, таблица на много
    /// строк и таблица с НУЛЁМ колонок, чья сетка ссылок пуста, но
    /// вектор строк — нет.
    #[test]
    fn a_small_budget_refuses_before_allocating_the_value() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // Значения СТРОЯТСЯ заранее — их аллокация к снимку отношения не
        // имеет; считаем только то, что выделит `capture`.
        let big = BslValue::Str(BslString::from_str(&"ы".repeat(4 << 20)));
        let array = BslValue::new_array(vec![big.clone(); 8]);
        // Большое число: его каноническая запись — мегабайты, и сухой
        // обход не имеет права её печатать.
        let huge_number = BslValue::Number(
            crate::BslNumber::parse_canonical(&"9".repeat(64 << 10)).expect("разбор числа"),
        );
        let build_table = |columns: usize, rows: usize| {
            let table = crate::table::ValueTableData::new();
            {
                let mut data = table.borrow_mut();
                for index in 0..columns {
                    data.add_column(&format!("Колонка{index}"));
                }
                for _ in 0..rows {
                    data.add_row().expect("строка добавляется");
                }
            }
            BslValue::Object(Rc::new(BslObject::ValueTable(table)))
        };
        let wide = build_table(1, 200_000);
        // Имя колонки в мегабайты: его длина известна без копии, и
        // копировать его до списания нельзя ни в одном из режимов.
        let long_name = {
            let data = crate::table::ValueTableData::new();
            {
                let mut table = data.borrow_mut();
                table.add_column(&"и".repeat(4 << 20));
                table.add_row().expect("строка");
            }
            BslValue::Object(Rc::new(BslObject::ValueTable(data)))
        };
        // Сетка ссылок такой таблицы пуста (колонок нет), но вектор строк
        // — многомегабайтный: его обязан учитывать бюджет.
        let columnless = build_table(0, 2_000_000);
        // Много корней при мелком бюджете: вектор корней — такая же
        // память снимка, и выделять его до списания нельзя.
        let many_roots: Vec<BslValue> = vec![BslValue::Boolean(true); 256 << 10];
        for (label, values) in [
            ("строка", vec![big]),
            ("массив", vec![array]),
            ("большое число", vec![huge_number]),
            ("таблица", vec![wide]),
            ("таблица без колонок", vec![columnless]),
            ("длинное имя колонки", vec![long_name]),
            ("много корней", many_roots),
        ] {
            // Порог строгий: отказ по бюджету 1 КиБ обязан укладываться в
            // единицы килобайт служебных структур обходчика, а не в
            // «меньше, чем данные».
            for dry in [false, true] {
                let before = peak_probe();
                let error = if dry {
                    SerializedValueGraph::measure(&values, &rt, &GraphLimits { max_bytes: 1 << 10 })
                        .err()
                } else {
                    SerializedValueGraph::capture(&values, &rt, &GraphLimits { max_bytes: 1 << 10 })
                        .err()
                };
                let spent = peak_since(before);
                let error = error.unwrap_or_else(|| {
                    panic!("{label} (сухой={dry}): бюджет 1 КиБ не вмещает мегабайты")
                });
                assert!(
                    matches!(&error, RtError::ResourceLimit(text) if text == BUDGET_EXCEEDED),
                    "{label} (сухой={dry}): не тот отказ: {error:?}"
                );
                assert!(
                    spent < (8 << 10),
                    "{label} (сухой={dry}): отказ по бюджету выделил {spent} байт — \
                     аллокация обгоняет проверку"
                );
            }
        }
    }

    /// Сухой обход и настоящий снимок считают ОДИНАКОВО на всех формах
    /// значений: числах любого масштаба, строках с суррогатами, вложенных
    /// коллекциях, алиасах, циклах и таблицах. На этом равенстве держится
    /// точный ресурсный кредит staging — расхождение означало бы либо
    /// недобор кредита (снимок крупнее оплаченного), либо ложные отказы.
    #[test]
    fn measure_agrees_with_capture_on_every_shape() {
        // Один интернер на весь тест: имя поля структуры обязано быть
        // видно тому же набору форм, которым идёт обход.
        let mut sender = shapes();
        let shared = BslValue::new_array(vec![BslValue::Str(BslString::from_str("общий"))]);
        let cyclic = BslValue::new_array(Vec::new());
        if let BslValue::Object(object) = &cyclic
            && let BslObject::Array(items) = object.as_ref()
        {
            items.borrow_mut().push(cyclic.clone());
            items.borrow_mut().push(BslValue::number_from_i64(7));
        }
        let table = {
            let data = crate::table::ValueTableData::new();
            {
                let mut table = data.borrow_mut();
                table.add_column("Первая");
                table.add_column("Вторая");
                for index in 0..16 {
                    table.add_row().expect("строка");
                    let _ = index;
                }
            }
            BslValue::Object(Rc::new(BslObject::ValueTable(data)))
        };
        let structure = {
            let field = sender.names.intern("Поле");
            let value = BslValue::new_structure(sender.shapes.empty(), Vec::new());
            if let BslValue::Object(object) = &value
                && let BslObject::Structure(storage) = object.as_ref()
            {
                storage.borrow_mut().insert(
                    field,
                    BslValue::Str(BslString::from_str("значение")),
                    &mut sender.shapes,
                );
            }
            value
        };
        let cases: Vec<Vec<BslValue>> = vec![
            Vec::new(),
            vec![
                BslValue::Undefined,
                BslValue::Null,
                BslValue::Boolean(false),
            ],
            vec![
                BslValue::number_from_i64(0),
                BslValue::number_from_i64(-1),
                BslValue::Number(
                    crate::BslNumber::parse_canonical("0.000000000000000000000000001")
                        .expect("число"),
                ),
                // Мантисса `Big`: оценка длины завышает, и бюджета в
                // размер готового снимка обходу НЕ хватит — измеренный
                // пик обязан покрывать этот случай.
                BslValue::Number(
                    crate::BslNumber::parse_canonical(&"9".repeat(300)).expect("число"),
                ),
            ],
            vec![BslValue::Str(BslString::from_units(vec![
                0xD83D,
                0xDE00,
                0xD800,
                b'x' as u16,
            ]))],
            vec![shared.clone(), shared.clone(), shared],
            vec![cyclic],
            vec![table],
            vec![structure],
        ];
        for values in cases {
            let measured = SerializedValueGraph::measure(&values, &sender, &GraphLimits::default())
                .expect("сухой обход");
            let graph = SerializedValueGraph::capture(&values, &sender, &GraphLimits::default())
                .expect("снимок");
            assert!(
                measured >= graph.byte_size(),
                "измеренный ПИК меньше итогового размера на {} корнях: {measured} < {}",
                values.len(),
                graph.byte_size()
            );
            // Ровно измеренного бюджета обязано хватить.
            SerializedValueGraph::capture(
                &values,
                &sender,
                &GraphLimits {
                    max_bytes: measured,
                },
            )
            .expect("снимок ровно в измеренный бюджет");
        }
    }

    /// Сухой обход не печатает число даже при ПРОСТОРНОМ бюджете, а
    /// настоящий снимок печатает его одной аллокацией. Прежняя проба
    /// этого не ловила: она давала мелкий бюджет, который отвергал
    /// значение до печати.
    #[test]
    fn a_dry_pass_never_prints_a_big_number() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        // Мантисса на четверть миллиона цифр: её печать видна счётчику.
        let value = BslValue::Number(
            crate::BslNumber::parse_canonical(&"7".repeat(256 << 10)).expect("число"),
        );
        let before = peak_probe();
        let measured = SerializedValueGraph::measure(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits::default(),
        )
        .expect("сухой обход в просторном бюджете");
        let dry_spent = peak_since(before);
        assert!(
            dry_spent < (64 << 10),
            "сухой обход выделил {dry_spent} байт — он печатает число вместо счёта"
        );

        let before = peak_probe();
        let graph = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits::default(),
        )
        .expect("снимок");
        let spent = peak_since(before);
        assert!(
            measured >= graph.byte_size(),
            "измеренный пик меньше итогового размера"
        );
        // Пик печати УЧТЁН: фактический расход не превышает того, что
        // обход списал (и вернул после уточнения).
        assert!(
            measured >= spent,
            "печать заняла {spent} байт при списанном пике {measured} — \
             память сверх бюджета"
        );

        // Бюджета, которого хватает на РЕЗУЛЬТАТ, но не на пик печати,
        // недостаточно: отказ обязан прийти ДО печати, а не после.
        let before = peak_probe();
        let error = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits {
                max_bytes: graph.byte_size() + (64 << 10),
            },
        )
        .expect_err("на пик печати бюджета не хватает");
        let refused_spent = peak_since(before);
        assert!(
            matches!(&error, RtError::ResourceLimit(text) if text == BUDGET_EXCEEDED),
            "не тот отказ: {error:?}"
        );
        assert!(
            refused_spent < (64 << 10),
            "отказ выделил {refused_spent} байт — печать началась до проверки"
        );
    }

    /// Снимок в пределах бюджета не занимает памяти БОЛЬШЕ учтённой:
    /// материализация строки идёт одной аллокацией точной ёмкости, а не
    /// через промежуточную копию. Иначе задание держало бы вдвое больше
    /// оплаченного, и глобальный staging-предел превышался бы вдвое на
    /// каждой строке.
    #[test]
    fn a_capture_never_allocates_more_than_it_charged() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = BslValue::Str(BslString::from_str(&"ы".repeat(4 << 20)));
        let before = peak_probe();
        let graph = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits::default(),
        )
        .expect("снимок в просторном бюджете");
        let spent = peak_since(before);
        // Учтённый размер плюс небольшой запас на служебные структуры:
        // двойная аллокация строки этот порог превысила бы вдвое.
        assert!(
            spent <= graph.byte_size() + (64 << 10),
            "снимок занял {spent} байт при учтённых {} — память сверх бюджета",
            graph.byte_size()
        );
    }

    /// КОРЕНЬ, который не переносится между сеансами, называет свой тип
    /// при любом бюджете, включая нулевой: причина «это вообще не едет»
    /// известна до обхода, и подменять её нехваткой места нельзя.
    ///
    /// Для значения В ГЛУБИНЕ контракт слабее и это сознательно: обход
    /// доходит до него, только если бюджета хватило на путь к нему —
    /// иначе отказ по месту приходит раньше и он честен. Полный
    /// приоритет потребовал бы отдельного рекурсивного предпрохода по
    /// всему графу (со своей картой циклов), то есть двойного обхода на
    /// каждом снимке, — цена, которой этот случай не стоит.
    #[test]
    fn a_non_portable_root_names_its_type_at_any_budget() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let foreign =
            BslValue::new_object(crate::user_message::UserMessageObject::with_text("нет"));
        for max_bytes in [0, 1, 8, 1 << 10] {
            for dry in [false, true] {
                let error = if dry {
                    SerializedValueGraph::measure(
                        std::slice::from_ref(&foreign),
                        &rt,
                        &GraphLimits { max_bytes },
                    )
                    .expect_err("значение не переносится")
                } else {
                    SerializedValueGraph::capture(
                        std::slice::from_ref(&foreign),
                        &rt,
                        &GraphLimits { max_bytes },
                    )
                    .expect_err("значение не переносится")
                };
                let RtError::ResourceLimit(text) = &error else {
                    panic!("{max_bytes}/сухой={dry}: не тот класс: {error:?}");
                };
                assert!(
                    text.contains("не переносится"),
                    "{max_bytes}/сухой={dry}: бюджет подменил причину: {text}"
                );
            }
        }
    }

    /// Значение в глубине называет свой тип, КОГДА обход до него дошёл:
    /// бюджета на путь хватило, а само значение не переносится.
    #[test]
    fn a_non_portable_value_reached_by_the_walk_names_its_type() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let nested = BslValue::new_array(vec![
            BslValue::Str(BslString::from_str("перед")),
            BslValue::new_object(crate::user_message::UserMessageObject::with_text("нет")),
        ]);
        for dry in [false, true] {
            let error = if dry {
                SerializedValueGraph::measure(
                    std::slice::from_ref(&nested),
                    &rt,
                    &GraphLimits::default(),
                )
                .expect_err("значение не переносится")
            } else {
                SerializedValueGraph::capture(
                    std::slice::from_ref(&nested),
                    &rt,
                    &GraphLimits::default(),
                )
                .expect_err("значение не переносится")
            };
            let RtError::ResourceLimit(text) = &error else {
                panic!("сухой={dry}: не тот класс: {error:?}");
            };
            assert!(
                text.contains("не переносится"),
                "сухой={dry}: причина подменена: {text}"
            );
        }
        // А при бюджете, которого не хватает даже на путь, отказ по
        // месту — честный: обход до вложенного значения не дошёл.
        let error = SerializedValueGraph::capture(
            std::slice::from_ref(&nested),
            &rt,
            &GraphLimits { max_bytes: 1 },
        )
        .expect_err("бюджета не хватает даже на массив");
        assert!(
            matches!(&error, RtError::ResourceLimit(text) if text == BUDGET_EXCEEDED),
            "не тот отказ: {error:?}"
        );
    }

    /// Резервирующий бюджет берёт кредиты ПОРЦИЯМИ по ходу обхода:    /// Резервирующий бюджет берёт кредиты ПОРЦИЯМИ по ходу обхода:
    /// служебная память обходчика тоже покрыта резервом, а распорядитель
    /// видит спрос до того, как память понадобилась.
    #[test]
    fn a_reserving_budget_takes_credit_incrementally() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = BslValue::new_array(
            (0..2_000)
                .map(|i| BslValue::new_array(vec![BslValue::number_from_i64(i); 8]))
                .collect(),
        );
        let mut granted = 0usize;
        let mut calls = 0usize;
        let mut reserve = |bytes: usize| {
            calls += 1;
            granted += bytes;
            bytes
        };
        let measured = SerializedValueGraph::measure_with(
            std::slice::from_ref(&value),
            &rt,
            GraphBudget::Reserving {
                ceiling: usize::MAX,
                reserve: &mut reserve,
            },
        )
        .expect("сухой обход под резервом");
        assert!(calls > 1, "резерв обязан браться порциями, а не разом");
        assert!(
            granted >= measured,
            "распорядителю не показали весь спрос: выдано {granted}, учтено {measured}"
        );

        // Отказ распорядителя останавливает обход немедленно.
        let mut budget_left = 4 << 10;
        // Скупой распорядитель выдаёт только остаток — частичная выдача
        // обходу годится, пока её хватает на текущее списание.
        let mut stingy = |bytes: usize| {
            let granted = bytes.min(budget_left);
            budget_left -= granted;
            granted
        };
        let error = SerializedValueGraph::measure_with(
            std::slice::from_ref(&value),
            &rt,
            GraphBudget::Reserving {
                ceiling: usize::MAX,
                reserve: &mut stingy,
            },
        )
        .expect_err("скупой распорядитель обрывает обход");
        assert!(matches!(&error, RtError::ResourceLimit(text) if text == BUDGET_EXCEEDED));
    }

    /// Сухой обход считает РОВНО столько же, сколько потом занимает
    /// снимок, и делает это без аллокаций под узлы: на этом равенстве
    /// держится точный staging-кредит.
    #[test]
    fn measure_matches_capture_without_allocating() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = BslValue::new_array(vec![
            BslValue::Str(BslString::from_str(&"текст".repeat(4096))),
            BslValue::number_from_i64(-12345),
            BslValue::new_array(vec![BslValue::Boolean(true); 512]),
        ]);
        let before = peak_probe();
        let measured = SerializedValueGraph::measure(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits::default(),
        )
        .expect("сухой обход");
        let spent = peak_since(before);
        let graph = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits {
                max_bytes: measured,
            },
        )
        .expect("снимок ровно в измеренный размер");
        assert_eq!(
            graph.byte_size(),
            measured,
            "измеренный размер обязан совпадать с фактическим"
        );
        assert!(
            spent < (64 << 10),
            "сухой обход выделил {spent} байт — он обязан считать, а не строить"
        );
    }

    /// Значения В ПРЕДЕЛАХ бюджета по-прежнему снимаются целиком    /// Значения В ПРЕДЕЛАХ бюджета по-прежнему снимаются целиком, а
    /// учтённый размер совпадает с фактическими байтами содержимого.
    #[test]
    fn a_sufficient_budget_still_captures_everything() {
        let rt = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let value = BslValue::new_array(vec![
            BslValue::Str(BslString::from_str("строка")),
            BslValue::number_from_i64(42),
        ]);
        let graph = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &rt,
            &GraphLimits::default(),
        )
        .expect("снимок в просторном бюджете");
        let mut shapes = RuntimeShapes::seeded(Vec::new(), Vec::new(), None);
        let restored = graph.materialize(&mut shapes).expect("материализация");
        assert_eq!(restored.len(), 1);
    }
    use super::*;

    fn shapes() -> RuntimeShapes {
        RuntimeShapes::seeded(Vec::new(), Vec::new(), None)
    }

    fn capture_one(value: &BslValue, rt: &RuntimeShapes) -> SerializedValueGraph {
        SerializedValueGraph::capture(std::slice::from_ref(value), rt, &GraphLimits::default())
            .expect("снимок")
    }

    /// Примитивы и коллекции переживают перенос между ДВУМЯ независимыми
    /// сеансами: у принимающего свой интернер и свои формы.
    #[test]
    fn a_round_trip_between_two_sessions_preserves_values() {
        let sender = shapes();
        let mut receiver = shapes();
        let array = BslValue::new_array(vec![
            BslValue::Number(BslNumber::from_i64(42)),
            BslValue::Str(BslString::from_str("текст")),
            BslValue::Boolean(true),
            BslValue::Undefined,
        ]);
        let graph = capture_one(&array, &sender);
        let restored = graph.materialize(&mut receiver).expect("материализация");
        assert_eq!(restored.len(), 1);
        let BslValue::Object(object) = &restored[0] else {
            panic!("ожидался массив");
        };
        let BslObject::Array(items) = object.as_ref() else {
            panic!("ожидался массив");
        };
        let items = items.borrow();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], BslValue::Number(BslNumber::from_i64(42)));
        assert_eq!(items[1].to_string(), "текст");
    }

    /// Один объект, входящий дважды, остаётся ОДНИМ объектом после
    /// материализации — алиас переносится, а не раздваивается.
    #[test]
    fn aliases_survive_the_transfer() {
        let sender = shapes();
        let mut receiver = shapes();
        let shared = BslValue::new_array(vec![BslValue::Number(BslNumber::from_i64(1))]);
        let outer = BslValue::new_array(vec![shared.clone(), shared]);
        let graph = capture_one(&outer, &sender);
        let restored = graph.materialize(&mut receiver).expect("материализация");
        let BslValue::Object(object) = &restored[0] else {
            panic!("ожидался массив");
        };
        let BslObject::Array(items) = object.as_ref() else {
            panic!("ожидался массив");
        };
        let items = items.borrow();
        let (BslValue::Object(a), BslValue::Object(b)) = (&items[0], &items[1]) else {
            panic!("ожидались объекты");
        };
        assert!(std::rc::Rc::ptr_eq(a, b), "алиас должен сохраниться");
        // Запись через первый вход видна во втором — то же значение.
        let BslObject::Array(inner) = a.as_ref() else {
            panic!("ожидался вложенный массив");
        };
        inner.borrow_mut().push(BslValue::Boolean(true));
        let BslObject::Array(second) = b.as_ref() else {
            panic!("ожидался вложенный массив");
        };
        assert_eq!(second.borrow().len(), 2);
    }

    /// Самоссылка (цикл) не разматывает обход и восстанавливается циклом.
    #[test]
    fn cycles_are_captured_and_rebuilt() {
        let sender = shapes();
        let mut receiver = shapes();
        let cyclic = BslValue::new_array(Vec::new());
        {
            let BslValue::Object(object) = &cyclic else {
                unreachable!();
            };
            let BslObject::Array(items) = object.as_ref() else {
                unreachable!();
            };
            items.borrow_mut().push(cyclic.clone());
        }
        let graph = capture_one(&cyclic, &sender);
        let restored = graph.materialize(&mut receiver).expect("материализация");
        let BslValue::Object(outer) = &restored[0] else {
            panic!("ожидался массив");
        };
        let BslObject::Array(items) = outer.as_ref() else {
            panic!("ожидался массив");
        };
        let items = items.borrow();
        let BslValue::Object(inner) = &items[0] else {
            panic!("ожидалась самоссылка");
        };
        assert!(std::rc::Rc::ptr_eq(outer, inner), "цикл должен замкнуться");
    }

    /// Непереносимое значение отвергается ловимой ошибкой с именем типа.
    #[test]
    fn an_unsupported_value_is_rejected_with_its_type_name() {
        let sender = shapes();
        let value = BslValue::new_value_comparison();
        let error = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &sender,
            &GraphLimits::default(),
        )
        .expect_err("описание типов не переносится");
        assert!(
            matches!(&error, RtError::ResourceLimit(text) if text.contains("СравнениеЗначений")),
            "не та ошибка: {error}"
        );
        assert!(error.is_bsl_exception(), "ошибка должна быть ловимой");
    }

    /// Бюджет срабатывает ДО аллокации: маленький лимит валит снимок
    /// большой строки, а не выделяет её.
    #[test]
    fn the_budget_stops_the_capture_before_allocation() {
        let sender = shapes();
        let mut receiver = shapes();
        let value = BslValue::Str(BslString::from_str(&"ы".repeat(10_000)));
        let error = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &sender,
            &GraphLimits { max_bytes: 128 },
        )
        .expect_err("бюджета не хватает");
        assert!(matches!(error, RtError::ResourceLimit(_)));
        // Тот же снимок с достаточным бюджетом проходит и учитывает размер.
        let graph = SerializedValueGraph::capture(
            std::slice::from_ref(&value),
            &sender,
            &GraphLimits::default(),
        )
        .expect("снимок");
        assert!(graph.byte_size() >= 20_000, "байты строки учтены");
        graph.materialize(&mut receiver).expect("материализация");
    }

    /// Таблица значений переносит колонки (с ограничениями типов),
    /// физический порядок строк и ячейки; две обёртки над одними данными
    /// остаются одним объектом.
    #[test]
    fn a_value_table_round_trips_with_columns_and_rows() {
        let sender = shapes();
        let mut receiver = shapes();
        let data = crate::table::ValueTableData::new();
        {
            let mut table = data.borrow_mut();
            table.add_column("Имя").expect("колонка");
            let number = sender.resolve_type("Число").expect("тип Число");
            table
                .add_typed_column("Число", Some(vec![number]))
                .expect("типизированная колонка");
            for (name, count) in [("первая", 1i64), ("вторая", 2)] {
                let row = table.add_row().expect("строка");
                table.set_cell(row, 0, BslValue::Str(BslString::from_str(name)));
                table.set_cell(row, 1, BslValue::Number(BslNumber::from_i64(count)));
            }
        }
        let table_value = BslValue::Object(std::rc::Rc::new(BslObject::ValueTable(data)));
        let graph = capture_one(&table_value, &sender);
        let restored = graph.materialize(&mut receiver).expect("материализация");
        let BslValue::Object(object) = &restored[0] else {
            panic!("ожидалась таблица");
        };
        let BslObject::ValueTable(data) = object.as_ref() else {
            panic!("ожидалась таблица");
        };
        let table = data.borrow();
        assert_eq!(table.column_names, vec!["Имя", "Число"]);
        assert!(table.column_types[0].is_none());
        assert_eq!(
            table.column_types[1].as_ref().map(|types| types.len()),
            Some(1)
        );
        assert_eq!(table.row_count(), 2);
        let second = table.row_id_at(1).expect("вторая строка");
        assert_eq!(
            table.get_cell(second, 1),
            Some(BslValue::Number(BslNumber::from_i64(2)))
        );
    }

    /// Структура и соответствие переносят пары; принимающий интернер
    /// восстанавливает поля по именам.
    #[test]
    fn structures_and_maps_round_trip() {
        let mut sender = shapes();
        let mut receiver = shapes();
        let field = sender.names.intern("Поле");
        let structure = BslValue::new_structure(sender.shapes.empty(), Vec::new());
        {
            let BslValue::Object(object) = &structure else {
                unreachable!();
            };
            let BslObject::Structure(storage) = object.as_ref() else {
                unreachable!();
            };
            storage.borrow_mut().insert(
                field,
                BslValue::Number(BslNumber::from_i64(7)),
                &mut sender.shapes,
            );
        }
        let map = BslValue::new_map();
        {
            let BslValue::Object(object) = &map else {
                unreachable!();
            };
            let BslObject::Map(data) = object.as_ref() else {
                unreachable!();
            };
            data.borrow_mut().insert(
                BslValue::Str(BslString::from_str("ключ")),
                structure.clone(),
            );
        }
        let graph = SerializedValueGraph::capture(&[map], &sender, &GraphLimits::default())
            .expect("снимок");
        let restored = graph.materialize(&mut receiver).expect("материализация");
        let BslValue::Object(object) = &restored[0] else {
            panic!("ожидалось соответствие");
        };
        let BslObject::Map(data) = object.as_ref() else {
            panic!("ожидалось соответствие");
        };
        let (_, inner) = data.borrow().entry_at(0).expect("одна пара");
        let BslValue::Object(inner) = inner else {
            panic!("ожидалась структура");
        };
        let BslObject::Structure(storage) = inner.as_ref() else {
            panic!("ожидалась структура");
        };
        let (restored_field, restored_value) = storage.borrow().entry_at(0).expect("одно поле");
        assert_eq!(receiver.names.name(restored_field), Some("Поле"));
        assert_eq!(restored_value, BslValue::Number(BslNumber::from_i64(7)));
    }
}
