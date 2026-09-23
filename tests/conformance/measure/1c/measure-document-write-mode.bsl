// PLATFORM-ONLY: проба фиксирует полное перечисление режимов записи документа.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "document-write-mode-1");
М("DOCUMENT.WRITE.MODE.WRITE", РежимЗаписиДокумента.Запись);
М("DOCUMENT.WRITE.MODE.POSTING", РежимЗаписиДокумента.Проведение);
М("DOCUMENT.WRITE.MODE.UNDO.POSTING", РежимЗаписиДокумента.ОтменаПроведения);
М("DOCUMENT.WRITE.MODE.TYPE", ТипЗнч(РежимЗаписиДокумента.Запись));

АнглийскиеИмена = "ERROR";
Попытка
	Если DocumentWriteMode.Write = РежимЗаписиДокумента.Запись
		И DocumentWriteMode.Posting = РежимЗаписиДокумента.Проведение
		И DocumentWriteMode.UndoPosting = РежимЗаписиДокумента.ОтменаПроведения Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("DOCUMENT.WRITE.MODE.ENGLISH", АнглийскиеИмена);
М("file.end", "document-write-mode-1");
