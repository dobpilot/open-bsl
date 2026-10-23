// PLATFORM-ONLY: структура свойства текущего пользователя в собственной базе.
Процедура М(Ид, Значение)
    Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "current-user-separation-snapshot-1");
ПользовательСнимкаПробы = ПользователиИнформационнойБазы.ТекущийПользователь();
СтруктураСнимкаПробы = ПользовательСнимкаПробы.РазделениеДанных;
Попытка
    СтруктураСнимкаПробы.Вставить("НаименованиеЯзык1", "own-current-user-probe");
    ИзменениеСнимкаПробы = "ACCEPTED";
Исключение
    ИзменениеСнимкаПробы = "REFUSED";
КонецПопытки;
М("CURRENT_USER_SEPARATION_SNAPSHOT.MUTATE_RESULT", ИзменениеСнимкаПробы);
М("CURRENT_USER_SEPARATION_SNAPSHOT.RETURNED_COUNT", СтруктураСнимкаПробы.Количество());
М("CURRENT_USER_SEPARATION_SNAPSHOT.SAME_USER_COUNT", ПользовательСнимкаПробы.РазделениеДанных.Количество());
М("CURRENT_USER_SEPARATION_SNAPSHOT.NEXT_USER_COUNT", ПользователиИнформационнойБазы.ТекущийПользователь().РазделениеДанных.Количество());
Попытка
    ПользовательСнимкаПробы.РазделениеДанных = Новый Структура("НаименованиеЯзык1", "own-assignment-probe");
    НазначениеСнимкаПробы = "ACCEPTED";
Исключение
    НазначениеСнимкаПробы = "REFUSED";
КонецПопытки;
М("CURRENT_USER_SEPARATION_SNAPSHOT.ASSIGN_RESULT", НазначениеСнимкаПробы);
М("CURRENT_USER_SEPARATION_SNAPSHOT.AFTER_ASSIGN_COUNT", ПользовательСнимкаПробы.РазделениеДанных.Количество());
М("file.end", "current-user-separation-snapshot-1");
