// PLATFORM-ONLY: проба фиксирует чтение проверки сложности паролей.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "user-password-strength-check-1");
М("PASSWORD.STRENGTH.TYPE", ТипЗнч(ПолучитьПроверкуСложностиПаролейПользователей()));
М("PASSWORD.STRENGTH.VALUE", ПолучитьПроверкуСложностиПаролейПользователей());

АнглийскоеИмя = "ERROR";
Попытка
	Если ТипЗнч(GetUserPasswordStrengthCheck()) = Тип("Булево") Тогда
		АнглийскоеИмя = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("PASSWORD.STRENGTH.ENGLISH", АнглийскоеИмя);
М("file.end", "user-password-strength-check-1");
