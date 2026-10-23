// PLATFORM-ONLY: проба фиксирует достигнутую функцию летнего смещения времени.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

Зима = Дата(2024, 1, 15, 12, 0, 0);
Лето = Дата(2024, 7, 15, 12, 0, 0);

М("file.begin", "daylight-time-offset-1");
М("TIME.DAYLIGHT.GMT.PLUS", СмещениеЛетнегоВремени("GMT+3", Зима));
М("TIME.DAYLIGHT.GMT.MINUS", СмещениеЛетнегоВремени("GMT-05:30", Зима));
М("TIME.DAYLIGHT.GMT.ZERO", СмещениеЛетнегоВремени("GMT+0", Зима));
М("TIME.DAYLIGHT.NEWYORK.WINTER", СмещениеЛетнегоВремени("America/New_York", Зима));
М("TIME.DAYLIGHT.NEWYORK.SUMMER", СмещениеЛетнегоВремени("America/New_York", Лето));
М("TIME.DAYLIGHT.BERLIN.WINTER", СмещениеЛетнегоВремени("Europe/Berlin", Зима));
М("TIME.DAYLIGHT.BERLIN.SUMMER", СмещениеЛетнегоВремени("Europe/Berlin", Лето));
М("TIME.DAYLIGHT.UNDEFINED.ZONE", СмещениеЛетнегоВремени(Неопределено, Зима));
М("TIME.DAYLIGHT.OMITTED.DATE", СмещениеЛетнегоВремени("GMT+3"));
М("TIME.DAYLIGHT.UNDEFINED.DATE", СмещениеЛетнегоВремени("GMT+3", Неопределено));
М("TIME.DAYLIGHT.OMITTED.ALL", СмещениеЛетнегоВремени());
М("TIME.DAYLIGHT.ENGLISH", DaylightTimeOffset("GMT+3", Зима));
М("TIME.DAYLIGHT.TYPE", ТипЗнч(СмещениеЛетнегоВремени("GMT+3", Зима)));

НеизвестнаяЗона = "ERROR";
Попытка
	НеизвестнаяЗона = СмещениеЛетнегоВремени("Invalid/Zone", Зима);
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.UNKNOWN.ZONE", НеизвестнаяЗона);

НеверныйТипЗоны = "ERROR";
Попытка
	НеверныйТипЗоны = СмещениеЛетнегоВремени(1, Зима);
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.BAD.ZONE.TYPE", НеверныйТипЗоны);

НеверныйТипДаты = "ERROR";
Попытка
	НеверныйТипДаты = СмещениеЛетнегоВремени("GMT+3", "20240115");
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.BAD.DATE.TYPE", НеверныйТипДаты);

СтроковаяДата = "ERROR";
Попытка
	СтроковаяДата = СмещениеЛетнегоВремени("America/New_York", "20240715120000");
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.STRING.DATE", СтроковаяДата);

НевернаяСтроковаяДата = "ERROR";
Попытка
	НевернаяСтроковаяДата = СмещениеЛетнегоВремени("America/New_York", "not-a-date");
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.BAD.STRING.DATE", НевернаяСтроковаяДата);

ЧисловаяДата = "ERROR";
Попытка
	ЧисловаяДата = СмещениеЛетнегоВремени("America/New_York", 1);
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.NUMBER.DATE", ЧисловаяДата);

МассивДата = "ERROR";
Попытка
	МассивДата = СмещениеЛетнегоВремени("America/New_York", Новый Массив);
Исключение
КонецПопытки;
М("TIME.DAYLIGHT.ARRAY.DATE", МассивДата);
М("file.end", "daylight-time-offset-1");
