// PLATFORM-ONLY: проба фиксирует закрытое перечисление способа кодирования не-ASCII символов почтового сообщения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "mail-non-ascii-encoding-mode-1");
М("MAIL.ENCODING.MIME", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.MIME);
М("MAIL.ENCODING.QUOTED", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.QuotedPrintable);
М("MAIL.ENCODING.NONE", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.БезКодирования);
М("MAIL.ENCODING.TYPE", ТипЗнч(СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.MIME));
М("MAIL.ENCODING.ALIAS.MIME", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.MIME = InternetMailMessageNonASCIISymbolsEncodingMode.MIME);
М("MAIL.ENCODING.ALIAS.QUOTED", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.QuotedPrintable = InternetMailMessageNonASCIISymbolsEncodingMode.QuotedPrintable);
М("MAIL.ENCODING.ALIAS.NONE", СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.БезКодирования = InternetMailMessageNonASCIISymbolsEncodingMode.None);
М("file.end", "mail-non-ascii-encoding-mode-1");
