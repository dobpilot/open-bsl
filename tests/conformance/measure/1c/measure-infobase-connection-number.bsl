// PLATFORM-ONLY: проба требует настоящего соединения с информационной базой.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "infobase-connection-number-1");

НомерПервый = НомерСоединенияИнформационнойБазы();
НомерВторой = InfoBaseConnectionNumber();
М("SESSION.CONNECTION_NUMBER.TYPE", ТипЗнч(НомерПервый));
М("SESSION.CONNECTION_NUMBER.POSITIVE", НомерПервый > 0);
М("SESSION.CONNECTION_NUMBER.STABLE", НомерПервый = НомерВторой);

М("file.end", "infobase-connection-number-1");
