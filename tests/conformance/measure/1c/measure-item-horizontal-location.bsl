// PLATFORM-ONLY: проба фиксирует закрытое перечисление ГоризонтальноеПоложениеЭлемента.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "item-horizontal-location-1");
М("ITEM.HORIZONTAL.AUTO", ГоризонтальноеПоложениеЭлемента.Авто);
М("ITEM.HORIZONTAL.LEFT", ГоризонтальноеПоложениеЭлемента.Лево);
М("ITEM.HORIZONTAL.RIGHT", ГоризонтальноеПоложениеЭлемента.Право);
М("ITEM.HORIZONTAL.CENTER", ГоризонтальноеПоложениеЭлемента.Центр);
М("ITEM.HORIZONTAL.TYPE", ТипЗнч(ГоризонтальноеПоложениеЭлемента.Авто));
М("ITEM.HORIZONTAL.ALIAS.AUTO", ГоризонтальноеПоложениеЭлемента.Авто = ItemHorizontalLocation.Auto);
М("ITEM.HORIZONTAL.ALIAS.LEFT", ГоризонтальноеПоложениеЭлемента.Лево = ItemHorizontalLocation.Left);
М("ITEM.HORIZONTAL.ALIAS.RIGHT", ГоризонтальноеПоложениеЭлемента.Право = ItemHorizontalLocation.Right);
М("ITEM.HORIZONTAL.ALIAS.CENTER", ГоризонтальноеПоложениеЭлемента.Центр = ItemHorizontalLocation.Center);
М("file.end", "item-horizontal-location-1");
