Сообщить("settings.division.before" + Символ(9) + "YES");
Попытка
    ХранилищаНастроек.ЗамерХранилища.Загрузить("division");
    Сообщить("settings.division.missed" + Символ(9) + "YES");
Исключение
    Сообщить("settings.division.caught" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
Сообщить("settings.division.after" + Символ(9) + "YES");
