use std::collections::HashMap;

/// Идентификатор интернированного имени поля. Сравнение имён при доступе к
/// полю сводится к сравнению `NameId` (`u32`), а не строк.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(u32);

impl NameId {
    pub fn index(self) -> usize {
        self.0 as usize
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
            .map(|(i, n)| (n.to_uppercase(), NameId(i as u32)))
            .collect();
        NameInterner { names, index }
    }

    pub fn intern(&mut self, name: &str) -> NameId {
        let key = name.to_uppercase();
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = NameId(self.names.len() as u32);
        self.names.push(name.to_string());
        self.index.insert(key, id);
        id
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
