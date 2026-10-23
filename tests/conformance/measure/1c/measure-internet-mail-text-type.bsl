// PLATFORM-ONLY: проба фиксирует закрытое перечисление ТипТекстаПочтовогоСообщения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "internet-mail-text-type-1");
М("MAIL.TEXT.HTML", ТипТекстаПочтовогоСообщения.HTML);
М("MAIL.TEXT.CUSTOM", ТипТекстаПочтовогоСообщения.ПроизвольныйТекст);
М("MAIL.TEXT.PLAIN", ТипТекстаПочтовогоСообщения.ПростойТекст);
М("MAIL.TEXT.RICH", ТипТекстаПочтовогоСообщения.РазмеченныйТекст);
М("MAIL.TEXT.TYPE", ТипЗнч(ТипТекстаПочтовогоСообщения.HTML));
М("MAIL.TEXT.ALIAS.HTML", ТипТекстаПочтовогоСообщения.HTML = InternetMailTextType.HTML);
М("MAIL.TEXT.ALIAS.CUSTOM", ТипТекстаПочтовогоСообщения.ПроизвольныйТекст = InternetMailTextType.CustomText);
М("MAIL.TEXT.ALIAS.PLAIN", ТипТекстаПочтовогоСообщения.ПростойТекст = InternetMailTextType.PlainText);
М("MAIL.TEXT.ALIAS.RICH", ТипТекстаПочтовогоСообщения.РазмеченныйТекст = InternetMailTextType.RichText);
М("file.end", "internet-mail-text-type-1");
