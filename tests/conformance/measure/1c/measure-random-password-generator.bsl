// PLATFORM-ONLY: проба фиксирует поверхность генератора случайных паролей.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "random-password-generator-1");

Генератор = Новый ГенераторСлучайныхПаролей;
М("PASSWORD.GENERATOR.TYPE", ТипЗнч(Генератор));

ПарольПоУмолчанию = Генератор.СлучайныйПароль();
М("PASSWORD.GENERATOR.DEFAULT.TYPE", ТипЗнч(ПарольПоУмолчанию));
М("PASSWORD.GENERATOR.DEFAULT.LENGTH.AT_LEAST_7", СтрДлина(ПарольПоУмолчанию) >= 7);

ПарольЗапоминающийся = Генератор.СлучайныйПароль(12, Истина);
М("PASSWORD.GENERATOR.MEMORABLE.LENGTH", СтрДлина(ПарольЗапоминающийся));

ПарольПроизвольный = Генератор.СлучайныйПароль(12, Ложь);
М("PASSWORD.GENERATOR.ARBITRARY.LENGTH", СтрДлина(ПарольПроизвольный));

МинимальныйПароль = Генератор.СлучайныйПароль(1);
М("PASSWORD.GENERATOR.ONE.LENGTH", СтрДлина(МинимальныйПароль));

АнглийскиеИмена = "ERROR";
Попытка
	EnglishGenerator = New RandomPasswordGenerator;
	EnglishPassword = EnglishGenerator.RandomPassword(9, False);
	Если СтрДлина(EnglishPassword) = 9 Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("PASSWORD.GENERATOR.ENGLISH", АнглийскиеИмена);

НулеваяДлина = "ERROR";
Попытка
	НулеваяДлина = СтрДлина(Генератор.СлучайныйПароль(0));
Исключение
КонецПопытки;
М("PASSWORD.GENERATOR.ZERO.LENGTH", НулеваяДлина);

ОтрицательнаяДлина = "ACCEPTED";
Попытка
	Генератор.СлучайныйПароль(-1);
Исключение
	ОтрицательнаяДлина = "ERROR";
КонецПопытки;
М("PASSWORD.GENERATOR.NEGATIVE.LENGTH", ОтрицательнаяДлина);

ДробнаяДлина = "ERROR";
Попытка
	ДробнаяДлина = СтрДлина(Генератор.СлучайныйПароль(7.9));
Исключение
КонецПопытки;
М("PASSWORD.GENERATOR.FRACTIONAL.LENGTH", ДробнаяДлина);

НеверныйФлаг = "ACCEPTED";
Попытка
	Генератор.СлучайныйПароль(8, "Истина");
Исключение
	НеверныйФлаг = "ERROR";
КонецПопытки;
М("PASSWORD.GENERATOR.BAD.MEMORABLE", НеверныйФлаг);

М("file.end", "random-password-generator-1");
