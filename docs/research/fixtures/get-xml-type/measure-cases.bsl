Процедура ПроверитьТипПробы(Идентификатор, ТекстXML, Шаги)
    Попытка
        ЧтениеТипаПробы = Новый ЧтениеXML;
        ЧтениеТипаПробы.УстановитьСтроку(ТекстXML);
        Для НомерШагаПробы = 1 По Шаги Цикл ЧтениеТипаПробы.Прочитать(); КонецЦикла;
        ТипXMLПробы = ПолучитьXMLТип(ЧтениеТипаПробы);
        Если ТипXMLПробы = Неопределено Тогда
            ОписаниеТипаПробы = "undefined";
        Иначе
            ОписаниеТипаПробы = ТипXMLПробы.URIПространстваИмен + "|" + ТипXMLПробы.ИмяТипа + "|" + ?(ТипЗнч(ТипXMLПробы) = Тип("ТипДанныхXML"), "1", "0");
        КонецЕсли;
        Сообщить(Идентификатор + Символ(9) + ОписаниеТипаПробы);
    Исключение
        Сообщить(Идентификатор + Символ(9) + "error:" + ОписаниеОшибки());
    КонецПопытки;
КонецПроцедуры
ПроверитьТипПробы("xml.gettype.initial", "<a/>", 0);
ПроверитьТипПробы("xml.gettype.plain", "<a/>", 1);
ПроверитьТипПробы("xml.gettype.namespace", "<thing xmlns=""urn:probe""/>", 1);
ПроверитьТипПробы("xml.gettype.builtin", "<string>value</string>", 1);
ПроверитьТипПробы("xml.gettype.explicit", "<a xmlns:xsi=""http://www.w3.org/2001/XMLSchema-instance"" xmlns:x=""urn:probe"" xsi:type=""x:Kind""/>", 1);
ПроверитьТипПробы("xml.gettype.missingprefix", "<a xmlns:xsi=""http://www.w3.org/2001/XMLSchema-instance"" xsi:type=""x:Kind""/>", 1);
ПроверитьТипПробы("xml.gettype.unprefixed", "<a xmlns=""urn:probe"" xmlns:xsi=""http://www.w3.org/2001/XMLSchema-instance"" xsi:type=""Kind""/>", 1);
ПроверитьТипПробы("xml.gettype.niltrue", "<a xmlns:xsi=""http://www.w3.org/2001/XMLSchema-instance"" xsi:nil=""true""/>", 1);
ПроверитьТипПробы("xml.gettype.nilone", "<a xmlns:xsi=""http://www.w3.org/2001/XMLSchema-instance"" xsi:nil=""1""/>", 1);
ПроверитьТипПробы("xml.gettype.text", "<a>value</a>", 2);
ПроверитьТипПробы("xml.gettype.end", "<a>value</a>", 3);
ПроверитьТипПробы("xml.gettype.whitespace", "<a>  <b/></a>", 2);
