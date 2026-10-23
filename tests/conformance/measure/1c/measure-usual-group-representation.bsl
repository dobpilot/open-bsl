// PLATFORM-ONLY: проба фиксирует закрытое перечисление ОтображениеОбычнойГруппы.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "usual-group-representation-1");
М("FORM.USUAL.NONE", ОтображениеОбычнойГруппы.Нет);
М("FORM.USUAL.NORMAL", ОтображениеОбычнойГруппы.ОбычноеВыделение);
М("FORM.USUAL.STRONG", ОтображениеОбычнойГруппы.СильноеВыделение);
М("FORM.USUAL.WEAK", ОтображениеОбычнойГруппы.СлабоеВыделение);
М("FORM.USUAL.TYPE", ТипЗнч(ОтображениеОбычнойГруппы.Нет));
М("FORM.USUAL.ALIAS.NONE", ОтображениеОбычнойГруппы.Нет = UsualGroupRepresentation.None);
М("FORM.USUAL.ALIAS.NORMAL", ОтображениеОбычнойГруппы.ОбычноеВыделение = UsualGroupRepresentation.NormalSeparation);
М("FORM.USUAL.ALIAS.STRONG", ОтображениеОбычнойГруппы.СильноеВыделение = UsualGroupRepresentation.StrongSeparation);
М("FORM.USUAL.ALIAS.WEAK", ОтображениеОбычнойГруппы.СлабоеВыделение = UsualGroupRepresentation.WeakSeparation);
М("file.end", "usual-group-representation-1");
