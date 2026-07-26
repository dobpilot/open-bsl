use std::fmt;
use std::rc::Rc;

/// Строка BSL — код-юниты UTF-16, как в самой 1С (COM/Windows-строки), не
/// UTF-8. `СтрДлина` считает код-юниты, а не кодовые точки: суррогатная
/// пара (символ вне BMP, например эмодзи) даёт 2, не 1 — на UTF-8 все
/// индексные функции разъехались бы именно на таких символах.
///
/// Сравнение — по содержимому: строки в BSL, в отличие от `Массив`/
/// `Структура`, тип значения, а не ссылочный.
///
/// Интернирование коротких строк (брифом заявлено как способ свести
/// сравнение к сравнению указателей) сюда сознательно не входит — это
/// чисто оптимизационная надстройка над уже корректным посимвольным
/// сравнением ниже, и, как остальные оптимизации в этом проекте, ждёт
/// профилирования (см. план M10), а не добавляется заранее "на всякий".
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BslString(Rc<[u16]>);

impl BslString {
    pub fn from_str(s: &str) -> Self {
        let units: Vec<u16> = s.encode_utf16().collect();
        BslString(units.into())
    }

    pub fn units(&self) -> &[u16] {
        &self.0
    }

    /// `СтрДлина`/`StrLen` — число код-юнитов UTF-16, НЕ кодовых точек.
    pub fn len_utf16(&self) -> usize {
        self.0.len()
    }

    pub fn concat(&self, other: &Self) -> Self {
        let mut v = Vec::with_capacity(self.0.len() + other.0.len());
        v.extend_from_slice(&self.0);
        v.extend_from_slice(&other.0);
        BslString(v.into())
    }

    /// `Сред`/`Mid`: `start_1based` — позиция первого символа (1 = начало
    /// строки), `len` — длина в код-юнитах. Индекс, режущий суррогатную
    /// пару пополам, даёт "битую" последовательность — так же ведёт себя и
    /// сама 1С (строка внутри неё — ровно такой же буфер код-юнитов), это
    /// не отсебятина реализации.
    pub fn substring(&self, start_1based: usize, len: usize) -> Self {
        let n = self.0.len();
        if start_1based == 0 || start_1based > n {
            return BslString(Vec::new().into());
        }
        let start = start_1based - 1;
        let end = (start + len).min(n);
        BslString(self.0[start..end].to_vec().into())
    }

    pub fn left(&self, len: usize) -> Self {
        self.substring(1, len)
    }

    pub fn right(&self, len: usize) -> Self {
        let n = self.0.len();
        let take = len.min(n);
        BslString(self.0[n - take..].to_vec().into())
    }

    /// Через `char` (декодируя код-юниты), не по код-юниту напрямую —
    /// иначе смена регистра могла бы разломать суррогатные пары.
    pub fn to_uppercase(&self) -> Self {
        BslString::from_str(&self.to_string().to_uppercase())
    }

    pub fn to_lowercase(&self) -> Self {
        BslString::from_str(&self.to_string().to_lowercase())
    }

    pub fn trim(&self) -> Self {
        BslString::from_str(self.to_string().trim())
    }
}

impl fmt::Display for BslString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf16_lossy(&self.0))
    }
}

impl fmt::Debug for BslString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogate_pair_counts_as_two_code_units() {
        // брифом явно указано: суррогатная пара даёт 2.
        let s = BslString::from_str("😀");
        assert_eq!(s.len_utf16(), 2);
        let s = BslString::from_str("a😀b");
        assert_eq!(s.len_utf16(), 4);
    }

    #[test]
    fn bmp_characters_count_one_per_code_unit() {
        assert_eq!(BslString::from_str("привет").len_utf16(), 6);
        assert_eq!(BslString::from_str("").len_utf16(), 0);
    }

    #[test]
    fn concat_matches_plus_operator_semantics() {
        let a = BslString::from_str("Привет, ");
        let b = BslString::from_str("мир!");
        assert_eq!(a.concat(&b).to_string(), "Привет, мир!");
    }

    #[test]
    fn left_right_mid_are_1_based_code_unit_slices() {
        let s = BslString::from_str("Привет");
        assert_eq!(s.left(3).to_string(), "При");
        assert_eq!(s.right(3).to_string(), "вет");
        assert_eq!(s.substring(2, 3).to_string(), "рив");
    }

    #[test]
    fn equality_is_by_content_not_identity() {
        let a = BslString::from_str("x");
        let b = BslString::from_str("x");
        assert_eq!(a, b);
    }

    #[test]
    fn case_conversion_does_not_break_on_non_ascii() {
        assert_eq!(BslString::from_str("привет").to_uppercase().to_string(), "ПРИВЕТ");
        assert_eq!(BslString::from_str("ПРИВЕТ").to_lowercase().to_string(), "привет");
    }
}
