Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба(АргументКадра);
    Сообщить("image.nonzero-label-private-defaults.public.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.nonzero-label-private-defaults.public.value=" + Строка(РезультатКадра));
    Сообщить("image.nonzero-label-private-defaults.public.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.nonzero-label-private-defaults.public.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.ПробаЧастногоВхода(АргументКадра);
    Сообщить("image.nonzero-label-private-defaults.private.type=" + Строка(ТипЗнч(РезультатКадра)));
    Сообщить("image.nonzero-label-private-defaults.private.value=" + Строка(РезультатКадра));
    Сообщить("image.nonzero-label-private-defaults.private.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.nonzero-label-private-defaults.private.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.nonzero-label-private-defaults.done=1");
