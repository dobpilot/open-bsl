// PLATFORM-ONLY: проба фиксирует полное перечисление способов обхода результата запроса.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "query-result-iteration-1");
М("QUERY.RESULT.ITERATION.LINEAR", ОбходРезультатаЗапроса.Прямой);
М("QUERY.RESULT.ITERATION.BY.GROUPS", ОбходРезультатаЗапроса.ПоГруппировкам);
М("QUERY.RESULT.ITERATION.BY.GROUPS.WITH.HIERARCHY", ОбходРезультатаЗапроса.ПоГруппировкамСИерархией);
М("QUERY.RESULT.ITERATION.TYPE", ТипЗнч(ОбходРезультатаЗапроса.Прямой));

АнглийскиеИмена = "ERROR";
Попытка
	Если QueryResultIteration.Linear = ОбходРезультатаЗапроса.Прямой
		И QueryResultIteration.ByGroups = ОбходРезультатаЗапроса.ПоГруппировкам
		И QueryResultIteration.ByGroupsWithHierarchy = ОбходРезультатаЗапроса.ПоГруппировкамСИерархией Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("QUERY.RESULT.ITERATION.ENGLISH", АнглийскиеИмена);
М("file.end", "query-result-iteration-1");
