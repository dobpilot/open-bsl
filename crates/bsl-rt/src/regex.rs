//! Регулярные выражения: собственный движок с бэктрекингом по подмножеству
//! диалекта ICU, на котором говорит платформа.
//!
//! Внешних крейтов в этом дереве не бывает (правило рабочей области), а
//! регулярные выражения нужны целиком — поэтому и разбор шаблона, и матчинг
//! написаны здесь. Поверхности BSL у движка пока нет:
//! `СтрНайтиПоРегулярномуВыражению` и соседи приходят следующей задачей,
//! этот модуль — только машинерия под ними. Отсюда `pub(crate)` и
//! `#[allow(dead_code)]` на объявлении модуля в `lib.rs`.
//!
//! # Устройство
//!
//! Слоя два. Первый — рекурсивный спуск по кодовым точкам шаблона
//! ([`Parser`]), строящий дерево [`Node`]: альтернация → конкатенация →
//! квантованный атом. Второй — [`Compiler`], разворачивающий дерево в
//! ПЛОСКУЮ программу [`Instr`] с абсолютными адресами переходов, и
//! `Regex::run_at`, исполняющий её с ЯВНЫМ стеком бэктрекинга.
//!
//! Хостовой рекурсии в матчинге нет намеренно: глубина перебора задаётся
//! входом пользователя, а переполнение стека Rust — не перехватываемая
//! `Попыткой` ошибка, а смерть процесса. По той же причине рекурсия РАЗБОРА
//! ограничена [`MAX_DEPTH`], а разворачивание `{n,m}` — [`MAX_PROGRAM`]: и
//! то, и другое отдаёт [`RtError::Regex`], а не падает.
//!
//! Цена бэктрекинга — ровно та же, что у ICU: на шаблонах вида `(а+)+б`
//! время экспоненциально от длины входа. Отсечки по числу шагов здесь нет
//! сознательно: платформа ведёт себя так же, а тихая отсечка превратила бы
//! «искали слишком долго» в «не нашли», то есть в неверный ответ вместо
//! медленного.
//!
//! # Кодовые точки поверх UTF-16
//!
//! Вход — код-юниты рантаймовой строки (`BslString`), декодирование идёт НА
//! МЕСТЕ: суррогатная пара собирается в одну кодовую точку, и `.`, классы и
//! кванторы считают её ОДНИМ символом. Непарный суррогат — законная точка
//! со своим значением, а не ошибка: строка BSL не обязана быть корректным
//! UTF-16, и матчинг не имеет права на этом падать. Позиции и границы групп
//! движок отдаёт в КОД-ЮНИТАХ — это родные индексы строкового рантайма,
//! которыми будут пользоваться builtins.
//!
//! # Что поддержано
//!
//! Литералы и экранирование (`\\`, `\.`, `\n`, `\t`, `\r`, `\f`, `\a`,
//! `\e`, `\uXXXX`, экранированная пунктуация), `.`, классы `[...]`/`[^...]`
//! с диапазонами и вложенными сокращениями, `\d`/`\D`/`\w`/`\W`/`\s`/`\S`,
//! `\b`/`\B`, якоря `^`/`$`, кванторы `*`/`+`/`?`/`{n}`/`{n,}`/`{n,m}` —
//! жадные и ленивые с суффиксом `?`, группы захвата `(...)` и
//! незахватывающие `(?:...)`, альтернация `|`, инлайн-флаги `(?i)`/`(?m)`,
//! свойства Unicode `\p{L}`/`\p{Nd}` и их отрицания `\P{...}`.
//!
//! Всё остальное — ЧЕСТНАЯ ошибка разбора, а не молчаливый литерал:
//! просмотр вперёд `(?=...)`, именованные группы, обратные ссылки `\1`,
//! операции над множествами `[a-z&&[^k]]`, притяжательные кванторы `a*+`,
//! флаг `(?s)`. Молчаливое превращение непонятой конструкции в литерал —
//! худший из вариантов: шаблон «работает», но ищет не то.
//!
//! # Семантика совпадения
//!
//! Поиск leftmost: первая позиция слева, на которой матч удался. На
//! позиции порядок обхода задан приоритетом ветвей бэктрекинга — жадный
//! квантор сперва пробует больше, ленивый меньше, альтернация идёт слева
//! направо. Группы нумеруются по ОТКРЫВАЮЩЕЙ скобке слева направо, поэтому
//! `((а)(б))(в)` даёт порядок «первая, её вложенные, затем вторая»; группа
//! 0 — всё совпадение; неучаствовавшая группа — `None`, а не пустая строка
//! (это разные вещи: `(а)?` на `б` не участвовала, `(а*)` на `б`
//! участвовала и пуста). НЕОБЯЗАТЕЛЬНАЯ итерация, не сдвинувшая позицию,
//! прекращает повторение: у `*`/`+`/`{n,}` иначе `(а*)*` зациклился бы
//! навсегда, а у `{n,m}` этим задан порядок захватов — текст достаётся
//! ранней итерации, а последней остаётся пустая правее. Обязательные копии
//! `0..min` идут без стража, поэтому `(а*){3}` совпадает с «б» тремя
//! пустыми проходами, а `{n}` обрывать нечего.
//!
//! # Что решено рассуждением, а не замером
//!
//! Поверхности BSL ещё нет, поэтому ни одно из решений ниже не проверено на
//! 8.3.27; каждому заведена тройка «метка + запись в реестре + строка в
//! скрипте замеров»: `REGEX.CLASS.WORD` (состав `\w`), `REGEX.CLASS.SPACE`
//! (состав `\s`), `REGEX.FOLD.SIMPLE` (свёртка регистра при `(?i)`),
//! `REGEX.FLAG.SCOPE` (область действия инлайн-флага),
//! `REGEX.LINE.TERMINATORS` (что считается концом строки для `.` и `(?m)`),
//! `REGEX.ANCHOR.EOL` (где стоят `^` и `$` относительно хвостового перевода
//! строки), `REGEX.REPEAT.EMPTY` (какой итерации ограниченного квантора
//! достаётся захват, когда тело совпало с пустотой).

mod tables;

use crate::{RtError, RtResult};
use std::collections::HashMap;
use tables::{in_ranges, DIGIT_RANGES, LETTER_RANGES};

/// Кодовая точка. Именно `u32`, а не `char`: непарный суррогат — законная
/// точка этого движка, а `char` его не представляет.
type Cp = u32;

/// Предел вложенности групп при разборе.
///
/// Разбор рекурсивен, и без предела шаблон вида `((((((…))))))` уносил бы
/// процесс переполнением стека вместо перехватываемой ошибки. Сто уровней —
/// заведомо больше любого осмысленного шаблона и заведомо меньше того, что
/// не держит стек debug-сборки.
const MAX_DEPTH: usize = 100;

/// Предел длины скомпилированной программы.
///
/// `{n,m}` разворачивается копиями тела, поэтому `(?:а{1000}){1000}` — это
/// миллион инструкций из двенадцати символов шаблона. Предел реализации, а
/// не платформы: он даёт внятную ошибку вместо гигабайтов памяти.
///
/// Считаются именно инструкции, и этого достаточно: таблица классов растёт
/// не с разворачиванием, а с числом классов В ШАБЛОНЕ — все копии одного
/// узла делят одну запись (`Compiler::class_index`).
const MAX_PROGRAM: usize = 200_000;

/// Предел счётчика в `{n,m}`. Тоже предел реализации; настоящую отсечку
/// делает [`MAX_PROGRAM`], а этот ловит бессмысленные числа раньше и
/// заодно не даёт счётчику переполниться.
const MAX_REPEAT: u32 = 65_535;

fn bad(what: impl Into<String>) -> RtError {
    RtError::Regex(what.into())
}

/// Кодовая точка для сообщения об ошибке: печатаемая — как есть, непарный
/// суррогат — кодом.
fn show(cp: Cp) -> String {
    char::from_u32(cp).map_or_else(|| format!("U+{cp:04X}"), |c| c.to_string())
}

// --- UTF-16 -> кодовые точки --------------------------------------------

const CP_TAB: Cp = 0x09;
const CP_LF: Cp = 0x0A;
const CP_VT: Cp = 0x0B;
const CP_FF: Cp = 0x0C;
const CP_CR: Cp = 0x0D;
const CP_NEL: Cp = 0x85;
const CP_LS: Cp = 0x2028;
const CP_PS: Cp = 0x2029;

/// Кодовая точка, начинающаяся в позиции `i`, и её ширина в код-юнитах.
/// `None` — за концом строки.
fn cp_at(units: &[u16], i: usize) -> Option<(Cp, usize)> {
    let first = Cp::from(*units.get(i)?);
    if (0xD800..0xDC00).contains(&first) {
        if let Some(low) = units.get(i + 1).copied().map(Cp::from) {
            if (0xDC00..0xE000).contains(&low) {
                return Some((0x1_0000 + ((first - 0xD800) << 10) + (low - 0xDC00), 2));
            }
        }
    }
    Some((first, 1))
}

/// Кодовая точка, ЗАКАНЧИВАЮЩАЯСЯ в позиции `i`, и её ширина. Нужна
/// границам слова и якорям: они смотрят назад.
fn cp_before(units: &[u16], i: usize) -> Option<(Cp, usize)> {
    if i == 0 {
        return None;
    }
    let last = Cp::from(*units.get(i - 1)?);
    if (0xDC00..0xE000).contains(&last) && i >= 2 {
        let high = Cp::from(units[i - 2]);
        if (0xD800..0xDC00).contains(&high) {
            return Some((0x1_0000 + ((high - 0xD800) << 10) + (last - 0xDC00), 2));
        }
    }
    Some((last, 1))
}

/// Весь текст в кодовых точках — так разбирается ШАБЛОН (входная строка
/// декодируется на месте, без этой копии).
fn decode(units: &[u16]) -> Vec<Cp> {
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    while let Some((cp, width)) = cp_at(units, i) {
        out.push(cp);
        i += width;
    }
    out
}

/// Конец строки для `.` и для `(?m)`.
///
/// Взят набор UTS#18, которому следует ICU: перевод строки, вертикальная
/// табуляция, перевод формата, возврат каретки, NEL и разделители строк и
/// абзацев. Java, например, вертикальную табуляцию и перевод формата
/// концом строки НЕ считает, так что выбор наблюдаем —
/// `НЕ ИЗМЕРЕНО(REGEX.LINE.TERMINATORS)`.
fn is_line_terminator(cp: Cp) -> bool {
    matches!(cp, CP_LF | CP_VT | CP_FF | CP_CR | CP_NEL | CP_LS | CP_PS)
}

/// Свёртка регистра для `(?i)` — ТОЛЬКО одинарные отображения.
///
/// `char::to_lowercase` для некоторых точек даёт несколько символов
/// (`İ` U+0130 → `i` + U+0307); брать первый символ такого отображения
/// нельзя — это уже другая точка, и `(?i)İ` начал бы совпадать с `i`.
/// Многосимвольные отображения оставляются как есть, то есть такие пары
/// под `(?i)` не сворачиваются вовсе. `ё`/`Ё`, вся кириллица и латиница
/// сворачиваются одинарно и работают.
/// `НЕ ИЗМЕРЕНО(REGEX.FOLD.SIMPLE)`.
fn simple_lower(cp: Cp) -> Cp {
    let Some(c) = char::from_u32(cp) else {
        return cp;
    };
    let mut mapping = c.to_lowercase();
    match (mapping.next(), mapping.next()) {
        (Some(single), None) => single as Cp,
        (Some(_), Some(_)) => cp,
        // `to_lowercase` всегда отдаёт хотя бы один символ; ветка — ради
        // исчерпывающего разбора, а не ради случая.
        (None, _) => cp,
    }
}

/// Обратная сторона [`simple_lower`] — нужна классам: `[А-Я]` под `(?i)`
/// обязан ловить `а`, а свёртка вниз этого не даёт.
fn simple_upper(cp: Cp) -> Cp {
    let Some(c) = char::from_u32(cp) else {
        return cp;
    };
    let mut mapping = c.to_uppercase();
    match (mapping.next(), mapping.next()) {
        (Some(single), None) => single as Cp,
        (Some(_), Some(_)) => cp,
        (None, _) => cp,
    }
}

// --- свойства символов --------------------------------------------------

/// Именованный набор точек: сокращения `\d`/`\w`/`\s` и свойства
/// `\p{...}`. `\d` — это в точности `\p{Nd}`, отдельного варианта у него
/// нет.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropKind {
    /// `\p{Nd}` и `\d` — десятичные цифры любой письменности.
    Digit,
    /// `\w`.
    Word,
    /// `\s`.
    Space,
    /// `\p{L}` — буквы.
    Letter,
}

impl PropKind {
    fn matches(self, cp: Cp) -> bool {
        match self {
            PropKind::Digit => in_ranges(DIGIT_RANGES, cp),
            PropKind::Letter => in_ranges(LETTER_RANGES, cp),
            // Состав `\w`: буква, десятичная цифра, подчёркивание. ICU
            // документирует более широкий набор (плюс `\p{M}`, `\p{Pc}` и
            // соединители U+200C/U+200D), но чем считает платформа —
            // `НЕ ИЗМЕРЕНО(REGEX.CLASS.WORD)`.
            PropKind::Word => {
                in_ranges(LETTER_RANGES, cp) || in_ranges(DIGIT_RANGES, cp) || cp == '_' as Cp
            }
            // Состав `\s`: свойство Unicode `White_Space`. ICU считает `\s`
            // как `[\t\n\f\r\p{Z}]` — там нет вертикальной табуляции,
            // которая в `White_Space` есть. Расхождение наблюдаемо —
            // `НЕ ИЗМЕРЕНО(REGEX.CLASS.SPACE)`.
            PropKind::Space => char::from_u32(cp).is_some_and(char::is_whitespace),
        }
    }
}

/// Символ слова для `\b`/`\B` — тот же набор, что у `\w`.
fn is_word_cp(cp: Cp) -> bool {
    PropKind::Word.matches(cp)
}

// --- разобранный шаблон -------------------------------------------------

/// Элемент класса символов.
#[derive(Clone, Debug)]
enum ClassItem {
    Single(Cp),
    Range(Cp, Cp),
    /// Вложенное сокращение: `[\d\p{L}-]`.
    Prop {
        kind: PropKind,
        negated: bool,
    },
}

impl ClassItem {
    fn matches(&self, cp: Cp) -> bool {
        match self {
            ClassItem::Single(one) => *one == cp,
            ClassItem::Range(from, to) => *from <= cp && cp <= *to,
            ClassItem::Prop { kind, negated } => kind.matches(cp) != *negated,
        }
    }
}

/// Класс символов целиком.
#[derive(Clone, Debug)]
struct ClassSpec {
    negated: bool,
    items: Vec<ClassItem>,
}

impl ClassSpec {
    /// Совпадение с учётом регистра.
    ///
    /// Свёртка применяется ДО отрицания, а не после: иначе `(?i)[^а-я]`
    /// поймал бы `А` (её нет среди элементов, значит отрицание пропускает),
    /// то есть отрицание перестало бы быть отрицанием того же множества.
    fn matches(&self, cp: Cp, icase: bool) -> bool {
        let mut hit = self.items.iter().any(|item| item.matches(cp));
        if !hit && icase {
            let lower = simple_lower(cp);
            let upper = simple_upper(cp);
            hit = (lower != cp && self.items.iter().any(|item| item.matches(lower)))
                || (upper != cp && self.items.iter().any(|item| item.matches(upper)));
        }
        hit != self.negated
    }
}

/// Узел дерева шаблона. Флаги, действовавшие в точке разбора, вморожены в
/// узел: инлайн-флаг `(?i)` меняет их по ходу, и хранить их снаружи
/// означало бы потерять, к чему именно флаг относился.
#[derive(Clone, Debug)]
enum Node {
    /// Ничего не потребляет — результат `(?i)` и пустой ветви `а|`.
    Empty,
    Literal {
        cp: Cp,
        icase: bool,
    },
    Class {
        spec: ClassSpec,
        icase: bool,
    },
    /// `.`
    Any,
    /// `^`
    LineStart {
        multiline: bool,
    },
    /// `$`
    LineEnd {
        multiline: bool,
    },
    /// `\b` и `\B`
    WordBoundary {
        negated: bool,
    },
    /// Группа; `slot` — её номер, `None` у незахватывающей `(?:...)`.
    Group {
        slot: Option<usize>,
        body: Box<Node>,
    },
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        body: Box<Node>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
}

/// Флаги, действующие в текущей точке разбора.
#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    icase: bool,
    multiline: bool,
}

/// Что дало экранирование.
enum Escape {
    Cp(Cp),
    Prop { kind: PropKind, negated: bool },
    WordBoundary { negated: bool },
}

// --- разбор -------------------------------------------------------------

struct Parser<'a> {
    src: &'a [Cp],
    pos: usize,
    depth: usize,
    /// Сколько групп захвата уже открыто — номер следующей.
    groups: usize,
    flags: Flags,
}

impl<'a> Parser<'a> {
    fn new(src: &'a [Cp]) -> Parser<'a> {
        Parser {
            src,
            pos: 0,
            depth: 0,
            groups: 0,
            flags: Flags::default(),
        }
    }

    fn peek(&self) -> Option<Cp> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<Cp> {
        self.src.get(self.pos + ahead).copied()
    }

    fn bump(&mut self) -> Option<Cp> {
        let cp = self.peek()?;
        self.pos += 1;
        Some(cp)
    }

    fn eat(&mut self, cp: Cp) -> bool {
        if self.peek() == Some(cp) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Альтернация — самый слабый уровень: `конкатенация ('|' конкатенация)*`.
    fn alternation(&mut self) -> RtResult<Node> {
        let mut branches = vec![self.concat()?];
        while self.eat('|' as Cp) {
            branches.push(self.concat()?);
        }
        Ok(if branches.len() == 1 {
            // `swap_remove` вместо `into_iter().next().unwrap()`: без
            // распаковки `Option` там, где длина уже проверена.
            branches.swap_remove(0)
        } else {
            Node::Alt(branches)
        })
    }

    /// Конкатенация: подряд идущие квантованные атомы до `|`, `)` или конца.
    fn concat(&mut self) -> RtResult<Node> {
        let mut parts: Vec<Node> = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(cp) if cp == '|' as Cp || cp == ')' as Cp => break,
                Some(_) => {}
            }
            // Инлайн-флаг — не атом: квантовать его нечем, и следующий за
            // ним `*` обязан стать ошибкой «квантор без атома».
            if self.inline_flags()? {
                continue;
            }
            parts.push(self.quantified()?);
        }
        Ok(match parts.len() {
            0 => Node::Empty,
            1 => parts.swap_remove(0),
            _ => Node::Concat(parts),
        })
    }

    /// `(?i)`, `(?m)`, `(?im)` — меняет флаги ДО конца охватывающей группы.
    ///
    /// Возвращает `false`, если в позиции не инлайн-флаг: тогда `(` разберёт
    /// [`Parser::atom`] как группу.
    fn inline_flags(&mut self) -> RtResult<bool> {
        if self.peek() != Some('(' as Cp) || self.peek_at(1) != Some('?' as Cp) {
            return Ok(false);
        }
        let mut ahead = 2;
        let mut icase = false;
        let mut multiline = false;
        loop {
            match self.peek_at(ahead) {
                Some(cp) if cp == 'i' as Cp => icase = true,
                Some(cp) if cp == 'm' as Cp => multiline = true,
                Some(cp) if cp == ')' as Cp && ahead > 2 => {
                    self.pos += ahead + 1;
                    // Флаги только включаются: `(?-i)` не поддержан (см.
                    // ниже), поэтому сброса тут быть не может.
                    self.flags.icase |= icase;
                    self.flags.multiline |= multiline;
                    return Ok(true);
                }
                // `(?i:...)` — групповая форма флага; `(?-i)` — сброс.
                // Обе меняют смысл шаблона, и молчаливо принять их за
                // что-то другое нельзя.
                Some(cp) if ahead > 2 => {
                    return Err(bad(format!(
                        "инлайн-флаг записан как «(?…{}…)»; поддержаны только \
                         «(?i)», «(?m)» и «(?im)»",
                        show(cp)
                    )))
                }
                _ => return Ok(false),
            }
            ahead += 1;
        }
    }

    /// Атом с необязательным квантором.
    fn quantified(&mut self) -> RtResult<Node> {
        let atom = self.atom()?;
        let Some((min, max)) = self.quantifier()? else {
            return Ok(atom);
        };
        let greedy = !self.eat('?' as Cp);
        if let Some(next) = self.peek() {
            if next == '*' as Cp || next == '+' as Cp || next == '?' as Cp {
                return Err(bad(format!(
                    "два квантора подряд («{}» после квантора); притяжательные \
                     кванторы вида «а*+» не поддержаны",
                    show(next)
                )));
            }
        }
        Ok(Node::Repeat {
            body: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    /// Квантор после атома, если он есть.
    fn quantifier(&mut self) -> RtResult<Option<(u32, Option<u32>)>> {
        let Some(cp) = self.peek() else {
            return Ok(None);
        };
        if cp == '*' as Cp {
            self.pos += 1;
            return Ok(Some((0, None)));
        }
        if cp == '+' as Cp {
            self.pos += 1;
            return Ok(Some((1, None)));
        }
        if cp == '?' as Cp {
            self.pos += 1;
            return Ok(Some((0, Some(1))));
        }
        if cp == '{' as Cp {
            return self.counted_quantifier().map(Some);
        }
        Ok(None)
    }

    /// `{n}`, `{n,}`, `{n,m}` — открывающая скобка ещё не съедена.
    fn counted_quantifier(&mut self) -> RtResult<(u32, Option<u32>)> {
        let start = self.pos;
        self.pos += 1;
        let Some(min) = self.count()? else {
            self.pos = start;
            return Err(bad(
                "после « { » ожидалось число: квантор пишется как «{n}», \
                 «{n,}» или «{n,m}», а литеральная скобка — как «\\{»",
            ));
        };
        let max = if self.eat(',' as Cp) {
            if self.peek() == Some('}' as Cp) {
                None
            } else {
                let Some(max) = self.count()? else {
                    return Err(bad("после запятой в кванторе «{n,m}» ожидалось число"));
                };
                if max < min {
                    return Err(bad(format!(
                        "в кванторе «{{{min},{max}}}» верхняя граница меньше нижней"
                    )));
                }
                Some(max)
            }
        } else {
            Some(min)
        };
        if !self.eat('}' as Cp) {
            return Err(bad("в кванторе не хватает « } »"));
        }
        Ok((min, max))
    }

    /// Десятичное число квантора.
    fn count(&mut self) -> RtResult<Option<u32>> {
        let mut digits = 0usize;
        let mut value: u64 = 0;
        while let Some(cp) = self.peek() {
            let Some(digit) = char::from_u32(cp).and_then(|c| c.to_digit(10)) else {
                break;
            };
            self.pos += 1;
            digits += 1;
            value = value * 10 + u64::from(digit);
            if value > u64::from(MAX_REPEAT) {
                return Err(bad(format!(
                    "счётчик в кванторе больше {MAX_REPEAT} — это предел этой реализации"
                )));
            }
        }
        if digits == 0 {
            return Ok(None);
        }
        // Значение уже проверено против `MAX_REPEAT`, то есть влезает в u32.
        Ok(Some(value as u32))
    }

    fn atom(&mut self) -> RtResult<Node> {
        let Some(cp) = self.peek() else {
            // Сюда попадаем только из `quantified`, которую `concat`
            // вызывает лишь при непустом остатке.
            return Err(bad("шаблон неожиданно кончился"));
        };
        if cp == '(' as Cp {
            return self.group();
        }
        if cp == '[' as Cp {
            self.pos += 1;
            let spec = self.class()?;
            return Ok(Node::Class {
                spec,
                icase: self.flags.icase,
            });
        }
        if cp == '.' as Cp {
            self.pos += 1;
            return Ok(Node::Any);
        }
        if cp == '^' as Cp {
            self.pos += 1;
            return Ok(Node::LineStart {
                multiline: self.flags.multiline,
            });
        }
        if cp == '$' as Cp {
            self.pos += 1;
            return Ok(Node::LineEnd {
                multiline: self.flags.multiline,
            });
        }
        if cp == '*' as Cp || cp == '+' as Cp || cp == '?' as Cp {
            return Err(bad(format!(
                "квантор «{}» без атома: квантовать нечего",
                show(cp)
            )));
        }
        if cp == '{' as Cp {
            // Отличаем «квантор в начале» от неэкранированной литеральной
            // скобки: разбор квантора уже умеет сказать про «\\{».
            self.counted_quantifier()?;
            return Err(bad("квантор «{…}» без атома: квантовать нечего"));
        }
        if cp == '\\' as Cp {
            self.pos += 1;
            return match self.escape(false)? {
                Escape::Cp(cp) => Ok(Node::Literal {
                    cp,
                    icase: self.flags.icase,
                }),
                Escape::Prop { kind, negated } => Ok(Node::Class {
                    spec: ClassSpec {
                        negated: false,
                        items: vec![ClassItem::Prop { kind, negated }],
                    },
                    icase: self.flags.icase,
                }),
                Escape::WordBoundary { negated } => Ok(Node::WordBoundary { negated }),
            };
        }
        self.pos += 1;
        Ok(Node::Literal {
            cp,
            icase: self.flags.icase,
        })
    }

    /// Группа: `(...)`, `(?:...)`. Инлайн-флаги сюда не доходят — их
    /// перехватывает [`Parser::inline_flags`].
    fn group(&mut self) -> RtResult<Node> {
        self.pos += 1;
        let slot = if self.peek() == Some('?' as Cp) {
            match self.peek_at(1) {
                Some(cp) if cp == ':' as Cp => {
                    self.pos += 2;
                    None
                }
                Some(cp) => {
                    return Err(bad(format!(
                        "конструкция «(?{}» не поддержана: есть только группа \
                         «(…)», незахватывающая «(?:…)» и флаги «(?i)»/«(?m)»",
                        show(cp)
                    )))
                }
                None => return Err(bad("шаблон обрывается на «(?»")),
            }
        } else {
            self.groups += 1;
            Some(self.groups)
        };
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(bad(format!(
                "группы вложены глубже {MAX_DEPTH} уровней — это предел этой реализации"
            )));
        }
        // Флаги действуют до конца ОХВАТЫВАЮЩЕЙ группы, поэтому на выходе
        // возвращается то, что было на входе.
        // `НЕ ИЗМЕРЕНО(REGEX.FLAG.SCOPE)`.
        let outer = self.flags;
        let body = self.alternation()?;
        self.flags = outer;
        self.depth -= 1;
        if !self.eat(')' as Cp) {
            return Err(bad("незакрытая группа: не хватает « ) »"));
        }
        Ok(Node::Group {
            slot,
            body: Box::new(body),
        })
    }

    /// Класс символов; `[` уже съеден.
    ///
    /// Дефис сразу ПОСЛЕ сокращения (`[\d-z]`) — обычный символ, а не
    /// начало диапазона: диапазон строится от символа к символу, а от
    /// множества строить нечего. Дефис ПЕРЕД сокращением (`[а-\d]`) —
    /// наоборот, ошибка: там уже заявлен диапазон, и второй его край
    /// обязан быть символом.
    fn class(&mut self) -> RtResult<ClassSpec> {
        let negated = self.eat('^' as Cp);
        let mut items: Vec<ClassItem> = Vec::new();
        loop {
            let Some(cp) = self.peek() else {
                return Err(bad("незакрытый класс символов: не хватает « ] »"));
            };
            if cp == ']' as Cp {
                if items.is_empty() {
                    return Err(bad(
                        "пустой класс символов; литеральная « ] » внутри класса \
                         пишется как «\\]»",
                    ));
                }
                self.pos += 1;
                return Ok(ClassSpec { negated, items });
            }
            if cp == '[' as Cp {
                return Err(bad(
                    "вложенные классы и операции над множествами не поддержаны; \
                     литеральная « [ » внутри класса пишется как «\\[»",
                ));
            }
            if cp == '&' as Cp && self.peek_at(1) == Some('&' as Cp) {
                return Err(bad(
                    "пересечение классов «&&» не поддержано; литеральный «&» \
                     пишется одиночным",
                ));
            }
            let low = if cp == '\\' as Cp {
                self.pos += 1;
                match self.escape(true)? {
                    Escape::Cp(cp) => Some(cp),
                    Escape::Prop { kind, negated } => {
                        items.push(ClassItem::Prop { kind, negated });
                        None
                    }
                    // Внутрь класса `\b`/`\B` не пропускает `escape`.
                    Escape::WordBoundary { .. } => None,
                }
            } else {
                self.pos += 1;
                Some(cp)
            };
            let Some(low) = low else {
                continue;
            };
            // Диапазон, только если после дефиса есть что-то кроме « ] »:
            // в `[а-]` дефис — обычный символ.
            let is_range =
                self.peek() == Some('-' as Cp) && self.peek_at(1).is_some_and(|cp| cp != ']' as Cp);
            if !is_range {
                items.push(ClassItem::Single(low));
                continue;
            }
            self.pos += 1;
            let high = match self.peek() {
                Some(cp) if cp == '\\' as Cp => {
                    self.pos += 1;
                    match self.escape(true)? {
                        Escape::Cp(cp) => cp,
                        Escape::Prop { .. } | Escape::WordBoundary { .. } => {
                            return Err(bad("границей диапазона в классе может быть только \
                                 символ, а не сокращение вида «\\d»"))
                        }
                    }
                }
                Some(cp) => {
                    self.pos += 1;
                    cp
                }
                None => return Err(bad("незакрытый класс символов: не хватает « ] »")),
            };
            if high < low {
                return Err(bad(format!(
                    "перевёрнутый диапазон «{}-{}» в классе символов",
                    show(low),
                    show(high)
                )));
            }
            items.push(ClassItem::Range(low, high));
        }
    }

    /// Экранирование; сама « \ » уже съедена.
    fn escape(&mut self, in_class: bool) -> RtResult<Escape> {
        let Some(cp) = self.bump() else {
            return Err(bad("шаблон обрывается на « \\ »"));
        };
        let Some(c) = char::from_u32(cp) else {
            // Экранированный непарный суррогат — просто он сам.
            return Ok(Escape::Cp(cp));
        };
        let prop = |kind, negated| Ok(Escape::Prop { kind, negated });
        match c {
            'd' => prop(PropKind::Digit, false),
            'D' => prop(PropKind::Digit, true),
            'w' => prop(PropKind::Word, false),
            'W' => prop(PropKind::Word, true),
            's' => prop(PropKind::Space, false),
            'S' => prop(PropKind::Space, true),
            'p' | 'P' => {
                let kind = self.unicode_property()?;
                prop(kind, c == 'P')
            }
            'b' | 'B' if in_class => Err(bad(
                "граница слова «\\b» внутри класса символов не имеет смысла",
            )),
            'b' => Ok(Escape::WordBoundary { negated: false }),
            'B' => Ok(Escape::WordBoundary { negated: true }),
            'n' => Ok(Escape::Cp(CP_LF)),
            'r' => Ok(Escape::Cp(CP_CR)),
            't' => Ok(Escape::Cp(CP_TAB)),
            'f' => Ok(Escape::Cp(CP_FF)),
            'a' => Ok(Escape::Cp(0x07)),
            'e' => Ok(Escape::Cp(0x1B)),
            'u' => self.hex_escape().map(Escape::Cp),
            '0'..='9' => Err(bad(format!(
                "экранирование «\\{c}» не поддержано: обратных ссылок на группы \
                 и восьмеричных кодов здесь нет"
            ))),
            // Экранированная пунктуация — она сама: `\.`, `\\`, `\[`, `\{`.
            _ if !c.is_ascii_alphanumeric() => Ok(Escape::Cp(cp)),
            _ => Err(bad(format!("экранирование «\\{c}» не поддержано"))),
        }
    }

    /// Имя свойства после `\p`/`\P` — только в фигурных скобках.
    fn unicode_property(&mut self) -> RtResult<PropKind> {
        if !self.eat('{' as Cp) {
            return Err(bad(
                "имя свойства Unicode пишется в скобках: «\\p{L}», «\\p{Nd}»",
            ));
        }
        let mut name = String::new();
        loop {
            let Some(cp) = self.bump() else {
                return Err(bad("в записи свойства Unicode не хватает « } »"));
            };
            if cp == '}' as Cp {
                break;
            }
            match char::from_u32(cp) {
                Some(c) => name.push(c),
                None => return Err(bad("в имени свойства Unicode посторонний символ")),
            }
        }
        match name.as_str() {
            "L" => Ok(PropKind::Letter),
            "Nd" => Ok(PropKind::Digit),
            other => Err(bad(format!(
                "свойство Unicode «{other}» не поддержано: есть только L и Nd"
            ))),
        }
    }

    /// `\uXXXX` — ровно четыре шестнадцатеричные цифры.
    ///
    /// Пара `😀` собирается в одну кодовую точку: в тексте
    /// шаблона суррогатная пара уже собрана декодером, и записанная
    /// экранированием она обязана значить то же самое.
    fn hex_escape(&mut self) -> RtResult<Cp> {
        let high = self.hex4()?;
        if (0xD800..0xDC00).contains(&high)
            && self.peek() == Some('\\' as Cp)
            && self.peek_at(1) == Some('u' as Cp)
        {
            let save = self.pos;
            self.pos += 2;
            match self.hex4() {
                Ok(low) if (0xDC00..0xE000).contains(&low) => {
                    return Ok(0x1_0000 + ((high - 0xD800) << 10) + (low - 0xDC00));
                }
                // Не низкий суррогат — откатываемся: это отдельное
                // экранирование, оно разберётся своим чередом.
                Ok(_) | Err(_) => self.pos = save,
            }
        }
        Ok(high)
    }

    fn hex4(&mut self) -> RtResult<Cp> {
        let mut value: Cp = 0;
        for _ in 0..4 {
            let Some(digit) = self
                .peek()
                .and_then(char::from_u32)
                .and_then(|c| c.to_digit(16))
            else {
                return Err(bad("после «\\u» ожидались четыре шестнадцатеричные цифры"));
            };
            self.pos += 1;
            value = value * 16 + digit;
        }
        Ok(value)
    }
}

// --- программа ----------------------------------------------------------

/// Инструкция плоской программы. Адреса переходов — АБСОЛЮТНЫЕ индексы,
/// как в байт-коде этого проекта.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Instr {
    /// Литерал; под `(?i)` в `cp` лежит уже свёрнутая точка.
    Char {
        cp: Cp,
        icase: bool,
    },
    /// Класс по индексу в таблице классов программы.
    Class(usize),
    /// `.`
    Any,
    /// Развилка: продолжить с `prefer`, а `alt` положить в стек
    /// бэктрекинга. Жадность и леность — это порядок этих двух полей.
    Split {
        prefer: usize,
        alt: usize,
    },
    Jump(usize),
    /// Записать текущую позицию в регистр (границы групп).
    Save(usize),
    /// Обнулить регистр-метку перед входом в цикл.
    ClearMark(usize),
    /// Страж пустой итерации: если метка уже равна текущей позиции —
    /// предыдущий проход ничего не съел, повторять нельзя.
    Guard(usize),
    /// `^`
    LineStart {
        multiline: bool,
    },
    /// `$`
    LineEnd {
        multiline: bool,
    },
    /// `\b` и `\B`
    WordBoundary {
        negated: bool,
    },
    /// Совпадение найдено.
    Match,
}

#[derive(Clone, Debug)]
struct CompiledClass {
    spec: ClassSpec,
    icase: bool,
}

struct Compiler {
    prog: Vec<Instr>,
    classes: Vec<CompiledClass>,
    /// Мемоизация «один узел класса — одна запись таблицы»: адрес
    /// [`ClassSpec`] в дереве разбора → индекс в `classes`.
    ///
    /// Ключ — именно адрес, а не содержимое: дерево живёт всю компиляцию и
    /// не мутирует, поэтому адреса стабильны и различны у разных узлов, а
    /// сравнение по содержимому стоило бы обхода всего класса. Смысл — в
    /// том, что [`Compiler::repeat`] обходит ОДИН И ТОТ ЖЕ `&Node` столько
    /// раз, сколько копий тела разворачивает: без мемоизации `[абв]{20000}`
    /// клал бы в таблицу двадцать тысяч клонов, и память таблицы росла бы
    /// произведением «размер класса × число копий» мимо [`MAX_PROGRAM`],
    /// который считает только инструкции. Флаг `icase` лежит в самом узле,
    /// поэтому один узел всегда компилируется в одну и ту же запись.
    class_index: HashMap<usize, usize>,
    /// Следующий свободный регистр: сперва по два на группу, дальше метки
    /// стражей пустого цикла.
    next_reg: usize,
}

impl Compiler {
    fn emit(&mut self, instr: Instr) -> RtResult<usize> {
        if self.prog.len() >= MAX_PROGRAM {
            return Err(bad(format!(
                "шаблон разворачивается больше чем в {MAX_PROGRAM} инструкций — \
                 это предел этой реализации"
            )));
        }
        self.prog.push(instr);
        Ok(self.prog.len() - 1)
    }

    fn here(&self) -> usize {
        self.prog.len()
    }

    fn new_mark(&mut self) -> usize {
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }

    /// Дописать развилку на место заглушки: `taken` — ветвь «повторить или
    /// войти», `skipped` — «пропустить». Жадный квантор предпочитает
    /// первую, ленивый вторую.
    fn set_split(&mut self, at: usize, greedy: bool, taken: usize, skipped: usize) {
        let instr = if greedy {
            Instr::Split {
                prefer: taken,
                alt: skipped,
            }
        } else {
            Instr::Split {
                prefer: skipped,
                alt: taken,
            }
        };
        if let Some(slot) = self.prog.get_mut(at) {
            *slot = instr;
        }
    }

    fn node(&mut self, node: &Node) -> RtResult<()> {
        match node {
            Node::Empty => Ok(()),
            Node::Literal { cp, icase } => {
                let cp = if *icase { simple_lower(*cp) } else { *cp };
                self.emit(Instr::Char { cp, icase: *icase })?;
                Ok(())
            }
            Node::Class { spec, icase } => {
                let key = std::ptr::from_ref(spec) as usize;
                let index = match self.class_index.get(&key) {
                    Some(index) => *index,
                    None => {
                        self.classes.push(CompiledClass {
                            spec: spec.clone(),
                            icase: *icase,
                        });
                        let index = self.classes.len() - 1;
                        self.class_index.insert(key, index);
                        index
                    }
                };
                self.emit(Instr::Class(index))?;
                Ok(())
            }
            Node::Any => {
                self.emit(Instr::Any)?;
                Ok(())
            }
            Node::LineStart { multiline } => {
                self.emit(Instr::LineStart {
                    multiline: *multiline,
                })?;
                Ok(())
            }
            Node::LineEnd { multiline } => {
                self.emit(Instr::LineEnd {
                    multiline: *multiline,
                })?;
                Ok(())
            }
            Node::WordBoundary { negated } => {
                self.emit(Instr::WordBoundary { negated: *negated })?;
                Ok(())
            }
            Node::Group { slot, body } => {
                if let Some(slot) = slot {
                    self.emit(Instr::Save(slot * 2))?;
                    self.node(body)?;
                    self.emit(Instr::Save(slot * 2 + 1))?;
                } else {
                    self.node(body)?;
                }
                Ok(())
            }
            Node::Concat(parts) => {
                for part in parts {
                    self.node(part)?;
                }
                Ok(())
            }
            Node::Alt(branches) => self.alt(branches),
            Node::Repeat {
                body,
                min,
                max,
                greedy,
            } => self.repeat(body, *min, *max, *greedy),
        }
    }

    fn alt(&mut self, branches: &[Node]) -> RtResult<()> {
        let mut ends = Vec::new();
        for (i, branch) in branches.iter().enumerate() {
            if i + 1 == branches.len() {
                // Последняя ветвь идёт без развилки: если не подошла она,
                // не подошла вся альтернация.
                self.node(branch)?;
                break;
            }
            let split = self.emit(Instr::Jump(0))?;
            self.node(branch)?;
            ends.push(self.emit(Instr::Jump(0))?);
            let next = self.here();
            self.set_split(split, true, split + 1, next);
        }
        let end = self.here();
        for at in ends {
            if let Some(slot) = self.prog.get_mut(at) {
                *slot = Instr::Jump(end);
            }
        }
        Ok(())
    }

    /// Повторение.
    ///
    /// Неограниченное сверху разворачивается в ОДНУ копию тела в цикле
    /// (плюс `min - 1` копий перед ним у `{n,}`), ограниченное — в `min`
    /// обязательных копий и `max - min` копий под развилками. Копирование
    /// тела — плата за плоскую программу без счётчиков в рантайме; предел
    /// [`MAX_PROGRAM`] держит её конечной.
    ///
    /// Страж пустой итерации ([`Instr::ClearMark`] перед цепочкой копий и
    /// [`Instr::Guard`] в голове каждой необязательной копии) стоит в ОБЕИХ
    /// ветвях, но отвечает в них за разное. В неограниченной — за
    /// завершимость: без него `(а*)*` крутился бы вечно. В ограниченной
    /// число проходов конечно и без стража, там он отвечает за ПОРЯДОК
    /// ЗАХВАТОВ. Проход, начавшийся ровно там, где начался предыдущий,
    /// означает, что предыдущий совпал пустотой; он обрывает цепочку,
    /// поэтому текст съедает РАННЯЯ итерация, а последней остаётся пустая
    /// правее. Без обрыва бэктрекинг доходил бы до последней копии и текст
    /// доставался бы ей: `(б{0,1}?){0,2}в` на «бв» дал бы группу 1 = `0,1`
    /// вместо `1,1`. Из четырёх измеренных диалектов `1,1` — ответ
    /// `python3 re`, выбранного здесь эталоном; классики расходятся на этом
    /// месте вчетвером, и вопрос платформе записан как
    /// `REGEX.REPEAT.EMPTY`. Обязательные копии `0..min` идут без стража:
    /// они не необязательны, обрывать цепочку в них нечего.
    fn repeat(&mut self, body: &Node, min: u32, max: Option<u32>, greedy: bool) -> RtResult<()> {
        match max {
            None => {
                for _ in 0..min.saturating_sub(1) {
                    self.node(body)?;
                }
                let mark = self.new_mark();
                self.emit(Instr::ClearMark(mark))?;
                let start = self.here();
                if min == 0 {
                    let split = self.emit(Instr::Jump(0))?;
                    self.emit(Instr::Guard(mark))?;
                    self.node(body)?;
                    self.emit(Instr::Jump(start))?;
                    let end = self.here();
                    self.set_split(split, greedy, split + 1, end);
                } else {
                    self.emit(Instr::Guard(mark))?;
                    self.node(body)?;
                    let split = self.emit(Instr::Jump(0))?;
                    let end = self.here();
                    self.set_split(split, greedy, start, end);
                }
            }
            Some(max) => {
                for _ in 0..min {
                    self.node(body)?;
                }
                if min < max {
                    // Точный `{n}` сюда не заходит: необязательных копий у
                    // него нет, а метка без стражей была бы мёртвой.
                    //
                    // НЕ ИЗМЕРЕНО(REGEX.REPEAT.EMPTY): здесь страж отвечает
                    // не за завершимость, а за то, какой итерации достаётся
                    // захват при пустом теле, и на этом месте классические
                    // движки расходятся вчетвером.
                    let mark = self.new_mark();
                    self.emit(Instr::ClearMark(mark))?;
                    let mut splits = Vec::new();
                    for _ in min..max {
                        splits.push(self.emit(Instr::Jump(0))?);
                        self.emit(Instr::Guard(mark))?;
                        self.node(body)?;
                    }
                    let end = self.here();
                    for at in splits {
                        self.set_split(at, greedy, at + 1, end);
                    }
                }
            }
        }
        Ok(())
    }
}

// --- готовое выражение --------------------------------------------------

/// Найденное совпадение: границы групп в КОД-ЮНИТАХ UTF-16.
///
/// Нулевая группа — всё совпадение, дальше по номерам открывающих скобок.
/// `None` — группа не участвовала (это не то же, что участвовала и пуста).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) spans: Vec<Option<(usize, usize)>>,
}

/// Скомпилированное регулярное выражение.
pub(crate) struct Regex {
    prog: Vec<Instr>,
    classes: Vec<CompiledClass>,
    /// Число групп ВМЕСТЕ с нулевой.
    group_count: usize,
    /// Сколько регистров нужно исполнителю: границы групп плюс метки.
    reg_count: usize,
}

/// Точка возврата бэктрекинга.
struct Backtrack {
    pc: usize,
    pos: usize,
    /// Длина журнала отмены на момент развилки: всё, что записано позже,
    /// откатывается.
    undo: usize,
}

/// Что делать после инструкции.
enum Step {
    /// Съедено столько код-юнитов, дальше следующая инструкция.
    Advance(usize),
    Next,
    Goto(usize),
    Done,
    Fail,
}

impl Regex {
    /// Разобрать и скомпилировать шаблон.
    ///
    /// # Errors
    ///
    /// [`RtError::Regex`] на любой ошибке разбора: незакрытая группа или
    /// класс, перевёрнутый диапазон, квантор без атома или с верхней
    /// границей меньше нижней, неподдержанная конструкция (просмотр
    /// вперёд, обратная ссылка, операции над множествами), а также на
    /// превышении пределов реализации — вложенности [`MAX_DEPTH`],
    /// счётчика [`MAX_REPEAT`] и длины программы [`MAX_PROGRAM`].
    pub(crate) fn parse(pattern: &[u16]) -> RtResult<Regex> {
        let src = decode(pattern);
        let mut parser = Parser::new(&src);
        let tree = parser.alternation()?;
        if parser.pos < src.len() {
            // Сюда приводит только лишняя закрывающая скобка: остальное
            // конкатенация съедает сама.
            return Err(bad("лишняя « ) » в шаблоне"));
        }
        let group_count = parser.groups + 1;
        let mut compiler = Compiler {
            prog: Vec::new(),
            classes: Vec::new(),
            class_index: HashMap::new(),
            next_reg: group_count * 2,
        };
        compiler.emit(Instr::Save(0))?;
        compiler.node(&tree)?;
        compiler.emit(Instr::Save(1))?;
        compiler.emit(Instr::Match)?;
        Ok(Regex {
            prog: compiler.prog,
            classes: compiler.classes,
            group_count,
            reg_count: compiler.next_reg,
        })
    }

    /// Сколько групп у выражения, считая нулевую.
    pub(crate) fn group_count(&self) -> usize {
        self.group_count
    }

    /// Первое совпадение, начинающееся не левее `start`.
    ///
    /// Позиции — код-юниты UTF-16. `start` ожидается на границе кодовой
    /// точки; если он попал в середину суррогатной пары, низкий суррогат
    /// читается как непарный — так же, как читался бы в любой другой
    /// позиции.
    pub(crate) fn find_at(&self, haystack: &[u16], start: usize) -> Option<Match> {
        if start > haystack.len() {
            return None;
        }
        let mut regs: Vec<Option<usize>> = vec![None; self.reg_count];
        let mut undo: Vec<(usize, Option<usize>)> = Vec::new();
        let mut stack: Vec<Backtrack> = Vec::new();
        let mut pos = start;
        loop {
            if let Some(found) = self.run_at(haystack, pos, &mut regs, &mut undo, &mut stack) {
                return Some(found);
            }
            match cp_at(haystack, pos) {
                // Шаг поиска — по кодовой точке, а не по код-юниту: начало
                // совпадения не должно попадать в середину пары.
                Some((_, width)) => pos += width,
                None => return None,
            }
        }
    }

    /// Попытка совпадения, НАЧИНАЮЩЕГОСЯ ровно в `start`.
    fn run_at(
        &self,
        hay: &[u16],
        start: usize,
        regs: &mut [Option<usize>],
        undo: &mut Vec<(usize, Option<usize>)>,
        stack: &mut Vec<Backtrack>,
    ) -> Option<Match> {
        regs.iter_mut().for_each(|slot| *slot = None);
        undo.clear();
        stack.clear();
        let mut pc = 0usize;
        let mut pos = start;
        loop {
            let Some(instr) = self.prog.get(pc) else {
                debug_assert!(false, "счётчик инструкций вышел за программу");
                return None;
            };
            let step = match *instr {
                Instr::Char { cp, icase } => match cp_at(hay, pos) {
                    Some((got, width)) => {
                        let got = if icase { simple_lower(got) } else { got };
                        if got == cp {
                            Step::Advance(width)
                        } else {
                            Step::Fail
                        }
                    }
                    None => Step::Fail,
                },
                Instr::Class(index) => match (cp_at(hay, pos), self.classes.get(index)) {
                    (Some((got, width)), Some(class)) if class.spec.matches(got, class.icase) => {
                        Step::Advance(width)
                    }
                    _ => Step::Fail,
                },
                Instr::Any => match cp_at(hay, pos) {
                    Some((got, width)) if !is_line_terminator(got) => Step::Advance(width),
                    _ => Step::Fail,
                },
                Instr::Split { prefer, alt } => {
                    stack.push(Backtrack {
                        pc: alt,
                        pos,
                        undo: undo.len(),
                    });
                    Step::Goto(prefer)
                }
                Instr::Jump(target) => Step::Goto(target),
                Instr::Save(reg) => {
                    if write_reg(regs, undo, reg, Some(pos)) {
                        Step::Next
                    } else {
                        debug_assert!(false, "инструкция сослалась на чужой регистр");
                        return None;
                    }
                }
                Instr::ClearMark(reg) => {
                    if write_reg(regs, undo, reg, None) {
                        Step::Next
                    } else {
                        debug_assert!(false, "инструкция сослалась на чужой регистр");
                        return None;
                    }
                }
                Instr::Guard(reg) => match regs.get(reg).copied() {
                    // Итерация, начавшаяся там же, где предыдущая, ничего
                    // не съела — повторять её нельзя.
                    Some(mark) if mark == Some(pos) => Step::Fail,
                    Some(_) => {
                        write_reg(regs, undo, reg, Some(pos));
                        Step::Next
                    }
                    None => {
                        debug_assert!(false, "инструкция сослалась на чужой регистр");
                        return None;
                    }
                },
                Instr::LineStart { multiline } => {
                    if at_line_start(hay, pos, multiline) {
                        Step::Next
                    } else {
                        Step::Fail
                    }
                }
                Instr::LineEnd { multiline } => {
                    if at_line_end(hay, pos, multiline) {
                        Step::Next
                    } else {
                        Step::Fail
                    }
                }
                Instr::WordBoundary { negated } => {
                    let before = cp_before(hay, pos).is_some_and(|(cp, _)| is_word_cp(cp));
                    let after = cp_at(hay, pos).is_some_and(|(cp, _)| is_word_cp(cp));
                    if (before != after) != negated {
                        Step::Next
                    } else {
                        Step::Fail
                    }
                }
                Instr::Match => Step::Done,
            };
            match step {
                Step::Advance(width) => {
                    pos += width;
                    pc += 1;
                }
                Step::Next => pc += 1,
                Step::Goto(target) => pc = target,
                Step::Done => {
                    let spans = (0..self.group_count)
                        .map(|group| {
                            let from = regs.get(group * 2).copied().flatten();
                            let to = regs.get(group * 2 + 1).copied().flatten();
                            from.zip(to)
                        })
                        .collect();
                    return Some(Match { spans });
                }
                Step::Fail => {
                    let point = stack.pop()?;
                    while undo.len() > point.undo {
                        let Some((reg, previous)) = undo.pop() else {
                            break;
                        };
                        if let Some(slot) = regs.get_mut(reg) {
                            *slot = previous;
                        }
                    }
                    pc = point.pc;
                    pos = point.pos;
                }
            }
        }
    }
}

/// Записать регистр, занеся прежнее значение в журнал отмены.
///
/// `false` — регистра с таким номером нет; корректная программа такого не
/// порождает, поэтому исполнитель на этом останавливается, а не молчит.
fn write_reg(
    regs: &mut [Option<usize>],
    undo: &mut Vec<(usize, Option<usize>)>,
    reg: usize,
    value: Option<usize>,
) -> bool {
    match regs.get_mut(reg) {
        Some(slot) => {
            undo.push((reg, *slot));
            *slot = value;
            true
        }
        None => false,
    }
}

/// Стоит ли позиция в начале строки текста.
///
/// Без `(?m)` — только самое начало входа. С `(?m)` — ещё и сразу после
/// конца строки, но НЕ между `\r` и `\n` (пара — один конец строки) и не в
/// самом конце входа: хвостовой перевод строки новой строки не открывает.
/// `НЕ ИЗМЕРЕНО(REGEX.ANCHOR.EOL)`.
fn at_line_start(hay: &[u16], pos: usize, multiline: bool) -> bool {
    if pos == 0 {
        return true;
    }
    if !multiline || pos >= hay.len() {
        return false;
    }
    let Some((previous, _)) = cp_before(hay, pos) else {
        return false;
    };
    if !is_line_terminator(previous) {
        return false;
    }
    !(previous == CP_CR && cp_at(hay, pos).map(|(cp, _)| cp) == Some(CP_LF))
}

/// Стоит ли позиция в конце строки текста.
///
/// С `(?m)` — перед любым концом строки и в конце входа. Без `(?m)` — в
/// конце входа и перед ХВОСТОВЫМ концом строки (`\r\n` считается одним),
/// то есть `«аб\n»` шаблоном `б$` находится. `НЕ ИЗМЕРЕНО(REGEX.ANCHOR.EOL)`.
fn at_line_end(hay: &[u16], pos: usize, multiline: bool) -> bool {
    let Some((cp, width)) = cp_at(hay, pos) else {
        return true;
    };
    if !is_line_terminator(cp) {
        return false;
    }
    // Внутри пары `\r\n` конца строки нет.
    if cp == CP_LF && cp_before(hay, pos).map(|(prev, _)| prev) == Some(CP_CR) {
        return false;
    }
    if multiline {
        return true;
    }
    let tail = if cp == CP_CR && cp_at(hay, pos + 1).map(|(next, _)| next) == Some(CP_LF) {
        pos + 2
    } else {
        pos + width
    };
    tail == hay.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn compile(pattern: &str) -> Regex {
        match Regex::parse(&utf16(pattern)) {
            Ok(regex) => regex,
            Err(e) => panic!("шаблон «{pattern}» не разобрался: {e}"),
        }
    }

    /// Текст нулевой группы или `None`.
    fn find(pattern: &str, text: &str) -> Option<String> {
        group(pattern, text, 0)
    }

    /// Текст группы `n` первого совпадения.
    fn group(pattern: &str, text: &str, n: usize) -> Option<String> {
        let hay = utf16(text);
        let found = compile(pattern).find_at(&hay, 0)?;
        let (from, to) = *found.spans.get(n)?.as_ref()?;
        Some(String::from_utf16_lossy(hay.get(from..to)?))
    }

    /// Границы всех групп совпадения.
    fn spans(pattern: &str, text: &str) -> Vec<Option<(usize, usize)>> {
        let hay = utf16(text);
        match compile(pattern).find_at(&hay, 0) {
            Some(found) => found.spans,
            None => panic!("шаблон «{pattern}» не нашёлся в «{text}»"),
        }
    }

    /// Текст ошибки разбора; шаблон, который разобрался, — провал теста.
    fn parse_error(pattern: &str) -> String {
        match Regex::parse(&utf16(pattern)) {
            Ok(_) => panic!("шаблон «{pattern}» разобрался, а должен был не суметь"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn literals_and_escapes_match_themselves() {
        assert_eq!(find("абв", "ххабвуу").as_deref(), Some("абв"));
        assert_eq!(find("абв", "ххабуу"), None);
        assert_eq!(find(r"а\.б", "а.б").as_deref(), Some("а.б"));
        assert_eq!(find(r"а\.б", "ахб"), None);
        assert_eq!(find(r"\\", r"а\б").as_deref(), Some(r"\"));
        assert_eq!(find(r"а\nб", "а\nб").as_deref(), Some("а\nб"));
        assert_eq!(find(r"а\tб", "а\tб").as_deref(), Some("а\tб"));
        assert_eq!(find(r"а\rб", "а\rб").as_deref(), Some("а\rб"));
        assert_eq!(find(r"ё", "Ёё").as_deref(), Some("ё"));
        assert_eq!(find(r"ё", "Ё"), None);
        // Экранированная пунктуация — она сама, экранированная буква без
        // смысла — ошибка, а не молчаливый литерал.
        assert_eq!(find(r"\{", "а{б").as_deref(), Some("{"));
        assert!(parse_error(r"\q").contains("не поддержано"));
    }

    #[test]
    fn a_dot_takes_any_character_but_a_line_terminator() {
        assert_eq!(find("а.в", "абв").as_deref(), Some("абв"));
        assert_eq!(find("а.в", "а\nв"), None);
        assert_eq!(find("а.в", "а\rв"), None);
        assert_eq!(find("а.в", "ав"), None);
    }

    #[test]
    fn classes_take_members_and_ranges() {
        assert_eq!(find("[абв]+", "ггабваг").as_deref(), Some("абва"));
        assert_eq!(find("[абв]", "гдд"), None);
        assert_eq!(find("[а-я]+", "12абв34").as_deref(), Some("абв"));
        assert_eq!(find("[^а-я]+", "абв12абв").as_deref(), Some("12"));
        assert_eq!(find("[^а-я]", "абв"), None);
        // Дефис по краям класса — обычный символ.
        assert_eq!(find("[-а]+", "б-аб").as_deref(), Some("-а"));
        assert_eq!(find("[а-]+", "б-аб").as_deref(), Some("-а"));
        // Вложенные сокращения внутри класса.
        assert_eq!(find(r"[\d\s]+", "аб 12в").as_deref(), Some(" 12"));
        assert_eq!(find(r"[\d]", "абв"), None);
    }

    #[test]
    fn shorthand_classes_and_their_negations() {
        assert_eq!(find(r"\d+", "аб123вг").as_deref(), Some("123"));
        assert_eq!(find(r"\d", "абвг"), None);
        assert_eq!(find(r"\D+", "12аб34").as_deref(), Some("аб"));
        assert_eq!(find(r"\w+", " -абв1_ ").as_deref(), Some("абв1_"));
        assert_eq!(find(r"\w", " - "), None);
        assert_eq!(find(r"\W+", "аб!? вг").as_deref(), Some("!? "));
        assert_eq!(find(r"\s+", "аб \t вг").as_deref(), Some(" \t "));
        assert_eq!(find(r"\s", "абвг"), None);
        assert_eq!(find(r"\S+", "  абв  ").as_deref(), Some("абв"));
    }

    #[test]
    fn unicode_properties_follow_the_general_category() {
        // `Ё` — буква, `٣` — десятичная цифра, `Ⅻ` (Nl) и `²` (No)
        // десятичными цифрами НЕ считаются.
        assert_eq!(find(r"\p{L}+", "12Ёё34").as_deref(), Some("Ёё"));
        assert_eq!(find(r"\p{L}", "123"), None);
        assert_eq!(find(r"\p{Nd}+", "аб٣4вг").as_deref(), Some("٣4"));
        assert_eq!(find(r"\p{Nd}", "Ⅻ²"), None);
        assert_eq!(find(r"\P{L}+", "аб12вг").as_deref(), Some("12"));
        assert_eq!(find(r"\P{Nd}+", "12Ⅻ²34").as_deref(), Some("Ⅻ²"));
        assert!(parse_error(r"\p{Lu}").contains("только L и Nd"));
        assert!(parse_error(r"\pL").contains("в скобках"));
    }

    #[test]
    fn word_boundaries_sit_between_word_and_punctuation() {
        assert_eq!(find(r"\bбор", "у бор").as_deref(), Some("бор"));
        assert_eq!(find(r"\bбор", "забор"), None);
        assert_eq!(find(r"бор\b", "бор, сосна").as_deref(), Some("бор"));
        assert_eq!(find(r"бор\b", "борный"), None);
        // Кириллица — символы слова, кавычка и запятая — нет.
        assert_eq!(find(r"\b\w+\b", "«лес», бор").as_deref(), Some("лес"));
        assert_eq!(find(r"\Bор", "бор").as_deref(), Some("ор"));
        assert_eq!(find(r"\Bбор", "у бор"), None);
        assert!(parse_error(r"[\b]").contains("не имеет смысла"));
    }

    #[test]
    fn anchors_hold_the_ends_of_the_text() {
        assert_eq!(find("^аб", "абв").as_deref(), Some("аб"));
        assert_eq!(find("^аб", "ваб"), None);
        assert_eq!(find("бв$", "абв").as_deref(), Some("бв"));
        assert_eq!(find("бв$", "абвг"), None);
        assert_eq!(find("^$", "").as_deref(), Some(""));
        // Без `(?m)` якоря не видят внутренних переводов строки, но `$`
        // достаёт до хвостового.
        assert_eq!(find("^бв", "а\nбв"), None);
        assert_eq!(find("аб$", "аб\n").as_deref(), Some("аб"));
    }

    #[test]
    fn the_multiline_flag_moves_the_anchors_to_every_line() {
        assert_eq!(find("(?m)^бв", "а\nбв").as_deref(), Some("бв"));
        assert_eq!(find("(?m)^бв", "а бв"), None);
        assert_eq!(find("(?m)аб$", "аб\nвг").as_deref(), Some("аб"));
        // Пара `\r\n` — ОДИН конец строки: между ними ни конца, ни начала.
        assert_eq!(find("(?m)^аб", "\r\nаб").as_deref(), Some("аб"));
        assert_eq!(find("(?m)аб$", "аб\r\n").as_deref(), Some("аб"));
    }

    #[test]
    fn quantifiers_count_repetitions() {
        assert_eq!(find("аб*", "ваббб").as_deref(), Some("аббб"));
        assert_eq!(find("аб*", "ва").as_deref(), Some("а"));
        assert_eq!(find("аб+", "ааббб").as_deref(), Some("аббб"));
        assert_eq!(find("аб+", "ва"), None);
        assert_eq!(find("аб?в", "абв").as_deref(), Some("абв"));
        assert_eq!(find("аб?в", "ав").as_deref(), Some("ав"));
        assert_eq!(find("б{3}", "аббббб").as_deref(), Some("ббб"));
        assert_eq!(find("б{3}", "абб"), None);
        assert_eq!(find("б{2,}", "аббббб").as_deref(), Some("ббббб"));
        assert_eq!(find("б{2,}", "аб"), None);
        assert_eq!(find("б{2,3}", "аббббб").as_deref(), Some("ббб"));
        assert_eq!(find("б{2,3}", "аб"), None);
    }

    #[test]
    fn a_lazy_quantifier_takes_the_shortest_match() {
        assert_eq!(find("<.+>", "<а> и <б>").as_deref(), Some("<а> и <б>"));
        assert_eq!(find("<.+?>", "<а> и <б>").as_deref(), Some("<а>"));
        assert_eq!(find("а.*в", "авав").as_deref(), Some("авав"));
        assert_eq!(find("а.*?в", "авав").as_deref(), Some("ав"));
        assert_eq!(find("б{2,3}?", "ббббб").as_deref(), Some("бб"));
        assert_eq!(find("б+?", "ббб").as_deref(), Some("б"));
    }

    #[test]
    fn groups_are_numbered_by_their_opening_parenthesis() {
        // «первая, затем её вложенные, затем вторая» — порядок из плана.
        assert_eq!(
            spans("((а)(б))(в)", "абв"),
            vec![
                Some((0, 3)),
                Some((0, 2)),
                Some((0, 1)),
                Some((1, 2)),
                Some((2, 3)),
            ]
        );
        // Незахватывающая группа номера не занимает.
        assert_eq!(spans("(?:а)(б)", "аб"), vec![Some((0, 2)), Some((1, 2))]);
        assert_eq!(group("(а)(б)", "аб", 2).as_deref(), Some("б"));
    }

    #[test]
    fn a_group_that_did_not_take_part_has_no_value() {
        // Не участвовала — `None`; участвовала и пуста — `Some` нулевой
        // длины. Это разные вещи, и путать их нельзя.
        assert_eq!(spans("а(б)?в", "ав"), vec![Some((0, 2)), None]);
        assert_eq!(spans("а(б*)в", "ав"), vec![Some((0, 2)), Some((1, 1))]);
        // Не подошедшая ветвь альтернации следов не оставляет.
        assert_eq!(
            spans("(а)|(б)", "б"),
            vec![Some((0, 1)), None, Some((0, 1))]
        );
    }

    #[test]
    fn a_repeated_group_keeps_what_its_iterations_wrote() {
        // Повторение НЕ обнуляет группы перед новым проходом: каждая
        // помнит ту итерацию, в которой участвовала последний раз.
        assert_eq!(
            spans("(?:(а)|(б))+", "аб"),
            vec![Some((0, 2)), Some((0, 1)), Some((1, 2))]
        );
        // У группы под квантором остаётся ПОСЛЕДНЯЯ итерация, а не первая.
        assert_eq!(spans("(аб)+", "абаб"), vec![Some((0, 4)), Some((2, 4))]);
    }

    #[test]
    fn alternation_prefers_the_leftmost_branch() {
        assert_eq!(find("аб|абв", "абв").as_deref(), Some("аб"));
        assert_eq!(find("абв|аб", "абв").as_deref(), Some("абв"));
        assert_eq!(find("а|б", "вба").as_deref(), Some("б"));
        assert_eq!(find("(?:г|д)", "абв"), None);
        // Пустая ветвь — законная и совпадает с пустотой.
        assert_eq!(find("а|", "б").as_deref(), Some(""));
    }

    #[test]
    fn the_search_is_leftmost() {
        assert_eq!(spans("б+", "аббаббб")[0], Some((1, 3)));
        // Пустое совпадение находится в самой левой позиции.
        assert_eq!(spans("б*", "аб")[0], Some((0, 0)));
        let hay = utf16("аббаббб");
        let regex = compile("б+");
        // Со сдвигом — первое совпадение правее старта.
        assert_eq!(
            regex.find_at(&hay, 3).and_then(|m| m.spans[0]),
            Some((4, 7))
        );
        assert_eq!(regex.find_at(&hay, hay.len()), None);
        assert_eq!(regex.find_at(&hay, hay.len() + 5), None);
    }

    #[test]
    fn the_case_insensitive_flag_folds_cyrillic() {
        assert_eq!(find("(?i)абв", "АБВ").as_deref(), Some("АБВ"));
        assert_eq!(find("(?i)АБВ", "абв").as_deref(), Some("абв"));
        assert_eq!(find("абв", "АБВ"), None);
        // Пара «ё»/«Ё» сворачивается одинарным отображением.
        assert_eq!(find("(?i)ёж", "Ёж").as_deref(), Some("Ёж"));
        assert_eq!(find("(?i)Ёж", "ёЖ").as_deref(), Some("ёЖ"));
        // Классы под `(?i)` сворачиваются в обе стороны, а отрицание
        // остаётся отрицанием того же множества.
        assert_eq!(find("(?i)[а-я]+", "АБВ").as_deref(), Some("АБВ"));
        assert_eq!(find("(?i)[А-Я]+", "абв").as_deref(), Some("абв"));
        assert_eq!(find("(?i)[^а-я]+", "абвАБВ12").as_deref(), Some("12"));
    }

    #[test]
    fn an_inline_flag_works_from_its_place_to_the_end_of_its_group() {
        // До флага регистр важен, после — нет.
        assert_eq!(find("а(?i)б", "аБ").as_deref(), Some("аБ"));
        assert_eq!(find("а(?i)б", "Аб"), None);
        // Из группы флаг наружу не выходит.
        assert_eq!(find("(?:(?i)а)б", "Аб").as_deref(), Some("Аб"));
        assert_eq!(find("(?:(?i)а)б", "АБ"), None);
        // Зато до конца своей группы действует, включая соседние ветви.
        assert_eq!(find("(?:(?i)а|б)", "Б").as_deref(), Some("Б"));
        assert_eq!(find("(?im)^а", "б\nА").as_deref(), Some("А"));
    }

    #[test]
    fn a_surrogate_pair_counts_as_one_character() {
        // U+1F600 — одна кодовая точка из двух код-юнитов.
        assert_eq!(find(".", "\u{1F600}б").as_deref(), Some("\u{1F600}"));
        assert_eq!(spans(".", "\u{1F600}б")[0], Some((0, 2)));
        assert_eq!(spans(".{2}", "\u{1F600}б")[0], Some((0, 3)));
        assert_eq!(
            find("\u{1F600}{2}", "\u{1F600}\u{1F600}").as_deref(),
            Some("\u{1F600}\u{1F600}")
        );
        // Экранированием пара записывается двумя `\u`, а значит то же самое.
        assert_eq!(find(r"😀", "а\u{1F600}").as_deref(), Some("\u{1F600}"));
        // Непарный суррогат — обычная точка со своим значением.
        let lonely = vec![0x0430u16, 0xD83Du16, 0x0431u16];
        let regex = compile("а.б");
        assert_eq!(
            regex.find_at(&lonely, 0).and_then(|m| m.spans[0]),
            Some((0, 3))
        );
    }

    #[test]
    fn an_empty_iteration_stops_the_loop() {
        // Без стража пустой итерации эти шаблоны крутились бы вечно.
        assert_eq!(find("(а*)*", "б").as_deref(), Some(""));
        assert_eq!(spans("(а*)*", "б"), vec![Some((0, 0)), Some((0, 0))]);
        assert_eq!(find("(а*)*", "аа").as_deref(), Some("аа"));
        assert_eq!(find("(?:|а)+", "аа").as_deref(), Some(""));
        assert_eq!(find("(?:а|)+б", "аб").as_deref(), Some("аб"));
        assert_eq!(find("(?:)*", "а").as_deref(), Some(""));
        assert_eq!(find("(а*){2,}", "аа").as_deref(), Some("аа"));
    }

    /// Пустой проход обрывает и ОГРАНИЧЕННЫЙ квантор.
    ///
    /// Формы ниже — те, на которых страж вообще виден: тело умеет совпасть
    /// с пустотой, поэтому вопрос не «где нашли» (нулевая группа во всех
    /// строках одна и та же), а какая итерация запомнилась группой.
    /// Ожидания сверены построчно с `python3 -c "import re; ..."` — и это
    /// единственный диалект, с которым движок совпал на всех шести
    /// строках. Согласия классиков здесь нет: на тех же шести строках
    /// `perl` совпадает на трёх (на `(б{0,2}?){1,3}в` и `(б{0,2}?){2,4}в`
    /// по «ббв» и на `(а*?){3,5}$` по «аа» он отдаёт группу 1 = `2,2` —
    /// допускает ещё один пустой проход после непустого), ECMAScript тоже
    /// на трёх, но на других — на первой, второй и шестой строках он
    /// отвечает `0,1`, оставляя захват за проходом, съевшим текст;
    /// PCRE2 не совпадает ни на одной, и до появления стража в
    /// ограниченной ветви [`Compiler::repeat`] движок отвечал ровно как
    /// PCRE2. Эталоном выбран `python3 re`; ответ 8.3.27 не измерен и
    /// записан вопросом `REGEX.REPEAT.EMPTY` — он станет измерим, когда
    /// появятся глобальные функции над движком.
    #[test]
    fn an_empty_pass_ends_a_bounded_quantifier_like_the_classics() {
        /// Шаблон, текст и ожидаемые границы всех групп.
        type Case<'a> = (&'a str, &'a str, &'a [Option<(usize, usize)>]);

        let table: &[Case] = &[
            ("(б{0,1}?){0,2}в", "бв", &[Some((0, 2)), Some((1, 1))]),
            ("(б{0,2}?){1,3}в", "бв", &[Some((0, 2)), Some((1, 1))]),
            ("(б{0,2}?){1,3}в", "ббв", &[Some((0, 3)), Some((1, 2))]),
            ("(б{0,2}?){2,4}в", "ббв", &[Some((0, 3)), Some((1, 2))]),
            ("(а*?){3,5}$", "аа", &[Some((0, 2)), Some((1, 2))]),
            ("((?:|б)){0,2}$", "б", &[Some((0, 1)), Some((1, 1))]),
        ];
        for (pattern, text, expected) in table {
            assert_eq!(
                spans(pattern, text),
                *expected,
                "шаблон «{pattern}» на «{text}»"
            );
        }
    }

    /// Таблица классов растёт с ШАБЛОНОМ, а не с разворачиванием `{n,m}`.
    ///
    /// Тест белого ящика: снаружи разница видна только пиковой памятью, а
    /// инвариант «одна запись на синтаксический класс» и есть та граница,
    /// ради которой заведён [`MAX_PROGRAM`] — сорок тысяч копий класса
    /// обязаны делить одну запись, иначе предел считает инструкции, а
    /// память съедают клоны `ClassSpec`.
    #[test]
    fn the_class_table_grows_with_the_pattern_not_the_expansion() {
        let many = compile("(?:[абв]){40000}");
        assert_eq!(many.classes.len(), 1);
        assert_eq!(compile("[а][б]").classes.len(), 2);
    }

    #[test]
    fn a_broken_pattern_is_an_error_and_not_a_panic() {
        assert!(parse_error("(аб").contains("не хватает « ) »"));
        assert!(parse_error("аб)").contains("лишняя"));
        assert!(parse_error("[аб").contains("не хватает « ] »"));
        assert!(parse_error("[]").contains("пустой класс"));
        assert!(parse_error("[я-а]").contains("перевёрнутый диапазон"));
        assert!(parse_error("а{2,1}").contains("меньше нижней"));
        assert!(parse_error("*аб").contains("без атома"));
        assert!(parse_error("{2}аб").contains("без атома"));
        assert!(parse_error("а**").contains("два квантора"));
        assert!(parse_error("а{").contains("ожидалось число"));
        assert!(parse_error("а{2").contains("не хватает « } »"));
        assert!(parse_error(r"а\").contains("обрывается"));
        assert!(parse_error(r"\u12").contains("четыре шестнадцатеричные"));
        assert!(parse_error(r"\1").contains("обратных ссылок"));
        assert!(parse_error("(?=аб)").contains("не поддержана"));
        assert!(parse_error("(?i:аб)").contains("инлайн-флаг"));
        assert!(parse_error("[а-я&&[^аеиоу]]").contains("«&&»"));
        assert!(parse_error("[[аб]]").contains("вложенные классы"));
    }

    #[test]
    fn the_implementation_limits_are_errors_too() {
        let deep = format!(
            "{}а{}",
            "(".repeat(MAX_DEPTH + 1),
            ")".repeat(MAX_DEPTH + 1)
        );
        assert!(parse_error(&deep).contains("вложены глубже"));
        assert!(parse_error("а{70000}").contains("предел этой реализации"));
        // Разворачивание `{n,m}` копиями упирается в длину программы.
        assert!(parse_error("(?:абвгде{9000}){40}").contains("инструкций"));
    }

    #[test]
    fn a_pattern_may_be_empty() {
        assert_eq!(find("", "аб").as_deref(), Some(""));
        assert_eq!(spans("", "аб"), vec![Some((0, 0))]);
        assert_eq!(compile("(а)(б)").group_count(), 3);
        assert_eq!(compile("").group_count(), 1);
    }

    /// Сводная таблица приоритета бэктрекинга.
    ///
    /// Ожидания взяты НЕ с платформы — поверхности BSL ещё нет, спрашивать
    /// 8.3.27 нечем. Это правила приоритета, общие для всех движков с
    /// бэктрекингом (слева направо, жадный берёт больше, ленивый меньше,
    /// группа помнит последнюю удачную итерацию); каждая строка сверена с
    /// независимой реализацией ТЕХ ЖЕ правил — `python3 -c "import re;
    /// m = re.search(шаблон, текст); print([m.span(i) …])"`. Настоящие
    /// расхождения диалекта с ICU ловятся отдельно, метками `REGEX.*`.
    #[test]
    fn backtracking_priority_matches_the_classic_rules() {
        /// Шаблон, текст и ожидаемые границы всех групп.
        type Case<'a> = (&'a str, &'a str, &'a [Option<(usize, usize)>]);

        let table: &[Case] = &[
            ("(а+)(б+)?", "аа", &[Some((0, 2)), Some((0, 2)), None]),
            ("а{2,3}б", "аааб", &[Some((0, 4))]),
            ("(?:аб)*", "абабв", &[Some((0, 4))]),
            ("(?:аб)*?в", "абабв", &[Some((0, 5))]),
            ("[а-я]+?", "абв", &[Some((0, 1))]),
            (
                "(а|аб)(в|бв)",
                "абв",
                &[Some((0, 3)), Some((0, 1)), Some((1, 3))],
            ),
            (
                "(а*)(а*)",
                "ааа",
                &[Some((0, 3)), Some((0, 3)), Some((3, 3))],
            ),
            ("(?:а(б)?)+", "ааб", &[Some((0, 3)), Some((2, 3))]),
            (r"\bбор\b", "бор!", &[Some((0, 3))]),
            ("(?m)^.$", "а\nб", &[Some((0, 1))]),
            ("^а*$", "ааа", &[Some((0, 3))]),
            (
                "(а)(?:б)(в)",
                "абв",
                &[Some((0, 3)), Some((0, 1)), Some((2, 3))],
            ),
            ("[^аб]+", "аабввг", &[Some((3, 6))]),
            ("а.*?в|аб", "абв", &[Some((0, 3))]),
            ("(а?)*б", "б", &[Some((0, 1)), Some((0, 0))]),
            ("(?:а{0,2})+б", "аааб", &[Some((0, 4))]),
            ("(а*){2,}", "аа", &[Some((0, 2)), Some((2, 2))]),
        ];
        for (pattern, text, expected) in table {
            assert_eq!(
                spans(pattern, text),
                *expected,
                "шаблон «{pattern}» на «{text}»"
            );
        }
    }

    #[test]
    fn backtracking_restores_the_groups_it_had_written() {
        // Вторая ветвь `(б)` побеждает, следов первой в группах нет.
        assert_eq!(
            spans("(?:(а)в|(б)в)", "бв"),
            vec![Some((0, 2)), None, Some((0, 1))]
        );
        // Захват внутри отката тоже откатывается.
        assert_eq!(spans("(?:(аб)|а)бв", "абв"), vec![Some((0, 3)), None]);
    }
}
