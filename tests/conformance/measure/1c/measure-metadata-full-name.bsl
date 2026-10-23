// PLATFORM-ONLY: полные имена метаданных и вариант встроенного языка.
Процедура М(Ид, Значение)
 Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры
М("file.begin", "metadata-full-name-1");
М("METADATA_FULL_NAME.SCRIPT_VARIANT", Метаданные.ВариантВстроенногоЯзыка);
М("METADATA_FULL_NAME.COMMON_RUSSIAN", Метаданные.ОбщиеРеквизиты.НаименованиеЯзык1.ПолноеИмя());
М("METADATA_FULL_NAME.COMMON_ENGLISH", Metadata.CommonAttributes.НаименованиеЯзык1.FullName());
М("METADATA_FULL_NAME.CATALOG_RUSSIAN", Метаданные.Справочники.ЗамерВременныхТаблиц.ПолноеИмя());
М("METADATA_FULL_NAME.CATALOG_ENGLISH", Metadata.Catalogs.ЗамерВременныхТаблиц.FullName());
М("METADATA_FULL_NAME.OWNER", Метаданные.Справочники.ВладелецПробы.ПолноеИмя());
М("file.end", "metadata-full-name-1");
