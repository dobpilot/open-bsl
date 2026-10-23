Попытка
    РезультатПробы = ЗамерыТранзитивногоРодителя.Проба();
    ПрямойРезультатПробы = НСтр("ru='контроль';en='контроль'",);
    Сообщить("image.native.external_function_nstr_hole.value=" + РезультатПробы);
    Сообщить("image.native.external_function_nstr_hole.direct=" + Строка(?(РезультатПробы = ПрямойРезультатПробы, 1, 0)));
Исключение
    Сообщить("image.native.external_function_nstr_hole.failure=" + ОписаниеОшибки());
КонецПопытки;
