// PLATFORM-ONLY: проба фиксирует закрытое перечисление ПолучениеЭлементаДанных.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-item-receive-1");
М("DATA.ITEM.RECEIVE.AUTO", ПолучениеЭлементаДанных.Авто);
М("DATA.ITEM.RECEIVE.ACCEPT", ПолучениеЭлементаДанных.Принять);
М("DATA.ITEM.RECEIVE.IGNORE", ПолучениеЭлементаДанных.Игнорировать);
М("DATA.ITEM.RECEIVE.TYPE", ТипЗнч(ПолучениеЭлементаДанных.Игнорировать));
М("DATA.ITEM.RECEIVE.ALIAS.AUTO", ПолучениеЭлементаДанных.Авто = DataItemReceive.Auto);
М("DATA.ITEM.RECEIVE.ALIAS.ACCEPT", ПолучениеЭлементаДанных.Принять = DataItemReceive.Accept);
М("DATA.ITEM.RECEIVE.ALIAS.IGNORE", ПолучениеЭлементаДанных.Игнорировать = DataItemReceive.Ignore);
М("file.end", "data-item-receive-1");
