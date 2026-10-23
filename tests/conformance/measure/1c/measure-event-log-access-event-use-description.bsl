// PLATFORM-ONLY: проба фиксирует достигнутый срез описания события доступа.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "event-log-access-event-use-description-1");

Описание = Новый ОписаниеИспользованияСобытияДоступЖурналаРегистрации;
М("EVENT.ACCESS.DESCRIPTION.TYPE", ТипЗнч(Описание));
М("EVENT.ACCESS.DESCRIPTION.DEFAULT.OBJECT", Описание.Объект);
М("EVENT.ACCESS.DESCRIPTION.DEFAULT.ACCESS.TYPE", ТипЗнч(Описание.ПоляДоступа));
М("EVENT.ACCESS.DESCRIPTION.DEFAULT.ACCESS.COUNT", Описание.ПоляДоступа.Количество());
М("EVENT.ACCESS.DESCRIPTION.DEFAULT.LOGGED.TYPE", ТипЗнч(Описание.ПоляРегистрации));
М("EVENT.ACCESS.DESCRIPTION.DEFAULT.LOGGED.COUNT", Описание.ПоляРегистрации.Количество());

ПоляДоступа = Новый Массив;
ПоляДоступа.Добавить("Поле1");
ПоляРегистрации = Новый Массив;
ПоляРегистрации.Добавить("Поле2");
Описание.Объект = "Справочник.Номенклатура";
Описание.ПоляДоступа = ПоляДоступа;
Описание.ПоляРегистрации = ПоляРегистрации;
М("EVENT.ACCESS.DESCRIPTION.MUTATED", Описание.Объект + "|" + Описание.ПоляДоступа[0] + "|" + Описание.ПоляРегистрации[0]);

ПоИмени = Новый ОписаниеИспользованияСобытияДоступЖурналаРегистрации("Документ.Заказ");
М("EVENT.ACCESS.DESCRIPTION.NAMED", ПоИмени.Объект);

АнглийскиеИмена = "ERROR";
Попытка
	English = New EventLogAccessEventUseDescription("Catalog.Products");
	English.AccessFields = New Array;
	English.LoggedFields = New Array;
	Если English.Object = "Catalog.Products" Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("EVENT.ACCESS.DESCRIPTION.ENGLISH", АнглийскиеИмена);

НеверныйОбъект = "ACCEPTED";
Попытка
	Описание.Объект = 1;
Исключение
	НеверныйОбъект = "ERROR";
КонецПопытки;
М("EVENT.ACCESS.DESCRIPTION.BAD.OBJECT", НеверныйОбъект);

НеверныеПоля = "ACCEPTED";
Попытка
	Описание.ПоляДоступа = "Поле";
Исключение
	НеверныеПоля = "ERROR";
КонецПопытки;
М("EVENT.ACCESS.DESCRIPTION.BAD.FIELDS", НеверныеПоля);

М("file.end", "event-log-access-event-use-description-1");
