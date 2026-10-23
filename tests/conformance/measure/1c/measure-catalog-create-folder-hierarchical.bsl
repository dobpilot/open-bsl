Попытка
    ОбъектПробы = Вычислить("Справочники.ЗамерВременныхТаблиц.СоздатьГруппу()");
    Для Каждого ИтогТелаПробы Из ОбъектПробы.ПрочитатьТело() Цикл
        Сообщить("folder.create.hierarchical.body." + ИтогТелаПробы);
    КонецЦикла;
    Сообщить("folder.create.hierarchical.bare=" + ?(ОбъектПробы.ПрочитатьПризнак(), "YES", "NO"));
    Сообщить("folder.create.hierarchical.qualified=" + ?(ОбъектПробы.ЭтоГруппа, "YES", "NO"));
    Сообщить("folder.create.hierarchical.alias=" + ?(ОбъектПробы.IsFolder = ОбъектПробы.ЭтоГруппа, "YES", "NO"));
    Сообщить("folder.create.hierarchical.type=" + ?(ТипЗнч(ОбъектПробы.IsFolder) = Тип("Boolean"), "YES", "NO"));
    Попытка
        ОбъектПробы.IsFolder = Истина;
        ИтогЗаписиПробы = "accepted";
    Исключение
        ИтогЗаписиПробы = "rejected";
    КонецПопытки;
    Сообщить("folder.create.hierarchical.write=" + ИтогЗаписиПробы);
    Сообщить("folder.create.hierarchical.after=" + ?(ОбъектПробы.ЭтоГруппа, "YES", "NO"));
Исключение
    Сообщить("folder.create.hierarchical.error=" + ОписаниеОшибки());
КонецПопытки;
