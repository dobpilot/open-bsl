// PLATFORM-ONLY: проба фиксирует конструктор компоновщика макета СКД.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-composition-template-composer-1");
М("DCS.TEMPLATE.COMPOSER.TYPE", ТипЗнч(Новый КомпоновщикМакетаКомпоновкиДанных));
М("DCS.PROCESSOR.TYPE", ТипЗнч(Новый ПроцессорКомпоновкиДанных));
ПроцессорВывода = Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений;
М("DCS.VALUE.OUTPUT.TYPE", ТипЗнч(ПроцессорВывода));
М("DCS.VALUE.OUTPUT.PERCENT.DEFAULT", ПроцессорВывода.ОтображатьПроцентВывода);
ПроцессорВывода.ShowOutputPercent = Истина;
М("DCS.VALUE.OUTPUT.PERCENT.SET", ПроцессорВывода.ОтображатьПроцентВывода);

АнглийскоеИмя = "ERROR";
Попытка
	АнглийскоеИмя = Строка(ТипЗнч(New DataCompositionTemplateComposer));
Исключение
КонецПопытки;
М("DCS.TEMPLATE.COMPOSER.ENGLISH", АнглийскоеИмя);

АнглийскоеИмяПроцессора = "ERROR";
Попытка
	АнглийскоеИмяПроцессора = Строка(ТипЗнч(New DataCompositionProcessor));
Исключение
КонецПопытки;
М("DCS.PROCESSOR.ENGLISH", АнглийскоеИмяПроцессора);

АнглийскоеИмяВывода = "ERROR";
Попытка
	АнглийскоеИмяВывода = Строка(ТипЗнч(New DataCompositionResultValueCollectionOutputProcessor));
Исключение
КонецПопытки;
М("DCS.VALUE.OUTPUT.ENGLISH", АнглийскоеИмяВывода);
М("file.end", "data-composition-template-composer-1");
