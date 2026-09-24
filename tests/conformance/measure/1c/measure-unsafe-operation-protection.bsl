// PLATFORM-ONLY

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "unsafe-operation-protection-1");
Описание = Новый ОписаниеЗащитыОтОпасныхДействий;
М("SECURITY.UNSAFE_PROTECTION.TYPE", ТипЗнч(Описание));
М("SECURITY.UNSAFE_PROTECTION.DEFAULT", Описание.ПредупреждатьОбОпасныхДействиях);
Описание.ПредупреждатьОбОпасныхДействиях = Ложь;
М("SECURITY.UNSAFE_PROTECTION.FALSE", Описание.ПредупреждатьОбОпасныхДействиях);
Описание.UnsafeOperationWarnings = Истина;
М("SECURITY.UNSAFE_PROTECTION.ENGLISH", Описание.UnsafeOperationWarnings);
АнглийскийТип = "ERROR";
Попытка
	АнглийскоеОписание = Новый UnsafeOperationProtectionDescription;
	АнглийскийТип = ТипЗнч(АнглийскоеОписание);
Исключение
КонецПопытки;
М("SECURITY.UNSAFE_PROTECTION.ENGLISH_TYPE", АнглийскийТип);
М("file.end", "unsafe-operation-protection-1");
