// `Формат` пустой даты по набору форматных строк.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + "[" + Значение + "]");
КонецПроцедуры

Функция Ф(Значение, ФорматнаяСтрока)
	Попытка
		Возврат Формат(Значение, ФорматнаяСтрока);
	Исключение
		Возврат "error";
	КонецПопытки;
КонецФункции

Пустая = Дата(1, 1, 1);
М("EMPTYDATE.NO_FORMAT", Ф(Пустая, ""));
М("EMPTYDATE.DF_FULL", Ф(Пустая, "ДФ=yyyyMMddHHmmss"));
М("EMPTYDATE.DF_DATE", Ф(Пустая, "ДФ=dd.MM.yyyy"));
М("EMPTYDATE.DF_TIME", Ф(Пустая, "ДФ=HH:mm:ss"));
М("EMPTYDATE.DLF_D", Ф(Пустая, "ДЛФ=D"));
М("EMPTYDATE.DLF_DT", Ф(Пустая, "ДЛФ=DT"));
М("EMPTYDATE.DLF_T", Ф(Пустая, "ДЛФ=T"));
М("EMPTYDATE.DP", Ф(Пустая, "ДП=пусто"));
М("EMPTYDATE.DF_DP", Ф(Пустая, "ДФ=yyyyMMdd; ДП=пусто"));
М("EMPTYDATE.EN_DF", Ф(Пустая, "DF=yyyyMMdd"));
М("EMPTYDATE.EN_DE", Ф(Пустая, "DE=nothing"));
М("EMPTYDATE.STRING", Строка(Пустая));
М("EMPTYDATE.NONEMPTY_DF", Ф(Дата(1, 1, 1, 0, 0, 1), "ДФ=yyyyMMddHHmmss"));
М("EMPTYDATE.YEAR_ONE_DF", Ф(Дата(1, 1, 2), "ДФ=yyyyMMdd"));
