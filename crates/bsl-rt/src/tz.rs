//! Смещения часовых поясов — минимальный читатель формата TZif (RFC 8536).
//! Датам JSON нужен фактический офсет машины на заданный момент, а
//! `СмещениеСтандартногоВремени` дополнительно читает стандартный офсет
//! текущей либо именованной зоны без сезонной поправки. Секунды координации
//! и правила за пределами записанных в файл переходов не используются.
//!
//! # Почему свой разбор, а не библиотека времени
//!
//! В крейте нет внешних зависимостей (см. `AGENTS.md`), а `std` не даёт
//! локального смещения вообще — только UTC. Источник истины на Linux один:
//! `/etc/localtime` и именованные файлы базы `tzdata` в формате TZif.
//! Формат компактный: заголовок с счётчиками,
//! таблица переходов, таблица типов смещения — сотня строк разбора вместо
//! внешнего крейта.
//!
//! # Ограничения (сознательные)
//!
//! * Только Linux, `/etc/localtime` и `/usr/share/zoneinfo`: системные пути
//!   просто не подключаются там, где неприменимы.
//! * POSIX-строка правил в хвосте файла (экстраполяция переходов ПОСЛЕ
//!   последнего записанного в таблицу) не разбирается: для смещения на
//!   момент новее последнего перехода берётся офсет этого последнего
//!   перехода. На практике база `tzdata` обновляется с переходами на много
//!   лет вперёд, так что расхождение возможно только на датах в далёком
//!   будущем.
//! * Любая ошибка — чтения файла, магической сигнатуры, счётчиков — даёт
//!   смещение `0` (UTC), а не панику: это вспомогательная, не критичная
//!   для остального рантайма способность (в отличие от, скажем, разбора
//!   байт-кода).
//!
//! Таблица текущей зоны читается ОДИН РАЗ при создании [`SystemTimeZone`]
//! и живёт вместе с ним: часовой пояс машины не меняется на лету, а
//! `/etc/localtime` — файл в несколько килобайт, который незачем
//! перечитывать на каждый вызов. Именованные зоны читаются по запросу;
//! они нужны значительно реже текущей. Раньше таблица лежала в
//! `OnceLock` на весь процесс — от этого пришлось отказаться не ради
//! чистоты, а потому что зона стала возможностью ПРОГОНА: две `State`
//! одного движка вправе видеть разные зоны (см. [`crate::TimeZone`]).

/// Смещение от UTC одного типа `ttinfo` — секунды, положительное значение
/// восточнее Гринвича, и признак сезонного времени.
#[derive(Debug, Clone, Copy)]
struct TzType {
    gmt_offset: i32,
    is_dst: bool,
}

/// Разобранная таблица переходов одного файла TZif.
#[derive(Debug, Clone)]
struct TzTable {
    /// `(момент перехода в секундах Unix-эпохи, индекс типа в `types`)` —
    /// формат сам гарантирует возрастающий порядок по времени.
    transitions: Vec<(i64, u8)>,
    types: Vec<TzType>,
}

impl TzTable {
    fn type_index_for(&self, unix_seconds: i64) -> Option<usize> {
        if self.types.is_empty() {
            return None;
        }
        let n = self
            .transitions
            .partition_point(|&(t, _)| t <= unix_seconds);
        Some(if n == 0 {
            0
        } else {
            self.transitions[n - 1].1 as usize
        })
    }

    /// Смещение, действующее в момент `unix_seconds`.
    fn offset_for(&self, unix_seconds: i64) -> i32 {
        self.type_index_for(unix_seconds)
            .and_then(|index| self.types.get(index))
            .map_or(0, |kind| kind.gmt_offset)
    }

    /// Стандартное смещение без сезонной поправки в заданный момент.
    fn standard_offset_for(&self, unix_seconds: i64) -> i32 {
        let Some(current) = self.type_index_for(unix_seconds) else {
            return 0;
        };
        if !self.types[current].is_dst {
            return self.types[current].gmt_offset;
        }
        let n = self
            .transitions
            .partition_point(|&(t, _)| t <= unix_seconds);
        self.transitions[..n]
            .iter()
            .rev()
            .chain(self.transitions[n..].iter())
            .filter_map(|&(_, index)| self.types.get(index as usize))
            .find(|kind| !kind.is_dst)
            .or_else(|| self.types.iter().find(|kind| !kind.is_dst))
            .map_or(0, |kind| kind.gmt_offset)
    }
}

/// Часовой пояс МАШИНЫ: разобранный при создании `/etc/localtime`.
///
/// Реализация [`crate::TimeZone`] по умолчанию — та, что стояла здесь
/// всегда, только теперь она принадлежит окружению прогона, а не
/// процессу. Непрочитанный или неразобранный файл даёт смещение `0`
/// (UTC): зона — вспомогательная способность, и её отсутствие не должно
/// ронять прогон (см. обзор модуля).
pub struct SystemTimeZone {
    table: Option<TzTable>,
    identifier: Option<String>,
}

impl SystemTimeZone {
    /// Читает и разбирает `/etc/localtime`. Дорого ровно один раз на
    /// создание — дальше смещение берётся из таблицы двоичным поиском.
    #[must_use]
    pub fn new() -> Self {
        SystemTimeZone {
            table: std::fs::read("/etc/localtime")
                .ok()
                .and_then(|bytes| parse_tzif(&bytes)),
            identifier: system_zone_identifier(),
        }
    }
}

impl Default for SystemTimeZone {
    fn default() -> Self {
        Self::new()
    }
}

struct NamedZone<'a>(&'a TzTable);

impl crate::TimeZone for NamedZone<'_> {
    fn offset_seconds(&self, unix_seconds: i64) -> i32 {
        self.0.offset_for(unix_seconds)
    }
}

fn named_zone_table(name: &str) -> Option<TzTable> {
    let path = std::path::Path::new(name);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return None;
    }
    std::fs::read(std::path::Path::new("/usr/share/zoneinfo").join(path))
        .ok()
        .and_then(|bytes| parse_tzif(&bytes))
}

impl crate::TimeZone for SystemTimeZone {
    fn offset_seconds(&self, unix_seconds: i64) -> i32 {
        self.table
            .as_ref()
            .map_or(0, |t| t.offset_for(unix_seconds))
    }

    fn standard_offset_seconds(&self, unix_seconds: i64) -> i32 {
        self.table
            .as_ref()
            .map_or(0, |table| table.standard_offset_for(unix_seconds))
    }

    fn named_standard_offset_seconds(&self, name: &str, unix_seconds: i64) -> Option<i32> {
        if let Some(offset) = crate::env::parse_gmt_offset(name) {
            return Some(offset);
        }
        named_zone_table(name).map(|table| table.standard_offset_for(unix_seconds))
    }

    fn named_offset_seconds(&self, name: &str, unix_seconds: i64) -> Option<i32> {
        if let Some(offset) = crate::env::parse_gmt_offset(name) {
            return Some(offset);
        }
        named_zone_table(name).map(|table| table.offset_for(unix_seconds))
    }

    fn named_offset_for_local(&self, name: &str, wall_seconds: i64) -> Option<i32> {
        if let Some(offset) = crate::env::parse_gmt_offset(name) {
            return Some(offset);
        }
        named_zone_table(name)
            .map(|table| crate::TimeZone::offset_for_local(&NamedZone(&table), wall_seconds))
    }

    fn identifier(&self) -> Option<String> {
        self.identifier.clone()
    }
}

fn system_zone_identifier() -> Option<String> {
    if let Ok(link) = std::fs::read_link("/etc/localtime") {
        let parts = link.components().collect::<Vec<_>>();
        if let Some(index) = parts.iter().position(|part| part.as_os_str() == "zoneinfo") {
            let relative = parts[index + 1..].iter().collect::<std::path::PathBuf>();
            if !relative.as_os_str().is_empty() {
                return relative.to_str().map(str::to_owned);
            }
        }
    }
    std::fs::read_to_string("/etc/timezone")
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// Заголовок TZif (44 байта) — общий формат у блока V1 и у блока V2/V3,
/// отличается только шириной времени переходов в данных ЗА заголовком.
struct Header {
    /// `0` — только V1; `2`/`3` — за этим блоком следует ещё один, с
    /// 8-байтовыми моментами переходов, который и нужно использовать.
    version: u8,
    isutcnt: u32,
    isstdcnt: u32,
    leapcnt: u32,
    timecnt: u32,
    typecnt: u32,
    charcnt: u32,
}

/// Разбирает буфер файла `/etc/localtime`. `None` — буфер не распознан:
/// не тот магический заголовок, неизвестная версия, битые счётчики или
/// буфер короче, чем счётчики обещают.
fn parse_tzif(data: &[u8]) -> Option<TzTable> {
    let (v1_header, v1_end) = parse_header(data)?;
    if v1_header.version == 0 {
        return parse_body(data, v1_end, &v1_header, 4);
    }
    // V2/V3: сразу за блоком V1 (тем же составом данных, но с 4-байтовыми
    // моментами) идёт ТОЧНЫЙ блок с 8-байтовыми моментами — берём его,
    // он покрывает весь диапазон дат TZif, а не только тот, что влезает
    // в `i32`.
    let v2_start = v1_end + body_len(&v1_header, 4);
    let (v2_header, v2_body_start) = parse_header(data.get(v2_start..)?)?;
    parse_body(data, v2_start + v2_body_start, &v2_header, 8)
}

fn parse_header(data: &[u8]) -> Option<(Header, usize)> {
    if data.len() < 44 || &data[0..4] != b"TZif" {
        return None;
    }
    let version = match data[4] {
        0 => 0,
        b'2' => 2,
        b'3' => 3,
        _ => return None,
    };
    let u32_at = |off: usize| -> Option<u32> {
        Some(u32::from_be_bytes(data.get(off..off + 4)?.try_into().ok()?))
    };
    let header = Header {
        version,
        isutcnt: u32_at(20)?,
        isstdcnt: u32_at(24)?,
        leapcnt: u32_at(28)?,
        timecnt: u32_at(32)?,
        typecnt: u32_at(36)?,
        charcnt: u32_at(40)?,
    };
    Some((header, 44))
}

/// Длина блока данных ЗА заголовком при заданной ширине момента перехода
/// (`4` у V1, `8` у V2/V3) — нужна, только чтобы перепрыгнуть блок V1 и
/// найти начало заголовка V2.
fn body_len(h: &Header, time_width: usize) -> usize {
    h.timecnt as usize * time_width
        + h.timecnt as usize
        + h.typecnt as usize * 6
        + h.charcnt as usize
        + h.leapcnt as usize * (time_width + 4)
        + h.isstdcnt as usize
        + h.isutcnt as usize
}

/// Разбирает собственно таблицу переходов и типов, начиная с `start`.
fn parse_body(data: &[u8], start: usize, h: &Header, time_width: usize) -> Option<TzTable> {
    let timecnt = h.timecnt as usize;
    let typecnt = h.typecnt as usize;
    if typecnt == 0 {
        return None;
    }
    let mut pos = start;

    let times_len = timecnt * time_width;
    let times_bytes = data.get(pos..pos + times_len)?;
    pos += times_len;

    let idx_bytes = data.get(pos..pos + timecnt)?;
    pos += timecnt;

    let mut types = Vec::with_capacity(typecnt);
    for i in 0..typecnt {
        let off = pos + i * 6;
        let gmt_offset = i32::from_be_bytes(data.get(off..off + 4)?.try_into().ok()?);
        let is_dst = *data.get(off + 4)? != 0;
        types.push(TzType { gmt_offset, is_dst });
    }

    let mut transitions = Vec::with_capacity(timecnt);
    for i in 0..timecnt {
        let t = if time_width == 4 {
            i32::from_be_bytes(times_bytes[i * 4..i * 4 + 4].try_into().ok()?) as i64
        } else {
            i64::from_be_bytes(times_bytes[i * 8..i * 8 + 8].try_into().ok()?)
        };
        let idx = *idx_bytes.get(i)?;
        if idx as usize >= typecnt {
            return None;
        }
        transitions.push((t, idx));
    }

    Some(TzTable { transitions, types })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Собирает синтетический буфер TZif: заголовок плюс тело с заданной
    /// шириной момента перехода. Для V1-файла (`version = 0`) это и есть
    /// весь буфер; для V2/V3 вызывающий тест склеивает два таких блока —
    /// один с версией `b'2'` и 4-байтовыми переходами, второй следом с
    /// той же версией и 8-байтовыми.
    fn block(
        version: u8,
        types: &[(i32, bool)],
        transitions: &[(i64, u8)],
        time_width: usize,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"TZif");
        buf.push(version);
        buf.extend_from_slice(&[0u8; 15]);
        buf.extend_from_slice(&0u32.to_be_bytes()); // isutcnt
        buf.extend_from_slice(&0u32.to_be_bytes()); // isstdcnt
        buf.extend_from_slice(&0u32.to_be_bytes()); // leapcnt
        buf.extend_from_slice(&(transitions.len() as u32).to_be_bytes());
        buf.extend_from_slice(&(types.len() as u32).to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes()); // charcnt

        for &(t, _) in transitions {
            if time_width == 4 {
                buf.extend_from_slice(&(t as i32).to_be_bytes());
            } else {
                buf.extend_from_slice(&t.to_be_bytes());
            }
        }
        for &(_, idx) in transitions {
            buf.push(idx);
        }
        for &(off, is_dst) in types {
            buf.extend_from_slice(&off.to_be_bytes());
            buf.push(u8::from(is_dst));
            buf.push(0); // desigidx
        }
        buf
    }

    #[test]
    fn v1_only_file_with_a_single_fixed_type_is_used_everywhere() {
        // Один тип, без переходов вовсе — то, что бывает у файлов вроде
        // `Etc/UTC` (офсет 0) или редких фиксированных зон.
        let buf = block(0, &[(4 * 3600, false)], &[], 4);
        let table = parse_tzif(&buf).expect("буфер обязан разобраться");
        assert_eq!(table.offset_for(0), 4 * 3600);
        assert_eq!(table.offset_for(-1_000_000), 4 * 3600);
        assert_eq!(table.offset_for(2_000_000_000), 4 * 3600);
    }

    #[test]
    fn transition_switches_the_offset_at_the_exact_moment() {
        // Типичный переход на летнее время: UTC+3 до 1000, UTC+4 с 1000
        // включительно.
        let buf = block(0, &[(3 * 3600, false), (4 * 3600, true)], &[(1000, 1)], 4);
        let table = parse_tzif(&buf).expect("буфер обязан разобраться");
        assert_eq!(table.offset_for(999), 3 * 3600);
        assert_eq!(table.offset_for(1000), 4 * 3600);
        assert_eq!(table.offset_for(1_000_000), 4 * 3600);
        assert_eq!(table.standard_offset_for(1_000_000), 3 * 3600);
    }

    #[test]
    fn v2_block_is_preferred_over_v1_and_covers_a_wider_range() {
        // V1-блок нарочно несёт ДРУГОЕ (заведомо неверное) смещение —
        // так тест ловит и ошибку в подсчёте длины V1-блока (тогда
        // заголовок V2 не найдётся и разбор провалится), и то, что
        // разбор молча остался бы на V1-данных.
        let v1 = block(b'2', &[(3600, false)], &[(500, 0)], 4);
        let v2 = block(b'2', &[(5 * 3600, false)], &[(500, 0)], 8);
        let mut buf = v1;
        buf.extend_from_slice(&v2);

        let table = parse_tzif(&buf).expect("буфер обязан разобраться");
        assert_eq!(table.offset_for(1_000_000_000_000), 5 * 3600);
    }

    #[test]
    fn garbage_input_is_none_not_a_panic() {
        assert!(parse_tzif(b"not a tzif file").is_none());
        assert!(parse_tzif(&[]).is_none());
        // Магия верна, но буфер обрывается раньше, чем обещают счётчики.
        let mut truncated = b"TZif".to_vec();
        truncated.push(0);
        truncated.extend_from_slice(&[0u8; 10]);
        assert!(parse_tzif(&truncated).is_none());
    }

    #[test]
    fn missing_file_falls_back_to_zero_offset() {
        use crate::TimeZone;
        // Зона машины никогда не паникует и не пробрасывает ошибку —
        // только фолбэк, даже если `/etc/localtime` вообще нет. Файл
        // машины при этом настоящий, поэтому проверяем не значение, а то,
        // что обращение определено и не падает.
        let _ = SystemTimeZone::new().offset_seconds(0);
        // Заведомо негодный буфер даёт зону, отвечающую нулём, а не
        // отсутствие зоны: `None` в таблице — это UTC.
        let broken = SystemTimeZone {
            table: None,
            identifier: None,
        };
        assert_eq!(broken.offset_seconds(0), 0);
        assert_eq!(broken.offset_seconds(2_000_000_000), 0);
    }
}
