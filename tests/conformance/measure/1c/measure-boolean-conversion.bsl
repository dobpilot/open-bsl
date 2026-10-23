// Преобразование документированных скалярных типов функцией Булево.

Процедура П(Ид, Выражение)
	Попытка
		Результат = Вычислить(Выражение);
		Сообщить(Ид + Символ(9) + "OK" + Символ(9) + ?(Результат, "true", "false"));
	Исключение
		Сообщить(Ид + Символ(9) + "ERROR");
	КонецПопытки;
КонецПроцедуры

П("BOOLEAN.NUMBER.ZERO", "Булево(0)");
П("BOOLEAN.NUMBER.ONE", "Булево(1)");
П("BOOLEAN.NUMBER.NEGATIVE", "Булево(-1)");
П("BOOLEAN.BOOLEAN.FALSE", "Булево(Ложь)");
П("BOOLEAN.BOOLEAN.TRUE", "Булево(Истина)");
П("BOOLEAN.STRING.RU.FALSE", "Булево(""Ложь"")");
П("BOOLEAN.STRING.RU.TRUE", "Булево(""Истина"")");
П("BOOLEAN.STRING.EN.FALSE", "Булево(""False"")");
П("BOOLEAN.STRING.EN.TRUE", "Булево(""True"")");
П("BOOLEAN.STRING.RU.LOWER", "Булево(""истина"")");
П("BOOLEAN.STRING.EN.LOWER", "Булево(""false"")");
П("BOOLEAN.STRING.RU.YES", "Булево(""Да"")");
П("BOOLEAN.STRING.RU.NO", "Булево(""Нет"")");
П("BOOLEAN.STRING.EN.YES", "Булево(""Yes"")");
П("BOOLEAN.STRING.EN.NO", "Булево(""No"")");
П("BOOLEAN.STRING.SPACE", "Булево("" Истина "")");
П("BOOLEAN.STRING.ZERO", "Булево(""0"")");
П("BOOLEAN.STRING.ONE", "Булево(""1"")");
П("BOOLEAN.STRING.EMPTY", "Булево("""")");
П("BOOLEAN.UNDEFINED", "Булево(Неопределено)");
П("BOOLEAN.ZERO_ARGS", "Булево()");
П("BOOLEAN.TWO_ARGS", "Булево(0, 1)");
П("BOOLEAN.ENGLISH", "Boolean(1)");
