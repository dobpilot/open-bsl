//! Рантайм-слой: значения, арифметика/сравнение, коллекции (`Массив`,
//! `Структура`), строки UTF-16 (`Str`/`BslString`). Даты сюда ещё не входят.
//! `BslValue` растёт по мере готовности остальных слоёв, а не заранее под
//! все типы из брифа.

mod builtin;
mod interner;
mod map;
mod object;
mod runtime_shapes;
mod shape;
mod string;
mod table;

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use bsl_number::{BslNumber, NumError};

pub use builtin::{call_builtin_method_ctx, call_builtin_fn, call_builtin_method, BuiltinFn, BuiltinMethod};
pub use interner::{NameId, NameInterner};
pub use map::MapData;
pub use object::{BslObject, StructureData};
pub use runtime_shapes::RuntimeShapes;
pub use shape::{Shape, ShapeTable};
pub use string::BslString;
pub use table::ValueTableData;

#[derive(Debug, Clone)]
pub enum BslValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(BslNumber),
    Str(BslString),
    Object(Rc<BslObject>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RtError {
    Num(NumError),
    /// Операция получила значение не того типа — например, `Если 1 Тогда`:
    /// в BSL условия строго булевы, никакой truthiness.
    TypeError {
        expected: &'static str,
        op: &'static str,
    },
    /// Индексация значения, которое не индексируется (не `Массив`).
    NotIndexable,
    IndexOutOfBounds {
        index: i64,
        len: usize,
    },
    /// Индекс — не целое неотрицательное число.
    BadIndex,
    /// Доступ к полю значения, у которого полей нет (не `Структура`).
    NotAnObject,
    /// Обращение к полю, которого нет в форме структуры.
    UnknownField(NameId),
    /// `ВызватьИсключение <значение>;` — значение, с которым бросили.
    Raised(BslValue),
    /// Обращение к `СтрокаТаблицы`, чья строка уже удалена (`row_id` не
    /// резолвится в `id_to_pos`) — не тихое чтение чужих данных.
    RowInvalidated,
    /// Обращение к несуществующей колонке `ТаблицыЗначений`/`СтрокиТаблицы`.
    UnknownColumn(String),
    /// Метод объекта существует, но не для этого типа получателя, либо
    /// вызван не с тем числом аргументов для этого типа (некоторые методы,
    /// например `Добавить`, полиморфны: означают разное в зависимости от
    /// типа получателя, и арность из-за этого проверяется в рантайме, не
    /// на этапе резолвинга).
    MethodNotApplicable {
        method: &'static str,
        receiver: &'static str,
    },
    /// Ошибка лексера/парсера/резолвинга/компиляции строки, переданной в
    /// `Выполнить`/`Вычислить` — текст уже отформатирован тем слоем, что
    /// её обнаружил (`bsl-syntax`/`bsl-sema`/`bsl-bytecode`), сюда попадает
    /// как есть: `bsl-rt` не знает про эти крейты (обратная зависимость).
    DynamicError(String),
    /// `Выполнить`/`Вычислить` вызваны не из кода верхнего уровня — внутри
    /// процедуры/функции они пока не поддержаны (нужен материализованный
    /// кадр с именной таблицей, см. бриф; это отдельная, ещё не сделанная
    /// работа).
    DynamicNotAtTopLevel,
}

impl From<NumError> for RtError {
    fn from(e: NumError) -> Self {
        RtError::Num(e)
    }
}

impl fmt::Display for RtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RtError::Num(e) => write!(f, "{e}"),
            RtError::TypeError { expected, op } => {
                write!(f, "ожидался тип «{expected}» для операции «{op}»")
            }
            RtError::NotIndexable => write!(f, "значение не поддерживает индексацию"),
            RtError::IndexOutOfBounds { index, len } => {
                write!(f, "индекс {index} вне границ (длина {len})")
            }
            RtError::BadIndex => write!(f, "индекс должен быть целым неотрицательным числом"),
            RtError::NotAnObject => write!(f, "значение не поддерживает доступ к полям"),
            RtError::UnknownField(_) => write!(f, "поле не найдено в структуре"),
            RtError::Raised(v) => write!(f, "{v}"),
            RtError::RowInvalidated => write!(f, "строка таблицы значений больше не существует"),
            RtError::UnknownColumn(name) => write!(f, "колонка «{name}» не найдена"),
            RtError::MethodNotApplicable { method, receiver } => {
                write!(f, "метод «{method}» не применим к «{receiver}»")
            }
            RtError::DynamicError(msg) => write!(f, "{msg}"),
            RtError::DynamicNotAtTopLevel => write!(
                f,
                "Выполнить/Вычислить внутри процедуры или функции пока не поддержаны"
            ),
        }
    }
}

impl std::error::Error for RtError {}

pub type RtResult<T> = Result<T, RtError>;

impl BslValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            BslValue::Undefined => "Неопределено",
            BslValue::Null => "Null",
            BslValue::Boolean(_) => "Булево",
            BslValue::Number(_) => "Число",
            BslValue::Str(_) => "Строка",
            BslValue::Object(o) => match &**o {
                BslObject::Array(_) => "Массив",
                BslObject::Structure(_) => "Структура",
                BslObject::ValueTable(_) => "ТаблицаЗначений",
                BslObject::TableColumns(_) => "КоллекцияКолонокТаблицыЗначений",
                BslObject::TableRow(_, _) => "СтрокаТаблицыЗначений",
                BslObject::Map(_) => "Соответствие",
                BslObject::KeyValuePair(_, _) => "КлючИЗначение",
            },
        }
    }

    fn as_number(&self, op: &'static str) -> RtResult<&BslNumber> {
        match self {
            BslValue::Number(n) => Ok(n),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op,
            }),
        }
    }

    fn as_bool(&self, op: &'static str) -> RtResult<bool> {
        match self {
            BslValue::Boolean(b) => Ok(*b),
            _ => Err(RtError::TypeError {
                expected: "Булево",
                op,
            }),
        }
    }

    /// `+` между двумя строками — конкатенация (реальная 1С считает это
    /// перегрузкой того же оператора, не отдельной функцией). Любая другая
    /// комбинация типов идёт по числовому пути и получает его же ошибку
    /// типа, если не подходит.
    pub fn add(&self, other: &Self) -> RtResult<Self> {
        if let (BslValue::Str(a), BslValue::Str(b)) = (self, other) {
            return Ok(BslValue::Str(a.concat(b)));
        }
        Ok(BslValue::Number(
            self.as_number("+")?.add(other.as_number("+")?)?,
        ))
    }

    pub fn sub(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("-")?.sub(other.as_number("-")?)?,
        ))
    }

    pub fn mul(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("*")?.mul(other.as_number("*")?)?,
        ))
    }

    pub fn div(&self, other: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(
            self.as_number("/")?.div(other.as_number("/")?)?,
        ))
    }

    pub fn neg(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("унарный -")?.neg()))
    }

    pub fn not(&self) -> RtResult<Self> {
        Ok(BslValue::Boolean(!self.as_bool("Не")?))
    }

    // --- Трансцендентные функции (через f64 в bsl-number) ------------------

    pub fn sqrt(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sqrt")?.sqrt()?))
    }

    pub fn pow(&self, exp: &Self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Pow")?.pow(exp.as_number("Pow")?)?))
    }

    pub fn ln(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log")?.ln()?))
    }

    pub fn log10(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Log10")?.log10()?))
    }

    pub fn exp(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Exp")?.exp()?))
    }

    pub fn sin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Sin")?.sin()?))
    }

    pub fn cos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Cos")?.cos()?))
    }

    pub fn tan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("Tan")?.tan()?))
    }

    pub fn asin(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ASin")?.asin()?))
    }

    pub fn acos(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ACos")?.acos()?))
    }

    pub fn atan(&self) -> RtResult<Self> {
        Ok(BslValue::Number(self.as_number("ATan")?.atan()?))
    }

    /// `И`/`ИЛИ` в 1С короткозамкнутые, поэтому у `BslValue` больше нет
    /// `and`/`or`: комбинирование живёт в потоке управления кодогена
    /// (`Instr::JumpIfFalse`/`JumpIfTrue` в `bsl-bytecode::compiler`), а не
    /// здесь, — правый операнд физически не вычисляется, если левый уже
    /// решил результат.
    pub fn as_condition(&self) -> RtResult<bool> {
        self.as_bool("Условие")
    }

    /// Сравнение по значению для `=`/`<>`. Разнотипные значения просто не
    /// равны — это не ошибка (в отличие от `<`/`>`/... между разными
    /// типами). `Массив`/`Структура` — ссылочные типы: равенство — это
    /// тождество объекта (`Rc::ptr_eq`), а не структурное сравнение
    /// содержимого (см. `PartialEq` ниже, эта функция ему делегирует).
    pub fn eq_value(&self, other: &Self) -> bool {
        self == other
    }

    /// Сравнение строк — упорядочивание код-юнитов UTF-16, без учёта
    /// локали (настоящая коллация для `Сортировать` в `ТаблицаЗначений` —
    /// отдельная, ещё не сделанная задача).
    pub fn compare(&self, other: &Self, op: &'static str) -> RtResult<Ordering> {
        match (self, other) {
            (BslValue::Number(a), BslValue::Number(b)) => Ok(a.cmp(b)),
            (BslValue::Str(a), BslValue::Str(b)) => Ok(a.cmp(b)),
            _ => Err(RtError::TypeError {
                expected: "Число или Строка",
                op,
            }),
        }
    }

    fn as_str(&self, op: &'static str) -> RtResult<&BslString> {
        match self {
            BslValue::Str(s) => Ok(s),
            _ => Err(RtError::TypeError {
                expected: "Строка",
                op,
            }),
        }
    }

    fn as_usize(&self, op: &'static str) -> RtResult<usize> {
        let n = self.as_number(op)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    // --- Строки ---------------------------------------------------------

    pub fn str_len(&self) -> RtResult<usize> {
        Ok(self.as_str("СтрДлина")?.len_utf16())
    }

    pub fn str_left(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("Лев")?.left(len.as_usize("Лев")?)))
    }

    pub fn str_right(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("Прав")?.right(len.as_usize("Прав")?)))
    }

    pub fn str_mid(&self, start: &Self, len: &Self) -> RtResult<Self> {
        let s = self.as_str("Сред")?;
        let start = start.as_usize("Сред")?;
        let len = len.as_usize("Сред")?;
        Ok(BslValue::Str(s.substring(start, len)))
    }

    pub fn str_upper(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("ВРег")?.to_uppercase()))
    }

    pub fn str_lower(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("НРег")?.to_lowercase()))
    }

    pub fn str_trim_all(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛП")?.trim()))
    }

    // --- Коллекции ----------------------------------------------------

    pub fn new_array(items: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Array(std::cell::RefCell::new(items))))
    }

    pub fn new_structure(shape: Rc<Shape>, slots: Vec<BslValue>) -> Self {
        BslValue::Object(Rc::new(BslObject::Structure(std::cell::RefCell::new(
            StructureData { shape, slots },
        ))))
    }

    pub fn new_table() -> Self {
        BslValue::Object(Rc::new(BslObject::ValueTable(ValueTableData::new())))
    }

    pub fn new_map() -> Self {
        BslValue::Object(Rc::new(BslObject::Map(std::cell::RefCell::new(MapData::new()))))
    }

    /// Индекс должен быть целым неотрицательным числом — `1С` использует
    /// `Число` для индексов, отдельного целочисленного типа нет.
    fn index_as_usize(idx: &BslValue) -> RtResult<usize> {
        let n = idx.as_number("[]").map_err(|_| RtError::BadIndex)?;
        let i = n.to_i64_exact().ok_or(RtError::BadIndex)?;
        usize::try_from(i).map_err(|_| RtError::BadIndex)
    }

    pub fn get_index(&self, idx: &BslValue) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let v = v.borrow();
                    let i = Self::index_as_usize(idx)?;
                    v.get(i)
                        .cloned()
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: v.len() })
                }
                BslObject::ValueTable(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let row_id = {
                        let d = data.borrow();
                        d.row_id_at(i)
                            .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: d.row_count() })?
                    };
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))))
                }
                // ПОЗИЦИОННЫЙ, не по ключу: `Для Каждого` компилируется в
                // общий для всех коллекций протокол `CollectionLen` + рост
                // числового индекса `0..len` через эту же функцию (см.
                // `bsl-bytecode::compiler::RStmt::ForEach`) — компилятор не
                // знает на этапе компиляции, что `idx` в рантайме окажется
                // `Соответствие`, и не может эмитить для него другой путь.
                // Доступ ПО КЛЮЧУ у `Соответствие` поэтому сознательно НЕ
                // здесь, а в `.Получить(Ключ)` (`map_get`) — если бы `[]`
                // тоже читал по ключу, `м[0]` было бы неразрешимо
                // неоднозначно между "0-я по счёту пара" и "значение по
                // ключу 0" для карты с целочисленными ключами.
                BslObject::Map(data) => {
                    let i = Self::index_as_usize(idx)?;
                    let data = data.borrow();
                    let (k, v) = data
                        .entry_at(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len: data.len() })?;
                    Ok(BslValue::Object(Rc::new(BslObject::KeyValuePair(k, v))))
                }
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn set_index(&self, idx: &BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    let slot = v
                        .get_mut(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len })?;
                    *slot = val;
                    Ok(())
                }
                _ => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    /// Длина коллекции — используется и `Для Каждого` (компилируется в
    /// индексный цикл поверх этой длины), и `Количество()`.
    pub fn collection_len(&self) -> RtResult<usize> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => Ok(v.borrow().len()),
                BslObject::Structure(s) => Ok(s.borrow().slots.len()),
                BslObject::ValueTable(data) => Ok(data.borrow().row_count()),
                BslObject::TableColumns(data) => Ok(data.borrow().column_names.len()),
                BslObject::TableRow(..) => Err(RtError::NotIndexable),
                BslObject::Map(data) => Ok(data.borrow().len()),
                BslObject::KeyValuePair(..) => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    pub fn get_field(&self, name: NameId) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let s = s.borrow();
                    match s.shape.index.get(&name) {
                        Some(&slot) => Ok(s.slots[slot as usize].clone()),
                        None => Err(RtError::UnknownField(name)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field(&self, name: NameId, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let mut s = s.borrow_mut();
                    match s.shape.index.get(&name).copied() {
                        Some(slot) => {
                            s.slots[slot as usize] = val;
                            Ok(())
                        }
                        None => Err(RtError::UnknownField(name)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Рантайм-мутация формы структуры ---------------------------------
    //
    // `Вставить`/`Удалить`/`Свойство` (двухаргументная форма) — в отличие
    // от `get_field`/`set_field` выше, которые лишь ЧИТАЮТ уже готовую
    // форму, эти три меняют её: `ShapeTable` здесь больше не голая таблица
    // компиляции, а рантайм-контекст (`RuntimeShapes`, см. одноимённый
    // модуль), поэтому и подписи ниже берут `&mut ShapeTable`, а не
    // работают в изоляции. Инлайн-кэш `GetProp`/`SetProp` (`Rc::ptr_eq` на
    // `s.shape`) сам заметит смену формы после любой из них — ничего
    // специально инвалидировать не нужно.

    /// `Структура.Вставить(Ключ, Значение)`. Поле уже есть — просто новое
    /// значение на том же слоте, форма не меняется (у 1С `Вставить`
    /// повторного поля — это не ошибка и не дубликат, а перезапись).
    pub fn structure_insert(&self, field: NameId, val: BslValue, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let mut s = s.borrow_mut();
                    match s.shape.index.get(&field).copied() {
                        Some(slot) => s.slots[slot as usize] = val,
                        None => {
                            s.slots.push(val);
                            s.shape = shapes.add_field(&s.shape, field);
                        }
                    }
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Удалить(Ключ)`. Поля нет — no-op (симметрично
    /// `MapData::remove`, см. его doc comment): убрать то, чего и так нет,
    /// не повод падать.
    pub fn structure_delete(&self, field: NameId, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let mut s = s.borrow_mut();
                    if let Some(&slot) = s.shape.index.get(&field) {
                        s.slots.remove(slot as usize);
                        s.shape = shapes.remove_field(&s.shape, field);
                    }
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Свойство(Ключ)` / `Структура.Свойство(Ключ,
    /// ЗначениеПоУмолчанию)`.
    ///
    /// ОТКЛОНЕНИЕ ОТ РЕАЛЬНОЙ 1С: настоящая сигнатура — `Свойство(Ключ,
    /// Значение)`, где `Значение` — параметр ПО ССЫЛКЕ (выходной), а
    /// возвращает функция `Булево` (найдено ли поле). Такой контракт для
    /// ВСТРОЕННОГО метода потребовал бы протаскивать режим аргумента
    /// (`ArgMode::ByRefLocal`) через `CallMethod` — сейчас это умеет только
    /// `Call` для пользовательских процедур/функций (см.
    /// `bsl-bytecode::instr::Instr::Call`), `CallMethod` вычисляет все
    /// аргументы как значения. Ради одного метода вводить второй протокол
    /// аргументов не стали — вместо этого второй аргумент трактуется как
    /// значение по умолчанию (безопасный geттер: нет поля — нет и ошибки).
    pub fn structure_property(&self, field: NameId, default: Option<BslValue>) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let s = s.borrow();
                    match s.shape.index.get(&field) {
                        Some(&slot) => Ok(s.slots[slot as usize].clone()),
                        None => Ok(default.unwrap_or(BslValue::Undefined)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Структура.Очистить()` — сбрасывает набор полей целиком (форма
    /// становится пустой), не только значения на месте: у 1С `Очистить()`
    /// на структуре убирает и сами поля, следующий `Свойство`/`.Х` их уже
    /// не найдёт.
    pub fn structure_clear(&self, shapes: &mut ShapeTable) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let mut s = s.borrow_mut();
                    s.slots.clear();
                    s.shape = shapes.empty();
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Вставить(Ключ, Значение)`.
    pub fn map_insert(&self, key: BslValue, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => {
                    data.borrow_mut().insert(key, val);
                    Ok(())
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// `Соответствие.Получить(Ключ)` — `Неопределено`, если ключа нет, не
    /// ошибка (соответствует `MapData::get`/реальной 1С).
    pub fn map_get(&self, key: &BslValue) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Map(data) => Ok(data.borrow().get(key).unwrap_or(BslValue::Undefined)),
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `GetProp` (см. брифовский план оптимизаций: "слот
    /// хранит (shape_ptr, slot_idx)"). `cache` — одна ячейка НА КОНКРЕТНУЮ
    /// инструкцию в чанке (см. `Chunk::prop_cache` в `bsl-bytecode`),
    /// живёт между исполнениями этой инструкции. Промах — обычный поиск
    /// по `Shape::index` плюс запись в кэш; форма меняется редко (обычно
    /// вообще никогда для данной инструкции — иначе откуда там структура
    /// другой формы), так что кэш почти всегда мономорфный.
    ///
    /// Держим `Rc<Shape>` целиком, а не голый указатель: так кэш не может
    /// протухнуть на чужой адрес, если форма где-то освободится — он сам
    /// продлевает ей жизнь, пока висит в кэше.
    pub fn get_field_cached(
        &self,
        name: NameId,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let s = s.borrow();
                    if let Some((cached_shape, slot)) = cache.borrow().as_ref() {
                        if Rc::ptr_eq(cached_shape, &s.shape) {
                            return Ok(s.slots[*slot as usize].clone());
                        }
                    }
                    match s.shape.index.get(&name) {
                        Some(&slot) => {
                            *cache.borrow_mut() = Some((s.shape.clone(), slot));
                            Ok(s.slots[slot as usize].clone())
                        }
                        None => Err(RtError::UnknownField(name)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Инлайн-кэш для `SetProp` — см. `get_field_cached`.
    pub fn set_field_cached(
        &self,
        name: NameId,
        val: BslValue,
        cache: &std::cell::RefCell<Option<(Rc<Shape>, u32)>>,
    ) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Structure(s) => {
                    let mut s = s.borrow_mut();
                    if let Some((cached_shape, slot)) = cache.borrow().as_ref() {
                        if Rc::ptr_eq(cached_shape, &s.shape) {
                            s.slots[*slot as usize] = val;
                            return Ok(());
                        }
                    }
                    match s.shape.index.get(&name).copied() {
                        Some(slot) => {
                            *cache.borrow_mut() = Some((s.shape.clone(), slot));
                            s.slots[slot as usize] = val;
                            Ok(())
                        }
                        None => Err(RtError::UnknownField(name)),
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    /// Резолвинг поля/псевдо-свойства по ИМЕНИ (не `NameId`) — нужен для
    /// объектов, чьи "поля" известны только в рантайме: колонки
    /// `СтрокиТаблицыЗначений` заводятся через `.Колонки.Добавить(имя)`, а
    /// не как статичная форма структуры, поэтому по ним нельзя
    /// интернировать `NameId` на этапе компиляции. `Структура` в эту
    /// функцию не заходит — у неё есть более быстрый путь через
    /// `get_field`/`NameId`, здесь она просто не находится.
    pub fn get_field_by_name(&self, name: &str) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    if name.eq_ignore_ascii_case("Колонки") || name.eq_ignore_ascii_case("Columns")
                    {
                        Ok(BslValue::Object(Rc::new(BslObject::TableColumns(data.clone()))))
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                BslObject::TableRow(data, row_id) => {
                    let data = data.borrow();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.get_cell(*row_id, col).ok_or(RtError::RowInvalidated)
                }
                // `КлючИЗначение.Ключ`/`.Значение` — те же два имени всегда,
                // ни разу не интернированные как `Shape`/`NameId`, потому
                // что этот объект заводится ЦЕЛИКОМ в рантайме
                // (`get_index` на `Соответствие`, см. `map.rs`), а не
                // компилируется из литерала полей.
                BslObject::KeyValuePair(k, v) => {
                    if name.eq_ignore_ascii_case("Ключ") || name.eq_ignore_ascii_case("Key") {
                        Ok(k.clone())
                    } else if name.eq_ignore_ascii_case("Значение") || name.eq_ignore_ascii_case("Value")
                    {
                        Ok(v.clone())
                    } else {
                        Err(RtError::UnknownColumn(name.to_string()))
                    }
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    pub fn set_field_by_name(&self, name: &str, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableRow(data, row_id) => {
                    let mut data = data.borrow_mut();
                    let col = data
                        .column_index(name)
                        .ok_or_else(|| RtError::UnknownColumn(name.to_string()))?;
                    data.set_cell(*row_id, col, val).ok_or(RtError::RowInvalidated)
                }
                _ => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
        }
    }

    // --- Методы, полиморфные по типу получателя --------------------------
    //
    // `Добавить`/`Удалить`/`Очистить` в реальной 1С означают разное в
    // зависимости от типа получателя (элемент массива, строка таблицы,
    // колонка, ...) — то же имя метода, разное поведение и разная арность.
    // Резолвинг имени в `bsl-sema` не может знать заранее, каким объектом
    // оказится `obj` в рантайме (BSL — динамически типизированный), поэтому
    // диспетчеризация и проверка арности — здесь, в рантайме, а не на этапе
    // компиляции.

    /// `Массив.Добавить(значение)`.
    pub fn push_element(&self, val: BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().push(val);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `ТаблицаЗначений.Добавить()` -> новая строка.
    pub fn table_add_row(&self) -> RtResult<BslValue> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::ValueTable(data) => {
                    let row_id = data.borrow_mut().add_row();
                    Ok(BslValue::Object(Rc::new(BslObject::TableRow(data.clone(), row_id))))
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Таблица.Колонки.Добавить(имя)`.
    pub fn table_add_column(&self, name: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::TableColumns(data) => {
                    let name = name.as_str("Колонки.Добавить")?.to_string();
                    data.borrow_mut().add_column(&name);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Добавить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Добавить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Удалить(индекс)` / `ТаблицаЗначений.Удалить(индекс)`.
    pub fn delete_element(&self, idx: &BslValue) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    let mut v = v.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = v.len();
                    if i >= len {
                        return Err(RtError::IndexOutOfBounds { index: i as i64, len });
                    }
                    v.remove(i);
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    let mut d = data.borrow_mut();
                    let i = Self::index_as_usize(idx)?;
                    let len = d.row_count();
                    d.delete_row_at(i)
                        .ok_or(RtError::IndexOutOfBounds { index: i as i64, len })
                }
                // `Соответствие.Удалить(Ключ)` — по значению ключа, не по
                // позиции (в отличие от Array/ValueTable выше): в этом
                // случае `idx` в имени параметра функции вводит в
                // заблуждение, но сигнатура (`&BslValue`) уже общая для
                // всех получателей, менять её ради одного случая не стоит.
                BslObject::Map(data) => {
                    data.borrow_mut().remove(idx);
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Удалить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Удалить",
                receiver: self.type_name(),
            }),
        }
    }

    /// `Массив.Очистить()` / `ТаблицаЗначений.Очистить()` /
    /// `Соответствие.Очистить()`.
    pub fn clear_collection(&self) -> RtResult<()> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => {
                    v.borrow_mut().clear();
                    Ok(())
                }
                BslObject::Map(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                BslObject::ValueTable(data) => {
                    data.borrow_mut().clear();
                    Ok(())
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Очистить",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Очистить",
                receiver: self.type_name(),
            }),
        }
    }
}

/// Ручная реализация вместо `derive`: `Массив`/`Структура` — ссылочные
/// типы, `=` для них — тождество объекта (`Rc::ptr_eq`), а не структурное
/// сравнение содержимого (в отличие от `Число`/`Строка`/`Булево`).
impl PartialEq for BslValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BslValue::Undefined, BslValue::Undefined) => true,
            (BslValue::Null, BslValue::Null) => true,
            (BslValue::Boolean(a), BslValue::Boolean(b)) => a == b,
            (BslValue::Number(a), BslValue::Number(b)) => a == b,
            (BslValue::Str(a), BslValue::Str(b)) => a == b,
            (BslValue::Object(a), BslValue::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// `Eq` — все варианты `PartialEq::eq` выше рефлексивны (десятичные числа
/// без NaN-подобных значений, указатели сравниваются с самими собой),
/// значит `Eq` держится честно, не только формально для `HashMap`.
impl Eq for BslValue {}

/// Нужен ключу `Соответствие` (`HashMap<BslValue, BslValue>` в `map.rs`).
/// Согласован с `PartialEq` выше пункт-в-пункт: `Число` хэширует через
/// `BslNumber` (см. его `impl Hash` — нормализация уже сделала хэш
/// независимым от масштаба представления, `1.0` и `1.00` дают одно и то же
/// (`m`, `scale`)), `Строка` — по содержимому (`BslString` производит
/// `Hash` от `Rc<[u16]>`, тоже по значению, не по адресу), `Массив`/
/// `Структура`/... — по адресу `Rc`, ровно как их `==` через `Rc::ptr_eq`.
impl Hash for BslValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            BslValue::Undefined | BslValue::Null => {}
            BslValue::Boolean(b) => b.hash(state),
            BslValue::Number(n) => n.hash(state),
            BslValue::Str(s) => s.hash(state),
            BslValue::Object(o) => Rc::as_ptr(o).hash(state),
        }
    }
}

impl fmt::Display for BslValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BslValue::Undefined => write!(f, ""),
            BslValue::Null => write!(f, "Null"),
            BslValue::Boolean(true) => write!(f, "Да"),
            BslValue::Boolean(false) => write!(f, "Нет"),
            BslValue::Number(n) => write!(f, "{n}"),
            BslValue::Str(s) => write!(f, "{s}"),
            BslValue::Object(o) => match &**o {
                BslObject::Array(_) => write!(f, "Массив"),
                BslObject::Structure(_) => write!(f, "Структура"),
                BslObject::ValueTable(_) => write!(f, "ТаблицаЗначений"),
                BslObject::TableColumns(_) => write!(f, "КоллекцияКолонокТаблицыЗначений"),
                BslObject::TableRow(_, _) => write!(f, "СтрокаТаблицыЗначений"),
                BslObject::Map(_) => write!(f, "Соответствие"),
                BslObject::KeyValuePair(_, _) => write!(f, "КлючИЗначение"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(s: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(s).unwrap())
    }

    #[test]
    fn strict_boolean_condition_rejects_numbers() {
        // Если 1 Тогда — ошибка, не приведение.
        let err = num("1").as_condition().unwrap_err();
        assert!(matches!(err, RtError::TypeError { expected: "Булево", .. }));
    }

    #[test]
    fn equality_by_value_across_representations() {
        assert!(num("1.0").eq_value(&num("1.00")));
    }

    #[test]
    fn display_matches_measured_platform_strings() {
        assert_eq!(BslValue::Boolean(true).to_string(), "Да");
        assert_eq!(BslValue::Boolean(false).to_string(), "Нет");
        assert_eq!(BslValue::Undefined.to_string(), "");
    }

    #[test]
    fn array_index_get_set_roundtrip() {
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        assert_eq!(arr.get_index(&num("1")).unwrap(), num("2"));
        arr.set_index(&num("1"), num("99")).unwrap();
        assert_eq!(arr.get_index(&num("1")).unwrap(), num("99"));
        assert_eq!(arr.collection_len().unwrap(), 3);
    }

    #[test]
    fn array_out_of_bounds_is_an_error() {
        let arr = BslValue::new_array(vec![num("1")]);
        assert!(matches!(
            arr.get_index(&num("5")).unwrap_err(),
            RtError::IndexOutOfBounds { .. }
        ));
    }

    #[test]
    fn arrays_and_structures_are_reference_types() {
        // b = a делает b тем же объектом, что и a: мутация через одну
        // переменную видна через другую (Rc, не глубокое копирование).
        let a = BslValue::new_array(vec![num("1")]);
        let b = a.clone();
        b.set_index(&num("0"), num("42")).unwrap();
        assert_eq!(a.get_index(&num("0")).unwrap(), num("42"));
        assert!(a.eq_value(&b));

        let c = BslValue::new_array(vec![num("42")]);
        assert!(!a.eq_value(&c), "структурно равные, но разные объекты — не равны");
    }

    #[test]
    fn structure_field_get_set_by_interned_name() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let y = names.intern("y");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x, y]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1"), num("2")]);
        assert_eq!(s.get_field(x).unwrap(), num("1"));
        s.set_field(y, num("99")).unwrap();
        assert_eq!(s.get_field(y).unwrap(), num("99"));
    }

    #[test]
    fn unknown_field_is_an_error() {
        let mut names = NameInterner::new();
        let x = names.intern("x");
        let z = names.intern("z");
        let mut shapes = ShapeTable::new();
        let shape_id = shapes.intern(&[x]);
        let shapes = shapes.into_shapes();
        let shape = shapes[shape_id as usize].clone();

        let s = BslValue::new_structure(shape, vec![num("1")]);
        assert!(matches!(s.get_field(z).unwrap_err(), RtError::UnknownField(_)));
    }

    #[test]
    fn display_matches_measured_platform_strings_for_collections() {
        // Строка(Новый Массив) -> "Массив" (измерено на платформе).
        assert_eq!(BslValue::new_array(vec![]).to_string(), "Массив");
    }

    #[test]
    fn builtin_math_functions_lookup_and_call() {
        assert_eq!(BuiltinFn::lookup("sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("Sqrt"), Some(BuiltinFn::Sqrt));
        assert_eq!(BuiltinFn::lookup("СООБЩИТЬ"), Some(BuiltinFn::Message));
        assert_eq!(BuiltinFn::lookup("НетТакойФункции"), None);
        assert_eq!(BuiltinFn::Pow.arity(), 2);
        assert_eq!(BuiltinFn::Sqrt.arity(), 1);

        let v = call_builtin_fn(BuiltinFn::Sqrt, &[num("2")]).unwrap();
        assert_eq!(v, num("1.4142135623731"));
    }

    #[test]
    fn builtin_method_count_on_array() {
        assert_eq!(BuiltinMethod::lookup("count"), Some(BuiltinMethod::Count));
        let arr = BslValue::new_array(vec![num("1"), num("2"), num("3")]);
        let v = call_builtin_method(BuiltinMethod::Count, &arr, &[]).unwrap();
        assert_eq!(v, num("3"));
    }
}
