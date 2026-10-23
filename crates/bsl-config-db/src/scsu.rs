//! Декодирование SCSU для сериализованных нативных образов.
//!
//! Таблицы и переходы режимов заданы Unicode UTS #6:
//! <https://www.unicode.org/reports/tr6/tr6-4.html>.
//! Нативная проба `measure-native-image-scsu` проверяет такой образ на 1С.

const STATIC: [u32; 8] = [0, 0x80, 0x100, 0x300, 0x2000, 0x2080, 0x2100, 0x3000];
const INITIAL: [u32; 8] = [0x80, 0xc0, 0x400, 0x600, 0x900, 0x3040, 0x30a0, 0xff00];

/// Декодирует целый поток; недопустимые последовательности не заменяются.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, String> {
    let mut windows = INITIAL;
    let mut active = 0;
    let mut unicode = false;
    let mut offset = 0;
    let mut units = Vec::with_capacity(bytes.len());
    while offset < bytes.len() {
        let at = offset;
        let byte = take(bytes, &mut offset)?;
        if unicode {
            match byte {
                0xe0..=0xe7 => {
                    active = usize::from(byte - 0xe0);
                    unicode = false;
                }
                0xe8..=0xef => {
                    active = usize::from(byte - 0xe8);
                    windows[active] = window(take(bytes, &mut offset)?, at)?;
                    unicode = false;
                }
                0xf0 => units.push(pair(bytes, &mut offset)?),
                0xf1 => {
                    let (index, base) = extended(bytes, &mut offset)?;
                    active = index;
                    windows[index] = base;
                    unicode = false;
                }
                0xf2 => return Err(error(at, "зарезервированная команда")),
                _ => {
                    let low = take(bytes, &mut offset)?;
                    units.push(u16::from_be_bytes([byte, low]));
                }
            }
        } else {
            match byte {
                0 | 9 | 10 | 13 | 0x20..=0x7f => units.push(u16::from(byte)),
                1..=8 => {
                    let index = usize::from(byte - 1);
                    let quoted = take(bytes, &mut offset)?;
                    let code = if quoted < 0x80 {
                        STATIC[index] + u32::from(quoted)
                    } else {
                        windows[index] + u32::from(quoted - 0x80)
                    };
                    push(&mut units, code);
                }
                0x0b => {
                    let (index, base) = extended(bytes, &mut offset)?;
                    active = index;
                    windows[index] = base;
                }
                0x0c => return Err(error(at, "зарезервированная команда")),
                0x0e => units.push(pair(bytes, &mut offset)?),
                0x0f => unicode = true,
                0x10..=0x17 => active = usize::from(byte - 0x10),
                0x18..=0x1f => {
                    active = usize::from(byte - 0x18);
                    windows[active] = window(take(bytes, &mut offset)?, at)?;
                }
                0x80..=0xff => push(&mut units, windows[active] + u32::from(byte - 0x80)),
            }
        }
    }
    String::from_utf16(&units).map_err(|_| error(offset, "недопустимая пара суррогатов"))
}

fn take(bytes: &[u8], offset: &mut usize) -> Result<u8, String> {
    let byte = bytes
        .get(*offset)
        .copied()
        .ok_or_else(|| error(*offset, "оборванная последовательность"))?;
    *offset += 1;
    Ok(byte)
}

fn pair(bytes: &[u8], offset: &mut usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes([
        take(bytes, offset)?,
        take(bytes, offset)?,
    ]))
}

fn window(index: u8, at: usize) -> Result<u32, String> {
    Ok(match index {
        1..=0x67 => u32::from(index) * 0x80,
        0x68..=0xa7 => u32::from(index) * 0x80 + 0xac00,
        0xf9..=0xff => {
            [0xc0, 0x250, 0x370, 0x530, 0x3040, 0x30a0, 0xff60][usize::from(index - 0xf9)]
        }
        _ => return Err(error(at, "зарезервированная позиция окна")),
    })
}

fn extended(bytes: &[u8], offset: &mut usize) -> Result<(usize, u32), String> {
    let high = take(bytes, offset)?;
    let low = take(bytes, offset)?;
    Ok((
        usize::from(high >> 5),
        0x10000 + (u32::from(high & 0x1f) * 0x100 + u32::from(low)) * 0x80,
    ))
}

fn push(units: &mut Vec<u16>, code: u32) {
    if code <= 0xffff {
        units.push(code as u16);
    } else {
        let code = code - 0x10000;
        units.push(0xd800 | (code >> 10) as u16);
        units.push(0xdc00 | (code & 0x3ff) as u16);
    }
}

fn error(at: usize, reason: &str) -> String {
    format!("image/SCSU: смещение {at}: {reason}")
}

#[cfg(test)]
#[path = "scsu_tests.rs"]
mod tests;
