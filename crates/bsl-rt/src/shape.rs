use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interner::NameId;

/// Форма структуры ("hidden class"): упорядоченный список полей (порядок
/// вставки — важен для `Для Каждого`) плюс индекс имя -> слот для доступа
/// за O(1).
///
/// `transitions`/`remove_transitions` — кэш переходов ("эта форма + это
/// поле -> следующая форма"), заполняемый `ShapeTable::add_field`/
/// `remove_field` в рантайме (`Вставить`/`Удалить` на структуре): без него
/// `Вставить` в цикле пересобирала бы `Vec<NameId>` из всех имён на каждой
/// итерации и заново хэшировала его в `ShapeTable::by_keys`. Это ТОЛЬКО
/// быстрый путь — истина об идентичности формы остаётся в
/// `ShapeTable::by_keys` (см. его doc comment): переход всегда либо находит
/// уже интернированную форму с тем же итоговым набором полей, либо заводит
/// её и кэширует здесь. `RefCell`, а не `&mut` — формы разделяются через
/// `Rc` между структурами и самим кэшем компиляции, `&mut Shape` при этом
/// недостижим.
///
/// `depth` — сколько шагов `add_field` пройдено от формы, интернированной
/// НЕ переходом (литерал `Новый Структура("x,y,z")` на этапе компиляции —
/// всегда `0`). Это счётчик для деградации в словарь, см.
/// `MAX_SHAPE_TRANSITIONS`. Значение фиксируется в момент создания формы и
/// больше не меняется: форма разделяется между всеми структурами с этим
/// набором полей, и «подкрутка» счётчика одной из них замедлила бы
/// остальные.
#[derive(Debug)]
pub struct Shape {
    pub names: Vec<NameId>,
    pub index: HashMap<NameId, u32>,
    pub depth: u32,
    transitions: RefCell<HashMap<NameId, Rc<Shape>>>,
    remove_transitions: RefCell<HashMap<NameId, Rc<Shape>>>,
}

/// После стольких `Вставить` подряд структура переходит на словарное
/// хранение. Значение из того же диапазона, что dictionary-mode порог
/// в V8: достаточно, чтобы литеральные структуры и разумные `Вставить`
/// оставались на быстром пути, и достаточно мало, чтобы цикл с
/// динамическими именами не съел память.
///
/// Зачем вообще порог: `ShapeTable::shapes` держит СИЛЬНУЮ ссылку на
/// каждую заведённую форму до конца исполнения, а `transitions`/
/// `remove_transitions` вдобавок образуют циклы `Rc` — выселить форму
/// нельзя в принципе. Пока ключи приходили только из литералов, форм было
/// конечное число и это было правильно. `Стр.Вставить("Поле" + Строка(и),
/// и)` в цикле на миллион итераций интернирует миллион форм с растущим
/// списком имён — квадратичная память и достижимый OOM на обычном
/// пользовательском скрипте. Порог обрывает эту цепочку: после него
/// объект хранит поля словарём и форм больше не заводит вообще.
pub const MAX_SHAPE_TRANSITIONS: u32 = 48;

/// Таблица интернирования форм. Изначально была целиком компиляционной
/// сущностью (см. `NameInterner`), но `Вставить`/`Удалить`/`Свойство` на
/// структуре меняют набор полей уже ПОСЛЕ компиляции — так что ровно та же
/// таблица теперь живёт и в рантайме: VM затравливает свой экземпляр уже
/// готовыми формами модуля (`from_existing`) и продолжает интернировать в
/// НЕГО ЖЕ по ходу исполнения (`add_field`/`remove_field`), вместо того
/// чтобы заводить формы вне таблицы. Именно это и держит интернирование
/// ГЛОБАЛЬНЫМ по нормализованному списку полей всё время исполнения, а не
/// только на этапе компиляции: если бы каждый вызов `Новый
/// Структура("x,y,z", ...)` — или каждый независимый путь `Вставить` до
/// одного и того же итогового набора полей — заводил свой объект формы,
/// инлайн-кэш на горячий доступ к полю видел бы N разных указателей вместо
/// одного — полиморфный кэш вместо мономорфного.
#[derive(Debug, Default)]
pub struct ShapeTable {
    by_keys: HashMap<Vec<NameId>, u32>,
    shapes: Vec<Rc<Shape>>,
}

impl ShapeTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Затравка рантайм-таблицы уже готовыми (компиляционными) формами
    /// модуля — они становятся предзаполнением, а не отдельным, навсегда
    /// замороженным списком: `add_field`/`remove_field` продолжают
    /// интернировать в ЭТУ ЖЕ таблицу.
    pub fn from_existing(shapes: Vec<Rc<Shape>>) -> Self {
        let by_keys = shapes
            .iter()
            .enumerate()
            .map(|(i, s)| (s.names.clone(), i as u32))
            .collect();
        ShapeTable { by_keys, shapes }
    }

    /// Возвращает индекс формы в итоговой таблице (см. `into_shapes`),
    /// заводя новую только если такого набора полей ещё не было.
    ///
    /// ВНИМАНИЕ: это путь ЛИТЕРАЛА (`Новый Структура("x,y,z")`) — форма
    /// получает `depth = 0`, то есть полный запас переходов. Сейчас
    /// `bsl-sema::resolver::resolve_new` требует, чтобы список ключей был
    /// строковым ЛИТЕРАЛОМ (иначе `SemaError::Unsupported`), поэтому число
    /// таких форм конечно и известно на этапе компиляции. Когда появится
    /// `Новый Структура(строка_вычисленная_в_рантайме)`, интернировать её
    /// ключи сюда НЕЛЬЗЯ — каждая итерация цикла давала бы новую бессмертную
    /// форму: такой конструктор должен сразу создавать структуру в словарном
    /// режиме (`StructureStorage::Dictionary`), минуя таблицу форм.
    pub fn intern(&mut self, names: &[NameId]) -> u32 {
        self.intern_with_depth(names, 0)
    }

    /// ВНИМАНИЕ ПРО `depth` НА УЖЕ СУЩЕСТВУЮЩЕЙ ФОРМЕ: возвращаемая ниже
    /// по раннему выходу форма сохраняет СВОЮ исходную глубину, а
    /// запрошенный `depth` игнорируется — глубина фиксируется в момент
    /// создания и потом не меняется (см. doc comment на `Shape::depth`:
    /// форма разделяется всеми структурами с этим набором полей, и
    /// «подкрутка» счётчика ради одной замедлила бы остальные).
    ///
    /// Отсюда следствие, которое надо знать, а не открывать заново: набор
    /// ключей, достижимый И из литерала (`intern`, `depth = 0`), И по
    /// цепочке переходов (`add_field`), ОБНУЛЯЕТ счётчик для идущих через
    /// него цепочек — от такой формы доступны ещё `MAX_SHAPE_TRANSITIONS`
    /// шагов, сколько бы переходов до неё уже ни было сделано. То есть
    /// фактический потолок выше номинального.
    ///
    /// Это не баг, и защита от OOM не рушится: рост всё равно ограничен
    /// сверху примерно как «число литеральных форм × `MAX_SHAPE_TRANSITIONS`»,
    /// а литеральных форм конечное число, известное на этапе компиляции
    /// (`resolve_new` требует строкового ЛИТЕРАЛА в списке ключей, см.
    /// `intern` выше). Растущая цепочка динамических `Вставить` попадает на
    /// литеральную форму только случайным совпадением набора полей, а не
    /// систематически, поэтому обнуление счётчика не превращается в
    /// неограниченный рост.
    fn intern_with_depth(&mut self, names: &[NameId], depth: u32) -> u32 {
        if let Some(&id) = self.by_keys.get(names) {
            return id;
        }
        let id = self.shapes.len() as u32;
        let index = names
            .iter()
            .enumerate()
            .map(|(i, &n)| (n, i as u32))
            .collect();
        let shape = Rc::new(Shape {
            names: names.to_vec(),
            index,
            depth,
            transitions: RefCell::new(HashMap::new()),
            remove_transitions: RefCell::new(HashMap::new()),
        });
        self.shapes.push(shape);
        self.by_keys.insert(names.to_vec(), id);
        id
    }

    /// Сколько форм сейчас в таблице — для тестов на то, что она
    /// перестаёт расти после деградации.
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Пустая форма (0 полей) — нужна `Структура.Очистить()`.
    pub fn empty(&mut self) -> Rc<Shape> {
        let id = self.intern(&[]);
        self.shapes[id as usize].clone()
    }

    /// Форма `current` + поле `field` (уже гарантированно отсутствующее в
    /// `current` — вызывающий проверяет это по `current.index` до вызова,
    /// т.к. "поле уже есть" в 1С означает перезапись значения, а не смену
    /// формы). Порядок остальных полей сохраняется — новое всегда
    /// добавляется последним, как в `Вставить` по месту в `слоты`.
    ///
    /// `None` — «порог `MAX_SHAPE_TRANSITIONS` исчерпан, деградируй»:
    /// вызывающий (`StructureStorage::insert`) обязан переключить объект на
    /// словарное хранение. Новая форма при этом НЕ интернируется — иначе
    /// смысл порога теряется, таблица продолжила бы расти.
    pub fn add_field(&mut self, current: &Rc<Shape>, field: NameId) -> Option<Rc<Shape>> {
        if let Some(next) = current.transitions.borrow().get(&field) {
            return Some(next.clone());
        }
        let mut names = current.names.clone();
        names.push(field);
        // Форма с таким набором полей УЖЕ есть — переход ничего не заводит,
        // таблица не растёт, значит и порог тут ни при чём. Именно это
        // держит инвариант «одинаковый набор ключей — один `Rc<Shape>`»:
        // структура, идущая протоптанным путём (тот же набор `Вставить`,
        // что и у литерала где-то ещё), не деградирует на ровном месте.
        if let Some(&id) = self.by_keys.get(&names) {
            let next = self.shapes[id as usize].clone();
            current.transitions.borrow_mut().insert(field, next.clone());
            return Some(next);
        }
        if current.depth >= MAX_SHAPE_TRANSITIONS {
            return None;
        }
        let id = self.intern_with_depth(&names, current.depth + 1);
        let next = self.shapes[id as usize].clone();
        current.transitions.borrow_mut().insert(field, next.clone());
        Some(next)
    }

    /// Форма `current` без поля `field` (вызывающий проверяет, что оно
    /// вообще есть — `Удалить` несуществующего поля no-op, смены формы не
    /// происходит). Порядок остальных полей сохраняется относительно друг
    /// друга, что важно: `StructureData::slots` теряет ровно один элемент
    /// (`Vec::remove` на позиции удаляемого поля) и должен совпасть по
    /// раскладке с результатом здесь без пересборки остальных значений.
    pub fn remove_field(&mut self, current: &Rc<Shape>, field: NameId) -> Rc<Shape> {
        if let Some(next) = current.remove_transitions.borrow().get(&field) {
            return next.clone();
        }
        let names: Vec<NameId> = current
            .names
            .iter()
            .copied()
            .filter(|&n| n != field)
            .collect();
        // Глубина НЕ растёт: `Удалить` не удлиняет цепочку переходов, а
        // укорачивает набор полей, и последующий `Вставить` должен
        // продолжить счёт с того же места, а не получить полный запас
        // заново. Порога у `remove_field` нет — число форм, достижимых
        // удалениями, ограничено числом полей самой структуры, тогда как
        // `add_field` ограничен только числом различных имён.
        let id = self.intern_with_depth(&names, current.depth);
        let next = self.shapes[id as usize].clone();
        current
            .remove_transitions
            .borrow_mut()
            .insert(field, next.clone());
        next
    }

    /// Готовая таблица форм в порядке интернирования, для встраивания в
    /// `Program`; на компиляции — исходное предзаполнение рантайм-таблицы
    /// (`from_existing`), в рантайме сама больше не читается напрямую.
    pub fn into_shapes(self) -> Vec<Rc<Shape>> {
        self.shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(n: u32) -> NameId {
        // NameId — приватное поле снаружи крейта, но здесь мы внутри
        // bsl-rt, где интерфейс единственный — интернер. Для теста формы
        // достаточно порядковых идентификаторов напрямую.
        let mut interner = crate::NameInterner::new();
        for i in 0..=n {
            interner.intern(&format!("f{i}"));
        }
        interner.intern(&format!("f{n}"))
    }

    #[test]
    fn add_field_reaches_same_shape_as_direct_intern_with_same_key_set() {
        let mut t = ShapeTable::new();
        let a = nid(0);
        let b = nid(1);
        let base_id = t.intern(&[a]);
        let base = t.shapes[base_id as usize].clone();

        let via_transition = t
            .add_field(&base, b)
            .expect("порог не исчерпан одним переходом");
        let direct_id = t.intern(&[a, b]);
        let direct = t.shapes[direct_id as usize].clone();

        assert!(Rc::ptr_eq(&via_transition, &direct));
    }

    #[test]
    fn remove_then_add_back_returns_to_the_original_shape() {
        let mut t = ShapeTable::new();
        let a = nid(0);
        let b = nid(1);
        let id = t.intern(&[a, b]);
        let start = t.shapes[id as usize].clone();

        let without_b = t.remove_field(&start, b);
        assert_eq!(without_b.names, vec![a]);

        let back = t
            .add_field(&without_b, b)
            .expect("возврат к уже интернированной форме");
        assert!(Rc::ptr_eq(&back, &start));
    }

    #[test]
    fn from_existing_seeds_by_keys_so_transitions_still_converge() {
        let mut compile_time = ShapeTable::new();
        let a = nid(0);
        let b = nid(1);
        let id = compile_time.intern(&[a]);
        let seed_shape = compile_time.shapes[id as usize].clone();
        let shapes = compile_time.into_shapes();

        let mut runtime = ShapeTable::from_existing(shapes);
        // Тот же переход, что уже проверен в первом тесте, но теперь через
        // ЗАТРАВЛЕННУЮ (не свежую) таблицу — базовая форма пришла снаружи.
        let extended = runtime
            .add_field(&seed_shape, b)
            .expect("порог не исчерпан");
        let direct_id = runtime.intern(&[a, b]);
        assert!(Rc::ptr_eq(&extended, &runtime.shapes[direct_id as usize]));
    }

    #[test]
    fn add_field_refuses_to_intern_past_the_transition_threshold() {
        let mut t = ShapeTable::new();
        let mut interner = crate::NameInterner::new();
        let empty_id = t.intern(&[]);
        let mut shape = t.shapes[empty_id as usize].clone();

        // Ровно `MAX_SHAPE_TRANSITIONS` переходов проходят — глубина
        // считается ОТ формы, заведённой не переходом (здесь пустой).
        for i in 0..MAX_SHAPE_TRANSITIONS {
            let f = interner.intern(&format!("f{i}"));
            shape = t.add_field(&shape, f).expect("порог ещё не исчерпан");
            assert_eq!(shape.depth, i + 1);
        }
        let overflow = interner.intern("перелив");
        assert!(t.add_field(&shape, overflow).is_none());

        // И отказ не оставил после себя форму в таблице.
        let before = t.len();
        let another = interner.intern("ещё_одно");
        assert!(t.add_field(&shape, another).is_none());
        assert_eq!(t.len(), before);
    }
}
