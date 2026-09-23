// PLATFORM-ONLY: проба фиксирует достигнутую поверхность управляемой блокировки.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-lock-surface-1");
Блокировка = Новый БлокировкаДанных;
Элемент = Блокировка.Добавить();
М("DATA.LOCK.TYPE", ТипЗнч(Блокировка));
М("DATA.LOCK.ITEM.TYPE", ТипЗнч(Элемент));

LockValue = New DataLock;
ItemValue = LockValue.Add();
М("DATA.LOCK.ALIAS.TYPE", ТипЗнч(LockValue) = ТипЗнч(Блокировка));
М("DATA.LOCK.ALIAS.ITEM", ТипЗнч(ItemValue) = ТипЗнч(Элемент));
М("file.end", "data-lock-surface-1");
