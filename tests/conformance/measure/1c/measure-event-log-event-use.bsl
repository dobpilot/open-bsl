// PLATFORM-ONLY: проба фиксирует значение настройки события журнала регистрации.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "event-log-event-use-1");

ПоУмолчанию = Новый ИспользованиеСобытияЖурналаРегистрации;
М("EVENT.LOG.USE.TYPE", ТипЗнч(ПоУмолчанию));
М("EVENT.LOG.USE.DEFAULT.ENABLED", ПоУмолчанию.Использование);
М("EVENT.LOG.USE.DEFAULT.DESCRIPTION.UNDEFINED", ПоУмолчанию.ОписаниеИспользования = Неопределено);

Явное = Новый ИспользованиеСобытияЖурналаРегистрации(Ложь, 42);
М("EVENT.LOG.USE.EXPLICIT.ENABLED", Явное.Использование);
М("EVENT.LOG.USE.EXPLICIT.DESCRIPTION", Явное.ОписаниеИспользования);

Явное.Использование = Истина;
Явное.ОписаниеИспользования = "описание";
М("EVENT.LOG.USE.MUTATED.ENABLED", Явное.Использование);
М("EVENT.LOG.USE.MUTATED.DESCRIPTION", Явное.ОписаниеИспользования);

АнглийскиеИмена = "ERROR";
Попытка
	English = New EventLogEventUse(False, "value");
	English.Use = True;
	English.UseDescription = "changed";
	Если English.Use И English.UseDescription = "changed" Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("EVENT.LOG.USE.ENGLISH", АнглийскиеИмена);

НеверныйКонструктор = "ACCEPTED";
Попытка
	Неверное = Новый ИспользованиеСобытияЖурналаРегистрации("Истина");
Исключение
	НеверныйКонструктор = "ERROR";
КонецПопытки;
М("EVENT.LOG.USE.BAD.CONSTRUCTOR", НеверныйКонструктор);

НеверноеПрисваивание = "ACCEPTED";
Попытка
	Явное.Использование = 1;
Исключение
	НеверноеПрисваивание = "ERROR";
КонецПопытки;
М("EVENT.LOG.USE.BAD.ASSIGNMENT", НеверноеПрисваивание);
М("EVENT.LOG.USE.BAD.ASSIGNMENT.VALUE", Явное.Использование);
М("EVENT.LOG.USE.BAD.ASSIGNMENT.TYPE", ТипЗнч(Явное.Использование));

М("file.end", "event-log-event-use-1");
