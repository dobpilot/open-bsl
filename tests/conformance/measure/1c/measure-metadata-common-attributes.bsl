// PLATFORM-ONLY: корневая коллекция общих реквизитов собственной конфигурации.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "metadata-common-attributes-1");
КоллекцияПробы = Метаданные.ОбщиеРеквизиты;
М("METADATA.COMMON_ATTRIBUTES.COUNT", КоллекцияПробы.Количество());
М("METADATA.COMMON_ATTRIBUTES.ENGLISH", Metadata.CommonAttributes.Count());
М("METADATA.COMMON_ATTRIBUTES.INDEX", КоллекцияПробы[0].Имя);
М("METADATA.COMMON_ATTRIBUTES.GET", КоллекцияПробы.Получить(0).Имя);
М("METADATA.COMMON_ATTRIBUTES.FIND", КоллекцияПробы.Найти("наименованиеязык1").Имя);
РезультатПробы = "0";
Если КоллекцияПробы.Найти("НесуществующийРеквизит") = Неопределено Тогда
	РезультатПробы = "1";
КонецЕсли;
М("METADATA.COMMON_ATTRIBUTES.MISSING", РезультатПробы);
ИменаПробы = "";
Для Каждого ЭлементПробы Из КоллекцияПробы Цикл
	ИменаПробы = ИменаПробы + ЭлементПробы.Имя + ";";
КонецЦикла;
М("METADATA.COMMON_ATTRIBUTES.ITERATION", ИменаПробы);
РезультатПробы = "0";
Попытка
	Метаданные.ОбщиеРеквизиты = Неопределено;
Исключение
	РезультатПробы = "1";
КонецПопытки;
М("METADATA.COMMON_ATTRIBUTES.READ_ONLY", РезультатПробы);
М("file.end", "metadata-common-attributes-1");
