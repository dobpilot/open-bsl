// PLATFORM-ONLY: проба фиксирует полное перечисление типов применения отбора.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "data-composition-filter-application-type-1");
М("DCS.FILTER.APPLICATION.ITEMS", ТипПримененияОтбораКомпоновкиДанных.Элементы);
М("DCS.FILTER.APPLICATION.HIERARCHY", ТипПримененияОтбораКомпоновкиДанных.Иерархия);
М("DCS.FILTER.APPLICATION.HIERARCHY_ONLY", ТипПримененияОтбораКомпоновкиДанных.ТолькоИерархия);
М("DCS.FILTER.APPLICATION.TYPE", ТипЗнч(ТипПримененияОтбораКомпоновкиДанных.Элементы));

АнглийскиеИмена = "ERROR";
Попытка
	Если DataCompositionFilterApplicationType.Items = ТипПримененияОтбораКомпоновкиДанных.Элементы
		И DataCompositionFilterApplicationType.Hierarchy = ТипПримененияОтбораКомпоновкиДанных.Иерархия
		И DataCompositionFilterApplicationType.HierarchyOnly = ТипПримененияОтбораКомпоновкиДанных.ТолькоИерархия Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("DCS.FILTER.APPLICATION.ENGLISH", АнглийскиеИмена);
М("file.end", "data-composition-filter-application-type-1");
