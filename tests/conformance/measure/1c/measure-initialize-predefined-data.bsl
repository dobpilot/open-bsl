// PLATFORM-ONLY: проба фиксирует достигнутую процедуру и оба её имени.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "initialize-predefined-data-1");
РусскоеИмя = "ERROR";
АнглийскоеИмя = "ERROR";
Попытка
	ИнициализироватьПредопределенныеДанные();
	РусскоеИмя = "OK";
Исключение
КонецПопытки;
Попытка
	InitializePredefinedData();
	АнглийскоеИмя = "OK";
Исключение
КонецПопытки;
М("PREDEFINED.INITIALIZE.RUSSIAN", РусскоеИмя);
М("PREDEFINED.INITIALIZE.ENGLISH", АнглийскоеИмя);
М("file.end", "initialize-predefined-data-1");
