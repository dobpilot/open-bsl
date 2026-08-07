//! `БуферДвоичныхДанных` — ИЗМЕНЯЕМЫЙ байтовый буфер фиксированного размера.
//!
//! Противоположность `ДвоичныеДанные` во всём, что важно: те неизменяемы и
//! сравниваются по содержимому, буфер же меняется на месте и сравнивается
//! ПО ТОЖДЕСТВУ — два свежих буфера одного размера, оба нулевые, платформа
//! равными не считает (измерено). Отсюда `Rc<RefCell<..>>` в
//! [`crate::BslObject::BinaryBuffer`]: равенство и хэш достаются даром от
//! общей ветки `Rc::ptr_eq`, а `get_index`/`set_index` берут `&self`.
//!
//! Размер задаётся конструктором и НЕ меняется: роста нет, `Размер` —
//! свойство только для чтения, а `Соединить` отдаёт новый буфер, а не
//! удлиняет получателя (всё измерено на 8.3.27).
//!
//! # Что измерено про край диапазона, а что нет
//!
//! Позиция и индекс — число, дробная часть ОТБРАСЫВАЕТСЯ (`Б[1.9]` читает
//! байт 1, `Установить(0.5, 7)` пишет в байт 0), выход за `0..Размер-1` —
//! перехватываемая `Попыткой` ошибка.
//!
//! А вот записать в байт значение вне `0..255` (или дробное) на платформе
//! не удалось ИЗМЕРИТЬ вовсе: `Попытка Б[1] = 256 Исключение` не ловит
//! ничего — сеанс 1С умирает молча, и прогон замеров висит до таймаута.
//! Проверено трижды, в том числе прямой `Попыткой` без `Выполнить`, и на
//! дробном значении (`Б[1] = 2.9`) тоже. Наблюдаемый вывод один: значение
//! отвергается; воспроизвести здесь ГИБЕЛЬ СЕАНСА нельзя и не нужно,
//! поэтому такие случаи дают обычную перехватываемую [`crate::RtError`].
//! В фикстуру они по той же причине не входят — платформа не досняла бы
//! эталон.

use std::cell::RefCell;
use std::rc::Rc;

use bsl_number::BslNumber;

use crate::{BslObject, BslValue, EnumValue, RtError, RtResult};

/// Порядок байтов многобайтового целого.
///
/// Печатается «Little-endian»/«Big-endian» — измерено; членов ровно два, и
/// пишутся они одинаково на обоих языках (`ПорядокБайтов.Прямой` и
/// `ПорядокБайтов.Обратный` платформа не знает).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    /// Член перечисления, которым этот порядок виден из BSL.
    pub fn to_enum(self) -> EnumValue {
        match self {
            ByteOrder::Little => EnumValue::ByteOrderLittle,
            ByteOrder::Big => EnumValue::ByteOrderBig,
        }
    }

    /// Обратное преобразование. `None` — член ЧУЖОГО перечисления: платформа
    /// такой аргумент отвергает (измерено на `Новый БуферДвоичныхДанных(4, 5)`).
    pub fn from_enum(e: EnumValue) -> Option<Self> {
        match e {
            EnumValue::ByteOrderLittle => Some(ByteOrder::Little),
            EnumValue::ByteOrderBig => Some(ByteOrder::Big),
            _ => None,
        }
    }
}

/// Содержимое буфера: сами байты и текущий порядок.
///
/// Порядок хранится в буфере, а не только передаётся в методы: свойство
/// `ПорядокБайтов` читается И ПИШЕТСЯ, и запись видна следующему
/// `ЗаписатьЦелое16` без третьего аргумента (измерено — после смены на
/// `BigEndian` число 258 легло байтами `1 2`, а не `2 1`).
#[derive(Debug)]
pub struct BinBufData {
    pub bytes: Vec<u8>,
    pub order: ByteOrder,
}

/// `Новый БуферДвоичныхДанных(Размер[, ПорядокБайтов])`.
///
/// Размер обязателен и фиксирует буфер навсегда; буфер заполняется нулями
/// (измерено). Порядок по умолчанию — `LittleEndian`, тоже измерено.
///
/// # Errors
///
/// [`RtError::TypeError`], если размер не целое неотрицательное число:
/// платформа отвергает `-1`, `2.5` и строку `"4"`, а `0` принимает
/// (измерено); если порядок байтов — не член `ПорядокБайтов` (измерено на
/// `Новый БуферДвоичныхДанных(4, 5)`); а также если буфер такого размера не
/// удалось разместить в памяти — расти на гигабайты по неосторожному
/// literalу лучше отказом, чем падением процесса.
pub fn new_binary_buffer(size: &BslValue, order: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "Новый БуферДвоичныхДанных";
    let bad_size = || RtError::TypeError {
        expected: "Целое неотрицательное число",
        op: OP,
    };
    let n = match size {
        BslValue::Number(n) => n,
        _ => return Err(bad_size()),
    };
    if !n.is_integer() {
        return Err(bad_size());
    }
    let size = usize::try_from(to_u64(n).ok_or_else(bad_size)?).map_err(|_| bad_size())?;
    let order = match order {
        BslValue::Undefined => ByteOrder::Little,
        BslValue::Enum(e) => ByteOrder::from_enum(*e).ok_or(RtError::TypeError {
            expected: "ПорядокБайтов",
            op: OP,
        })?,
        _ => {
            return Err(RtError::TypeError {
                expected: "ПорядокБайтов",
                op: OP,
            })
        }
    };
    // `vec![0; size]` на неразмещаемом размере ПАНИКУЕТ, а размер пришёл из
    // пользовательского текста — значит, отказ обязан быть перехватываемым.
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| RtError::TypeError {
            expected: "Размер, который удаётся разместить в памяти",
            op: OP,
        })?;
    bytes.resize(size, 0);
    Ok(BslValue::Object(Rc::new(BslObject::BinaryBuffer(Rc::new(
        RefCell::new(BinBufData { bytes, order }),
    )))))
}

/// Внутренности буфера; для любого другого значения — ошибка «метод не
/// применим», с именем операции, которая его потребовала.
fn data<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<RefCell<BinBufData>>> {
    match v {
        BslValue::Object(o) => match &**o {
            BslObject::BinaryBuffer(d) => Ok(d),
            _ => Err(RtError::MethodNotApplicable {
                method: op,
                receiver: v.type_name(),
            }),
        },
        _ => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        }),
    }
}

/// `БуферДвоичныхДанных` ли это. Нужно диспетчеризации полиморфных методов
/// (`Получить`, `Разделить`, ...), которые у разных получателей значат разное.
pub fn is_buffer(v: &BslValue) -> bool {
    matches!(v, BslValue::Object(o) if matches!(&**o, BslObject::BinaryBuffer(_)))
}

// --- преобразование чисел ---------------------------------------------------

/// Целое `0..=u64::MAX` из числа BSL. `to_i64_exact` тут мал: `ПрочитатьЦелое64`
/// отдаёт до `18446744073709551615` (измерено на восьми байтах `FF`), и
/// столько же обязана принимать запись, поэтому верхняя половина диапазона
/// добирается вычитанием `2^63`.
fn to_u64(n: &BslNumber) -> Option<u64> {
    if n.is_negative() || !n.is_integer() {
        return None;
    }
    if let Some(v) = n.to_i64_exact() {
        return u64::try_from(v).ok();
    }
    let half = BslNumber::from_parts(1i128 << 63, 0);
    let rest = n.sub(&half).ok()?;
    let rest = u64::try_from(rest.to_i64_exact()?).ok()?;
    (1u64 << 63).checked_add(rest)
}

/// Число из беззнакового целого — обратная сторона [`to_u64`].
fn from_u64(v: u64) -> BslValue {
    BslValue::Number(BslNumber::from_parts(v as i128, 0))
}

/// ПОЗИЦИЯ в буфере: число, дробная часть отбрасывается К НУЛЮ (измерено:
/// `Б[1.9]` -> байт 1, `Б[0.9]` -> байт 0, `Получить(0.5)` -> байт 0).
///
/// Проверка на попадание в буфер делается отдельно, [`checked_pos`]: у
/// `Инвертировать` вторым числом идёт ДЛИНА отрезка, которую в буфер
/// укладывает уже сам метод, а не эта функция.
///
/// Имени операции здесь нет намеренно: [`RtError::BadIndex`] его не несёт, и
/// протаскивать неиспользуемый параметр ради симметрии с соседями незачем.
fn pos_of(v: &BslValue) -> RtResult<u64> {
    let n = match v {
        BslValue::Number(n) => n,
        _ => return Err(RtError::BadIndex),
    };
    to_u64(&n.trunc_to_scale(0)).ok_or(RtError::BadIndex)
}

/// Позиция, про которую известно, что она указывает на существующий байт.
fn checked_pos(v: &BslValue, len: usize) -> RtResult<usize> {
    let p = pos_of(v)?;
    let p = usize::try_from(p).map_err(|_| RtError::BadIndex)?;
    if p >= len {
        return Err(RtError::IndexOutOfBounds {
            index: p as i64,
            len,
        });
    }
    Ok(p)
}

/// ЗНАЧЕНИЕ байта: целое `0..=255`.
///
/// Вне диапазона и дробное — ошибка. Как именно отвечает платформа, снять
/// не удалось (см. заголовок модуля: она уносит сеанс), но то, что значение
/// НЕ принимается, наблюдаемо, а `255` принимается — измерено.
fn byte_of(v: &BslValue, op: &'static str) -> RtResult<u8> {
    let bad = || RtError::TypeError {
        expected: "Целое число от 0 до 255",
        op,
    };
    let n = match v {
        BslValue::Number(n) => n,
        _ => return Err(bad()),
    };
    if !n.is_integer() {
        return Err(bad());
    }
    let v = to_u64(n).ok_or_else(bad)?;
    u8::try_from(v).map_err(|_| bad())
}

// --- свойства ---------------------------------------------------------------

/// `Буфер.Размер` — СВОЙСТВО, не метод: `Б.Размер()` платформа отвергает
/// (измерено), и запись в него тоже.
pub fn size(v: &BslValue) -> RtResult<BslValue> {
    let d = data(v, "Размер")?;
    let len = d.borrow().bytes.len();
    Ok(BslValue::Number(BslNumber::from_i64(len as i64)))
}

/// `Буфер.ПорядокБайтов` — читается и пишется.
pub fn get_order(v: &BslValue) -> RtResult<BslValue> {
    let d = data(v, "ПорядокБайтов")?;
    let order = d.borrow().order;
    Ok(BslValue::Enum(order.to_enum()))
}

/// Запись в `Буфер.ПорядокБайтов`. Не член `ПорядокБайтов` — ошибка
/// (измерено: `Б.ПорядокБайтов = 5` отвергнуто, прежнее значение уцелело).
pub fn set_order(v: &BslValue, val: BslValue) -> RtResult<()> {
    let d = data(v, "ПорядокБайтов")?;
    let order = match val {
        BslValue::Enum(e) => ByteOrder::from_enum(e),
        _ => None,
    }
    .ok_or(RtError::TypeError {
        expected: "ПорядокБайтов",
        op: "ПорядокБайтов",
    })?;
    d.borrow_mut().order = order;
    Ok(())
}

// --- доступ к байту ---------------------------------------------------------

/// `Буфер[Позиция]` и `Буфер.Получить(Позиция)` — одно и то же (измерено).
pub fn get_byte(v: &BslValue, idx: &BslValue) -> RtResult<BslValue> {
    let d = data(v, "Получить")?;
    let d = d.borrow();
    let p = checked_pos(idx, d.bytes.len())?;
    Ok(BslValue::Number(BslNumber::from_i64(d.bytes[p] as i64)))
}

/// `Буфер[Позиция] = Значение` и `Буфер.Установить(Позиция, Значение)`.
pub fn set_byte(v: &BslValue, idx: &BslValue, val: &BslValue) -> RtResult<()> {
    let d = data(v, "Установить")?;
    let mut d = d.borrow_mut();
    let p = checked_pos(idx, d.bytes.len())?;
    d.bytes[p] = byte_of(val, "Установить")?;
    Ok(())
}

// --- целые ------------------------------------------------------------------

/// Ширина целого в байтах. Восьмибитного метода у платформы НЕТ — ни
/// `ПрочитатьЦелое8`, ни `ReadInt8`, ни `ПрочитатьБайт` (проверено
/// перебором); один байт берётся индексом или `Получить`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntWidth {
    W16,
    W32,
    W64,
}

impl IntWidth {
    fn bytes(self) -> usize {
        match self {
            IntWidth::W16 => 2,
            IntWidth::W32 => 4,
            IntWidth::W64 => 8,
        }
    }

    /// Наибольшее значение, которое влезает. Целые БЕЗЗНАКОВЫЕ — измерено:
    /// восемь байтов `FF` читаются как `18446744073709551615`, а не как `-1`.
    fn max(self) -> u64 {
        match self {
            IntWidth::W16 => u16::MAX as u64,
            IntWidth::W32 => u32::MAX as u64,
            IntWidth::W64 => u64::MAX,
        }
    }

    /// Имя для диагностики — то же, что стоит в исходном тексте BSL.
    fn read_op(self) -> &'static str {
        match self {
            IntWidth::W16 => "ПрочитатьЦелое16",
            IntWidth::W32 => "ПрочитатьЦелое32",
            IntWidth::W64 => "ПрочитатьЦелое64",
        }
    }

    fn write_op(self) -> &'static str {
        match self {
            IntWidth::W16 => "ЗаписатьЦелое16",
            IntWidth::W32 => "ЗаписатьЦелое32",
            IntWidth::W64 => "ЗаписатьЦелое64",
        }
    }
}

/// Порядок для этой операции: явный третий (у чтения — второй) аргумент,
/// иначе собственный порядок буфера.
fn order_arg(arg: Option<&BslValue>, own: ByteOrder, op: &'static str) -> RtResult<ByteOrder> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(own),
        Some(BslValue::Enum(e)) => ByteOrder::from_enum(*e).ok_or(RtError::TypeError {
            expected: "ПорядокБайтов",
            op,
        }),
        Some(_) => Err(RtError::TypeError {
            expected: "ПорядокБайтов",
            op,
        }),
    }
}

/// `ПрочитатьЦелое16/32/64(Позиция[, ПорядокБайтов])`.
///
/// # Errors
///
/// [`RtError::MethodNotApplicable`] — получатель не буфер;
/// [`RtError::BadIndex`] — позиция не число; [`RtError::IndexOutOfBounds`] —
/// целое не влезает в остаток буфера (измерено: чтение `Целое16` с позиции
/// 3 в буфере из четырёх байтов платформа отвергает, а не добивает нулями);
/// [`RtError::TypeError`] — аргумент порядка байтов не член `ПорядокБайтов`.
pub fn read_int(v: &BslValue, args: &[BslValue], w: IntWidth) -> RtResult<BslValue> {
    let op = w.read_op();
    let d = data(v, op)?;
    let d = d.borrow();
    let [pos, rest @ ..] = args else {
        return Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        });
    };
    if rest.len() > 1 {
        return Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        });
    }
    let order = order_arg(rest.first(), d.order, op)?;
    let p = checked_pos(pos, d.bytes.len())?;
    let end = p.checked_add(w.bytes()).ok_or(RtError::BadIndex)?;
    if end > d.bytes.len() {
        return Err(RtError::IndexOutOfBounds {
            index: p as i64,
            len: d.bytes.len(),
        });
    }
    let mut acc: u64 = 0;
    for i in 0..w.bytes() {
        let b = d.bytes[p + i] as u64;
        match order {
            ByteOrder::Little => acc |= b << (8 * i),
            ByteOrder::Big => acc = (acc << 8) | b,
        }
    }
    Ok(from_u64(acc))
}

/// `ЗаписатьЦелое16/32/64(Позиция, Значение[, ПорядокБайтов])`.
///
/// # Errors
///
/// Те же, что у [`read_int`], плюс [`RtError::TypeError`], если значение не
/// целое из `0..=2^N-1`. Верхний край измерен: `65535` и `4294967295`
/// платформа принимает.
pub fn write_int(v: &BslValue, args: &[BslValue], w: IntWidth) -> RtResult<BslValue> {
    let op = w.write_op();
    let d = data(v, op)?;
    let mut d = d.borrow_mut();
    let [pos, value, rest @ ..] = args else {
        return Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        });
    };
    if rest.len() > 1 {
        return Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        });
    }
    let order = order_arg(rest.first(), d.order, op)?;
    let p = checked_pos(pos, d.bytes.len())?;
    let end = p.checked_add(w.bytes()).ok_or(RtError::BadIndex)?;
    if end > d.bytes.len() {
        return Err(RtError::IndexOutOfBounds {
            index: p as i64,
            len: d.bytes.len(),
        });
    }
    let bad = || RtError::TypeError {
        expected: "Целое неотрицательное число, влезающее в целое этой ширины",
        op,
    };
    let n = match value {
        BslValue::Number(n) => n,
        _ => return Err(bad()),
    };
    let value = to_u64(n).ok_or_else(bad)?;
    if value > w.max() {
        return Err(bad());
    }
    for i in 0..w.bytes() {
        let shift = match order {
            ByteOrder::Little => 8 * i,
            ByteOrder::Big => 8 * (w.bytes() - 1 - i),
        };
        d.bytes[p + i] = ((value >> shift) & 0xff) as u8;
    }
    Ok(BslValue::Undefined)
}

// --- Разделить / Соединить --------------------------------------------------

/// `Буфер.Разделить(Разделитель)` -> `Массив` буферов.
///
/// Разделитель — БУФЕР, а не размер части: это раскрой по вхождениям, как у
/// строкового `СтрРазделить`, а не нарезка на куски равной длины (измерено:
/// число платформа отвергает, буфер принимает). `[1,2,0,3,4,0,5]` по `[0]`
/// даёт три части `[1,2]`, `[3,4]`, `[5]`; разделителя нет в буфере — одна
/// часть целиком; `[1,0,0,2,0,0,3]` по `[0,0]` — три части.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не буфер либо разделитель ПУСТ
/// (измерено: платформа отвергает и то, и другое).
pub fn split(v: &BslValue, sep: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "Разделить";
    let d = data(v, OP)?;
    let sep = match sep {
        BslValue::Object(o) => match &**o {
            BslObject::BinaryBuffer(s) => s,
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: OP,
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "БуферДвоичныхДанных",
                op: OP,
            })
        }
    };
    // Разделитель и получатель могут быть ОДНИМ объектом (`Б.Разделить(Б)`),
    // а `RefCell` двух заимствований подряд не даст — копируем байты.
    let sep_bytes = sep.borrow().bytes.clone();
    let d = d.borrow();
    if sep_bytes.is_empty() {
        return Err(RtError::TypeError {
            expected: "Непустой БуферДвоичныхДанных",
            op: OP,
        });
    }
    let order = d.order;
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i + sep_bytes.len() <= d.bytes.len() {
        if d.bytes[i..i + sep_bytes.len()] == sep_bytes[..] {
            parts.push(make_part(&d.bytes[start..i], order));
            i += sep_bytes.len();
            start = i;
        } else {
            i += 1;
        }
    }
    parts.push(make_part(&d.bytes[start..], order));
    Ok(BslValue::new_array(parts))
}

/// Часть раскроя — самостоятельный буфер, а не вид на исходный.
fn make_part(bytes: &[u8], order: ByteOrder) -> BslValue {
    BslValue::Object(Rc::new(BslObject::BinaryBuffer(Rc::new(RefCell::new(
        BinBufData {
            bytes: bytes.to_vec(),
            order,
        },
    )))))
}

/// `Буфер.Соединить(Другой)` -> НОВЫЙ буфер.
///
/// Получатель не растёт (измерено: `Размер` после вызова прежний), а
/// порядок байтов результата наследуется от ЛЕВОГО буфера — тоже измерено,
/// на паре `BigEndian` + буфер по умолчанию.
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не буфер: строку, число и
/// `Неопределено` платформа отвергает.
pub fn concat(v: &BslValue, other: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "Соединить";
    let d = data(v, OP)?;
    let other = match other {
        BslValue::Object(o) => match &**o {
            BslObject::BinaryBuffer(s) => s,
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: OP,
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "БуферДвоичныхДанных",
                op: OP,
            })
        }
    };
    // `Б.Соединить(Б)` измерено — даёт удвоенный размер, поэтому второе
    // заимствование берётся копией, а не одновременно с первым.
    let tail = other.borrow().bytes.clone();
    let d = d.borrow();
    let mut bytes = Vec::with_capacity(d.bytes.len() + tail.len());
    bytes.extend_from_slice(&d.bytes);
    bytes.extend_from_slice(&tail);
    Ok(BslValue::Object(Rc::new(BslObject::BinaryBuffer(Rc::new(
        RefCell::new(BinBufData {
            bytes,
            order: d.order,
        }),
    )))))
}

// --- побитовые --------------------------------------------------------------

/// Какую побитовую операцию накладывать. Раздельные варианты, а не
/// замыкание: диспетчер `call_builtin_method` матчится по имени метода
/// исчерпывающе, и новая операция обязана быть заведена и здесь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOp {
    And,
    Or,
    Xor,
    AndNot,
}

impl BitOp {
    fn apply(self, dst: u8, mask: u8) -> u8 {
        match self {
            BitOp::And => dst & mask,
            BitOp::Or => dst | mask,
            BitOp::Xor => dst ^ mask,
            // `12 И НЕ 10` = 12 & 245 = 4 — измерено.
            BitOp::AndNot => dst & !mask,
        }
    }

    fn op_name(self) -> &'static str {
        match self {
            BitOp::And => "ЗаписатьПобитовоеИ",
            BitOp::Or => "ЗаписатьПобитовоеИли",
            BitOp::Xor => "ЗаписатьПобитовоеИсключительноеИли",
            BitOp::AndNot => "ЗаписатьПобитовоеИНе",
        }
    }
}

/// `ЗаписатьПобитовоеИ/Или/ИсключительноеИли/ИНе(Позиция, Маска)`.
///
/// Маска — БУФЕР (число платформа отвергает), накладывается начиная с
/// `Позиция`. Маска, не влезающая в остаток буфера, — ошибка; ПУСТАЯ маска
/// ошибкой не является и ничего не меняет (всё измерено).
///
/// # Errors
///
/// [`RtError::TypeError`] — маска не буфер; [`RtError::IndexOutOfBounds`] —
/// позиция вне буфера либо маска длиннее его остатка.
pub fn bitwise(v: &BslValue, args: &[BslValue], op: BitOp) -> RtResult<BslValue> {
    let name = op.op_name();
    let d = data(v, name)?;
    let [pos, mask] = args else {
        return Err(RtError::MethodNotApplicable {
            method: name,
            receiver: v.type_name(),
        });
    };
    let mask = match mask {
        BslValue::Object(o) => match &**o {
            BslObject::BinaryBuffer(m) => m,
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: name,
                })
            }
        },
        _ => {
            return Err(RtError::TypeError {
                expected: "БуферДвоичныхДанных",
                op: name,
            })
        }
    };
    // Маской может быть сам получатель — берём её копией до `borrow_mut`.
    let mask_bytes = mask.borrow().bytes.clone();
    let mut d = d.borrow_mut();
    let p = checked_pos(pos, d.bytes.len())?;
    let end = p.checked_add(mask_bytes.len()).ok_or(RtError::BadIndex)?;
    if end > d.bytes.len() {
        return Err(RtError::IndexOutOfBounds {
            index: p as i64,
            len: d.bytes.len(),
        });
    }
    for (i, m) in mask_bytes.iter().enumerate() {
        d.bytes[p + i] = op.apply(d.bytes[p + i], *m);
    }
    Ok(BslValue::Undefined)
}

/// `Инвертировать([Позиция][, Количество])` — побитовое НЕ по месту.
///
/// Без аргументов инвертируется весь буфер, с одним — от позиции ДО КОНЦА,
/// с двумя — ровно `Количество` байтов. `Количество = 0` означает «до
/// конца», а не «ничего»: измерено — `Инвертировать(0, 0)` перевернуло все
/// четыре байта.
///
/// # Errors
///
/// [`RtError::IndexOutOfBounds`] — позиция вне буфера либо количество
/// выходит за его конец (измерено: `Инвертировать(0, 99)` на четырёх
/// байтах отвергнуто); [`RtError::BadIndex`] — позиция или количество не
/// целое неотрицательное число.
pub fn invert(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "Инвертировать";
    let d = data(v, OP)?;
    let mut d = d.borrow_mut();
    let total = d.bytes.len();
    let (from, count) = match args {
        // Пустой буфер платформа отвергает даже без аргументов (измерено):
        // позиция 0 в нём уже не указывает ни на какой байт.
        [] if total == 0 => return Err(RtError::IndexOutOfBounds { index: 0, len: 0 }),
        [] => (0usize, total),
        [pos] => {
            let p = checked_pos(pos, total)?;
            (p, total - p)
        }
        [pos, count] => {
            let p = checked_pos(pos, total)?;
            let c = pos_of(count)?;
            let c = usize::try_from(c).map_err(|_| RtError::BadIndex)?;
            // Ноль — «до конца», измерено; иначе хвост обязан влезать.
            let c = if c == 0 { total - p } else { c };
            if p + c > total {
                return Err(RtError::IndexOutOfBounds {
                    index: (p + c) as i64,
                    len: total,
                });
            }
            (p, c)
        }
        _ => {
            return Err(RtError::MethodNotApplicable {
                method: OP,
                receiver: v.type_name(),
            })
        }
    };
    for b in &mut d.bytes[from..from + count] {
        *b = !*b;
    }
    Ok(BslValue::Undefined)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Буфер с заданным содержимым — через тот же конструктор, что и BSL,
    /// плюс прямая заливка байтов: `Установить` здесь был бы лишним звеном
    /// в тестах, которые как раз его и проверяют.
    fn buf(bytes: &[u8]) -> BslValue {
        let v = new_binary_buffer(
            &BslValue::Number(BslNumber::from_i64(bytes.len() as i64)),
            &BslValue::Undefined,
        )
        .expect("размер из длины среза всегда годен");
        data(&v, "тест")
            .expect("только что созданный буфер")
            .borrow_mut()
            .bytes
            .copy_from_slice(bytes);
        v
    }

    fn num(v: i64) -> BslValue {
        BslValue::Number(BslNumber::from_i64(v))
    }

    fn dec(text: &str) -> BslValue {
        BslValue::Number(BslNumber::parse_canonical(text).expect("литерал числа"))
    }

    fn dump(v: &BslValue) -> Vec<u8> {
        data(v, "тест").unwrap().borrow().bytes.clone()
    }

    fn as_u64(v: &BslValue) -> u64 {
        match v {
            BslValue::Number(n) => to_u64(n).expect("беззнаковое целое"),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    #[test]
    fn the_constructor_checks_both_arguments() {
        let le = BslValue::Enum(EnumValue::ByteOrderLittle);
        let be = BslValue::Enum(EnumValue::ByteOrderBig);
        let b = new_binary_buffer(&num(4), &BslValue::Undefined).unwrap();
        assert_eq!(dump(&b), vec![0, 0, 0, 0]);
        assert_eq!(get_order(&b).unwrap(), le);
        // Размер 0 платформа принимает.
        assert_eq!(
            dump(&new_binary_buffer(&num(0), &BslValue::Undefined).unwrap()),
            Vec::<u8>::new()
        );
        // А отрицательный, дробный и строковый — нет.
        assert!(new_binary_buffer(&num(-1), &BslValue::Undefined).is_err());
        assert!(new_binary_buffer(&dec("2.5"), &BslValue::Undefined).is_err());
        assert!(new_binary_buffer(
            &BslValue::Str(crate::BslString::from_str("4")),
            &BslValue::Undefined
        )
        .is_err());
        // Порядок байтов — только член перечисления.
        assert_eq!(
            get_order(&new_binary_buffer(&num(2), &be).unwrap()).unwrap(),
            be
        );
        assert!(new_binary_buffer(&num(2), &num(5)).is_err());
        assert!(new_binary_buffer(&num(2), &BslValue::Enum(EnumValue::JsonString)).is_err());
    }

    /// Заведомо неразмещаемый размер — перехватываемая ошибка, а не паника
    /// на `vec![0; size]`: число приходит из пользовательского текста.
    #[test]
    fn an_unallocatable_size_is_an_error_not_a_panic() {
        assert!(new_binary_buffer(&dec("18446744073709551615"), &BslValue::Undefined).is_err());
    }

    #[test]
    fn new_buffer_is_zero_filled_and_little_endian() {
        let b = buf(&[0; 4]);
        assert_eq!(dump(&b), vec![0, 0, 0, 0]);
        assert_eq!(
            get_order(&b).unwrap(),
            BslValue::Enum(EnumValue::ByteOrderLittle)
        );
        assert_eq!(size(&b).unwrap(), num(4));
    }

    /// Два свежих одинаковых буфера НЕ равны — измерено; равенство у этого
    /// типа по тождеству, в отличие от `ДвоичныеДанные`.
    #[test]
    fn two_buffers_with_equal_content_are_not_equal() {
        assert_ne!(buf(&[0, 0]), buf(&[0, 0]));
        let b = buf(&[1, 2]);
        assert_eq!(b, b.clone());
    }

    #[test]
    fn byte_access_reads_and_writes() {
        let b = buf(&[0, 0, 0, 0]);
        set_byte(&b, &num(0), &num(255)).unwrap();
        set_byte(&b, &num(3), &num(1)).unwrap();
        assert_eq!(dump(&b), vec![255, 0, 0, 1]);
        assert_eq!(get_byte(&b, &num(0)).unwrap(), num(255));
    }

    /// Дробная позиция отбрасывается к нулю: `1.9` — байт 1, а не 2.
    #[test]
    fn fractional_position_truncates_towards_zero() {
        let b = buf(&[10, 11, 12, 13]);
        assert_eq!(get_byte(&b, &dec("1.5")).unwrap(), num(11));
        assert_eq!(get_byte(&b, &dec("1.9")).unwrap(), num(11));
        assert_eq!(get_byte(&b, &dec("0.9")).unwrap(), num(10));
        assert_eq!(get_byte(&b, &dec("3.9")).unwrap(), num(13));
    }

    #[test]
    fn byte_access_outside_the_buffer_is_a_catchable_error() {
        let b = buf(&[1, 2]);
        assert!(get_byte(&b, &num(2)).is_err());
        assert!(get_byte(&b, &num(-1)).is_err());
        assert!(get_byte(&b, &BslValue::Str(crate::BslString::from_str("1"))).is_err());
        assert!(set_byte(&b, &num(2), &num(1)).is_err());
        assert!(set_byte(&b, &num(-1), &num(1)).is_err());
    }

    /// Значение вне `0..255` и дробное отвергается (сама платформа на этом
    /// уносит сеанс — см. заголовок модуля).
    #[test]
    fn byte_value_outside_the_range_is_rejected() {
        let b = buf(&[0, 0]);
        assert!(set_byte(&b, &num(0), &num(256)).is_err());
        assert!(set_byte(&b, &num(0), &num(-1)).is_err());
        assert!(set_byte(&b, &num(0), &dec("2.5")).is_err());
        assert!(set_byte(&b, &num(0), &BslValue::Str(crate::BslString::from_str("7"))).is_err());
        // А край диапазона принимается — это измерено.
        assert!(set_byte(&b, &num(0), &num(255)).is_ok());
        assert_eq!(dump(&b), vec![255, 0]);
    }

    #[test]
    fn integers_round_trip_in_both_byte_orders() {
        let b = buf(&[0; 8]);
        write_int(&b, &[num(0), num(258)], IntWidth::W16).unwrap();
        assert_eq!(dump(&b)[..2], [2, 1]);
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W16).unwrap()),
            258
        );

        let be = BslValue::Enum(EnumValue::ByteOrderBig);
        write_int(&b, &[num(2), num(258), be.clone()], IntWidth::W16).unwrap();
        assert_eq!(dump(&b)[2..4], [1, 2]);
        assert_eq!(
            as_u64(&read_int(&b, &[num(2), be.clone()], IntWidth::W16).unwrap()),
            258
        );
        // Тот же байт, прочитанный другим порядком, — другое число.
        assert_eq!(
            as_u64(&read_int(&b, &[num(2)], IntWidth::W16).unwrap()),
            513
        );

        let b = buf(&[0; 8]);
        write_int(&b, &[num(0), num(16909060)], IntWidth::W32).unwrap();
        assert_eq!(dump(&b)[..4], [4, 3, 2, 1]);
        write_int(&b, &[num(4), num(16909060), be], IntWidth::W32).unwrap();
        assert_eq!(dump(&b)[4..], [1, 2, 3, 4]);

        let b = buf(&[0; 8]);
        write_int(&b, &[num(0), num(72623859790382856)], IntWidth::W64).unwrap();
        assert_eq!(dump(&b), vec![8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W64).unwrap()),
            72623859790382856
        );
    }

    /// Целые БЕЗЗНАКОВЫЕ: восемь `FF` — это `2^64-1`, а не `-1`.
    #[test]
    fn integers_are_unsigned_across_the_whole_range() {
        let b = buf(&[255; 8]);
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W16).unwrap()),
            65535
        );
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W32).unwrap()),
            4294967295
        );
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W64).unwrap()),
            u64::MAX
        );
        // И записать столько же можно.
        let b = buf(&[0; 8]);
        write_int(&b, &[num(0), dec("18446744073709551615")], IntWidth::W64).unwrap();
        assert_eq!(dump(&b), vec![255; 8]);
        assert!(write_int(&b, &[num(0), dec("18446744073709551616")], IntWidth::W64).is_err());
    }

    #[test]
    fn integer_write_rejects_values_that_do_not_fit() {
        let b = buf(&[0; 8]);
        assert!(write_int(&b, &[num(0), num(65535)], IntWidth::W16).is_ok());
        assert!(write_int(&b, &[num(0), num(65536)], IntWidth::W16).is_err());
        assert!(write_int(&b, &[num(0), num(-1)], IntWidth::W16).is_err());
        assert!(write_int(&b, &[num(0), dec("1.5")], IntWidth::W16).is_err());
        assert!(write_int(&b, &[num(0), num(4294967295)], IntWidth::W32).is_ok());
        assert!(write_int(&b, &[num(0), dec("4294967296")], IntWidth::W32).is_err());
    }

    /// Многобайтовое целое у края буфера не добивается нулями — это ошибка.
    #[test]
    fn integer_access_past_the_end_is_an_error() {
        let b = buf(&[0; 4]);
        assert!(read_int(&b, &[num(3)], IntWidth::W16).is_err());
        assert!(read_int(&b, &[num(1)], IntWidth::W32).is_err());
        assert!(read_int(&b, &[num(0)], IntWidth::W64).is_err());
        assert!(write_int(&b, &[num(3), num(1)], IntWidth::W16).is_err());
        assert!(read_int(&b, &[num(-1)], IntWidth::W16).is_err());
        assert!(read_int(&b, &[num(4)], IntWidth::W16).is_err());
        // Ровно впритык — можно.
        assert!(read_int(&b, &[num(2)], IntWidth::W16).is_ok());
    }

    /// Собственный порядок буфера — не украшение: он и есть значение по
    /// умолчанию для методов без явного аргумента.
    #[test]
    fn the_buffers_own_byte_order_drives_the_default() {
        let b = buf(&[0; 4]);
        set_order(&b, BslValue::Enum(EnumValue::ByteOrderBig)).unwrap();
        write_int(&b, &[num(0), num(258)], IntWidth::W16).unwrap();
        assert_eq!(dump(&b)[..2], [1, 2]);
        assert_eq!(
            as_u64(&read_int(&b, &[num(0)], IntWidth::W16).unwrap()),
            258
        );
        assert!(set_order(&b, num(5)).is_err());
        // Отвергнутая запись прежнее значение не портит.
        assert_eq!(
            get_order(&b).unwrap(),
            BslValue::Enum(EnumValue::ByteOrderBig)
        );
    }

    fn part_bytes(parts: &BslValue) -> Vec<Vec<u8>> {
        let BslValue::Object(o) = parts else {
            panic!("разбиение обязано отдать массив");
        };
        let BslObject::Array(items) = &**o else {
            panic!("разбиение обязано отдать массив");
        };
        items.borrow().iter().map(dump).collect()
    }

    #[test]
    fn split_cuts_on_every_occurrence_of_the_separator() {
        let b = buf(&[1, 2, 0, 3, 4, 0, 5]);
        let parts = split(&b, &buf(&[0])).unwrap();
        assert_eq!(part_bytes(&parts), vec![vec![1, 2], vec![3, 4], vec![5]]);
    }

    #[test]
    fn split_without_a_match_yields_the_whole_buffer() {
        let b = buf(&[1, 2, 3]);
        let parts = split(&b, &buf(&[99])).unwrap();
        assert_eq!(part_bytes(&parts), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn split_handles_a_multibyte_separator() {
        let b = buf(&[1, 0, 0, 2, 0, 0, 3]);
        let parts = split(&b, &buf(&[0, 0])).unwrap();
        assert_eq!(part_bytes(&parts), vec![vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn split_rejects_an_empty_or_non_buffer_separator() {
        let b = buf(&[1, 2]);
        assert!(split(&b, &buf(&[])).is_err());
        assert!(split(&b, &num(1)).is_err());
        assert!(split(&b, &BslValue::Undefined).is_err());
    }

    #[test]
    fn concat_builds_a_new_buffer_and_leaves_the_receiver_alone() {
        let a = buf(&[1, 2, 3, 4]);
        let b = buf(&[100, 200]);
        let joined = concat(&a, &b).unwrap();
        assert_eq!(dump(&joined), vec![1, 2, 3, 4, 100, 200]);
        assert_eq!(dump(&a), vec![1, 2, 3, 4]);
        // Порядок наследуется от левого.
        set_order(&a, BslValue::Enum(EnumValue::ByteOrderBig)).unwrap();
        let joined = concat(&a, &b).unwrap();
        assert_eq!(
            get_order(&joined).unwrap(),
            BslValue::Enum(EnumValue::ByteOrderBig)
        );
        // Сам с собой — удвоение.
        let doubled = concat(&b, &b).unwrap();
        assert_eq!(dump(&doubled), vec![100, 200, 100, 200]);
        assert!(concat(&a, &num(5)).is_err());
    }

    #[test]
    fn bitwise_ops_apply_the_mask_at_the_given_position() {
        let b = buf(&[12, 12]);
        bitwise(&b, &[num(0), buf(&[10, 10])], BitOp::And).unwrap();
        assert_eq!(dump(&b), vec![8, 8]);

        let b = buf(&[12, 12]);
        bitwise(&b, &[num(0), buf(&[3, 3])], BitOp::Or).unwrap();
        assert_eq!(dump(&b), vec![15, 15]);

        let b = buf(&[12, 12]);
        bitwise(&b, &[num(0), buf(&[10, 10])], BitOp::Xor).unwrap();
        assert_eq!(dump(&b), vec![6, 6]);

        let b = buf(&[12, 12]);
        bitwise(&b, &[num(0), buf(&[10, 10])], BitOp::AndNot).unwrap();
        assert_eq!(dump(&b), vec![4, 4]);

        // Смещение и краевые случаи.
        let b = buf(&[12, 12]);
        bitwise(&b, &[num(1), buf(&[10])], BitOp::And).unwrap();
        assert_eq!(dump(&b), vec![12, 8]);
        assert!(bitwise(&b, &[num(1), buf(&[10, 10])], BitOp::And).is_err());
        assert!(bitwise(&b, &[num(-1), buf(&[10])], BitOp::And).is_err());
        assert!(bitwise(&b, &[num(2), buf(&[10])], BitOp::And).is_err());
        assert!(bitwise(&b, &[num(0), num(10)], BitOp::And).is_err());
        // Пустая маска — не ошибка и ничего не меняет.
        assert!(bitwise(&b, &[num(0), buf(&[])], BitOp::And).is_ok());
        assert_eq!(dump(&b), vec![12, 8]);
    }

    #[test]
    fn invert_covers_the_whole_buffer_the_tail_or_a_span() {
        let b = buf(&[12, 10]);
        invert(&b, &[]).unwrap();
        assert_eq!(dump(&b), vec![243, 245]);

        let b = buf(&[1, 2, 3, 4]);
        invert(&b, &[num(1)]).unwrap();
        assert_eq!(dump(&b), vec![1, 253, 252, 251]);

        let b = buf(&[1, 2, 3, 4]);
        invert(&b, &[num(0), num(2)]).unwrap();
        assert_eq!(dump(&b), vec![254, 253, 3, 4]);
    }

    /// `Количество = 0` — «до конца», а не «ничего»: измерено.
    #[test]
    fn invert_with_a_zero_count_runs_to_the_end() {
        let b = buf(&[1, 2, 3, 4]);
        invert(&b, &[num(1), num(0)]).unwrap();
        assert_eq!(dump(&b), vec![1, 253, 252, 251]);
    }

    #[test]
    fn invert_rejects_a_span_that_leaves_the_buffer() {
        let b = buf(&[1, 2, 3, 4]);
        assert!(invert(&b, &[num(0), num(99)]).is_err());
        assert!(invert(&b, &[num(4)]).is_err());
        assert!(invert(&b, &[num(-1)]).is_err());
        assert!(invert(&b, &[num(0), num(-1)]).is_err());
        assert_eq!(dump(&b), vec![1, 2, 3, 4]);
        // Ровно до конца — можно.
        assert!(invert(&b, &[num(0), num(4)]).is_ok());
        assert_eq!(dump(&b), vec![254, 253, 252, 251]);
    }

    /// Пустой буфер инвертировать нельзя даже без аргументов — измерено.
    #[test]
    fn invert_rejects_an_empty_buffer() {
        assert!(invert(&buf(&[]), &[]).is_err());
    }

    /// Методы буфера на чужом получателе — «метод не применим», а не паника.
    #[test]
    fn buffer_methods_reject_a_foreign_receiver() {
        let s = BslValue::Str(crate::BslString::from_str("не буфер"));
        assert!(size(&s).is_err());
        assert!(get_order(&s).is_err());
        assert!(get_byte(&s, &num(0)).is_err());
        assert!(read_int(&s, &[num(0)], IntWidth::W16).is_err());
        assert!(split(&s, &buf(&[0])).is_err());
        assert!(concat(&s, &buf(&[0])).is_err());
        assert!(invert(&s, &[]).is_err());
    }
}
