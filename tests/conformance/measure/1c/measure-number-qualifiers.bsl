// PLATFORM-ONLY: проба фиксирует конструктор и свойства квалификаторов числа.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "number-qualifiers-1");

ПоУмолчанию = Новый КвалификаторыЧисла;
М("NUMBER.QUALIFIERS.DEFAULT.DIGITS", ПоУмолчанию.Разрядность);
М("NUMBER.QUALIFIERS.DEFAULT.FRACTION", ПоУмолчанию.РазрядностьДробнойЧасти);
М("NUMBER.QUALIFIERS.DEFAULT.SIGN", ПоУмолчанию.ДопустимыйЗнак);
М("NUMBER.QUALIFIERS.TYPE", ТипЗнч(ПоУмолчанию));

Явные = Новый КвалификаторыЧисла(10, 2, ДопустимыйЗнак.Неотрицательный);
М("NUMBER.QUALIFIERS.EXPLICIT.DIGITS", Явные.Разрядность);
М("NUMBER.QUALIFIERS.EXPLICIT.FRACTION", Явные.РазрядностьДробнойЧасти);
М("NUMBER.QUALIFIERS.EXPLICIT.SIGN", Явные.ДопустимыйЗнак);

АнглийскиеИмена = "ERROR";
Попытка
	English = New NumberQualifiers(8, 3, AllowedSign.Any);
	Если English.Digits = 8
		И English.FractionDigits = 3
		И English.AllowedSign = ДопустимыйЗнак.Любой Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("NUMBER.QUALIFIERS.ENGLISH", АнглийскиеИмена);

Изменяемость = "MUTABLE";
Попытка
	Явные.Разрядность = 5;
Исключение
	Изменяемость = "READONLY";
КонецПопытки;
М("NUMBER.QUALIFIERS.MUTABILITY", Изменяемость);

Нормализация = Новый КвалификаторыЧисла(-1, 1.5);
М("NUMBER.QUALIFIERS.NORMALIZED.DIGITS", Нормализация.Разрядность);
М("NUMBER.QUALIFIERS.NORMALIZED.FRACTION", Нормализация.РазрядностьДробнойЧасти);

ДробнаяРазрядность = Новый КвалификаторыЧисла(10, 1.5);
М("NUMBER.QUALIFIERS.TRUNCATED.FRACTION", ДробнаяРазрядность.РазрядностьДробнойЧасти);

ЛишняяДробнаяЧасть = "ERROR";
Попытка
	Лишние = Новый КвалификаторыЧисла(3, 5);
	ЛишняяДробнаяЧасть = Строка(Лишние.РазрядностьДробнойЧасти);
Исключение
КонецПопытки;
М("NUMBER.QUALIFIERS.EXCESS.FRACTION", ЛишняяДробнаяЧасть);

НеверныйЗнак = "ACCEPTED";
Попытка
	Неверные = Новый КвалификаторыЧисла(10, 2, "Любой");
Исключение
	НеверныйЗнак = "ERROR";
КонецПопытки;
М("NUMBER.QUALIFIERS.BAD.SIGN", НеверныйЗнак);

М("file.end", "number-qualifiers-1");
