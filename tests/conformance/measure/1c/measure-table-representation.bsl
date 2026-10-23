// PLATFORM-ONLY: проба фиксирует полное перечисление вариантов отображения таблицы.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "table-representation-1");
М("TABLE.REPRESENTATION.LIST", ОтображениеТаблицы.Список);
М("TABLE.REPRESENTATION.HIERARCHICAL_LIST", ОтображениеТаблицы.ИерархическийСписок);
М("TABLE.REPRESENTATION.TREE", ОтображениеТаблицы.Дерево);
М("TABLE.REPRESENTATION.TYPE", ТипЗнч(ОтображениеТаблицы.Список));

АнглийскиеИмена = "ERROR";
Попытка
	Если TableRepresentation.List = ОтображениеТаблицы.Список
		И TableRepresentation.HierarchicalList = ОтображениеТаблицы.ИерархическийСписок
		И TableRepresentation.Tree = ОтображениеТаблицы.Дерево Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("TABLE.REPRESENTATION.ENGLISH", АнглийскиеИмена);
М("file.end", "table-representation-1");
