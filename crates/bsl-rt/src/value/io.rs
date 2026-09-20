//! Ввод-вывод значения BSL: текстовая запись, двоичные данные и буфер.

use super::BslValue;
use crate::{BslNumber, BslObject, ByteStreamProtocol, RtError, RtResult, bindata};
use std::io::Write;
use std::rc::Rc;

impl BslValue {
    /// Возвращает байтовую потоковую возможность внешнего объекта.
    pub fn byte_stream(&self) -> Option<&dyn ByteStreamProtocol> {
        self.object_ref()?.byte_stream()
    }

    /// Байты `ДвоичныеДанные` без копирования.
    pub fn binary_data_bytes(&self) -> Option<&[u8]> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryData(bytes) => Some(bytes),
                _ => None,
            },
            _ => None,
        }
    }

    /// Создаёт `БуферДвоичныхДанных` с готовыми байтами и малым порядком.
    pub fn binary_buffer_of(bytes: Vec<u8>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryBuffer(std::cell::RefCell::new(
            bindata::BinBufData::new(bytes, bindata::ByteOrder::Little),
        ))))
    }

    /// Размер буфера либо `None` для значения другого типа.
    pub fn binary_buffer_len(&self) -> Option<usize> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().len()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Снимок байтов буфера.
    pub fn binary_buffer_bytes(&self) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().to_vec()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Копирует ограниченный отрезок буфера; позиция за концом даёт пустой
    /// отрезок, как чтение потока.
    pub fn binary_buffer_slice(&self, offset: u64, count: usize) -> Option<Vec<u8>> {
        match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => Some(buffer.borrow().with_bytes(|bytes| {
                    let Ok(start) = usize::try_from(offset) else {
                        return Vec::new();
                    };
                    let start = start.min(bytes.len());
                    let end = start.saturating_add(count).min(bytes.len());
                    bytes[start..end].to_vec()
                })),
                _ => None,
            },
            _ => None,
        }
    }

    /// Записывает отрезок в существующий буфер без изменения его размера.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для не-буфера или ошибку границ, если отрезок
    /// не помещается в буфер.
    pub fn binary_buffer_write(&self, offset: usize, bytes: &[u8]) -> RtResult<()> {
        let buffer = match self {
            BslValue::Object(object) => match &**object {
                BslObject::BinaryBuffer(buffer) => buffer,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "БуферДвоичныхДанных",
                        op: "Записать",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "БуферДвоичныхДанных",
                    op: "Записать",
                });
            }
        };
        let end = offset.checked_add(bytes.len()).ok_or(RtError::BadIndex)?;
        if end > buffer.borrow().len() {
            return Err(RtError::IndexOutOfBounds {
                index: i64::try_from(end).unwrap_or(i64::MAX),
                len: buffer.borrow().len(),
            });
        }
        buffer.borrow().with_bytes_mut(|target| {
            target[offset..end].copy_from_slice(bytes);
        });
        Ok(())
    }

    /// `Закрыть()` — полиморфен по получателю: `ЗаписьТекста` сбрасывает
    /// буфер и ничего не возвращает, `ЗаписьJSON` отдаёт накопленный
    /// текст.
    ///
    /// Разведение делает сам объект: общая диспетчеризация метода не знает его
    /// конкретный тип заранее.
    ///
    /// # Errors
    ///
    /// Ошибку ввода-вывода либо неприменимость метода к получателю.
    pub fn close_object(&self) -> RtResult<BslValue> {
        self.text_writer_close()
    }

    /// Создаёт объект `ЗаписьТекста` и открывает файл для буферизованной
    /// записи UTF-8 С МЕТКОЙ ПОРЯДКА БАЙТОВ.
    ///
    /// `ЗаписьТекста` над файловой системой прогона. Файл открывается ЗДЕСЬ,
    /// в конструкторе, и объект дальше держит только `FileHandle` — поэтому
    /// файловая система нужна ему лишь на время построения (BORROW), и VM
    /// передаёт её ссылкой из окружения (`host.env()?.files()`), как уже
    /// делает для `Новый ДвоичныеДанные`.
    ///
    /// BOM — ИЗМЕРЕНО на 8.3.27: файл, созданный `Новый ЗаписьТекста(Путь)`
    /// без прочих аргументов, начинается с `EF BB BF`. Отключается он
    /// шестым аргументом конструктора, которого здесь пока нет. Существующий
    /// файл обрезается до нулевой длины.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файл невозможно создать.
    pub fn new_text_writer_with_files(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ЗаписьТекста")?.to_string();
        let path = crate::prepare_file_operation_path(&path, files)
            .map_err(|error| RtError::IoError(error.to_string()))?;
        // `File::create` = открыть-или-создать с обрезанием.
        let handle = files
            .open(
                &path,
                crate::FileOpenOptions::write(crate::FileCreate::OpenOrCreate).truncate(true),
            )
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        let mut buffered = std::io::BufWriter::new(handle);
        std::io::Write::write_all(&mut buffered, &[0xef, 0xbb, 0xbf])
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::Object(Rc::new(BslObject::TextWriter(
            std::cell::RefCell::new(Some(buffered)),
        ))))
    }

    /// Записывает строку в буфер объекта `ЗаписьТекста`.
    ///
    /// UTF-16-представление [`BslString`](crate::BslString) кодируется непосредственно в
    /// UTF-8 без промежуточного [`String`], а перевод строки разворачивается
    /// в CRLF — см. [`BslString::write_utf8_crlf`](crate::BslString::write_utf8_crlf).
    ///
    /// # Errors
    ///
    /// Возвращает ошибку типа для нестрокового аргумента, ошибку
    /// применимости для другого объекта либо [`RtError::IoError`] при
    /// записи в закрытый файл или ошибке файловой системы.
    pub fn text_writer_write(&self, text: &BslValue) -> RtResult<Self> {
        let text = text.as_str("Записать")?;
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    let mut writer = writer.borrow_mut();
                    text.write_utf8_crlf(
                        writer
                            .as_mut()
                            .ok_or_else(|| RtError::IoError("файл уже закрыт".to_string()))?,
                    )
                    .map_err(|e| RtError::IoError(e.to_string()))?;
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Записать",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Записать",
                receiver: self.type_name(),
            }),
        }
    }

    /// Сбрасывает буфер и закрывает `ЗаписьТекста`.
    ///
    /// Повторный вызов безопасен и возвращает `Неопределено`.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку применимости для другого объекта либо
    /// [`RtError::IoError`], если буфер не удалось сбросить.
    pub fn text_writer_close(&self) -> RtResult<Self> {
        match self {
            BslValue::Object(obj) => match &**obj {
                BslObject::TextWriter(writer) => {
                    let mut slot = writer.borrow_mut();
                    // Сброс НА МЕСТЕ, а писатель снимается только ПОСЛЕ
                    // успеха: прежде `take()` забирал буфер ДО `flush()`, и на
                    // отказе `?` уносил ошибку наружу с уже опустевшим слотом
                    // — накопленный текст терялся, а повторный `Закрыть()`
                    // находил `None` и врал успехом при незаписанном тексте.
                    // Теперь на отказе слот цел, и повторный `Закрыть()`
                    // пробует снова.
                    if let Some(writer) = slot.as_mut() {
                        // Сброс буфера в дескриптор, затем ЯВНОЕ закрытие
                        // дескриптора: оба на `?` оставляют слот целым, так
                        // что повторный `Закрыть()` пробует снова (закон
                        // `close` из ABI-G0). `BufWriter` на `Drop` молча
                        // проглотил бы отказ — здесь он наблюдаем.
                        writer
                            .flush()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                        writer
                            .get_mut()
                            .close()
                            .map_err(|e| RtError::IoError(e.to_string()))?;
                    }
                    *slot = None;
                    Ok(BslValue::Undefined)
                }
                _ => Err(RtError::MethodNotApplicable {
                    method: "Закрыть",
                    receiver: self.type_name(),
                }),
            },
            _ => Err(RtError::MethodNotApplicable {
                method: "Закрыть",
                receiver: self.type_name(),
            }),
        }
    }

    /// Двоичные данные из готовых байтов — общий конструктор для чтения
    /// файла, `РазделитьДвоичныеДанные` и `СоединитьДвоичныеДанные`.
    pub fn binary_data_of(bytes: impl Into<Rc<[u8]>>) -> Self {
        BslValue::Object(Rc::new(BslObject::BinaryData(bytes.into())))
    }

    /// Байты значения `ДвоичныеДанные`; для любого другого значения —
    /// ошибка типа с указанием операции, которая его потребовала.
    fn as_binary_data(&self, op: &'static str) -> RtResult<&Rc<[u8]>> {
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::BinaryData(bytes) => Ok(bytes),
                _ => Err(RtError::TypeError {
                    expected: "ДвоичныеДанные",
                    op,
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "ДвоичныеДанные",
                op,
            }),
        }
    }

    /// `Новый ДвоичныеДанные(ИмяФайла)` — файл читается ЦЕЛИКОМ в память
    /// сразу, как и на платформе (размер известен немедленно, а `Размер()`
    /// после удаления файла продолжает отвечать).
    ///
    /// Конструктор без аргументов, с числом вместо имени файла и с двумя
    /// аргументами платформа отвергает (пробы `BIN.NEW.NOARG`,
    /// `BIN.NEW.NUMARG`, `BIN.NEW.TWOARGS`) — ровно один строковый
    /// аргумент, и это проверяет резолвер.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если путь не строка; [`RtError::IoError`],
    /// если файла нет, он недоступен или это каталог (пробы
    /// `BIN.NEW.MISSING`, `BIN.NEW.DIR` — платформа в обоих случаях
    /// бросает исключение).
    pub(crate) fn new_binary_data(
        path: &BslValue,
        files: &dyn crate::FileSystem,
    ) -> RtResult<Self> {
        let path = path.as_str("Новый ДвоичныеДанные")?.to_string();
        let path = crate::prepare_file_operation_path(&path, files)
            .map_err(|e| RtError::IoError(e.to_string()))?;
        let bytes = files
            .read(&path)
            .map_err(|e| RtError::IoError(format!("{path}: {e}")))?;
        Ok(BslValue::binary_data_of(bytes))
    }

    /// `Новый БуферДвоичныхДанных(Размер[, ПорядокБайтов])`.
    ///
    /// Размер обязателен и фиксирует буфер навсегда — роста у него нет;
    /// байты нулевые, порядок по умолчанию `LittleEndian` (измерено).
    /// Пропущенный второй аргумент приходит сюда как
    /// [`BslValue::Undefined`].
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если размер не целое неотрицательное число
    /// (платформа отвергает `-1`, `2.5` и строку `"4"`, а `0` принимает),
    /// если порядок байтов не член `ПорядокБайтов`, а также если буфер
    /// такого размера не удалось разместить в памяти: отказом это лучше,
    /// чем падением процесса на числе из пользовательского текста.
    pub fn new_binary_buffer(size: &BslValue, order: &BslValue) -> RtResult<Self> {
        bindata::new_binary_buffer(size, order)
    }

    /// `ДвоичныеДанные.Размер()` — число байтов.
    ///
    /// # Errors
    ///
    /// [`RtError::MethodNotApplicable`], если получатель — не двоичные
    /// данные.
    pub fn binary_data_size(&self) -> RtResult<Self> {
        let bytes = self
            .as_binary_data("Размер")
            .map_err(|_| RtError::MethodNotApplicable {
                method: "Размер",
                receiver: self.type_name(),
            })?;
        Ok(BslValue::Number(BslNumber::from_i64(bytes.len() as i64)))
    }

    /// `РазделитьДвоичныеДанные(Данные, РазмерЧасти)` -> `Массив` частей.
    ///
    /// ИЗМЕРЕНО на 8.3.27 (пробы `BIN.SPLIT.*`) на 13 байтах: по 5 — три
    /// части 5, 5, 3; по 3 — пять частей 3, 3, 3, 3, 1; по 100 и по
    /// 10 000 000 000 — одна часть целиком; по 10 — две части 10 и 3.
    /// То есть хвост КОРОЧЕ, а не дополняется, и размер части больше
    /// целого — не ошибка.
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если первый аргумент не двоичные данные
    /// (проба `BIN.SPLIT.BADARG`) либо размер части не целое положительное
    /// число, влезающее в 64 бита без знака: ноль, отрицательное, дробное
    /// и даже числовая СТРОКА `"5"` платформой отвергнуты (пробы
    /// `BIN.SPLIT.ZERO`, `.NEGATIVE`, `.FRACTIONAL`, `.STRSIZE`), а
    /// верхняя граница снята фикстурой `binary-data` с точностью до
    /// единицы: `2^64-1` принимается, `2^64` — уже ошибка.
    pub fn binary_data_split(&self, part_size: &BslValue) -> RtResult<Self> {
        const OP: &str = "РазделитьДвоичныеДанные";
        let bad_size = || RtError::TypeError {
            expected: "Целое положительное число не больше 2^64-1",
            op: OP,
        };
        let bytes = self.as_binary_data(OP)?;
        let size = part_size.as_number(OP).map_err(|_| bad_size())?;
        if !size.is_integer()
            || size.is_negative()
            || size.is_zero()
            || *size > binary_split_max_part()
        {
            return Err(bad_size());
        }
        // Размер части шире `usize` ошибкой НЕ является, пока он в
        // пределах `2^64-1`: платформа на 10^10 и на `2^64-1` одинаково
        // отдаёт одну часть целиком, и насыщение до `usize::MAX` даёт
        // ровно это.
        let size = size
            .to_i64_exact()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(usize::MAX);
        // ПУСТЫЕ данные — краевой случай, где `chunks` расходится с
        // платформой: он не даёт ни одной части, а платформа отдаёт массив
        // из ОДНОЙ пустой части (измерено фикстурой `binary-data`, строка
        // «разбиение пустых»). Пустое на входе — пустое на выходе, но
        // обёрнутое.
        if bytes.is_empty() {
            return Ok(BslValue::new_array(vec![BslValue::binary_data_of(
                Vec::new(),
            )]));
        }
        Ok(BslValue::new_array(
            bytes
                .chunks(size)
                .map(BslValue::binary_data_of)
                .collect::<Vec<_>>(),
        ))
    }

    /// `СоединитьДвоичныеДанные(Массив)` -> склеенные данные в порядке
    /// массива (ИЗМЕРЕНО, проба `BIN.COMBINE.ORDER`: три элемента по 13
    /// байт дают 39 байт, и дамп идёт в порядке массива).
    ///
    /// # Errors
    ///
    /// [`RtError::TypeError`], если аргумент не массив (проба
    /// `BIN.COMBINE.NOTARRAY`) или его элемент — не двоичные данные:
    /// платформа отвергает и строку, и `Неопределено` (пробы
    /// `BIN.COMBINE.BADELEM`, `BIN.COMBINE.UNDEF`). Пустой массив
    /// ошибкой НЕ является — он даёт пустые двоичные данные (проба
    /// `BIN.COMBINE.EMPTY`).
    pub fn binary_data_combine(&self) -> RtResult<Self> {
        const OP: &str = "СоединитьДвоичныеДанные";
        let items = match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => items,
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Массив",
                        op: OP,
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Массив",
                    op: OP,
                });
            }
        };
        let items = items.borrow();
        let mut out = Vec::new();
        for item in items.iter() {
            out.extend_from_slice(item.as_binary_data(OP)?);
        }
        Ok(BslValue::binary_data_of(out))
    }
}

/// Наибольший размер части, который принимает `РазделитьДвоичныеДанные`.
///
/// ИЗМЕРЕНО фикстурой `binary-data` с точностью до единицы: `2^64-1`
/// платформа принимает (и отдаёт одну часть целиком), `2^64` — уже
/// ошибка. То есть счётчик у неё 64-битный БЕЗ знака, а не `i64`: `2^63`
/// тоже проходит.
fn binary_split_max_part() -> BslNumber {
    BslNumber::from_i128(u64::MAX as i128)
}

#[cfg(test)]
mod tests {
    use super::super::tests::bin;
    use super::*;
    use crate::tests::num;
    use crate::{BslString, FileHandle};

    /// `ЗаписьТекста.Закрыть()` не теряет буфер при отказе сброса. Файл,
    /// открытый ТОЛЬКО НА ЧТЕНИЕ, заворачивается в `BufWriter`: маленькая
    /// запись остаётся в памяти, а `flush` на закрытии падает. Прежде
    /// `take()` снимал писатель ДО `flush`, и второй `Закрыть()` находил
    /// `None` и врал успехом при незаписанном тексте.
    #[test]
    fn text_writer_close_keeps_the_buffer_when_flush_fails() {
        use std::io::Write as _;
        let path = std::env::temp_dir().join("open-bsl-text-writer-close-fail.txt");
        std::fs::File::create(&path).expect("создать файл");
        let read_only = std::fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .expect("открыть на чтение");
        let mut buffered = std::io::BufWriter::new(Box::new(read_only) as Box<dyn FileHandle>);
        buffered
            .write_all("незаписанный текст".as_bytes())
            .expect("в буфер");
        let writer = BslValue::Object(Rc::new(BslObject::TextWriter(std::cell::RefCell::new(
            Some(buffered),
        ))));
        assert!(
            matches!(writer.text_writer_close(), Err(RtError::IoError(_))),
            "сброс в файл только на чтение обязан упасть"
        );
        assert!(
            matches!(writer.text_writer_close(), Err(RtError::IoError(_))),
            "повторный Закрыть() снова падает — буфер не потерян"
        );

        let ok_path = std::env::temp_dir().join("open-bsl-text-writer-close-ok.txt");
        let ok_file = std::fs::File::create(&ok_path).expect("создать файл");
        let writer_ok = BslValue::Object(Rc::new(BslObject::TextWriter(std::cell::RefCell::new(
            Some(std::io::BufWriter::new(
                Box::new(ok_file) as Box<dyn FileHandle>
            )),
        ))));
        assert!(writer_ok.text_writer_close().is_ok(), "исправный закрылся");
        assert!(
            writer_ok.text_writer_close().is_ok(),
            "повторный Закрыть() идемпотентен"
        );
    }

    /// Размеры частей разбиения — то, что видно из BSL через `Размер()`.
    fn part_sizes(parts: &BslValue) -> Vec<usize> {
        let BslValue::Object(o) = parts else {
            panic!("разбиение обязано отдать массив, отдало {parts:?}");
        };
        let BslObject::Array(items) = &**o else {
            panic!("разбиение обязано отдать массив, отдало {parts:?}");
        };
        items
            .borrow()
            .iter()
            .map(
                |part| match part.binary_data_size().expect("у части есть размер") {
                    BslValue::Number(n) => n.to_i64_exact().expect("размер целый") as usize,
                    other => panic!("Размер() вернул не число: {other:?}"),
                },
            )
            .collect()
    }

    #[test]
    fn binary_data_split_exact_multiple() {
        let parts = bin(b"0123456789ab").binary_data_split(&num("4")).unwrap();
        assert_eq!(part_sizes(&parts), vec![4, 4, 4]);
    }

    #[test]
    fn binary_data_split_short_tail() {
        let parts = bin(b"0123456789").binary_data_split(&num("4")).unwrap();
        assert_eq!(part_sizes(&parts), vec![4, 4, 2]);
    }

    #[test]
    fn binary_data_split_part_larger_than_whole() {
        let parts = bin(b"012").binary_data_split(&num("100")).unwrap();
        assert_eq!(part_sizes(&parts), vec![3]);
        // Размер части шире `usize`, но в пределах `2^64-1`, — не ошибка:
        // та же одна часть (измерено, см. `binary_split_max_part`).
        let parts = bin(b"012")
            .binary_data_split(&num("18446744073709551615"))
            .unwrap();
        assert_eq!(part_sizes(&parts), vec![3]);
        // На единицу больше — уже ошибка, ровно как у платформы.
        assert!(
            bin(b"012")
                .binary_data_split(&num("18446744073709551616"))
                .is_err()
        );
    }

    /// Пустые данные дают массив из ОДНОЙ пустой части, а не пустой массив
    /// (измерено фикстурой `binary-data`).
    #[test]
    fn binary_data_split_empty_yields_one_empty_part() {
        let parts = bin(b"").binary_data_split(&num("5")).unwrap();
        assert_eq!(part_sizes(&parts), vec![0]);
    }

    #[test]
    fn binary_data_split_rejects_non_positive_and_fractional_sizes() {
        for bad in ["0", "-1", "2.5"] {
            assert!(
                bin(b"0123").binary_data_split(&num(bad)).is_err(),
                "размер части {bad} обязан быть ошибкой"
            );
        }
        // Числовая строка тоже отвергается — платформа её не приводит.
        assert!(
            bin(b"0123")
                .binary_data_split(&BslValue::Str(BslString::from_str("5")))
                .is_err()
        );
        // Разбивать не двоичные данные нечего.
        assert!(
            BslValue::Str(BslString::from_str("абв"))
                .binary_data_split(&num("2"))
                .is_err()
        );
    }

    #[test]
    fn binary_data_combine_empty_array() {
        let joined = BslValue::new_array(vec![]).binary_data_combine().unwrap();
        assert_eq!(joined, bin(b""));
        assert!(!joined.is_filled().expect("пустые данные не заполнены"));
        assert_eq!(joined.to_string(), "");
    }

    #[test]
    fn binary_data_combine_concatenates_in_array_order() {
        let joined = BslValue::new_array(vec![bin(b"ab"), bin(b""), bin(b"cd")])
            .binary_data_combine()
            .unwrap();
        assert_eq!(joined, bin(b"abcd"));
    }

    #[test]
    fn binary_data_combine_rejects_a_non_binary_element() {
        let bad = BslValue::new_array(vec![bin(b"ab"), BslValue::Str(BslString::from_str("вг"))]);
        assert!(bad.binary_data_combine().is_err());
        let bad = BslValue::new_array(vec![bin(b"ab"), BslValue::Undefined]);
        assert!(bad.binary_data_combine().is_err());
        // Аргумент вообще не массив.
        assert!(bin(b"ab").binary_data_combine().is_err());
    }
}
