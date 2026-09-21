// PLATFORM-ONLY: проба требует настоящего соединения с информационной базой.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "infobase-connections-1");

Соединения = ПолучитьСоединенияИнформационнойБазы();
СоединенияАнгл = GetInfoBaseConnections();
ТекущийНомер = НомерСоединенияИнформационнойБазы();
Текущее = Неопределено;
Для Каждого Соединение Из Соединения Цикл
	Если Соединение.НомерСоединения = ТекущийНомер Тогда
		Текущее = Соединение;
		Прервать;
	КонецЕсли;
КонецЦикла;

М("SESSION.CONNECTIONS.TYPE", ТипЗнч(Соединения));
М("SESSION.CONNECTIONS.NON_EMPTY", Соединения.Количество() > 0);
М("SESSION.CONNECTIONS.ENGLISH_COUNT", СоединенияАнгл.Количество() = Соединения.Количество());
М("SESSION.CONNECTIONS.CURRENT_FOUND", Текущее <> Неопределено);

Если Текущее <> Неопределено Тогда
	М("SESSION.CONNECTION.TYPE", ТипЗнч(Текущее));
	М("SESSION.CONNECTION.NUMBER_TYPE", ТипЗнч(Текущее.НомерСоединения));
	М("SESSION.CONNECTION.NUMBER_MATCH", Текущее.НомерСоединения = ТекущийНомер);
	М("SESSION.CONNECTION.APPLICATION_NAME_TYPE", ТипЗнч(Текущее.ИмяПриложения));
	М("SESSION.CONNECTION.COMPUTER_NAME_TYPE", ТипЗнч(Текущее.ИмяКомпьютера));
Иначе
	М("SESSION.CONNECTION.TYPE", "NOT_FOUND");
	М("SESSION.CONNECTION.NUMBER_TYPE", "NOT_FOUND");
	М("SESSION.CONNECTION.NUMBER_MATCH", Ложь);
	М("SESSION.CONNECTION.APPLICATION_NAME_TYPE", "NOT_FOUND");
	М("SESSION.CONNECTION.COMPUTER_NAME_TYPE", "NOT_FOUND");
КонецЕсли;

М("file.end", "infobase-connections-1");
