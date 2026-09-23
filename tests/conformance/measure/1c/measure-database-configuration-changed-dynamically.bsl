// PLATFORM-ONLY: проба фиксирует тип и результат глобальной функции в новом сеансе.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "database-configuration-changed-dynamically-1");
М("CONFIG.DYNAMIC.VALUE", КонфигурацияБазыДанныхИзмененаДинамически());
М("CONFIG.DYNAMIC.TYPE", ТипЗнч(КонфигурацияБазыДанныхИзмененаДинамически()));
М("CONFIG.DYNAMIC.ALIAS", КонфигурацияБазыДанныхИзмененаДинамически() = DataBaseConfigurationChangedDynamically());
М("file.end", "database-configuration-changed-dynamically-1");
