// PLATFORM-ONLY: проба фиксирует полное перечисление назначения типа XML.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "xml-type-assignment-1");
М("XML.TYPE.ASSIGNMENT.IMPLICIT", НазначениеТипаXML.Неявное);
М("XML.TYPE.ASSIGNMENT.EXPLICIT", НазначениеТипаXML.Явное);
М("XML.TYPE.ASSIGNMENT.TYPE", ТипЗнч(НазначениеТипаXML.Явное));

АнглийскиеИмена = "ERROR";
Попытка
	Если XMLTypeAssignment.Implicit = НазначениеТипаXML.Неявное
		И XMLTypeAssignment.Explicit = НазначениеТипаXML.Явное Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("XML.TYPE.ASSIGNMENT.ENGLISH", АнглийскиеИмена);
М("file.end", "xml-type-assignment-1");
