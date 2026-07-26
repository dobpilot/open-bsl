//! Рантайм-слой: значения, арифметика/сравнение, коллекции (`Массив`,
//! `Структура`). Даты и полноценные UTF-16 строки (со всеми операциями из
//! брифа — `СтрДлина`, индексация по кодовым единицам, интернирование) сюда
//! ещё не входят: `Str` здесь — минимальный носитель для строкового
//! литерала-конструктора `Новый Структура("x,y,z", ...)`, не более того.
//! `BslValue` растёт по мере готовности остальных слоёв, а не заранее под
//! все типы из брифа.

mod interner;
mod object;
mod shape;

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use bsl_number::{BslNumber, NumError};

pub use interner::{NameId, NameInterner};
pub use object::{BslObject, StructureData};
pub use shape::{Shape, ShapeTable};

#[derive(Debug, Clone)]
pub enum BslValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(BslNumber),
    Str(Rc<str>),
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

    pub fn add(&self, other: &Self) -> RtResult<Self> {
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

    /// `И`/`ИЛИ` в 1С НЕ короткозамкнутые: оба операнда всегда вычислены до
    /// вызова этой функции (это делает вызывающий код в VM), здесь только
    /// комбинирование уже готовых булевых значений. Важно: `a? && b?` в
    /// Rust сам короткозамкнутый (`&&` не оценит правую часть, если левая
    /// уже `false`) — это ровно то, чего мы хотим избежать, поэтому обе
    /// стороны приводятся к `bool` до `&&`/`||`, отдельными выражениями.
    pub fn and(&self, other: &Self) -> RtResult<Self> {
        let a = self.as_bool("И")?;
        let b = other.as_bool("И")?;
        Ok(BslValue::Boolean(a && b))
    }

    pub fn or(&self, other: &Self) -> RtResult<Self> {
        let a = self.as_bool("ИЛИ")?;
        let b = other.as_bool("ИЛИ")?;
        Ok(BslValue::Boolean(a || b))
    }

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

    pub fn compare(&self, other: &Self, op: &'static str) -> RtResult<Ordering> {
        match (self, other) {
            (BslValue::Number(a), BslValue::Number(b)) => Ok(a.cmp(b)),
            _ => Err(RtError::TypeError {
                expected: "Число",
                op,
            }),
        }
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
                BslObject::Structure(_) => Err(RtError::NotIndexable),
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
                BslObject::Structure(_) => Err(RtError::NotIndexable),
            },
            _ => Err(RtError::NotIndexable),
        }
    }

    /// Длина коллекции — используется и `Для Каждого` (компилируется в
    /// индексный цикл поверх этой длины), и будущим `Количество()`.
    pub fn collection_len(&self) -> RtResult<usize> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(v) => Ok(v.borrow().len()),
                BslObject::Structure(s) => Ok(s.borrow().slots.len()),
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
                BslObject::Array(_) => Err(RtError::NotAnObject),
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
                BslObject::Array(_) => Err(RtError::NotAnObject),
            },
            _ => Err(RtError::NotAnObject),
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
    fn logical_ops_do_not_need_short_circuit_evaluation_here() {
        assert_eq!(
            BslValue::Boolean(true).and(&BslValue::Boolean(false)).unwrap(),
            BslValue::Boolean(false)
        );
        assert_eq!(
            BslValue::Boolean(false).or(&BslValue::Boolean(true)).unwrap(),
            BslValue::Boolean(true)
        );
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
}
