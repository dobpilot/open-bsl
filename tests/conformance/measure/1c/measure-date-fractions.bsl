// PLATFORM-ONLY: проба фиксирует полное перечисление частей даты.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "date-fractions-1");
М("DATE.FRACTIONS.TIME", ЧастиДаты.Время);
М("DATE.FRACTIONS.DATE", ЧастиДаты.Дата);
М("DATE.FRACTIONS.DATETIME", ЧастиДаты.ДатаВремя);
М("DATE.FRACTIONS.TYPE", ТипЗнч(ЧастиДаты.Время));

АнглийскиеИмена = "ERROR";
Попытка
	Если DateFractions.Time = ЧастиДаты.Время
		И DateFractions.Date = ЧастиДаты.Дата
		И DateFractions.DateTime = ЧастиДаты.ДатаВремя Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("DATE.FRACTIONS.ENGLISH", АнглийскиеИмена);
М("file.end", "date-fractions-1");
