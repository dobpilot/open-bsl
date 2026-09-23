// PLATFORM-ONLY: проба фиксирует язык текущего сеанса.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "current-language-1");
ЯзыкСеанса = ТекущийЯзык();
М("CURRENT.LANGUAGE.TYPE", ТипЗнч(ЯзыкСеанса));
М("CURRENT.LANGUAGE.STRING", ЯзыкСеанса);
М("CURRENT.LANGUAGE.ENGLISH.EQUAL", CurrentLanguage() = ЯзыкСеанса);

ИмяЯзыка = "ERROR";
Попытка
	ИмяЯзыка = ЯзыкСеанса.Имя;
Исключение
КонецПопытки;
М("CURRENT.LANGUAGE.NAME", ИмяЯзыка);

ПервыйЯзык = "ERROR";
Попытка
	ПервыйЯзык = Метаданные.Языки[0];
Исключение
КонецПопытки;
М("CURRENT.LANGUAGE.FIRST.TYPE", ТипЗнч(ПервыйЯзык));
М("CURRENT.LANGUAGE.FIRST.EQUAL", ПервыйЯзык = ЯзыкСеанса);
М("file.end", "current-language-1");
