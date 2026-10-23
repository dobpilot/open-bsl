Попытка
    ОбъектПробы = Вычислить("Документы.ЗамерДокумента.СоздатьДокумент()");
    Для Каждого ЗначениеПробы Из ОбъектПробы.ПрочитатьРезультатыТела() Цикл
        Сообщить("document.property.number.body." + ЗначениеПробы);
    КонецЦикла;
    Сообщить("document.property.number.number.type=" + Строка(ТипЗнч(ОбъектПробы.Номер)));
    Сообщить("document.property.number.number=" + "[" + Строка(ОбъектПробы.Номер) + "]");
    Сообщить("document.property.number.date.empty=" + Строка(ОбъектПробы.Дата = Дата(1, 1, 1)));
    Сообщить("document.property.number.isnew=" + Строка(ОбъектПробы.ЭтоНовый()));
    Сообщить("document.property.number.ref.empty=" + Строка(ОбъектПробы.Ссылка.Пустая()));
    Сообщить("document.property.number.deletion=" + Строка(ОбъектПробы.ПометкаУдаления));
    Сообщить("document.property.number.posted=" + Строка(ОбъектПробы.Проведен));
Исключение
    Сообщить("document.property.number.error=" + ОписаниеОшибки());
КонецПопытки;
