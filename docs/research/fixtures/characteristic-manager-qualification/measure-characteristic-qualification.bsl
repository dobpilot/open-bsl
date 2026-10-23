Попытка
    ПробаХарактеристики = ПланыВидовХарактеристик.ЗамерХарактеристик.СоздатьЧерезКонтекст();
    Сообщить("characteristic.qualification.manager.context=" + Строка(?(ПробаХарактеристики.ЭтоНовый(), 1, 0)));
    ПробаХарактеристики.Код = "123456789ABCDE";
    Сообщить("characteristic.qualification.code=" + ПробаХарактеристики.Код);
    ПробаХарактеристики.Наименование = "123456789012345678901234567890";
    Сообщить("characteristic.qualification.description=" + ПробаХарактеристики.Наименование);
    ПробаХарактеристики.ТипЗначения = Новый ОписаниеТипов("Строка");
    Сообщить("characteristic.qualification.type.count=" + Строка(ПробаХарактеристики.ТипЗначения.Типы().Количество()));
    Сообщить("characteristic.qualification.type.string=" + Строка(?(ПробаХарактеристики.ТипЗначения.СодержитТип(Тип("Строка")), 1, 0)));
Исключение
    Сообщить("characteristic.qualification.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    ПробаХарактеристики.ТипЗначения = Неопределено;
    Сообщить("characteristic.qualification.undefined=ACCEPT");
Исключение
    Сообщить("characteristic.qualification.undefined=ERROR");
КонецПопытки;
Попытка
    Сообщить("characteristic.qualification.undefined.type_description=" + Строка(?(ТипЗнч(ПробаХарактеристики.ТипЗначения) = Тип("ОписаниеТипов"), 1, 0)));
    Сообщить("characteristic.qualification.undefined.count=" + Строка(ПробаХарактеристики.ТипЗначения.Типы().Количество()));
Исключение
    Сообщить("characteristic.qualification.undefined.read=ERROR");
КонецПопытки;
Попытка
    ПробаХарактеристики.ЭтоГруппа = Истина;
    Сообщить("characteristic.qualification.folder.write=ACCEPT");
Исключение
    Сообщить("characteristic.qualification.folder.write=ERROR");
КонецПопытки;
