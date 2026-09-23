// PLATFORM-ONLY: проба фиксирует закрытое перечисление ОтправкаЭлементаДанных.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-item-send-1");
М("DATA.ITEM.SEND.AUTO", ОтправкаЭлементаДанных.Авто);
М("DATA.ITEM.SEND.DELETE", ОтправкаЭлементаДанных.Удалить);
М("DATA.ITEM.SEND.IGNORE", ОтправкаЭлементаДанных.Игнорировать);
М("DATA.ITEM.SEND.TYPE", ТипЗнч(ОтправкаЭлементаДанных.Игнорировать));
М("DATA.ITEM.SEND.ALIAS.AUTO", ОтправкаЭлементаДанных.Авто = DataItemSend.Auto);
М("DATA.ITEM.SEND.ALIAS.DELETE", ОтправкаЭлементаДанных.Удалить = DataItemSend.Delete);
М("DATA.ITEM.SEND.ALIAS.IGNORE", ОтправкаЭлементаДанных.Игнорировать = DataItemSend.Ignore);
М("file.end", "data-item-send-1");
