// Полный набор системного ВидСравнения: алиасы и представления.

Процедура ПроверитьВидСравнения(Имя, Русский, Английский)
	Если Русский = Английский Тогда
		Совпало = "PASS";
	Иначе
		Совпало = "FAIL";
	КонецЕсли;
	Сообщить(Имя + Символ(9) + Совпало + "|" + Строка(Русский));
КонецПроцедуры

ПроверитьВидСравнения("Greater", ВидСравнения.Больше, ComparisonType.Greater);
ПроверитьВидСравнения("GreaterOrEqual", ВидСравнения.БольшеИлиРавно, ComparisonType.GreaterOrEqual);
ПроверитьВидСравнения("InHierarchy", ВидСравнения.ВИерархии, ComparisonType.InHierarchy);
ПроверитьВидСравнения("InList", ВидСравнения.ВСписке, ComparisonType.InList);
ПроверитьВидСравнения("InListByHierarchy", ВидСравнения.ВСпискеПоИерархии, ComparisonType.InListByHierarchy);
ПроверитьВидСравнения("Interval", ВидСравнения.Интервал, ComparisonType.Interval);
ПроверитьВидСравнения("IntervalIncludingBounds", ВидСравнения.ИнтервалВключаяГраницы, ComparisonType.IntervalIncludingBounds);
ПроверитьВидСравнения("IntervalIncludingLowerBound", ВидСравнения.ИнтервалВключаяНачало, ComparisonType.IntervalIncludingLowerBound);
ПроверитьВидСравнения("IntervalIncludingUpperBound", ВидСравнения.ИнтервалВключаяОкончание, ComparisonType.IntervalIncludingUpperBound);
ПроверитьВидСравнения("Less", ВидСравнения.Меньше, ComparisonType.Less);
ПроверитьВидСравнения("LessOrEqual", ВидСравнения.МеньшеИлиРавно, ComparisonType.LessOrEqual);
ПроверитьВидСравнения("NotInHierarchy", ВидСравнения.НеВИерархии, ComparisonType.NotInHierarchy);
ПроверитьВидСравнения("NotInList", ВидСравнения.НеВСписке, ComparisonType.NotInList);
ПроверитьВидСравнения("NotInListByHierarchy", ВидСравнения.НеВСпискеПоИерархии, ComparisonType.NotInListByHierarchy);
ПроверитьВидСравнения("NotEqual", ВидСравнения.НеРавно, ComparisonType.NotEqual);
ПроверитьВидСравнения("NotContains", ВидСравнения.НеСодержит, ComparisonType.NotContains);
ПроверитьВидСравнения("Equal", ВидСравнения.Равно, ComparisonType.Equal);
ПроверитьВидСравнения("Contains", ВидСравнения.Содержит, ComparisonType.Contains);

Сообщить("ROOT" + Символ(9) + Строка(ВидСравнения));
Сообщить("TYPE" + Символ(9) + Строка(ТипЗнч(ВидСравнения.Равно)));
