// PLATFORM-ONLY: проба фиксирует закрытое перечисление важности почтового сообщения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "mail-message-importance-1");
М("MAIL.IMPORTANCE.HIGH", ВажностьИнтернетПочтовогоСообщения.Высокая);
М("MAIL.IMPORTANCE.HIGHEST", ВажностьИнтернетПочтовогоСообщения.Наивысшая);
М("MAIL.IMPORTANCE.LOWEST", ВажностьИнтернетПочтовогоСообщения.Наименьшая);
М("MAIL.IMPORTANCE.LOW", ВажностьИнтернетПочтовогоСообщения.Низкая);
М("MAIL.IMPORTANCE.NORMAL", ВажностьИнтернетПочтовогоСообщения.Обычная);
М("MAIL.IMPORTANCE.TYPE", ТипЗнч(ВажностьИнтернетПочтовогоСообщения.Обычная));
М("MAIL.IMPORTANCE.ALIAS.HIGH", ВажностьИнтернетПочтовогоСообщения.Высокая = InternetMailMessageImportance.High);
М("MAIL.IMPORTANCE.ALIAS.HIGHEST", ВажностьИнтернетПочтовогоСообщения.Наивысшая = InternetMailMessageImportance.Highest);
М("MAIL.IMPORTANCE.ALIAS.LOWEST", ВажностьИнтернетПочтовогоСообщения.Наименьшая = InternetMailMessageImportance.Lowest);
М("MAIL.IMPORTANCE.ALIAS.LOW", ВажностьИнтернетПочтовогоСообщения.Низкая = InternetMailMessageImportance.Low);
М("MAIL.IMPORTANCE.ALIAS.NORMAL", ВажностьИнтернетПочтовогоСообщения.Обычная = InternetMailMessageImportance.Normal);
М("file.end", "mail-message-importance-1");
