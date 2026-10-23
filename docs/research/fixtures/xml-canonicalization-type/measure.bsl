Для Каждого ИмяПробы Из СтрРазделить("КаноническийXML,XMLCanonicalization,КаноническийXMLСКомментариями,XMLCanonicalizationWithComments,ИсключающийКаноническийXML,XMLExclusiveCanonicalization,ИсключающийКаноническийXMLСКомментариями,XMLExclusiveCanonicalizationWithComments,КаноническийXML1_1,XMLCanonicalization1_1,КаноническийXML1_1СКомментариями,XMLCanonicalization1_1WithComments", ",") Цикл
    Попытка
        ЗначениеПробы = Вычислить("ТипКаноническогоXML." + ИмяПробы);
        АнглийскоеЗначениеПробы = Вычислить("XMLCanonicalizationType." + ИмяПробы);
        Сообщить("xml.canonical." + ИмяПробы + Символ(9) + Строка(ЗначениеПробы) + Символ(9) + ?(ЗначениеПробы = АнглийскоеЗначениеПробы, "1", "0"));
    Исключение
        Сообщить("xml.canonical.error." + ИмяПробы + Символ(9) + ОписаниеОшибки());
    КонецПопытки;
КонецЦикла;
Попытка
    Сообщить("xml.canonical.root" + Символ(9) + Строка(Вычислить("ТипКаноническогоXML")));
    Сообщить("xml.canonical.type" + Символ(9) + Строка(ТипЗнч(Вычислить("ТипКаноническогоXML.КаноническийXML"))));
    Сообщить("xml.canonical.type.alias" + Символ(9) + ?(Тип("ТипКаноническогоXML") = Тип("XMLCanonicalizationType"), "1", "0"));
Исключение
    Сообщить("xml.canonical.root.error" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
