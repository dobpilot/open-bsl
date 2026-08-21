//! Инструкции препроцессора: условная компиляция (`#Если`) и области
//! (`#Область`).
//!
//! Поведение снято на 8.3.27.2130, разбор замеров — в `docs/bsl-preproc.md`.
//! Три факта оттуда определяют устройство модуля:
//!
//! * директива режет ВЫРАЖЕНИЕ, а не только оператор (`Рез = "а"`,
//!   `#Если … Тогда`, `+ "б"`, `#КонецЕсли` даёт `аб`), поэтому работа идёт
//!   на уровне токенов, а не разбора;
//! * выключенную ветку платформа НЕ ЛЕКСИРУЕТ — она терпит там не-токены
//!   вроде `@` и `~`, — поэтому мёртвый текст пропускается построчным
//!   сканом до парной директивы;
//! * неизвестный символ — ложь, а не ошибка, и регистр имени не важен.

/// Символы условной компиляции: закрытый список пар «русское имя,
/// английское имя».
///
/// Измерены распознаваемыми `Сервер`/`Server`, `НаСервере`/`AtServer`,
/// `Клиент`/`Client`, `НаКлиенте`/`AtClient` и
/// `ТолстыйКлиентУправляемоеПриложение`/`ThickClientManagedApplication`.
/// Остальные английские написания взяты из документации платформы: в
/// контексте стенда замеров они ложны при любом прочтении, поэтому
/// «распознано и ложно» там неотличимо от «неизвестно и ложно». Цена
/// ошибки в таком написании мала — имя просто окажется неизвестным, то
/// есть ложным, а это ровно измеренное поведение неизвестных имён.
const SYMBOLS: &[(&str, &str)] = &[
    ("Сервер", "Server"),
    ("НаСервере", "AtServer"),
    ("Клиент", "Client"),
    ("НаКлиенте", "AtClient"),
    ("ТонкийКлиент", "ThinClient"),
    ("ВебКлиент", "WebClient"),
    (
        "ТолстыйКлиентУправляемоеПриложение",
        "ThickClientManagedApplication",
    ),
    (
        "ТолстыйКлиентОбычноеПриложение",
        "ThickClientOrdinaryApplication",
    ),
    ("ВнешнееСоединение", "ExternalConnection"),
    ("МобильноеПриложениеКлиент", "MobileAppClient"),
    ("МобильноеПриложениеСервер", "MobileAppServer"),
];

/// Индексы символов, истинных по умолчанию: сервер плюс внешнее соединение.
///
/// open-bsl — это BSL, исполняемый внешней программой без интерфейса, и из
/// контекстов платформы ближе всего именно внешнее соединение. Значение
/// символа вне контекста развёртывания не определено, поэтому платформой
/// этот набор не проверяется: она может ответить лишь про свои контексты
/// (в серверном фрагменте формы, например, `Клиент` истинен).
const DEFAULT_ON: &[usize] = &[0, 1, 8];

/// Набор символов условной компиляции для одной компиляции.
///
/// `Copy`: внутри — массив из одиннадцати флагов, и набор ездит по
/// рантайму рядом с реестром компонентов, чтобы динамический код
/// (`Выполнить`/`Вычислить`) видел тот же контекст, что и статический.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreprocSymbols {
    on: [bool; SYMBOLS.len()],
}

impl PreprocSymbols {
    /// Набор по умолчанию: `Сервер`, `НаСервере`, `ВнешнееСоединение`.
    #[must_use]
    pub fn new() -> Self {
        let mut on = [false; SYMBOLS.len()];
        for &i in DEFAULT_ON {
            on[i] = true;
        }
        PreprocSymbols { on }
    }

    /// Набор, в котором ложно всё.
    #[must_use]
    pub fn none() -> Self {
        PreprocSymbols {
            on: [false; SYMBOLS.len()],
        }
    }

    /// Включает или выключает символ по любому из двух его написаний.
    /// Неизвестное имя игнорируется: список символов закрыт, а
    /// нераспознанное имя в условии и без того ложно.
    pub fn set(&mut self, name: &str, value: bool) {
        if let Some(i) = index_of(name) {
            self.on[i] = value;
        }
    }

    /// Истинен ли символ. Неизвестное имя — ложь (измерено).
    #[must_use]
    pub fn is_on(&self, name: &str) -> bool {
        index_of(name).is_some_and(|i| self.on[i])
    }
}

impl Default for PreprocSymbols {
    fn default() -> Self {
        Self::new()
    }
}

/// Регистронезависимое сравнение имён. `eq_ignore_ascii_case` здесь не
/// годится: имена кириллические, а он складывает только латиницу.
fn same_name(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.to_lowercase() == b.to_lowercase()
}

fn index_of(name: &str) -> Option<usize> {
    SYMBOLS
        .iter()
        .position(|&(ru, en)| same_name(name, ru) || same_name(name, en))
}

/// Ключевые слова строки директивы, каждое парой «русское, английское».
const KW_IF: (&str, &str) = ("Если", "If");
const KW_ELSIF: (&str, &str) = ("ИначеЕсли", "ElsIf");
const KW_ELSE: (&str, &str) = ("Иначе", "Else");
const KW_ENDIF: (&str, &str) = ("КонецЕсли", "EndIf");
const KW_REGION: (&str, &str) = ("Область", "Region");
const KW_ENDREGION: (&str, &str) = ("КонецОбласти", "EndRegion");
const KW_THEN: (&str, &str) = ("Тогда", "Then");
const KW_NOT: (&str, &str) = ("НЕ", "NOT");
const KW_AND: (&str, &str) = ("И", "AND");
const KW_OR: (&str, &str) = ("ИЛИ", "OR");

/// Директивы расширений конфигурации. Их у open-bsl нет, и молчаливо
/// пропускать их нельзя: смысл такого кода в расширении прямо
/// противоположен прочтению «просто текст».
const EXTENSION_KW: &[(&str, &str)] = &[
    ("Вставка", "Insert"),
    ("КонецВставки", "EndInsert"),
    ("Удаление", "Delete"),
    ("КонецУдаления", "EndDelete"),
];

fn is_kw(word: &str, kw: (&str, &str)) -> bool {
    same_name(word, kw.0) || same_name(word, kw.1)
}

/// Что за директива стоит в строке.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Directive {
    If,
    ElsIf,
    Else,
    EndIf,
    Region,
    EndRegion,
}

impl Directive {
    fn classify(word: &str) -> Option<Directive> {
        // Порядок важен: `ИначеЕсли` проверяется до `Иначе`, иначе
        // префиксного совпадения не будет вовсе — сравнение идёт по
        // целому слову, но перечислять от длинного к короткому нагляднее.
        if is_kw(word, KW_ELSIF) {
            Some(Directive::ElsIf)
        } else if is_kw(word, KW_IF) {
            Some(Directive::If)
        } else if is_kw(word, KW_ELSE) {
            Some(Directive::Else)
        } else if is_kw(word, KW_ENDIF) {
            Some(Directive::EndIf)
        } else if is_kw(word, KW_ENDREGION) {
            Some(Directive::EndRegion)
        } else if is_kw(word, KW_REGION) {
            Some(Directive::Region)
        } else {
            None
        }
    }
}

/// Разбирает строку директивы, начинающуюся с `#` в позиции `at`.
///
/// Возвращает слово после `#`, «хвост» строки без комментария и смещение
/// конца строки. Ошибок не даёт СОЗНАТЕЛЬНО: в активном тексте строгость
/// наводит вызывающий, а при пропуске выключенной ветки её быть не должно
/// вовсе — платформа мёртвый текст не разбирает.
pub(crate) fn split_line(src: &str, at: usize) -> (&str, &str, usize) {
    let line_end = src[at..].find('\n').map_or(src.len(), |i| at + i);
    let line = &src[at..line_end];
    let body = line[1..].trim_start();
    let word_end = body
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(body.len());
    let (word, rest) = body.split_at(word_end);
    // Комментарий после директивы измерен допустимым.
    let rest = rest.split("//").next().unwrap_or("").trim();
    (word, rest, line_end)
}

/// Директива ли это, и какая. `None` — слово не опознано.
pub(crate) fn classify(word: &str) -> Option<Directive> {
    Directive::classify(word)
}

/// Инструкция расширения конфигурации.
pub(crate) fn is_extension(word: &str) -> bool {
    EXTENSION_KW.iter().any(|&kw| is_kw(word, kw))
}

/// Проверяет хвост директивы, у которой хвоста быть не должно.
/// Посторонний текст после `#КонецЕсли` — измеренная ошибка компиляции.
pub(crate) fn expect_empty_tail(rest: &str) -> Result<(), &'static str> {
    if rest.is_empty() {
        Ok(())
    } else {
        Err("после инструкции препроцессора остался посторонний текст")
    }
}

/// Проверяет имя области: это ОДИН идентификатор целиком.
///
/// Измерено на 8.3.27: `#Область Имя.Точка` и `#Область 1Имя` не
/// компилируются, причём указатель ошибки стоит в начале имени, а не на
/// точке, — платформа берёт весь хвост строки и проверяет его как один
/// идентификатор. `#Область Две Слова` тоже отвергается, а
/// `Имя_Второе2` и `LatinName` принимаются. Безымянная область —
/// ошибка отдельным замером.
pub(crate) fn expect_region_name(rest: &str) -> Result<(), &'static str> {
    let mut chars = rest.chars();
    let ok = match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => chars.all(|c| c.is_alphanumeric() || c == '_'),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("имя области должно быть одним идентификатором")
    }
}

/// Вычисляет условие директивы `#Если`/`#ИначеЕсли`.
///
/// Хвост обязан заканчиваться словом `Тогда`: без него платформа не
/// компилирует модуль (измерено).
pub(crate) fn eval_condition(rest: &str, symbols: &PreprocSymbols) -> Result<bool, &'static str> {
    let words = tokenize(rest)?;
    let last = words.last().ok_or("в инструкции «#Если» нет условия")?;
    let Word::Name(name) = last else {
        return Err("инструкция «#Если» должна заканчиваться словом «Тогда»");
    };
    if !is_kw(name, KW_THEN) {
        return Err("инструкция «#Если» должна заканчиваться словом «Тогда»");
    }
    let mut p = Parser {
        words: &words[..words.len() - 1],
        pos: 0,
        symbols,
    };
    let value = p.or_expr()?;
    if p.pos != p.words.len() {
        return Err("лишний текст в условии инструкции препроцессора");
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Word<'a> {
    Name(&'a str),
    LParen,
    RParen,
}

fn tokenize(rest: &str) -> Result<Vec<Word<'_>>, &'static str> {
    let mut out = Vec::new();
    let mut it = rest.char_indices().peekable();
    while let Some(&(i, c)) = it.peek() {
        if c.is_whitespace() {
            it.next();
        } else if c == '(' {
            it.next();
            out.push(Word::LParen);
        } else if c == ')' {
            it.next();
            out.push(Word::RParen);
        } else if c.is_alphanumeric() || c == '_' {
            let mut end = rest.len();
            for (j, d) in it.by_ref() {
                if !(d.is_alphanumeric() || d == '_') {
                    end = j;
                    break;
                }
            }
            out.push(Word::Name(&rest[i..end]));
            // Символ, оборвавший имя, ещё не разобран: перезапускаем обход
            // с этой позиции, иначе скобка после имени потеряется.
            if end < rest.len() {
                let tail = tokenize(&rest[end..])?;
                out.extend(tail);
            }
            return Ok(out);
        } else {
            return Err("недопустимый символ в условии инструкции препроцессора");
        }
    }
    Ok(out)
}

struct Parser<'a, 'b> {
    words: &'a [Word<'a>],
    pos: usize,
    symbols: &'b PreprocSymbols,
}

impl Parser<'_, '_> {
    fn peek_name(&self) -> Option<&str> {
        match self.words.get(self.pos) {
            Some(Word::Name(n)) => Some(n),
            _ => None,
        }
    }

    fn eat_kw(&mut self, kw: (&str, &str)) -> bool {
        if self.peek_name().is_some_and(|n| is_kw(n, kw)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn or_expr(&mut self) -> Result<bool, &'static str> {
        let mut value = self.and_expr()?;
        while self.eat_kw(KW_OR) {
            // Без короткого замыкания: правая часть всё равно должна быть
            // разобрана, иначе синтаксическая ошибка в ней пройдёт молча.
            value = self.and_expr()? || value;
        }
        Ok(value)
    }

    fn and_expr(&mut self) -> Result<bool, &'static str> {
        let mut value = self.not_expr()?;
        while self.eat_kw(KW_AND) {
            value = self.not_expr()? && value;
        }
        Ok(value)
    }

    fn not_expr(&mut self) -> Result<bool, &'static str> {
        if self.eat_kw(KW_NOT) {
            return Ok(!self.not_expr()?);
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<bool, &'static str> {
        match self.words.get(self.pos) {
            Some(Word::LParen) => {
                self.pos += 1;
                let value = self.or_expr()?;
                if self.words.get(self.pos) != Some(&Word::RParen) {
                    return Err("в условии инструкции препроцессора не закрыта скобка");
                }
                self.pos += 1;
                Ok(value)
            }
            Some(Word::Name(name)) => {
                let name = *name;
                if is_kw(name, KW_AND) || is_kw(name, KW_OR) {
                    return Err("в условии инструкции препроцессора пропущен операнд");
                }
                self.pos += 1;
                Ok(self.symbols.is_on(name))
            }
            _ => Err("в условии инструкции препроцессора пропущен операнд"),
        }
    }
}
