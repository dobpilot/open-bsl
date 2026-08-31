//! Перевод байтового смещения в номер строки.
//!
//! Единственный источник этого перевода. Лексер и диагностики работают
//! на байтовых смещениях, а образу байт-кода и отладчику нужны строки;
//! посчитать `src[..offset].matches('\n').count()` на каждом операторе
//! означало бы квадрат по длине файла, а завести вторую такую же
//! функцию рядом — гарантировать, что однажды они разойдутся на краю.

/// Начала строк исходника, чтобы переводить смещение в номер строки за
/// логарифм, а не за линию.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Смещение начала каждой строки. Первый элемент всегда `0`, поэтому
    /// вектор непуст и у пустого текста есть строка 1.
    starts: Vec<u32>,
}

impl LineIndex {
    /// Строит указатель по исходному тексту за один проход.
    #[must_use]
    pub fn new(src: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            src.bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i as u32 + 1),
        );
        Self { starts }
    }

    /// Номер строки, считая с единицы, для байтового смещения.
    ///
    /// Смещение за концом текста относится к последней строке: так
    /// отвечает и позиция «конец файла» в диагностиках.
    #[must_use]
    pub fn line_of(&self, offset: u32) -> u32 {
        match self.starts.binary_search(&offset) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }

    /// Число строк в тексте.
    #[must_use]
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Пустым не бывает: строка 1 есть даже у пустого текста.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    #[test]
    fn first_byte_is_line_one() {
        let ix = LineIndex::new("а = 1;\nб = 2;\n");
        assert_eq!(ix.line_of(0), 1);
    }

    #[test]
    fn offset_after_newline_starts_the_next_line() {
        let src = "а = 1;\nб = 2;\n";
        let second = src.find('б').unwrap() as u32;
        assert_eq!(LineIndex::new(src).line_of(second), 2);
    }

    #[test]
    fn newline_itself_belongs_to_the_line_it_ends() {
        let src = "а = 1;\nб = 2;\n";
        let nl = src.find('\n').unwrap() as u32;
        assert_eq!(LineIndex::new(src).line_of(nl), 1);
    }

    #[test]
    fn empty_source_still_has_line_one() {
        assert_eq!(LineIndex::new("").line_of(0), 1);
    }

    #[test]
    fn offset_past_the_end_is_the_last_line() {
        let src = "а = 1;\nб = 2;";
        let ix = LineIndex::new(src);
        assert_eq!(ix.line_of(src.len() as u32 + 100), 2);
    }

    #[test]
    fn multibyte_characters_do_not_shift_the_line() {
        // Смещения БАЙТОВЫЕ, а кириллица занимает по два байта: если бы
        // указатель считал символы, строка уехала бы уже на первой.
        let src = "переменная = \"значение\";\nвторая = 2;\n";
        let second = src.find('в').unwrap() as u32;
        assert_eq!(LineIndex::new(src).line_of(second), 2);
    }

    #[test]
    fn line_count_matches_the_newlines() {
        assert_eq!(LineIndex::new("а\nб\nв").len(), 3);
        assert_eq!(LineIndex::new("а\nб\nв\n").len(), 4);
    }
}
