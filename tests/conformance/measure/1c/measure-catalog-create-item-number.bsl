Попытка
    ОбъектПробы = Вычислить("Справочники.ЗамерВременныхТаблиц.СоздатьЭлемент()");
    Сообщить("catalog.numeric.new=" + Строка(ОбъектПробы.ЭтоНовый()));
    Сообщить("catalog.numeric.reference.empty=" + Строка(Не ЗначениеЗаполнено(ОбъектПробы.Ссылка)));
    Сообщить("catalog.numeric.code.type=" + Строка(ТипЗнч(ОбъектПробы.Код) = Тип("Число")));
    Сообщить("catalog.numeric.code.zero=" + Строка(ОбъектПробы.Код = 0));
    Сообщить("catalog.numeric.presentation=[" + Строка(ОбъектПробы) + "]");
    ОбъектПробы.Код = "7";
    Сообщить("catalog.numeric.code.string=" + Строка(ОбъектПробы.Код));
Исключение
    Сообщить("catalog.numeric.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    НаименованиеПробы = ОбъектПробы.Наименование;
    Сообщить("catalog.numeric.description=present");
Исключение
    Сообщить("catalog.numeric.description=absent");
КонецПопытки;
Попытка
    ОбъектПробы.Код = "invalid";
    Сообщить("catalog.numeric.invalid.code=accepted");
Исключение
    Сообщить("catalog.numeric.invalid.code=rejected");
КонецПопытки;
Сообщить("catalog.numeric.code.after.invalid=" + Строка(ОбъектПробы.Код));

Попытка
    Сообщить("catalog.numeric.description.default=[" + ОбъектПробы.Наименование + "]");
    ОбъектПробы.Наименование = "Пробное имя";
    Сообщить("catalog.numeric.description.assigned=[" + ОбъектПробы.Наименование + "]");
Исключение
    Сообщить("catalog.numeric.description.assigned=rejected");
КонецПопытки;
