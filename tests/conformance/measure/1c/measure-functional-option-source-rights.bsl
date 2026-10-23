// PLATFORM-ONLY: сеанс пользователя без чтения в собственной базе замера.
Процедура М(Ид, Значение)
 Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры
Функция ПробаОпции(ИмяПробы)
 Попытка
  ЗначениеПробы = ПолучитьФункциональнуюОпцию(ИмяПробы);
  Возврат Строка(ТипЗнч(ЗначениеПробы)) + ":" + Строка(ЗначениеПробы);
 Исключение
  Возврат "ERROR:" + ОписаниеОшибки();
 КонецПопытки;
КонецФункции
М("file.begin", "functional-option-source-rights-1");
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_USER", ПользователиИнформационнойБазы.ТекущийПользователь().Имя);
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_CATALOG_READ", ПравоДоступа("Чтение", Метаданные.Справочники.ЗамерВременныхТаблиц));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_REGISTER_READ", ПравоДоступа("Чтение", Метаданные.РегистрыСведений.ОпцииПробы));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_CATALOG_NORMAL", ПробаОпции("ОпцияСправочникаОбычная"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_CATALOG_PRIVILEGED", ПробаОпции("ОпцияСправочника"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_CATALOG_NORMAL_AFTER", ПробаОпции("ОпцияСправочникаОбычная"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_REGISTER_NORMAL", ПробаОпции("ОпцияРегистраОбычная"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_REGISTER_PRIVILEGED", ПробаОпции("ОпцияРегистра"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_REGISTER_NORMAL_AFTER", ПробаОпции("ОпцияРегистраОбычная"));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_CATALOG_READ_AFTER", ПравоДоступа("Чтение", Метаданные.Справочники.ЗамерВременныхТаблиц));
М("FUNCTIONAL_OPTION.SOURCE_RIGHTS_REGISTER_READ_AFTER", ПравоДоступа("Чтение", Метаданные.РегистрыСведений.ОпцииПробы));
М("file.end", "functional-option-source-rights-1");
