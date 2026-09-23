// PLATFORM-ONLY: проба фиксирует полное перечисление режимов проведения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "document-posting-mode-1");
М("DOCUMENT.POSTING.MODE.OPERATIONAL", РежимПроведенияДокумента.Оперативный);
М("DOCUMENT.POSTING.MODE.NONOPERATIONAL", РежимПроведенияДокумента.Неоперативный);
М("DOCUMENT.POSTING.MODE.TYPE", ТипЗнч(РежимПроведенияДокумента.Оперативный));

АнглийскиеИмена = "ERROR";
Попытка
	Если DocumentPostingMode.RealTime = РежимПроведенияДокумента.Оперативный
		И DocumentPostingMode.Regular = РежимПроведенияДокумента.Неоперативный Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("DOCUMENT.POSTING.MODE.ENGLISH", АнглийскиеИмена);
М("file.end", "document-posting-mode-1");
