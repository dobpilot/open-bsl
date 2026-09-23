// PLATFORM-ONLY: проба фиксирует доступность обоих имён глобальной процедуры.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "refresh-reusable-values-1");
ОбновитьПовторноИспользуемыеЗначения();
RefreshReusableValues();
М("REFRESH.REUSABLE.COMPLETED", Истина);
М("file.end", "refresh-reusable-values-1");
