// PLATFORM-ONLY: проба фиксирует ТипДанныхXML до поддержки этого значения runtime.

Процедура М(Ид, Значение)
	Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры

М("file.begin", "xml-type-of-1");
ПространствоXSD = "http://www.w3.org/2001/XMLSchema";
ТипСтроки = XMLТипЗнч("текст");
ТипЧисла = XMLТипЗнч(1);
ТипБулева = XMLТипЗнч(Истина);
ТипДаты = XMLТипЗнч('20240101');
М("XML.TYPE.STRING.TYPE", ТипЗнч(ТипСтроки));
М("XML.TYPE.STRING.VALUE", ТипСтроки);
М("XML.TYPE.STRING.NAME", ТипСтроки.ИмяТипа);
М("XML.TYPE.STRING.NAMESPACE", ТипСтроки.URIПространстваИмен);
М("XML.TYPE.NUMBER", ТипЧисла);
М("XML.TYPE.BOOLEAN", ТипБулева);
М("XML.TYPE.DATE", ТипДаты);
М("XML.TYPE.UNDEFINED", XMLТипЗнч(Неопределено) = Неопределено);
М("XML.TYPE.ARRAY", XMLТипЗнч(Новый Массив) = Неопределено);

Сконструированный = Новый ТипДанныхXML("string", ПространствоXSD);
М("XML.TYPE.CONSTRUCTED.EQUAL", Сконструированный = ТипСтроки);
М("XML.TYPE.CONSTRUCTED.TYPE", ТипЗнч(Сконструированный));

АнглийскиеИмена = "ERROR";
Попытка
	EnglishType = XMLTypeOf("text");
	EnglishConstructed = New XMLDataType("string", ПространствоXSD);
	Если EnglishType = EnglishConstructed
		И EnglishType.TypeName = "string"
		И EnglishType.NamespaceURI = ПространствоXSD Тогда
		АнглийскиеИмена = "OK";
	КонецЕсли;
Исключение
КонецПопытки;
М("XML.TYPE.ENGLISH", АнглийскиеИмена);

ЗаписьСвойства = "OK";
Попытка
	Сконструированный.ИмяТипа = "decimal";
Исключение
	ЗаписьСвойства = "ERROR";
КонецПопытки;
М("XML.TYPE.WRITE", ЗаписьСвойства);
М("file.end", "xml-type-of-1");
