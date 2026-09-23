// PLATFORM-ONLY: проба фиксирует достигнутые вызовы монопольного режима.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "exclusive-mode-1");
М("EXCLUSIVE.DEFAULT", МонопольныйРежим());
М("EXCLUSIVE.DEFAULT.TYPE", ТипЗнч(МонопольныйРежим()));
М("EXCLUSIVE.DEFAULT.ENGLISH", ExclusiveMode());
Попытка
	УстановитьМонопольныйРежим(Истина);
	М("EXCLUSIVE.SET.TRUE", МонопольныйРежим());
	SetExclusiveMode(False);
	М("EXCLUSIVE.SET.FALSE", МонопольныйРежим());
Исключение
	М("EXCLUSIVE.SET.TRUE", "ERROR");
	Попытка
		УстановитьМонопольныйРежим(Ложь);
	Исключение
	КонецПопытки;
	М("EXCLUSIVE.SET.FALSE", "ERROR");
КонецПопытки;
М("file.end", "exclusive-mode-1");
