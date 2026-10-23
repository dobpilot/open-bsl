Сообщить("settings.initialization.before" + Символ(9) + "YES");
Попытка
    ХранилищаНастроек.ЗамерХранилища.Загрузить("initialization");
    Сообщить("settings.initialization.missed" + Символ(9) + "YES");
Исключение
    Сообщить("settings.initialization.caught" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
Сообщить("settings.initialization.after" + Символ(9) + "YES");
