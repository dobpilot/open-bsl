// Порядок свойств в `ФабрикаXDTO.ЗаписатьXML`: схемный у закрытого типа,
// порядок заполнения у ОТКРЫТОГО — того, в чьём объявлении есть маска
// `xs:any` или `xs:anyAttribute`.
//
// Эталон снят с платформы 8.3.27:
//   ./tests/conformance/measure/1c/run-on-1c.sh \
//       tests/conformance/fixtures/xdto-write-order.bsl
//
// Пробы, которые отвечают на «почему», и их разбор — в
// `tests/conformance/measure/measure-xdto-order.bsl` рядом со снятым
// `measure-xdto-order.platform.txt`; ИД якорей — `XDTO.WRITE_ORDER.*`.
// Здесь та же развилка в виде сети регрессий: каждый тип схемы отличается
// от соседа ровно одной чертой, а заполнение везде одно и то же и ПРОТИВ
// схемы — сначала `b`, потом `a`.
//
// Схема пишется во временный файл: `СоздатьФабрикуXDTO` берёт только путь
// к XSD (измерено), и путь работает по обе стороны.

Перем Фаб;

Процедура П(Метка, Значение)
	Сообщить(Метка + ": " + Значение);
КонецПроцедуры

// Переводы строки и табуляции записанного XML — видимыми, иначе
// построчное сличение с платформой ничего не покажет.
Функция Видимо(Текст)
	Р = СтрЗаменить(Текст, Символ(13), "<ВК>");
	Р = СтрЗаменить(Р, Символ(10), "<ПС>");
	Р = СтрЗаменить(Р, Символ(9), "<Т>");
	Возврат Р;
КонецФункции

Функция Зап(Об)
	Попытка
		Зп = Новый ЗаписьXML;
		Зп.УстановитьСтроку();
		Фаб.ЗаписатьXML(Зп, Об, "к");
		Возврат Видимо(Зп.Закрыть());
	Исключение
		Возврат "ошибка";
	КонецПопытки;
КонецФункции

Функция Флаги(Тип)
	Возврат "открытый=" + Строка(Тип.Открытый)
		+ " упорядоченный=" + Строка(Тип.Упорядоченный)
		+ " последовательный=" + Строка(Тип.Последовательный);
КонецФункции

// Заполнение ПРОТИВ схемного порядка: сначала `b`, потом `a`.
Функция Обратно(Имя)
	Об = Фаб.Создать(Фаб.Тип("urn:order", Имя));
	Об.b = "бэ";
	Об.a = "а";
	Возврат Об;
КонецФункции

Функция Секв(Об)
	С = Об.Последовательность();
	Если С = Неопределено Тогда
		Возврат "Неопределено";
	КонецЕсли;
	Р = "";
	Для Сч = 0 По С.Количество() - 1 Цикл
		Р = Р + "[" + С.ПолучитьСвойство(Сч).Имя + "="
			+ Строка(С.ПолучитьЗначение(Сч)) + "]";
	КонецЦикла;
	Возврат Р;
КонецФункции

// --- схема ------------------------------------------------------------------

ТекстСхемы = "<?xml version=""1.0"" encoding=""UTF-8""?>"
	+ "<xs:schema xmlns:xs=""http://www.w3.org/2001/XMLSchema"" xmlns:t=""urn:order"""
	+ " targetNamespace=""urn:order"" elementFormDefault=""qualified"">"
	+ "<xs:complexType name=""Closed""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""c"" type=""xs:string"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence></xs:complexType>"
	+ "<xs:complexType name=""Opened""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""c"" type=""xs:string"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "<xs:any namespace=""##any"" processContents=""lax"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence>"
	+ "<xs:anyAttribute namespace=""##any"" processContents=""lax""/>"
	+ "</xs:complexType>"
	+ "<xs:complexType name=""OpenElem""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""c"" type=""xs:string"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "<xs:any namespace=""##any"" processContents=""lax"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence></xs:complexType>"
	+ "<xs:complexType name=""OpenAttr""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""c"" type=""xs:string"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence>"
	+ "<xs:anyAttribute namespace=""##any"" processContents=""lax""/>"
	+ "</xs:complexType>"
	+ "<xs:complexType name=""ExtOpen""><xs:complexContent>"
	+ "<xs:extension base=""t:Opened""><xs:sequence>"
	+ "<xs:element name=""d"" type=""xs:string"" minOccurs=""0""/>"
	+ "</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"
	+ "<xs:complexType name=""ExtClosed""><xs:complexContent>"
	+ "<xs:extension base=""t:Closed""><xs:sequence>"
	+ "<xs:element name=""d"" type=""xs:string"" minOccurs=""0""/>"
	+ "</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"
	+ "<xs:complexType name=""DeepAny""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:sequence>"
	+ "<xs:any namespace=""##any"" processContents=""lax"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""c"" type=""xs:string"" minOccurs=""0"" maxOccurs=""unbounded""/>"
	+ "</xs:sequence></xs:complexType>"
	+ "<xs:complexType name=""ExtAny""><xs:complexContent>"
	+ "<xs:extension base=""xs:anyType""><xs:sequence>"
	+ "<xs:element name=""a"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""b"" type=""xs:string"" minOccurs=""0""/>"
	+ "</xs:sequence></xs:extension></xs:complexContent></xs:complexType>"
	+ "<xs:complexType name=""Host""><xs:sequence>"
	+ "<xs:element name=""x"" type=""xs:string"" minOccurs=""0""/>"
	+ "<xs:element name=""inner"" type=""t:Opened"" minOccurs=""0""/>"
	+ "</xs:sequence></xs:complexType>"
	+ "</xs:schema>";

Путь = "/tmp/open-bsl-xdto-write-order.xsd";
Зпс = Новый ЗаписьТекста(Путь);
Зпс.Записать(ТекстСхемы);
Зпс.Закрыть();

Фаб = СоздатьФабрикуXDTO(Путь);

// --- флаги типов ------------------------------------------------------------

П("Closed", Флаги(Фаб.Тип("urn:order", "Closed")));
П("Opened", Флаги(Фаб.Тип("urn:order", "Opened")));
П("OpenElem", Флаги(Фаб.Тип("urn:order", "OpenElem")));
П("OpenAttr", Флаги(Фаб.Тип("urn:order", "OpenAttr")));
П("ExtOpen", Флаги(Фаб.Тип("urn:order", "ExtOpen")));
П("ExtClosed", Флаги(Фаб.Тип("urn:order", "ExtClosed")));
П("DeepAny", Флаги(Фаб.Тип("urn:order", "DeepAny")));
П("ExtAny", Флаги(Фаб.Тип("urn:order", "ExtAny")));
П("Host", Флаги(Фаб.Тип("urn:order", "Host")));

// --- выгрузка при заполнении против схемы -----------------------------------

П("зап Closed", Зап(Обратно("Closed")));
П("зап Opened", Зап(Обратно("Opened")));
П("зап OpenElem", Зап(Обратно("OpenElem")));
П("зап OpenAttr", Зап(Обратно("OpenAttr")));
П("зап ExtOpen", Зап(Обратно("ExtOpen")));
П("зап ExtClosed", Зап(Обратно("ExtClosed")));
П("зап DeepAny", Зап(Обратно("DeepAny")));
П("зап ExtAny", Зап(Обратно("ExtAny")));

// --- повторное присваивание -------------------------------------------------

ОбПовторЗакр = Фаб.Создать(Фаб.Тип("urn:order", "Closed"));
ОбПовторЗакр.a = "а1";
ОбПовторЗакр.b = "бэ";
ОбПовторЗакр.a = "а2";
П("зап Closed повтор", Зап(ОбПовторЗакр));

ОбПовтор = Фаб.Создать(Фаб.Тип("urn:order", "Opened"));
ОбПовтор.a = "а1";
ОбПовтор.b = "бэ";
ОбПовтор.a = "а2";
П("зап Opened повтор", Зап(ОбПовтор));

// --- множественное свойство -------------------------------------------------

ОбМножЗакр = Фаб.Создать(Фаб.Тип("urn:order", "Closed"));
ОбМножЗакр.c.Добавить("це1");
ОбМножЗакр.c.Добавить("це2");
ОбМножЗакр.a = "а";
П("зап Closed множественное", Зап(ОбМножЗакр));

ОбМнож = Фаб.Создать(Фаб.Тип("urn:order", "Opened"));
ОбМнож.c.Добавить("це1");
ОбМнож.c.Добавить("це2");
ОбМнож.a = "а";
П("зап Opened множественное", Зап(ОбМнож));

// --- открытый экземпляр внутри закрытого ------------------------------------

ОбХост = Фаб.Создать(Фаб.Тип("urn:order", "Host"));
ОбХост.inner = Обратно("Opened");
ОбХост.x = "икс";
П("зап вложенного открытого", Зап(ОбХост));

// --- последовательность -----------------------------------------------------

П("посл Closed", Секв(Обратно("Closed")));
П("посл Opened", Секв(Обратно("Opened")));
П("посл OpenElem", Секв(Обратно("OpenElem")));
П("посл ExtOpen", Секв(Обратно("ExtOpen")));
