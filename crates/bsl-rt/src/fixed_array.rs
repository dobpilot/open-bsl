//! `ФиксированныйМассив` — неизменяемый массив по выписке
//! синтакс-помощника 8.3.27: конструктор на основании обычного массива,
//! методы `ВГраница`, `Количество`, `Найти`, `Получить`; заполняется при
//! создании и не меняется. Возвращается глобальной
//! `ПолучитьСообщенияПользователю`.

use crate::{
    Arity, BslValue, CallContext, MethodDescriptor, ObjectProtocol, RtError, RtResult,
    TypeDescriptor, receiver_of,
};

pub(crate) static FIXED_ARRAY_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ФиксированныйМассив",
    type_display: "Fixed array",
    type_names: &["FixedArray"],
};

/// Неизменяемый снимок элементов: без `RefCell` — менять нечем.
pub(crate) struct FixedArrayObject {
    items: Vec<BslValue>,
}

impl FixedArrayObject {
    pub(crate) fn new(items: Vec<BslValue>) -> Self {
        Self { items }
    }
}

impl std::fmt::Debug for FixedArrayObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ФиксированныйМассив[{}]", self.items.len())
    }
}

fn index_of(value: &BslValue, op: &'static str) -> RtResult<usize> {
    let BslValue::Number(number) = value else {
        return Err(RtError::TypeError {
            expected: "Число",
            op,
        });
    };
    let index = number.to_i64_exact().ok_or(RtError::BadIndex)?;
    usize::try_from(index).map_err(|_| RtError::BadIndex)
}

impl ObjectProtocol for FixedArrayObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &FIXED_ARRAY_TYPE
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        FIXED_ARRAY_METHODS
    }

    fn collection_len(&self) -> RtResult<usize> {
        Ok(self.items.len())
    }

    fn get_index(&self, index: &BslValue) -> RtResult<BslValue> {
        let i = index_of(index, "ФиксированныйМассив")?;
        self.items.get(i).cloned().ok_or(RtError::IndexOutOfBounds {
            index: i as i64,
            len: self.items.len(),
        })
    }
}

fn fixed_ubound(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let array = receiver_of::<FixedArrayObject>(receiver, "ВГраница")?;
    Ok(BslValue::number_from_i64(array.items.len() as i64 - 1))
}

fn fixed_count(
    receiver: &dyn ObjectProtocol,
    _args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let array = receiver_of::<FixedArrayObject>(receiver, "Количество")?;
    Ok(BslValue::number_from_i64(array.items.len() as i64))
}

fn fixed_find(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let array = receiver_of::<FixedArrayObject>(receiver, "Найти")?;
    let needle = args.first().cloned().unwrap_or(BslValue::Undefined);
    Ok(match array.items.iter().position(|item| *item == needle) {
        Some(index) => BslValue::number_from_i64(index as i64),
        None => BslValue::Undefined,
    })
}

fn fixed_get(
    receiver: &dyn ObjectProtocol,
    args: &[BslValue],
    _ctx: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let array = receiver_of::<FixedArrayObject>(receiver, "Получить")?;
    array.get_index(args.first().unwrap_or(&BslValue::Undefined))
}

static FIXED_ARRAY_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor::new(&["ВГраница", "UBound"], Arity::exact(0), fixed_ubound),
    MethodDescriptor::new(&["Количество", "Count"], Arity::exact(0), fixed_count),
    MethodDescriptor::new(&["Найти", "Find"], Arity::exact(1), fixed_find),
    MethodDescriptor::new(&["Получить", "Get"], Arity::exact(1), fixed_get),
];

/// `Новый ФиксированныйМассив(<Массив>)` — конструктор ядра.
pub(crate) fn construct_fixed_array(
    _ctx: &mut CallContext<'_>,
    args: &[BslValue],
) -> RtResult<BslValue> {
    let source = args.first().ok_or(RtError::TypeError {
        expected: "Массив",
        op: "ФиксированныйМассив",
    })?;
    let BslValue::Object(object) = source else {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "ФиксированныйМассив",
        });
    };
    let crate::BslObject::Array(items) = object.as_ref() else {
        return Err(RtError::TypeError {
            expected: "Массив",
            op: "ФиксированныйМассив",
        });
    };
    let snapshot = items.borrow().clone();
    Ok(BslValue::new_object(FixedArrayObject::new(snapshot)))
}
