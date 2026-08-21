//! Побитовые и «числовые» глобальные функции: `ПобитовоеИ`, `ПобитовоеИли`,
//! `ПобитовоеНе`, `ПобитовоеИНе`, `ПобитовоеИсключительноеИли`, оба
//! `ПобитовыйСдвиг`, `ПроверитьБит`, `ПроверитьПоБитовойМаске`,
//! `УстановитьБит`, `ЧислоИзШестнадцатеричнойСтроки` и
//! `ЧислоИзДвоичнойСтроки`.
//!
//! # Что ИЗМЕРЕНО на 8.3.27
//!
//! Всё, что здесь описано, снято фикстурой `tests/conformance/fixtures/
//! binary-bitops.bsl`; краевые случаи этого семейства платформа отвергает
//! ПЕРЕХВАТЫВАЕМЫМ исключением, а не гибелью сеанса, — в отличие от записи в
//! байт `БуфераДвоичныхДанных`, поэтому в фикстуре стоят и
//! они.
//!
//! * Операнд побитовых — целое `0..=4294967295`. Отрицательное, дробное и
//!   `2^32` дают «Недопустимое значение аргумента», не-число — «Несоответствие
//!   типов»; и то и другое здесь — [`RtError::TypeError`].
//! * Номер бита у `ПроверитьБит` и `УстановитьБит` — целое `0..=31`; `32`
//!   отвергается.
//! * Величина сдвига — тоже `0..=31`, и сдвиг на `32` или `33` — именно
//!   ОШИБКА, а не нулевой результат. Это единственное место, где здравый
//!   смысл («сдвинули всё вон, значит ноль») разошёлся бы с платформой.
//! * Вытесненные влево старшие биты теряются молча: `2147483648 << 1` — это
//!   `0`, а не переполнение.
//! * `ПроверитьПоБитовойМаске` — это «ВСЕ биты маски стоят в числе»
//!   (`Число И Маска = Маска`), а не «хотя бы один»: `(12, 4)` — `Да`,
//!   `(12, 5)` — `Нет`, `(12, 0)` — `Да`.
//! * Третий аргумент `УстановитьБит` ОБЯЗАТЕЛЕН и обязан быть `Булево`:
//!   вызов с двумя аргументами не компилируется, а `1` вместо `Истина` даёт
//!   «Несоответствие типов (параметр номер '3')».
//! * Оба разбора строк требуют приставку, и приставка РЕГИСТРОЗАВИСИМА:
//!   `0xFF` и `0xff` дают 255, а `0XFF` отвергается; `0b1010` даёт 10,
//!   `0B1010` отвергается. Без приставки (`FF`, `1010`), пустая строка, одна
//!   приставка (`0x`), пробелы по краям, посторонний знак и минус —
//!   отвергаются; аргумент-не-строка тоже.
//! * Разбор НЕ ограничен 32 битами и вообще ничем на нашей стороне: измерено,
//!   что `0xFFFFFFFFFF` -> 1099511627775, шестнадцать `F` -> `u64::MAX`,
//!   тридцать два `F` -> `u128::MAX`, а СОРОК `F` -> 2^160-1, то есть 49
//!   десятичных цифр. Поэтому значение копится не в `u64` и не в `u128`, а
//!   средствами самого [`BslNumber`], у которого мантисса — `BigInt`.

use bsl_number::BslNumber;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, FunctionCode,
    FunctionDescriptor, FunctionKind, LibraryDescriptor, RtError, RtResult,
};

/// Идентификатор компонента в заголовке байткода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байткода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Проверяет, что значение является `БуферомДвоичныхДанных`.
pub fn is_binary_buffer(value: &BslValue) -> bool {
    value.binary_buffer_len().is_some()
}

/// Размер `БуфераДвоичныхДанных`.
pub fn binary_buffer_len(value: &BslValue) -> Option<usize> {
    value.binary_buffer_len()
}

/// Снимок байтов `БуфераДвоичныхДанных`.
pub fn binary_buffer_bytes(value: &BslValue) -> Option<Vec<u8>> {
    value.binary_buffer_bytes()
}

/// Копирует не больше `count` байтов буфера с заданной позиции.
pub fn binary_buffer_slice(value: &BslValue, offset: u64, count: usize) -> Option<Vec<u8>> {
    value.binary_buffer_slice(offset, count)
}

/// Записывает отрезок в буфер без изменения его размера.
///
/// # Errors
///
/// Возвращает ошибку типа или границ из `bsl-rt`.
pub fn binary_buffer_write(value: &BslValue, offset: usize, bytes: &[u8]) -> RtResult<()> {
    value.binary_buffer_write(offset, bytes)
}

/// Создаёт `БуферДвоичныхДанных` над готовыми байтами.
pub fn binary_buffer_of(bytes: Vec<u8>) -> BslValue {
    BslValue::binary_buffer_of(bytes)
}

/// Байты `ДвоичныхДанных` без копирования.
pub fn binary_data_bytes(value: &BslValue) -> Option<&[u8]> {
    value.binary_data_bytes()
}

/// Операнд побитовой функции: целое `0..=4294967295` (измерено).
///
/// Один помощник на все двенадцать функций — чтобы граница `2^32` была
/// записана ровно один раз.
fn u32_arg(v: &BslValue, op: &'static str) -> RtResult<u32> {
    let bad = || RtError::TypeError {
        expected: "Целое число от 0 до 4294967295",
        op,
    };
    let n = match v {
        BslValue::Number(n) => n,
        _ => return Err(bad()),
    };
    if !n.is_integer() {
        return Err(bad());
    }
    // `to_i64_exact` отсекает всё, что не влезло в `i64`, а `try_from` —
    // отрицательное и то, что не влезло в 32 бита. Обе границы измерены:
    // `4294967295` принимается, `4294967296` и `-1` — нет.
    let n = n.to_i64_exact().ok_or_else(bad)?;
    u32::try_from(n).map_err(|_| bad())
}

/// Номер бита либо величина сдвига: целое `0..=31` (измерено — `32`
/// отвергается и там, и там).
fn bit_index_arg(v: &BslValue, op: &'static str) -> RtResult<u32> {
    let bad = || RtError::TypeError {
        expected: "Целое число от 0 до 31",
        op,
    };
    let n = match v {
        BslValue::Number(n) => n,
        _ => return Err(bad()),
    };
    if !n.is_integer() {
        return Err(bad());
    }
    let n = n.to_i64_exact().ok_or_else(bad)?;
    let n = u32::try_from(n).map_err(|_| bad())?;
    if n > 31 {
        return Err(bad());
    }
    Ok(n)
}

/// Третий аргумент `УстановитьБит`: строго `Булево` (измерено — `1` не
/// подходит).
fn bool_arg(v: &BslValue, op: &'static str) -> RtResult<bool> {
    match v {
        BslValue::Boolean(b) => Ok(*b),
        _ => Err(RtError::TypeError {
            expected: "Булево",
            op,
        }),
    }
}

/// Результат побитовой функции — всегда целое число.
fn number(v: u32) -> BslValue {
    BslValue::Number(BslNumber::from_parts(i128::from(v), 0))
}

/// `ПобитовоеИ(Число1, Число2)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295`.
pub fn and(a: &BslValue, b: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовоеИ";
    Ok(number(u32_arg(a, OP)? & u32_arg(b, OP)?))
}

/// `ПобитовоеИли(Число1, Число2)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295`.
pub fn or(a: &BslValue, b: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовоеИли";
    Ok(number(u32_arg(a, OP)? | u32_arg(b, OP)?))
}

/// `ПобитовоеНе(Число)` — дополнение в 32 разрядах: `ПобитовоеНе(0)` — это
/// `4294967295` (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295`.
pub fn not(a: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовоеНе";
    Ok(number(!u32_arg(a, OP)?))
}

/// `ПобитовоеИНе(Число1, Число2)` — это `Число1 И (НЕ Число2)`: измерено на
/// `(12, 10)` -> 4.
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295`.
pub fn and_not(a: &BslValue, b: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовоеИНе";
    Ok(number(u32_arg(a, OP)? & !u32_arg(b, OP)?))
}

/// `ПобитовоеИсключительноеИли(Число1, Число2)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295`.
pub fn xor(a: &BslValue, b: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовоеИсключительноеИли";
    Ok(number(u32_arg(a, OP)? ^ u32_arg(b, OP)?))
}

/// `ПобитовыйСдвигВлево(Число, Сдвиг)`. Вытесненные старшие биты теряются
/// (измерено: `2147483648` со сдвигом 1 — это `0`).
///
/// # Errors
///
/// [`RtError::TypeError`], если операнд не целое `0..=4294967295` либо сдвиг
/// не целое `0..=31`. Сдвиг на 32 — ошибка, а не ноль (измерено), поэтому
/// `<<` здесь заведомо в пределах разрядности и паниковать не может.
pub fn shift_left(a: &BslValue, s: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовыйСдвигВлево";
    let a = u32_arg(a, OP)?;
    let s = bit_index_arg(s, OP)?;
    Ok(number(a << s))
}

/// `ПобитовыйСдвигВправо(Число, Сдвиг)` — сдвиг ЛОГИЧЕСКИЙ: старшие разряды
/// заполняются нулями, потому что операнд беззнаковый.
///
/// # Errors
///
/// Те же, что у [`shift_left`].
pub fn shift_right(a: &BslValue, s: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПобитовыйСдвигВправо";
    let a = u32_arg(a, OP)?;
    let s = bit_index_arg(s, OP)?;
    Ok(number(a >> s))
}

/// `ПроверитьБит(Число, НомерБита)` -> `Булево`.
///
/// # Errors
///
/// [`RtError::TypeError`], если число не целое `0..=4294967295` либо номер
/// бита не целое `0..=31`.
pub fn check_bit(a: &BslValue, n: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПроверитьБит";
    let a = u32_arg(a, OP)?;
    let n = bit_index_arg(n, OP)?;
    Ok(BslValue::Boolean((a >> n) & 1 == 1))
}

/// `ПроверитьПоБитовойМаске(Число, Маска)` -> `Булево`.
///
/// ВСЕ биты маски, а не хотя бы один: измерено `(12, 4)` -> `Да`,
/// `(12, 5)` -> `Нет`. Пустая маска подходит всегда (`(12, 0)` -> `Да`).
///
/// # Errors
///
/// [`RtError::TypeError`], если число или маска не целое `0..=4294967295`.
pub fn check_by_bit_mask(a: &BslValue, m: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПроверитьПоБитовойМаске";
    let a = u32_arg(a, OP)?;
    let m = u32_arg(m, OP)?;
    Ok(BslValue::Boolean(a & m == m))
}

/// `УстановитьБит(Число, НомерБита, Значение)` -> число с изменённым битом.
///
/// # Errors
///
/// [`RtError::TypeError`], если число не целое `0..=4294967295`, номер бита
/// не целое `0..=31` либо значение не `Булево` (всё измерено).
pub fn set_bit(a: &BslValue, n: &BslValue, value: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "УстановитьБит";
    let a = u32_arg(a, OP)?;
    let n = bit_index_arg(n, OP)?;
    let value = bool_arg(value, OP)?;
    Ok(number(if value { a | (1 << n) } else { a & !(1 << n) }))
}

/// `ЧислоИзШестнадцатеричнойСтроки(Строка)`.
///
/// Приставка `0x` обязательна и пишется строчными; сами знаки регистра не
/// различают (всё измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка либо строка не той формы.
pub fn number_from_hex_string(v: &BslValue) -> RtResult<BslValue> {
    parse_radix(
        v,
        "0x",
        16,
        "ЧислоИзШестнадцатеричнойСтроки",
        "Строка вида 0x…, знаки 0-9A-F",
    )
}

/// `ЧислоИзДвоичнойСтроки(Строка)`.
///
/// Приставка `0b` обязательна и пишется строчными (измерено: `0B1010`
/// отвергается).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не строка либо строка не той формы.
pub fn number_from_binary_string(v: &BslValue) -> RtResult<BslValue> {
    parse_radix(
        v,
        "0b",
        2,
        "ЧислоИзДвоичнойСтроки",
        "Строка вида 0b…, знаки 0 и 1",
    )
}

/// Общий разбор строки с приставкой. Значение копится средствами
/// [`BslNumber`], а не в `u64`/`u128`: платформа принимает сорок знаков `F`
/// (2^160-1) и отдаёт их точно — измерено.
///
/// Ширина разбора ИЗМЕРЕНА (якорь `BIN.RADIX.WIDTH`): до двухсот знаков `F`
/// (2^800-1, 241 десятичный знак) платформа разбирает точно и сходится с
/// нами, а потолок у неё лежит между двумястами и пятью тысячами знаков. За
/// потолком она не ошибается и старших разрядов не теряет — она отдаёт
/// чужое число (на 5000 знаках: 309 знаков вместо точных 6021), не равное
/// ни точному значению, ни его началу, ни его хвосту.
///
/// Здесь это расхождение оставлено сознательно: принимается строка ЛЮБОЙ
/// длины, ограничение — только память. Разбор при этом O(n²) по длине,
/// потому что каждый знак умножает накопитель, а `BigInt` внутри
/// [`BslNumber`] растёт вместе со строкой. Копировать чужой потолок нечем:
/// какое именно число платформа отдаёт за ним, замер не выяснил.
fn parse_radix(
    v: &BslValue,
    prefix: &str,
    radix: u32,
    op: &'static str,
    expected: &'static str,
) -> RtResult<BslValue> {
    let bad = || RtError::TypeError { expected, op };
    let text = match v {
        BslValue::Str(s) => s.to_string(),
        _ => return Err(bad()),
    };
    // Приставка сравнивается как есть: `0X` и `0B` платформа отвергает.
    let digits = text.strip_prefix(prefix).ok_or_else(bad)?;
    if digits.is_empty() {
        return Err(bad());
    }
    let base = BslNumber::from_i64(i64::from(radix));
    let mut acc = BslNumber::from_i64(0);
    for ch in digits.chars() {
        let digit = ch.to_digit(radix).ok_or_else(bad)?;
        acc = acc
            .mul(&base)?
            .add(&BslNumber::from_i64(i64::from(digit)))?;
    }
    Ok(BslValue::Number(acc))
}

fn call_and(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    and(&args[0], &args[1])
}

fn call_or(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    or(&args[0], &args[1])
}

fn call_not(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    not(&args[0])
}

fn call_and_not(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    and_not(&args[0], &args[1])
}

fn call_xor(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    xor(&args[0], &args[1])
}

fn call_shift_left(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    shift_left(&args[0], &args[1])
}

fn call_shift_right(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    shift_right(&args[0], &args[1])
}

fn call_check_bit(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    check_bit(&args[0], &args[1])
}

fn call_check_by_bit_mask(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    check_by_bit_mask(&args[0], &args[1])
}

fn call_set_bit(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    set_bit(&args[0], &args[1], &args[2])
}

fn call_number_from_hex(_context: &mut CallContext<'_>, args: &[BslValue]) -> RtResult<BslValue> {
    number_from_hex_string(&args[0])
}

fn call_number_from_binary(
    _context: &mut CallContext<'_>,
    args: &[BslValue],
) -> RtResult<BslValue> {
    number_from_binary_string(&args[0])
}

fn argument(arguments: &[BslValue], index: usize) -> &BslValue {
    arguments.get(index).unwrap_or(&BslValue::Undefined)
}

/// Представление буфера временно живёт в `bsl-rt` (см. док рефакторинга:
/// `ДвоичныеДанные` ссылается на буферы, а зависимость `bsl-rt -> bsl-binbuf`
/// запрещена), но ТИП принадлежит этому компоненту — конструктор объявлен
/// здесь и лишь делегирует представлению.
fn construct_binary_buffer(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    BslValue::new_binary_buffer(argument(arguments, 0), argument(arguments, 1))
}

// Арность снята с платформы и повторяет прежнюю legacy-ветку `resolve_new`:
// размер обязателен, порядок байтов необязателен (по умолчанию LittleEndian).
const CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["БуферДвоичныхДанных", "BinaryDataBuffer"],
    arity: Arity::range(1, 2),
    call: construct_binary_buffer,
}];

const FUNCTIONS: &[FunctionDescriptor] = &[
    FunctionDescriptor {
        code: FunctionCode::new(1),
        names: &["ПобитовоеИ", "BitwiseAnd"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_and,
    },
    FunctionDescriptor {
        code: FunctionCode::new(2),
        names: &["ПобитовоеИли", "BitwiseOr"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_or,
    },
    FunctionDescriptor {
        code: FunctionCode::new(3),
        names: &["ПобитовоеНе", "BitwiseNot"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_not,
    },
    FunctionDescriptor {
        code: FunctionCode::new(4),
        names: &["ПобитовоеИНе", "BitwiseAndNot"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_and_not,
    },
    FunctionDescriptor {
        code: FunctionCode::new(5),
        names: &["ПобитовоеИсключительноеИли", "BitwiseXor"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_xor,
    },
    FunctionDescriptor {
        code: FunctionCode::new(6),
        names: &["ПобитовыйСдвигВлево", "BitwiseShiftLeft"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_shift_left,
    },
    FunctionDescriptor {
        code: FunctionCode::new(7),
        names: &["ПобитовыйСдвигВправо", "BitwiseShiftRight"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_shift_right,
    },
    FunctionDescriptor {
        code: FunctionCode::new(8),
        names: &["ПроверитьБит", "CheckBit"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_check_bit,
    },
    FunctionDescriptor {
        code: FunctionCode::new(9),
        names: &["ПроверитьПоБитовойМаске", "CheckByBitMask"],
        arity: Arity::exact(2),
        kind: FunctionKind::Function,
        call: call_check_by_bit_mask,
    },
    FunctionDescriptor {
        code: FunctionCode::new(10),
        names: &["УстановитьБит", "SetBit"],
        arity: Arity::exact(3),
        kind: FunctionKind::Function,
        call: call_set_bit,
    },
    FunctionDescriptor {
        code: FunctionCode::new(11),
        names: &["ЧислоИзШестнадцатеричнойСтроки", "NumberFromHexString"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_number_from_hex,
    },
    FunctionDescriptor {
        code: FunctionCode::new(12),
        names: &["ЧислоИзДвоичнойСтроки", "NumberFromBinaryString"],
        arity: Arity::exact(1),
        kind: FunctionKind::Function,
        call: call_number_from_binary,
    },
];

/// Дескриптор статически подключаемого компонента двоичных операций.
///
/// Первая стадия выноса содержит независимые глобальные функции. Объекты
/// двоичных данных и буфера остаются в переходном слое `bsl-rt`, пока
/// `bsl-stream`, PDF и XDTO не переведены на общий двоичный протокол.
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor {
        package: PACKAGE_NAME,
        version: PACKAGE_VERSION,
        // Ядро в зависимостях не объявляется: реестр включает его в
        // требования любой программы (`RuntimeRegistry::requirements_for`).
        dependencies: &[],
        functions: FUNCTIONS,
        constructors: CONSTRUCTORS,
        types: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_function_codes_are_static_and_dense() {
        let codes = library()
            .functions
            .iter()
            .map(|function| function.code.get())
            .collect::<Vec<_>>();
        assert_eq!(codes, (1..=12).collect::<Vec<_>>());
    }

    fn n(v: i64) -> BslValue {
        BslValue::Number(BslNumber::from_i64(v))
    }

    fn frac() -> BslValue {
        BslValue::Number(BslNumber::from_parts(15, 1))
    }

    fn s(v: &str) -> BslValue {
        BslValue::Str(bsl_rt::BslString::from_str(v))
    }

    fn num_of(v: &BslValue) -> String {
        match v {
            BslValue::Number(n) => n.to_string(),
            other => panic!("ожидалось число, получено {other:?}"),
        }
    }

    /// Опорная таблица из фикстуры: пять побитовых над краями диапазона.
    #[test]
    fn bitwise_matches_the_measured_table() {
        assert_eq!(num_of(&and(&n(12), &n(10)).unwrap()), "8");
        assert_eq!(num_of(&or(&n(12), &n(10)).unwrap()), "14");
        assert_eq!(num_of(&and_not(&n(12), &n(10)).unwrap()), "4");
        assert_eq!(num_of(&xor(&n(12), &n(10)).unwrap()), "6");
        assert_eq!(num_of(&not(&n(0)).unwrap()), "4294967295");
        assert_eq!(num_of(&not(&n(4294967295)).unwrap()), "0");
        assert_eq!(num_of(&not(&n(2147483648)).unwrap()), "2147483647");
    }

    /// Граница ровно на `2^32`, а не на единицу ниже: `4294967295`
    /// принимается, `4294967296` — нет (измерено).
    #[test]
    fn bitwise_and_rejects_an_operand_at_2_32() {
        assert!(and(&n(4294967295), &n(1)).is_ok());
        assert!(and(&n(4294967296), &n(1)).is_err());
        assert!(and(&n(-1), &n(1)).is_err());
        assert!(and(&frac(), &n(1)).is_err());
        assert!(and(&s("1"), &n(1)).is_err());
        assert!(and(&BslValue::Undefined, &n(1)).is_err());
    }

    /// Сдвиг на 32 — ОШИБКА, а не ноль. Ровно то место, где рассуждение
    /// разошлось бы с платформой.
    #[test]
    fn a_shift_of_32_is_an_error_not_zero() {
        assert_eq!(num_of(&shift_left(&n(1), &n(31)).unwrap()), "2147483648");
        assert!(shift_left(&n(1), &n(32)).is_err());
        assert!(shift_left(&n(1), &n(33)).is_err());
        assert!(shift_right(&n(1), &n(32)).is_err());
        assert!(shift_left(&n(1), &n(-1)).is_err());
    }

    /// Вытесненные влево биты теряются молча.
    #[test]
    fn shifting_left_drops_the_high_bits() {
        assert_eq!(num_of(&shift_left(&n(2147483648), &n(1)).unwrap()), "0");
        assert_eq!(num_of(&shift_left(&n(3), &n(31)).unwrap()), "2147483648");
        assert_eq!(num_of(&shift_right(&n(2147483648), &n(31)).unwrap()), "1");
    }

    #[test]
    fn check_bit_uses_indices_0_to_31() {
        assert_eq!(check_bit(&n(1), &n(0)).unwrap(), BslValue::Boolean(true));
        assert_eq!(check_bit(&n(2), &n(0)).unwrap(), BslValue::Boolean(false));
        assert_eq!(
            check_bit(&n(2147483648), &n(31)).unwrap(),
            BslValue::Boolean(true)
        );
        assert!(check_bit(&n(1), &n(32)).is_err());
        assert!(check_bit(&n(1), &n(-1)).is_err());
    }

    /// Маска — это ВСЕ её биты, а не хотя бы один.
    #[test]
    fn the_mask_requires_every_bit_not_just_one() {
        assert_eq!(
            check_by_bit_mask(&n(12), &n(4)).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            check_by_bit_mask(&n(12), &n(5)).unwrap(),
            BslValue::Boolean(false)
        );
        assert_eq!(
            check_by_bit_mask(&n(12), &n(0)).unwrap(),
            BslValue::Boolean(true)
        );
        assert_eq!(
            check_by_bit_mask(&n(0), &n(1)).unwrap(),
            BslValue::Boolean(false)
        );
    }

    /// Третий аргумент — строго `Булево`.
    #[test]
    fn set_bit_takes_a_boolean_only() {
        assert_eq!(
            num_of(&set_bit(&n(0), &n(0), &BslValue::Boolean(true)).unwrap()),
            "1"
        );
        assert_eq!(
            num_of(&set_bit(&n(1), &n(0), &BslValue::Boolean(false)).unwrap()),
            "0"
        );
        assert_eq!(
            num_of(&set_bit(&n(0), &n(31), &BslValue::Boolean(true)).unwrap()),
            "2147483648"
        );
        assert!(set_bit(&n(0), &n(0), &n(1)).is_err());
        assert!(set_bit(&n(0), &n(0), &s("Истина")).is_err());
        assert!(set_bit(&n(0), &n(32), &BslValue::Boolean(true)).is_err());
    }

    /// Приставка обязательна и регистрозависима.
    #[test]
    fn the_radix_prefix_is_required_and_lowercase() {
        assert_eq!(num_of(&number_from_hex_string(&s("0xFF")).unwrap()), "255");
        assert_eq!(num_of(&number_from_hex_string(&s("0xff")).unwrap()), "255");
        assert_eq!(num_of(&number_from_hex_string(&s("0x0")).unwrap()), "0");
        assert!(number_from_hex_string(&s("0XFF")).is_err());
        assert!(number_from_hex_string(&s("FF")).is_err());
        assert!(number_from_hex_string(&s("0x")).is_err());
        assert!(number_from_hex_string(&s("")).is_err());
        assert!(number_from_hex_string(&s(" 0xFF ")).is_err());
        assert!(number_from_hex_string(&s("0xGG")).is_err());
        assert!(number_from_hex_string(&s("-0x1")).is_err());
        assert!(number_from_hex_string(&n(255)).is_err());

        assert_eq!(
            num_of(&number_from_binary_string(&s("0b1010")).unwrap()),
            "10"
        );
        assert!(number_from_binary_string(&s("0B1010")).is_err());
        assert!(number_from_binary_string(&s("1010")).is_err());
        assert!(number_from_binary_string(&s("0b2")).is_err());
        assert!(number_from_binary_string(&s("0b")).is_err());
    }

    /// Разбор шире не только 32, но и 128 бит: сорок знаков `F` — это
    /// 2^160-1, и платформа отдаёт их точно.
    #[test]
    fn parsing_is_not_capped_at_any_machine_width() {
        assert_eq!(
            num_of(&number_from_hex_string(&s("0xFFFFFFFFFFFFFFFF")).unwrap()),
            "18446744073709551615"
        );
        assert_eq!(
            num_of(&number_from_hex_string(&s("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")).unwrap()),
            "340282366920938463463374607431768211455"
        );
        assert_eq!(
            num_of(
                &number_from_hex_string(&s("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")).unwrap()
            ),
            "1461501637330902918203684832716283019655932542975"
        );
        let ones = format!("0b{}", "1".repeat(64));
        assert_eq!(
            num_of(&number_from_binary_string(&s(&ones)).unwrap()),
            "18446744073709551615"
        );
    }

    /// Двести знаков `F` — 2^800-1, 241 десятичный знак. Это ИЗМЕРЕННОЕ
    /// значение, а не выбранное: платформа на том же зонде отвечает
    /// `200=241` (якорь `BIN.RADIX.WIDTH`), то есть разбирает двести знаков
    /// точно, как и мы. Ждём само число, а не только его длину: молчаливая
    /// потеря старших разрядов иначе неотличима от разбора.
    ///
    /// Верхний край платформы этот тест не воспроизводит и воспроизводить не
    /// должен — за своим потолком она отдаёт число, не выведенное из
    /// точного значения (см. `parse_radix` и якорь).
    #[test]
    fn parse_radix_matches_the_platform_at_two_hundred_digits() {
        let wide = format!("0x{}", "F".repeat(200));
        let text = num_of(&number_from_hex_string(&s(&wide)).unwrap());
        assert_eq!(text.len(), 241);
        assert_eq!(
            text,
            "666801443287985427407985179072125779714475832231590816039625781176403723781763207152143\
             220087155429074292991059343324044588880165411936508036335605233083004609515757951401455\
             8463078285911814024728965016135886601981690748037476461291163877375"
        );
        // Двоичная сторона того же разбора: 800 единиц — то же число.
        let ones = format!("0b{}", "1".repeat(800));
        assert_eq!(num_of(&number_from_binary_string(&s(&ones)).unwrap()), text);
    }
}
