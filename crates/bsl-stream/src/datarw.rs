//! `ЧтениеДанных`, `ЗаписьДанных` и `РезультатЧтенияДанных`.
//!
//! Оба типа — одна и та же надстройка над источником байтов, поэтому
//! состояние у них общее ([`DataRwState`]), а различает их вариант
//! [`Side`]: `DataReader` и `DataWriter` — РАЗНЫЕ типы платформы
//! (`Тип("ЧтениеДанных") = Тип("ЗаписьДанных")` даёт «Нет» — измерено).
//!
//! # Что измерено на 8.3.27.2074
//!
//! **СОБСТВЕННОГО БУФЕРА НЕТ.** Название «буферизованные читатель и
//! писатель» не описывает платформу: упреждающего чтения нет ни над
//! `ПотокВПамяти`, ни над `ФайловыйПоток`. Сразу после
//! `Новый ЧтениеДанных(Поток)` позиция потока 0, после первого
//! `ПрочитатьБайт()` — 1, после второго — 2, и то же самое на файле в 70 000
//! байт. У писателя симметрично: `ЗаписатьБайт(65)` немедленно даёт
//! `Поток.Размер() = 1`, сбрасывать нечего — метода `ОчиститьБуфер` у
//! платформы НЕТ ни в каком написании (проверено `ОчиститьБуфер`,
//! `ОчиститьБуферы`, `Сбросить`, `СброситьБуфер`). Отсюда и устройство:
//! читатель и писатель держат ЛОГИЧЕСКУЮ позицию, которая обязана совпадать
//! с фактической позицией целевого потока, а не собственный `Vec<u8>`.
//!
//! **Чередование ловится сверкой позиций и НЕ залипает.** Прямой
//! `Поток.Перейти(...)` между двумя операциями читателя даёт исключение с
//! текстом (снят дословно):
//!
//! > В процессе работы с объектом 'ЧтениеДанных' произошло изменение позиции
//! > нижележащего потока извне. Это может привести к некорректной работе
//! > приложения.
//!
//! Измерены и все три соседних случая: `Перейти` НА ТУ ЖЕ позицию ошибки не
//! даёт (ни до первого чтения, ни после), а возврат позиции обратно на
//! ожидаемую ЛЕЧИТ читателя — следующая операция снова работает. Значит это
//! не «сломанный навсегда объект», а сверка на каждой операции, и стоять она
//! обязана ДО побочного эффекта: отвергнутая `ЗаписатьЦелое32` размера
//! целевого потока не поменяла (измерено).
//!
//! **`ПорядокБайтов` у читателя — свойство-обманка.** У писателя оно живое:
//! `Зп.ПорядокБайтов = BigEndian` меняет байты следующего
//! `ЗаписатьЦелое16(258)` с `2 1` на `1 2`. У ЧИТАТЕЛЯ присваивание меняет
//! только то, что отдаёт геттер: после `Чт.ПорядокБайтов = BigEndian` над
//! байтами `65 66` геттер говорит «Big-endian», а `ПрочитатьЦелое16()`
//! по-прежнему отдаёт 16961 (little-endian). Тот же BigEndian, заданный
//! ТРЕТЬИМ АРГУМЕНТОМ КОНСТРУКТОРА, читается: 16706. Отсюда два поля порядка
//! у читателя ([`DataRwState::order_shown`] и [`DataRwState::order_used`]).
//! `КодировкаТекста` и `РазделительСтрок`, наоборот, живые у обоих.
//!
//! **Перевод строки при записи.** Каждый `\n` в записываемом ТЕКСТЕ
//! превращается в `\r\n` — и в теле, и в разделителе: `ЗаписатьСимволы("A\nB")`
//! кладёт `65 13 10 66`, `ЗаписатьСимволы("A\r\nB")` — `65 13 13 10 66`
//! (то есть `\r` не поглощается, а именно каждый `\n` разворачивается).
//! Разделитель по умолчанию — один `\n`, поэтому `ЗаписатьСтроку("AB")`
//! кладёт `65 66 13 10`, а геттер `РазделительСтрок` показывает при этом
//! строку длины 1. При ЧТЕНИИ обратного перевода нет: разделитель ищется
//! побайтово как есть, и `ПрочитатьСтроку()` при разделителе `\n` над
//! `65 66 13 10` отдаёт `"AB\r"` — три символа (измерено).
//!
//! **Сигнатуры (BOM) в поток не пишется.** «Аб» в UTF-8 — ровно четыре байта
//! `208 144 208 177`, без `EF BB BF`; в windows-1251 — `192 225`
//! (`binary-datarw.platform.txt`, строки «байты ЗаписатьСимволы(Аб)» и
//! «…(Аб, windows-1251)»).
//!
//! **Край источника — `Неопределено`, а не ошибка и не усечение.**
//! `ПрочитатьБайт()` на исчерпанном источнике отдаёт `Неопределено`, и так же
//! `ПрочитатьЦелое32()`, когда до конца осталось три байта. А вот
//! `Прочитать(N)`, `ПрочитатьВБуферДвоичныхДанных(N)` и `ПрочитатьСимволы(N)`
//! отдают СКОЛЬКО ЕСТЬ: `Прочитать(100)` над шестнадцатью байтами даёт
//! результат размера 16, на пустом источнике — размера 0. Ноль и пропущенный
//! аргумент у всех трёх значат «сколько есть» целиком.
//!
//! **Огромное количество: `Прочитать` принимает, два других отвергают.**
//! Измерено 08.08.2026 на 8.3.27.2074 разведкой над шестнадцатибайтовым
//! ФАЙЛОМ, канарейка `Прочитать(100)` = 16 в том же прогоне:
//!
//! * `Прочитать(35184372088832)` (2^45) — принято, размер 16, то есть та же
//!   «сколько есть». Здесь так и сделано, и файловый путь читает порциями
//!   именно поэтому (см. `read_limited` ниже).
//! * `ПрочитатьВБуферДвоичныхДанных(N)` и `ПрочитатьСимволы(N)` при
//!   `N >= 2^32` — ОШИБКА «Недопустимое значение числа» (проверено на 2^32,
//!   2^33 и 2^45). Граница ровно на `2^32`, и здесь она сделана проверкой
//!   АРГУМЕНТА ([`bounded_limit_arg`]). Текст ошибки у платформы свой и здесь
//!   не воспроизводится, поэтому фикстура печатает всякую неудачу как
//!   «<ошибка>».
//!
//! Прогоном самой фикстуры измерены ещё две стороны той же границы, и обе
//! совпали с тем, что здесь сделано (`binary-datarw.platform.txt`, строки
//! «память: …(2^32)» и «файл: …(2^32)»): граница ОДИНАКОВА над `ПотокВПамяти`
//! и над файлом, то есть от носителя не зависит; а отказ идёт ДО побочного
//! эффекта — следующий `ПрочитатьБайт` тем же читателем отдаёт байт нулевой
//! позиции (65 над памятью, 208 над файлом), значит позицию отвергнутая проба
//! не двигает. `Прочитать(2^32)` в том же прогоне принят обоими носителями.
//!
//! Замером НЕ закрыто вот что: при `N = 2^32 - 1` платформа количество
//! принимает, но за 180 с не отвечает и сеанс снимается по таймауту, так что
//! чем она отвечает НИЖЕ границы на количестве больше носителя — неизвестно.
//! Здесь такое количество принимается и отдаёт «сколько есть», как у
//! `Прочитать`. Пробу на `2^32 - 1` повторять не стоит: она стоит целого
//! прогона и заведомо кончается таймаутом, поэтому в фикстуре её нет и быть
//! не должно, а живёт она только в юнит-тесте
//! `the_bound_is_exactly_at_2_32_not_below`. Тройка меток неизмеренного
//! поведения этому не положена: метка — для решения, принятого рассуждением
//! ВМЕСТО замера, а здесь замер проведён и на границе ответил, под границей же
//! платформа не отвечает вовсе. Тот же случай так же решён у края `u64` при
//! записи в `crate::stream`.
//!
//! # Чего у платформы НЕТ
//!
//! Проверено вызовом, каждое имя отвечает «Метод объекта не обнаружен»:
//! `ЕстьДанныеДляЧтения`, `ПропуститьДоРазделителя`, `ПрочитатьВДвоичныеДанные`,
//! `ПрочитатьВСтроку`, `ПолучитьЦелевойПоток`, `ЗакрытьИПолучитьЦелевойПоток`,
//! `ЗаписатьСтрокуСРазделителем`, `ОчиститьБуфер`. У `РезультатЧтенияДанных`
//! нет полей `ДостигнутКонец`, `ЗавершениеПоРазделителю`, `ЕстьДанные`,
//! `ДостигнутКонецФайла`, `Данные`, а `Размер` у него — СВОЙСТВО, не метод.
//! Ничего из этого здесь не выдумано: отсутствующий метод лучше угаданного.

use std::cell::RefCell;
use std::rc::Rc;

use bsl_rt::{
    encoding::Encoding, BslNumber, BslString, BslValue, BuiltinMethod, ByteStreamProtocol,
    CallContext, EnumValue, ObjectProtocol, RtError, RtResult, TypeDescriptor, TypeId,
};

/// Порядок байтов многобайтового целого.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn to_enum(self) -> EnumValue {
        match self {
            Self::Little => EnumValue::ByteOrderLittle,
            Self::Big => EnumValue::ByteOrderBig,
        }
    }

    fn from_enum(value: EnumValue) -> Option<Self> {
        match value {
            EnumValue::ByteOrderLittle => Some(Self::Little),
            EnumValue::ByteOrderBig => Some(Self::Big),
            _ => None,
        }
    }
}

/// Ширина беззнакового целого в байтах.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntWidth {
    W16,
    W32,
    W64,
}

impl IntWidth {
    fn bytes(self) -> usize {
        match self {
            Self::W16 => 2,
            Self::W32 => 4,
            Self::W64 => 8,
        }
    }

    fn max(self) -> u64 {
        match self {
            Self::W16 => u16::MAX as u64,
            Self::W32 => u32::MAX as u64,
            Self::W64 => u64::MAX,
        }
    }

    fn read_op(self) -> &'static str {
        match self {
            Self::W16 => "ПрочитатьЦелое16",
            Self::W32 => "ПрочитатьЦелое32",
            Self::W64 => "ПрочитатьЦелое64",
        }
    }

    fn write_op(self) -> &'static str {
        match self {
            Self::W16 => "ЗаписатьЦелое16",
            Self::W32 => "ЗаписатьЦелое32",
            Self::W64 => "ЗаписатьЦелое64",
        }
    }

    fn decode(self, bytes: &[u8], order: ByteOrder) -> u64 {
        let mut result = 0u64;
        for (index, byte) in bytes.iter().enumerate() {
            match order {
                ByteOrder::Little => result |= u64::from(*byte) << (8 * index),
                ByteOrder::Big => result = (result << 8) | u64::from(*byte),
            }
        }
        result
    }

    fn encode(self, value: u64, order: ByteOrder) -> Vec<u8> {
        (0..self.bytes())
            .map(|index| {
                let shift = match order {
                    ByteOrder::Little => 8 * index,
                    ByteOrder::Big => 8 * (self.bytes() - 1 - index),
                };
                (value >> shift) as u8
            })
            .collect()
    }
}

/// Состояние читателя и писателя — общее для обоих типов.
#[derive(Debug)]
pub struct DataRwState {
    /// Целевой поток. Им же представлены и источники, у которых своего
    /// потока нет: `ДвоичныеДанные` копируются в поток над памятью, а имя
    /// файла открывается файловым потоком, который наружу не отдаётся.
    /// Сверка позиций для таких источников безвредна — подвинуть их извне
    /// всё равно некому.
    target: BslValue,
    /// Логическая позиция. Она же — ОЖИДАЕМАЯ фактическая позиция целевого
    /// потока: расхождение и есть чередование.
    pos: u64,
    /// Кодировка, которой пишется и читается текст. Живое свойство у обоих.
    encoding: Encoding,
    /// Имя кодировки КАК ЗАДАНО — геттер `КодировкаТекста` отдаёт именно
    /// его («windows-1251» после присваивания, «UTF-8» по умолчанию).
    encoding_name: String,
    /// То, что отдаёт геттер `ПорядокБайтов`.
    order_shown: ByteOrder,
    /// То, чем на самом деле читаются целые. У писателя всегда равен
    /// `order_shown`, у читателя — застывает на значении из конструктора
    /// (см. про свойство-обманку в шапке модуля).
    order_used: ByteOrder,
    /// `РазделительСтрок`.
    separator: String,
}

impl DataRwState {
    /// Сверка ожидаемой позиции с фактической и, при совпадении, доступ к
    /// целевому потоку. Стоит ПЕРВОЙ в каждой операции: отвергнутая операция
    /// не имеет права ничего записать.
    ///
    /// Сверка идёт над любым носителем: устройство одно, а для источников,
    /// у которых своего потока нет (`ДвоичныеДанные`, имя файла), она просто
    /// всегда сходится — двигать такой поток извне некому, наружу он не
    /// отдаётся.
    fn check_not_moved(&self, who: &'static str) -> RtResult<()> {
        // Заимствование короткое и не пересекается с рабочим: `position`
        // только читает поле.
        let actual = self.stream(who)?.position(who)?;
        if actual == self.pos {
            return Ok(());
        }
        Err(RtError::IoError(format!(
            "В процессе работы с объектом '{who}' произошло изменение позиции \
             нижележащего потока извне. Это может привести к некорректной работе \
             приложения."
        )))
    }

    /// Прочитать не больше `count` байтов, сдвинув позицию на фактически
    /// прочитанное. У конца отдаёт сколько есть.
    fn read_bytes(&mut self, count: usize, op: &'static str) -> RtResult<Vec<u8>> {
        let chunk = self.stream(op)?.read_bytes(count, op)?;
        // Переполниться не может: длина прочитанного ограничена длиной
        // носителя, а он живёт в памяти этого же процесса.
        self.pos += chunk.len() as u64;
        Ok(chunk)
    }

    /// Записать байты, сдвинув позицию.
    fn write_bytes(&mut self, chunk: &[u8], op: &'static str) -> RtResult<()> {
        self.stream(op)?.write_bytes(chunk, op)?;
        self.pos = self
            .pos
            .checked_add(chunk.len() as u64)
            .ok_or(RtError::TypeError {
                expected: "Позиция, умещающаяся в потоке",
                op,
            })?;
        Ok(())
    }

    /// Поставить позицию и себе, и целевому потоку — так, чтобы сверка
    /// снова сходилась. Нужно `Пропустить` (оно ПЕРЕВОДИТ позицию) и откату
    /// неудавшегося чтения целого.
    fn move_to(&mut self, pos: u64, op: &'static str) -> RtResult<()> {
        self.stream(op)?.set_position(pos, op)?;
        self.pos = pos;
        Ok(())
    }

    /// Изменяемое заимствование целевого потока.
    fn stream(&self, op: &'static str) -> RtResult<&dyn ByteStreamProtocol> {
        self.target.byte_stream().ok_or(RtError::TypeError {
            expected: "Байтовый поток",
            op,
        })
    }
}

// --- доступ к объекту ---------------------------------------------------------

/// Кто получатель: от этого зависит и текст ошибки чередования, и живость
/// свойства `ПорядокБайтов`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Reader,
    Writer,
}

static DATA_READER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЧтениеДанных",
    legacy_type_id: Some(TypeId::DataReader),
};

static DATA_WRITER_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "ЗаписьДанных",
    legacy_type_id: Some(TypeId::DataWriter),
};

static DATA_READ_RESULT_TYPE: TypeDescriptor = TypeDescriptor {
    package: crate::PACKAGE_NAME,
    name: "РезультатЧтенияДанных",
    legacy_type_id: Some(TypeId::DataReadResult),
};

#[derive(Debug)]
struct DataRwObject {
    side: Side,
    state: Rc<RefCell<DataRwState>>,
}

#[derive(Debug)]
struct DataReadResult {
    bytes: Rc<[u8]>,
}

impl Side {
    /// Имя типа в том написании, в каком платформа ставит его в сообщение об
    /// ошибке чередования (измерено для обоих).
    fn who(self) -> &'static str {
        match self {
            Side::Reader => "ЧтениеДанных",
            Side::Writer => "ЗаписьДанных",
        }
    }
}

impl ObjectProtocol for DataRwObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        match self.side {
            Side::Reader => &DATA_READER_TYPE,
            Side::Writer => &DATA_WRITER_TYPE,
        }
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        let value = self.as_value();
        match DataRwProp::lookup(name) {
            Some(property) => get_prop(&value, property),
            None => Err(RtError::UnknownColumn(name.to_string())),
        }
    }

    fn set_property(
        &self,
        name: &str,
        value: BslValue,
        _context: &mut CallContext<'_>,
    ) -> RtResult<()> {
        let receiver = self.as_value();
        match DataRwProp::lookup(name) {
            Some(property) => set_prop(&receiver, property, &value),
            None => Err(RtError::UnknownColumn(name.to_string())),
        }
    }

    fn call_method(
        &self,
        name: &str,
        arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        let value = self.as_value();
        match BuiltinMethod::lookup(name) {
            Some(BuiltinMethod::ReadNext) => read(&value, arguments),
            Some(BuiltinMethod::Write) => {
                write(&value, arguments)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::Close) => {
                close(&value)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::SkipNode) => skip(&value, arguments),
            Some(BuiltinMethod::ReadInt16) => read_int(&value, arguments, IntWidth::W16),
            Some(BuiltinMethod::ReadInt32) => read_int(&value, arguments, IntWidth::W32),
            Some(BuiltinMethod::ReadInt64) => read_int(&value, arguments, IntWidth::W64),
            Some(BuiltinMethod::WriteInt16) => {
                write_int(&value, arguments, IntWidth::W16)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::WriteInt32) => {
                write_int(&value, arguments, IntWidth::W32)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::WriteInt64) => {
                write_int(&value, arguments, IntWidth::W64)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::DataReadByte) => read_byte(&value),
            Some(BuiltinMethod::DataReadIntoBuffer) => read_into_buffer(&value, arguments),
            Some(BuiltinMethod::DataReadChars) => read_chars(&value, arguments),
            Some(BuiltinMethod::DataReadLine) => read_line(&value, arguments),
            Some(BuiltinMethod::DataWriteByte) => {
                write_byte(&value, arguments)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::DataWriteChars) => {
                write_chars(&value, arguments)?;
                Ok(BslValue::Undefined)
            }
            Some(BuiltinMethod::DataWriteLine) => {
                write_line(&value, arguments)?;
                Ok(BslValue::Undefined)
            }
            _ => Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: self.type_descriptor().name,
            }),
        }
    }
}

impl DataRwObject {
    fn as_value(&self) -> BslValue {
        BslValue::new_object(Self {
            side: self.side,
            state: self.state.clone(),
        })
    }
}

impl ObjectProtocol for DataReadResult {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_READ_RESULT_TYPE
    }

    fn get_property(&self, name: &str, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
        if name.eq_ignore_ascii_case("Размер") || name.eq_ignore_ascii_case("Size") {
            Ok(from_u64(self.bytes.len() as u64))
        } else {
            Err(RtError::UnknownColumn(name.to_string()))
        }
    }

    fn call_method(
        &self,
        name: &str,
        _arguments: &[BslValue],
        _context: &mut CallContext<'_>,
    ) -> RtResult<BslValue> {
        match BuiltinMethod::lookup(name) {
            Some(BuiltinMethod::GetBinaryData) => Ok(BslValue::binary_data_of(self.bytes.to_vec())),
            Some(BuiltinMethod::GetBinaryDataBuffer) => Ok(buffer_of_bytes(self.bytes.to_vec())),
            _ => Err(RtError::UnknownMethod {
                method: name.to_string(),
                receiver: DATA_READ_RESULT_TYPE.name,
            }),
        }
    }
}

/// Внутренности читателя либо писателя вместе с тем, кто из них это.
fn data<'a>(v: &'a BslValue, op: &'static str) -> RtResult<(&'a Rc<RefCell<DataRwState>>, Side)> {
    let not_applicable = || RtError::MethodNotApplicable {
        method: op,
        receiver: v.type_name(),
    };
    v.object_ref()
        .and_then(|object| object.downcast_ref::<DataRwObject>())
        .map(|object| (&object.state, object.side))
        .ok_or_else(not_applicable)
}

/// Внутренности ЧИТАТЕЛЯ: методы чтения на писателе платформа не знает.
fn reader<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<RefCell<DataRwState>>> {
    match data(v, op)? {
        (d, Side::Reader) => Ok(d),
        (_, Side::Writer) => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        }),
    }
}

/// Внутренности ПИСАТЕЛЯ.
fn writer<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<RefCell<DataRwState>>> {
    match data(v, op)? {
        (d, Side::Writer) => Ok(d),
        (_, Side::Reader) => Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        }),
    }
}

/// Байты результата чтения.
#[cfg(test)]
fn result_bytes<'a>(v: &'a BslValue, op: &'static str) -> RtResult<&'a Rc<[u8]>> {
    v.object_ref()
        .and_then(|object| object.downcast_ref::<DataReadResult>())
        .map(|result| &result.bytes)
        .ok_or_else(|| RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        })
}

// --- разбор аргументов ---------------------------------------------------------

fn from_u64(v: u64) -> BslValue {
    BslValue::Number(BslNumber::from_parts(v as i128, 0))
}

/// Целое `0..=u64::MAX` из числа BSL — тот же путь, что у буфера и потока.
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

/// Количество байтов или символов: целое неотрицательное. Дробное и
/// отрицательное платформа отвергает — измерено на `Прочитать(2.5)`,
/// `Прочитать(-1)` и `Пропустить(-1)` («Недопустимое значение числа»), и
/// здесь никакого усечения нет, в отличие от смещения в буфере у потока.
fn count_arg(v: Option<&BslValue>, op: &'static str) -> RtResult<Option<u64>> {
    let bad = || RtError::TypeError {
        expected: "Целое неотрицательное число",
        op,
    };
    match v {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Number(n)) => Ok(Some(to_u64(n).ok_or_else(bad)?)),
        Some(_) => Err(bad()),
    }
}

/// «Сколько есть» — ноль и пропущенный аргумент значат одно и то же
/// (измерено: `Прочитать(0)` над шестнадцатью байтами даёт 16).
fn limit_arg(v: Option<&BslValue>, op: &'static str) -> RtResult<Option<u64>> {
    Ok(match count_arg(v, op)? {
        None | Some(0) => None,
        Some(n) => Some(n),
    })
}

/// То же количество, но с измеренной верхней границей `2^32`: у
/// `ПрочитатьВБуферДвоичныхДанных` и `ПрочитатьСимволы` платформа отвергает
/// всё, что не меньше `2^32` («Недопустимое значение числа» — проверено на
/// `2^32`, `2^33` и `2^45`), а `Прочитать` то же количество принимает и
/// отдаёт «сколько есть». Асимметрия измерена, а не выведена, поэтому
/// граница живёт в отдельном помощнике, а не внутри [`limit_arg`].
///
/// Проверка идёт ПОСЛЕ отображения `Some(0)` -> `None`: ноль по-прежнему
/// значит «сколько есть» и границей не задевается.
fn bounded_limit_arg(v: Option<&BslValue>, op: &'static str) -> RtResult<Option<u64>> {
    let limit = limit_arg(v, op)?;
    // Ровно `>=`: `2^32 - 1` платформа принимает, `2^32` уже отвергает.
    if limit.is_some_and(|n| n >= 1u64 << 32) {
        return Err(RtError::TypeError {
            expected: "Количество меньше 2^32",
            op,
        });
    }
    Ok(limit)
}

/// Порядок байтов из аргумента; `None` — брать собственный.
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

/// Кодировка из аргумента; `None` — брать собственную.
fn encoding_arg(arg: Option<&BslValue>, own: Encoding, op: &'static str) -> RtResult<Encoding> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(own),
        Some(BslValue::Str(s)) => Encoding::by_name(&s.to_string()),
        Some(_) => Err(RtError::TypeError {
            expected: "Строка с именем кодировки",
            op,
        }),
    }
}

/// Строка из аргумента; `None` — брать собственную.
fn str_arg(arg: Option<&BslValue>, op: &'static str) -> RtResult<Option<String>> {
    match arg {
        None | Some(BslValue::Undefined) => Ok(None),
        Some(BslValue::Str(s)) => Ok(Some(s.to_string())),
        Some(_) => Err(RtError::TypeError {
            expected: "Строка",
            op,
        }),
    }
}

/// Каждый `\n` разворачивается в `\r\n` — измерено и в теле текста, и в
/// разделителе, и `\r` перед ним при этом не поглощается.
fn expand_line_feeds(text: &str) -> String {
    text.replace('\n', "\r\n")
}

/// Лишние аргументы платформа отвергает («Слишком много фактических
/// параметров» — измерено на `ЗаписьДанных.Записать(Буфер, 0, 4)`). Резолвер
/// у этих методов арность не фиксирует (необязательные хвостовые аргументы),
/// поэтому верхнюю границу проверяет рантайм.
fn at_most(v: &BslValue, args: &[BslValue], max: usize, op: &'static str) -> RtResult<()> {
    if args.len() > max {
        return Err(RtError::MethodNotApplicable {
            method: op,
            receiver: v.type_name(),
        });
    }
    Ok(())
}

// --- конструкторы ------------------------------------------------------------------

/// Разбор источника и общих для обоих типов хвостовых аргументов
/// `(Кодировка, ПорядокБайтов, РазделительСтрок)`.
fn new_state(
    source: &BslValue,
    encoding: &BslValue,
    order: &BslValue,
    separator: &BslValue,
    side: Side,
    op: &'static str,
) -> RtResult<DataRwState> {
    // Набор допустимых источников у сторон РАЗНЫЙ, и это измерено:
    // `ДвоичныеДанные` читателю годятся, а писателю платформа отвечает
    // «Несоответствие типов (параметр номер '1')». Общее у них — поток и имя
    // файла; число, `БуферДвоичныхДанных` и `Неопределено` отвергают оба, а
    // пустой конструктор не находится вовсе.
    let expected = match side {
        Side::Reader => "Поток, ДвоичныеДанные либо имя файла строкой",
        Side::Writer => "Поток либо имя файла строкой",
    };
    let bad_source = || RtError::TypeError { expected, op };
    let target = match source {
        value if value.byte_stream().is_some() => value.clone(),
        value if side == Side::Reader && bsl_binbuf::binary_data_bytes(value).is_some() => {
            crate::stream::data_over_bytes(
                bsl_binbuf::binary_data_bytes(value)
                    .expect("проверено guard'ом")
                    .to_vec(),
            )
        }
        // Имя файла. У читателя измерено, что отсутствующий файл — ошибка
        // «Файл не найден», а существующий читается с первого байта; у
        // писателя — что отсутствующий файл появляется и получает ровно
        // записанное, а СУЩЕСТВУЮЩИЙ НЕ ОБРЕЗАЕТСЯ: измерено на
        // 8.3.27.2074, что после записи «ABCDEFGH», закрытия и повторного
        // `Новый ЗаписьДанных(тот же файл)` + `ЗаписатьБайт(90)` в файле
        // остаётся восемь байтов `90 66 67 68 69 70 71 72`. То есть позиция
        // ставится в 0 и запись ложится ПОВЕРХ — ровно поведение уже
        // измеренного соседа `ФайловыеПотоки.ОткрытьДляЗаписи`, а не
        // `РежимОткрытияФайла.Создать`.
        BslValue::Str(s) => {
            let path = s.to_string();
            match side {
                Side::Reader => crate::stream::data_over_file(
                    &path,
                    crate::stream::FileOpenMode::Open,
                    crate::stream::FileAccess::Read,
                )?,
                Side::Writer => crate::stream::data_over_file(
                    &path,
                    crate::stream::FileOpenMode::OpenOrCreate,
                    crate::stream::FileAccess::ReadWrite,
                )?,
            }
        }
        _ => return Err(bad_source()),
    };
    // Позиция стартует с ТЕКУЩЕЙ позиции потока, а не с нуля: конструктор её
    // не двигает (измерено — сразу после него поток стоит там же, где стоял).
    let pos = target
        .byte_stream()
        .expect("конструктор принимает только поток")
        .position(op)?;
    let encoding_name = match encoding {
        BslValue::Undefined => "UTF-8".to_string(),
        BslValue::Str(s) => s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка с именем кодировки",
                op,
            })
        }
    };
    // Имя кодировки проверяется ЛЕНИВО: конструктор с «нет-такой-кодировки»
    // платформа принимает, а падает уже `ПрочитатьСимволы` (измерено).
    // Поэтому здесь неизвестное имя не ошибка, а UTF-8 до первого
    // использования — разбор идёт в `encoding_of`.
    let encoding_now = Encoding::by_name(&encoding_name).unwrap_or(Encoding::Utf8);
    let order = match order {
        BslValue::Undefined => ByteOrder::Little,
        BslValue::Enum(e) => ByteOrder::from_enum(*e).ok_or(RtError::TypeError {
            expected: "ПорядокБайтов",
            op,
        })?,
        _ => {
            return Err(RtError::TypeError {
                expected: "ПорядокБайтов",
                op,
            })
        }
    };
    // Разделитель по умолчанию РАЗНЫЙ у сторон: у читателя пустой (и
    // `ПрочитатьСтроку` тогда отдаёт всё до конца), у писателя — один `\n`,
    // который при записи разворачивается в `\r\n`. Измерено обоими геттерами.
    let separator = match separator {
        BslValue::Undefined => match side {
            Side::Reader => String::new(),
            Side::Writer => "\n".to_string(),
        },
        BslValue::Str(s) => s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op,
            })
        }
    };
    Ok(DataRwState {
        target,
        pos,
        encoding: encoding_now,
        encoding_name,
        order_shown: order,
        order_used: order,
        separator,
    })
}

/// `Новый ЧтениеДанных(Источник[, КодировкаТекста][, ПорядокБайтов][, РазделительСтрок])`.
///
/// # Errors
///
/// [`RtError::TypeError`], если источник не поток, не `ДвоичныеДанные` и не
/// строка, а также при негодных типах хвостовых аргументов;
/// [`RtError::IoError`], если файл с таким именем не открылся.
pub fn new_data_reader(
    source: &BslValue,
    encoding: &BslValue,
    order: &BslValue,
    separator: &BslValue,
) -> RtResult<BslValue> {
    let state = new_state(
        source,
        encoding,
        order,
        separator,
        Side::Reader,
        "Новый ЧтениеДанных",
    )?;
    Ok(BslValue::new_object(DataRwObject {
        side: Side::Reader,
        state: Rc::new(RefCell::new(state)),
    }))
}

/// `Новый ЗаписьДанных(Приёмник[, КодировкаТекста][, ПорядокБайтов][, РазделительСтрок])`.
///
/// # Errors
///
/// Те же, что у [`new_data_reader`].
pub fn new_data_writer(
    target: &BslValue,
    encoding: &BslValue,
    order: &BslValue,
    separator: &BslValue,
) -> RtResult<BslValue> {
    let state = new_state(
        target,
        encoding,
        order,
        separator,
        Side::Writer,
        "Новый ЗаписьДанных",
    )?;
    Ok(BslValue::new_object(DataRwObject {
        side: Side::Writer,
        state: Rc::new(RefCell::new(state)),
    }))
}

/// Кодировка для этой операции: явный аргумент, иначе собственная. Имя,
/// заданное конструктором или свойством, разбирается ЗДЕСЬ — платформа тоже
/// откладывает жалобу на негодное имя до места использования (измерено).
fn encoding_of(d: &DataRwState, arg: Option<&BslValue>, op: &'static str) -> RtResult<Encoding> {
    match arg {
        None | Some(BslValue::Undefined) => Encoding::by_name(&d.encoding_name),
        other => encoding_arg(other, d.encoding, op),
    }
}

// --- свойства ------------------------------------------------------------------------

/// Какое свойство читают или пишут.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRwProp {
    ByteOrder,
    TextEncoding,
    LineSeparator,
}

impl DataRwProp {
    /// Имя свойства по написанию на любом из двух языков.
    pub fn lookup(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("ПорядокБайтов") || name.eq_ignore_ascii_case("ByteOrder")
        {
            Some(DataRwProp::ByteOrder)
        } else if name.eq_ignore_ascii_case("КодировкаТекста")
            || name.eq_ignore_ascii_case("TextEncoding")
        {
            Some(DataRwProp::TextEncoding)
        } else if name.eq_ignore_ascii_case("РазделительСтрок")
            // ИЗМЕРЕНО: английское имя именно `LineSplitter`; на
            // `LineSeparator` платформа отвечает «Поле объекта не обнаружено».
            || name.eq_ignore_ascii_case("LineSplitter")
        {
            Some(DataRwProp::LineSeparator)
        } else {
            None
        }
    }
}

/// Чтение свойства читателя или писателя.
///
/// # Errors
///
/// «Метод не применим», если получатель не читатель и не писатель.
pub fn get_prop(v: &BslValue, which: DataRwProp) -> RtResult<BslValue> {
    let (d, _) = data(v, "ПорядокБайтов")?;
    let d = d.borrow();
    Ok(match which {
        DataRwProp::ByteOrder => BslValue::Enum(d.order_shown.to_enum()),
        DataRwProp::TextEncoding => BslValue::Str(BslString::from_str(&d.encoding_name)),
        DataRwProp::LineSeparator => BslValue::Str(BslString::from_str(&d.separator)),
    })
}

/// Запись свойства.
///
/// `ПорядокБайтов` у ЧИТАТЕЛЯ меняет только геттер: чтение целых им не
/// управляется (измерено, см. шапку модуля). У писателя оно живое.
///
/// # Errors
///
/// [`RtError::TypeError`] при негодном типе значения; «метод не применим»,
/// если получатель не читатель и не писатель.
pub fn set_prop(v: &BslValue, which: DataRwProp, value: &BslValue) -> RtResult<()> {
    const OP: &str = "Присваивание свойству";
    let (d, side) = data(v, OP)?;
    let mut d = d.borrow_mut();
    match which {
        DataRwProp::ByteOrder => {
            let order = match value {
                BslValue::Enum(e) => ByteOrder::from_enum(*e),
                _ => None,
            }
            .ok_or(RtError::TypeError {
                expected: "ПорядокБайтов",
                op: OP,
            })?;
            d.order_shown = order;
            if side == Side::Writer {
                d.order_used = order;
            }
        }
        DataRwProp::TextEncoding => {
            let name = match value {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка с именем кодировки",
                        op: OP,
                    })
                }
            };
            // Как и в конструкторе, негодное имя откладывается до места
            // использования.
            if let Ok(enc) = Encoding::by_name(&name) {
                d.encoding = enc;
            }
            d.encoding_name = name;
        }
        DataRwProp::LineSeparator => {
            d.separator = match value {
                BslValue::Str(s) => s.to_string(),
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка",
                        op: OP,
                    })
                }
            };
        }
    }
    Ok(())
}

// --- чтение ----------------------------------------------------------------------------

/// Прочитать `limit` байтов, либо всё до конца, если предела нет.
///
/// Читается ОДИНАКОВО в обоих случаях — порциями, с выходом на первом пустом
/// куске. Асимметрия здесь была дефектом: количество приходит из
/// пользовательского текста и легко превышает носитель, а файловый носитель
/// выделяет приёмный буфер ровно на запрошенное, так что `Прочитать(2^45)`
/// над файлом убивало процесс отказом размещения вместо того, чтобы отдать
/// «сколько есть» — то самое, что уже измерено для `Прочитать(100)` над
/// шестнадцатью байтами и что путь над памятью делал и до правки.
///
/// Поблочное чтение обязано остаться и после того, как у двух других форм
/// появилась граница `2^32`: `Прочитать` количество больше носителя
/// по-прежнему ПРИНИМАЕТ (измерено), так что путь `Прочитать(2^45)` над
/// файлом сюда доходит и остаётся единственной защитой от того же отказа
/// размещения.
fn read_limited(d: &mut DataRwState, limit: Option<u64>, op: &'static str) -> RtResult<Vec<u8>> {
    /// Шаг чтения. Предел живёт в `u64` и в `usize` может не влезать вовсе,
    /// а порция влезает всегда.
    const CHUNK: u64 = 64 * 1024;
    let mut out = Vec::new();
    // Остаток предела; `None` — предела нет, читаем до конца носителя.
    let mut left = limit;
    loop {
        let want = left.map_or(CHUNK, |n| n.min(CHUNK));
        if want == 0 {
            return Ok(out);
        }
        // Сужение безопасно: `want` не больше `CHUNK`.
        let chunk = d.read_bytes(want as usize, op)?;
        if chunk.is_empty() {
            return Ok(out);
        }
        out.try_reserve(chunk.len())
            .map_err(|_| RtError::TypeError {
                expected: "Размер, который удаётся разместить в памяти",
                op,
            })?;
        out.extend_from_slice(&chunk);
        // Вычитание не уйдёт ниже нуля: прочитано не больше запрошенного.
        left = left.map(|n| n - chunk.len() as u64);
    }
}

/// `ЧтениеДанных.Прочитать([Количество])` -> `РезультатЧтенияДанных`.
///
/// Ноль и пропущенный аргумент значат «сколько есть» (измерено). Разделитель
/// строк этот метод НЕ учитывает: `Прочитать(100)` при разделителе `F` над
/// шестнадцатью байтами всё равно даёт 16.
///
/// # Errors
///
/// [`RtError::TypeError`] на дробном и отрицательном количестве;
/// [`RtError::IoError`], если позицию целевого потока подвинули извне.
pub fn read(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "Прочитать";
    let d = reader(v, OP)?;
    at_most(v, args, 1, OP)?;
    let mut d = d.borrow_mut();
    let limit = limit_arg(args.first(), OP)?;
    d.check_not_moved(Side::Reader.who())?;
    let bytes = read_limited(&mut d, limit, OP)?;
    Ok(BslValue::new_object(DataReadResult {
        bytes: bytes.into(),
    }))
}

/// `ЧтениеДанных.ПрочитатьВБуферДвоичныхДанных([Количество])` -> буфер.
///
/// Отдаёт буфер РОВНО по числу прочитанных байтов: на пустом источнике —
/// нулевого размера, а не `Неопределено` (измерено).
///
/// Количество здесь ОГРАНИЧЕНО сверху: `N >= 2^32` платформа отвергает, в
/// отличие от [`read`] (измерено — см. шапку модуля).
///
/// # Errors
///
/// Те же, что у [`read`], и [`RtError::TypeError`] на количестве `2^32` и
/// больше.
pub fn read_into_buffer(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "ПрочитатьВБуферДвоичныхДанных";
    let d = reader(v, OP)?;
    at_most(v, args, 1, OP)?;
    let mut d = d.borrow_mut();
    let limit = bounded_limit_arg(args.first(), OP)?;
    d.check_not_moved(Side::Reader.who())?;
    let bytes = read_limited(&mut d, limit, OP)?;
    Ok(buffer_of_bytes(bytes))
}

/// Буфер двоичных данных над готовыми байтами. Порядок у него собственный,
/// по умолчанию `LittleEndian`, как у `Новый БуферДвоичныхДанных`.
fn buffer_of_bytes(bytes: Vec<u8>) -> BslValue {
    bsl_binbuf::binary_buffer_of(bytes)
}

/// `ЧтениеДанных.ПрочитатьБайт()` -> число либо `Неопределено` на краю.
///
/// # Errors
///
/// Те же, что у [`read`].
pub fn read_byte(v: &BslValue) -> RtResult<BslValue> {
    const OP: &str = "ПрочитатьБайт";
    let d = reader(v, OP)?;
    let mut d = d.borrow_mut();
    d.check_not_moved(Side::Reader.who())?;
    let chunk = d.read_bytes(1, OP)?;
    Ok(match chunk.first() {
        Some(b) => from_u64(*b as u64),
        None => BslValue::Undefined,
    })
}

/// `ЧтениеДанных.ПрочитатьЦелое16/32/64([ПорядокБайтов])`.
///
/// Не хватило байтов до конца — `Неопределено`, и позиция при этом
/// ОТКАТЫВАЕТСЯ: измерено, что после неудачного `ПрочитатьЦелое32` при трёх
/// оставшихся байтах поток стоит там же, где стоял (13 из 16), а следующий
/// `ПрочитатьБайт` отдаёт тот самый байт 13-й позиции. Неполный остаток не
/// поглощается.
///
/// Порядок берётся из явного аргумента либо из того, что задал КОНСТРУКТОР —
/// присваивание свойству `ПорядокБайтов` на читателя не действует (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если аргумент не член `ПорядокБайтов`; ошибка
/// чередования, как у [`read`].
fn read_int(v: &BslValue, args: &[BslValue], w: IntWidth) -> RtResult<BslValue> {
    let op = w.read_op();
    let d = reader(v, op)?;
    at_most(v, args, 1, op)?;
    let mut d = d.borrow_mut();
    let order = order_arg(args.first(), d.order_used, op)?;
    d.check_not_moved(Side::Reader.who())?;
    let before = d.pos;
    let chunk = d.read_bytes(w.bytes(), op)?;
    if chunk.len() < w.bytes() {
        d.move_to(before, op)?;
        return Ok(BslValue::Undefined);
    }
    Ok(from_u64(w.decode(&chunk, order)))
}

/// `ЧтениеДанных.Пропустить(Количество)` -> то же количество.
///
/// Это ПЕРЕВОД позиции, а не вычитывание байтов: `Пропустить(100)` над
/// шестнадцатибайтовым потоком уводит его на позицию 100 и возвращает 100 —
/// не 16 (измерено). Уход за конец потоку разрешён и размера не меняет, так
/// что копировать здесь нечего.
///
/// # Errors
///
/// [`RtError::TypeError`] на дробном и отрицательном количестве (измерено:
/// `Пропустить(-1)` платформа отвергает); ошибка чередования, как у [`read`].
pub fn skip(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "Пропустить";
    let d = reader(v, OP)?;
    at_most(v, args, 1, OP)?;
    let mut d = d.borrow_mut();
    let count = count_arg(args.first(), OP)?.unwrap_or(0);
    d.check_not_moved(Side::Reader.who())?;
    let to = d.pos.checked_add(count).ok_or(RtError::TypeError {
        expected: "Позиция, умещающаяся в потоке",
        op: OP,
    })?;
    d.move_to(to, OP)?;
    Ok(from_u64(count))
}

/// `ЧтениеДанных.ПрочитатьСимволы([Количество][, КодировкаТекста])`.
///
/// Количество — в СИМВОЛАХ, а не в байтах: `ПрочитатьСимволы(2)` над «АБВГ»
/// в UTF-8 отдаёт «АБ», то есть съедает четыре байта (измерено). Ноль и
/// пропущенный аргумент значат «сколько есть», а `N >= 2^32` отвергается —
/// та же граница, что у [`read_into_buffer`], и её тоже нет у [`read`].
///
/// # Errors
///
/// [`RtError::TextDoc`] на неизвестном имени кодировки (платформа жалуется
/// именно здесь, а не в конструкторе — измерено); [`RtError::TypeError`] на
/// количестве `2^32` и больше; ошибка чередования, как у [`read`].
pub fn read_chars(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "ПрочитатьСимволы";
    let d = reader(v, OP)?;
    at_most(v, args, 2, OP)?;
    let mut d = d.borrow_mut();
    let limit = bounded_limit_arg(args.first(), OP)?;
    let enc = encoding_of(&d, args.get(1), OP)?;
    d.check_not_moved(Side::Reader.who())?;
    let bytes = match limit {
        None => read_limited(&mut d, None, OP)?,
        Some(chars) => read_exactly_chars(&mut d, enc, chars, OP)?,
    };
    Ok(BslValue::Str(BslString::from_str(
        &enc.decode_without_bom(&bytes),
    )))
}

/// Снять байты ровно под `chars` символов в кодировке `enc`.
///
/// Читать приходится по одному «кванту» кодировки, а не с запасом: у
/// платформы упреждающего чтения нет, и позиция целевого потока обязана
/// остаться ровно там, где остановился разбор.
fn read_exactly_chars(
    d: &mut DataRwState,
    enc: Encoding,
    chars: u64,
    op: &'static str,
) -> RtResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut left = chars;
    while left > 0 {
        let lead = d.read_bytes(1, op)?;
        let Some(&lead) = lead.first() else {
            break;
        };
        out.push(lead);
        // Сколько ЕЩЁ байтов держит этот символ и сколько единиц UTF-16 он
        // займёт: 1С считает «символы» так же, как `СтрДлина`, то есть в
        // единицах UTF-16.
        let (more, units) = match enc {
            Encoding::Utf8 => match lead {
                0x00..=0x7F => (0, 1),
                0xC0..=0xDF => (1, 1),
                0xE0..=0xEF => (2, 1),
                0xF0..=0xF7 => (3, 2),
                // Продолжающий или негодный байт сам по себе — один символ
                // замещения; иначе разбор зациклился бы.
                _ => (0, 1),
            },
            Encoding::Utf16Le | Encoding::Utf16Be => (1, 1),
            Encoding::Utf32Le | Encoding::Utf32Be => (3, 1),
            Encoding::Windows1251 | Encoding::Cp866 | Encoding::Koi8R => (0, 1),
        };
        if more > 0 {
            let rest = d.read_bytes(more, op)?;
            let short = rest.len() < more;
            out.extend_from_slice(&rest);
            if short {
                break;
            }
        }
        left = left.saturating_sub(units);
    }
    Ok(out)
}

/// `ЧтениеДанных.ПрочитатьСтроку([КодировкаТекста])`.
///
/// Читает до РАЗДЕЛИТЕЛЯ, сам разделитель поглощает и в результат не
/// включает; пустой разделитель (умолчание читателя) означает «до конца».
/// Единственный аргумент — КОДИРОВКА, а не разделитель: измерено, что
/// `ПрочитатьСтроку("F")` платформа встречает жалобой «Неправильное имя
/// кодировки 'F'».
///
/// Обратного перевода `\r\n` в `\n` при чтении НЕТ: над байтами `65 66 13 10`
/// при разделителе `\n` результат — `"AB\r"`, три символа (измерено).
///
/// # Errors
///
/// Те же, что у [`read_chars`].
pub fn read_line(v: &BslValue, args: &[BslValue]) -> RtResult<BslValue> {
    const OP: &str = "ПрочитатьСтроку";
    let d = reader(v, OP)?;
    at_most(v, args, 1, OP)?;
    let mut d = d.borrow_mut();
    let enc = encoding_of(&d, args.first(), OP)?;
    d.check_not_moved(Side::Reader.who())?;
    // Разделитель ищется ПОБАЙТОВО, в той же кодировке, и без разворачивания
    // `\n` в `\r\n` — этот перевод существует только на записи (измерено).
    let sep = enc.encode_without_signature(&d.separator);
    if sep.is_empty() {
        let bytes = read_limited(&mut d, None, OP)?;
        return Ok(BslValue::Str(BslString::from_str(
            &enc.decode_without_bom(&bytes),
        )));
    }
    let mut out: Vec<u8> = Vec::new();
    loop {
        let chunk = d.read_bytes(1, OP)?;
        let Some(&b) = chunk.first() else {
            break;
        };
        out.push(b);
        if out.ends_with(&sep) {
            out.truncate(out.len() - sep.len());
            break;
        }
    }
    Ok(BslValue::Str(BslString::from_str(
        &enc.decode_without_bom(&out),
    )))
}

// --- запись ------------------------------------------------------------------------------

/// `ЗаписьДанных.Записать(ДвоичныеДанные)`.
///
/// Аргумент — именно `ДвоичныеДанные`: буфер, поток и сам читатель платформа
/// отвергает «Несоответствие типов», а трёхаргументную форму
/// `Записать(Буфер, 0, 4)` — «Слишком много фактических параметров»
/// (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`] на любом другом аргументе; [`RtError::IoError`],
/// если позицию целевого потока подвинули извне.
pub fn write(v: &BslValue, args: &[BslValue]) -> RtResult<()> {
    const OP: &str = "Записать";
    let d = writer(v, OP)?;
    let mut d = d.borrow_mut();
    let [data] = args else {
        return Err(RtError::MethodNotApplicable {
            method: OP,
            receiver: v.type_name(),
        });
    };
    let bytes = bsl_binbuf::binary_data_bytes(data).ok_or(RtError::TypeError {
        expected: "ДвоичныеДанные",
        op: OP,
    })?;
    d.check_not_moved(Side::Writer.who())?;
    d.write_bytes(bytes, OP)
}

/// `ЗаписьДанных.ЗаписатьБайт(Значение)`.
///
/// # Errors
///
/// [`RtError::TypeError`], если значение не целое из `0..=255` (измерено:
/// платформа отвергает и `256`, и `-1`); ошибка чередования, как у [`write`].
pub fn write_byte(v: &BslValue, args: &[BslValue]) -> RtResult<()> {
    const OP: &str = "ЗаписатьБайт";
    let d = writer(v, OP)?;
    at_most(v, args, 1, OP)?;
    let mut d = d.borrow_mut();
    let bad = || RtError::TypeError {
        expected: "Целое число от 0 до 255",
        op: OP,
    };
    let byte = match count_arg(args.first(), OP) {
        Ok(Some(n)) if n <= 255 => n as u8,
        _ => return Err(bad()),
    };
    d.check_not_moved(Side::Writer.who())?;
    d.write_bytes(&[byte], OP)
}

/// `ЗаписьДанных.ЗаписатьЦелое16/32/64(Значение[, ПорядокБайтов])`.
///
/// Порядок — из явного аргумента, иначе собственный; у писателя свойство
/// `ПорядокБайтов` ЖИВОЕ, в отличие от читателя (измерено).
///
/// # Errors
///
/// [`RtError::TypeError`], если значение не целое из `0..=2^N-1` (измерено:
/// `65536` и `-1` для шестнадцати бит платформа отвергает); ошибка
/// чередования, как у [`write`].
fn write_int(v: &BslValue, args: &[BslValue], w: IntWidth) -> RtResult<()> {
    let op = w.write_op();
    let d = writer(v, op)?;
    at_most(v, args, 2, op)?;
    let mut d = d.borrow_mut();
    let bad = || RtError::TypeError {
        expected: "Целое неотрицательное число, влезающее в целое этой ширины",
        op,
    };
    let value = match count_arg(args.first(), op) {
        Ok(Some(n)) if n <= w.max() => n,
        _ => return Err(bad()),
    };
    let order = order_arg(args.get(1), d.order_used, op)?;
    d.check_not_moved(Side::Writer.who())?;
    let bytes = w.encode(value, order);
    d.write_bytes(&bytes, op)
}

/// `ЗаписьДанных.ЗаписатьСимволы(Строка[, КодировкаТекста])` — текст БЕЗ
/// разделителя и без сигнатуры.
///
/// # Errors
///
/// [`RtError::TypeError`], если первый аргумент не строка;
/// [`RtError::TextDoc`] на неизвестном имени кодировки; ошибка чередования,
/// как у [`write`].
pub fn write_chars(v: &BslValue, args: &[BslValue]) -> RtResult<()> {
    const OP: &str = "ЗаписатьСимволы";
    let d = writer(v, OP)?;
    at_most(v, args, 2, OP)?;
    let mut d = d.borrow_mut();
    let text = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: OP,
            })
        }
    };
    let enc = encoding_of(&d, args.get(1), OP)?;
    d.check_not_moved(Side::Writer.who())?;
    let bytes = enc.encode_without_signature(&expand_line_feeds(&text));
    d.write_bytes(&bytes, OP)
}

/// `ЗаписьДанных.ЗаписатьСтроку(Строка[, КодировкаТекста][, РазделительСтрок])`
/// — то же, что [`write_chars`], плюс разделитель следом.
///
/// # Errors
///
/// Те же, что у [`write_chars`].
pub fn write_line(v: &BslValue, args: &[BslValue]) -> RtResult<()> {
    const OP: &str = "ЗаписатьСтроку";
    let d = writer(v, OP)?;
    at_most(v, args, 3, OP)?;
    let mut d = d.borrow_mut();
    let text = match args.first() {
        Some(BslValue::Str(s)) => s.to_string(),
        _ => {
            return Err(RtError::TypeError {
                expected: "Строка",
                op: OP,
            })
        }
    };
    let enc = encoding_of(&d, args.get(1), OP)?;
    let sep = str_arg(args.get(2), OP)?.unwrap_or_else(|| d.separator.clone());
    d.check_not_moved(Side::Writer.who())?;
    // Перевод `\n` -> `\r\n` накрывает и тело, и разделитель: измерено, что
    // разделитель по умолчанию (один `\n`) ложится байтами `13 10`, а явно
    // назначенный `\r\n` — байтами `13 13 10`.
    let bytes = enc.encode_without_signature(&expand_line_feeds(&(text + &sep)));
    d.write_bytes(&bytes, OP)
}

/// `Закрыть()` у читателя и у писателя.
///
/// Ничего не закрывает и ничего не запрещает: измерено, что целевой поток
/// после этого жив (его `Размер()` работает), повторное закрытие принимается,
/// а `ПрочитатьБайт`/`ЗаписатьБайт` после закрытия по-прежнему работают.
/// Собственного буфера, который стоило бы сбросить, у платформы нет — см.
/// шапку модуля. Поэтому здесь только проверка получателя.
///
/// # Errors
///
/// «Метод не применим», если получатель не читатель и не писатель.
pub fn close(v: &BslValue) -> RtResult<()> {
    data(v, "Закрыть")?;
    Ok(())
}

// --- результат чтения ----------------------------------------------------------------------

/// `РезультатЧтенияДанных.Размер` — именно СВОЙСТВО: вызов со скобками
/// платформа отвергает «Метод объекта не обнаружен (Размер)» (измерено).
///
/// # Errors
///
/// «Метод не применим», если получатель не результат чтения.
#[cfg(test)]
fn result_size(v: &BslValue) -> RtResult<BslValue> {
    let bytes = result_bytes(v, "Размер")?;
    Ok(from_u64(bytes.len() as u64))
}

/// `РезультатЧтенияДанных.ПолучитьДвоичныеДанные()`.
///
/// # Errors
///
/// «Метод не применим», если получатель не результат чтения.
#[cfg(test)]
fn result_binary_data(v: &BslValue) -> RtResult<BslValue> {
    let bytes = result_bytes(v, "ПолучитьДвоичныеДанные")?;
    Ok(BslValue::binary_data_of(bytes.to_vec()))
}

/// `РезультатЧтенияДанных.ПолучитьБуферДвоичныхДанных()`.
///
/// # Errors
///
/// «Метод не применим», если получатель не результат чтения.
#[cfg(test)]
fn result_binary_buffer(v: &BslValue) -> RtResult<BslValue> {
    let bytes = result_bytes(v, "ПолучитьБуферДвоичныхДанных")?;
    Ok(buffer_of_bytes(bytes.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream;

    fn num(v: i64) -> BslValue {
        BslValue::Number(BslNumber::from_i64(v))
    }

    fn frac(units: i128, scale: i32) -> BslValue {
        BslValue::Number(BslNumber::from_parts(units, scale))
    }

    fn text(s: &str) -> BslValue {
        BslValue::Str(BslString::from_str(s))
    }

    fn buffer(bytes: &[u8]) -> BslValue {
        bsl_binbuf::binary_buffer_of(bytes.to_vec())
    }

    fn bytes_of(b: &BslValue) -> Vec<u8> {
        bsl_binbuf::binary_buffer_bytes(b).expect("буфер")
    }

    fn as_u64(v: &BslValue) -> u64 {
        match v {
            BslValue::Number(n) => n.to_i64_exact().expect("целое") as u64,
            _ => panic!("не число: {v:?}"),
        }
    }

    fn as_str(v: &BslValue) -> String {
        match v {
            BslValue::Str(s) => s.to_string(),
            _ => panic!("не строка: {v:?}"),
        }
    }

    /// Поток из шестнадцати байтов 65..80 — тот же, на котором снимались
    /// замеры.
    fn source_stream() -> BslValue {
        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let bytes: Vec<u8> = (0..16u8).map(|i| i + 65).collect();
        stream::write(&s, &[buffer(&bytes), num(0), num(16)]).unwrap();
        stream::seek(
            &s,
            &[num(0), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        s
    }

    fn reader_over(src: &BslValue) -> BslValue {
        new_data_reader(
            src,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined,
        )
        .unwrap()
    }

    fn writer_over(dst: &BslValue) -> BslValue {
        new_data_writer(
            dst,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined,
        )
        .unwrap()
    }

    /// Писатель над потоком НАД БУФЕРОМ: так байты видны прямо в буфере, без
    /// перемотки целевого потока (которая была бы чередованием).
    fn writer_over_buffer(size: usize) -> (BslValue, BslValue) {
        let buf = buffer(&vec![0u8; size]);
        let stream = stream::new_memory_stream(&buf).unwrap();
        (writer_over(&stream), buf)
    }

    // --- ЦЕНТРАЛЬНАЯ СЕМАНТИКА: чередование --------------------------------

    /// Прямой `Перейти` на целевом потоке между операциями читателя обязан
    /// давать ОШИБКУ, а не панику и не тихую рассинхронизацию.
    #[test]
    fn reader_rejects_operation_after_target_stream_was_moved_directly() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 65);
        stream::seek(
            &s,
            &[num(0), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert!(read_byte(&r).is_err());
        // И у остальных операций читателя тот же ответ.
        assert!(read(&r, &[num(4)]).is_err());
        assert!(read_int(&r, &[], IntWidth::W32).is_err());
        assert!(skip(&r, &[num(1)]).is_err());
        assert!(read_line(&r, &[]).is_err());
        assert!(read_chars(&r, &[num(1)]).is_err());
        assert!(read_into_buffer(&r, &[num(1)]).is_err());
    }

    /// То же у писателя — и отвергнутая запись НИЧЕГО не пишет.
    #[test]
    fn writer_rejects_operation_after_target_stream_was_moved_directly() {
        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let w = writer_over(&s);
        write_byte(&w, &[num(65)]).unwrap();
        let size_before = as_u64(&stream::size(&s).unwrap());
        stream::seek(
            &s,
            &[num(0), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert!(write_byte(&w, &[num(66)]).is_err());
        assert!(write_int(&w, &[num(1234567)], IntWidth::W32).is_err());
        assert!(write_chars(&w, &[text("АБ")]).is_err());
        assert!(write_line(&w, &[text("АБ")]).is_err());
        assert!(write(&w, &[BslValue::binary_data_of(vec![1, 2])]).is_err());
        // Проверка стоит ДО побочного эффекта: размер не изменился (измерено).
        assert_eq!(as_u64(&stream::size(&s).unwrap()), size_before);
    }

    /// Ошибка НЕ залипает: возврат позиции на ожидаемую лечит объект, а
    /// `Перейти` на ту же позицию ошибки не даёт вовсе. Измерено обоими
    /// случаями.
    #[test]
    fn moving_the_target_back_to_the_expected_position_heals_the_reader() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 65);
        stream::seek(
            &s,
            &[num(5), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert!(read_byte(&r).is_err());
        stream::seek(
            &s,
            &[num(1), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 66);

        // Переход НА ТУ ЖЕ позицию — не чередование.
        let s = source_stream();
        let r = reader_over(&s);
        stream::seek(
            &s,
            &[num(0), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 65);
        stream::seek(
            &s,
            &[num(1), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 66);
    }

    /// Над `ДвоичныеДанные` понятия чередования нет: подвинуть такой источник
    /// извне нечем.
    #[test]
    fn a_reader_over_binary_data_has_no_interleaving_at_all() {
        let r = reader_over(&BslValue::binary_data_of(vec![65, 66, 67]));
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 65);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 66);
    }

    // --- отсутствие собственного буфера -------------------------------------

    /// Упреждающего чтения нет: позиция целевого потока идёт байт в байт за
    /// читателем (измерено и на памяти, и на файле в 70 000 байт).
    #[test]
    fn the_reader_never_reads_ahead_of_the_target_stream() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 0);
        read_byte(&r).unwrap();
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 1);
        read_byte(&r).unwrap();
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 2);
    }

    /// У писателя симметрично: записанное видно в целевом потоке немедленно,
    /// сбрасывать нечего.
    #[test]
    fn the_writer_reaches_the_target_stream_immediately() {
        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let w = writer_over(&s);
        write_byte(&w, &[num(65)]).unwrap();
        assert_eq!(as_u64(&stream::size(&s).unwrap()), 1);
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 1);
    }

    // --- край источника -------------------------------------------------------

    #[test]
    fn reading_past_the_end_yields_undefined_for_byte_and_integers() {
        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let r = reader_over(&s);
        assert_eq!(read_byte(&r).unwrap(), BslValue::Undefined);

        let s = source_stream();
        let r = reader_over(&s);
        skip(&r, &[num(13)]).unwrap();
        assert_eq!(
            read_int(&r, &[], IntWidth::W32).unwrap(),
            BslValue::Undefined
        );
    }

    /// А вот `Прочитать`, `ПрочитатьВБуферДвоичныхДанных` и `ПрочитатьСимволы`
    /// отдают СКОЛЬКО ЕСТЬ, и ноль у них значит «всё» (измерено).
    #[test]
    fn bulk_reads_return_what_is_left_and_zero_means_everything() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(
            as_u64(&result_size(&read(&r, &[num(100)]).unwrap()).unwrap()),
            16
        );

        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(
            as_u64(&result_size(&read(&r, &[num(0)]).unwrap()).unwrap()),
            16
        );

        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&result_size(&read(&r, &[]).unwrap()).unwrap()), 16);

        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let r = reader_over(&s);
        assert_eq!(
            as_u64(&result_size(&read(&r, &[num(4)]).unwrap()).unwrap()),
            0
        );
        let buf = read_into_buffer(&r, &[num(4)]).unwrap();
        assert_eq!(bytes_of(&buf).len(), 0);
        assert_eq!(as_str(&read_chars(&r, &[num(4)]).unwrap()), "");
    }

    /// `Пропустить` ПЕРЕВОДИТ позицию, а не вычитывает байты: над
    /// шестнадцатью байтами `Пропустить(100)` уводит поток на 100 и отдаёт
    /// 100 (измерено обеими половинами).
    #[test]
    fn skip_moves_the_position_past_the_end_instead_of_reading() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&skip(&r, &[num(100)]).unwrap()), 100);
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 100);
        // Размер потока от этого не меняется, и читать там уже нечего.
        assert_eq!(as_u64(&stream::size(&s).unwrap()), 16);
        assert_eq!(read_byte(&r).unwrap(), BslValue::Undefined);
    }

    /// Неудавшееся чтение целого ОТКАТЫВАЕТ позицию: измерено, что после
    /// него поток стоит там же (13 из 16), а следующий байт — тот самый.
    #[test]
    fn a_short_integer_read_consumes_nothing() {
        let s = source_stream();
        let r = reader_over(&s);
        skip(&r, &[num(13)]).unwrap();
        assert_eq!(
            read_int(&r, &[], IntWidth::W32).unwrap(),
            BslValue::Undefined
        );
        assert_eq!(as_u64(&stream::current_position(&s).unwrap()), 13);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 78);
    }

    // --- целые -----------------------------------------------------------------

    /// Свойство `ПорядокБайтов` у ЧИТАТЕЛЯ на чтение не влияет, а тот же
    /// порядок ИЗ КОНСТРУКТОРА — влияет. Измерено обеими половинами.
    #[test]
    fn the_readers_byte_order_property_is_shown_but_not_used() {
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_u64(&read_int(&r, &[], IntWidth::W16).unwrap()), 16961);

        let s = source_stream();
        let r = reader_over(&s);
        set_prop(
            &r,
            DataRwProp::ByteOrder,
            &BslValue::Enum(EnumValue::ByteOrderBig),
        )
        .unwrap();
        assert_eq!(
            get_prop(&r, DataRwProp::ByteOrder).unwrap(),
            BslValue::Enum(EnumValue::ByteOrderBig)
        );
        assert_eq!(as_u64(&read_int(&r, &[], IntWidth::W16).unwrap()), 16961);

        // Из конструктора — читается.
        let s = source_stream();
        let r = new_data_reader(
            &s,
            &BslValue::Undefined,
            &BslValue::Enum(EnumValue::ByteOrderBig),
            &BslValue::Undefined,
        )
        .unwrap();
        assert_eq!(as_u64(&read_int(&r, &[], IntWidth::W16).unwrap()), 16706);

        // Явный аргумент — тоже.
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(
            as_u64(
                &read_int(
                    &r,
                    &[BslValue::Enum(EnumValue::ByteOrderBig)],
                    IntWidth::W16
                )
                .unwrap()
            ),
            16706
        );
    }

    /// У ПИСАТЕЛЯ то же свойство живое.
    #[test]
    fn the_writers_byte_order_property_is_live() {
        let (w, buf) = writer_over_buffer(4);
        write_int(&w, &[num(258)], IntWidth::W16).unwrap();
        assert_eq!(bytes_of(&buf)[..2], [2, 1]);

        let (w, buf) = writer_over_buffer(4);
        set_prop(
            &w,
            DataRwProp::ByteOrder,
            &BslValue::Enum(EnumValue::ByteOrderBig),
        )
        .unwrap();
        write_int(&w, &[num(258)], IntWidth::W16).unwrap();
        assert_eq!(bytes_of(&buf)[..2], [1, 2]);

        let (w, buf) = writer_over_buffer(4);
        write_int(
            &w,
            &[num(258), BslValue::Enum(EnumValue::ByteOrderBig)],
            IntWidth::W16,
        )
        .unwrap();
        assert_eq!(bytes_of(&buf)[..2], [1, 2]);
    }

    #[test]
    fn integer_and_byte_ranges_match_the_platform() {
        let (w, _buf) = writer_over_buffer(16);
        assert!(write_byte(&w, &[num(255)]).is_ok());
        assert!(write_byte(&w, &[num(256)]).is_err());
        assert!(write_byte(&w, &[num(-1)]).is_err());
        assert!(write_int(&w, &[num(65535)], IntWidth::W16).is_ok());
        assert!(write_int(&w, &[num(65536)], IntWidth::W16).is_err());
        assert!(write_int(&w, &[num(-1)], IntWidth::W16).is_err());
    }

    /// Дробное и отрицательное количество платформа отвергает — без усечения,
    /// в отличие от смещения в буфере у потока.
    /// Лишние аргументы платформа отвергает («Слишком много фактических
    /// параметров» — измерено на `Записать(Буфер, 0, 4)`), и резолвер их не
    /// ловит: у этих методов хвостовые аргументы необязательны.
    #[test]
    fn extra_arguments_are_rejected_at_runtime() {
        let s = source_stream();
        let r = reader_over(&s);
        assert!(read(&r, &[num(1), num(2)]).is_err());
        assert!(read_into_buffer(&r, &[num(1), num(2)]).is_err());
        assert!(skip(&r, &[num(1), num(2)]).is_err());
        assert!(read_line(&r, &[text("UTF-8"), num(2)]).is_err());
        assert!(read_chars(&r, &[num(1), text("UTF-8"), num(2)]).is_err());
        assert!(read_int(&r, &[BslValue::Undefined, num(2)], IntWidth::W16).is_err());

        let (w, _buf) = writer_over_buffer(16);
        assert!(write_byte(&w, &[num(1), num(2)]).is_err());
        assert!(write_int(&w, &[num(1), BslValue::Undefined, num(2)], IntWidth::W16).is_err());
        assert!(write_chars(&w, &[text("a"), text("UTF-8"), num(2)]).is_err());
        assert!(write_line(&w, &[text("a"), text("UTF-8"), text("!"), num(2)]).is_err());
        assert!(write(&w, &[BslValue::binary_data_of(vec![1]), num(0), num(1)]).is_err());
    }

    #[test]
    fn fractional_and_negative_counts_are_rejected() {
        let s = source_stream();
        let r = reader_over(&s);
        assert!(read(&r, &[frac(25, 1)]).is_err());
        assert!(read(&r, &[num(-1)]).is_err());
        assert!(skip(&r, &[num(-1)]).is_err());
        assert!(read_chars(&r, &[frac(25, 1)]).is_err());
    }

    // --- текст ---------------------------------------------------------------------

    /// Сигнатуры в поток нет, а каждый `\n` разворачивается в `\r\n` — и в
    /// теле, и в разделителе.
    #[test]
    fn writing_text_adds_no_signature_but_expands_line_feeds() {
        let (w, buf) = writer_over_buffer(16);
        write_chars(&w, &[text("Аб")]).unwrap();
        assert_eq!(bytes_of(&buf)[..4], [0xD0, 0x90, 0xD0, 0xB1]);

        let (w, buf) = writer_over_buffer(16);
        write_chars(&w, &[text("A\nB")]).unwrap();
        assert_eq!(bytes_of(&buf)[..4], [65, 13, 10, 66]);

        let (w, buf) = writer_over_buffer(16);
        write_chars(&w, &[text("A\r\nB")]).unwrap();
        assert_eq!(bytes_of(&buf)[..5], [65, 13, 13, 10, 66]);

        // Разделитель по умолчанию — один `\n`, ложится как `13 10`.
        let (w, buf) = writer_over_buffer(16);
        write_line(&w, &[text("AB")]).unwrap();
        assert_eq!(bytes_of(&buf)[..4], [65, 66, 13, 10]);
        assert_eq!(
            as_str(&get_prop(&w, DataRwProp::LineSeparator).unwrap()),
            "\n"
        );

        // Назначенный явно — как назначили.
        let (w, buf) = writer_over_buffer(16);
        set_prop(&w, DataRwProp::LineSeparator, &text("!")).unwrap();
        write_line(&w, &[text("AB")]).unwrap();
        assert_eq!(bytes_of(&buf)[..3], [65, 66, 33]);

        // Третий аргумент `ЗаписатьСтроку` — тоже разделитель.
        let (w, buf) = writer_over_buffer(16);
        write_line(&w, &[text("AB"), BslValue::Undefined, text("!")]).unwrap();
        assert_eq!(bytes_of(&buf)[..3], [65, 66, 33]);
    }

    #[test]
    fn text_encoding_is_honoured_on_both_sides() {
        let (w, buf) = writer_over_buffer(8);
        write_chars(&w, &[text("Аб"), text("windows-1251")]).unwrap();
        assert_eq!(bytes_of(&buf)[..2], [0xC0, 0xE1]);

        // Свойство живое у обеих сторон.
        let (w, buf) = writer_over_buffer(8);
        set_prop(&w, DataRwProp::TextEncoding, &text("windows-1251")).unwrap();
        write_chars(&w, &[text("Аб")]).unwrap();
        assert_eq!(bytes_of(&buf)[..2], [0xC0, 0xE1]);

        let src = stream::new_memory_stream(&buffer(&[0xC0, 0xE1])).unwrap();
        let r = reader_over(&src);
        set_prop(&r, DataRwProp::TextEncoding, &text("windows-1251")).unwrap();
        assert_eq!(as_str(&read_chars(&r, &[num(2)]).unwrap()), "Аб");
    }

    /// Количество у `ПрочитатьСимволы` — в СИМВОЛАХ: над кириллицей в UTF-8
    /// два символа съедают четыре байта.
    #[test]
    fn read_chars_counts_characters_not_bytes() {
        let src = stream::new_memory_stream(&buffer("АБВГ".as_bytes())).unwrap();
        let r = reader_over(&src);
        assert_eq!(as_str(&read_chars(&r, &[num(2)]).unwrap()), "АБ");
        assert_eq!(as_u64(&stream::current_position(&src).unwrap()), 4);
    }

    /// Разделитель поглощается и в результат не входит; обратного перевода
    /// `\r\n` в `\n` при чтении НЕТ.
    #[test]
    fn read_line_splits_on_the_separator_without_translating_it_back() {
        let s = source_stream();
        let r = reader_over(&s);
        set_prop(&r, DataRwProp::LineSeparator, &text("F")).unwrap();
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "ABCDE");
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "GHIJKLMNOP");

        // Многосимвольный разделитель.
        let s = source_stream();
        let r = reader_over(&s);
        set_prop(&r, DataRwProp::LineSeparator, &text("CD")).unwrap();
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "AB");
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "EFGHIJKLMNOP");

        // Пустой разделитель (умолчание читателя) — всё до конца.
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "ABCDEFGHIJKLMNOP");

        // Круг: писатель положил `65 66 13 10`, читатель по `\n` отдаёт "AB\r".
        let src = stream::new_memory_stream(&buffer(&[65, 66, 13, 10])).unwrap();
        let r = reader_over(&src);
        set_prop(&r, DataRwProp::LineSeparator, &text("\n")).unwrap();
        assert_eq!(as_str(&read_line(&r, &[]).unwrap()), "AB\r");
    }

    // --- результат чтения --------------------------------------------------------

    #[test]
    fn the_read_result_exposes_size_and_both_getters() {
        let s = source_stream();
        let r = reader_over(&s);
        let res = read(&r, &[num(4)]).unwrap();
        assert_eq!(as_u64(&result_size(&res).unwrap()), 4);
        let buf = result_binary_buffer(&res).unwrap();
        assert_eq!(bytes_of(&buf), vec![65, 66, 67, 68]);
        let bin = result_binary_data(&res).unwrap();
        assert_eq!(bin.type_name(), "ДвоичныеДанные");
    }

    // --- конструктор и закрытие ------------------------------------------------------

    #[test]
    fn the_constructor_rejects_what_the_platform_rejects() {
        assert!(new_data_reader(
            &num(5),
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_err());
        assert!(new_data_reader(
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_err());
        // Буфер двоичных данных источником платформа НЕ принимает.
        assert!(new_data_reader(
            &buffer(&[1, 2]),
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_err());
        // Отсутствующий файл — ошибка.
        assert!(new_data_reader(
            &text("/tmp/open-bsl-нет-такого-файла-datarw.bin"),
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_err());
    }

    /// Конструктор НЕ перематывает поток: читатель продолжает с той позиции,
    /// где поток стоял (измерено — сразу после конструктора позиция та же).
    #[test]
    fn the_constructor_starts_at_the_streams_current_position() {
        let s = source_stream();
        stream::seek(
            &s,
            &[num(4), BslValue::Enum(EnumValue::StreamPositionBegin)],
        )
        .unwrap();
        let r = reader_over(&s);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 69);
    }

    /// `Закрыть` ничего не закрывает и ничего не запрещает — измерено.
    #[test]
    fn closing_neither_closes_the_target_nor_disables_the_object() {
        let s = source_stream();
        let r = reader_over(&s);
        close(&r).unwrap();
        close(&r).unwrap();
        assert_eq!(as_u64(&stream::size(&s).unwrap()), 16);
        assert_eq!(as_u64(&read_byte(&r).unwrap()), 65);

        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let w = writer_over(&s);
        close(&w).unwrap();
        write_byte(&w, &[num(65)]).unwrap();
        assert_eq!(as_u64(&stream::size(&s).unwrap()), 1);
    }

    /// Методы чтения не применимы к писателю и наоборот, а посторонний
    /// получатель отвергается обоими.
    #[test]
    fn methods_reject_the_wrong_side_and_a_foreign_receiver() {
        let s = stream::new_memory_stream(&BslValue::Undefined).unwrap();
        let w = writer_over(&s);
        assert!(read_byte(&w).is_err());
        assert!(read_line(&w, &[]).is_err());

        let s = source_stream();
        let r = reader_over(&s);
        assert!(write_byte(&r, &[num(1)]).is_err());
        assert!(write_chars(&r, &[text("x")]).is_err());

        let not_ours = buffer(&[1, 2]);
        assert!(read_byte(&not_ours).is_err());
        assert!(write_byte(&not_ours, &[num(1)]).is_err());
        assert!(close(&not_ours).is_err());
        assert!(result_size(&not_ours).is_err());
    }

    /// Поток без нужного доступа отвечает ошибкой, а не тихо ничего не делает.
    #[test]
    fn a_stream_without_the_needed_access_is_a_catchable_error() {
        let path = format!(
            "{}/open-bsl-datarw-test-{}.bin",
            std::env::temp_dir().display(),
            std::process::id()
        );
        std::fs::write(&path, b"0123456789").expect("временный файл");
        let ro = stream::manager_open_for_read(&[text(&path)]).unwrap();
        let w = writer_over(&ro);
        assert!(write_byte(&w, &[num(65)]).is_err());
        let wo = stream::manager_open_for_write(&[text(&path)]).unwrap();
        let r = reader_over(&wo);
        assert!(read_byte(&r).is_err());
        let _ = std::fs::remove_file(&path);
    }

    /// Огромное количество у `Прочитать` над ФАЙЛОМ обязано отдавать
    /// «сколько есть». До правки `read_limited` отдавала пользовательское
    /// количество прямо файловому носителю, а тот выделял приёмный буфер
    /// ровно на запрошенное, и `Прочитать(2^45)` убивало процесс отказом
    /// размещения («memory allocation of 35184372088832 bytes failed»,
    /// SIGABRT) — такого `Попытка` не ловит. Над памятью того же дефекта не
    /// было, потому что там количество подрезается длиной носителя.
    ///
    /// Проверка именно на РАВЕНСТВО, а не «не больше носителя»: измерено, что
    /// платформа отдаёт ровно шестнадцать байтов, значит и ошибка вместо
    /// результата этот гейт обязана валить. Двух других форм здесь больше
    /// нет: у них своя, измеренная граница `2^32`, и она проверяется в
    /// [`the_bounded_read_forms_reject_a_count_of_2_32_or_more`].
    #[test]
    fn a_huge_read_over_a_file_is_a_catchable_error_not_an_abort() {
        // 2^45 — то же количество, на котором дефект и был замечен.
        const HUGE: i64 = 35_184_372_088_832;
        let path = format!(
            "{}/open-bsl-datarw-huge-{}.bin",
            std::env::temp_dir().display(),
            std::process::id()
        );
        std::fs::write(&path, b"0123456789ABCDEF").expect("временный файл");

        let r = reader_over(&text(&path));
        let by_read = read(&r, &[num(HUGE)]).expect("Прочитать(2^45) над файлом");
        assert_eq!(
            as_u64(&result_size(&by_read).unwrap()),
            16,
            "Прочитать(2^45) над шестнадцатибайтовым файлом обязан дать 16"
        );

        // Над памятью ответ тот же — асимметрия между носителями и была
        // дефектом.
        let s = source_stream();
        let r = reader_over(&s);
        assert_eq!(
            as_u64(&result_size(&read(&r, &[num(HUGE)]).unwrap()).unwrap()),
            16
        );

        let _ = std::fs::remove_file(&path);
    }

    /// Измеренная граница количества: `ПрочитатьВБуферДвоичныхДанных` и
    /// `ПрочитатьСимволы` при `N >= 2^32` отвечают ОШИБКОЙ («Недопустимое
    /// значение числа» — снято 08.08.2026 на 8.3.27.2074 над
    /// шестнадцатибайтовым файлом, канарейка `Прочитать(100)` = 16 в том же
    /// прогоне, проверено на 2^32, 2^33 и 2^45).
    ///
    /// Отказ идёт по АРГУМЕНТУ, то есть до всякого побочного эффекта:
    /// следующий `ПрочитатьБайт` тем же читателем обязан отдать байт НУЛЕВОЙ
    /// позиции. Проверяется над обоими носителями — граница живёт в разборе
    /// количества и от носителя зависеть не имеет права.
    #[test]
    fn the_bounded_read_forms_reject_a_count_of_2_32_or_more() {
        const COUNTS: [i64; 3] = [4_294_967_296, 8_589_934_592, 35_184_372_088_832];
        let path = format!(
            "{}/open-bsl-datarw-bound-{}.bin",
            std::env::temp_dir().display(),
            std::process::id()
        );
        std::fs::write(&path, b"0123456789ABCDEF").expect("временный файл");

        for count in COUNTS {
            // Над памятью: первый байт источника — 65 («A»).
            let r = reader_over(&source_stream());
            assert!(
                read_into_buffer(&r, &[num(count)]).is_err(),
                "ПрочитатьВБуферДвоичныхДанных({count}) над памятью принято"
            );
            assert_eq!(
                as_u64(&read_byte(&r).unwrap()),
                65,
                "отказ ПрочитатьВБуферДвоичныхДанных({count}) сдвинул читателя над памятью"
            );

            let r = reader_over(&source_stream());
            assert!(
                read_chars(&r, &[num(count)]).is_err(),
                "ПрочитатьСимволы({count}) над памятью принято"
            );
            assert_eq!(
                as_u64(&read_byte(&r).unwrap()),
                65,
                "отказ ПрочитатьСимволы({count}) сдвинул читателя над памятью"
            );

            // Над файлом: первый байт — 48 («0»).
            let r = reader_over(&text(&path));
            assert!(
                read_into_buffer(&r, &[num(count)]).is_err(),
                "ПрочитатьВБуферДвоичныхДанных({count}) над файлом принято"
            );
            assert_eq!(
                as_u64(&read_byte(&r).unwrap()),
                48,
                "отказ ПрочитатьВБуферДвоичныхДанных({count}) сдвинул читателя над файлом"
            );

            let r = reader_over(&text(&path));
            assert!(
                read_chars(&r, &[num(count)]).is_err(),
                "ПрочитатьСимволы({count}) над файлом принято"
            );
            assert_eq!(
                as_u64(&read_byte(&r).unwrap()),
                48,
                "отказ ПрочитатьСимволы({count}) сдвинул читателя над файлом"
            );
        }

        let _ = std::fs::remove_file(&path);
    }

    /// Граница ровно на `2^32`, а не на единицу ниже, и в третью форму она не
    /// протекает.
    ///
    /// `2^32 - 1` живёт ТОЛЬКО здесь: на платформе это количество принимается,
    /// но ответа за 180 с нет и сеанс снимается по таймауту, поэтому в
    /// фикстуру такую пробу класть нельзя — прогон был бы потерян вместе с
    /// эталоном. Чем платформа отвечает ниже границы на количестве больше
    /// носителя — неизвестно; здесь это «сколько есть», как у `Прочитать`.
    #[test]
    fn the_bound_is_exactly_at_2_32_not_below() {
        const JUST_UNDER: i64 = 4_294_967_295;
        const AT_BOUND: i64 = 4_294_967_296;

        let r = reader_over(&source_stream());
        assert_eq!(
            bytes_of(&read_into_buffer(&r, &[num(JUST_UNDER)]).unwrap()).len(),
            16,
            "ПрочитатьВБуферДвоичныхДанных(2^32 - 1) обязан быть принят"
        );

        let r = reader_over(&source_stream());
        assert_eq!(
            as_str(&read_chars(&r, &[num(JUST_UNDER)]).unwrap()),
            "ABCDEFGHIJKLMNOP",
            "ПрочитатьСимволы(2^32 - 1) обязан быть принят"
        );

        // А у `Прочитать` границы нет вовсе — это измеренная асимметрия.
        let r = reader_over(&source_stream());
        assert_eq!(
            as_u64(&result_size(&read(&r, &[num(AT_BOUND)]).unwrap()).unwrap()),
            16,
            "Прочитать(2^32) границы двух других форм не наследует"
        );
    }

    /// `Новый ЗаписьДанных(<существующий файл>)` файл НЕ обрезает: измерено на
    /// 8.3.27.2074, что после «ABCDEFGH», закрытия, повторного открытия
    /// писателем и `ЗаписатьБайт(90)` в файле остаётся восемь байтов
    /// `90 66 67 68 69 70 71 72` — позиция ставится в 0, запись ложится
    /// поверх.
    #[test]
    fn a_writer_over_an_existing_file_does_not_truncate_it() {
        let path = format!(
            "{}/open-bsl-datarw-reopen-{}.bin",
            std::env::temp_dir().display(),
            std::process::id()
        );
        let _ = std::fs::remove_file(&path);

        let w = writer_over(&text(&path));
        write_chars(&w, &[text("ABCDEFGH")]).unwrap();
        close(&w).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"ABCDEFGH");

        let w = writer_over(&text(&path));
        write_byte(&w, &[num(90)]).unwrap();
        close(&w).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"ZBCDEFGH");

        let _ = std::fs::remove_file(&path);
    }

    /// Наборы источников у сторон РАЗНЫЕ: `ДвоичныеДанные` читателю годятся,
    /// а писателю платформа отвечает «Несоответствие типов» уже в
    /// конструкторе (измерено). Имя файла берут оба.
    #[test]
    fn the_two_sides_accept_different_sources() {
        let bin = BslValue::binary_data_of(vec![1, 2, 3]);
        assert!(new_data_reader(
            &bin,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_ok());
        assert!(new_data_writer(
            &bin,
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined
        )
        .is_err());

        // Писатель по имени файла заводит файл и кладёт в него ровно
        // записанное — без сигнатуры (измерено).
        let path = format!(
            "{}/open-bsl-datarw-writer-{}.bin",
            std::env::temp_dir().display(),
            std::process::id()
        );
        let _ = std::fs::remove_file(&path);
        let w = new_data_writer(
            &text(&path),
            &BslValue::Undefined,
            &BslValue::Undefined,
            &BslValue::Undefined,
        )
        .unwrap();
        write_chars(&w, &[text("AB")]).unwrap();
        close(&w).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"AB");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn readers_and_writers_are_equal_by_identity() {
        let s = source_stream();
        let a = reader_over(&s);
        let b = reader_over(&s);
        assert!(a.eq_value(&a));
        assert!(!a.eq_value(&b));
        let same = a.clone();
        assert!(a.eq_value(&same));
    }
}
