//! Контейнер: чтение записей ZIP, имена и даты как их видит платформа.

use super::*;

// --- контейнер: читатель ---------------------------------------------------

/// Способ хранения 0 — данные лежат как есть.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const METHOD_STORED: u16 = 0;
/// Способ хранения 8 — поток deflate (RFC 1951).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const METHOD_DEFLATED: u16 = 8;

pub(crate) fn zip_err(what: &str) -> RtError {
    RtError::Zip(format!("ZIP: {what}"))
}

/// Адаптер ошибок крейта `zip` в [`RtError`].
pub(crate) fn zip_error_to_rt(e: zip::result::ZipError) -> RtError {
    use zip::result::ZipError::*;
    match e {
        Io(io) => zip_err(&format!("ошибка ввода-вывода: {io}")),
        InvalidArchive(msg) => zip_err(&format!("это не архив ZIP или он испорчен: {msg}")),
        UnsupportedArchive(msg) => zip_err(&format!("архив не поддерживается: {msg}")),
        FileNotFound => zip_err("запись не найдена"),
        InvalidPassword => zip_err("неверный пароль"),
        _ => zip_err("неизвестная ошибка ZIP"),
    }
}

/// Одна запись архива — метаданные, считанные из центрального каталога.
///
/// Имя хранится СЫРЫМИ байтами (`name_raw` из крейта `zip`), а декодированное
/// отдаётся только когда байты образуют UTF-8. Платформа, как выяснилось
/// замером, поступает иначе: она декодирует имя КАК UTF-8 независимо от бита
/// 11, подставляя `U+FFFD` на негодных байтах. Это решение поверхности
/// встроенного языка, а не формата, поэтому оно и живёт ниже, в
/// [`ArchiveState`], а здесь имя остаётся байтами.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct RawEntry {
    pub(crate) name: Vec<u8>,
    pub(crate) method: u16,
    pub(crate) crc: u32,
    pub(crate) compressed_size: u64,
    pub(crate) size: u64,
    pub(crate) encrypted: bool,
    pub(crate) is_directory: bool,
    /// Поля времени и даты MS-DOS как они лежат в каталоге. Разбираются
    /// не здесь, а в [`dos_datetime`]: правило нормализации у платформы
    /// своё и измерено отдельно.
    pub(crate) mod_time: u16,
    pub(crate) mod_date: u16,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RawEntry {
    /// Имя записи как оно лежит в каталоге, без всякого перекодирования.
    pub(crate) fn name_bytes(&self) -> &[u8] {
        &self.name
    }

    /// Способ хранения из каталога: 0 — как есть, 8 — deflate, прочие
    /// читателю неизвестны.
    pub(crate) fn method(&self) -> u16 {
        self.method
    }

    /// Контрольная сумма распакованных данных из каталога.
    pub(crate) fn crc(&self) -> u32 {
        self.crc
    }

    /// Размер данных в архиве.
    pub(crate) fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// Размер распакованных данных.
    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    /// Данные записи зашифрованы. Расшифровки нет: [`read_entry`] на такой
    /// записи отказывает, не читая данных.
    pub(crate) fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Запись — это каталог: формат обозначает его завершающим слэшем в
    /// имени, отдельного признака в нём нет.
    pub(crate) fn is_directory(&self) -> bool {
        self.is_directory
    }

    /// Время изменения записи так, как его показывает встроенный язык.
    pub(crate) fn modified(&self) -> BslDate {
        dos_datetime(self.mod_time, self.mod_date)
    }
}

/// Сигнатура записи центрального каталога (APPNOTE 4.3.12).
pub(crate) const SIG_CENTRAL: u32 = 0x0201_4B50;
/// Сигнатура записи конца каталога (APPNOTE 4.3.16).
pub(crate) const SIG_EOCD: u32 = 0x0605_4B50;
/// Длина неизменяемой части записи каталога.
pub(crate) const CENTRAL_HEADER_LEN: usize = 46;
/// Длина записи конца каталога без комментария.
pub(crate) const EOCD_LEN: usize = 22;
/// Комментарий архива не длиннее 65535 байт.
pub(crate) const MAX_COMMENT: usize = 0xFFFF;
/// Бит 0 общих флагов — данные записи зашифрованы.
pub(crate) const FLAG_ENCRYPTED: u16 = 1;
/// Бит 11 — имя записи в UTF-8.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FLAG_UTF8_NAME: u16 = 1 << 11;

/// Срез `len` байт по смещению `at`.
pub(crate) fn slice_at(data: &[u8], at: usize, len: usize) -> Result<&[u8], RtError> {
    let end = at.checked_add(len).ok_or_else(truncated)?;
    data.get(at..end).ok_or_else(truncated)
}

pub(crate) fn truncated() -> RtError {
    zip_err("архив обрезан или испорчен")
}

pub(crate) fn u16_at(data: &[u8], at: usize) -> Result<u16, RtError> {
    let b = slice_at(data, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) fn u32_at(data: &[u8], at: usize) -> Result<u32, RtError> {
    let b = slice_at(data, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Разобрать архив: записи центрального каталога и комментарий.
///
/// Разбор каталога идёт вручную, а не через `zip::ZipArchive`, потому что
/// крейт `zip` хранит записи в `IndexMap` по имени и молча выбрасывает
/// дубликаты — а платформа дубликаты допускает и `build_items` нумерует их
/// суффиксом `(N)`. Распаковка записей в [`read_entry`] делегирована крейту
/// `zip`.
///
/// # Errors
///
/// [`RtError::Zip`] на любом входе, который не является читаемым архивом ZIP.
pub(crate) fn parse_archive(data: &[u8]) -> RtResult<(Vec<RawEntry>, Vec<u8>)> {
    let eocd = find_eocd(data)?;
    let cd_size = u32::from_le_bytes([
        data[eocd + 12],
        data[eocd + 13],
        data[eocd + 14],
        data[eocd + 15],
    ]);
    let cd_offset = u32::from_le_bytes([
        data[eocd + 16],
        data[eocd + 17],
        data[eocd + 18],
        data[eocd + 19],
    ]) as usize;
    let cd_size = cd_size as usize;
    let cd_end = cd_offset.checked_add(cd_size).ok_or_else(truncated)?;
    if cd_end > data.len() {
        return Err(zip_err("центральный каталог выходит за границу файла"));
    }
    let mut entries = Vec::new();
    let mut at = cd_offset;
    while at + CENTRAL_HEADER_LEN <= cd_end {
        if u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) != SIG_CENTRAL {
            return Err(zip_err("в центральном каталоге запись без сигнатуры"));
        }
        let flags = u16_at(data, at + 8)?;
        let method = u16_at(data, at + 10)?;
        let mod_time = u16_at(data, at + 12)?;
        let mod_date = u16_at(data, at + 14)?;
        let crc = u32_at(data, at + 16)?;
        let compressed_size = u64::from(u32_at(data, at + 20)?);
        let size = u64::from(u32_at(data, at + 24)?);
        let name_len = usize::from(u16_at(data, at + 28)?);
        let extra_len = usize::from(u16_at(data, at + 30)?);
        let comment_len = usize::from(u16_at(data, at + 32)?);
        let name_at = at + CENTRAL_HEADER_LEN;
        let name_end = name_at.checked_add(name_len).ok_or_else(truncated)?;
        if name_end > cd_end {
            return Err(truncated());
        }
        let name = data[name_at..name_end].to_vec();
        let is_dir = name.last() == Some(&b'/');
        entries.push(RawEntry {
            name,
            method,
            crc,
            compressed_size,
            size,
            encrypted: flags & FLAG_ENCRYPTED != 0,
            is_directory: is_dir,
            mod_time,
            mod_date,
        });
        at = name_end + extra_len + comment_len;
    }
    let comment_len = usize::from(u16_at(data, eocd + 20)?);
    let comment = slice_at(data, eocd + EOCD_LEN, comment_len)
        .map_err(|_| truncated())?
        .to_vec();
    Ok((entries, comment))
}

/// Найти запись конца центрального каталога сканированием от конца файла.
pub(crate) fn find_eocd(data: &[u8]) -> Result<usize, RtError> {
    if data.len() < EOCD_LEN {
        return Err(zip_err(
            "файл короче записи конца каталога — это не архив ZIP",
        ));
    }
    let last = data.len() - EOCD_LEN;
    let first = last.saturating_sub(MAX_COMMENT);
    for at in (first..=last).rev() {
        if u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) != SIG_EOCD {
            continue;
        }
        let comment = usize::from(u16::from_le_bytes([data[at + 20], data[at + 21]]));
        if at + EOCD_LEN + comment == data.len() {
            return Ok(at);
        }
    }
    Err(zip_err(
        "не найдена запись конца центрального каталога — это не архив ZIP",
    ))
}

/// Сигнатура локального заголовка записи (APPNOTE 4.3.7).
pub(crate) const SIG_LOCAL: u32 = 0x0403_4B50;
/// Длина неизменяемой части локального заголовка.
pub(crate) const LOCAL_HEADER_LEN: usize = 30;

/// Прочитать и распаковать запись с номером `index` из байтов архива.
///
/// Запись находится по смещению локального заголовка, которое берётся из
/// центрального каталога (а не через `zip::ZipArchive`, который дедуплицирует
/// имена и теряет дубликаты). Распаковка делегирована `flate2` для метода 8
/// и копированием для метода 0.
///
/// # Errors
///
/// [`RtError::Zip`], если записи с таким номером нет, данные зашифрованы,
/// способ хранения не поддерживается или распаковка не удалась.
pub(crate) fn read_entry(data: &[u8], index: usize, entry: &RawEntry) -> RtResult<Vec<u8>> {
    if entry.encrypted {
        return Err(zip_err(&format!(
            "запись «{}» зашифрована, а зашифрованные архивы не поддерживаются",
            String::from_utf8_lossy(&entry.name)
        )));
    }
    let local_offset = find_local_offset(data, index)?;
    let header = local_offset;
    if u32::from_le_bytes([
        data[header],
        data[header + 1],
        data[header + 2],
        data[header + 3],
    ]) != SIG_LOCAL
    {
        return Err(zip_err(&format!(
            "у записи «{}» нет локального заголовка по объявленному смещению",
            String::from_utf8_lossy(&entry.name)
        )));
    }
    // Длины имени и extra берутся из локального заголовка: они законно
    // отличаются от каталожных.
    let name_len = usize::from(u16::from_le_bytes([data[header + 26], data[header + 27]]));
    let extra_len = usize::from(u16::from_le_bytes([data[header + 28], data[header + 29]]));
    let data_start = header + LOCAL_HEADER_LEN + name_len + extra_len;
    let compressed_size = usize::try_from(entry.compressed_size)
        .map_err(|_| zip_err("запись архива не помещается в адресное пространство"))?;
    let packed = slice_at(data, data_start, compressed_size)?;

    let out = match entry.method {
        METHOD_STORED => {
            if packed.len() != usize::try_from(entry.size).unwrap_or(usize::MAX) {
                return Err(zip_err(&format!(
                    "у записи «{}» способ хранения 0, но размеры не совпадают",
                    String::from_utf8_lossy(&entry.name)
                )));
            }
            packed.to_vec()
        }
        METHOD_DEFLATED => {
            let mut decoder = flate2::read::DeflateDecoder::new(packed);
            let mut out = Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0).min(1 << 24));
            decoder
                .read_to_end(&mut out)
                .map_err(|e| zip_err(&format!("ошибка распаковки записи: {e}")))?;
            out
        }
        other => {
            return Err(zip_err(&format!(
                "у записи «{}» способ хранения {other} не поддерживается",
                String::from_utf8_lossy(&entry.name)
            )));
        }
    };
    Ok(out)
}

/// Найти смещение локального заголовка записи номер `index` по центральному
/// каталогу.
pub(crate) fn find_local_offset(data: &[u8], index: usize) -> RtResult<usize> {
    let eocd = find_eocd(data)?;
    let cd_offset = u32::from_le_bytes([
        data[eocd + 16],
        data[eocd + 17],
        data[eocd + 18],
        data[eocd + 19],
    ]) as usize;
    let mut at = cd_offset;
    let mut current = 0usize;
    while at + CENTRAL_HEADER_LEN <= data.len() {
        if u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) != SIG_CENTRAL {
            break;
        }
        let name_len = usize::from(u16::from_le_bytes([data[at + 28], data[at + 29]]));
        let extra_len = usize::from(u16::from_le_bytes([data[at + 30], data[at + 31]]));
        let comment_len = usize::from(u16::from_le_bytes([data[at + 32], data[at + 33]]));
        if current == index {
            let offset =
                u32::from_le_bytes([data[at + 42], data[at + 43], data[at + 44], data[at + 45]])
                    as usize;
            return Ok(offset);
        }
        current += 1;
        at += CENTRAL_HEADER_LEN + name_len + extra_len + comment_len;
    }
    Err(zip_err(&format!("в архиве нет записи с номером {index}")))
}

/// Время изменения записи из полей MS-DOS.
///
/// Правило ИЗМЕРЕНО по краям всех полей, и оно арифметическое, а не
/// зажимающее: компоненты нормализуются, как если бы их сложили. Месяц 0
/// уходит в декабрь прошлого года, месяц 13 — в январь следующего, день 0 —
/// в последний день предыдущего месяца, 30 февраля — во 2 марта, а час 25
/// добавляет сутки (2000-й год: месяц 13 -> `2001-01-01`, месяц 0 ->
/// `1999-12-01`, день 0 января -> `1999-12-31`, `2001-02-30` ->
/// `2001-03-02`, час 25 -> `2000-01-02 01:00`, минута 61 -> `01:01`,
/// поле секунд 31 -> `00:01:02`). Нулевые поля целиком дают `1979-11-30`.
pub(crate) fn dos_datetime(time: u16, date: u16) -> BslDate {
    let year = 1980 + i64::from(date >> 9);
    let month = i64::from((date >> 5) & 0xF);
    let day = i64::from(date & 0x1F);
    let hour = i64::from(time >> 11);
    let minute = i64::from((time >> 5) & 0x3F);
    let second = i64::from(time & 0x1F) * 2;

    // Месяц нормализуется в паре с годом, остальное — простым сложением
    // секунд поверх первого числа этого месяца.
    let months = year * 12 + month - 1;
    let (y, m) = (months.div_euclid(12), months.rem_euclid(12) + 1);
    let Some(first) = BslDate::from_civil(y, m as u32, 1, 0, 0, 0) else {
        return BslDate::empty();
    };
    let shift = (day - 1) * 86_400 + hour * 3600 + minute * 60 + second;
    BslDate::from_seconds(first.seconds() + shift).unwrap_or_else(BslDate::empty)
}
