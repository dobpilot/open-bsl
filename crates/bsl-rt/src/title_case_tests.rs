//! Случаи сняты с платформы пробой `measure-title-case.bsl`.

use super::title_case;

#[test]
fn words_follow_unicode_boundaries() {
    for (input, expected) in [
        ("пРиВеТ мИр", "Привет Мир"),
        ("иван-петров", "Иван-Петров"),
        ("o'neil", "O'neil"),
        ("а.б,в;г:д!е?ж", "А.б,В;Г:д!Е?Ж"),
        ("а..б", "А..Б"),
        ("а._б", "А._Б"),
        ("а__б", "А__б"),
        ("слово1слово", "Слово1слово"),
        ("1слово 2СЛОВО", "1Слово 2Слово"),
        ("1.2б", "1.2Б"),
        ("а.1б", "А.1Б"),
        ("1:б", "1:Б"),
        ("а\u{00AD}б", "А\u{00AD}б"),
        ("е\u{0301}ж", "Е\u{0301}ж"),
        ("中b", "中B"),
        ("  два  пробела  ", "  Два  Пробела  "),
    ] {
        assert_eq!(title_case(input), expected, "{input}");
    }
}

#[test]
fn title_forms_differ_from_upper_case_where_unicode_says_so() {
    for (input, expected) in [
        ("ǆemal ß", "ǅemal Ss"),
        ("ǇUBLJANA", "ǈubljana"),
        ("ﬁx ﬀ", "Fix Ff"),
        ("ᾳβ", "ᾼβ"),
        ("և", "Եւ"),
        ("ŉa", "ʼNa"),
        ("ΟΔΟΣ ΟΔΟΣΑ", "Οδος Οδοσα"),
        ("İSTANBUL", "İstanbul"),
        ("ıi", "Ii"),
    ] {
        assert_eq!(title_case(input), expected, "{input}");
    }
}
