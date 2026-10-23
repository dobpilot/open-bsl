// Примитивные XML-типы проверяются на платформе и в интерпретаторе.
Перем СериализаторПробы;
Процедура М(Ид, Значение)
 Сообщить(Ид + Символ(9) + Строка(Значение));
КонецПроцедуры
Функция ПробаXMLТипа(ВыражениеПробы)
 Попытка
  ЗначениеПробы = Вычислить(ВыражениеПробы);
  Если ТипЗнч(ЗначениеПробы) = Тип("Тип") Тогда
   Если ЗначениеПробы = Тип("Строка") Тогда Возврат "String"; КонецЕсли;
   Если ЗначениеПробы = Тип("Число") Тогда Возврат "Number"; КонецЕсли;
   Если ЗначениеПробы = Тип("Булево") Тогда Возврат "Boolean"; КонецЕсли;
   Если ЗначениеПробы = Тип("Дата") Тогда Возврат "Date"; КонецЕсли;
   Если ЗначениеПробы = Тип("ДвоичныеДанные") Тогда Возврат "Binary data"; КонецЕсли;
  КонецЕсли;
  Возврат Строка(ЗначениеПробы);
 Исключение
  Возврат "error";
 КонецПопытки;
КонецФункции
М("file.begin", "from-xml-type-1");
М("from_xml_type.anyType", ПробаXMLТипа("ИзXMLТипа(""anyType"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.anySimpleType", ПробаXMLТипа("ИзXMLТипа(""anySimpleType"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.string", ПробаXMLТипа("ИзXMLТипа(""string"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.normalizedString", ПробаXMLТипа("ИзXMLТипа(""normalizedString"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.token", ПробаXMLТипа("ИзXMLТипа(""token"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.Name", ПробаXMLТипа("ИзXMLТипа(""Name"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.NCName", ПробаXMLТипа("ИзXMLТипа(""NCName"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.ID", ПробаXMLТипа("ИзXMLТипа(""ID"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.NMTOKEN", ПробаXMLТипа("ИзXMLТипа(""NMTOKEN"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.language", ПробаXMLТипа("ИзXMLТипа(""language"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.decimal", ПробаXMLТипа("ИзXMLТипа(""decimal"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.integer", ПробаXMLТипа("ИзXMLТипа(""integer"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.long", ПробаXMLТипа("ИзXMLТипа(""long"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.int", ПробаXMLТипа("ИзXMLТипа(""int"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.short", ПробаXMLТипа("ИзXMLТипа(""short"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.byte", ПробаXMLТипа("ИзXMLТипа(""byte"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.nonPositiveInteger", ПробаXMLТипа("ИзXMLТипа(""nonPositiveInteger"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.negativeInteger", ПробаXMLТипа("ИзXMLТипа(""negativeInteger"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.nonNegativeInteger", ПробаXMLТипа("ИзXMLТипа(""nonNegativeInteger"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.positiveInteger", ПробаXMLТипа("ИзXMLТипа(""positiveInteger"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.unsignedLong", ПробаXMLТипа("ИзXMLТипа(""unsignedLong"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.unsignedInt", ПробаXMLТипа("ИзXMLТипа(""unsignedInt"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.unsignedShort", ПробаXMLТипа("ИзXMLТипа(""unsignedShort"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.unsignedByte", ПробаXMLТипа("ИзXMLТипа(""unsignedByte"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.double", ПробаXMLТипа("ИзXMLТипа(""double"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.float", ПробаXMLТипа("ИзXMLТипа(""float"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.boolean", ПробаXMLТипа("ИзXMLТипа(""boolean"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.date", ПробаXMLТипа("ИзXMLТипа(""date"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.dateTime", ПробаXMLТипа("ИзXMLТипа(""dateTime"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.time", ПробаXMLТипа("ИзXMLТипа(""time"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.duration", ПробаXMLТипа("ИзXMLТипа(""duration"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.gYear", ПробаXMLТипа("ИзXMLТипа(""gYear"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.gMonth", ПробаXMLТипа("ИзXMLТипа(""gMonth"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.gDay", ПробаXMLТипа("ИзXMLТипа(""gDay"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.gYearMonth", ПробаXMLТипа("ИзXMLТипа(""gYearMonth"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.gMonthDay", ПробаXMLТипа("ИзXMLТипа(""gMonthDay"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.anyURI", ПробаXMLТипа("ИзXMLТипа(""anyURI"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.base64Binary", ПробаXMLТипа("ИзXMLТипа(""base64Binary"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.hexBinary", ПробаXMLТипа("ИзXMLТипа(""hexBinary"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.QName", ПробаXMLТипа("ИзXMLТипа(""QName"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.NOTATION", ПробаXMLТипа("ИзXMLТипа(""NOTATION"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.IDREF", ПробаXMLТипа("ИзXMLТипа(""IDREF"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.IDREFS", ПробаXMLТипа("ИзXMLТипа(""IDREFS"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.ENTITIES", ПробаXMLТипа("ИзXMLТипа(""ENTITIES"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.NMTOKENS", ПробаXMLТипа("ИзXMLТипа(""NMTOKENS"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.String", ПробаXMLТипа("ИзXMLТипа(""String"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.unknown", ПробаXMLТипа("ИзXMLТипа(""unknown"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.OBJECT", ПробаXMLТипа("ИзXMLТипа(Новый ТипДанныхXML(""string"", ""http://www.w3.org/2001/XMLSchema""))"));
М("from_xml_type.ALIAS", ПробаXMLТипа("FromXMLType(""boolean"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.WRONG_NS", ПробаXMLТипа("ИзXMLТипа(""string"", ""urn:unknown"")"));
М("from_xml_type.EMPTY_NS", ПробаXMLТипа("ИзXMLТипа(""string"", """")"));
М("from_xml_type.CORE_UNDEFINED", ПробаXMLТипа("ИзXMLТипа(""Undefined"", ""http://v8.1c.ru/8.1/data/core"")"));
М("from_xml_type.CORE_UUID", ПробаXMLТипа("ИзXMLТипа(""UUID"", ""http://v8.1c.ru/8.1/data/core"")"));
М("from_xml_type.ONE_STRING", ПробаXMLТипа("ИзXMLТипа(""string"")"));
М("from_xml_type.NUMBER", ПробаXMLТипа("ИзXMLТипа(1)"));
М("from_xml_type.UNDEFINED", ПробаXMLТипа("ИзXMLТипа(Неопределено)"));
М("from_xml_type.BAD_NAMESPACE", ПробаXMLТипа("ИзXMLТипа(""string"", 1)"));
ПутьСхемыПробы = ПолучитьИмяВременногоФайла("xsd");
СостояниеФабрикиПробы = "error";
Попытка
 ЗаписьСхемыПробы = Новый ЗаписьТекста(ПутьСхемыПробы, КодировкаТекста.UTF8);
 ЗаписьСхемыПробы.Записать("<xs:schema xmlns:xs=""http://www.w3.org/2001/XMLSchema"" targetNamespace=""urn:from-xml-type-probe""><xs:element name=""value"" type=""xs:string""/></xs:schema>");
 ЗаписьСхемыПробы.Закрыть();
 СериализаторПробы = Новый СериализаторXDTO(СоздатьФабрикуXDTO(ПутьСхемыПробы));
 СостояниеФабрикиПробы = "ok";
Исключение
 СериализаторПробы = Неопределено;
КонецПопытки;
М("from_xml_type.FACTORY_SETUP", СостояниеФабрикиПробы);
М("from_xml_type.SERIALIZER", ПробаXMLТипа("СериализаторПробы.FromXMLType(""string"", ""http://www.w3.org/2001/XMLSchema"")"));
М("from_xml_type.SERIALIZER_OBJECT", ПробаXMLТипа("СериализаторПробы.ИзXMLТипа(Новый ТипДанныхXML(""decimal"", ""http://www.w3.org/2001/XMLSchema""))"));
Попытка
 УдалитьФайлы(ПутьСхемыПробы);
Исключение
КонецПопытки;
М("file.end", "from-xml-type-1");
