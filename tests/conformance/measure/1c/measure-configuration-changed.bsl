// PLATFORM-ONLY: проба фиксирует тип и результат глобальной функции.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "configuration-changed-1");
М("CONFIG.CHANGED.VALUE", КонфигурацияИзменена());
М("CONFIG.CHANGED.TYPE", ТипЗнч(КонфигурацияИзменена()));
М("CONFIG.CHANGED.ALIAS", КонфигурацияИзменена() = ConfigurationChanged());
М("file.end", "configuration-changed-1");
