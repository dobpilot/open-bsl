Попытка
    ОбъектПробы = Вычислить("Документы.ЗамерДокумента.СоздатьДокумент()");
    Для Каждого ЗначениеПробы Из ОбъектПробы.ПрочитатьРезультатыТела() Цикл
        Сообщить("document.property.disabled.body." + ЗначениеПробы);
    КонецЦикла;
    Сообщить("document.property.disabled.number.type=" + Строка(ТипЗнч(ОбъектПробы.Номер)));
    Сообщить("document.property.disabled.number=" + "[" + Строка(ОбъектПробы.Номер) + "]");
    Сообщить("document.property.disabled.date.empty=" + Строка(ОбъектПробы.Дата = Дата(1, 1, 1)));
    Сообщить("document.property.disabled.isnew=" + Строка(ОбъектПробы.ЭтоНовый()));
    Сообщить("document.property.disabled.ref.empty=" + Строка(ОбъектПробы.Ссылка.Пустая()));
    Сообщить("document.property.disabled.deletion=" + Строка(ОбъектПробы.ПометкаУдаления));
    Сообщить("document.property.disabled.posted=" + Строка(ОбъектПробы.Проведен));
Исключение
    Сообщить("document.property.disabled.error=" + ОписаниеОшибки());
КонецПопытки;
