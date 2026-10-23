Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба(АргументКадра);
    Сообщить("image.zero-label-private-procedure.public.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.zero-label-private-procedure.public.value=" + Строка(РезультатКадра));
    Сообщить("image.zero-label-private-procedure.public.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.zero-label-private-procedure.public.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.ПробаЧастногоВхода(АргументКадра);
    Сообщить("image.zero-label-private-procedure.private.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.zero-label-private-procedure.private.value=" + Строка(РезультатКадра));
    Сообщить("image.zero-label-private-procedure.private.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.zero-label-private-procedure.private.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.zero-label-private-procedure.done=1");
