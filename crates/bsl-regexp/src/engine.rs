//! Движок регулярных выражений: РАЗБОР здесь, МАТЧИНГ у `fancy-regex`.
//!
//! Слоёв два. Разбор — рекурсивный спуск по кодовым точкам в дерево
//! [`Node`]: он навешивает вмороженные флаги, нумерует группы по
//! открывающей скобке и честно отвергает всё, чего в диалекте нет, —
//! молча превратить непонятую конструкцию в литерал было бы худшим из
//! вариантов. Затем рендерер печатает дерево в синтаксис крейта
//! `fancy-regex`: простые шаблоны крейт исполняет линейным NFA своего
//! нижнего слоя `regex`, а просмотры и прочие «fancy»-формы — своим
//! бэктрекером. Позиции наружу — всегда код-юниты UTF-16.
//!
//! Рендерер не переписывает шаблон буквально: измеренные на 8.3.27 края
//! диалекта (якоря `REGEX.*`, контракты `measure-regex.bsl` и
//! `measure-regex2.bsl`) отличаются от родной семантики крейта, поэтому
//! `\w`, `\b`, `.`, `^`, `$` и флаги печатаются РАЗВЁРНУТЫМИ формами —
//! каждая задокументирована у своей константы ниже.
//!
//! Два сознательных решения этого слоя:
//!
//!   * лимит бэктрекинга крейта отключён (`backtrack_limit(usize::MAX)`):
//!     цена бэктрекинга — та же, что у ICU, на шаблонах вида `(а+)+б`
//!     время экспоненциально, и платформа ведёт себя так же. Отсечка
//!     превратила бы «искали слишком долго» в ошибку, которой у платформы
//!     нет;
//!   * непарный суррогат входа заменяется на U+FFFD: крейт работает по
//!     UTF-8-строке, где непарной половине пары представления нет. Сам
//!     интерпретатор такую строку не строит, платформа с ней не
//!     промерена — это огрубление неизмеримого угла, а не совместимость.

#[path = "regex/tables.rs"]
mod tables;

use bsl_rt::{RtError, RtResult};

pub(crate) use tables::decimal_digit_value;

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

/// Предел развёрнутой оценки шаблона.
///
/// Крейт разворачивает `{n,m}` копиями тела, поэтому `(?:а{1000}){1000}` —
/// это миллион инструкций из двенадцати символов шаблона. Предел
/// реализации, а не платформы: он даёт внятную ошибку вместо гигабайтов
/// памяти, и срабатывает в рендерере — ДО крейта и его собственных
/// пределов размера.
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
const CP_FF: Cp = 0x0C;
const CP_CR: Cp = 0x0D;

/// Кодовая точка, начинающаяся в позиции `i`, и её ширина в код-юнитах.
/// `None` — за концом строки.
pub(crate) fn cp_at(units: &[u16], i: usize) -> Option<(Cp, usize)> {
    let first = Cp::from(*units.get(i)?);
    if (0xD800..0xDC00).contains(&first)
        && let Some(low) = units.get(i + 1).copied().map(Cp::from)
        && (0xDC00..0xE000).contains(&low)
    {
        return Some((0x1_0000 + ((first - 0xD800) << 10) + (low - 0xDC00), 2));
    }
    Some((first, 1))
}

/// Кодовая точка, ЗАКАНЧИВАЮЩАЯСЯ в позиции `i`, и её ширина. Нужна
/// границам слова и якорям: они смотрят назад.
pub(crate) fn cp_before(units: &[u16], i: usize) -> Option<(Cp, usize)> {
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
    /// Вложенный класс: `[[аб]в]`, `[[^аб]]`.
    Nested(Box<ClassSpec>),
}

/// Класс символов целиком: пересечение объединений.
///
/// Обычный класс — одно объединение; каждое `&&` открывает следующий
/// операнд пересечения (ИЗМЕРЕНО, якорь `REGEX.CLASSOPS`). Запись
/// `[а-я-[б]]` пересечением или разностью НЕ является: минус после
/// диапазона — литерал, скобки — вложенный класс, итог — объединение.
#[derive(Clone, Debug)]
struct ClassSpec {
    negated: bool,
    inter: Vec<Vec<ClassItem>>,
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
    /// `.`; под `(?s)` берёт и терминаторы (якорь `REGEX.DOTALL`).
    Any {
        dotall: bool,
    },
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
    /// Просмотры `(?=…)`, `(?!…)`, `(?<=…)`, `(?<!…)`.
    Look {
        behind: bool,
        negated: bool,
        body: Box<Node>,
    },
    /// Обратная ссылка `\1`–`\9`; свёртка вморожена, как у литерала:
    /// ссылка сравнивает по флагу, действующему в ЕЁ месте.
    Backref {
        slot: usize,
        icase: bool,
    },
    /// Атомарная группа `(?>…)`; притяжательный квантор — её сахар.
    Atomic(Box<Node>),
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
        /// `а*+` и родня: назад не отдаёт (якорь `REGEX.POSSESSIVE`).
        possessive: bool,
    },
}

/// Флаги, действующие в текущей точке разбора.
#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    icase: bool,
    multiline: bool,
    dotall: bool,
}

/// Что дало экранирование.
enum Escape {
    Cp(Cp),
    Prop { kind: PropKind, negated: bool },
    WordBoundary { negated: bool },
    Backref(usize),
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

    /// Разбор списка флагов `i`/`m`/`s` с необязательной снимающей частью
    /// после `-`; курсор — сразу за `(?`. Возвращает пару (установить,
    /// снять) и число разобранных символов, `None` — если это не список
    /// флагов вовсе.
    fn flag_list(&self, mut ahead: usize) -> Option<(Flags, Flags, usize)> {
        let mut set = Flags::default();
        let mut unset = Flags::default();
        let mut clearing = false;
        let mut named = 0usize;
        loop {
            match self.peek_at(ahead).and_then(char::from_u32) {
                Some(c @ ('i' | 'm' | 's')) => {
                    let target = if clearing { &mut unset } else { &mut set };
                    match c {
                        'i' => target.icase = true,
                        'm' => target.multiline = true,
                        _ => target.dotall = true,
                    }
                    named += 1;
                }
                Some('-') if !clearing => clearing = true,
                _ if named == 0 => return None,
                _ => return Some((set, unset, ahead)),
            }
            ahead += 1;
        }
    }

    /// `(?i)`, `(?m)`, `(?s)` и их сочетания со снятием `(?-i)` — меняют
    /// флаги ДО конца охватывающей группы. ИЗМЕРЕНО (якоря
    /// `REGEX.FLAG.SCOPE`, `REGEX.DOTALL`): снятая групповая точка
    /// `(?s)а(?-s:.)б` терминатор не берёт, снятие без группы действует
    /// так же до конца охватывающей группы.
    ///
    /// Возвращает `false`, если в позиции не инлайн-флаг: групповую форму
    /// `(?i:…)` и всё остальное разберёт [`Parser::atom`] как группу.
    fn inline_flags(&mut self) -> RtResult<bool> {
        if self.peek() != Some('(' as Cp) || self.peek_at(1) != Some('?' as Cp) {
            return Ok(false);
        }
        let Some((set, unset, end)) = self.flag_list(2) else {
            return Ok(false);
        };
        if self.peek_at(end) != Some(')' as Cp) {
            // `(?i:…)` — форма группы, не инлайн-флага.
            return Ok(false);
        }
        self.pos += end + 1;
        self.flags.icase = (self.flags.icase || set.icase) && !unset.icase;
        self.flags.multiline = (self.flags.multiline || set.multiline) && !unset.multiline;
        self.flags.dotall = (self.flags.dotall || set.dotall) && !unset.dotall;
        Ok(true)
    }

    /// Атом с необязательным квантором.
    fn quantified(&mut self) -> RtResult<Node> {
        let atom = self.atom()?;
        let Some((min, max)) = self.quantifier()? else {
            return Ok(atom);
        };
        if matches!(atom, Node::Look { .. }) {
            // ИЗМЕРЕНО: `а(?=б)?` — ошибка шаблона, а не необязательный
            // просмотр.
            return Err(bad("квантор на просмотр не поддержан"));
        }
        let greedy = !self.eat('?' as Cp);
        // `а*+` — притяжательная форма: жадный квантор, не отдающий назад
        // (ИЗМЕРЕНО, якорь `REGEX.POSSESSIVE`). Ленивый суффикс `?` с `+`
        // не сочетается — это ловит проверка двойного квантора ниже.
        let possessive = greedy && self.eat('+' as Cp);
        if let Some(next) = self.peek()
            && (next == '*' as Cp || next == '+' as Cp || next == '?' as Cp)
        {
            return Err(bad(format!(
                "два квантора подряд («{}» после квантора)",
                show(next)
            )));
        }
        Ok(Node::Repeat {
            body: Box::new(atom),
            min,
            max,
            greedy,
            possessive,
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
            return Ok(Node::Any {
                dotall: self.flags.dotall,
            });
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
                        inter: vec![vec![ClassItem::Prop { kind, negated }]],
                    },
                    icase: self.flags.icase,
                }),
                Escape::WordBoundary { negated } => Ok(Node::WordBoundary { negated }),
                Escape::Backref(slot) => Ok(Node::Backref {
                    slot,
                    icase: self.flags.icase,
                }),
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
        enum Kind {
            Plain(Option<usize>),
            Look { behind: bool, negated: bool },
            Atomic,
        }
        let mut flag_group: Option<(Flags, Flags)> = None;
        let kind = if self.peek() == Some('?' as Cp) {
            match self.peek_at(1) {
                Some(cp) if cp == ':' as Cp => {
                    self.pos += 2;
                    Kind::Plain(None)
                }
                // Просмотры вперёд и назад, обе полярности. ИЗМЕРЕНО
                // (якорь `REGEX.LOOK.BEHIND` и контракт
                // `measure-regex2.bsl`): захваты внутри видны снаружи,
                // сам просмотр текста не ест.
                Some(cp) if cp == '=' as Cp => {
                    self.pos += 2;
                    Kind::Look {
                        behind: false,
                        negated: false,
                    }
                }
                Some(cp) if cp == '!' as Cp => {
                    self.pos += 2;
                    Kind::Look {
                        behind: false,
                        negated: true,
                    }
                }
                Some(cp) if cp == '<' as Cp && self.peek_at(2) == Some('=' as Cp) => {
                    self.pos += 3;
                    Kind::Look {
                        behind: true,
                        negated: false,
                    }
                }
                Some(cp) if cp == '<' as Cp && self.peek_at(2) == Some('!' as Cp) => {
                    self.pos += 3;
                    Kind::Look {
                        behind: true,
                        negated: true,
                    }
                }
                Some(cp) if cp == '>' as Cp => {
                    self.pos += 2;
                    Kind::Atomic
                }
                Some(_) => {
                    // Групповая форма флагов `(?i:…)`/`(?-i:…)` — тело
                    // разбирается с изменёнными флагами, а по выходе они
                    // восстанавливаются вместе с флагами охватывающей
                    // группы.
                    if let Some((set, unset, end)) = self.flag_list(1)
                        && self.peek_at(end) == Some(':' as Cp)
                    {
                        self.pos += end + 1;
                        flag_group = Some((set, unset));
                        Kind::Plain(None)
                    } else {
                        let cp = self.peek_at(1).unwrap_or('?' as Cp);
                        return Err(bad(format!(
                            "конструкция «(?{}» не поддержана: есть группы «(…)» и \
                             «(?:…)», просмотры «(?=…)»/«(?!…)»/«(?<=…)»/«(?<!…)», \
                             атомарная «(?>…)» и флаги «(?i)»/«(?m)»/«(?s)» с \
                             групповой и снимающей формами",
                            show(cp)
                        )));
                    }
                }
                None => return Err(bad("шаблон обрывается на «(?»")),
            }
        } else {
            self.groups += 1;
            Kind::Plain(Some(self.groups))
        };
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(bad(format!(
                "группы вложены глубже {MAX_DEPTH} уровней — это предел этой реализации"
            )));
        }
        // Флаги действуют до конца ОХВАТЫВАЮЩЕЙ группы, поэтому на выходе
        // возвращается то, что было на входе. ИЗМЕРЕНО на 8.3.27 (якорь
        // `REGEX.FLAG.SCOPE`) сразу с четырёх сторон: `а(?i)б` не находится
        // в «Аб» и находится в «аБ» — то есть налево флаг не действует; из
        // `(?:(?i)а)б` и из `((?i)а)б` он наружу не выходит, зато в соседнюю
        // ветвь альтернации (`б(?i)в|а` на «А») выходит: ветвь — не группа.
        let outer = self.flags;
        if let Some((set, unset)) = flag_group {
            self.flags.icase = (self.flags.icase || set.icase) && !unset.icase;
            self.flags.multiline = (self.flags.multiline || set.multiline) && !unset.multiline;
            self.flags.dotall = (self.flags.dotall || set.dotall) && !unset.dotall;
        }
        let body = self.alternation()?;
        self.flags = outer;
        self.depth -= 1;
        if !self.eat(')' as Cp) {
            return Err(bad("незакрытая группа: не хватает « ) »"));
        }
        let body = Box::new(body);
        Ok(match kind {
            Kind::Plain(slot) => Node::Group { slot, body },
            Kind::Atomic => Node::Atomic(body),
            Kind::Look { behind, negated } => {
                if behind && !bounded(&body) {
                    // ИЗМЕРЕНО (якорь `REGEX.LOOK.BEHIND`): `(?<=а+)` и
                    // `(?<=а*)` платформа отвергает, ограниченные формы
                    // работают.
                    return Err(bad("просмотр назад требует ограниченной длины: кванторы \
                         без верхней границы в нём не поддержаны"));
                }
                Node::Look {
                    behind,
                    negated,
                    body,
                }
            }
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
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(bad(format!(
                "классы вложены глубже {MAX_DEPTH} уровней — это предел этой реализации"
            )));
        }
        let parsed = self.class_body();
        self.depth -= 1;
        parsed
    }

    fn class_body(&mut self) -> RtResult<ClassSpec> {
        let negated = self.eat('^' as Cp);
        let mut inter: Vec<Vec<ClassItem>> = vec![Vec::new()];
        loop {
            let items = inter.last_mut().expect("хотя бы один операнд есть всегда");
            let Some(cp) = self.peek() else {
                return Err(bad("незакрытый класс символов: не хватает « ] »"));
            };
            if cp == ']' as Cp {
                if items.is_empty() {
                    if inter.len() == 1 {
                        return Err(bad(
                            "пустой класс символов; литеральная « ] » внутри класса \
                             пишется как «\\]»",
                        ));
                    }
                    return Err(bad("пустой операнд пересечения «&&» в классе символов"));
                }
                self.pos += 1;
                return Ok(ClassSpec { negated, inter });
            }
            if cp == '[' as Cp {
                // Вложенный класс — операнд объединения (ИЗМЕРЕНО, якорь
                // `REGEX.CLASSOPS`: `[[аб]в]` и `[[^аб]]` работают).
                self.pos += 1;
                let nested = self.class()?;
                items.push(ClassItem::Nested(Box::new(nested)));
                continue;
            }
            if cp == '&' as Cp && self.peek_at(1) == Some('&' as Cp) {
                if items.is_empty() {
                    return Err(bad("пустой операнд пересечения «&&» в классе символов"));
                }
                self.pos += 2;
                inter.push(Vec::new());
                continue;
            }
            let low = if cp == '\\' as Cp {
                self.pos += 1;
                match self.escape(true)? {
                    Escape::Cp(cp) => Some(cp),
                    Escape::Prop { kind, negated } => {
                        items.push(ClassItem::Prop { kind, negated });
                        None
                    }
                    // Внутрь класса `\b`/`\B` и ссылки не пропускает
                    // `escape`.
                    Escape::WordBoundary { .. } | Escape::Backref(_) => None,
                }
            } else {
                self.pos += 1;
                Some(cp)
            };
            let Some(low) = low else {
                continue;
            };
            // Диапазон, только если после дефиса есть что-то кроме « ] » и
            // « [ »: в `[а-]` дефис — обычный символ, а в `[а-я-[б]]` он
            // литерал перед вложенным классом, и итог — объединение
            // (ИЗМЕРЕНО, якорь `REGEX.CLASSOPS`: «б» в `[а-я-[б]]`
            // находится).
            let is_range = self.peek() == Some('-' as Cp)
                && self
                    .peek_at(1)
                    .is_some_and(|cp| cp != ']' as Cp && cp != '[' as Cp);
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
                        Escape::Prop { .. } | Escape::WordBoundary { .. } | Escape::Backref(_) => {
                            return Err(bad("границей диапазона в классе может быть только \
                                 символ, а не сокращение вида «\\d»"));
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
            '0' => Err(bad(
                "экранирование «\\0» не поддержано: восьмеричных кодов здесь нет",
            )),
            '1'..='9' if in_class => Err(bad(
                "обратная ссылка внутри класса символов не имеет смысла",
            )),
            // Ссылка на группу, которой в шаблоне нет вовсе, остаётся
            // ошибкой — её отдаст крейт на компиляции рендера; ссылка,
            // стоящая РАНЬШЕ своей группы, законна и проваливает
            // совпадение (ИЗМЕРЕНО, контракт `measure-regex2.bsl`:
            // «ссылка вперёд» — Нет, не ошибка).
            '1'..='9' => Ok(Escape::Backref(c as usize - '0' as usize)),
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

// --- рендеринг в синтаксис крейта ---------------------------------------

/// Набор `\w` в записи крейта. Состав ИЗМЕРЕН на 8.3.27 (якорь
/// `REGEX.CLASS.WORD`): свойство `Alphabetic` (римская единица U+2160
/// категории Nl и буква в круге U+24B6 категории So — символы слова), все
/// знаки `\p{M}`, десятичные цифры `\p{Nd}` и соединители `\p{Pc}`. У
/// самого крейта `\w` шире — он включает `\p{Join_Control}`, то есть
/// невидимые U+200C/U+200D, которые документация ICU в `\w` перечисляет,
/// а платформа символами слова НЕ считает. Вычитание возвращает
/// измеренный состав; крейтовые `\w` и `\b` поэтому не эмитятся никогда.
const WORD_CLASS: &str = r"[\w--[\x{200C}\x{200D}]]";

/// Дополнение [`WORD_CLASS`]: не-слово крейта плюс оба джойнера.
const NON_WORD_CLASS: &str = r"[[^\w]\x{200C}\x{200D}]";

/// `.` — любой символ, кроме семи терминаторов UTS#18: перевод строки,
/// вертикальная табуляция, перевод формата, возврат каретки, NEL и
/// разделители строк и абзацев. ИЗМЕРЕНО на 8.3.27 (якорь
/// `REGEX.LINE.TERMINATORS`): Java, для сравнения, вертикальную табуляцию
/// и перевод формата концом строки не считает, и именно этой парой проба
/// два набора и разделила. У крейта точка исключает только `\n`, поэтому
/// всегда печатается класс.
const DOT_CLASS: &str = r"[^\n\x0B\x0C\r\x{85}\x{2028}\x{2029}]";

/// `$` без `(?m)`: конец входа либо позиция перед РОВНО ОДНИМ хвостовым
/// терминатором, где `\r\n` считается одним, а между `\r` и `\n` конца
/// нет. ИЗМЕРЕНО на 8.3.27 (якорь `REGEX.ANCHOR.EOL`, блок «конец …» в
/// `measure-regex.bsl`): «аб$» находится в «аб», «аб\n», «аб\r», «аб\r\n»
/// и «аб» + NEL, но не в «аб\n\n», а «аб\r$» в «аб\r\n» — нет. У крейта
/// `$` — строгий конец входа, поэтому форма собрана из просмотров.
const DOLLAR_FORM: &str =
    r"(?:\z|(?=\r\n?\z)|(?<!\r)(?=\n\z)|(?=[\x0B\x0C\x{85}\x{2028}\x{2029}]\z))";

/// `^` под `(?m)`: начало входа либо позиция сразу за терминатором — но
/// не между `\r` и `\n` и не в самом конце входа: за хвостовым переводом
/// строки новая строка НЕ открывается (якорь `REGEX.ANCHOR.EOL`: в «а\n»
/// позиций у `^` одна, в «а\nб» — две). Страж `(?=[\s\S])` и отвечает за
/// «не в самом конце»; крейтовый `(?m)` знает только `\n`, поэтому форма
/// перечисляет терминаторы просмотрами назад.
const MULTILINE_START_FORM: &str =
    r"(?:\A|(?:(?<=[\n\x0B\x0C\x{85}\x{2028}\x{2029}])|(?<=\r)(?!\n))(?=[\s\S]))";

/// `$` под `(?m)`: конец входа либо позиция перед терминатором — но не
/// между `\r` и `\n` (в «а\n» позиций две: перед `\n` и в конце; в
/// «а\r\nб» — перед `\r` и в конце, якорь `REGEX.ANCHOR.EOL` и блок
/// «многострочный …» контракта).
const MULTILINE_END_FORM: &str = r"(?:\z|(?=[\x0B\x0C\r\x{85}\x{2028}\x{2029}])|(?<!\r)(?=\n))";

/// Печать дерева в синтаксис крейта.
///
/// Рендерер не оптимизирует: каждое квантуемое тело оборачивается
/// незахватывающей группой, каждый регистронезависимый лист — группой
/// `(?i:…)`. Лишние группы крейту безразличны, а нумерацию захватов они
/// не сдвигают — физические номера групп совпадают с логическими, потому
/// что своих ЗАХВАТЫВАЮЩИХ групп рендерер не вставляет никогда.
struct Renderer {
    out: String,
    /// Обнуляемость тела каждой логической группы; индекс — номер группы,
    /// нулевой не используется. Нужна чистке фантомных пустых участий,
    /// см. [`Regex::find_at`].
    nullable: Vec<bool>,
    /// Оценка числа «инструкций» развёрнутого шаблона — прежний предел
    /// [`MAX_PROGRAM`] переехал сюда из компилятора: сам рендерер копий
    /// не печатает, но крейт разворачивает `{n,m}` копиями тела, и предел
    /// обязан сработать ДО него, с прежним сообщением.
    cost: usize,
}

impl Renderer {
    fn new(group_count: usize) -> Renderer {
        Renderer {
            out: String::new(),
            nullable: vec![false; group_count],
            cost: 0,
        }
    }

    fn charge(&mut self, amount: usize) -> RtResult<()> {
        self.cost = self.cost.saturating_add(amount);
        if self.cost > MAX_PROGRAM {
            return Err(bad(format!(
                "шаблон разворачивается больше чем в {MAX_PROGRAM} инструкций — \
                 это предел этой реализации"
            )));
        }
        Ok(())
    }

    /// Кодовая точка в литеральной позиции. ASCII-буквоцифры и всё вне
    /// ASCII печатаются как есть, прочий ASCII — записью `\x{..}`: так не
    /// нужен список метасимволов, а в классе заодно закрыты `-`, `]` и
    /// `^`. Непарный суррогат представления в UTF-8 не имеет и печатается
    /// как U+FFFD (см. шапку модуля).
    fn literal_cp(&mut self, cp: Cp) {
        let cp = if (0xD800..0xE000).contains(&cp) {
            0xFFFD
        } else {
            cp
        };
        match char::from_u32(cp) {
            Some(c) if c.is_ascii_alphanumeric() || !c.is_ascii() => self.out.push(c),
            _ => {
                self.out.push_str(&format!("\\x{{{cp:X}}}"));
            }
        }
    }

    /// Сокращение или свойство в позиции элемента класса (или атома).
    fn prop(&mut self, kind: PropKind, negated: bool) {
        let text = match (kind, negated) {
            (PropKind::Digit, false) => r"\p{Nd}",
            (PropKind::Digit, true) => r"\P{Nd}",
            (PropKind::Letter, false) => r"\p{L}",
            (PropKind::Letter, true) => r"\P{L}",
            // Состав `\s` ИЗМЕРЕН на 8.3.27 (якорь `REGEX.CLASS.SPACE`):
            // свойство Unicode `White_Space` целиком — вертикальная
            // табуляция U+000B и NEL U+0085 пробельные, хотя набор
            // `[\t\n\f\r\p{Z}]` из документации ICU обоих не содержит.
            // У крейта `\s` — то же свойство, печатается как есть.
            (PropKind::Space, false) => r"\s",
            (PropKind::Space, true) => r"\S",
            (PropKind::Word, false) => WORD_CLASS,
            (PropKind::Word, true) => NON_WORD_CLASS,
        };
        self.out.push_str(text);
    }

    /// Класс символов; операнды пересечения печатаются через `&&` крейта,
    /// вложенные классы — рекурсивно. Суррогатные края диапазонов
    /// клипуются до валидных половин, целиком суррогатный элемент
    /// вырождается в U+FFFD.
    fn class(&mut self, spec: &ClassSpec) {
        self.out.push('[');
        if spec.negated {
            self.out.push('^');
        }
        for (term_index, items) in spec.inter.iter().enumerate() {
            if term_index > 0 {
                self.out.push_str("&&");
            }
            for item in items {
                match item {
                    ClassItem::Single(cp) => self.literal_cp(*cp),
                    ClassItem::Range(low, high) => {
                        let mut emit = |a: Cp, b: Cp| {
                            self.literal_cp(a);
                            self.out.push('-');
                            self.literal_cp(b);
                        };
                        let (low, high) = (*low, *high);
                        if low >= 0xD800 && high < 0xE000 {
                            // Диапазон целиком в суррогатах.
                            self.literal_cp(0xFFFD);
                        } else if low < 0xD800 && high >= 0xE000 {
                            emit(low, 0xD7FF);
                            emit(0xE000, high);
                        } else if (0xD800..0xE000).contains(&low) {
                            emit(0xE000, high);
                        } else if (0xD800..0xE000).contains(&high) {
                            emit(low, 0xD7FF);
                        } else {
                            emit(low, high);
                        }
                    }
                    ClassItem::Prop { kind, negated } => self.prop(*kind, *negated),
                    ClassItem::Nested(nested) => self.class(nested),
                }
            }
        }
        self.out.push(']');
    }

    /// Обе границы слова — просмотры над [`WORD_CLASS`]: `\b` — смена
    /// принадлежности, `\B` — её сохранение. Края входа корректны сами
    /// собой: несуществующий сосед проваливает и `(?<=W)`, и `(?=W)`.
    fn word_boundary(&mut self, negated: bool) {
        let w = WORD_CLASS;
        let form = if negated {
            format!("(?:(?<={w})(?={w})|(?<!{w})(?!{w}))")
        } else {
            format!("(?:(?<={w})(?!{w})|(?<!{w})(?={w}))")
        };
        self.out.push_str(&form);
    }

    /// Узел целиком; возвращает, обнуляемо ли его тело (совпадает ли оно
    /// хоть когда-нибудь с пустотой) — это и заполняет [`Renderer::nullable`].
    fn node(&mut self, node: &Node) -> RtResult<bool> {
        match node {
            Node::Empty => Ok(true),
            Node::Literal { cp, icase } => {
                self.charge(1)?;
                if *icase {
                    self.out.push_str("(?i:");
                    self.literal_cp(*cp);
                    self.out.push(')');
                } else {
                    self.literal_cp(*cp);
                }
                Ok(false)
            }
            Node::Class { spec, icase } => {
                self.charge(1)?;
                if *icase {
                    self.out.push_str("(?i:");
                    self.class(spec);
                    self.out.push(')');
                } else {
                    self.class(spec);
                }
                Ok(false)
            }
            Node::Any { dotall } => {
                self.charge(1)?;
                // Под `(?s)` точка берёт все семь терминаторов (ИЗМЕРЕНО,
                // якорь `REGEX.DOTALL`) — печатается класс-всё: флаговой
                // семантике крейта рендер не доверяет ничего.
                self.out
                    .push_str(if *dotall { r"[\s\S]" } else { DOT_CLASS });
                Ok(false)
            }
            Node::LineStart { multiline } => {
                self.charge(1)?;
                self.out.push_str(if *multiline {
                    MULTILINE_START_FORM
                } else {
                    r"\A"
                });
                Ok(true)
            }
            Node::LineEnd { multiline } => {
                self.charge(1)?;
                self.out.push_str(if *multiline {
                    MULTILINE_END_FORM
                } else {
                    DOLLAR_FORM
                });
                Ok(true)
            }
            Node::WordBoundary { negated } => {
                self.charge(1)?;
                self.word_boundary(*negated);
                Ok(true)
            }
            Node::Look {
                behind,
                negated,
                body,
            } => {
                self.charge(1)?;
                self.out.push_str(match (behind, negated) {
                    (false, false) => "(?=",
                    (false, true) => "(?!",
                    (true, false) => "(?<=",
                    (true, true) => "(?<!",
                });
                self.node(body)?;
                self.out.push(')');
                // Просмотр текста не ест — он обнуляем всегда.
                Ok(true)
            }
            Node::Backref { slot, icase } => {
                self.charge(1)?;
                if *icase {
                    self.out.push_str(&format!("(?i:\\{slot})"));
                } else {
                    self.out.push_str(&format!("\\{slot}"));
                }
                // Ссылка на пустую группу совпадает с пустотой.
                Ok(true)
            }
            Node::Atomic(body) => {
                self.charge(1)?;
                self.out.push_str("(?>");
                let nullable = self.node(body)?;
                self.out.push(')');
                Ok(nullable)
            }
            Node::Group { slot, body } => {
                self.charge(1)?;
                self.out.push_str(if slot.is_some() { "(" } else { "(?:" });
                let nullable = self.node(body)?;
                self.out.push(')');
                if let Some(slot) = slot {
                    self.nullable[*slot] = nullable;
                }
                Ok(nullable)
            }
            Node::Concat(parts) => {
                let mut nullable = true;
                for part in parts {
                    nullable &= self.node(part)?;
                }
                Ok(nullable)
            }
            Node::Alt(branches) => {
                self.charge(branches.len())?;
                self.out.push_str("(?:");
                let mut nullable = false;
                for (i, branch) in branches.iter().enumerate() {
                    if i > 0 {
                        self.out.push('|');
                    }
                    nullable |= self.node(branch)?;
                }
                self.out.push(')');
                Ok(nullable)
            }
            Node::Repeat {
                body,
                min,
                max,
                greedy,
                possessive,
            } => {
                // Квантор над заведомо пустым телом крейт не принимает
                // («target of repeat operator is invalid» на `(?:)*`), да
                // и печатать его незачем: пустое тело совпадает с
                // пустотой, а больше одной пустой итерации не различимо.
                // Группы внутри такого тела при `min > 0` участвуют —
                // тело печатается один раз без квантора.
                if is_purely_empty(body) {
                    if *min > 0 || has_group(body) && *greedy {
                        return self.node(body);
                    }
                    return Ok(true);
                }
                let copies = usize::try_from(max.unwrap_or_else(|| (*min).max(1))).unwrap_or(1);
                let before = self.cost;
                if *possessive {
                    // `а*+` печатается атомарной группой над обычным
                    // квантором: `(?>(?:а)*)` — та же семантика без
                    // отдачи назад.
                    self.out.push_str("(?>");
                }
                self.out.push_str("(?:");
                let nullable = self.node(body)?;
                self.out.push(')');
                let body_cost = self.cost - before;
                self.charge(body_cost.saturating_mul(copies.saturating_sub(1)))?;
                match (*min, *max) {
                    (0, None) => self.out.push('*'),
                    (1, None) => self.out.push('+'),
                    (0, Some(1)) => self.out.push('?'),
                    (n, None) => {
                        self.out.push_str(&format!("{{{n},}}"));
                    }
                    (n, Some(m)) if n == m => {
                        self.out.push_str(&format!("{{{n}}}"));
                    }
                    (n, Some(m)) => {
                        self.out.push_str(&format!("{{{n},{m}}}"));
                    }
                }
                if !greedy {
                    self.out.push('?');
                }
                if *possessive {
                    self.out.push(')');
                }
                Ok(*min == 0 || nullable)
            }
        }
    }
}

/// Ограничена ли длина возможных совпадений поддерева — требование
/// платформы к телу просмотра назад (якорь `REGEX.LOOK.BEHIND`).
/// Обратная ссылка консервативно считается неограниченной: её длина не
/// видна из структуры шаблона.
fn bounded(node: &Node) -> bool {
    match node {
        Node::Repeat { max: None, .. } | Node::Backref { .. } => false,
        Node::Repeat { body, .. } | Node::Atomic(body) | Node::Group { body, .. } => bounded(body),
        Node::Concat(parts) | Node::Alt(parts) => parts.iter().all(bounded),
        // Просмотр сам текста не ест, его тело на длину не влияет.
        _ => true,
    }
}

/// Тело, не порождающее текста ни при каком совпадении.
fn is_purely_empty(node: &Node) -> bool {
    match node {
        Node::Empty => true,
        Node::Group { body, .. } => is_purely_empty(body),
        Node::Concat(parts) => parts.iter().all(is_purely_empty),
        Node::Repeat { body, .. } => is_purely_empty(body),
        _ => false,
    }
}

/// Есть ли в поддереве захватывающая группа.
fn has_group(node: &Node) -> bool {
    match node {
        Node::Group { slot, body } => slot.is_some() || has_group(body),
        Node::Concat(parts) | Node::Alt(parts) => parts.iter().any(has_group),
        Node::Repeat { body, .. } => has_group(body),
        _ => false,
    }
}

// --- вход в UTF-8 и обратно ----------------------------------------------

/// Вход поиска: UTF-8-копия строки и таблица соответствия границ.
///
/// Строится ОДИН раз на вызов BSL-функции — `scan`/`scan_back` зовут
/// `find_at` в цикле, и конверсия внутри него превращала бы один проход в
/// квадратичный. Для чисто ASCII-строк таблица не строится: смещения
/// совпадают сами собой.
pub(crate) struct Haystack {
    text: String,
    /// `u8_of[i]` — байтовое смещение UTF-16-границы `i`; всего
    /// `len + 1` значений. Пуст у ASCII-строк. Середина суррогатной пары
    /// смотрит на КОНЕЦ символа: законная позиция `НачальнаяПозиция`
    /// не должна попасть внутрь UTF-8-последовательности.
    u8_of: Vec<u32>,
    /// Длина входа в код-юнитах.
    len: usize,
}

impl Haystack {
    pub(crate) fn new(units: &[u16]) -> Haystack {
        if units.iter().all(|&u| u < 0x80) {
            let text = units.iter().map(|&u| char::from(u as u8)).collect();
            return Haystack {
                text,
                u8_of: Vec::new(),
                len: units.len(),
            };
        }
        let mut text = String::with_capacity(units.len() * 3 / 2);
        let mut u8_of = Vec::with_capacity(units.len() + 1);
        let mut i = 0;
        while i < units.len() {
            u8_of.push(text.len() as u32);
            match cp_at(units, i) {
                Some((cp, width)) => {
                    let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
                    text.push(c);
                    if width == 2 {
                        // Середина пары указывает на конец символа.
                        u8_of.push(text.len() as u32);
                    }
                    i += width;
                }
                None => break,
            }
        }
        u8_of.push(text.len() as u32);
        Haystack {
            text,
            u8_of,
            len: units.len(),
        }
    }

    fn to_u8(&self, cu: usize) -> usize {
        if self.u8_of.is_empty() {
            cu
        } else {
            self.u8_of[cu] as usize
        }
    }

    fn to_cu(&self, byte: usize) -> usize {
        if self.u8_of.is_empty() {
            byte
        } else {
            // Последняя граница с этим смещением, а не первая: у середины
            // суррогатной пары смещение совпадает с КОНЦОМ символа, и
            // ответ обязан быть концом — иначе шаг по кодовым точкам
            // топтался бы на месте.
            self.u8_of.partition_point(|&off| (off as usize) <= byte) - 1
        }
    }
}

// --- скомпилированное выражение ------------------------------------------

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
    re: fancy_regex::Regex,
    /// Обнуляемость тел логических групп — для чистки фантомов.
    nullable: Vec<bool>,
    /// Число групп ВМЕСТЕ с нулевой.
    group_count: usize,
}

/// Ошибка крейта — в ловимую ошибку времени выполнения. Разбор шаблона
/// целиком наш, поэтому сюда попадают только пределы размера и прочие
/// внутренние отказы крейта на уже разобранном шаблоне.
fn engine_error(e: &fancy_regex::Error) -> RtError {
    bad(format!(
        "внутренняя ошибка движка регулярных выражений: {e}"
    ))
}

impl Regex {
    /// Разобрать шаблон и скомпилировать его рендер крейтом; флаги,
    /// ЗАДАННЫЕ СНАРУЖИ, — начальное состояние парсера.
    ///
    /// Платформа принимает регистронезависимость и многострочность двумя
    /// путями сразу: инлайн-флагами `(?i)`/`(?m)` внутри шаблона и
    /// отдельными аргументами `ИгнорироватьРегистр`/`МногострочныйПоиск` у
    /// всех четырёх функций поиска. Второй путь заведён начальным
    /// состоянием [`Flags`], а не дописыванием `(?i)` к тексту шаблона:
    /// приписка сдвинула бы позиции в сообщениях об ошибках разбора и
    /// зависела бы от области действия инлайн-флага, которая у платформы
    /// кончается на границе охватывающей группы (якорь `REGEX.FLAG.SCOPE`).
    ///
    /// # Errors
    ///
    /// [`RtError::Regex`] на любой ошибке разбора: незакрытая группа или
    /// класс, перевёрнутый диапазон, квантор без атома или с верхней
    /// границей меньше нижней, неподдержанная конструкция, а также на
    /// превышении пределов реализации — вложенности [`MAX_DEPTH`],
    /// счётчика [`MAX_REPEAT`] и развёрнутой оценки [`MAX_PROGRAM`].
    pub(crate) fn parse_with(pattern: &[u16], icase: bool, multiline: bool) -> RtResult<Regex> {
        Regex::build(pattern, icase, multiline, false)
    }

    /// То же, но скомпилированное для совпадения СО ВСЕЙ строкой: рендер
    /// оборачивается в `\A(?:…)\z`. Нужно одной `СтрПодобна…`, и только
    /// ей: обёртка «на всякий случай» удваивала бы компиляцию у всех.
    ///
    /// `\z` — СТРОГИЙ конец входа, и это не то же, что `$`: «аб» +
    /// перевод строки под `аб` НЕ подходит (измерено), хотя `б$` в нём
    /// находится. Перебор ветвей при этом остаётся за крейтом: `а|аб` на
    /// «аб» обязан дойти до второй ветви и ответить «подходит».
    ///
    /// # Errors
    ///
    /// Как у [`Regex::parse_with`].
    pub(crate) fn parse_full_with(
        pattern: &[u16],
        icase: bool,
        multiline: bool,
    ) -> RtResult<Regex> {
        Regex::build(pattern, icase, multiline, true)
    }

    fn build(pattern: &[u16], icase: bool, multiline: bool, full: bool) -> RtResult<Regex> {
        let src = decode(pattern);
        let mut parser = Parser::new(&src);
        parser.flags = Flags {
            icase,
            multiline,
            dotall: false,
        };
        let tree = parser.alternation()?;
        if parser.pos < src.len() {
            // Сюда приводит только лишняя закрывающая скобка: остальное
            // конкатенация съедает сама.
            return Err(bad("лишняя « ) » в шаблоне"));
        }
        let group_count = parser.groups + 1;
        let mut renderer = Renderer::new(group_count);
        if full {
            renderer.out.push_str(r"\A(?:");
        }
        renderer.node(&tree)?;
        if full {
            renderer.out.push_str(r")\z");
        }
        let mut builder = fancy_regex::RegexBuilder::new(&renderer.out);
        // Пределы: свой бюджет уже отработал на рендере, поэтому предел
        // крейта поднят до запаса, в который влезает всё, что бюджет
        // пропустил (например, `\w{5000}` — большой класс, повторённый
        // крейтом покопийно). Лимит бэктрекинга отключён сознательно —
        // довод в шапке модуля.
        builder.backtrack_limit(usize::MAX);
        builder.delegate_size_limit(256 << 20);
        let re = builder.build().map_err(|e| engine_error(&e))?;
        Ok(Regex {
            re,
            nullable: renderer.nullable,
            group_count,
        })
    }

    /// Сколько групп у выражения, считая нулевую.
    ///
    /// Поверхности BSL это число незачем — она берёт длину `Match::spans`
    /// уже найденного совпадения, — а вот разбору шаблона оно нужно как
    /// проверяемый инвариант, отсюда `#[cfg(test)]`.
    #[cfg(test)]
    pub(crate) fn group_count(&self) -> usize {
        self.group_count
    }

    /// Захваты крейта — в [`Match`] с код-юнитными границами и чисткой
    /// фантомов: бэктрекер крейта нормализует `(X+)?` в `(X*)`, и группа,
    /// чьё тело не совпадает с пустотой, приходит «участвовавшей пусто»
    /// там, где обязана быть неучаствовавшей. Обнуляемость тел знает
    /// рендерер, и пустой спан НЕобнуляемой группы — всегда фантом.
    fn convert(&self, hay: &Haystack, caps: &fancy_regex::Captures<'_, str>) -> Match {
        let spans = (0..self.group_count)
            .map(|i| {
                let m = caps.get(i)?;
                let (from, to) = (hay.to_cu(m.start()), hay.to_cu(m.end()));
                if i > 0 && from == to && !self.nullable[i] {
                    return None;
                }
                Some((from, to))
            })
            .collect();
        Match { spans }
    }

    /// Первое совпадение, начинающееся не левее `start`.
    ///
    /// Позиции — код-юниты UTF-16. `start` ожидается на границе кодовой
    /// точки; попавший в середину суррогатной пары — округляется к её
    /// концу.
    ///
    /// # Errors
    ///
    /// [`RtError::Regex`] на внутренней ошибке крейта: «не смогли» не
    /// прячется под «не нашли».
    pub(crate) fn find_at(&self, hay: &Haystack, start: usize) -> RtResult<Option<Match>> {
        if start > hay.len {
            return Ok(None);
        }
        let caps = self
            .re
            .captures_from_pos(hay.text.as_str(), hay.to_u8(start))
            .map_err(|e| engine_error(&e))?;
        Ok(caps.map(|caps| self.convert(hay, &caps)))
    }

    /// Совпадение, НАЧИНАЮЩЕЕСЯ ровно в `at`, — без поиска правее.
    ///
    /// Нужно проходу справа налево (`НаправлениеПоиска.СКонца`): он
    /// перебирает позиции начала сам. Якорного поиска у крейта нет,
    /// поэтому берётся первое совпадение не левее `at` с проверкой, что
    /// оно началось именно там: приоритет ветвей в позиции `at` у обоих
    /// способов один и тот же. Промах при этом стоит просмотра хвоста —
    /// на длинных строках проход справа налево квадратичен; если станет
    /// горячо, выход — якорный поиск нижнего слоя `regex-automata`.
    ///
    /// # Errors
    ///
    /// Как у [`Regex::find_at`].
    pub(crate) fn match_at(&self, hay: &Haystack, at: usize) -> RtResult<Option<Match>> {
        let Some(found) = self.find_at(hay, at)? else {
            return Ok(None);
        };
        // Нулевая группа совпадения есть всегда.
        if found.spans[0].is_some_and(|(from, _)| from == at) {
            Ok(Some(found))
        } else {
            Ok(None)
        }
    }

    /// Совпадает ли выражение со ВСЕЙ строкой целиком; выражение обязано
    /// быть собрано [`Regex::parse_full_with`].
    ///
    /// # Errors
    ///
    /// Как у [`Regex::find_at`].
    pub(crate) fn matches_full(&self, hay: &Haystack) -> RtResult<bool> {
        self.re
            .is_match(hay.text.as_str())
            .map_err(|e| engine_error(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn compile(pattern: &str) -> Regex {
        match Regex::parse_with(&utf16(pattern), false, false) {
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
        let haystack = Haystack::new(&hay);
        let found = compile(pattern)
            .find_at(&haystack, 0)
            .expect("движок не должен отказывать")?;
        let (from, to) = *found.spans.get(n)?.as_ref()?;
        Some(String::from_utf16_lossy(hay.get(from..to)?))
    }

    /// Границы всех групп совпадения.
    fn spans(pattern: &str, text: &str) -> Vec<Option<(usize, usize)>> {
        let hay = utf16(text);
        let haystack = Haystack::new(&hay);
        match compile(pattern)
            .find_at(&haystack, 0)
            .expect("движок не должен отказывать")
        {
            Some(found) => found.spans,
            None => panic!("шаблон «{pattern}» не нашёлся в «{text}»"),
        }
    }

    /// Текст ошибки разбора; шаблон, который разобрался, — провал теста.
    fn parse_error(pattern: &str) -> String {
        match Regex::parse_with(&utf16(pattern), false, false) {
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

    /// Состав `\w` — по измеренным точкам, а не по общей категории.
    ///
    /// Точки ниже — те самые, которыми набор снят с 8.3.27 (якорь
    /// `REGEX.CLASS.WORD`): входят свойство `Alphabetic` (римская единица
    /// `Ⅰ` категории Nl и буква в круге `Ⓐ` категории So — символы слова,
    /// хотя `\p{L}` их не содержит), все знаки `\p{M}` (ударение U+0301,
    /// висарга U+0903, охватывающий круг U+20DD), десятичные цифры и
    /// соединители `\p{Pc}` целиком, включая связку снизу U+203F. Не входят
    /// надстрочная двойка `²` (No) и невидимые U+200C/U+200D, которые
    /// документация ICU в `\w` перечисляет. Пробел и восклицательный знак
    /// платформе не задавались: они здесь контролем самой пробы.
    #[test]
    fn the_word_class_holds_the_measured_set() {
        for word in [
            "я", "7", "_", "\u{0301}", "\u{0903}", "\u{20DD}", "٠", "Ⅰ", "Ⓐ", "\u{203F}",
        ] {
            assert_eq!(find(r"\w", word).as_deref(), Some(word), "«{word}» — слово");
            assert_eq!(find(r"\W", word), None, "«{word}» — не «не-слово»");
        }
        for other in ["²", "\u{200C}", "\u{200D}", " ", "!"] {
            assert_eq!(find(r"\w", other), None, "«{other}» — не слово");
            assert_eq!(
                find(r"\W", other).as_deref(),
                Some(other),
                "«{other}» — «не-слово»"
            );
        }
        // Границы слова читают тот же набор: между буквой и ударением
        // границы нет, между буквой и надстрочной двойкой — есть.
        assert_eq!(find(r"а\b", "а\u{0301}"), None);
        assert_eq!(find(r"а\b", "а_"), None);
        assert_eq!(find(r"а\b", "а²").as_deref(), Some("а"));
        assert_eq!(find(r"а\b", "а.").as_deref(), Some("а"));
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
        let haystack = Haystack::new(&hay);
        let regex = compile("б+");
        // Со сдвигом — первое совпадение правее старта.
        assert_eq!(
            regex
                .find_at(&haystack, 3)
                .unwrap()
                .and_then(|m| m.spans[0]),
            Some((4, 7))
        );
        assert_eq!(regex.find_at(&haystack, hay.len()).unwrap(), None);
        assert_eq!(regex.find_at(&haystack, hay.len() + 5).unwrap(), None);
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
            regex
                .find_at(&Haystack::new(&lonely), 0)
                .unwrap()
                .and_then(|m| m.spans[0]),
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

    /// Пустой проход ОГРАНИЧЕННЫЙ квантор не обрывает.
    ///
    /// Формы ниже — те, на которых это вообще видно: тело умеет совпасть с
    /// пустотой, поэтому вопрос не «где нашли» (нулевая группа во всех
    /// строках одна и та же), а какая итерация запомнилась группой. Все
    /// шесть строк СНЯТЫ С ПЛАТФОРМЫ 8.3.27 (якорь `REGEX.REPEAT.EMPTY`,
    /// блок «пустой проход …» в measure-regex.bsl), и ответ на всех шести
    /// один: текст остаётся за той копией, которой его хватило, — группа 1
    /// начинается там же, где всё совпадение. Согласия классиков здесь нет
    /// и близко, и с платформой они сверены поимённо: `python3 re` (захват
    /// сдвинут вправо на всех шести) и `perl` (то же, а на трёх строках ещё
    /// правее, `2,2`) не совпадают с ней НИ НА ОДНОЙ строке, ECMAScript —
    /// на трёх (первой, второй и шестой), PCRE2 — на всех шести. На этом
    /// месте платформа отвечает ровно как PCRE2.
    #[test]
    fn an_empty_pass_of_a_bounded_quantifier_keeps_the_text_like_the_platform() {
        /// Шаблон, текст и ожидаемые границы всех групп.
        type Case<'a> = (&'a str, &'a str, &'a [Option<(usize, usize)>]);

        let table: &[Case] = &[
            ("(б{0,1}?){0,2}в", "бв", &[Some((0, 2)), Some((0, 1))]),
            ("(б{0,2}?){1,3}в", "бв", &[Some((0, 2)), Some((0, 1))]),
            ("(б{0,2}?){1,3}в", "ббв", &[Some((0, 3)), Some((0, 2))]),
            ("(б{0,2}?){2,4}в", "ббв", &[Some((0, 3)), Some((0, 2))]),
            ("(а*?){3,5}$", "аа", &[Some((0, 2)), Some((0, 2))]),
            ("((?:|б)){0,2}$", "б", &[Some((0, 1)), Some((0, 1))]),
        ];
        for (pattern, text, expected) in table {
            assert_eq!(
                spans(pattern, text),
                *expected,
                "шаблон «{pattern}» на «{text}»"
            );
        }
    }

    /// Большой повтор в бюджете обязан и компилироваться крейтом.
    ///
    /// Прежний тест белого ящика проверял разделяемую таблицу классов
    /// собственного компилятора; у крейта такой таблицы нет — сорок тысяч
    /// копий класса он разворачивает честно, и предел его размера поднят
    /// в [`Regex::build`] так, чтобы всё, что пропустил бюджет
    /// [`MAX_PROGRAM`], компилировалось. Этот тест держит именно ту
    /// границу: бюджет пропускает — крейт не отказывает.
    #[test]
    fn the_rendered_pattern_grows_with_the_source_not_the_expansion() {
        compile("(?:[абв]){40000}");
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
        assert!(parse_error(r"\0").contains("восьмеричных"));
        assert!(parse_error(r"[\1]").contains("внутри класса"));
        assert!(parse_error("(?<имя>а)").contains("не поддержана"));
        assert!(parse_error("а(?#х)б").contains("не поддержана"));
        assert!(parse_error("(?x)а б").contains("не поддержана"));
        assert!(parse_error(r"а\Q.\Eб").contains("не поддержано"));
        assert!(parse_error("(?<=а+)б").contains("ограниченной длины"));
        assert!(parse_error("(?<=а*)б").contains("ограниченной длины"));
        assert!(parse_error("а(?=б)?").contains("квантор на просмотр"));
        assert!(parse_error("[а-в&&]").contains("пустой операнд"));
    }

    /// Просмотры — по строкам платформенного контракта
    /// `measure-regex2.platform.txt`: сам просмотр текста не ест, захваты
    /// из него видны, просмотр назад ограниченной длины работает во всех
    /// измеренных формах.
    #[test]
    fn lookaround_matches_the_platform_contract() {
        assert_eq!(find("а(?=б)", "аб").as_deref(), Some("а"));
        assert_eq!(find("а(?=б)", "ав"), None);
        assert_eq!(spans("(а(?=б))", "аб"), vec![Some((0, 1)), Some((0, 1))]);
        assert_eq!(find("а(?!б)", "ав").as_deref(), Some("а"));
        assert_eq!(find("а(?!б)", "аб"), None);
        assert_eq!(spans("а(?=(б))", "аб"), vec![Some((0, 1)), Some((1, 2))]);
        assert_eq!(spans("(?!(б))а", "а"), vec![Some((0, 1)), None]);
        assert_eq!(find("(?<=а)б", "аб").as_deref(), Some("б"));
        assert_eq!(find("(?<=а)б", "хб"), None);
        assert_eq!(find("(?<!а)б", "хб").as_deref(), Some("б"));
        assert_eq!(find("(?<=а{1,3})б", "аааб").as_deref(), Some("б"));
        assert_eq!(find("(?<=а|аа)б", "ааб").as_deref(), Some("б"));
        assert_eq!(spans("(?<=(а))б", "аб"), vec![Some((1, 2)), Some((0, 1))]);
        assert_eq!(find(r"(?<=\bа)б", "аб").as_deref(), Some("б"));
    }

    /// Обратные ссылки — как PCRE, и это ИЗМЕРЕНО: ссылка на
    /// неучаствовавшую группу проваливает совпадение, а не подставляет
    /// пустоту; свёртка `(?i)` действует и на ссылку; ссылка раньше своей
    /// группы законна и проваливается.
    #[test]
    fn backrefs_match_the_platform_contract() {
        assert_eq!(find(r"(а)\1", "аа").as_deref(), Some("аа"));
        assert_eq!(find(r"(а)\1", "аб"), None);
        assert_eq!(find(r"(?i)(а)\1", "аА").as_deref(), Some("аА"));
        assert_eq!(find(r"(а)\1", "аА"), None);
        assert_eq!(find(r"(б)?а\1", "а"), None);
        assert_eq!(find(r"(б?)а\1", "а").as_deref(), Some("а"));
        assert_eq!(
            spans(r"((а)|б)+\2", "баа"),
            vec![Some((0, 3)), Some((1, 2)), Some((1, 2))]
        );
        assert_eq!(find(r"\1(а)", "аа"), None);
    }

    /// `(?s)` берёт все семь терминаторов, действует групповой формой и
    /// снимается — по якорю `REGEX.DOTALL`.
    #[test]
    fn dotall_takes_every_terminator() {
        for terminator in ["\n", "\r", "\x0B", "\x0C", "\u{85}", "\u{2028}", "\u{2029}"] {
            let text = format!("а{terminator}б");
            assert_eq!(
                find("(?s)а.б", &text).as_deref(),
                Some(text.as_str()),
                "терминатор U+{:04X}",
                terminator.chars().next().unwrap() as u32
            );
        }
        assert_eq!(find("а(?s:.)б", "а\nб").as_deref(), Some("а\nб"));
        assert_eq!(find("а(?s:.)б.в", "а\nб\nв"), None);
        assert_eq!(find("(?s)а(?-s:.)б", "а\nб"), None);
        assert_eq!(find("а.б", "а\nб"), None);
    }

    /// Групповые и снимающие формы флагов — область до конца группы.
    #[test]
    fn flag_groups_scope_like_the_platform() {
        assert_eq!(find("(?i:а)б", "Аб").as_deref(), Some("Аб"));
        assert_eq!(find("(?i:а)б", "АБ"), None);
        assert_eq!(find("(?i)а(?-i:б)", "Аб").as_deref(), Some("Аб"));
        assert_eq!(find("(?i)а(?-i:б)", "АБ"), None);
        assert_eq!(find("(?i)а(?-i)б", "АБ"), None);
        assert_eq!(find("(?m:а)^б", "а\nб"), None);
        assert_eq!(find("(?m:^б)", "а\nб").as_deref(), Some("б"));
    }

    /// Притяжательные кванторы и атомарные группы назад не отдают — по
    /// якорю `REGEX.POSSESSIVE`.
    #[test]
    fn possessive_and_atomic_do_not_give_back() {
        assert_eq!(find("а*+а", "аа"), None);
        assert_eq!(find("а++а", "аа"), None);
        assert_eq!(find("а?+а", "а"), None);
        assert_eq!(find("а{1,2}+а", "ааа").as_deref(), Some("ааа"));
        assert_eq!(find("(?>а|аб)в", "абв"), None);
        assert_eq!(find("(?>а)б", "аб").as_deref(), Some("аб"));
        assert_eq!(find("(?>а)+б", "аб").as_deref(), Some("аб"));
    }

    /// Операции над множествами в классе — по якорю `REGEX.CLASSOPS`:
    /// пересечение и вложение работают, а `[а-я-[б]]` — объединение с
    /// литеральным минусом, не разность.
    #[test]
    fn class_operations_follow_the_measured_semantics() {
        assert_eq!(find("[а-в&&[^б]]", "в").as_deref(), Some("в"));
        assert_eq!(find("[а-в&&[^б]]", "б"), None);
        assert_eq!(find("[а-в&&б-г]", "б").as_deref(), Some("б"));
        assert_eq!(find("[[аб]в]", "в").as_deref(), Some("в"));
        assert_eq!(find("[[^аб]]", "х").as_deref(), Some("х"));
        assert_eq!(find("[а-я-[б]]", "а").as_deref(), Some("а"));
        assert_eq!(find("[а-я-[б]]", "б").as_deref(), Some("б"));
        assert_eq!(find("(?i)[а-в&&[^б]]", "Б"), None);
        assert_eq!(find("[а&]", "&").as_deref(), Some("&"));
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
    /// Ожидания взяты НЕ с платформы: это правила приоритета, общие для
    /// всех движков с бэктрекингом (слева направо, жадный берёт больше,
    /// ленивый меньше, группа помнит последнюю удачную итерацию), и каждая
    /// строка сверена с независимой реализацией ТЕХ ЖЕ правил — `python3 -c
    /// "import re; m = re.search(шаблон, текст); print([m.span(i) …])"`.
    /// Места, где движки расходятся между собой, а значит и с платформой,
    /// сняты отдельно контрактом
    /// `tests/conformance/measure/measure-regex.bsl` (якоря `REGEX.*`) —
    /// именно там, а не здесь, и стоит `python3 re` опровергать.
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
