// PLATFORM-ONLY: проба фиксирует закрытое перечисление ВидГруппыФормы.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "form-group-type-1");
М("FORM.GROUP.BUTTON", ВидГруппыФормы.ГруппаКнопок);
М("FORM.GROUP.COLUMN", ВидГруппыФормы.ГруппаКолонок);
М("FORM.GROUP.COMMAND", ВидГруппыФормы.КоманднаяПанель);
М("FORM.GROUP.CONTEXT", ВидГруппыФормы.КонтекстноеМеню);
М("FORM.GROUP.USUAL", ВидГруппыФормы.ОбычнаяГруппа);
М("FORM.GROUP.POPUP", ВидГруппыФормы.Подменю);
М("FORM.GROUP.PAGE", ВидГруппыФормы.Страница);
М("FORM.GROUP.PAGES", ВидГруппыФормы.Страницы);
М("FORM.GROUP.TYPE", ТипЗнч(ВидГруппыФормы.ОбычнаяГруппа));
М("FORM.GROUP.ALIAS.BUTTON", ВидГруппыФормы.ГруппаКнопок = FormGroupType.ButtonGroup);
М("FORM.GROUP.ALIAS.COLUMN", ВидГруппыФормы.ГруппаКолонок = FormGroupType.ColumnGroup);
М("FORM.GROUP.ALIAS.COMMAND", ВидГруппыФормы.КоманднаяПанель = FormGroupType.CommandBar);
М("FORM.GROUP.ALIAS.CONTEXT", ВидГруппыФормы.КонтекстноеМеню = FormGroupType.ContextMenu);
М("FORM.GROUP.ALIAS.USUAL", ВидГруппыФормы.ОбычнаяГруппа = FormGroupType.UsualGroup);
М("FORM.GROUP.ALIAS.POPUP", ВидГруппыФормы.Подменю = FormGroupType.Popup);
М("FORM.GROUP.ALIAS.PAGE", ВидГруппыФормы.Страница = FormGroupType.Page);
М("FORM.GROUP.ALIAS.PAGES", ВидГруппыФормы.Страницы = FormGroupType.Pages);
М("file.end", "form-group-type-1");
