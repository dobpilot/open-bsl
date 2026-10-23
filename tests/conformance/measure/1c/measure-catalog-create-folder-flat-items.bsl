Попытка
    ОбъектПробы = Вычислить("Справочники.ЗамерВременныхТаблиц.СоздатьГруппу()");
    Для Каждого ИтогТелаПробы Из ОбъектПробы.ПрочитатьТело() Цикл
        Сообщить("folder.create.flat_items.body." + ИтогТелаПробы);
    КонецЦикла;
    Сообщить("folder.create.flat_items.bare=" + ?(ОбъектПробы.ПрочитатьПризнак(), "YES", "NO"));
    Сообщить("folder.create.flat_items.qualified=" + ?(ОбъектПробы.ЭтоГруппа, "YES", "NO"));
    Сообщить("folder.create.flat_items.alias=" + ?(ОбъектПробы.IsFolder = ОбъектПробы.ЭтоГруппа, "YES", "NO"));
    Сообщить("folder.create.flat_items.type=" + ?(ТипЗнч(ОбъектПробы.IsFolder) = Тип("Boolean"), "YES", "NO"));
    Попытка
        ОбъектПробы.IsFolder = Истина;
        ИтогЗаписиПробы = "accepted";
    Исключение
        ИтогЗаписиПробы = "rejected";
    КонецПопытки;
    Сообщить("folder.create.flat_items.write=" + ИтогЗаписиПробы);
    Сообщить("folder.create.flat_items.after=" + ?(ОбъектПробы.ЭтоГруппа, "YES", "NO"));
Исключение
    Сообщить("folder.create.flat_items.error=" + ОписаниеОшибки());
КонецПопытки;
