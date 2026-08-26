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

/// Обходчик снимка: дедуп по указателю объекта и бюджет байтов.
struct Capture<'a> {
    rt: &'a RuntimeShapes,
    nodes: Vec<GraphNode>,
    seen: HashMap<*const BslObject, NodeId>,
    remaining: usize,
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
        let mut capture = Capture {
            rt,
            nodes: Vec::new(),
            seen: HashMap::new(),
            remaining: limits.max_bytes,
        };
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            roots.push(capture.node(value)?);
        }
        Ok(Self {
            nodes: capture.nodes,
            roots,
            byte_size: limits.max_bytes - capture.remaining,
        })
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
                _ => {}
            }
        }
        self.roots
            .iter()
            .map(|root| resolve(&cache, *root))
            .collect()
    }
}

impl Capture<'_> {
    /// Списывает `cost` из бюджета ДО аллокации узла.
    fn charge(&mut self, cost: usize) -> RtResult<()> {
        if cost > self.remaining {
            return Err(RtError::ResourceLimit(
                "снимок значений превысил бюджет памяти".to_string(),
            ));
        }
        self.remaining -= cost;
        Ok(())
    }

    fn push(&mut self, node: GraphNode) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(node);
        id
    }

    fn node(&mut self, value: &BslValue) -> RtResult<NodeId> {
        // Каждый узел стоит не меньше собственного заголовка; строки и
        // числа доплачивают за содержимое.
        const NODE_COST: usize = 32;
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
                let text = v.to_canonical();
                self.charge(NODE_COST + text.len())?;
                Ok(self.push(GraphNode::Number(text)))
            }
            BslValue::Str(v) => {
                let text = v.to_string();
                self.charge(NODE_COST + text.len())?;
                Ok(self.push(GraphNode::Str(text)))
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
                let name = v.to_string();
                self.charge(NODE_COST + name.len())?;
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
        let key: *const BslObject = Rc::as_ptr(object);
        if let Some(existing) = self.seen.get(&key) {
            return Ok(*existing);
        }
        const NODE_COST: usize = 32;
        match object.as_ref() {
            BslObject::Array(items) => {
                self.charge(NODE_COST)?;
                // Узел-заглушка встаёт ДО обхода детей: самоссылка найдёт
                // его в `seen` и станет обычной ссылкой узла — так цикл
                // не разматывается в бесконечный обход.
                let id = self.push(GraphNode::Array(Vec::new()));
                self.seen.insert(key, id);
                let snapshot: Vec<BslValue> = items.borrow().clone();
                let mut children = Vec::with_capacity(snapshot.len());
                for item in &snapshot {
                    self.charge(std::mem::size_of::<NodeId>())?;
                    children.push(self.node(item)?);
                }
                self.nodes[id as usize] = GraphNode::Array(children);
                Ok(id)
            }
            BslObject::Structure(storage) => {
                self.charge(NODE_COST)?;
                let id = self.push(GraphNode::Structure(Vec::new()));
                self.seen.insert(key, id);
                let entries: Vec<(String, BslValue)> = {
                    let storage = storage.borrow();
                    (0..storage.len())
                        .filter_map(|i| storage.entry_at(i))
                        .filter_map(|(field, item)| {
                            self.rt
                                .names
                                .name(field)
                                .map(|name| (name.to_string(), item))
                        })
                        .collect()
                };
                let mut children = Vec::with_capacity(entries.len());
                for (name, item) in entries {
                    self.charge(name.len() + std::mem::size_of::<NodeId>())?;
                    let node = self.node(&item)?;
                    children.push((name, node));
                }
                self.nodes[id as usize] = GraphNode::Structure(children);
                Ok(id)
            }
            BslObject::Map(data) => {
                self.charge(NODE_COST)?;
                let id = self.push(GraphNode::Map(Vec::new()));
                self.seen.insert(key, id);
                let entries: Vec<(BslValue, BslValue)> = {
                    let data = data.borrow();
                    (0..data.len()).filter_map(|i| data.entry_at(i)).collect()
                };
                let mut children = Vec::with_capacity(entries.len());
                for (map_key, item) in &entries {
                    self.charge(2 * std::mem::size_of::<NodeId>())?;
                    let key_node = self.node(map_key)?;
                    let value_node = self.node(item)?;
                    children.push((key_node, value_node));
                }
                self.nodes[id as usize] = GraphNode::Map(children);
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
