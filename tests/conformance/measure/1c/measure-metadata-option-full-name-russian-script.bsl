// PLATFORM-ONLY: только чтение метаданных собственной базы замера.
Процедура М(Ид, Значение)
 Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры
М("file.begin", "metadata-option-full-name-russian-script-1");
М("METADATA_OPTION_FULL_NAME.CONSTANT_RUSSIAN", Метаданные.Константы.ЗначениеОпцииПробы.ПолноеИмя());
М("METADATA_OPTION_FULL_NAME.CONSTANT_ENGLISH", Metadata.Constants.ЗначениеОпцииПробы.FullName());
М("METADATA_OPTION_FULL_NAME.OPTION_RUSSIAN", Метаданные.ФункциональныеОпции.ОпцияПробы.ПолноеИмя());
М("METADATA_OPTION_FULL_NAME.OPTION_ENGLISH", Metadata.FunctionalOptions.ОпцияПробы.FullName());
М("file.end", "metadata-option-full-name-russian-script-1");
