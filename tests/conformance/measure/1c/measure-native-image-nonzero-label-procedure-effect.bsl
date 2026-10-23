Попытка
    Сообщить("image.label-procedure-effect.entry=public");
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.Проба(АргументКадра);
    Сообщить("image.label-procedure-effect.public.value=" + Строка(РезультатКадра));
    Сообщить("image.label-procedure-effect.public.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.label-procedure-effect.public.error=" + ОписаниеОшибки());
КонецПопытки;
Попытка
    Сообщить("image.label-procedure-effect.entry=private");
    АргументКадра = 5;
    РезультатКадра = ЗамерыТранзитивногоРодителя.ПробаЧастногоВхода(АргументКадра);
    Сообщить("image.label-procedure-effect.private.value=" + Строка(РезультатКадра));
    Сообщить("image.label-procedure-effect.private.argument=" + Строка(АргументКадра));
Исключение
    Сообщить("image.label-procedure-effect.private.error=" + ОписаниеОшибки());
КонецПопытки;
Сообщить("image.label-procedure-effect.done=1");
