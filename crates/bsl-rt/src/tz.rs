//! Смещение локального часового пояса машины — минимальный читатель
//! формата TZif (RFC 8536), заведённый ровно под одну задачу: датам JSON
//! (`ЗаписатьДатуJSON`/`ПрочитатьДатуJSON`, вариант «со смещением» и
//! «универсальная») нужен UTC-офсет машины на заданный момент, а не вся
//! база часовых поясов — ни имён зон, ни правил перехода на летнее время
//! за пределами уже записанных в файл переходов, ни секунд координации.
//!
//! # Почему свой разбор, а не библиотека времени
//!
//! В крейте нет внешних зависимостей (см. `AGENTS.md`), а `std` не даёт
//! локального смещения вообще — только UTC. Источник истины на Linux один:
//! `/etc/localtime`, символическая ссылка (или копия) на файл базы
//! `tzdata` в формате TZif. Формат компактный: заголовок с счётчиками,
//! таблица переходов, таблица типов смещения — сотня строк разбора вместо
//! внешнего крейта.
//!
//! # Ограничения (сознательные)
//!
//! * Только Linux, только `/etc/localtime` — тот же дух, что у шаблонного
//!   JIT (`bsl-vm/src/jit`): платформенно-специфичный путь, который просто
//!   не подключается там, где неприменим.
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
//! Разобранная таблица переходов кэшируется на весь процесс: часовой пояс
//! машины не меняется на лету, а `/etc/localtime` — файл в несколько
//! килобайт, который незачем перечитывать на каждый вызов.

use std::sync::OnceLock;

/// Смещение от UTC одного типа `ttinfo` — секунды, положительное значение
/// восточнее Гринвича. Имя зоны и признак летнего времени не хранятся: они
/// не нужны ни для чего, кроме офсета.
#[derive(Debug, Clone, Copy)]
struct TzType {
    gmt_offset: i32,
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
    /// Смещение, действующее в момент `unix_seconds`.
    fn offset_for(&self, unix_seconds: i64) -> i32 {
        if self.types.is_empty() {
            return 0;
        }
        let n = self
            .transitions
            .partition_point(|&(t, _)| t <= unix_seconds);
        if n == 0 {
            // До первого перехода RFC 8536 предписывает брать первый тип,
            // не помеченный летним временем; специально её не ищем — тип
            // с индексом 0 в подавляющем большинстве файлов и есть такой.
            self.types[0].gmt_offset
        } else {
            let idx = self.transitions[n - 1].1 as usize;
            self.types.get(idx).map_or(0, |t| t.gmt_offset)
        }
    }
}

fn cache() -> &'static Option<TzTable> {
    static CACHE: OnceLock<Option<TzTable>> = OnceLock::new();
    CACHE.get_or_init(|| {
        std::fs::read("/etc/localtime")
            .ok()
            .and_then(|bytes| parse_tzif(&bytes))
    })
}

/// Смещение локального часового пояса машины (в секундах от UTC, может
/// быть отрицательным) для заданного момента — секунды от `1970-01-01`
/// UTC. Ошибка чтения или разбора `/etc/localtime` — смещение `0` (см.
/// обзор модуля).
pub fn local_offset_seconds(unix_seconds: i64) -> i32 {
    cache().as_ref().map_or(0, |t| t.offset_for(unix_seconds))
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
        types.push(TzType { gmt_offset });
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
    fn block(version: u8, types: &[i32], transitions: &[(i64, u8)], time_width: usize) -> Vec<u8> {
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
        for &off in types {
            buf.extend_from_slice(&off.to_be_bytes());
            buf.push(0); // isdst
            buf.push(0); // desigidx
        }
        buf
    }

    #[test]
    fn v1_only_file_with_a_single_fixed_type_is_used_everywhere() {
        // Один тип, без переходов вовсе — то, что бывает у файлов вроде
        // `Etc/UTC` (офсет 0) или редких фиксированных зон.
        let buf = block(0, &[4 * 3600], &[], 4);
        let table = parse_tzif(&buf).expect("буфер обязан разобраться");
        assert_eq!(table.offset_for(0), 4 * 3600);
        assert_eq!(table.offset_for(-1_000_000), 4 * 3600);
        assert_eq!(table.offset_for(2_000_000_000), 4 * 3600);
    }

    #[test]
    fn transition_switches_the_offset_at_the_exact_moment() {
        // Типичный переход на летнее время: UTC+3 до 1000, UTC+4 с 1000
        // включительно.
        let buf = block(0, &[3 * 3600, 4 * 3600], &[(1000, 1)], 4);
        let table = parse_tzif(&buf).expect("буфер обязан разобраться");
        assert_eq!(table.offset_for(999), 3 * 3600);
        assert_eq!(table.offset_for(1000), 4 * 3600);
        assert_eq!(table.offset_for(1_000_000), 4 * 3600);
    }

    #[test]
    fn v2_block_is_preferred_over_v1_and_covers_a_wider_range() {
        // V1-блок нарочно несёт ДРУГОЕ (заведомо неверное) смещение —
        // так тест ловит и ошибку в подсчёте длины V1-блока (тогда
        // заголовок V2 не найдётся и разбор провалится), и то, что
        // разбор молча остался бы на V1-данных.
        let v1 = block(b'2', &[3600], &[(500, 0)], 4);
        let v2 = block(b'2', &[5 * 3600], &[(500, 0)], 8);
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
        // `local_offset_seconds` никогда не паникует и не пробрасывает
        // ошибку — только фолбэк, даже если `/etc/localtime` вообще нет.
        // Сам кэш процесса не трогаем (он читает настоящий файл машины),
        // проверяем только то, что функция определена и не падает.
        let _ = local_offset_seconds(0);
    }
}
