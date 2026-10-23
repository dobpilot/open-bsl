Попытка
    ОбъектПолейТелаПробы = БизнесПроцессы.ЗамерБизнесПроцесса.СоздатьБизнесПроцесс();
    РезультатыПолейТелаПробы = ОбъектПолейТелаПробы.РезультатыТелаСобственныхПолейПробы();
    Сообщить("business.fields.body.text.read.error" + Символ(9) + РезультатыПолейТелаПробы.text_read);
    Сообщить("business.fields.body.text.assign.error" + Символ(9) + РезультатыПолейТелаПробы.text_assign);
    Сообщить("business.fields.body.flag.read.error" + Символ(9) + РезультатыПолейТелаПробы.flag_read);
    Сообщить("business.fields.body.flag.assign.error" + Символ(9) + РезультатыПолейТелаПробы.flag_assign);
    Сообщить("business.fields.body.text.after-create" + Символ(9) + ОбъектПолейТелаПробы.ТекстПробы);
    Сообщить("business.fields.body.flag.after-create" + Символ(9) + ?(ОбъектПолейТелаПробы.ФлагПробы, 1, 0));
Исключение
    Сообщить("business.fields.body.error" + Символ(9) + ОписаниеОшибки());
КонецПопытки;
