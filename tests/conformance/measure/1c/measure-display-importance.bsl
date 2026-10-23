// PLATFORM-ONLY: проба фиксирует закрытое перечисление ВажностьПриОтображении.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "display-importance-1");
М("DISPLAY.IMPORTANCE.AUTO", ВажностьПриОтображении.Авто);
М("DISPLAY.IMPORTANCE.HIGH", ВажностьПриОтображении.Высокая);
М("DISPLAY.IMPORTANCE.LOW", ВажностьПриОтображении.Низкая);
М("DISPLAY.IMPORTANCE.USUAL", ВажностьПриОтображении.Обычная);
М("DISPLAY.IMPORTANCE.VERY_HIGH", ВажностьПриОтображении.ОченьВысокая);
М("DISPLAY.IMPORTANCE.VERY_LOW", ВажностьПриОтображении.ОченьНизкая);
М("DISPLAY.IMPORTANCE.TYPE", ТипЗнч(ВажностьПриОтображении.Обычная));
М("DISPLAY.IMPORTANCE.ALIAS.AUTO", ВажностьПриОтображении.Авто = DisplayImportance.Auto);
М("DISPLAY.IMPORTANCE.ALIAS.HIGH", ВажностьПриОтображении.Высокая = DisplayImportance.High);
М("DISPLAY.IMPORTANCE.ALIAS.LOW", ВажностьПриОтображении.Низкая = DisplayImportance.Low);
М("DISPLAY.IMPORTANCE.ALIAS.USUAL", ВажностьПриОтображении.Обычная = DisplayImportance.Usual);
М("DISPLAY.IMPORTANCE.ALIAS.VERY_HIGH", ВажностьПриОтображении.ОченьВысокая = DisplayImportance.VeryHigh);
М("DISPLAY.IMPORTANCE.ALIAS.VERY_LOW", ВажностьПриОтображении.ОченьНизкая = DisplayImportance.VeryLow);
М("file.end", "display-importance-1");
