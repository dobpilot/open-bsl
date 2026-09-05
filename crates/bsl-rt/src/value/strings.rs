//! Строковые операции значения BSL; алгоритмы принадлежат `BslString`.

use super::BslValue;
use crate::{BslNumber, BslObject, BslString, EnumValue, RtError, RtResult, uuid};

fn optional_positive_usize(value: &BslValue, default: usize, op: &'static str) -> RtResult<usize> {
    if matches!(value, BslValue::Undefined) {
        return Ok(default);
    }
    let BslValue::Number(number) = value else {
        return Err(RtError::TypeError {
            expected: "Положительное целое число",
            op,
        });
    };
    number
        .to_i64_exact()
        .and_then(|number| usize::try_from(number).ok())
        .filter(|number| *number != 0)
        .ok_or(RtError::TypeError {
            expected: "Положительное целое число",
            op,
        })
}

impl BslValue {
    // --- Строки ---------------------------------------------------------

    pub fn str_len(&self) -> RtResult<usize> {
        Ok(self.as_str("СтрДлина")?.len_utf16())
    }

    pub fn str_left(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(
            self.as_str("Лев")?.left(len.as_usize("Лев")?),
        ))
    }

    pub fn str_right(&self, len: &Self) -> RtResult<Self> {
        Ok(BslValue::Str(
            self.as_str("Прав")?.right(len.as_usize("Прав")?),
        ))
    }

    /// `Сред(Строка, Начало[, Длина])` — `Неопределено` на месте длины
    /// (аргумент опущен, см. `BuiltinFn::arity_range`) значит "до конца
    /// строки".
    pub fn str_mid(&self, start: &Self, len: &Self) -> RtResult<Self> {
        let s = self.as_str("Сред")?;
        let start = start.as_usize("Сред")?;
        let len = match len {
            BslValue::Undefined => s.len_utf16(),
            other => other.as_usize("Сред")?,
        };
        Ok(BslValue::Str(s.substring(start, len)))
    }

    fn string_for_case(&self, op: &'static str) -> RtResult<BslString> {
        match self {
            BslValue::Str(value) => Ok(value.clone()),
            // ИЗМЕРЕНО(STRING.LOWER.UUID.VALUE,
            // STRING.UPPER.UUID.VALUE): функции регистра принимают UUID и
            // сначала используют его каноническую строковую форму.
            BslValue::Object(object) => match &**object {
                BslObject::Uuid(bytes) => Ok(BslString::from_utf8_string(uuid::format(bytes))),
                _ => Err(RtError::TypeError {
                    expected: "Строка или УникальныйИдентификатор",
                    op,
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Строка или УникальныйИдентификатор",
                op,
            }),
        }
    }

    pub fn str_upper(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.string_for_case("ВРег")?.to_uppercase()))
    }

    pub fn str_lower(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.string_for_case("НРег")?.to_lowercase()))
    }

    pub fn str_trim_all(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛП")?.trim()))
    }

    pub fn str_trim_left(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрЛ")?.trim_start()))
    }

    pub fn str_trim_right(&self) -> RtResult<Self> {
        Ok(BslValue::Str(self.as_str("СокрП")?.trim_end()))
    }

    /// `СтрНайти(Строка, Подстрока)` — позиция в КОД-ЮНИТАХ UTF-16, 1-based,
    /// `0` если не найдено. Те же единицы, что считает `СтрДлина`, чтобы
    /// результат можно было передать в `Сред`/`Лев` без пересчёта.
    pub fn str_find(
        &self,
        needle: &Self,
        direction: &Self,
        start: &Self,
        occurrence: &Self,
    ) -> RtResult<Self> {
        let text = self.as_str("СтрНайти")?;
        let needle = needle.as_str("СтрНайти")?;
        if needle.units().is_empty() || needle.units().len() > text.units().len() {
            return Ok(BslValue::number_from_i64(0));
        }
        let from_end = match direction {
            BslValue::Undefined | BslValue::Enum(EnumValue::SearchFromBegin) => false,
            BslValue::Enum(EnumValue::SearchFromEnd) => true,
            _ => {
                return Err(RtError::TypeError {
                    expected: "НаправлениеПоиска",
                    op: "СтрНайти",
                });
            }
        };
        let default_start = if from_end { text.units().len() } else { 1 };
        let start = optional_positive_usize(start, default_start, "СтрНайти")?;
        let occurrence = optional_positive_usize(occurrence, 1, "СтрНайти")?;
        if start > text.units().len() {
            return Ok(BslValue::number_from_i64(0));
        }

        let last = text.units().len() - needle.units().len();
        let matches = |position: &usize| {
            text.units()[*position..*position + needle.units().len()] == *needle.units()
        };
        let position = if from_end {
            (0..=last)
                .rev()
                .filter(|position| *position < start)
                .filter(matches)
                .nth(occurrence - 1)
        } else {
            (start - 1..=last).filter(matches).nth(occurrence - 1)
        };
        Ok(BslValue::number_from_i64(
            position.map_or(0, |position| position as i64 + 1),
        ))
    }

    pub fn str_starts_with(&self, prefix: &Self) -> RtResult<Self> {
        let text = self.as_str("СтрНачинаетсяС")?;
        let prefix = prefix.as_str("СтрНачинаетсяС")?;
        if prefix.units().is_empty() {
            return Err(RtError::TypeError {
                expected: "Непустая строка",
                op: "СтрНачинаетсяС",
            });
        }
        Ok(BslValue::Boolean(text.units().starts_with(prefix.units())))
    }

    pub fn str_ends_with(&self, suffix: &Self) -> RtResult<Self> {
        let text = self.as_str("СтрЗаканчиваетсяНа")?;
        let suffix = suffix.as_str("СтрЗаканчиваетсяНа")?;
        if suffix.units().is_empty() {
            return Err(RtError::TypeError {
                expected: "Непустая строка",
                op: "СтрЗаканчиваетсяНа",
            });
        }
        Ok(BslValue::Boolean(text.units().ends_with(suffix.units())))
    }

    pub fn str_replace(&self, from: &Self, to: &Self) -> RtResult<Self> {
        let uuid_text;
        let text = match self {
            BslValue::Str(text) => text,
            BslValue::Object(object) => match &**object {
                BslObject::Uuid(bytes) => {
                    uuid_text = BslString::from_utf8_string(uuid::format(bytes));
                    &uuid_text
                }
                _ => {
                    return Err(RtError::TypeError {
                        expected: "Строка или УникальныйИдентификатор",
                        op: "СтрЗаменить",
                    });
                }
            },
            _ => {
                return Err(RtError::TypeError {
                    expected: "Строка или УникальныйИдентификатор",
                    op: "СтрЗаменить",
                });
            }
        };
        Ok(BslValue::Str(text.replace(
            from.as_str("СтрЗаменить")?,
            to.as_str("СтрЗаменить")?,
        )))
    }

    /// `СтрРазделить(Строка, Разделитель[, ВключатьПустые])` -> `Массив`.
    pub fn str_split(&self, sep: &Self, include_empty: &Self) -> RtResult<Self> {
        let include_empty = match include_empty {
            BslValue::Undefined => true,
            BslValue::Boolean(value) => *value,
            _ => {
                return Err(RtError::TypeError {
                    expected: "Булево",
                    op: "СтрРазделить",
                });
            }
        };
        let parts = self
            .as_str("СтрРазделить")?
            .split(sep.as_str("СтрРазделить")?)
            .into_iter()
            .filter(|part| include_empty || !part.units().is_empty());
        Ok(BslValue::new_array(parts.map(BslValue::Str).collect()))
    }

    /// `СтрСоединить(Массив, Разделитель)`. Не-строковые элементы массива
    /// приводятся к строке через `Display` — у 1С `СтрСоединить` тоже
    /// принимает массив любых значений, а не только строк. ВНИМАНИЕ:
    /// приведение здесь идёт МИМО `bsl-format` (этот крейт ниже него
    /// слоем), поэтому число получает каноническую форму, а не
    /// локализованную с NBSP-группировкой — см. `Display for BslNumber`.
    /// Это осознанное расхождение уровня слоёв, а не забытая локализация:
    /// массив строк (обычный случай) оно не затрагивает вовсе.
    pub fn str_join(&self, sep: &Self) -> RtResult<Self> {
        let sep = sep.as_str("СтрСоединить")?;
        match self {
            BslValue::Object(o) => match &**o {
                BslObject::Array(items) => {
                    let parts: Vec<BslString> = items
                        .borrow()
                        .iter()
                        .map(|v| match v {
                            BslValue::Str(s) => s.clone(),
                            other => BslString::from_str(&other.to_string()),
                        })
                        .collect();
                    Ok(BslValue::Str(BslString::join(&parts, sep)))
                }
                _ => Err(RtError::TypeError {
                    expected: "Массив",
                    op: "СтрСоединить",
                }),
            },
            _ => Err(RtError::TypeError {
                expected: "Массив",
                op: "СтрСоединить",
            }),
        }
    }

    pub fn str_line_count(&self) -> RtResult<Self> {
        let n = self.as_str("СтрЧислоСтрок")?.line_count();
        Ok(BslValue::Number(BslNumber::from_i64(n as i64)))
    }

    pub fn str_get_line(&self, n: &Self) -> RtResult<Self> {
        let s = self.as_str("СтрПолучитьСтроку")?;
        let n = n.as_usize("СтрПолучитьСтроку")?;
        Ok(BslValue::Str(s.line_at(n)))
    }

    /// `СтрШаблон(Шаблон, З1, ..., З10)` — значения уже дополнены до
    /// `MAX_TEMPLATE_ARGS` штук `Неопределено` резолвером (см.
    /// `BuiltinFn::arity_range`); хвостовые `Неопределено` отбрасываются
    /// здесь, чтобы `%3` при двух реально переданных значениях дал пусто, а
    /// не строковое представление `Неопределено` (оно, впрочем, тоже
    /// пустое — но полагаться на это совпадение не нужно).
    pub fn str_template(&self, values: &[Self]) -> RtResult<Self> {
        let tmpl = self.as_str("СтрШаблон")?;
        let end = values
            .iter()
            .rposition(|v| !matches!(v, BslValue::Undefined))
            .map(|i| i + 1)
            .unwrap_or(0);
        let vals: Vec<BslString> = values[..end]
            .iter()
            .map(|v| match v {
                BslValue::Str(s) => s.clone(),
                other => BslString::from_str(&other.to_string()),
            })
            .collect();
        Ok(BslValue::Str(tmpl.template(&vals)))
    }

    /// `Символ(Код)`.
    /// ИЗМЕРЕНО на 8.3.27: `Символ(128512)` возвращает ПУСТУЮ строку, а не
    /// суррогатную пару и не ошибку — платформа за пределы BMP не выходит.
    /// `Символ(65535)` при этом даёт строку длины 1, а `Символ(65)` — «A».
    pub fn char_from_code(&self) -> RtResult<Self> {
        let code = self.as_number("Символ")?;
        let code = code.to_i64_exact().and_then(|c| u32::try_from(c).ok());
        let text = match code {
            Some(c) if c <= 0xFFFF => BslString::from_char_code(c),
            // Астральный код — пустая строка, как на платформе.
            Some(_) => Some(BslString::from_str("")),
            None => None,
        };
        Ok(BslValue::Str(text.ok_or(RtError::TypeError {
            expected: "Код символа (целое в диапазоне Unicode)",
            op: "Символ",
        })?))
    }

    /// `КодСимвола(Строка[, Позиция])` — позиция по умолчанию `1`.
    pub fn char_code(&self, pos: &Self) -> RtResult<Self> {
        let s = self.as_str("КодСимвола")?;
        let pos = match pos {
            BslValue::Undefined => 1,
            other => other.as_usize("КодСимвола")?,
        };
        // ИЗМЕРЕНО: `КодСимвола("")` даёт -1, а не ошибку. Позиция за
        // границей непустой строки замером не покрыта — трактуем так же,
        // потому что «за границей» тут ровно тот же случай.
        let code = s.char_code_at(pos).map_or(-1, |c| c as i64);
        Ok(BslValue::Number(BslNumber::from_i64(code)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn uuid_is_accepted_by_case_conversion_functions() {
        let uuid = BslValue::Object(Rc::new(BslObject::Uuid([
            0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56,
            0x78, 0x90,
        ])));
        assert_eq!(
            uuid.str_lower().unwrap().to_string(),
            "abcdef12-3456-7890-abcd-ef1234567890"
        );
        assert_eq!(
            uuid.str_upper().unwrap().to_string(),
            "ABCDEF12-3456-7890-ABCD-EF1234567890"
        );
    }
}
