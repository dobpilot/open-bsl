Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба(АргументКадра);
    Сообщить("image.nonzero-label-private-frame.public.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.nonzero-label-private-frame.public.value=" + Строка(РезультатКадра));
    Сообщить("image.nonzero-label-private-frame.public.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.nonzero-label-private-frame.public.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.ПробаЧастногоВхода(АргументКадра);
    Сообщить("image.nonzero-label-private-frame.private.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.nonzero-label-private-frame.private.value=" + Строка(РезультатКадра));
    Сообщить("image.nonzero-label-private-frame.private.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.nonzero-label-private-frame.private.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.nonzero-label-private-frame.done=1");
