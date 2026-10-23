Попытка
    ОбъектОбмена = БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс();
    Сообщить("business.exchange-module.initialization" + Символ(9) + ОбъектОбмена.СостояниеОбмена());
    Сообщить("business.exchange-module.load" + Символ(9) + ?(ОбъектОбмена.ОбменДанными.Загрузка, "1", "0"));
    Сообщить("business.exchange-module.autofill" + Символ(9) + ?(ОбъектОбмена.DataExchange.Recipients.AutoFill, "1", "0"));
    Сообщить("business.exchange-module.modified" + Символ(9) + ?(ОбъектОбмена.Modified(), "1", "0"));
Исключение
    Сообщить("business.exchange-module.error" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
