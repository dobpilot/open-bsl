// PLATFORM-ONLY: проба фиксирует идентификатор часового пояса сеанса.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "session-time-zone-1");
М("TIME.SESSION.VALUE", ЧасовойПоясСеанса());
М("TIME.SESSION.ENGLISH", SessionTimeZone());
М("TIME.SESSION.TYPE", ТипЗнч(ЧасовойПоясСеанса()));
М("TIME.SESSION.SAME", ЧасовойПоясСеанса() = SessionTimeZone());
М("file.end", "session-time-zone-1");
