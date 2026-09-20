//! Простой общий механизм сопоставления имён без файлового ввода-вывода.

/// Сопоставляет имя с простой регистрозависимой маской `*`/`?`.
///
/// Сохраняет прежний алгоритм ZIP: `?` обрабатывает один Unicode-скаляр,
/// квадратные скобки и разделители не получают специального смысла.
/// Нормализация путей остаётся у вызывающего. Эта функция не объявляет
/// простой диалект полным файловым диалектом целевой ОС.
pub fn simple_mask_matches(mask: &str, name: &str) -> bool {
    let mask: Vec<char> = mask.chars().collect();
    let name: Vec<char> = name.chars().collect();
    match_units(&mask, &name, '*', |at, value| {
        mask.get(at)
            .filter(|token| **token == '?' || **token == value)
            .map(|_| at + 1)
    })
}

/// Сопоставляет имя с байтовым файловым диалектом измеренной Linux-среды.
///
/// Не нормализует пути и не моделирует особые объекты `sfile://`.
/// Неизмеренные расширения перечислены в реестре вопросов совместимости.
pub fn file_mask_matches(mask: &str, name: &str) -> bool {
    let mask = mask.as_bytes();
    match_units(mask, name.as_bytes(), b'*', |at, value| {
        match *mask.get(at)? {
            b'*' => None,
            b'?' => Some(at + 1),
            b'\\' => (mask.get(at + 1) == Some(&value)).then_some(at + 2),
            b'[' => match class_match(mask, at + 1, value) {
                Some((end, matched)) => matched.then_some(end),
                None => (value == b'[').then_some(at + 1),
            },
            literal => (literal == value).then_some(at + 1),
        }
    })
}

// Литеральные `*` и `\` не входят в область чистого сопоставителя: на
// измеренной платформе первое имя становится объектом `sfile://`, а второе
// разбирается как разделитель пути. Их поиск остаётся в `FIND.TRAVERSAL`.
fn named_class(name: &[u8], value: u8) -> bool {
    match name {
        b"alpha" => value.is_ascii_alphabetic(),
        b"alnum" => value.is_ascii_alphanumeric(),
        b"digit" => value.is_ascii_digit(),
        b"xdigit" => value.is_ascii_hexdigit(),
        b"lower" => value.is_ascii_lowercase(),
        b"upper" => value.is_ascii_uppercase(),
        b"blank" => matches!(value, b' ' | b'\t'),
        b"space" => matches!(value, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12),
        b"cntrl" => value.is_ascii_control(),
        b"print" => value.is_ascii_graphic() || value == b' ',
        b"graph" => value.is_ascii_graphic(),
        b"punct" => value.is_ascii_punctuation(),
        _ => false,
    }
}

fn class_match(mask: &[u8], mut at: usize, value: u8) -> Option<(usize, bool)> {
    let negated = matches!(mask.get(at), Some(b'!' | b'^'));
    at += usize::from(negated);
    let first = at;
    let mut matched = false;
    while at < mask.len() {
        if mask[at] == b']' && at != first {
            return Some((at + 1, matched != negated));
        }
        if mask[at..].starts_with(b"[:") {
            let end = mask[at + 2..].windows(2).position(|pair| pair == b":]")? + at + 2;
            matched |= named_class(&mask[at + 2..end], value);
            at = end + 2;
            continue;
        }
        let low = if mask[at] == b'\\' {
            at += 1;
            *mask.get(at)?
        } else {
            mask[at]
        };
        at += 1;
        if mask.get(at) == Some(&b'-') && mask.get(at + 1).is_some_and(|byte| *byte != b']') {
            at += 1;
            if mask[at] == b'\\' {
                at += 1;
            }
            let high = *mask.get(at)?;
            matched |= low <= value && value <= high;
            at += 1;
        } else {
            matched |= low == value;
        }
    }
    None
}

fn match_units<T: Copy + Eq>(
    mask: &[T],
    name: &[T],
    star_token: T,
    one: impl Fn(usize, T) -> Option<usize>,
) -> bool {
    // Классический двухуказательный разбор со звёздочкой-точкой возврата:
    // рекурсия по маске из чужого ввода могла бы уйти сколь угодно глубоко.
    let (mut m, mut n) = (0usize, 0usize);
    let (mut star, mut back) = (usize::MAX, 0usize);
    while n < name.len() {
        if let Some(next) = one(m, name[n]) {
            m = next;
            n += 1;
        } else if m < mask.len() && mask[m] == star_token {
            star = m;
            back = n;
            m += 1;
        } else if star != usize::MAX {
            back += 1;
            m = star + 1;
            n = back;
        } else {
            return false;
        }
    }
    while m < mask.len() && mask[m] == star_token {
        m += 1;
    }
    m == mask.len()
}
