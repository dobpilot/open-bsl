use std::collections::HashMap;

/// Идентификатор интернированного имени поля. Сравнение имён при доступе к
/// полю сводится к сравнению `NameId` (`u32`), а не строк.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(u32);

impl NameId {
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Обратная сборка из индекса. Нужна ровно одному потребителю —
    /// загрузчику текстового байт-кода (`bsl_bytecode::text`), который
    /// читает уже готовую таблицу имён из файла и восстанавливает по ней
    /// `NameId` инструкций. Ни компилятору, ни VM это не нужно: там
    /// `NameId` всегда приходит из интернера.
    pub fn from_index(index: u32) -> Self {
        NameId(index)
    }
}

/// Интернер имён полей. Используется компилятором байт-кода (не VM в
/// рантайме): все имена полей известны на этапе компиляции модуля, поэтому
/// интернирование — целиком компиляционный процесс, а VM только читает
/// готовую таблицу `Program::names` по индексу для сообщений об ошибках.
///
/// Доступ к полю регистронезависим (`а.ИМЯ` и `а.имя` — одно поле), но
/// оригинальное написание первого вхождения сохраняется.
#[derive(Debug, Default)]
pub struct NameInterner {
    names: Vec<String>,
    index: HashMap<String, NameId>,
}

/// Верхний регистр для ключа индекса. ASCII и кириллица минуют таблицы
/// Unicode — имена полей BSL почти целиком из них и состоят, а полный
/// `to_uppercase` с двоичным поиском по таблицам был заметен в профиле
/// на миллионах `Вставить`. Прочие символы уходят в полный путь. Обе
/// операции интернера обязаны нормализовать ключ ЭТОЙ функцией — иначе
/// `intern` и `lookup` разойдутся на одном имени.
fn key_upper(name: &str) -> String {
    // Чистый ASCII — векторизованный путь стандартной библиотеки; он
    // быстрее посимвольного цикла ниже.
    if name.is_ascii() {
        return name.to_ascii_uppercase();
    }
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            'a'..='z' => out.push(((ch as u8) - (b'a' - b'A')) as char),
            'а'..='я' => out.push(char::from_u32(ch as u32 - 0x20).unwrap_or(ch)),
            'ё' => out.push('Ё'),
            '\0'..='\u{7f}' | 'А'..='Я' | 'Ё' => out.push(ch),
            _ => out.extend(ch.to_uppercase()),
        }
    }
    out
}

impl NameInterner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Затравка рантайм-интернера уже готовой (компиляционной) таблицей
    /// имён — `Вставить`/`Свойство` на структуре могут получить строковый
    /// ключ, которого не было ни в одном литерале модуля, и им нужно
    /// продолжить ИМЕННО эту нумерацию `NameId`, а не начать с нуля заново
    /// (иначе рантайм-`NameId` столкнулись бы с компиляционными).
    pub fn from_existing(names: Vec<String>) -> Self {
        let index = names
            .iter()
            .enumerate()
            .map(|(i, n)| (key_upper(n), NameId(i as u32)))
            .collect();
        NameInterner { names, index }
    }

    pub fn intern(&mut self, name: &str) -> NameId {
        let key = key_upper(name);
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = NameId(self.names.len() as u32);
        self.names.push(name.to_string());
        self.index.insert(key, id);
        id
    }

    /// Поиск БЕЗ интернирования: `None` — такого имени в таблице ещё нет.
    ///
    /// Нужен `ЗаполнитьЗначенияСвойств`, который проверяет, есть ли у
    /// приёмника поле с именем свойства источника. Поле живой структуры
    /// всегда уже интернировано (иначе его нечем было бы завести), поэтому
    /// отсутствие имени в таблице — это и есть ответ «такого поля нет».
    /// Через [`intern`](Self::intern) тот же вопрос задавать нельзя: он
    /// растил бы таблицу на каждое чужое имя, а имена не выселяются.
    pub fn lookup(&self, name: &str) -> Option<NameId> {
        self.index.get(&key_upper(name)).copied()
    }

    /// Оригинальное написание имени. Нужно там, где `NameId` приходится
    /// возвращать пользовательскому коду обратно СТРОКОЙ — `Для Каждого` по
    /// `Структура` отдаёт `КлючИЗначение.Ключ` как `Строка`, а внутри поле
    /// хранится только идентификатором.
    pub fn name(&self, id: NameId) -> Option<&str> {
        self.names.get(id.index()).map(|s| s.as_str())
    }

    /// Готовая таблица `NameId -> оригинальное написание`, для встраивания
    /// в `Program`.
    pub fn into_names(self) -> Vec<String> {
        self.names
    }
}
