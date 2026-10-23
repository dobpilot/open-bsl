// PLATFORM-ONLY: зона компьютера и изменяемая зона сеанса — разные значения.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "computer-time-zone-1");
М("TIME.COMPUTER.VALUE", ЧасовойПояс());
М("TIME.COMPUTER.ENGLISH", TimeZone());
М("TIME.COMPUTER.TYPE", ТипЗнч(ЧасовойПояс()));
М("TIME.COMPUTER.SAME", ЧасовойПояс() = TimeZone());
СтараяЗона = ЧасовойПоясСеанса();
Попытка
	УстановитьЧасовойПоясСеанса("GMT+0");
	М("TIME.COMPUTER.AFTER_SESSION_CHANGE", ЧасовойПояс());
	М("TIME.COMPUTER.SESSION_CHANGED", ЧасовойПоясСеанса());
Исключение
	М("TIME.COMPUTER.SESSION_CHANGED", "ERROR");
КонецПопытки;
Попытка
	УстановитьЧасовойПоясСеанса(СтараяЗона);
Исключение
КонецПопытки;
М("file.end", "computer-time-zone-1");
