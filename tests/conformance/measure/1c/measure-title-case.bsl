// Контракт `ТРег`: где начинается слово и что происходит с остальными
// буквами слова. Результат печатается в квадратных скобках, чтобы были
// видны пробелы по краям.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Значение);
КонецПроцедуры

Функция Т(Значение)
	Попытка
		Возврат "[" + ТРег(Значение) + "]";
	Исключение
		Возврат "error";
	КонецПопытки;
КонецФункции

М("TITLE.LOWER", Т("привет мир"));
М("TITLE.UPPER", Т("ПРИВЕТ МИР"));
М("TITLE.MIXED", Т("пРиВеТ мИр"));
М("TITLE.LATIN", Т("hello WORLD"));
М("TITLE.YO", Т("ёлка ЁЖ"));
М("TITLE.HYPHEN", Т("иван-петров"));
М("TITLE.APOSTROPHE", Т("o'neil"));
М("TITLE.PUNCT", Т("а.б,в;г:д!е?ж"));
М("TITLE.BRACKETS", Т("(а)[б]{в}""г"""));
М("TITLE.UNDERSCORE", Т("а_б"));
М("TITLE.DIGIT_INSIDE", Т("слово1слово"));
М("TITLE.DIGIT_FIRST", Т("1слово 2СЛОВО"));
М("TITLE.EDGE_SPACES", Т("  два  пробела  "));
М("TITLE.TAB", Т("а" + Символ(9) + "б"));
М("TITLE.LF", Т("а" + Символ(10) + "б"));
М("TITLE.NBSP", Т("а" + Символ(160) + "б"));
М("TITLE.SLASH", Т("а/б\в"));
М("TITLE.ACCENT", Т("émile ÉCOLE"));
М("TITLE.GREEK", Т("αβγ ΔΕΖ"));
М("TITLE.SEP.C00", Т("а" + "#" + "б"));
М("TITLE.SEP.C01", Т("а" + "@" + "б"));
М("TITLE.SEP.C02", Т("а" + "+" + "б"));
М("TITLE.SEP.C03", Т("а" + "=" + "б"));
М("TITLE.SEP.C04", Т("а" + "*" + "б"));
М("TITLE.SEP.C05", Т("а" + "&" + "б"));
М("TITLE.SEP.C06", Т("а" + "%" + "б"));
М("TITLE.SEP.C07", Т("а" + "$" + "б"));
М("TITLE.SEP.C08", Т("а" + "<" + "б"));
М("TITLE.SEP.C09", Т("а" + ">" + "б"));
М("TITLE.SEP.C10", Т("а" + "|" + "б"));
М("TITLE.SEP.C11", Т("а" + "~" + "б"));
М("TITLE.SEP.C12", Т("а" + "`" + "б"));
М("TITLE.SEP.C13", Т("а" + "^" + "б"));
М("TITLE.SEP.C14", Т("а" + "№" + "б"));
М("TITLE.SEP.C15", Т("а" + "«" + "б"));
М("TITLE.SEP.C16", Т("а" + "»" + "б"));
М("TITLE.SEP.C17", Т("а" + "—" + "б"));
М("TITLE.SEP.C18", Т("а" + "–" + "б"));
М("TITLE.SEP.C19", Т("а" + "…" + "б"));
М("TITLE.SEP.C20", Т("а" + "§" + "б"));
М("TITLE.SEP.C21", Т("а" + "·" + "б"));
М("TITLE.SEP.CODE8203", Т("а" + Символ(8203) + "б"));
М("TITLE.SEP.CODE173", Т("а" + Символ(173) + "б"));
М("TITLE.SEP.CODE13", Т("а" + Символ(13) + "б"));
М("TITLE.SEP.CODE12", Т("а" + Символ(12) + "б"));
М("TITLE.SEP.CODE11", Т("а" + Символ(11) + "б"));
М("TITLE.SEP.CODE8195", Т("а" + Символ(8195) + "б"));
М("TITLE.SEP.CODE12288", Т("а" + Символ(12288) + "б"));
М("TITLE.SEP.CODE8232", Т("а" + Символ(8232) + "б"));
М("TITLE.EDGE.DOT_DOUBLE", Т("а..б"));
М("TITLE.EDGE.DOT_LEAD", Т(".б"));
М("TITLE.EDGE.DOT_TRAIL", Т("а. б"));
М("TITLE.EDGE.DOT_DIGIT_LETTER", Т("1.б"));
М("TITLE.EDGE.LETTER_DOT_DIGIT", Т("а.1б"));
М("TITLE.EDGE.UNDERSCORE_LEAD", Т("_б"));
М("TITLE.EDGE.UNDERSCORE_DOUBLE", Т("а__б"));
М("TITLE.EDGE.APOS_LEAD", Т("'б"));
М("TITLE.EDGE.APOS_DOUBLE", Т("а''б"));
М("TITLE.EDGE.COLON_DIGITS", Т("1:б"));
М("TITLE.EDGE.DOT_UNDERSCORE", Т("а._б"));
М("TITLE.EDGE.DIGIT_DOT_DIGIT", Т("1.2б"));
М("TITLE.EDGE.LATIN_COLON", Т("a:b"));
М("TITLE.EDGE.MIXED_SCRIPT", Т("аb"));
М("TITLE.EDGE.RIGHT_QUOTE", Т("а’б"));
М("TITLE.EDGE.HEBREW", Т("שלום עולם"));
М("TITLE.EDGE.CJK", Т("中文 abc"));
М("TITLE.EDGE.ARABIC_DIGITS", Т("٣б"));
М("TITLE.EDGE.COMBINING", Т("е́ж"));
М("TITLE.EDGE.KATAKANA", Т("カタカナ abc"));
М("TITLE.SURROGATE_LITERAL_LEN", Строка(СтрДлина(ТРег("а😀б"))));
М("TITLE.SURROGATE_LITERAL_INPUT_LEN", Строка(СтрДлина("а😀б")));
М("TITLE.EDGE.FINAL_SIGMA", Т("ΟΔΟΣ ΟΔΟΣΑ"));
М("TITLE.EDGE.LIGATURE_FI", Т("ﬁx ﬀ"));
М("TITLE.EDGE.DIGRAPH_LJ", Т("ǈubljana ǇUBLJANA"));
М("TITLE.EDGE.GREEK_IOTA", Т("ᾳβ"));
М("TITLE.EDGE.IDEO_COLON", Т("中:b"));
М("TITLE.EDGE.KANA_DOT", Т("カ.b"));
М("TITLE.EDGE.IDEO_LETTER", Т("中b"));
М("TITLE.EDGE.DOTTED_I", Т("İSTANBUL"));
М("TITLE.EDGE.TURKIC_I", Т("ıi"));
М("TITLE.EDGE.ARMENIAN_LIG", Т("և"));
М("TITLE.EDGE.N_APOSTROPHE", Т("ŉa"));
М("TITLE.SINGLE", Т("я"));
М("TITLE.THOUSANDS", Т(1000));
М("TITLE.DATE", Т('20240102'));
М("TITLE.LIGATURE", Т("ǆemal ß"));
М("TITLE.EMPTY", Т(""));
М("TITLE.NUMBER", Т(12));
М("TITLE.UNDEFINED", Т(Неопределено));
Результат = "error";
Попытка
	Результат = "[" + Title("abc def") + "]";
Исключение
КонецПопытки;
М("TITLE.ENGLISH_ALIAS", Результат);
